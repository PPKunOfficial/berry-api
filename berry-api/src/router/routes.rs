use crate::app::AppState;
use crate::middleware::metrics_middleware;
use crate::static_files::{serve_index, serve_static_file};
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use super::{
    admin::{get_backend_health, get_model_weights, get_rate_limit_usage, get_system_stats},
    chat::chat_completions,
    health::{detailed_health_check, simple_health_check},
    metrics::metrics,
    models::{list_models, list_models_v1},
    monitoring::{
        clear_cache, get_model_weights as get_monitoring_model_weights, get_monitoring_info,
        get_performance_metrics, health_check as monitoring_health_check,
    },
    smart_ai::{get_model_smart_ai_weights, get_smart_ai_weights},
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
        .route(
            "/smart-ai/models/{model}/weights",
            get(get_model_smart_ai_weights),
        )
        // 用户管理路由
        .nest("/api", crate::router::users::create_user_routes())
        .nest("/v1", create_v1_routes())
        .nest("/admin", create_admin_routes())
        .nest("/monitoring", create_monitoring_routes())
        // 静态文件路由 - 使用嵌入的文件
        .route("/status", get(serve_index))
        .route("/status/{*path}", get(serve_static_file))
        // 添加指标中间件
        .layer(middleware::from_fn(metrics_middleware))
        .layer(TraceLayer::new_for_http())
}

/// 创建 v1 API 路由
fn create_v1_routes() -> Router<AppState> {
    Router::new()
        .route("/chat/completions", post(chat_completions))
        .route("/models", get(list_models_v1))
        .route("/health", get(simple_health_check))
        // 为v1 API添加CORS支持
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::ACCEPT,
                ])
                .expose_headers([axum::http::header::CONTENT_TYPE]),
        )
}

/// 创建管理 API 路由
fn create_admin_routes() -> Router<AppState> {
    Router::new()
        .route("/model-weights", get(get_model_weights))
        .route("/rate-limit-usage", get(get_rate_limit_usage))
        .route("/backend-health", get(get_backend_health))
        .route("/system-stats", get(get_system_stats))
}

/// 创建监控 API 路由
fn create_monitoring_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_monitoring_info))
        .route("/info", get(get_monitoring_info))
        .route("/model-weights", get(get_monitoring_model_weights))
        .route("/performance", get(get_performance_metrics))
        .route("/health", get(monitoring_health_check))
        .route("/cache/clear", post(clear_cache))
}

/// 首页处理器
pub async fn index() -> &'static str {
    "Berry API - Load Balanced AI Gateway"
}
