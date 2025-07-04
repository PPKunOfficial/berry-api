use axum::response::sse::Event;
use axum::response::Sse;
use axum::{extract::Json, response::IntoResponse};
use axum_extra::TypedHeader;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

use crate::relay::client::{AIBackendClient, ClientFactory, UnifiedClient};
use berry_loadbalance::{RouteErrorType, RouteResult, RouteSelector, SelectedRoute};

use super::types::ErrorHandler;

/// 基于路由选择器的OpenAI兼容处理器
///
/// 这是新的简化实现，使用RouteSelector接口来处理负载均衡
pub struct RouteBasedHandler {
    route_selector: Arc<dyn RouteSelector>,
}

impl RouteBasedHandler {
    /// 创建新的路由处理器
    pub fn new(route_selector: Arc<dyn RouteSelector>) -> Self {
        Self { route_selector }
    }

    /// 处理聊天完成请求（简化版本）
    pub async fn handle_completions(
        self: Arc<Self>,
        TypedHeader(authorization): TypedHeader<
            headers::Authorization<headers::authorization::Bearer>,
        >,
        TypedHeader(content_type): TypedHeader<headers::ContentType>,
        Json(body): Json<Value>,
    ) -> impl IntoResponse {
        let start_time = Instant::now();

        // 提取模型名称
        let model_name = match body.get("model").and_then(|v| v.as_str()) {
            Some(name) => name.to_string(),
            None => {
                return ErrorHandler::from_anyhow_error(
                    &anyhow::anyhow!("Missing 'model' field in request body"),
                    Some("Invalid request"),
                )
                .into_response();
            }
        };

        // 提取用户标签（如果有）
        let user_tags = self.extract_user_tags(&body);

        // 检查是否为流式请求（提前检查以避免后续借用问题）
        let is_stream = body
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 处理请求
        match self
            .process_request(
                &model_name,
                body,
                &authorization,
                &content_type,
                start_time,
                user_tags.as_deref(),
            )
            .await
        {
            Ok(response) => response,
            Err(e) => {
                tracing::error!(
                    "Request processing failed for model '{}': {}",
                    model_name,
                    e
                );

                if is_stream {
                    ErrorHandler::streaming_from_anyhow_error(
                        &e,
                        Some(&format!(
                            "Streaming request processing failed for model '{model_name}'"
                        )),
                    )
                    .into_response()
                } else {
                    ErrorHandler::from_anyhow_error(
                        &e,
                        Some(&format!(
                            "Request processing failed for model '{model_name}'"
                        )),
                    )
                    .into_response()
                }
            }
        }
    }

    /// 处理请求的核心逻辑
    async fn process_request(
        &self,
        model_name: &str,
        mut body: Value,
        authorization: &headers::Authorization<headers::authorization::Bearer>,
        content_type: &headers::ContentType,
        start_time: Instant,
        user_tags: Option<&[String]>,
    ) -> Result<axum::response::Response, anyhow::Error> {
        let max_retries = 3; // 可以从配置中读取
        let original_model = model_name.to_string();

        // 检查是否指定了特定的后端
        let backend_param = body
            .get("backend")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 如果指定了backend参数，移除它（不传递给上游API）
        if backend_param.is_some() {
            if let Some(obj) = body.as_object_mut() {
                obj.remove("backend");
            }
        }

        for attempt in 0..max_retries {
            // 重置模型名称为原始请求的模型名称
            body["model"] = Value::String(original_model.clone());

            // 1. 选择路由 - 这是唯一的负载均衡调用
            let route = if let Some(ref backend) = backend_param {
                // 如果指定了backend，使用特定路由选择
                self.route_selector
                    .select_specific_route(model_name, backend)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to select specific backend '{}': {}", backend, e)
                    })?
            } else {
                // 正常的路由选择
                self.route_selector
                    .select_route(model_name, user_tags)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to select route: {}", e))?
            };

            tracing::debug!(
                "Selected route for model '{}': {} (attempt {}/{})",
                model_name,
                route.route_id,
                attempt + 1,
                max_retries
            );

            // 2. 更新请求体中的模型名称为后端的真实模型名称
            body["model"] = Value::String(route.backend.model.clone());

