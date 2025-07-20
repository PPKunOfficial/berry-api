use crate::app::AppState;
use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;

/// HTTP请求指标中间件
///
/// 使用批量指标收集器记录HTTP请求的指标信息
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let start_time = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    // 从请求扩展中获取应用状态
    let state = request.extensions().get::<AppState>().cloned();

    // 处理请求
    let response = next.run(request).await;

    // 记录指标
    let duration = start_time.elapsed();
    let status_code = response.status().as_u16();

    // 如果有状态，记录指标
    if let Some(state) = state {
        // 使用批量指标收集器记录HTTP请求
        state
            .batch_metrics
            .record_http_request(&method, &path, status_code, duration);
    }

    response
}

/// 后端请求指标记录辅助函数
///
/// 在LoadBalancedHandler中调用，记录后端请求的指标
pub fn record_backend_request_metrics(
    state: &AppState,
    provider: &str,
    model: &str,
    success: bool,
    latency: std::time::Duration,
    error_type: Option<&str>,
) {
    // 使用批量指标收集器记录后端请求
    state
        .batch_metrics
        .record_backend_request(provider, model, success, latency, error_type);
}

/// 健康检查指标记录辅助函数
pub fn record_health_check_metrics(
    state: &AppState,
    backend_key: &str,
    healthy: bool,
    check_duration: std::time::Duration,
) {
    // 使用批量指标收集器记录健康检查
    state
        .batch_metrics
        .record_health_check(backend_key, healthy, check_duration);
}

/// 缓存指标记录辅助函数
pub fn record_cache_metrics(
    state: &AppState,
    cache_type: &str,
    operation: &str, // "hit", "miss", "eviction"
) {
    // 使用批量指标收集器记录缓存指标
    state
        .batch_metrics
        .record_cache_metric(cache_type, operation);
}
