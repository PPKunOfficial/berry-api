use anyhow::Result;
use std::time::Duration;

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

impl Default for RouteStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            route_details: std::collections::HashMap::new(),
        }
    }
}
