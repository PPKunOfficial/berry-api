use crate::relay::client::ClientError;
use axum::response::sse::Event;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::{json, Value};

// 创建错误事件
pub fn create_error_event(error: &ClientError) -> Event {
    Event::default().data(
        json!({
            "error": {
                "message": error.to_string()
            }
        })
        .to_string(),
    )
}

// 创建错误 JSON 响应
pub fn create_error_json(error: &ClientError) -> Value {
    json!({
        "error": {
            "message": error.to_string()
        }
    })
}

// 创建网络错误 JSON 响应
pub fn create_network_error_json(message: &str, details: Option<String>) -> Value {
    json!({
        "error": {
            "message": message,
            "status": Option::<u16>::None,
            "details": details,
        }
    })
}

// 创建上游错误 JSON 响应
pub fn create_upstream_error_json(
    message: &str,
    status: Option<u16>,
    details: Option<String>,
) -> Value {
    json!({
        "error": {
            "message": message,
            "status": status,
            "details": details,
        }
    })
}

/// 错误类型枚举，用于确定HTTP状态码
#[derive(Debug, Clone)]
pub enum ErrorType {
    /// 客户端错误 - 400 Bad Request
    BadRequest,
    /// 认证错误 - 401 Unauthorized
    Unauthorized,
    /// 权限错误 - 403 Forbidden
    Forbidden,
    /// 资源未找到 - 404 Not Found
    NotFound,
    /// 请求超时 - 408 Request Timeout
    RequestTimeout,
    /// 请求过多 - 429 Too Many Requests
    TooManyRequests,
    /// 服务器内部错误 - 500 Internal Server Error
    InternalServerError,
    /// 服务不可用 - 503 Service Unavailable
    ServiceUnavailable,
    /// 网关超时 - 504 Gateway Timeout
    GatewayTimeout,
}

impl ErrorType {
    /// 获取对应的HTTP状态码
    pub fn status_code(&self) -> StatusCode {
        match self {
            ErrorType::BadRequest => StatusCode::BAD_REQUEST,
            ErrorType::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorType::Forbidden => StatusCode::FORBIDDEN,
            ErrorType::NotFound => StatusCode::NOT_FOUND,
            ErrorType::RequestTimeout => StatusCode::REQUEST_TIMEOUT,
            ErrorType::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            ErrorType::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorType::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorType::GatewayTimeout => StatusCode::GATEWAY_TIMEOUT,
        }
    }

    /// 根据错误消息内容推断错误类型
    pub fn from_error_message(message: &str) -> Self {
        let message_lower = message.to_lowercase();

        if message_lower.contains("unauthorized")
            || message_lower.contains("invalid token")
            || message_lower.contains("authentication")
        {
            ErrorType::Unauthorized
        } else if message_lower.contains("forbidden")
            || message_lower.contains("permission")
            || message_lower.contains("access denied")
        {
            ErrorType::Forbidden
        } else if message_lower.contains("not found")
            || message_lower.contains("model") && message_lower.contains("not")
        {
            ErrorType::NotFound
        } else if message_lower.contains("timeout") || message_lower.contains("timed out") {
            ErrorType::GatewayTimeout
        } else if message_lower.contains("too many requests")
            || message_lower.contains("rate limit")
        {
            ErrorType::TooManyRequests
        } else if message_lower.contains("service unavailable")
            || message_lower.contains("no available backends")
            || message_lower.contains("all") && message_lower.contains("unhealthy")
        {
            ErrorType::ServiceUnavailable
        } else if message_lower.contains("bad request")
            || message_lower.contains("invalid") && !message_lower.contains("token")
        {
            ErrorType::BadRequest
        } else {
            ErrorType::InternalServerError
        }
    }
}

/// 创建带有正确HTTP状态码的错误响应
pub fn create_error_response(
    error_type: ErrorType,
    message: &str,
    details: Option<String>,
) -> impl IntoResponse {
    let status_code = error_type.status_code();
    let error_json = json!({
        "error": {
            "message": message,
            "type": format!("{:?}", error_type),
            "status": status_code.as_u16(),
            "details": details,
        }
    });

    (status_code, Json(error_json))
}

