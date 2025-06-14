use axum::response::Sse;
use axum::response::sse::Event;
use axum::{extract::Json, response::IntoResponse};
use axum_extra::TypedHeader;
use eventsource_stream::Eventsource;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;

use berry_loadbalance::{LoadBalanceService, RequestResult};
use crate::relay::client::{ClientFactory, UnifiedClient, AIBackendClient};

use super::types::{ErrorHandler, ErrorRecorder, RetryErrorHandler, ResponseBodyHandler, ErrorType, create_error_response};

/// 负载均衡的OpenAI兼容处理器
pub struct LoadBalancedHandler {
    load_balancer: std::sync::Arc<LoadBalanceService>,
}

impl LoadBalancedHandler {
    pub fn new(load_balancer: std::sync::Arc<LoadBalanceService>) -> Self {
        Self { load_balancer }
    }

    /// 处理聊天完成请求（支持负载均衡和智能重试）
    pub async fn handle_completions(
        self: Arc<Self>,
        TypedHeader(authorization): TypedHeader<
            headers::Authorization<headers::authorization::Bearer>,
        >,
        TypedHeader(content_type): TypedHeader<headers::ContentType>,
        Json(body): Json<Value>,
    ) -> axum::response::Response {
        self.handle_completions_with_user_tags(
            TypedHeader(authorization),
            TypedHeader(content_type),
            Json(body),
            None,
        ).await
    }

    /// 处理聊天完成请求（支持用户标签过滤）
    pub async fn handle_completions_with_user_tags(
        self: Arc<Self>,
        TypedHeader(authorization): TypedHeader<
            headers::Authorization<headers::authorization::Bearer>,
        >,
        TypedHeader(content_type): TypedHeader<headers::ContentType>,
        Json(mut body): Json<Value>,
        user_tags: Option<&[String]>,
    ) -> axum::response::Response {
        let start_time = Instant::now();

        // 从请求体中提取模型名称
        let model_name = match body.get("model").and_then(|m| m.as_str()) {
            Some(name) => name.to_string(),
            None => {
                tracing::error!("Missing model field in request");
                return create_error_response(
                    ErrorType::BadRequest,
                    "Missing model field in request",
                    Some("The 'model' field is required in the request body".to_string()),
                ).into_response();
            }
        };

        // 尝试处理请求，带内部重试机制
        match self
            .try_handle_with_retries(
                &model_name,
                &mut body,
                &authorization,
                &content_type,
                start_time,
                user_tags,
            )
            .await
        {
            Ok(response) => response,
            Err(e) => {
                tracing::error!(
                    "All retry attempts failed for model '{}': {}",
                    model_name,
                    e
                );

                // 使用统一的错误处理器
                ErrorHandler::from_anyhow_error(&e, Some(&format!("Request processing failed for model '{}'", model_name))).into_response()
            }
        }
    }

