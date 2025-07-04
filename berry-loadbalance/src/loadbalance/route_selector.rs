use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;

/// 线路选择器 - 负载均衡的核心抽象
///
/// 这个trait将复杂的负载均衡逻辑抽象为简单的线路选择接口：
/// 1. 选择线路 - 根据模型名称选择最佳后端线路
/// 2. 报告状态 - 告知选择器请求的成功/失败状态
#[async_trait]
pub trait RouteSelector: Send + Sync {
    /// 选择线路
    ///
    /// # 参数
    /// - `model_name`: 请求的模型名称
    /// - `user_tags`: 可选的用户标签，用于过滤后端
    ///
    /// # 返回
    /// - `Ok(SelectedRoute)`: 成功选择的线路信息
    /// - `Err(RouteSelectionError)`: 选择失败的详细错误信息
    async fn select_route(
        &self,
        model_name: &str,
        user_tags: Option<&[String]>,
    ) -> Result<SelectedRoute, RouteSelectionError>;

    /// 选择指定提供商的线路（用于调试和测试）
    async fn select_specific_route(
        &self,
        model_name: &str,
        provider_name: &str,
    ) -> Result<SelectedRoute, RouteSelectionError>;

    /// 报告请求结果
    ///
    /// 这是选择器了解线路状态的唯一方式，用于：
    /// - 更新健康状态
    /// - 调整权重
    /// - 记录指标
    async fn report_result(&self, route_id: &str, result: RouteResult);

    /// 获取线路统计信息（用于监控）
    async fn get_route_stats(&self) -> RouteStats;
}

/// 选中的线路信息
#[derive(Debug, Clone)]
pub struct SelectedRoute {
    /// 唯一的线路标识符，用于后续状态报告
    pub route_id: String,

    /// 后端提供商信息
    pub provider: RouteProvider,

    /// 后端模型信息
    pub backend: RouteBackend,

    /// 选择耗时
    pub selection_time: Duration,
}

impl SelectedRoute {
    /// 获取完整的API URL
    pub fn get_api_url(&self, endpoint: &str) -> String {
        format!(
            "{}/{}",
            self.provider.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        )
    }

    /// 获取API密钥
    pub fn get_api_key(&self) -> Result<String> {
        if self.provider.api_key.is_empty() {
            anyhow::bail!("API key is empty for provider: {}", self.provider.name);
        }
        Ok(self.provider.api_key.clone())
    }

    /// 获取请求头
    pub fn get_headers(&self) -> std::collections::HashMap<String, String> {
        self.provider.headers.clone()
    }

    /// 获取超时设置
    pub fn get_timeout(&self) -> Duration {
        Duration::from_secs(self.provider.timeout_seconds)
    }
}

/// 线路提供商信息
#[derive(Debug, Clone)]
pub struct RouteProvider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub headers: std::collections::HashMap<String, String>,
    pub timeout_seconds: u64,
    pub backend_type: berry_core::ProviderBackendType,
}

/// 线路后端信息
#[derive(Debug, Clone)]
pub struct RouteBackend {
    pub provider: String,
    pub model: String,
    pub weight: f64,
    pub enabled: bool,
    pub tags: Vec<String>,
}

/// 请求结果
#[derive(Debug, Clone)]
pub enum RouteResult {
    /// 请求成功
    Success {
        /// 请求延迟
        latency: Duration,
    },
    /// 请求失败
    Failure {
        /// 错误信息
        error: String,
        /// 错误类型（用于智能分析）
        error_type: Option<RouteErrorType>,
    },
}

/// 路由错误类型
#[derive(Debug, Clone)]
pub enum RouteErrorType {
    /// 网络错误（连接超时、DNS失败等）
    Network,
    /// 认证错误（401、403、API密钥无效等）
    Authentication,
    /// 限流错误（429 Too Many Requests）
    RateLimit,
    /// 服务器错误（5xx错误）
    Server,
    /// 模型错误（模型不存在、参数错误等）
    Model,
    /// 请求超时
    Timeout,
}

/// 线路选择错误
#[derive(Debug, Clone)]
pub struct RouteSelectionError {
    pub model_name: String,
    pub message: String,
    pub total_routes: usize,
    pub healthy_routes: usize,
    pub enabled_routes: usize,
    pub failed_attempts: Vec<FailedRouteAttempt>,
}

impl std::fmt::Display for RouteSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Route selection failed for model '{}': {}",
            self.model_name, self.message
        )
    }
}

impl std::error::Error for RouteSelectionError {}

/// 失败的线路尝试记录
#[derive(Debug, Clone)]
pub struct FailedRouteAttempt {
    pub route_id: String,
    pub provider: String,
    pub model: String,
    pub reason: String,
    pub is_healthy: bool,
}

/// 线路统计信息
#[derive(Debug, Clone)]
pub struct RouteStats {
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 各线路的详细统计
    pub route_details: std::collections::HashMap<String, RouteDetail>,
}

/// 单个线路的详细统计
#[derive(Debug, Clone)]
pub struct RouteDetail {
    pub route_id: String,
    pub provider: String,
    pub model: String,
    pub is_healthy: bool,
    pub request_count: u64,
    pub error_count: u64,
    pub average_latency: Option<Duration>,
    pub current_weight: f64,
}

impl RouteStats {
    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            self.successful_requests as f64 / self.total_requests as f64
        } else {
            0.0
        }
    }

    /// 获取健康线路数量
    pub fn healthy_routes_count(&self) -> usize {
        self.route_details.values().filter(|r| r.is_healthy).count()
    }
}

/// 基于现有LoadBalanceService的RouteSelector实现
pub struct LoadBalanceRouteSelector {
    service: std::sync::Arc<super::service::LoadBalanceService>,
}