/// 创建带有正确HTTP状态码的错误响应（从ClientError）
pub fn create_client_error_response(error: &ClientError) -> impl IntoResponse {
    let message = error.to_string();
    let error_type = ErrorType::from_error_message(&message);

    let status_code = error_type.status_code();
    let error_json = json!({
        "error": {
            "message": message,
            "type": format!("{:?}", error_type),
            "status": status_code.as_u16(),
            "details": Option::<String>::None,
        }
    });

    (status_code, Json(error_json))
}

/// 创建统一的流式错误响应
pub fn create_streaming_error_response(
    error_type: ErrorType,
    message: &str,
    details: Option<String>,
) -> impl IntoResponse {
    use axum::response::Sse;
    use futures::stream;

    let status_code = error_type.status_code();

    let error_event = json!({
        "error": {
            "message": message,
            "type": format!("{:?}", error_type),
            "code": status_code.as_u16(),
            "details": details
        }
    });

    // 创建包含错误信息的SSE流
    let error_stream = stream::once(async move {
        Ok::<Event, std::convert::Infallible>(
            Event::default()
                .event("error")
                .data(error_event.to_string()),
        )
    });

    // 返回带有正确HTTP状态码的SSE响应
    (
        status_code,
        Sse::new(Box::pin(error_stream)
            as futures::stream::BoxStream<
                'static,
                Result<Event, std::convert::Infallible>,
            >),
    )
        .into_response()
}

/// 创建服务不可用错误响应
pub fn create_service_unavailable_response(
    message: &str,
    details: Option<String>,
) -> impl IntoResponse {
    create_error_response(ErrorType::ServiceUnavailable, message, details)
}

/// 创建内部服务器错误响应
pub fn create_internal_error_response(message: &str, details: Option<String>) -> impl IntoResponse {
    create_error_response(ErrorType::InternalServerError, message, details)
}

/// 创建网关超时错误响应
pub fn create_gateway_timeout_response(
    message: &str,
    details: Option<String>,
) -> impl IntoResponse {
    create_error_response(ErrorType::GatewayTimeout, message, details)
}

/// 统一的错误处理器
/// 提供一站式的错误处理功能，包括错误分类、响应创建、日志记录等
pub struct ErrorHandler;

impl ErrorHandler {
    /// 从anyhow::Error创建HTTP响应
    pub fn from_anyhow_error(error: &anyhow::Error, context: Option<&str>) -> impl IntoResponse {
        let error_str = error.to_string();
        let error_type = ErrorType::from_error_message(&error_str);

        let message = if let Some(ctx) = context {
            format!(
                "{}: {}",
                ctx,
                Self::extract_user_friendly_message(&error_str)
            )
        } else {
            Self::extract_user_friendly_message(&error_str)
        };

        // 记录错误日志
        tracing::error!("Error occurred: {} - Details: {}", message, error_str);

        create_error_response(error_type, &message, Some(error_str))
    }

    /// 从HTTP状态码和响应体创建错误响应
    pub fn from_http_error(
        status_code: u16,
        response_body: &str,
        context: Option<&str>,
    ) -> impl IntoResponse {
        let error_type = Self::status_code_to_error_type(status_code);
        let backend_error = Self::parse_backend_error_message(response_body);

        let message = if let Some(ctx) = context {
            format!("{}: {}", ctx, backend_error.message)
        } else {
            backend_error.message
        };

        // 记录错误日志
        tracing::debug!(
            "HTTP error {}: {} - Body: {}",
            status_code,
            message,
            response_body
        );

        create_error_response(
            error_type,
            &message,
            Some(format!("Backend error: {}", backend_error.details)),
        )
    }

    /// 创建业务逻辑错误响应
    pub fn business_error(
        error_type: ErrorType,
        message: &str,
        details: Option<String>,
    ) -> impl IntoResponse {
        tracing::warn!("Business error: {} - Details: {:?}", message, details);
        create_error_response(error_type, message, details)
    }

    /// 创建配置错误响应
    pub fn config_error(message: &str, details: Option<String>) -> impl IntoResponse {
        tracing::error!("Configuration error: {} - Details: {:?}", message, details);
        create_error_response(ErrorType::InternalServerError, message, details)
    }