    /// 尝试处理请求，带重试机制
    async fn try_handle_with_retries(
        &self,
        model_name: &str,
        body: &mut Value,
        authorization: &headers::Authorization<headers::authorization::Bearer>,
        content_type: &headers::ContentType,
        start_time: Instant,
        user_tags: Option<&[String]>,
    ) -> Result<axum::response::Response, anyhow::Error> {
        let max_retries = 3; // 可以从配置中读取
        let original_model = model_name.to_string();

        // 检查是否指定了特定的后端
        let backend_param = body.get("backend")
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

            // 根据是否指定了backend参数选择后端
            let selected_backend = if let Some(ref provider_name) = backend_param {
                // 直接选择指定的后端
                tracing::info!("Using specified backend '{}' for model '{}'", provider_name, model_name);
                match self.load_balancer.select_specific_backend(model_name, provider_name).await {
                    Ok(backend) => backend,
                    Err(e) => {
                        tracing::error!(
                            "Failed to select specified backend '{}' for model '{}': {}",
                            provider_name,
                            model_name,
                            e
                        );
                        return Err(anyhow::anyhow!(
                            "Specified backend '{}' is not available for model '{}': {}",
                            provider_name,
                            model_name,
                            e
                        ));
                    }
                }
            } else {
                // 使用负载均衡器选择后端（考虑用户标签）
                match self.load_balancer.select_backend_with_user_tags(model_name, user_tags).await {
                    Ok(backend) => backend,
                    Err(e) => {
                        if attempt == max_retries - 1 {
                            // 最后一次尝试失败，提供详细错误信息
                            tracing::error!(
                                "Failed to select backend for model '{}' after {} attempts: {}",
                                model_name,
                                max_retries,
                                e
                            );
                            return Err(anyhow::anyhow!(
                                "Backend selection failed for model '{}' after {} attempts. {}",
                                model_name,
                                max_retries,
                                e
                            ));
                        }
                        tracing::warn!(
                            "Backend selection failed on attempt {}, retrying: {}",
                            attempt + 1,
                            e
                        );
                        continue;
                    }
                }
            };

            tracing::info!(
                "Selected backend for model '{}' (attempt {}): provider='{}', model='{}', selection_time={}ms",
                model_name,
                attempt + 1,
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
                    // 使用统一的错误记录器
                    ErrorRecorder::record_request_failure(
                        &self.load_balancer,
                        &selected_backend.backend.provider,
                        &selected_backend.backend.model,
                        &anyhow::anyhow!("{}", e),
                    ).await;

                    // 使用统一的重试错误处理器
                    if let Err(final_error) = RetryErrorHandler::handle_retry_error(
                        attempt,
                        max_retries,
                        &anyhow::anyhow!("{}", e),
                        &format!("API key configuration error for model '{}'", model_name),
                    ) {
                        return Err(final_error);
                    }
                    continue;
                }
            };

            // 创建客户端，只设置连接超时，不限制总请求时间
            // 连接成功后允许无限时间生成内容，直到客户端断开连接
            let connect_timeout = std::time::Duration::from_secs(selected_backend.provider.timeout_seconds);
            let client = match ClientFactory::create_client_from_provider_type(
                selected_backend.provider.backend_type.clone(),
                selected_backend.provider.base_url.clone(),
                connect_timeout,
            ) {
                Ok(c) => c,
                Err(e) => {
                    // 使用统一的错误记录器
                    ErrorRecorder::record_request_failure(
                        &self.load_balancer,
                        &selected_backend.backend.provider,
                        &selected_backend.backend.model,
                        &anyhow::anyhow!("{}", e),
                    ).await;

                    // 使用统一的重试错误处理器
                    if let Err(final_error) = RetryErrorHandler::handle_retry_error(
                        attempt,
                        max_retries,
                        &anyhow::anyhow!("{}", e),
                        &format!("Failed to create client for model '{}'", model_name),
                    ) {
                        return Err(final_error);
                    }
                    continue;
                }
            };

            // 构建请求头
            let headers = match client.build_request_headers(&authorization, &content_type) {
                Ok(mut h) => {
                    // 使用选中后端的API密钥
                    h.insert(
                        "Authorization",
                        format!("Bearer {}", api_key).parse().unwrap(),
                    );

                    // 添加自定义头部
                    for (key, value) in selected_backend.get_headers() {
                        if let (Ok(header_name), Ok(header_value)) = (
                            key.parse::<reqwest::header::HeaderName>(),
                            value.parse::<reqwest::header::HeaderValue>(),
                        ) {
                            h.insert(header_name, header_value);
                        }
                    }
                    h
                }
                Err(e) => {
                    self.load_balancer
                        .record_request_result(
                            &selected_backend.backend.provider,
                            &selected_backend.backend.model,
                            RequestResult::Failure {
                                error: e.to_string(),
                            },
                        )
                        .await;

                    if attempt == max_retries - 1 {
                        return Err(anyhow::anyhow!(
                            "Request header configuration error for model '{}': {}. Please check provider configuration.",
                            model_name,
                            e
                        ));
                    }
                    tracing::warn!(
                        "Header build error on attempt {}, retrying: {}",
                        attempt + 1,
                        e
                    );
                    continue;
                }
            };

