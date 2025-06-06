use crate::relay::openai::OPENAI_API_URL;
use axum::response::sse::Event;
use axum::response::{ErrorResponse, Sse};
use axum::{extract::Json, response::IntoResponse};
use axum_extra::TypedHeader;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{Value, json};
use thiserror::Error;

// 定义错误类型
#[derive(Error, Debug)]
pub enum CompletionError {
    #[error("请求头解析失败: {0}")]
    HeaderParseError(String),
    #[error("HTTP请求失败: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("JSON解析失败: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("上游API返回错误: 状态码 {status}")]
    UpstreamError { status: u16, body: String },
}
fn build_request_headers(
    authorization: &headers::Authorization<headers::authorization::Bearer>,
    content_type: &headers::ContentType,
) -> Result<reqwest::header::HeaderMap, CompletionError> {
    let mut headers = reqwest::header::HeaderMap::new();

    let auth_value = format!("Bearer {}", authorization.token())
        .parse()
        .map_err(|e| CompletionError::HeaderParseError(format!("Authorization header: {}", e)))?;

    let content_type_value = content_type
        .to_string()
        .parse()
        .map_err(|e| CompletionError::HeaderParseError(format!("Content-Type header: {}", e)))?;

    headers.insert("Authorization", auth_value);
    headers.insert("Content-Type", content_type_value);

    Ok(headers)
}
// 统一的API请求函数
async fn make_api_request(
    headers: reqwest::header::HeaderMap,
    body: &Value,
) -> Result<reqwest::Response, CompletionError> {
    let client = Client::new();
    let response = client
        .post(format!("{}/chat/completions", OPENAI_API_URL))
        .headers(headers)
        .json(body)
        .send()
        .await?;

    Ok(response)
}
// 创建错误事件
fn create_error_event(error: &CompletionError) -> Event {
    let _error_response = match error {
        CompletionError::UpstreamError { status, body } => ErrorResponse::from(
            axum::http::Response::builder()
                .status(*status)
                .body(axum::body::Body::from(body.clone()))
                .unwrap(),
        ),
        _ => ErrorResponse::from(
            axum::http::Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::from(error.to_string()))
                .unwrap(),
        ),
    };

    Event::default().data(
        json!({
            "error": {
                "message": error.to_string()
            }
        })
        .to_string(),
    )
}
async fn sse_completions(
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    Json(body): Json<Value>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let api_client = Client::new();

    let event_stream = async move {
        // 构建请求头
        let headers = match build_request_headers(&authorization, &content_type) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("构建请求头失败: {:?}", e);
                return futures::stream::once(async move { Ok(create_error_event(&e)) }).boxed();
            }
        };

        let request_result = api_client
            .post(format!("{}/chat/completions", OPENAI_API_URL))
            .headers(headers)
            .json(&body)
            .send()
            .await;
        match request_result {
            Ok(resp) if resp.status().is_success() => {
                // 成功情况
                resp.bytes_stream()
                    .eventsource()
                    .map(|result| match result {
                        Ok(event) => {
                            tracing::debug!("SSE event: {:?}", event.data);
                            Ok(Event::default().data(event.data))
                        }
                        Err(err) => {
                            tracing::error!("SSE error: {:?}", err);
                            Ok(
                                Event::default()
                                    .data(json!({"error": err.to_string()}).to_string()),
                            )
                        }
                    })
                    .boxed() // 统一类型
            }
            Ok(resp) => {
                // HTTP 错误
                tracing::error!("请求上游失败: {}", resp.status());
                futures::stream::once(async move {
                    Ok(Event::default().data(
                        json!({
                            "error": {
                                "message": "请求上游失败",
                                "status": resp.status().as_u16()
                            }
                        })
                        .to_string(),
                    ))
                })
                .boxed()
            }
            Err(e) => {
                // 网络错误
                tracing::error!("请求异常: {:?}", e);
                futures::stream::once(async move {
                    Ok(Event::default().data(
                        json!({
                            "error": {
                                "message": "请求异常",
                                "details": e.to_string()
                            }
                        })
                        .to_string(),
                    ))
                })
                .boxed()
            }
        }
    }
    .await;
    Sse::new(event_stream)
}

async fn no_sse_completions(
    // 提取 Bearer Token 类型的 Authorization 头
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    // 接收 JSON 格式的请求体
    Json(body): Json<Value>,
) -> Json<Value> {

    // 构建请求头
    let headers = match build_request_headers(&authorization, &content_type) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("构建请求头失败: {:?}", e);
            return Json(json!({
                "error": {
                    "message": e.to_string()
                }
            }));
        }
    };

    // 发送API请求
    let response = match make_api_request(headers, &body).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("API请求失败: {:?}", e);
            return Json(json!({
                "error":  {
                    "message": e.to_string(),
                    "status": Option::<u16>::None,
                    "details": Option::<String>::None,
                }
            }));
        }
    };

    // 处理响应
    if response.status().is_success() {
        match response.text().await {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(value) => Json(value),
                Err(e) => {
                    tracing::error!("JSON解析失败: {:?}", e);
                    Json(json!( {
                        "error":  {
                            "message": "上游返回数据格式错误".to_string(),
                            "status": Option::<u16>::None,
                            "details": Some(text),
                        }
                    }))
                }
            },
            Err(e) => {
                tracing::error!("读取响应体失败: {:?}", e);
                Json(json!( {
                    "error":  {
                        "message": "无法读取响应数据".to_string(),
                        "status": Option::<u16>::None,
                        "details": Some(e.to_string()),
                    }
                }))
            }
        }
    } else {
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "无法读取错误响应".to_string());

        tracing::error!("上游API错误: 状态码 {}, 响应: {}", status, body);
        Json(json!( {
            "error": {
                "message": "上游API返回错误".to_string(),
                "status": Some(status),
                "details": Some(body),
            }
        }))
    }
}
pub async fn handle_completions(
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let is_stream = body
        .get("stream")
        .unwrap_or(&Value::Bool(true))
        .as_bool()
        .unwrap_or(true);

    if is_stream {
        sse_completions(
            TypedHeader(authorization),
            TypedHeader(content_type),
            Json(body),
        )
        .await
        .into_response()
    } else {
        no_sse_completions(
            TypedHeader(authorization),
            TypedHeader(content_type),
            Json(body),
        )
        .await
        .into_response()
    }
}
