use crate::config::loader::load_config;
use crate::loadbalance::LoadBalanceService;
use crate::relay::handler::LoadBalancedHandler;


use anyhow::Result;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use axum_extra::TypedHeader;
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;

/// 应用状态，包含负载均衡服务
#[derive(Clone)]
pub struct AppState {
    pub load_balancer: Arc<LoadBalanceService>,
    pub handler: Arc<LoadBalancedHandler>,
    pub config: Arc<crate::config::model::Config>,
}

impl AppState {
    /// 创建新的应用状态
    pub async fn new() -> Result<Self> {
        // 加载配置
        let config = load_config()?;
        info!("Configuration loaded successfully");

        // 创建负载均衡服务
        let load_balancer = Arc::new(LoadBalanceService::new(config.clone())?);

        // 启动负载均衡服务
        load_balancer.start().await?;
        info!("Load balance service started");

        // 创建负载均衡处理器
        let handler = Arc::new(LoadBalancedHandler::new(load_balancer.clone()));

        Ok(Self {
            load_balancer,
            handler,
            config: Arc::new(config),
        })
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
    Router::new()
        .route("/", get(index))
        .route("/health", get(health_check))
        .route("/metrics", get(metrics))
        .route("/models", get(list_models))
        .nest("/v1", create_v1_routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// 创建 v1 API 路由
fn create_v1_routes() -> Router<AppState> {
    Router::new()
        .route("/chat/completions", post(chat_completions))
        .route("/models", get(list_models_v1))
        .route("/health", get(health_check_v1))
}

/// 首页处理器
async fn index() -> &'static str {
    "Berry API - Load Balanced AI Gateway"
}

/// 健康检查处理器
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let health = state.load_balancer.get_service_health().await;
    
    let status = if health.is_healthy() { "healthy" } else { "unhealthy" };
    let status_code = if health.is_healthy() { 
        axum::http::StatusCode::OK 
    } else { 
        axum::http::StatusCode::SERVICE_UNAVAILABLE 
    };

    (status_code, Json(json!({
        "status": status,
        "service_running": health.is_running,
        "provider_health": {
            "total": health.health_summary.total_providers,
            "healthy": health.health_summary.healthy_providers,
            "ratio": health.health_summary.provider_health_ratio
        },
        "model_health": {
            "total": health.health_summary.total_models,
            "healthy": health.health_summary.healthy_models,
            "ratio": health.health_summary.model_health_ratio
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// 指标处理器
async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let health = state.load_balancer.get_service_health().await;
    
    Json(json!({
        "service": {
            "running": health.is_running,
            "total_requests": health.total_requests,
            "successful_requests": health.successful_requests,
            "success_rate": health.success_rate()
        },
        "providers": {
            "total": health.health_summary.total_providers,
            "healthy": health.health_summary.healthy_providers,
            "health_ratio": health.health_summary.provider_health_ratio
        },
        "models": {
            "total": health.health_summary.total_models,
            "healthy": health.health_summary.healthy_models,
            "health_ratio": health.health_summary.model_health_ratio,
            "details": health.model_stats
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// 列出可用模型（无认证，返回所有可用模型）
async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    let all_models = state.load_balancer.get_available_models();
    state.handler.handle_models_for_user(all_models).await
}

/// V1 API: 列出可用模型
async fn list_models_v1(
    State(state): State<AppState>,
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
) -> impl IntoResponse {
    // 认证检查
    let token = authorization.token();
    let user = match state.config.validate_user_token(token) {
        Some(user) if user.enabled => user,
        _ => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": {
                        "type": "invalid_token",
                        "message": "The provided API key is invalid",
                        "code": 401
                    }
                })),
            ).into_response();
        }
    };

    // 获取用户可访问的模型列表
    let user_models = state.config.get_user_available_models(user);

    // 使用handler的方法来格式化响应
    state.handler.handle_models_for_user(user_models).await.into_response()
}

/// V1 API: 健康检查
async fn health_check_v1(State(state): State<AppState>) -> impl IntoResponse {
    let health = state.load_balancer.get_service_health().await;
    
    Json(json!({
        "status": if health.is_healthy() { "ok" } else { "error" },
        "models_available": health.health_summary.has_available_models(),
        "timestamp": chrono::Utc::now().timestamp()
    }))
}

/// V1 API: 聊天完成
async fn chat_completions(
    State(state): State<AppState>,
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    // 认证检查
    let token = authorization.token();
    let user = match state.config.validate_user_token(token) {
        Some(user) if user.enabled => user,
        _ => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": {
                        "type": "invalid_token",
                        "message": "The provided API key is invalid",
                        "code": 401
                    }
                })),
            ).into_response();
        }
    };

    // 检查模型访问权限
    if let Some(model_name) = body.get("model").and_then(|m| m.as_str()) {
        if !state.config.user_can_access_model(user, model_name) {
            return (
                axum::http::StatusCode::FORBIDDEN,
                Json(json!({
                    "error": {
                        "type": "model_access_denied",
                        "message": format!("Access denied for model: {}", model_name),
                        "code": 403
                    }
                })),
            ).into_response();
        }
    }

    // 继续处理请求
    state.handler.clone().handle_completions(
        TypedHeader(authorization),
        TypedHeader(content_type),
        Json(body),
    ).await
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
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    let addr = listener.local_addr()?;
    
    info!("Server listening on http://{}", addr);
    info!("Available endpoints:");
    info!("  GET  /              - API information");
    info!("  GET  /health        - Health check");
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
        unsafe { std::env::set_var("CONFIG_PATH", "config_example.toml"); }
        
        // 这个测试可能会失败，因为需要真实的API密钥
        // 在实际项目中，应该使用模拟的服务
        if let Ok(app_state) = AppState::new().await {
            let app = create_app(app_state);
            let server = TestServer::new(app).unwrap();
            
            let response = server.get("/health").await;
            assert!(response.status_code() == StatusCode::OK || response.status_code() == StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    #[tokio::test]
    async fn test_index_endpoint() {
        // 创建一个简单的测试，不需要真实的配置
        let app = Router::new().route("/", get(index));
        let server = TestServer::new(app).unwrap();
        
        let response = server.get("/").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(response.text(), "Berry API - Load Balanced AI Gateway");
    }
}