            // 尝试发送请求
            match self
                .try_single_request(&client, headers, body, &selected_backend, start_time)
                .await
            {
                Ok(response) => return Ok(response),
                Err(e) => {
                    // 记录失败
                    self.load_balancer
                        .record_request_result(
                            &selected_backend.backend.provider,
                            &selected_backend.backend.model,
                            RequestResult::Failure {
                                error: e.to_string(),
                            },
                        )
                        .await;

                    if attempt == max_retries - 1 {
                        return Err(anyhow::anyhow!(
                            "Request to backend failed for model '{}' after {} attempts: {}. All available backends may be experiencing issues.",
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
        client: &UnifiedClient,
        headers: reqwest::header::HeaderMap,
        body: &Value,
        selected_backend: &berry_loadbalance::SelectedBackend,
        start_time: Instant,
    ) -> Result<axum::response::Response, anyhow::Error> {
        // 检查是否为流式请求
        let is_stream = body
            .get("stream")
            .unwrap_or(&Value::Bool(false))
            .as_bool()
            .unwrap_or(false);

        if is_stream {
            // 流式请求：尝试发送请求，失败时返回错误以触发重试
            match self
                .try_streaming_request(
                    client.clone(),
                    headers,
                    body.clone(),
                    selected_backend.clone(),
                    start_time,
                )
                .await
            {
                Ok(response) => Ok(response.into_response()),
                Err(e) => Err(anyhow::anyhow!("Streaming request failed: {}", e)),
            }
        } else {
            // 非流式请求：使用保活机制，立即开始响应
            match self
                .try_non_streaming_request_with_keepalive(
                    client.clone(),
                    headers,
                    body.clone(),
                    selected_backend.clone(),
                    start_time,
                )
                .await
            {
                Ok(response) => Ok(response),
                Err(e) => Err(anyhow::anyhow!("Non-streaming request failed: {}", e)),
            }
        }
    }

    /// 尝试流式请求（可能失败以触发重试）
    async fn try_streaming_request(
        &self,
        client: UnifiedClient,
        headers: reqwest::header::HeaderMap,
        body: Value,
        selected_backend: berry_loadbalance::SelectedBackend,
        start_time: Instant,
    ) -> Result<
        Sse<futures::stream::BoxStream<'static, Result<Event, std::convert::Infallible>>>,
        anyhow::Error,
    > {
        let provider = &selected_backend.backend.provider;
        let model = &selected_backend.backend.model;

        // 发送API请求
        let response = match client.chat_completions_raw(headers, &body).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::debug!("Streaming request failed: {:?}", e);
                // 记录失败但不在这里处理，让重试机制处理
                self.load_balancer
                    .record_request_result(
                        provider,
                        model,
                        RequestResult::Failure {
                            error: e.to_string(),
                        },
                    )
                    .await;
                return Err(anyhow::anyhow!("API request failed: {}", e));
            }
        };

        // 检查HTTP状态
        if !response.status().is_success() {
            // 使用统一的响应体处理器
            let (status, error_body) = ResponseBodyHandler::read_and_log_error_body(
                response,
                "Streaming request"
            ).await;

            // 使用统一的错误记录器
            ErrorRecorder::record_http_failure(
                &self.load_balancer,
                provider,
                model,
                status,
                &error_body,
            ).await;

            return Err(anyhow::anyhow!("HTTP error {}: {}", status, error_body));
        }

        // 成功情况 - 创建流式响应
        Ok(self
            .create_successful_stream(response, selected_backend, start_time)
            .await)
    }

    /// 创建成功的流式响应
    async fn create_successful_stream(
        &self,
        response: reqwest::Response,
        selected_backend: berry_loadbalance::SelectedBackend,
        start_time: Instant,
    ) -> Sse<futures::stream::BoxStream<'static, Result<Event, std::convert::Infallible>>> {
        let load_balancer = self.load_balancer.clone();
        let provider = selected_backend.backend.provider.clone();
        let model = selected_backend.backend.model.clone();
        let latency = start_time.elapsed();

        // 检查backend是否在不健康列表中
        let backend_key = format!("{}:{}", provider, model);
        let metrics = load_balancer.get_metrics();

        if metrics.is_in_unhealthy_list(&backend_key) {
            // 不健康的backend请求成功，主动恢复为健康状态
            tracing::info!(
                "Unhealthy backend {} streaming request succeeded, automatically marking as healthy",
                backend_key
            );
        }

        // 无论之前是否健康，都记录成功（实现自动恢复）
        let lb_clone = load_balancer.clone();
        let provider_clone = provider.clone();
        let model_clone = model.clone();
        tokio::spawn(async move {
            lb_clone
                .record_request_result(
                    &provider_clone,
                    &model_clone,
                    RequestResult::Success { latency },
                )
                .await;
        });

        // 创建带保活机制的流式响应
        let data_stream = response
            .bytes_stream()
            .eventsource()
            .map(|result| match result {
                Ok(event) => {
                    tracing::debug!("SSE event: {:?}", event.data);
                    Ok(Event::default().data(event.data))
                }
                Err(err) => {
                    // SSE解析错误，记录日志但继续传递原始数据
                    // 不在流中包含错误信息，让连接自然断开
                    tracing::error!("SSE parsing error: {:?}", err);
                    Ok(Event::default().data(""))
                }
            });

        // 创建智能保活流，当数据流结束时自动停止
        use futures::StreamExt;
        let stream = futures::stream::unfold(
            (data_stream, tokio::time::interval(std::time::Duration::from_secs(30)), false),
            move |(mut data_stream, mut keepalive_interval, data_ended)| async move {
                if data_ended {
                    return None;
                }

                tokio::select! {
                    // 优先处理数据流
                    data_result = data_stream.next() => {
                        match data_result {
                            Some(event) => {
                                // 有数据，继续处理
                                Some((event, (data_stream, keepalive_interval, false)))
                            }
                            None => {
                                // 数据流结束，不再发送保活信号
                                tracing::debug!("Data stream ended, stopping keep-alive");
                                None
                            }
                        }
                    }
                    // 发送保活信号
                    _ = keepalive_interval.tick() => {
                        tracing::debug!("Sending keep-alive comment");
                        Some((Ok(Event::default().comment("keep-alive")), (data_stream, keepalive_interval, false)))
                    }
                }
            }
        ).boxed();

        Sse::new(stream)
    }

    /// 尝试非流式请求（可能失败以触发重试）
    async fn try_non_streaming_request(
        &self,
        client: UnifiedClient,
        headers: reqwest::header::HeaderMap,
        body: Value,
        selected_backend: berry_loadbalance::SelectedBackend,
        start_time: Instant,
    ) -> Result<Json<Value>, anyhow::Error> {
        let provider = &selected_backend.backend.provider;
        let model = &selected_backend.backend.model;

        // 发送API请求
        let response = match client.chat_completions_raw(headers, &body).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::debug!("Non-streaming request failed: {:?}", e);
                // 记录失败但不在这里处理，让重试机制处理
                self.load_balancer
                    .record_request_result(
                        provider,
                        model,
                        RequestResult::Failure {
                            error: e.to_string(),
                        },
                    )
                    .await;
                return Err(anyhow::anyhow!("API request failed: {}", e));
            }
        };

