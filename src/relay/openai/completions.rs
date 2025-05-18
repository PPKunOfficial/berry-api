use axum::{
    extract::Json,
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
};
use axum_extra::TypedHeader;
use eventsource_stream::EventStream;
use futures::stream::{self, BoxStream, StreamExt}; // 用于处理异步流
use reqwest::Client; // 用于发起 HTTP 请求
use serde_json::{Value, json}; // 用于解析 JSON 数据
use std::error::Error as StdError; // 用于统一错误类型

use crate::relay::openai::OPENAI_API_URL;

/// SSE 接口函数：将上游 API 的事件流代理给前端客户端
async fn sse_completions(
    // 提取 Bearer Token 类型的 Authorization 头
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    // 接收 JSON 格式的请求体
    Json(body): Json<Value>,
) -> Sse<BoxStream<'static, Result<Event, Box<dyn StdError + Send + Sync + 'static>>>> {
    println!("收到授权头");

    // 提取 Token 字符串
    let token = authorization.token();
    let auth_header_value = format!("Bearer {}", token);
    println!("接收到的请求体 JSON: {}", body);

    // 将 body 转为字符串形式以便作为请求体发送
    let body_json_str = body.to_string();

    // 创建一个 HTTP 客户端
    let api_client = Client::new();

    // 向远程 API 发起 POST 请求
    let request_result = api_client
        .post(format!("{}/chat/completions", OPENAI_API_URL))
        .header("Authorization", auth_header_value)
        .header("Content-Type", content_type.to_string())
        .body(body_json_str)
        .send()
        .await;

    // 构建返回值类型为 BoxStream<Result<Event, Box<dyn StdError>>>
    let stream = match request_result {
        Ok(resp) => {
            if resp.status().is_success() {
                // 如果响应成功，使用 EventStream 包装 bytes_stream 来解析事件流
                EventStream::new(resp.bytes_stream())
                    .map(|parse_result| {
                        match parse_result {
                            Ok(parsed_event) => {
                                // 将 eventsource_stream::ParsedEvent 转换为 axum 的 Event
                                let axum_event = Event::default().data(&parsed_event.data);

                                // 不用过多设置 原汁原味转发即可
                                tracing::trace!(
                                    "转发事件: 传入: {:?} 传出: {:?}",
                                    &parsed_event.data,
                                    &axum_event
                                );
                                Ok(axum_event)
                            }
                            Err(e) => {
                                // 解析失败时返回错误
                                Err(Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
                            }
                        }
                    })
                    .boxed()
            } else {
                // 上游 API 返回非 2xx 状态码
                let status = resp.status();
                // 读取错误信息文本
                let error_text_future = resp.text().await;
                let error_text =
                    error_text_future.unwrap_or_else(|e| format!("无法读取上游错误内容: {}", e));
                let error_message = format!("上游 API 错误: {} - {}", status, error_text);
                eprintln!("{}", error_message);

                // 构造一个错误事件并放入流中
                let error_event = Event::default()
                    .event("error") // 使用 "error" 作为事件类型供前端识别
                    .data(error_message);

                // 构造一个仅包含一次事件的流，并 boxed
                let err_stream: BoxStream<
                    'static,
                    Result<Event, Box<dyn StdError + Send + Sync + 'static>>,
                > = stream::once(async move { Ok(error_event) }).boxed();
                err_stream
            }
        }
        Err(e) => {
            // 网络请求本身失败（如 DNS、连接失败等）
            eprintln!("向上游 API 发送请求失败: {:?}", e);

            // 构造一个只返回错误的流
            stream::once(
                async move { Err(Box::new(e) as Box<dyn StdError + Send + Sync + 'static>) },
            )
            .boxed()
        }
    };

    // 返回 Sse 响应，并设置保持连接活跃（心跳）
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn no_sse_completions(
    // 提取 Bearer Token 类型的 Authorization 头
    TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    TypedHeader(content_type): TypedHeader<headers::ContentType>,
    // 接收 JSON 格式的请求体
    Json(body): Json<Value>,
) -> Json<Value> {
    let api_client = Client::new();
    let body_json_str = body.to_string();
    let token = authorization.token();
    let auth_header_value = format!("Bearer {}", token);
    let request_result = api_client
        .post(format!("{}/chat/completions", OPENAI_API_URL))
        .header("Authorization", auth_header_value)
        .header("Content-Type", content_type.to_string())
        .body(body_json_str)
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
