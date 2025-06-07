use axum::response::sse::Event;
use axum::response::Sse;
use axum::{extract::Json, response::IntoResponse};
use axum_extra::TypedHeader;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;

use crate::loadbalance::{LoadBalanceService, RequestResult};
use crate::relay::client::openai::OpenAIClient;

use super::types::{create_error_json, create_network_error_json};

/// 负载均衡的OpenAI兼容处理器
pub struct LoadBalancedHandler {
    load_balancer: std::sync::Arc<LoadBalanceService>,
}

impl LoadBalancedHandler {
    pub fn new(load_balancer: std::sync::Arc<LoadBalanceService>) -> Self {
        Self { load_balancer }
    }

    /// 处理聊天完成请求（支持负载均衡）
    pub async fn handle_completions(
        self: Arc<Self>,
        TypedHeader(authorization): TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
        TypedHeader(content_type): TypedHeader<headers::ContentType>,
        Json(mut body): Json<Value>,
    ) -> axum::response::Response {
        let start_time = Instant::now();
        
        // 从请求体中提取模型名称
        let model_name = match body.get("model").and_then(|m| m.as_str()) {
            Some(name) => name,
            None => {
                tracing::error!("Missing model field in request");
                return Json(create_error_json(&crate::relay::client::ClientError::HeaderParseError(
                    "Missing model field in request".to_string()
                ))).into_response();
            }
        };

        // 使用负载均衡器选择后端
        let selected_backend = match self.load_balancer.select_backend(model_name).await {
            Ok(backend) => backend,
            Err(e) => {
                tracing::error!("Failed to select backend for model '{}': {}", model_name, e);
                return Json(create_error_json(&crate::relay::client::ClientError::HeaderParseError(
                    format!("No available backend for model '{}': {}", model_name, e)
                ))).into_response();
            }
        };

        tracing::info!(
            "Selected backend for model '{}': provider='{}', model='{}', selection_time={}ms",
            model_name,
            selected_backend.backend.provider,
            selected_backend.backend.model,
            selected_backend.selection_time.as_millis()
        );

        // 更新请求体中的模型名称为后端的真实模型名称
        body["model"] = Value::String(selected_backend.backend.model.clone());

        // 获取API密钥
        let api_key = match selected_backend.get_api_key() {
            Ok(key) => key,
            Err(e) => {
                tracing::error!("Failed to get API key: {}", e);
                self.load_balancer.record_request_result(
                    &selected_backend.backend.provider,
                    &selected_backend.backend.model,
                    RequestResult::Failure { error: e.to_string() },
                ).await;
                return Json(create_error_json(&crate::relay::client::ClientError::HeaderParseError(
                    "API key not found".to_string()
                ))).into_response();
            }
        };

        // 创建客户端
        let client = OpenAIClient::with_base_url(selected_backend.provider.base_url.clone());

        // 构建请求头
        let headers = match client.build_request_headers(&authorization, &content_type) {
            Ok(mut h) => {
                // 使用选中后端的API密钥
                h.insert("Authorization", format!("Bearer {}", api_key).parse().unwrap());
                
                // 添加自定义头部
                for (key, value) in selected_backend.get_headers() {
                    if let (Ok(header_name), Ok(header_value)) = (
                        key.parse::<reqwest::header::HeaderName>(),
                        value.parse::<reqwest::header::HeaderValue>()
                    ) {
                        h.insert(header_name, header_value);
                    }
                }
                h
            }
            Err(e) => {
                tracing::error!("Failed to build request headers: {:?}", e);
                self.load_balancer.record_request_result(
                    &selected_backend.backend.provider,
                    &selected_backend.backend.model,
                    RequestResult::Failure { error: e.to_string() },
                ).await;
                return Json(create_error_json(&e)).into_response();
            }
        };

        // 检查是否为流式请求
        let is_stream = body
            .get("stream")
            .unwrap_or(&Value::Bool(false))
            .as_bool()
            .unwrap_or(false);

        if is_stream {
            self.handle_streaming_request(client, headers, body, selected_backend, start_time).await.into_response()
        } else {
            self.handle_non_streaming_request(client, headers, body, selected_backend, start_time).await.into_response()
        }
    }

    /// 处理流式请求
    async fn handle_streaming_request(
        &self,
        client: OpenAIClient,
        headers: reqwest::header::HeaderMap,
        body: Value,
        selected_backend: crate::loadbalance::SelectedBackend,
        start_time: Instant,
    ) -> Sse<futures::stream::BoxStream<'static, Result<Event, std::convert::Infallible>>> {
        let load_balancer = self.load_balancer.clone();
        let provider = selected_backend.backend.provider.clone();
        let model = selected_backend.backend.model.clone();