        let latency = start_time.elapsed();

        // 处理响应
        if response.status().is_success() {
            // 检查backend是否在不健康列表中
            let backend_key = format!("{}:{}", provider, model);
            let metrics = self.load_balancer.get_metrics();

            if metrics.is_in_unhealthy_list(&backend_key) {
                // 不健康的backend请求成功，主动恢复为健康状态
                tracing::info!(
                    "Unhealthy backend {} request succeeded, automatically marking as healthy",
                    backend_key
                );
            }

            // 无论之前是否健康，都记录成功（实现自动恢复）
            self.load_balancer
                .record_request_result(provider, model, RequestResult::Success { latency })
                .await;

            match response.text().await {
                Ok(text) => match serde_json::from_str::<Value>(&text) {
                    Ok(value) => Ok(Json(value)),
                    Err(e) => {
                        tracing::error!("JSON parsing failed: {:?}", e);
                        Err(anyhow::anyhow!("JSON parsing failed: {}", e))
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read response body: {:?}", e);
                    Err(anyhow::anyhow!("Failed to read response body: {}", e))
                }
            }
        } else {
            // 使用统一的响应体处理器
            let (status, error_body) = ResponseBodyHandler::read_and_log_error_body(
                response,
                "Non-streaming request"
            ).await;

            // 使用统一的错误记录器
            ErrorRecorder::record_http_failure(
                &self.load_balancer,
                provider,
                model,
                status,
                &error_body,
            ).await;

            Err(anyhow::anyhow!("HTTP error {}: {}", status, error_body))
        }
    }

