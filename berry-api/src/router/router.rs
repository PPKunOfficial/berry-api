use crate::app::AppState;
use crate::static_files::{serve_index, serve_static_file};
use axum::{
    Router,
    routing::{get, post},
};
use tower_http::trace::TraceLayer;

use super::{
    chat::chat_completions,
    health::{detailed_health_check, simple_health_check},
    metrics::metrics,
    models::{list_models, list_models_v1},
    smart_ai::{get_smart_ai_weights, get_model_smart_ai_weights},
    admin::{get_model_weights, get_rate_limit_usage, get_backend_health, get_system_stats},
};
use crate::observability::prometheus_metrics::prometheus_metrics_handler;

/// 创建应用路由
pub fn create_app_router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/health", get(detailed_health_check))
        .route("/metrics", get(metrics))
        .route("/prometheus", get(prometheus_metrics_handler))
        .route("/models", get(list_models))
        .route("/smart-ai/weights", get(get_smart_ai_weights))
        .route("/smart-ai/models/{model}/weights", get(get_model_smart_ai_weights))
        .nest("/v1", create_v1_routes())
        .nest("/admin", create_admin_routes())
        // 静态文件路由 - 使用嵌入的文件
        .route("/status", get(serve_index))
        .route("/status/{*path}", get(serve_static_file))
        .layer(TraceLayer::new_for_http())
}

/// 创建 v1 API 路由
fn create_v1_routes() -> Router<AppState> {
    Router::new()
        .route("/chat/completions", post(chat_completions))
        .route("/models", get(list_models_v1))
        .route("/health", get(simple_health_check))
}

/// 创建管理 API 路由
fn create_admin_routes() -> Router<AppState> {
    Router::new()
        .route("/model-weights", get(get_model_weights))
        .route("/rate-limit-usage", get(get_rate_limit_usage))
        .route("/backend-health", get(get_backend_health))
        .route("/system-stats", get(get_system_stats))
}

/// 首页处理器
pub async fn index() -> &'static str {
    "Berry API - Load Balanced AI Gateway"
}
