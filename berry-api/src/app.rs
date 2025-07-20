use crate::router::routes::create_app_router;
use berry_core::auth::rate_limit::RateLimitService;
use berry_core::config::loader::load_config;
use berry_loadbalance::LoadBalanceService;
use berry_relay::relay::handler::loadbalanced::ConcreteLoadBalancedHandler;

use anyhow::Result;
use axum::Router;
use std::future::IntoFuture;
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
        let handler = Arc::new(ConcreteLoadBalancedHandler::new_with_service(
            load_balancer.clone(),
        ));

        // 创建速率限制服务
        let rate_limiter = Arc::new(RateLimitService::new());

        // 创建批量指标收集器
        let batch_metrics = Arc::new(
            crate::observability::batch_metrics::BatchMetricsCollector::with_default_config(),
        );
        info!("Batch metrics collector initialized");

        let app_state = Self {
            load_balancer,
            handler,
            config: Arc::new(config),
            rate_limiter,
            batch_metrics,
        };

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

use tokio::task::JoinHandle;

/// 启动应用服务器并返回监听地址和服务器句柄
pub async fn start_server() -> Result<(
    std::net::SocketAddr,
    JoinHandle<Result<(), std::io::Error>>,
    AppState,
)> {
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

    // 返回监听地址、服务器句柄和应用状态
    let server = tokio::spawn(axum::serve(listener, app).into_future());
    Ok((addr, server, app_state))
}