        let request_result = client.chat_completions(headers, &body).await;

        let stream = match request_result {
            Ok(resp) if resp.status().is_success() => {
                let latency = start_time.elapsed();

                // 记录成功
                let lb_clone = load_balancer.clone();
                let provider_clone = provider.clone();
                let model_clone = model.clone();
                tokio::spawn(async move {
                    lb_clone.record_request_result(
                        &provider_clone,
                        &model_clone,
                        RequestResult::Success { latency },
                    ).await;
                });

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
                            Ok(Event::default().data(json!({"error": err.to_string()}).to_string()))
                        }
                    })
                    .boxed()
            }
            Ok(resp) => {
                // HTTP 错误
                let status = resp.status();
                tracing::error!("Request to upstream failed: {}", status);

                // 记录失败
                let lb_clone = load_balancer.clone();
                let provider_clone = provider.clone();
                let model_clone = model.clone();
                tokio::spawn(async move {
                    lb_clone.record_request_result(
                        &provider_clone,
                        &model_clone,
                        RequestResult::Failure { error: format!("HTTP {}", status) },
                    ).await;
                });

                futures::stream::once(async move {
                    Ok(Event::default().data(
                        json!({
                            "error": {
                                "message": "Request to upstream failed",
                                "status": status.as_u16()
                            }
                        }).to_string(),
                    ))
                }).boxed()
            }
            Err(e) => {
                // 网络错误
                let error_msg = e.to_string();
                tracing::error!("Network error: {:?}", error_msg);

                // 记录失败
                let lb_clone = load_balancer.clone();
                let provider_clone = provider.clone();
                let model_clone = model.clone();
                let error_msg_clone = error_msg.clone();
                tokio::spawn(async move {
                    lb_clone.record_request_result(
                        &provider_clone,
                        &model_clone,
                        RequestResult::Failure { error: error_msg_clone },
                    ).await;
                });

                futures::stream::once(async move {
                    Ok(Event::default().data(
                        json!({
                            "error": {
                                "message": "Network error",
                                "details": error_msg
                            }
                        }).to_string(),
                    ))
                }).boxed()
            }
        };

        Sse::new(stream)
    }

    /// 处理非流式请求
    async fn handle_non_streaming_request(
        &self,
        client: OpenAIClient,
        headers: reqwest::header::HeaderMap,
        body: Value,
        selected_backend: crate::loadbalance::SelectedBackend,
        start_time: Instant,
    ) -> Json<Value> {
        let provider = &selected_backend.backend.provider;
        let model = &selected_backend.backend.model;

        // 发送API请求
        let response = match client.chat_completions(headers, &body).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("API request failed: {:?}", e);
                self.load_balancer.record_request_result(
                    provider,
                    model,
                    RequestResult::Failure { error: e.to_string() },
                ).await;
                return Json(create_error_json(&crate::relay::client::ClientError::HeaderParseError(
                    format!("API request failed: {}", e)
                )));
            }
        };

        let latency = start_time.elapsed();

        // 处理响应
        if response.status().is_success() {
            // 记录成功
            self.load_balancer.record_request_result(
                provider,
                model,
                RequestResult::Success { latency },
            ).await;

            match response.text().await {
                Ok(text) => match serde_json::from_str::<Value>(&text) {
                    Ok(value) => Json(value),
                    Err(e) => {
                        tracing::error!("JSON parsing failed: {:?}", e);
                        Json(create_network_error_json(
                            "Upstream returned invalid JSON",
                            Some(text),
                        ))
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read response body: {:?}", e);
                    Json(create_network_error_json(
                        "Unable to read response data",
                        Some(e.to_string()),
                    ))
                }
            }
        } else {
            // 记录失败
            let status = response.status().as_u16();
            self.load_balancer.record_request_result(
                provider,
                model,
                RequestResult::Failure { error: format!("HTTP {}", status) },
            ).await;

            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());

            tracing::error!("Upstream API error: status {}, response: {}", status, body);
            Json(json!({
                "error": {
                    "message": "Upstream API returned error",
                    "status": status,
                    "details": body
                }
            }))
        }
    }

    /// 获取可用模型列表（根据用户权限过滤）
    pub async fn handle_models_for_user(&self, user_models: Vec<String>) -> Json<Value> {
        let model_list: Vec<Value> = user_models
            .into_iter()
            .map(|model_name| {
                json!({
                    "id": model_name,
                    "object": "model",
                    "created": chrono::Utc::now().timestamp(),
                    "owned_by": "berry-api"
                })
            })
            .collect();

        Json(json!({
            "object": "list",
            "data": model_list
        }))
    }
}
