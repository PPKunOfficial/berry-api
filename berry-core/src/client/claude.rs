use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;
use async_trait::async_trait;
use super::types::{ClientError, ClientResponse};
use super::traits::{AIBackendClient, BackendType, ChatCompletionConfig};

#[derive(Clone)]
pub struct ClaudeClient {
    client: Client,
    base_url: String,
}

impl ClaudeClient {
    /// 创建新的Claude客户端，需要提供base_url
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
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
        }
    }

    pub fn with_base_url_and_timeout(base_url: String, connect_timeout: Duration) -> Self {
        let client = Client::builder()
            .connect_timeout(connect_timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
        }
    }

    /// 将OpenAI格式的消息转换为Claude格式
    fn convert_messages_to_claude_format(messages: &[Value]) -> Result<(String, Vec<Value>), ClientError> {
        let mut system_message = String::new();
        let mut claude_messages = Vec::new();

        for message in messages {
            let role = message.get("role")
                .and_then(|r| r.as_str())
                .ok_or_else(|| ClientError::HeaderParseError(
                    "Missing role in message".to_string()
                ))?;

            let content = message.get("content")
                .and_then(|c| c.as_str())
                .ok_or_else(|| ClientError::HeaderParseError(
                    "Missing content in message".to_string()
                ))?;

            match role {
                "system" => {
                    if !system_message.is_empty() {
                        system_message.push('\n');
                    }
                    system_message.push_str(content);
                }
                "user" | "assistant" => {
                    claude_messages.push(json!({
                        "role": role,
                        "content": content
                    }));
                }
                _ => {
                    return Err(ClientError::HeaderParseError(
                        format!("Unsupported role: {}", role)
                    ));
                }
            }
        }

        Ok((system_message, claude_messages))
    }

    /// 将OpenAI格式的请求转换为Claude格式
    fn convert_openai_to_claude_format(&self, body: &Value) -> Result<Value, ClientError> {
        let model = body.get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("claude-3-sonnet-20240229");

        let messages = body.get("messages")
            .and_then(|m| m.as_array())
            .ok_or_else(|| ClientError::HeaderParseError(
                "Missing messages field".to_string()
            ))?;

        let (system_message, claude_messages) = Self::convert_messages_to_claude_format(messages)?;

        let mut claude_body = json!({
            "model": model,
            "messages": claude_messages,
            "max_tokens": body.get("max_tokens").unwrap_or(&json!(4096))
        });

        if !system_message.is_empty() {
            claude_body["system"] = json!(system_message);
        }

        if let Some(temp) = body.get("temperature") {
            claude_body["temperature"] = temp.clone();
        }

        if let Some(top_p) = body.get("top_p") {
            claude_body["top_p"] = top_p.clone();
        }

        if let Some(stream) = body.get("stream") {
            claude_body["stream"] = stream.clone();
        }

        Ok(claude_body)
    }

    /// 发送聊天完成请求
    pub async fn chat_completions(
        &self,
        headers: reqwest::header::HeaderMap,
        body: &Value,
    ) -> Result<reqwest::Response, ClientError> {
        let claude_body = self.convert_openai_to_claude_format(body)?;
        
        let response = self.client
            .post(format!("{}/v1/messages", self.base_url))
            .headers(headers)
            .json(&claude_body)
            .send()
            .await?;

        Ok(response)
    }

    /// 获取模型列表
    pub async fn models(&self, token: &str) -> Result<ClientResponse, ClientError> {
        // Claude API现在支持models端点
        let response = self.client
            .get(format!("{}/v1/models", self.base_url))
            .header("x-api-key", token)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await?;

        let status = response.status().as_u16();
        let body = response.text().await?;

        Ok(ClientResponse::new(status, body))
    }
}

// 实现AIBackendClient trait
#[async_trait]
impl AIBackendClient for ClaudeClient {
    fn backend_type(&self) -> BackendType {
        BackendType::Claude
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

        // Claude使用x-api-key而不是Authorization
        headers.insert(
            "x-api-key",
            authorization.token().parse().map_err(|e| {
                ClientError::HeaderParseError(format!("Invalid API key: {}", e))
            })?,
        );

        // 添加Claude特定的头部
        headers.insert(
            "anthropic-version",
            "2023-06-01".parse().map_err(|e| {
                ClientError::HeaderParseError(format!("Invalid anthropic-version header: {}", e))
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
        // 转换为Claude格式
        let openai_json = config.to_openai_json();
        self.convert_openai_to_claude_format(&openai_json)
            .unwrap_or_else(|_| openai_json)
    }

    fn supports_model(&self, _model: &str) -> bool {
        // 不限制模型，让后端API自己验证
        // 用户可以使用任何模型名称，由Claude API决定是否支持
        true
    }

    fn supported_models(&self) -> Vec<String> {
        // 返回空列表，表示支持所有模型（由后端决定）
        // 实际支持的模型列表应该通过models() API获取
        vec![]
    }
}