    /// 尝试非流式请求（带保活机制）
    async fn try_non_streaming_request_with_keepalive(
        &self,
        client: UnifiedClient,
        headers: reqwest::header::HeaderMap,
        body: Value,
        selected_backend: berry_loadbalance::SelectedBackend,
        start_time: Instant,
    ) -> Result<axum::response::Response, anyhow::Error> {
        let provider = &selected_backend.backend.provider;
        let model = &selected_backend.backend.model;

        // 创建一个通道来传递最终结果
        let (result_tx, result_rx) = tokio::sync::mpsc::channel::<Result<String, anyhow::Error>>(1);

        // 在后台发送API请求
        let client_clone = client.clone();
        let headers_clone = headers.clone();
        let body_clone = body.clone();
        let provider_clone = provider.clone();
        let model_clone = model.clone();
        let load_balancer_clone = self.load_balancer.clone();
        let start_time_clone = start_time.clone();

        tokio::spawn(async move {
            let response = match client_clone.chat_completions_raw(headers_clone, &body_clone).await {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::debug!("Non-streaming request failed: {:?}", e);
                    // 记录失败
                    load_balancer_clone
                        .record_request_result(
                            &provider_clone,
                            &model_clone,
                            RequestResult::Failure {
                                error: e.to_string(),
                            },
                        )
                        .await;
                    let _ = result_tx.send(Err(anyhow::anyhow!("API request failed: {}", e))).await;
                    return;
                }
            };

            let latency = start_time_clone.elapsed();

            // 处理响应
            if response.status().is_success() {
                // 检查backend是否在不健康列表中
                let backend_key = format!("{}:{}", provider_clone, model_clone);
                let metrics = load_balancer_clone.get_metrics();

                if metrics.is_in_unhealthy_list(&backend_key) {
                    // 不健康的backend请求成功，主动恢复为健康状态
                    tracing::info!(
                        "Unhealthy backend {} request succeeded, automatically marking as healthy",
                        backend_key
                    );
                }

                // 无论之前是否健康，都记录成功（实现自动恢复）
                load_balancer_clone
                    .record_request_result(&provider_clone, &model_clone, RequestResult::Success { latency })
                    .await;

                match response.text().await {
                    Ok(text) => {
                        let _ = result_tx.send(Ok(text)).await;
                    },
                    Err(e) => {
                        tracing::error!("Failed to read response body: {:?}", e);
                        let _ = result_tx.send(Err(anyhow::anyhow!("Failed to read response body: {}", e))).await;
                    }
                }
            } else {
                // 使用统一的响应体处理器
                let (status, error_body) = ResponseBodyHandler::read_and_log_error_body(
                    response,
                    "Non-streaming request"
                ).await;

                // 使用统一的错误记录器
                ErrorRecorder::record_http_failure(
                    &load_balancer_clone,
                    &provider_clone,
                    &model_clone,
                    status,
                    &error_body,
                ).await;

                let _ = result_tx.send(Err(anyhow::anyhow!("HTTP error {}: {}", status, error_body))).await;
            }
        });