            // 3. 尝试发送请求
            match self
                .try_single_request(&route, &body, authorization, content_type, start_time)
                .await
            {
                Ok(response) => {
                    // 4. 报告成功结果
                    let latency = start_time.elapsed();
                    self.route_selector
                        .report_result(&route.route_id, RouteResult::Success { latency })
                        .await;

                    return Ok(response);
                }
                Err(e) => {
                    // 4. 报告失败结果
                    let error_type = self.classify_error(&e);
                    self.route_selector
                        .report_result(
                            &route.route_id,
                            RouteResult::Failure {
                                error: e.to_string(),
                                error_type: Some(error_type),
                            },
                        )
                        .await;

                    if attempt == max_retries - 1 {
                        return Err(anyhow::anyhow!(
                            "Request to backend failed for model '{}' after {} attempts: {}",
                            model_name,
                            max_retries,
                            e
                        ));
                    }

                    tracing::warn!("Request failed on attempt {}, retrying: {}", attempt + 1, e);
                    continue;
                }
            }
        }

        Err(anyhow::anyhow!("Unexpected end of retry loop"))
    }

    /// 尝试单次请求
    async fn try_single_request(
        &self,
        route: &SelectedRoute,
        body: &Value,
        _authorization: &headers::Authorization<headers::authorization::Bearer>,
        content_type: &headers::ContentType,
        start_time: Instant,
    ) -> Result<axum::response::Response, anyhow::Error> {
        // 获取API密钥
        let api_key = route.get_api_key()?;

        // 构建请求头
        let mut headers = route.get_headers();
        headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));
        headers.insert("Content-Type".to_string(), content_type.to_string());

        // 创建客户端
        let client = ClientFactory::create_client_from_provider_type(
            route.provider.backend_type.clone(),
            route.provider.base_url.clone(),
            route.get_timeout(),
        )?;

        // 检查是否为流式请求
        let is_stream = body
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_stream {
            self.handle_streaming_request(&client, route, body, headers, start_time)
                .await
        } else {
            self.handle_non_streaming_request(&client, route, body, headers, start_time)
                .await
        }
    }

    /// 处理非流式请求
    async fn handle_non_streaming_request(
        &self,
        client: &UnifiedClient,
        route: &SelectedRoute,
        body: &Value,
        headers: std::collections::HashMap<String, String>,
        start_time: Instant,
    ) -> Result<axum::response::Response, anyhow::Error> {
        // 转换headers格式
        let mut header_map = reqwest::header::HeaderMap::new();
        for (key, value) in headers {
            if let (Ok(header_name), Ok(header_value)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(&value),
            ) {
                header_map.insert(header_name, header_value);
            }
        }

        let response = client.chat_completions_raw(header_map, body).await?;

        let latency = start_time.elapsed();

        if response.status().is_success() {
            tracing::debug!(
                "Non-streaming request succeeded for route {} in {:?}",
                route.route_id,
                latency
            );

            match response.text().await {
                Ok(text) => Ok(axum::response::Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(text.into())?),
                Err(e) => Err(anyhow::anyhow!("Failed to read response body: {}", e)),
            }
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("HTTP {}: {}", status, error_text))
        }
    }

    /// 处理流式请求
    async fn handle_streaming_request(
        &self,
        client: &UnifiedClient,
        route: &SelectedRoute,
        body: &Value,
        headers: std::collections::HashMap<String, String>,
        start_time: Instant,
    ) -> Result<axum::response::Response, anyhow::Error> {
        // 转换headers格式
        let mut header_map = reqwest::header::HeaderMap::new();
        for (key, value) in headers {
            if let (Ok(header_name), Ok(header_value)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(&value),
            ) {
                header_map.insert(header_name, header_value);
            }
        }

        let response = client.chat_completions_raw(header_map, body).await?;

        let latency = start_time.elapsed();

        if response.status().is_success() {
            tracing::debug!(
                "Streaming request started for route {} in {:?}",
                route.route_id,
                latency
            );

            let stream = response.bytes_stream().eventsource().map(
                |event| -> Result<Event, std::convert::Infallible> {
                    match event {
                        Ok(event) => Ok(Event::default().data(event.data)),
                        Err(e) => Ok(
                            Event::default().data(format!("data: {{\"error\": \"{}\"}}\n\n", e))
                        ),
                    }
                },
            );

            Ok(Sse::new(stream).into_response())
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("HTTP {}: {}", status, error_text))
        }
    }

    /// 提取用户标签
    fn extract_user_tags(&self, body: &Value) -> Option<Vec<String>> {
        body.get("user_tags").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
    }

    /// 分类错误类型
    fn classify_error(&self, error: &anyhow::Error) -> RouteErrorType {
        let error_str = error.to_string().to_lowercase();

        if error_str.contains("timeout") || error_str.contains("timed out") {
            RouteErrorType::Timeout
        } else if error_str.contains("connection")
            || error_str.contains("network")
            || error_str.contains("dns")
        {
            RouteErrorType::Network
        } else if error_str.contains("401")
            || error_str.contains("403")
            || error_str.contains("unauthorized")
            || error_str.contains("forbidden")
        {
            RouteErrorType::Authentication
        } else if error_str.contains("429")
            || error_str.contains("rate limit")
            || error_str.contains("too many requests")
        {
            RouteErrorType::RateLimit
        } else if error_str.contains("5")
            && (error_str.contains("00") || error_str.contains("02") || error_str.contains("03"))
        {
            RouteErrorType::Server
        } else if error_str.contains("model") || error_str.contains("parameter") {
            RouteErrorType::Model
        } else {
            RouteErrorType::Network // 默认分类
        }
    }
}