impl LoadBalanceRouteSelector {
    /// 创建新的路由选择器
    pub fn new(service: std::sync::Arc<super::service::LoadBalanceService>) -> Self {
        Self { service }
    }

    /// 将Backend转换为RouteBackend
    fn convert_backend(backend: &berry_core::Backend) -> RouteBackend {
        RouteBackend {
            provider: backend.provider.clone(),
            model: backend.model.clone(),
            weight: backend.weight,
            enabled: backend.enabled,
            tags: backend.tags.clone(),
        }
    }

    /// 将Provider转换为RouteProvider
    fn convert_provider(provider: &berry_core::Provider) -> RouteProvider {
        RouteProvider {
            name: provider.name.clone(),
            base_url: provider.base_url.clone(),
            api_key: provider.api_key.clone(),
            headers: provider.headers.clone(),
            timeout_seconds: provider.timeout_seconds,
            backend_type: provider.backend_type.clone(),
        }
    }

    /// 将SelectedBackend转换为SelectedRoute
    fn convert_selected_backend(selected: super::service::SelectedBackend) -> SelectedRoute {
        let route_id = format!("{}:{}", selected.backend.provider, selected.backend.model);

        SelectedRoute {
            route_id,
            provider: Self::convert_provider(&selected.provider),
            backend: Self::convert_backend(&selected.backend),
            selection_time: selected.selection_time,
        }
    }

    /// 将RouteResult转换为RequestResult
    fn convert_route_result(result: RouteResult) -> super::service::RequestResult {
        match result {
            RouteResult::Success { latency } => super::service::RequestResult::Success { latency },
            RouteResult::Failure { error, .. } => super::service::RequestResult::Failure { error },
        }
    }

    /// 解析route_id获取provider和model
    fn parse_route_id(route_id: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = route_id.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid route_id format: {}", route_id);
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }
}

#[async_trait]
impl RouteSelector for LoadBalanceRouteSelector {
    async fn select_route(
        &self,
        model_name: &str,
        user_tags: Option<&[String]>,
    ) -> Result<SelectedRoute, RouteSelectionError> {
        match self
            .service
            .select_backend_with_user_tags(model_name, user_tags)
            .await
        {
            Ok(selected) => Ok(Self::convert_selected_backend(selected)),
            Err(e) => {
                // 尝试从错误中提取详细信息
                if let Some(detailed_error) =
                    e.downcast_ref::<super::selector::BackendSelectionError>()
                {
                    let failed_attempts = detailed_error
                        .failed_attempts
                        .iter()
                        .map(|attempt| FailedRouteAttempt {
                            route_id: attempt.backend_key.clone(),
                            provider: attempt.provider.clone(),
                            model: attempt.model.clone(),
                            reason: attempt.reason.clone(),
                            is_healthy: attempt.is_healthy,
                        })
                        .collect();

                    Err(RouteSelectionError {
                        model_name: detailed_error.model_name.clone(),
                        message: detailed_error.error_message.clone(),
                        total_routes: detailed_error.total_backends,
                        healthy_routes: detailed_error.healthy_backends,
                        enabled_routes: detailed_error.enabled_backends,
                        failed_attempts,
                    })
                } else {
                    Err(RouteSelectionError {
                        model_name: model_name.to_string(),
                        message: e.to_string(),
                        total_routes: 0,
                        healthy_routes: 0,
                        enabled_routes: 0,
                        failed_attempts: vec![],
                    })
                }
            }
        }
    }

    async fn select_specific_route(
        &self,
        model_name: &str,
        provider_name: &str,
    ) -> Result<SelectedRoute, RouteSelectionError> {
        match self
            .service
            .select_specific_backend(model_name, provider_name)
            .await
        {
            Ok(selected) => Ok(Self::convert_selected_backend(selected)),
            Err(e) => Err(RouteSelectionError {
                model_name: model_name.to_string(),
                message: e.to_string(),
                total_routes: 0,
                healthy_routes: 0,
                enabled_routes: 0,
                failed_attempts: vec![],
            }),
        }
    }

    async fn report_result(&self, route_id: &str, result: RouteResult) {
        match Self::parse_route_id(route_id) {
            Ok((provider, model)) => {
                let request_result = Self::convert_route_result(result);
                self.service
                    .record_request_result(&provider, &model, request_result)
                    .await;
            }
            Err(e) => {
                tracing::error!("Failed to parse route_id '{}': {}", route_id, e);
            }
        }
    }

    async fn get_route_stats(&self) -> RouteStats {
        let service_health = self.service.get_service_health().await;
        let metrics = self.service.get_metrics();

        let mut route_details = std::collections::HashMap::new();

        // 获取所有请求计数
        let request_counts = metrics.get_all_request_counts();

        for (backend_key, request_count) in request_counts {
            if let Ok((provider, model)) = Self::parse_route_id(&backend_key) {
                let is_healthy = metrics.is_healthy(&provider, &model);
                let error_count = metrics.get_failure_count(&provider, &model) as u64;
                let average_latency = metrics.get_latency(&provider, &model);

                // 尝试获取当前权重（这需要知道模型名称，这里简化处理）
                let current_weight = 1.0; // 简化处理，实际应该从配置中获取

                route_details.insert(
                    backend_key.clone(),
                    RouteDetail {
                        route_id: backend_key,
                        provider,
                        model,
                        is_healthy,
                        request_count,
                        error_count,
                        average_latency,
                        current_weight,
                    },
                );
            }
        }

        RouteStats {
            total_requests: service_health.total_requests,
            successful_requests: service_health.successful_requests,
            route_details,
        }
    }
}