    /// 创建认证错误响应
    pub fn auth_error(message: &str, error_type: Option<ErrorType>) -> impl IntoResponse {
        let err_type = error_type.unwrap_or(ErrorType::Unauthorized);
        tracing::warn!("Authentication error: {}", message);
        create_error_response(err_type, message, None)
    }

    /// 从anyhow::Error创建流式HTTP响应
    pub fn streaming_from_anyhow_error(
        error: &anyhow::Error,
        context: Option<&str>,
    ) -> impl IntoResponse {
        let error_str = error.to_string();
        let error_type = ErrorType::from_error_message(&error_str);

        let message = if let Some(ctx) = context {
            format!(
                "{}: {}",
                ctx,
                Self::extract_user_friendly_message(&error_str)
            )
        } else {
            Self::extract_user_friendly_message(&error_str)
        };

        // 记录错误日志
        tracing::error!(
            "Streaming error occurred: {} - Details: {}",
            message,
            error_str
        );

        create_streaming_error_response(error_type, &message, Some(error_str))
    }

    /// 从HTTP状态码和响应体创建流式错误响应
    pub fn streaming_from_http_error(
        status_code: u16,
        response_body: &str,
        context: Option<&str>,
    ) -> impl IntoResponse {
        let error_type = Self::status_code_to_error_type(status_code);
        let backend_error = Self::parse_backend_error_message(response_body);

        let message = if let Some(ctx) = context {
            format!("{}: {}", ctx, backend_error.message)
        } else {
            backend_error.message
        };

        // 记录错误日志
        tracing::debug!(
            "Streaming HTTP error {}: {} - Body: {}",
            status_code,
            message,
            response_body
        );

        create_streaming_error_response(
            error_type,
            &message,
            Some(format!("Backend error: {}", backend_error.details)),
        )
    }

    /// 创建后端不可用错误响应
    pub fn backend_unavailable(model_name: &str, details: Option<String>) -> impl IntoResponse {
        let message = format!("Service temporarily unavailable for model '{model_name}'");
        tracing::error!("Backend unavailable: {} - Details: {:?}", message, details);
        create_error_response(ErrorType::ServiceUnavailable, &message, details)
    }

    /// 从状态码映射到错误类型
    fn status_code_to_error_type(status_code: u16) -> ErrorType {
        match status_code {
            400 => ErrorType::BadRequest,
            401 => ErrorType::Unauthorized,
            403 => ErrorType::Forbidden,
            404 => ErrorType::NotFound,
            408 => ErrorType::RequestTimeout,
            429 => ErrorType::TooManyRequests,
            503 => ErrorType::ServiceUnavailable,
            504 => ErrorType::GatewayTimeout,
            _ => ErrorType::InternalServerError,
        }
    }

    /// 解析后端错误消息
    fn parse_backend_error_message(response_body: &str) -> BackendErrorInfo {
        // 尝试解析JSON格式的错误
        if let Ok(json_error) = serde_json::from_str::<serde_json::Value>(response_body) {
            if let Some(error_obj) = json_error.get("error") {
                if let Some(message) = error_obj.get("message").and_then(|m| m.as_str()) {
                    return BackendErrorInfo {
                        message: message.to_string(),
                        details: response_body.to_string(),
                    };
                }
            }
        }

        // 如果不是JSON或解析失败，使用原始内容
        let message = if response_body.len() > 100 {
            format!("{}...", &response_body[..100])
        } else {
            response_body.to_string()
        };

        BackendErrorInfo {
            message: if message.is_empty() {
                "Unknown error".to_string()
            } else {
                message
            },
            details: response_body.to_string(),
        }
    }

