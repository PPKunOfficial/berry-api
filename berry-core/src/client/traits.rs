use super::types::{ClientError, ClientResponse};
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use serde_json::{json, Value};
use std::fmt;
use std::time::Duration;

/// 后端类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    OpenAI,
    Claude,
    Gemini,
    Custom(String),
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendType::OpenAI => write!(f, "OpenAI"),
            BackendType::Claude => write!(f, "Claude"),
            BackendType::Gemini => write!(f, "Gemini"),
            BackendType::Custom(name) => write!(f, "Custom({name})"),
        }
    }
}

impl BackendType {
    /// 从base_url推断后端类型
    /// 大部分自定义后端都使用OpenAI兼容的API格式
    pub fn from_base_url(base_url: &str) -> Self {
        let url_lower = base_url.to_lowercase();

        if url_lower.contains("anthropic.com") || url_lower.contains("claude") {
            BackendType::Claude
        } else if url_lower.contains("googleapis.com") || url_lower.contains("gemini") {
            BackendType::Gemini
        } else {
            // 默认使用OpenAI兼容格式，包括：
            // - 官方OpenAI API
            // - 各种代理服务
            // - 自定义后端
            // 这样可以支持更多的后端服务
            BackendType::OpenAI
        }
    }
}

/// 聊天消息角色
#[derive(Debug, Clone)]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

impl ChatRole {
    pub fn as_str(&self) -> &str {
        match self {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
        }
    }
}

/// 聊天消息
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// 聊天完成请求的配置
#[derive(Debug, Clone)]
pub struct ChatCompletionConfig {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
}

impl ChatCompletionConfig {
    /// 从JSON body创建配置
    pub fn from_json(body: &Value) -> Result<Self, ClientError> {
        let model = body
            .get("model")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClientError::HeaderParseError("Missing 'model' field".to_string()))?
            .to_string();

        let messages_json = body
            .get("messages")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ClientError::HeaderParseError("Missing or invalid 'messages' field".to_string())
            })?;

        let mut messages = Vec::new();
        for msg in messages_json {
            let role_str = msg.get("role").and_then(|v| v.as_str()).ok_or_else(|| {
                ClientError::HeaderParseError("Missing 'role' field in message".to_string())
            })?;

            let role = match role_str {
                "system" => ChatRole::System,
                "user" => ChatRole::User,
                "assistant" => ChatRole::Assistant,
                _ => {
                    return Err(ClientError::HeaderParseError(format!(
                        "Invalid role: {role_str}"
                    )))
                }
            };

            let content = msg
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ClientError::HeaderParseError("Missing 'content' field in message".to_string())
                })?
                .to_string();

            messages.push(ChatMessage { role, content });
        }

        let stream = body.get("stream").and_then(|v| v.as_bool());

        Ok(Self {
            model,
            messages,
            stream,
            temperature: body
                .get("temperature")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32),
            max_tokens: body
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            top_p: body.get("top_p").and_then(|v| v.as_f64()).map(|v| v as f32),
            frequency_penalty: body
                .get("frequency_penalty")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32),
            presence_penalty: body
                .get("presence_penalty")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32),
            stop: body.get("stop").and_then(|v| {
                if let Some(arr) = v.as_array() {
                    Some(
                        arr.iter()
                            .filter_map(|s| s.as_str().map(|s| s.to_string()))
                            .collect(),
                    )
                } else if let Some(s) = v.as_str() {
                    Some(vec![s.to_string()])
                } else {
                    None
                }
            }),
        })
    }

    /// 转换为OpenAI格式的JSON
    pub fn to_openai_json(&self) -> Value {
        let messages: Vec<Value> = self
            .messages
            .iter()
            .map(|msg| {
                json!({
                    "role": msg.role.as_str(),
                    "content": msg.content
                })
            })
            .collect();

        let mut json = json!({
            "model": self.model,
            "messages": messages
        });

        if let Some(stream) = self.stream {
            json["stream"] = stream.into();
        }
        if let Some(temp) = self.temperature {
            json["temperature"] = temp.into();
        }
        if let Some(max_tokens) = self.max_tokens {
            json["max_tokens"] = max_tokens.into();
        }
        if let Some(top_p) = self.top_p {
            json["top_p"] = top_p.into();
        }
        if let Some(freq_penalty) = self.frequency_penalty {
            json["frequency_penalty"] = freq_penalty.into();
        }
        if let Some(pres_penalty) = self.presence_penalty {
            json["presence_penalty"] = pres_penalty.into();
        }
        if let Some(stop) = &self.stop {
            json["stop"] = stop.clone().into();
        }

        json
    }
}

/// 通用的AI后端客户端trait
#[async_trait]
pub trait AIBackendClient: Send + Sync + Clone {
    /// 获取后端类型
    fn backend_type(&self) -> BackendType;

    /// 获取base URL
    fn base_url(&self) -> &str;

    /// 设置超时时间
    fn with_timeout(self, timeout: Duration) -> Self;

    /// 构建请求头
    fn build_request_headers(
        &self,
        authorization: &headers::Authorization<headers::authorization::Bearer>,
        content_type: &headers::ContentType,
    ) -> Result<HeaderMap, ClientError>;

    /// 发送聊天完成请求（原始JSON格式）
    async fn chat_completions_raw(
        &self,
        headers: HeaderMap,
        body: &Value,
    ) -> Result<reqwest::Response, ClientError>;

    /// 发送聊天完成请求（结构化配置）
    async fn chat_completions(
        &self,
        headers: HeaderMap,
        config: &ChatCompletionConfig,
    ) -> Result<reqwest::Response, ClientError> {
        // 默认实现：转换为对应格式的JSON并调用raw方法
        let body = self.convert_config_to_json(config);
        self.chat_completions_raw(headers, &body).await
    }

    /// 获取模型列表
    async fn models(&self, token: &str) -> Result<ClientResponse, ClientError>;

    /// 健康检查
    async fn health_check(&self, token: &str) -> Result<bool, ClientError> {
        // 默认实现：通过获取模型列表来检查健康状态
        match self.models(token).await {
            Ok(response) => Ok(response.is_success),
            Err(_) => Ok(false),
        }
    }

    /// 将ChatCompletionConfig转换为后端特定的JSON格式
    fn convert_config_to_json(&self, config: &ChatCompletionConfig) -> Value {
        // 默认使用OpenAI格式
        config.to_openai_json()
    }

    /// 验证模型名称是否支持
    fn supports_model(&self, _model: &str) -> bool {
        // 默认实现：支持所有模型
        true
    }

    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<String> {
        // 默认实现：返回空列表，表示支持所有模型
        vec![]
    }
}
