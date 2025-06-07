use crate::config::model::{Config, Provider};
use crate::relay::client::openai::OpenAIClient;
use super::MetricsCollector;
use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// 健康检查器
/// 定期检查所有provider的健康状态
pub struct HealthChecker {
    config: Arc<Config>,
    metrics: Arc<MetricsCollector>,
    client: Client,
    check_interval: Duration,
}

impl HealthChecker {
    /// 创建新的健康检查器
    pub fn new(config: Arc<Config>, metrics: Arc<MetricsCollector>) -> Self {
        let check_interval = Duration::from_secs(config.settings.health_check_interval_seconds);
        let timeout = Duration::from_secs(config.settings.request_timeout_seconds);
        
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            metrics,
            client,
            check_interval,
        }
    }

    /// 启动健康检查循环
    pub async fn start(&self) {
        info!("Starting health checker with interval: {:?}", self.check_interval);
        
        let mut interval = interval(self.check_interval);
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.check_all_providers().await {
                error!("Health check failed: {}", e);
            }
        }
    }

    /// 检查所有provider的健康状态
    async fn check_all_providers(&self) -> Result<()> {
        debug!("Starting health check for all providers");
        
        let mut tasks = Vec::new();
        
        for (provider_id, provider) in &self.config.providers {
            if provider.enabled {
                let provider_id = provider_id.clone();
                let provider = provider.clone();
                let client = self.client.clone();
                let metrics = self.metrics.clone();
                
                let task = tokio::spawn(async move {
                    Self::check_provider_health(&provider_id, &provider, &client, &metrics).await
                });
                
                tasks.push(task);
            }
        }
        
        // 等待所有健康检查完成
        for task in tasks {
            if let Err(e) = task.await {
                error!("Health check task failed: {}", e);
            }
        }
        
        debug!("Completed health check for all providers");
        Ok(())
    }

    /// 检查单个provider的健康状态
    async fn check_provider_health(
        provider_id: &str,
        provider: &Provider,
        client: &Client,
        metrics: &MetricsCollector,
    ) {
        let start_time = Instant::now();

        // 直接使用配置中的API密钥
        let api_key = &provider.api_key;

        if api_key.is_empty() {
            warn!("API key is empty for provider {}", provider_id);
            // 标记所有模型为不健康
            for model in &provider.models {
                metrics.record_failure(&format!("{}:{}", provider_id, model));
            }
            return;
        }

        // 使用真实的API检查
        if provider.base_url.contains("httpbin.org") {
            // 对于测试服务，使用httpbin的状态端点
            Self::check_test_provider(provider_id, provider, client, metrics, start_time).await;
        } else {
            // 对于真实的AI服务，使用model list API检查
            Self::check_real_provider(provider_id, provider, metrics, start_time).await;
        }
    }

    /// 检查测试provider（httpbin等）
    async fn check_test_provider(
        provider_id: &str,
        provider: &Provider,
        client: &Client,
        metrics: &MetricsCollector,
        start_time: Instant,
    ) {
        let health_check_url = format!("{}/status/200", provider.base_url);
        let mut request = client.get(&health_check_url);

        // 添加自定义头部
        for (key, value) in &provider.headers {
            request = request.header(key, value);
        }

        // 发送请求
        match request.send().await {
            Ok(response) => {
                let latency = start_time.elapsed();

                if response.status().is_success() {
                    debug!("Provider {} health check passed ({}ms)", provider_id, latency.as_millis());

                    // 标记所有模型为健康
                    for model in &provider.models {
                        let backend_key = format!("{}:{}", provider_id, model);
                        metrics.record_latency(&backend_key, latency);
                        metrics.record_success(&backend_key);
                        metrics.update_health_check(&backend_key);
                    }
                } else {
                    warn!("Provider {} health check failed with status: {}", provider_id, response.status());

                    // 标记所有模型为不健康
                    for model in &provider.models {
                        metrics.record_failure(&format!("{}:{}", provider_id, model));
                    }
                }
            }
            Err(e) => {
                error!("Provider {} health check error: {}", provider_id, e);

                // 标记所有模型为不健康
                for model in &provider.models {
                    metrics.record_failure(&format!("{}:{}", provider_id, model));
                }
            }
        }
    }

    /// 检查真实的AI provider
    async fn check_real_provider(
        provider_id: &str,
        provider: &Provider,
        metrics: &MetricsCollector,
        start_time: Instant,
    ) {
        let openai_client = OpenAIClient::with_base_url(provider.base_url.clone());

        // 使用models API检查provider健康状态
        match openai_client.models(&provider.api_key).await {
            Ok(response) => {
                let latency = start_time.elapsed();

                if response.is_success {
                    debug!("Provider {} models API check passed ({}ms)", provider_id, latency.as_millis());

                    // 标记所有模型为健康
                    for model in &provider.models {
                        let backend_key = format!("{}:{}", provider_id, model);
                        metrics.record_latency(&backend_key, latency);
                        metrics.record_success(&backend_key);
                        metrics.update_health_check(&backend_key);
                    }
                } else {
                    warn!("Provider {} models API check failed: {}", provider_id, response.body);

                    // 标记所有模型为不健康
                    for model in &provider.models {
                        metrics.record_failure(&format!("{}:{}", provider_id, model));
                    }
                }
            }
            Err(e) => {
                error!("Provider {} models API error: {}", provider_id, e);

                // 标记所有模型为不健康
                for model in &provider.models {
                    metrics.record_failure(&format!("{}:{}", provider_id, model));
                }
            }
        }
    }

    /// 手动触发健康检查
    pub async fn check_now(&self) -> Result<()> {
        info!("Manual health check triggered");
        self.check_all_providers().await
    }

    /// 检查特定provider的健康状态
    pub async fn check_provider(&self, provider_id: &str) -> Result<()> {
        if let Some(provider) = self.config.providers.get(provider_id) {
            if provider.enabled {
                Self::check_provider_health(
                    provider_id,
                    provider,
                    &self.client,
                    &self.metrics,
                ).await;
                Ok(())
            } else {
                anyhow::bail!("Provider '{}' is disabled", provider_id);
            }
        } else {
            anyhow::bail!("Provider '{}' not found", provider_id);
        }
    }

    /// 检查不健康的provider是否可以恢复
    pub async fn check_recovery(&self) -> Result<()> {
        let recovery_interval = Duration::from_secs(self.config.settings.recovery_check_interval_seconds);
        let unhealthy_backends = self.metrics.get_unhealthy_backends();

        if unhealthy_backends.is_empty() {
            debug!("No unhealthy backends to check for recovery");
            return Ok(());
        }

        info!("Checking recovery for {} unhealthy backends", unhealthy_backends.len());

        for unhealthy_backend in unhealthy_backends {
            if self.metrics.needs_recovery_check(&unhealthy_backend.backend_key, recovery_interval) {
                // 解析backend_key获取provider_id和model
                let parts: Vec<&str> = unhealthy_backend.backend_key.split(':').collect();
                if parts.len() != 2 {
                    warn!("Invalid backend key format: {}", unhealthy_backend.backend_key);
                    continue;
                }

                let provider_id = parts[0];
                let model_name = parts[1];

                if let Some(provider) = self.config.providers.get(provider_id) {
                    if provider.enabled {
                        info!("Attempting recovery check for {}:{}", provider_id, model_name);
                        self.metrics.record_recovery_attempt(&unhealthy_backend.backend_key);

                        // 使用chat请求进行恢复检查
                        self.check_recovery_with_chat(provider_id, provider, model_name).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// 使用chat请求检查provider恢复状态
    async fn check_recovery_with_chat(
        &self,
        provider_id: &str,
        provider: &Provider,
        model_name: &str,
    ) {
        let start_time = Instant::now();
        let openai_client = OpenAIClient::with_base_url(provider.base_url.clone());

        // 构建简单的chat请求
        let test_body = json!({
            "model": model_name,
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ],
            "max_tokens": 1,
            "stream": false
        });

        // 构建请求头
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", provider.api_key).parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        // 添加自定义头部
        for (key, value) in &provider.headers {
            if let (Ok(header_name), Ok(header_value)) = (
                key.parse::<reqwest::header::HeaderName>(),
                value.parse::<reqwest::header::HeaderValue>()
            ) {
                headers.insert(header_name, header_value);
            }
        }

        match openai_client.chat_completions(headers, &test_body).await {
            Ok(response) => {
                let latency = start_time.elapsed();
                let backend_key = format!("{}:{}", provider_id, model_name);

                if response.status().is_success() {
                    info!("Recovery check passed for {}:{} ({}ms)", provider_id, model_name, latency.as_millis());

                    // 恢复成功，标记为健康
                    self.metrics.record_latency(&backend_key, latency);
                    self.metrics.record_success(&backend_key);
                    self.metrics.update_health_check(&backend_key);
                } else {
                    warn!("Recovery check failed for {}:{} with status: {}", provider_id, model_name, response.status());
                    // 保持不健康状态
                }
            }
            Err(e) => {
                error!("Recovery check error for {}:{}: {}", provider_id, model_name, e);
                // 保持不健康状态
            }
        }
    }

    /// 获取健康检查统计信息
    pub fn get_health_summary(&self) -> HealthSummary {
        let mut total_providers = 0;
        let mut healthy_providers = 0;
        let mut total_models = 0;
        let mut healthy_models = 0;

        for (provider_id, provider) in &self.config.providers {
            if provider.enabled {
                total_providers += 1;
                let mut provider_healthy = true;

                for model in &provider.models {
                    total_models += 1;

                    if self.metrics.is_healthy(provider_id, model) {
                        healthy_models += 1;
                    } else {
                        provider_healthy = false;
                    }
                }

                if provider_healthy {
                    healthy_providers += 1;
                }
            }
        }

        HealthSummary {
            total_providers,
            healthy_providers,
            total_models,
            healthy_models,
            provider_health_ratio: if total_providers > 0 {
                healthy_providers as f64 / total_providers as f64
            } else {
                0.0
            },
            model_health_ratio: if total_models > 0 {
                healthy_models as f64 / total_models as f64
            } else {
                0.0
            },
        }
    }
}

/// 健康检查摘要
#[derive(Debug, Clone)]
pub struct HealthSummary {
    pub total_providers: usize,
    pub healthy_providers: usize,
    pub total_models: usize,
    pub healthy_models: usize,
    pub provider_health_ratio: f64,
    pub model_health_ratio: f64,
}

impl HealthSummary {
    /// 检查整体系统是否健康
    pub fn is_system_healthy(&self) -> bool {
        self.model_health_ratio > 0.5 // 至少50%的模型健康
    }

    /// 检查是否有任何可用的模型
    pub fn has_available_models(&self) -> bool {
        self.healthy_models > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{GlobalSettings, ModelMapping, Backend, LoadBalanceStrategy};
    use std::collections::HashMap;

    fn create_test_config() -> Config {
        let mut providers = HashMap::new();
        providers.insert("test-provider".to_string(), Provider {
            name: "Test Provider".to_string(),
            base_url: "https://httpbin.org".to_string(), // 使用httpbin进行测试
            api_key: "test-api-key".to_string(),
            models: vec!["test-model".to_string()],
            headers: HashMap::new(),
            enabled: true,
            timeout_seconds: 5,
            max_retries: 1,
        });

        let mut models = HashMap::new();
        models.insert("test-model".to_string(), ModelMapping {
            name: "test-model".to_string(),
            backends: vec![Backend {
                provider: "test-provider".to_string(),
                model: "test-model".to_string(),
                weight: 1.0,
                priority: 1,
                enabled: true,
                tags: vec![],
            }],
            strategy: LoadBalanceStrategy::WeightedRandom,
            enabled: true,
        });

        Config {
            providers,
            models,
            users: HashMap::new(),
            settings: GlobalSettings {
                health_check_interval_seconds: 10,
                request_timeout_seconds: 5,
                max_retries: 1,
                circuit_breaker_failure_threshold: 3,
                circuit_breaker_timeout_seconds: 30,
                recovery_check_interval_seconds: 120,
                max_internal_retries: 2,
                health_check_timeout_seconds: 10,
            },
        }
    }

    #[tokio::test]
    async fn test_health_checker_creation() {
        let config = Arc::new(create_test_config());
        let metrics = Arc::new(MetricsCollector::new());
        
        let checker = HealthChecker::new(config, metrics);
        let summary = checker.get_health_summary();
        
        assert_eq!(summary.total_providers, 1);
        assert_eq!(summary.total_models, 1);
    }
}
