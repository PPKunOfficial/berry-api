use super::selector::{RequestResult, SmartAiErrorType};
use super::{BackendSelector, MetricsCollector};
use anyhow::Result;
use berry_core::config::model::{Backend, Config, ModelMapping};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 负载均衡管理器
/// 负责管理所有模型的负载均衡选择器和指标收集
pub struct LoadBalanceManager {
    config: Arc<Config>,
    selectors: Arc<RwLock<HashMap<String, BackendSelector>>>,
    metrics: Arc<MetricsCollector>,
}

impl LoadBalanceManager {
    /// 创建新的负载均衡管理器
    pub fn new(config: Config) -> Self {
        let config = Arc::new(config);
        let metrics = Arc::new(MetricsCollector::new());
        let selectors = Arc::new(RwLock::new(HashMap::new()));

        Self {
            config,
            selectors,
            metrics,
        }
    }

    /// 初始化所有模型的选择器
    pub async fn initialize(&self) -> Result<()> {
        let mut selectors = self.selectors.write().await;

        for (model_id, model_mapping) in &self.config.models {
            if model_mapping.enabled {
                let selector = BackendSelector::new(model_mapping.clone(), self.metrics.clone());
                selectors.insert(model_id.clone(), selector);
            }
        }

        tracing::info!("Initialized {} model selectors", selectors.len());
        Ok(())
    }

    /// 为指定模型选择后端
    pub async fn select_backend(&self, model_name: &str) -> Result<Backend> {
        self.select_backend_with_user_tags(model_name, None).await
    }

    /// 为指定模型选择后端（支持用户标签过滤）
    pub async fn select_backend_with_user_tags(
        &self,
        model_name: &str,
        user_tags: Option<&[String]>,
    ) -> Result<Backend> {
        // 首先尝试通过模型ID查找
        if let Some(selector) = self.selectors.read().await.get(model_name) {
            return if let Some(tags) = user_tags {
                selector.select_with_user_tags(tags)
            } else {
                selector.select()
            };
        }

        // 如果没找到，尝试通过模型的真实名称查找
        for (_, selector) in self.selectors.read().await.iter() {
            if selector.get_model_name() == model_name {
                return if let Some(tags) = user_tags {
                    selector.select_with_user_tags(tags)
                } else {
                    selector.select()
                };
            }
        }

        anyhow::bail!("Model '{}' not found or not enabled", model_name)
    }

    /// 获取指定模型的配置
    pub fn get_model_config(&self, model_name: &str) -> Option<&ModelMapping> {
        self.config.get_model(model_name)
    }

    /// 获取所有可用的模型列表
    pub fn get_available_models(&self) -> Vec<String> {
        self.config.get_available_models()
    }

    /// 记录请求成功
    pub fn record_success(&self, provider: &str, model: &str, latency: std::time::Duration) {
        let backend_key = format!("{}:{}", provider, model);
        self.metrics.record_latency(&backend_key, latency);
        self.metrics.record_success(&backend_key);
    }

    /// 记录请求失败
    pub fn record_failure(&self, provider: &str, model: &str) {
        let backend_key = format!("{}:{}", provider, model);
        self.metrics.record_failure(&backend_key);
    }

    /// 获取指标收集器的引用
    pub fn get_metrics(&self) -> Arc<MetricsCollector> {
        self.metrics.clone()
    }

    /// 记录SmartAI请求结果
    pub fn record_smart_ai_request(&self, provider: &str, model: &str, result: RequestResult) {
        let backend_key = format!("{}:{}", provider, model);
        self.metrics.record_smart_ai_request(&backend_key, result);
    }

    /// 分类错误类型（用于SmartAI）
    pub fn classify_error(error: &anyhow::Error) -> SmartAiErrorType {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("timeout") || error_str.contains("timed out") {
            SmartAiErrorType::TimeoutError
        } else if error_str.contains("401")
            || error_str.contains("403")
            || error_str.contains("unauthorized")
        {
            SmartAiErrorType::AuthError
        } else if error_str.contains("429") || error_str.contains("rate limit") {
            SmartAiErrorType::RateLimitError
        } else if error_str.contains("5")
            && (error_str.contains("500") || error_str.contains("502") || error_str.contains("503"))
        {
            SmartAiErrorType::ServerError
        } else if error_str.contains("model") && error_str.contains("not found") {
            SmartAiErrorType::ModelError
        } else {
            SmartAiErrorType::NetworkError
        }
    }

    /// 更新SmartAI连通性检查结果
    pub fn update_smart_ai_connectivity(&self, provider: &str, model: &str, connectivity_ok: bool) {
        let backend_key = format!("{}:{}", provider, model);
        self.metrics
            .update_smart_ai_connectivity(&backend_key, connectivity_ok);
    }

    /// 获取模型的健康状态统计
    pub async fn get_health_stats(&self) -> HashMap<String, HealthStats> {
        let mut stats = HashMap::new();

        for (model_id, selector) in self.selectors.read().await.iter() {
            let mut healthy_backends = 0;
            let mut total_backends = 0;
            let mut total_latency = std::time::Duration::ZERO;
            let mut latency_count = 0;

            for backend in &selector.get_mapping().backends {
                if backend.enabled {
                    total_backends += 1;

                    if self.metrics.is_healthy(&backend.provider, &backend.model) {
                        healthy_backends += 1;
                    }

                    if let Some(latency) =
                        self.metrics.get_latency(&backend.provider, &backend.model)
                    {
                        total_latency += latency;
                        latency_count += 1;
                    }
                }
            }

            let avg_latency = if latency_count > 0 {
                Some(total_latency / latency_count as u32)
            } else {
                None
            };

            stats.insert(
                model_id.clone(),
                HealthStats {
                    healthy_backends,
                    total_backends,
                    health_ratio: if total_backends > 0 {
                        healthy_backends as f64 / total_backends as f64
                    } else {
                        0.0
                    },
                    average_latency: avg_latency,
                },
            );
        }

        stats
    }

    /// 获取配置的引用
    pub fn get_config(&self) -> Arc<Config> {
        self.config.clone()
    }

    /// 获取缓存统计信息
    pub async fn get_cache_stats(&self) -> Option<super::cache::CacheStats> {
        let selectors = self.selectors.read().await;
        selectors
            .values()
            .next()
            .map(|selector| selector.get_cache_stats())
    }

    /// 获取模型权重信息（用于监控）
    pub async fn get_model_weights(
        &self,
        model_name: &str,
    ) -> Result<std::collections::HashMap<String, f64>> {
        let selectors = self.selectors.read().await;

        // 查找对应的selector
        let selector = selectors
            .get(model_name)
            .or_else(|| {
                // 尝试通过显示名称查找
                selectors
                    .values()
                    .find(|s| s.get_model_name() == model_name)
            })
            .ok_or_else(|| anyhow::anyhow!("Model '{}' not found or not enabled", model_name))?;

        // 直接使用selector的权重计算方法
        Ok(selector.get_current_weights())
    }
}

/// 健康状态统计
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStats {
    pub healthy_backends: usize,
    pub total_backends: usize,
    pub health_ratio: f64,
    pub average_latency: Option<std::time::Duration>,
}

impl HealthStats {
    /// 检查模型是否健康
    pub fn is_healthy(&self) -> bool {
        self.health_ratio > 0.0
    }

    /// 检查模型是否完全健康
    pub fn is_fully_healthy(&self) -> bool {
        self.health_ratio >= 1.0
    }
}
