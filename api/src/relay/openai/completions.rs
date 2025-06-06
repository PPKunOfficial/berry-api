use crate::relay::openai::OPENAI_API_URL;
use axum::response::Sse;
use axum::{extract::Json, response::IntoResponse};
use axum::response::sse::Event;
use axum_extra::TypedHeader;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};

async fn sse_completions(
    // 提取 Bearer Token 类型的 Authorization 头
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    // 接收 JSON 格式的请求体
    Json(body): Json<Value>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    // 提取 Token 字符串
    let token = authorization.token();
    let auth_header_value = format!("Bearer {}", token);

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Authorization", auth_header_value.parse().unwrap());
    headers.insert("Content-Type", content_type.to_string().parse().unwrap());

    // 创建一个 HTTP 客户端
    let api_client = Client::new();

    // 向远程 API 发起 POST 请求
    let request_result = api_client
        .post(format!("{}/chat/completions", OPENAI_API_URL))
        .headers(headers)
        .json(&body)
        .send()
        .await
        .unwrap()
        .bytes_stream();

    tracing::debug!("{:?}", body);

    let event_stream = request_result.map(|result| {
        match result {
            Ok(bytes) => {
                let data = String::from_utf8_lossy(&bytes.strip_prefix("data: ".as_bytes()).unwrap());
                tracing::debug!("{:?}", data);
                Ok(Event::default().data(data))
            },
            Err(err) => {
                Ok(Event::default().data("error"))
            }
        }
    });
    Sse::new(event_stream)
}
async fn no_sse_completions(
    // 提取 Bearer Token 类型的 Authorization 头
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    // 接收 JSON 格式的请求体
    Json(body): Json<Value>,
) -> Json<Value> {
    let api_client = Client::new();
    let token = authorization.token();
    let auth_header_value = format!("Bearer {}", token);

    let mut headers  = reqwest::header::HeaderMap::new();
    headers.append("Authorization", auth_header_value.parse().unwrap());
    headers.append("Content-Type", content_type.to_string().parse().unwrap());

    let request_result = api_client
        .post(format!("{}/chat/completions", OPENAI_API_URL))
        .headers(headers)
        .json(&body)
        .send()
        .await;
    match request_result {
        Ok(resp) => {
            if resp.status().is_success() {
                let text = resp.text().await.unwrap();
                match serde_json::from_str(&text) {
                    Ok(val) => Json(val),
                    Err(_) => Json(
                        json!({"berry-api-error": text,"错误信息": "上游返回数据格式错误，解析失败"}),
                    ),
                }
            } else {
                let text = resp.text().await.unwrap();
                Json(json!({"berry-api-error": text,"错误信息": "请求上游失败"}))
            }
        }
        Err(e) => Json(json!({"berry-api-error": e.to_string(),"错误信息": "请求上游失败"})),
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
