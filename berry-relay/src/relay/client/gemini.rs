//! Google Gemini API client implementation
//! 
//! This module provides a client for interacting with Google's Gemini API.
//! Note: Gemini API has a different format compared to OpenAI, so this is a basic implementation.

use crate::relay::client::{ClientError, ClientResult, BackendType, ClientResponse};
use crate::relay::client::traits::{AIBackendClient, ChatCompletionConfig};
use async_trait::async_trait;
use reqwest::{Client, Response, header::HeaderMap};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, error};

/// Google Gemini API client
#[derive(Debug, Clone)]
pub struct GeminiClient {
    client: Client,
    base_url: String,
    timeout: Duration,
}

impl GeminiClient {
    /// 创建新的Gemini客户端
    pub fn new() -> Self {
        Self::with_base_url("https://generativelanguage.googleapis.com/v1beta".to_string())
    }

    /// 使用自定义base URL创建客户端
    pub fn with_base_url(base_url: String) -> Self {
        Self::with_base_url_and_timeout(base_url, Duration::from_secs(30))
    }

    /// 使用自定义base URL和超时创建客户端
    pub fn with_base_url_and_timeout(base_url: String, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            timeout,
        }
    }

    /// 获取后端类型
    pub fn backend_type(&self) -> BackendType {
        BackendType::Gemini
    }

    /// 获取base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// 获取超时设置
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// 发送chat completions请求（转换为Gemini格式）
    pub async fn chat_completions(
        &self,
        headers: reqwest::header::HeaderMap,
        body: &Value,
    ) -> ClientResult<Response> {
        debug!("Sending Gemini chat completions request");

        // 从headers中提取API key
        let api_key = self.extract_api_key(&headers)?;
        
        // 转换OpenAI格式到Gemini格式
        let gemini_body = self.convert_openai_to_gemini(body)?;
        
        // 构建Gemini API URL
        let model = body.get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("gemini-pro");
        
        let url = format!("{}/models/{}:generateContent?key={}", 
                         self.base_url, model, api_key);

        debug!("Gemini API URL: {}", url);
        debug!("Gemini request body: {}", serde_json::to_string_pretty(&gemini_body).unwrap_or_default());

        // 发送请求
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&gemini_body)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send Gemini request: {}", e);
                ClientError::RequestError(e)
            })?;

        debug!("Gemini response status: {}", response.status());
        Ok(response)
    }

    /// 获取模型列表（Gemini格式）
    pub async fn models(&self, api_key: &str) -> ClientResult<Response> {
        debug!("Fetching Gemini models list");

        let url = format!("{}/models?key={}", self.base_url, api_key);
        debug!("Gemini models API URL: {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to fetch Gemini models: {}", e);
                ClientError::RequestError(e)
            })?;

        debug!("Gemini models response status: {}", response.status());
        Ok(response)
    }

    /// 从headers中提取API key
    fn extract_api_key(&self, headers: &reqwest::header::HeaderMap) -> ClientResult<String> {
        if let Some(auth_header) = headers.get("Authorization") {
            let auth_str = auth_header.to_str()
                .map_err(|e| ClientError::HeaderParseError(format!("Invalid Authorization header: {}", e)))?;
            
            if auth_str.starts_with("Bearer ") {
                return Ok(auth_str[7..].to_string());
            }
        }

        // 也尝试从x-api-key header中获取
        if let Some(api_key_header) = headers.get("x-api-key") {
            let api_key = api_key_header.to_str()
                .map_err(|e| ClientError::HeaderParseError(format!("Invalid x-api-key header: {}", e)))?;
            return Ok(api_key.to_string());
        }

        Err(ClientError::HeaderParseError("No API key found in headers".to_string()))
    }

    /// 转换OpenAI格式的请求到Gemini格式
    fn convert_openai_to_gemini(&self, openai_body: &Value) -> ClientResult<Value> {
        let messages = openai_body.get("messages")
            .and_then(|m| m.as_array())
            .ok_or_else(|| ClientError::HeaderParseError("Missing messages field".to_string()))?;

        let mut contents = Vec::new();
        
        for message in messages {
            let role = message.get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("user");
            
            let content = message.get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("");

            // Gemini使用不同的角色名称
            let gemini_role = match role {
                "system" => "user", // Gemini没有system角色，转换为user
                "user" => "user",
                "assistant" => "model",
                _ => "user",
            };

            contents.push(json!({
                "role": gemini_role,
                "parts": [{"text": content}]
            }));
        }

        let mut gemini_body = json!({
            "contents": contents
        });

        // 添加生成配置
        if let Some(max_tokens) = openai_body.get("max_tokens") {
            gemini_body["generationConfig"] = json!({
                "maxOutputTokens": max_tokens
            });
        }

        if let Some(temperature) = openai_body.get("temperature") {
            if gemini_body.get("generationConfig").is_none() {
                gemini_body["generationConfig"] = json!({});
            }
            gemini_body["generationConfig"]["temperature"] = temperature.clone();
        }

        Ok(gemini_body)
    }

    /// 转换Gemini响应到OpenAI格式（返回JSON字符串）
    pub async fn convert_gemini_response_to_openai_json(&self, response: Response) -> ClientResult<String> {
        let status = response.status();

        if !status.is_success() {
            // 错误响应直接返回原始文本
            return Ok(response.text().await.map_err(|e| ClientError::RequestError(e))?);
        }

        let body_text = response.text().await
            .map_err(|e| ClientError::RequestError(e))?;

        debug!("Original Gemini response: {}", body_text);

        // 解析Gemini响应
        let gemini_response: Value = serde_json::from_str(&body_text)
            .map_err(|e| ClientError::HeaderParseError(format!("Failed to parse Gemini response: {}", e)))?;

        // 转换为OpenAI格式
        let openai_response = self.convert_gemini_to_openai_format(&gemini_response)?;

        debug!("Converted to OpenAI format: {}", serde_json::to_string_pretty(&openai_response).unwrap_or_default());

        // 返回JSON字符串
        serde_json::to_string(&openai_response)
            .map_err(|e| ClientError::HeaderParseError(format!("Failed to serialize response: {}", e)))
    }

    /// 转换Gemini响应格式到OpenAI格式
    fn convert_gemini_to_openai_format(&self, gemini_response: &Value) -> ClientResult<Value> {
        // 提取Gemini响应中的内容
        let candidates = gemini_response.get("candidates")
            .and_then(|c| c.as_array())
            .ok_or_else(|| ClientError::HeaderParseError("Missing candidates in Gemini response".to_string()))?;

        if candidates.is_empty() {
            return Err(ClientError::HeaderParseError("Empty candidates in Gemini response".to_string()));
        }

        let first_candidate = &candidates[0];
        let content = first_candidate.get("content")
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.as_array())
            .and_then(|parts| parts.get(0))
            .and_then(|part| part.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        // 构建OpenAI格式的响应
        let openai_response = json!({
            "id": format!("chatcmpl-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
            "object": "chat.completion",
            "created": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            "model": "gemini-pro", // 默认模型名
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 0, // Gemini API可能不提供这些信息
                "completion_tokens": 0,
                "total_tokens": 0
            }
        });

        Ok(openai_response)
    }
}

