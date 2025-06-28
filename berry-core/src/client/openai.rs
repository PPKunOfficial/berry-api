use super::traits::{AIBackendClient, BackendType, ChatCompletionConfig};
use super::types::{ClientError, ClientResponse};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

#[derive(Clone)]
pub struct OpenAIClient {
    client: Client,
    base_url: String,
}

impl OpenAIClient {
    /// 创建新的OpenAI客户端，需要提供base_url
    pub fn new(base_url: String) -> Self {
        Self::with_base_url_and_timeout(base_url, Duration::from_secs(30))
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self::with_base_url_and_timeout(base_url, Duration::from_secs(30))
    }

    pub fn with_timeout(base_url: String, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, base_url }
    }

    pub fn with_base_url_and_timeout(base_url: String, connect_timeout: Duration) -> Self {
        let client = Client::builder()
            .connect_timeout(connect_timeout) // 只设置连接超时，不限制总请求时间
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, base_url }
    }

    // 发送聊天完成请求
    pub async fn chat_completions(
        &self,
        headers: reqwest::header::HeaderMap,
        body: &Value,
    ) -> Result<reqwest::Response, ClientError> {
        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .headers(headers)
            .json(body)
            .send()
            .await?;

        Ok(response)
    }

    // 获取模型列表
    pub async fn models(&self, token: &str) -> Result<ClientResponse, ClientError> {
        let auth_header_value = format!("Bearer {}", token);
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", auth_header_value)
            .send()
            .await?;

        let status = response.status().as_u16();
        let body = response.text().await?;

        Ok(ClientResponse::new(status, body))
    }
}

// 实现AIBackendClient trait
#[async_trait]
impl AIBackendClient for OpenAIClient {
    fn backend_type(&self) -> BackendType {
        BackendType::OpenAI
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn with_timeout(self, timeout: Duration) -> Self {
        Self::with_base_url_and_timeout(self.base_url, timeout)
    }

    fn build_request_headers(
        &self,
        authorization: &headers::Authorization<headers::authorization::Bearer>,
        content_type: &headers::ContentType,
    ) -> Result<reqwest::header::HeaderMap, ClientError> {
        let mut headers = reqwest::header::HeaderMap::new();

        // 添加Authorization头
        let auth_value = format!("Bearer {}", authorization.token());
        headers.insert(
            reqwest::header::AUTHORIZATION,
            auth_value.parse().map_err(|e| {
                ClientError::HeaderParseError(format!("Invalid authorization header: {}", e))
            })?,
        );

        // 添加Content-Type头
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            content_type.to_string().parse().map_err(|e| {
                ClientError::HeaderParseError(format!("Invalid content-type header: {}", e))
            })?,
        );

        Ok(headers)
    }

    async fn chat_completions_raw(
        &self,
        headers: reqwest::header::HeaderMap,
        body: &Value,
    ) -> Result<reqwest::Response, ClientError> {
        self.chat_completions(headers, body).await
    }

    async fn models(&self, token: &str) -> Result<ClientResponse, ClientError> {
        self.models(token).await
    }

    fn convert_config_to_json(&self, config: &ChatCompletionConfig) -> Value {
        config.to_openai_json()
    }

    fn supports_model(&self, _model: &str) -> bool {
        // 不限制模型，让后端API自己验证
        // 用户可以使用任何模型名称，由OpenAI兼容API决定是否支持
        true
    }

    fn supported_models(&self) -> Vec<String> {
        // 返回空列表，表示支持所有模型（由后端决定）
        // 实际支持的模型列表应该通过models() API获取
        vec![]
    }
}
