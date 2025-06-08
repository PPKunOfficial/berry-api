use crate::app::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use tower_http::{services::ServeDir, trace::TraceLayer};

use super::{
    chat::chat_completions,
    health::{detailed_health_check, simple_health_check},
    metrics::metrics,
    models::{list_models, list_models_v1},
};

/// 创建应用路由
pub fn create_app_router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/health", get(detailed_health_check))
        .route("/metrics", get(metrics))
        .route("/models", get(list_models))
        .nest("/v1", create_v1_routes())
        .nest_service("/status", ServeDir::new("public"))
        .layer(TraceLayer::new_for_http())
}

/// 创建 v1 API 路由
fn create_v1_routes() -> Router<AppState> {
    Router::new()
        .route("/chat/completions", post(chat_completions))
        .route("/models", get(list_models_v1))
        .route("/health", get(simple_health_check))
}

/// 首页处理器
pub async fn index() -> &'static str {
    "Berry API - Load Balanced AI Gateway"
}