    /// 提取用户友好的错误消息
    fn extract_user_friendly_message(error_str: &str) -> String {
        // 移除技术细节，提取用户可理解的部分
        if error_str.contains("Backend selection failed") {
            "No available backends for this model".to_string()
        } else if error_str.contains("API key") {
            "API key configuration error".to_string()
        } else if error_str.contains("timeout") || error_str.contains("timed out") {
            "Request timeout".to_string()
        } else if error_str.contains("HTTP error") {
            // 提取HTTP错误的具体信息
            if let Some(pos) = error_str.find("HTTP error ") {
                if let Some(colon_pos) = error_str[pos..].find(": ") {
                    let start = pos + colon_pos + 2;
                    if start < error_str.len() {
                        return error_str[start..].trim().to_string();
                    }
                }
            }
            "Request failed".to_string()
        } else {
            // 截取前100个字符作为用户友好消息
            if error_str.len() > 100 {
                format!("{}...", &error_str[..100])
            } else {
                error_str.to_string()
            }
        }
    }
}

/// 后端错误信息结构
#[derive(Debug, Clone)]
pub struct BackendErrorInfo {
    pub message: String,
    pub details: String,
}

/// 错误记录工具
pub struct ErrorRecorder;

impl ErrorRecorder {
    /// 记录请求失败到负载均衡器
    pub async fn record_request_failure<T: berry_loadbalance::loadbalance::LoadBalancer>(
        load_balancer: &T,
        provider: &str,
        model: &str,
        error: &anyhow::Error,
    ) {
        load_balancer
            .record_request_result(
                provider,
                model,
                berry_loadbalance::RequestResult::Failure {
                    error: error.to_string(),
                },
            )
            .await;
    }

    /// 记录请求失败（字符串错误）
    pub async fn record_failure_with_message<T: berry_loadbalance::loadbalance::LoadBalancer>(
        load_balancer: &T,
        provider: &str,
        model: &str,
        error_message: String,
    ) {
        load_balancer
            .record_request_result(
                provider,
                model,
                berry_loadbalance::RequestResult::Failure {
                    error: error_message,
                },
            )
            .await;
    }

    /// 记录HTTP错误失败
    pub async fn record_http_failure<T: berry_loadbalance::loadbalance::LoadBalancer>(
        load_balancer: &T,
        provider: &str,
        model: &str,
        status_code: u16,
        response_body: &str,
    ) {
        let error_message = format!("HTTP {status_code} - {response_body}");
        Self::record_failure_with_message(load_balancer, provider, model, error_message).await;
    }
}

/// 重试错误处理器
pub struct RetryErrorHandler;

impl RetryErrorHandler {
    /// 处理重试过程中的错误
    pub fn handle_retry_error(
        attempt: usize,
        max_retries: usize,
        error: &anyhow::Error,
        context: &str,
    ) -> Result<(), anyhow::Error> {
        if attempt == max_retries - 1 {
            // 最后一次重试失败，返回错误
            Err(anyhow::anyhow!(
                "{} after {} attempts: {}",
                context,
                max_retries,
                error
            ))
        } else {
            // 记录警告并继续重试
            tracing::warn!(
                "{} on attempt {}, retrying: {}",
                context,
                attempt + 1,
                error
            );
            Ok(())
        }
    }

    /// 创建重试失败的最终错误
    pub fn create_final_error(
        context: &str,
        max_retries: usize,
        last_error: &anyhow::Error,
    ) -> anyhow::Error {
        anyhow::anyhow!(
            "{} failed after {} attempts. Last error: {}. All available backends may be experiencing issues.",
            context,
            max_retries,
            last_error
        )
    }
}

/// 响应体读取错误处理器
pub struct ResponseBodyHandler;

impl ResponseBodyHandler {
    /// 安全地读取响应体，处理读取失败的情况
    pub async fn read_error_body(response: reqwest::Response) -> (u16, String) {
        let status = response.status().as_u16();
        let body = match response.text().await {
            Ok(body) => body,
            Err(e) => {
                tracing::warn!("Failed to read error response body: {}", e);
                "Failed to read error response".to_string()
            }
        };
        (status, body)
    }

    /// 读取响应体并记录调试信息
    pub async fn read_and_log_error_body(
        response: reqwest::Response,
        request_type: &str,
    ) -> (u16, String) {
        let (status, body) = Self::read_error_body(response).await;
        tracing::debug!(
            "{} failed with status: {}, body: {}",
            request_type,
            status,
            body
        );
        (status, body)
    }
}
