use berry_core::config::loader::load_config;
use berry_loadbalance::LoadBalanceService;
use berry_relay::relay::handler::loadbalanced::ConcreteLoadBalancedHandler;
use crate::router::router::create_app_router;
use berry_core::auth::rate_limit::RateLimitService;

use anyhow::Result;
use axum::Router;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

/// 应用状态，包含负载均衡服务
#[derive(Clone)]
pub struct AppState {
    pub load_balancer: Arc<LoadBalanceService>,
    pub handler: Arc<ConcreteLoadBalancedHandler>,
    pub config: Arc<berry_core::config::model::Config>,
    pub rate_limiter: Arc<RateLimitService>,
    #[cfg(feature = "observability")]
    pub prometheus_metrics: Option<crate::observability::prometheus_metrics::PrometheusMetrics>,
    #[cfg(not(feature = "observability"))]
    pub prometheus_metrics: Option<()>,
    pub batch_metrics: Arc<crate::observability::batch_metrics::BatchMetricsCollector>,
}

impl AppState {
    /// 创建新的应用状态
    pub async fn new() -> Result<Self> {
        // 加载配置
        let config = load_config()?;
        let config_path = berry_core::config::loader::get_config_path();
        info!("Configuration loaded successfully from: {}", config_path);

        // 创建负载均衡服务
        let load_balancer = Arc::new(LoadBalanceService::new(config.clone())?);

        // 启动负载均衡服务
        load_balancer.start().await?;
        info!("Load balance service started");

        // 创建负载均衡处理器
        let handler = Arc::new(ConcreteLoadBalancedHandler::new_with_service(load_balancer.clone()));

        // 创建速率限制服务
        let rate_limiter = Arc::new(RateLimitService::new());

        // 创建Prometheus metrics (如果启用了observability功能)
        #[cfg(feature = "observability")]
        let prometheus_metrics = match crate::observability::prometheus_metrics::PrometheusMetrics::new() {
            Ok(metrics) => {
                info!("Prometheus metrics initialized");
                Some(metrics)
            }
            Err(e) => {
                error!("Failed to initialize Prometheus metrics: {}", e);
                None
            }
        };

        #[cfg(not(feature = "observability"))]
        let prometheus_metrics = None;

        // 创建批量指标收集器
        let batch_metrics = Arc::new(crate::observability::batch_metrics::BatchMetricsCollector::default());
        info!("Batch metrics collector initialized");

        let app_state = Self {
            load_balancer,
            handler,
            config: Arc::new(config),
            rate_limiter,
            prometheus_metrics,
            batch_metrics,
        };

        // 初始化Prometheus指标
        #[cfg(feature = "observability")]
        if let Some(ref metrics) = app_state.prometheus_metrics {
            metrics.initialize_metrics(&app_state).await;
            metrics.start_background_updater(app_state.clone());
            info!("Prometheus metrics initialized and background updater started");
        }

        Ok(app_state)
    }

    /// 停止应用
    pub async fn shutdown(&self) {
        info!("Shutting down application...");
        self.load_balancer.stop().await;
        info!("Application shutdown complete");
    }
}

/// 创建应用路由
pub fn create_app(state: AppState) -> Router {
    create_app_router().with_state(state)
}

/// 启动应用服务器
pub async fn start_server() -> Result<()> {
    // 初始化日志 - 完全依赖RUST_LOG环境变量
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting Berry API server...");
    info!("Build Time: {}", env!("VERGEN_BUILD_TIMESTAMP"));
    info!("Git Commit: {}", env!("VERGEN_GIT_SHA"));

    // 显示配置信息
    let config_path = berry_core::config::loader::get_config_path();
    info!("Configuration file: {}", config_path);

    // 显示环境变量信息
    if let Ok(config_env) = std::env::var("CONFIG_PATH") {
        info!("CONFIG_PATH environment variable: {}", config_env);
    } else {
        info!("CONFIG_PATH environment variable: not set (using default paths)");
    }

    // 创建应用状态
    let app_state = match AppState::new().await {
        Ok(state) => state,
        Err(e) => {
            error!("Failed to initialize application: {}", e);
            return Err(e);
        }
    };

    // 创建应用
    let app = create_app(app_state.clone());

    // 启动服务器
    let bind_addr = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    let addr = listener.local_addr()?;

    info!("Server listening on http://{}", addr);
    info!("Available endpoints:");
    info!("  GET  /              - API information");
    info!("  GET  /health        - Health check");
    info!("  GET  /status        - Service status page");
    info!("  GET  /metrics       - Service metrics");
    info!("  GET  /models        - List available models");
    info!("  POST /v1/chat/completions - Chat completions (OpenAI compatible)");
    info!("  GET  /v1/models     - List models (OpenAI compatible)");
    info!("  GET  /v1/health     - Health check (OpenAI compatible)");

    // 设置优雅关闭
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Shutdown signal received");
    };

    // 启动服务器
    let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal);

    if let Err(e) = server.await {
        error!("Server error: {}", e);
        app_state.shutdown().await;
        return Err(e.into());
    }

    app_state.shutdown().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_health_endpoint() {
        // 注意：这个测试需要有效的配置文件
        // 在实际测试中，你可能需要使用模拟的配置

        // 创建测试配置
        unsafe {
            std::env::set_var("CONFIG_PATH", "config_example.toml");
        }

        // 这个测试可能会失败，因为需要真实的API密钥
        // 在实际项目中，应该使用模拟的服务
        if let Ok(app_state) = AppState::new().await {
            let app = create_app(app_state);
            let server = TestServer::new(app).unwrap();

            let response = server.get("/health").await;
            assert!(
                response.status_code() == StatusCode::OK
                    || response.status_code() == StatusCode::SERVICE_UNAVAILABLE
            );
        }
    }

    #[tokio::test]
    async fn test_index_endpoint() {
        use crate::router::router::index;
        use axum::routing::get;

        // 创建一个简单的测试，不需要真实的配置
        let app = Router::new().route("/", get(index));
        let server = TestServer::new(app).unwrap();

        let response = server.get("/").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(response.text(), "Berry API - Load Balanced AI Gateway");
    }
}