// 实现AIBackendClient trait
#[async_trait]
impl AIBackendClient for GeminiClient {
    fn backend_type(&self) -> BackendType {
        BackendType::Gemini
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
        _content_type: &headers::ContentType,
    ) -> Result<HeaderMap, ClientError> {
        let mut headers = HeaderMap::new();

        // Gemini使用API key而不是Bearer token，所以我们从Bearer token中提取API key
        let api_key = authorization.token();
        headers.insert("x-api-key", api_key.parse()
            .map_err(|e| ClientError::HeaderParseError(format!("Invalid API key: {}", e)))?);

        headers.insert("Content-Type", "application/json".parse()
            .map_err(|e| ClientError::HeaderParseError(format!("Invalid content type: {}", e)))?);

        Ok(headers)
    }

    async fn chat_completions_raw(
        &self,
        headers: HeaderMap,
        body: &Value,
    ) -> Result<reqwest::Response, ClientError> {
        self.chat_completions(headers, body).await
    }

    async fn models(&self, token: &str) -> Result<ClientResponse, ClientError> {
        let response = self.models(token).await?;
        let status = response.status().as_u16();
        let body = response.text().await
            .map_err(|e| ClientError::RequestError(e))?;

        Ok(ClientResponse::new(status, body))
    }

    fn convert_config_to_json(&self, config: &ChatCompletionConfig) -> Value {
        // 将ChatCompletionConfig转换为Gemini格式
        let openai_json = config.to_openai_json();
        // 然后转换为Gemini格式
        self.convert_openai_to_gemini(&openai_json).unwrap_or(openai_json)
    }