        // 创建真正的流式保活响应
        let response_stream = futures::stream::unfold(
            (result_rx, false),
            move |(mut result_rx, finished)| async move {
                if finished {
                    return None;
                }

                // 创建保活定时器
                let mut keepalive_interval = tokio::time::interval(std::time::Duration::from_secs(10));
                keepalive_interval.tick().await; // 跳过第一次立即触发

                tokio::select! {
                    // 检查是否有最终结果
                    result = result_rx.recv() => {
                        match result {
                            Some(Ok(text)) => {
                                // 发送实际响应数据，然后结束流
                                Some((Ok::<bytes::Bytes, std::convert::Infallible>(bytes::Bytes::from(text)), (result_rx, true)))
                            }
                            Some(Err(e)) => {
                                // 处理错误，然后结束流
                                let error_json = serde_json::json!({
                                    "error": {
                                        "message": "Request failed",
                                        "details": e.to_string()
                                    }
                                });
                                Some((Ok(bytes::Bytes::from(error_json.to_string())), (result_rx, true)))
                            }
                            None => {
                                // 通道关闭，发送错误然后结束流
                                let error_json = serde_json::json!({
                                    "error": {
                                        "message": "Request was cancelled"
                                    }
                                });
                                Some((Ok(bytes::Bytes::from(error_json.to_string())), (result_rx, true)))
                            }
                        }
                    }
                    // 发送保活信号（空格）
                    _ = keepalive_interval.tick() => {
                        Some((Ok(bytes::Bytes::from(" ")), (result_rx, false)))
                    }
                }
            }
        );

        // 创建流式响应
        let body = axum::body::Body::from_stream(response_stream);
        let response = axum::response::Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .header("Transfer-Encoding", "chunked")
            .body(body)
            .map_err(|e| anyhow::anyhow!("Failed to build response: {}", e))?;

        Ok(response)
    }



    #[allow(dead_code)]
    /// 处理非流式请求（兜底方法，当重试失败时使用）
    async fn handle_non_streaming_request(
        &self,
        client: UnifiedClient,
        headers: reqwest::header::HeaderMap,
        body: Value,
        selected_backend: berry_loadbalance::SelectedBackend,
        start_time: Instant,
    ) -> Json<Value> {
        // 尝试请求，如果失败则返回错误响应
        match self
            .try_non_streaming_request(client, headers, body, selected_backend, start_time)
            .await
        {
            Ok(response) => response,
            Err(e) => Json(json!({
                "error": {
                    "message": "All retry attempts failed",
                    "details": e.to_string()
                }
            })),
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

#[cfg(test)]
mod tests {
    use super::*;



    #[test]
    fn test_error_handler_from_anyhow() {
        // 测试从anyhow错误创建响应
        let error = anyhow::anyhow!("HTTP error 400: Invalid request format");
        let response = ErrorHandler::from_anyhow_error(&error, Some("Test context"));

        // 确保函数不会panic
        let _response = response.into_response();
    }

    #[test]
    fn test_error_handler_from_http() {
        // 测试从HTTP错误创建响应
        let response_body = r#"{"error":{"message":"Rate limit exceeded"}}"#;
        let response = ErrorHandler::from_http_error(429, response_body, Some("Test context"));

        // 确保函数不会panic
        let _response = response.into_response();
    }

    #[test]
    fn test_error_handler_business_error() {
        // 测试业务错误
        let response = ErrorHandler::business_error(
            ErrorType::BadRequest,
            "Invalid input",
            Some("Details about the error".to_string())
        );

        // 确保函数不会panic
        let _response = response.into_response();
    }
}
