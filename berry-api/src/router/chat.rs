use crate::app::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use axum_extra::TypedHeader;
use serde_json::{json, Value};
use std::time::Instant;

/// V1 API: 聊天完成
pub async fn chat_completions(
    State(state): State<AppState>,
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    let start = Instant::now();
    let method = "POST";
    let endpoint = "/v1/chat/completions";

    // 记录正在处理的请求
    #[cfg(feature = "observability")]
    if let Some(ref metrics) = state.prometheus_metrics {
        metrics
            .http_requests_in_flight
            .with_label_values(&[method, endpoint])
            .inc();
    }
    // 认证检查
    let token = authorization.token();
    let user = match state.config.validate_user_token(token) {
        Some(user) if user.enabled => user,
        _ => {
            let response = (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": {
                        "type": "invalid_token",
                        "message": "The provided API key is invalid",
                        "code": 401
                    }
                })),
            )
                .into_response();

            record_request_metrics(&state, method, endpoint, start, &response);
            return response;
        }
    };

    // 检查速率限制
    if let Some(rate_limit) = &user.rate_limit {
        if let Err(e) = state.rate_limiter.check_rate_limit(token, rate_limit).await {
            let response = (
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": {
                        "type": "rate_limit_exceeded",
                        "message": format!("Rate limit exceeded: {}", e),
                        "code": 429
                    }
                })),
            )
                .into_response();

            record_request_metrics(&state, method, endpoint, start, &response);
            return response;
        }
    }

    // 检查模型访问权限
    if let Some(model_name) = body.get("model").and_then(|m| m.as_str()) {
        if !state.config.user_can_access_model(user, model_name) {
            let response = (
                axum::http::StatusCode::FORBIDDEN,
                Json(json!({
                    "error": {
                        "type": "model_access_denied",
                        "message": format!("Access denied for model: {}", model_name),
                        "code": 403
                    }
                })),
            )
                .into_response();

            record_request_metrics(&state, method, endpoint, start, &response);
            return response;
        }
    }

    // 继续处理请求（传递用户标签）
    let user_tags = if user.tags.is_empty() {
        None
    } else {
        Some(user.tags.as_slice())
    };
    let response = state
        .handler
        .clone()
        .handle_completions_with_user_tags(
            TypedHeader(authorization),
            TypedHeader(content_type),
            Json(body),
            user_tags,
        )
        .await;

    // 记录指标
    record_request_metrics(&state, method, endpoint, start, &response);

    response
}

/// 记录请求指标的辅助函数
fn record_request_metrics(
    state: &AppState,
    method: &str,
    endpoint: &str,
    start: Instant,
    response: &axum::response::Response,
) {
    #[cfg(feature = "observability")]
    {
        let latency = start.elapsed();
        let status = response.status().to_string();

        if let Some(ref metrics) = state.prometheus_metrics {
            // 减少正在处理的请求计数
            metrics
                .http_requests_in_flight
                .with_label_values(&[method, endpoint])
                .dec();

            // 记录请求总数
            metrics
                .http_requests_total
                .with_label_values(&[method, endpoint, &status])
                .inc();

            // 记录请求延迟
            metrics
                .http_request_duration_seconds
                .with_label_values(&[method, endpoint])
                .observe(latency.as_secs_f64());
        }
    }

    #[cfg(not(feature = "observability"))]
    {
        // 避免未使用变量警告
        let _ = (state, method, endpoint, start, response);
    }
}