    fn supports_model(&self, model: &str) -> bool {
        // Gemini支持的模型
        model.starts_with("gemini-") || model.contains("gemini")
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "gemini-pro".to_string(),
            "gemini-pro-vision".to_string(),
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_gemini_client_creation() {
        let client = GeminiClient::new();
        assert_eq!(client.backend_type(), BackendType::Gemini);
        assert_eq!(client.base_url(), "https://generativelanguage.googleapis.com/v1beta");
    }

    #[test]
    fn test_gemini_client_with_custom_url() {
        let custom_url = "https://custom-gemini.com/v1";
        let client = GeminiClient::with_base_url(custom_url.to_string());
        assert_eq!(client.base_url(), custom_url);
    }

    #[test]
    fn test_openai_to_gemini_conversion() {
        let client = GeminiClient::new();
        let openai_body = json!({
            "model": "gemini-pro",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Hello!"},
                {"role": "assistant", "content": "Hi there!"},
                {"role": "user", "content": "How are you?"}
            ],
            "max_tokens": 100,
            "temperature": 0.7
        });

        let gemini_body = client.convert_openai_to_gemini(&openai_body).unwrap();

        // 验证转换结果
        assert!(gemini_body.get("contents").is_some());
        assert!(gemini_body.get("generationConfig").is_some());

        let contents = gemini_body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 4);

        // 验证角色转换
        assert_eq!(contents[0]["role"], "user"); // system -> user
        assert_eq!(contents[1]["role"], "user");
        assert_eq!(contents[2]["role"], "model"); // assistant -> model
        assert_eq!(contents[3]["role"], "user");
    }

    #[test]
    fn test_gemini_to_openai_conversion() {
        let client = GeminiClient::new();
        let gemini_response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "Hello! How can I help you today?"
                    }]
                }
            }]
        });

        let openai_response = client.convert_gemini_to_openai_format(&gemini_response).unwrap();

        // 验证转换结果
        assert_eq!(openai_response["object"], "chat.completion");
        assert!(openai_response.get("id").is_some());
        assert!(openai_response.get("created").is_some());
        assert_eq!(openai_response["model"], "gemini-pro");

        let choices = openai_response["choices"].as_array().unwrap();
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0]["message"]["role"], "assistant");
        assert_eq!(choices[0]["message"]["content"], "Hello! How can I help you today?");
        assert_eq!(choices[0]["finish_reason"], "stop");
    }

    #[test]
    fn test_supports_model() {
        let client = GeminiClient::new();

        // 测试支持的模型
        assert!(client.supports_model("gemini-pro"));
        assert!(client.supports_model("gemini-1.5-pro"));
        assert!(client.supports_model("gemini-pro-vision"));

        // 测试不支持的模型
        assert!(!client.supports_model("gpt-4"));
        assert!(!client.supports_model("claude-3"));
        assert!(!client.supports_model("random-model"));
    }

    #[test]
    fn test_supported_models() {
        let client = GeminiClient::new();
        let models = client.supported_models();

        assert!(!models.is_empty());
        assert!(models.contains(&"gemini-pro".to_string()));
        assert!(models.contains(&"gemini-1.5-pro".to_string()));
    }
}
