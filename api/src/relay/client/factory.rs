use std::time::Duration;
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use serde_json::Value;
use super::traits::{AIBackendClient, BackendType, ChatCompletionConfig};
use super::openai::OpenAIClient;
use super::claude::ClaudeClient;
use super::types::{ClientError, ClientResponse};
use crate::config::model::ProviderBackendType;

/// 统一的客户端枚举，包装不同类型的AI后端客户端
#[derive(Clone)]
pub enum UnifiedClient {
    OpenAI(OpenAIClient),
    Claude(ClaudeClient),
}

#[async_trait]
impl AIBackendClient for UnifiedClient {
    fn backend_type(&self) -> BackendType {
        match self {
            UnifiedClient::OpenAI(client) => client.backend_type(),
            UnifiedClient::Claude(client) => client.backend_type(),
        }
    }

    fn base_url(&self) -> &str {
        match self {
            UnifiedClient::OpenAI(client) => client.base_url(),
            UnifiedClient::Claude(client) => client.base_url(),
        }
    }

    fn with_timeout(self, timeout: Duration) -> Self {
        match self {
            UnifiedClient::OpenAI(client) => UnifiedClient::OpenAI(client.with_timeout(timeout)),
            UnifiedClient::Claude(client) => UnifiedClient::Claude(client.with_timeout(timeout)),
        }
    }

    fn build_request_headers(
        &self,
        authorization: &headers::Authorization<headers::authorization::Bearer>,
        content_type: &headers::ContentType,
    ) -> Result<HeaderMap, ClientError> {
        match self {
            UnifiedClient::OpenAI(client) => client.build_request_headers(authorization, content_type),
            UnifiedClient::Claude(client) => client.build_request_headers(authorization, content_type),
        }
    }

    async fn chat_completions_raw(
        &self,
        headers: HeaderMap,
        body: &Value,
    ) -> Result<reqwest::Response, ClientError> {
        match self {
            UnifiedClient::OpenAI(client) => client.chat_completions_raw(headers, body).await,
            UnifiedClient::Claude(client) => client.chat_completions_raw(headers, body).await,
        }
    }

    async fn models(&self, token: &str) -> Result<ClientResponse, ClientError> {
        match self {
            UnifiedClient::OpenAI(client) => client.models(token).await,
            UnifiedClient::Claude(client) => client.models(token).await,
        }
    }

    async fn health_check(&self, token: &str) -> Result<bool, ClientError> {
        match self {
            UnifiedClient::OpenAI(client) => client.health_check(token).await,
            UnifiedClient::Claude(client) => client.health_check(token).await,
        }
    }

    fn convert_config_to_json(&self, config: &ChatCompletionConfig) -> Value {
        match self {
            UnifiedClient::OpenAI(client) => client.convert_config_to_json(config),
            UnifiedClient::Claude(client) => client.convert_config_to_json(config),
        }
    }

    fn supports_model(&self, model: &str) -> bool {
        match self {
            UnifiedClient::OpenAI(client) => client.supports_model(model),
            UnifiedClient::Claude(client) => client.supports_model(model),
        }
    }

    fn supported_models(&self) -> Vec<String> {
        match self {
            UnifiedClient::OpenAI(client) => client.supported_models(),
            UnifiedClient::Claude(client) => client.supported_models(),
        }
    }
}

/// 客户端工厂，用于创建不同类型的AI后端客户端
pub struct ClientFactory;

impl ClientFactory {
    /// 根据配置的后端类型创建客户端（推荐使用）
    pub fn create_client_from_provider_type(
        provider_backend_type: ProviderBackendType,
        base_url: String,
        timeout: Duration,
    ) -> Result<UnifiedClient, ClientError> {
        match provider_backend_type {
            ProviderBackendType::OpenAI => {
                let client = OpenAIClient::with_base_url_and_timeout(base_url, timeout);
                Ok(UnifiedClient::OpenAI(client))
            }
            ProviderBackendType::Claude => {
                let client = ClaudeClient::with_base_url_and_timeout(base_url, timeout);
                Ok(UnifiedClient::Claude(client))
            }
            ProviderBackendType::Gemini => {
                // TODO: 实现Gemini客户端
                Err(ClientError::HeaderParseError(
                    "Gemini client not implemented yet".to_string()
                ))
            }
        }
    }

    /// 根据后端类型和配置创建客户端（兼容旧接口）
    pub fn create_client(
        backend_type: BackendType,
        base_url: String,
        timeout: Duration,
    ) -> Result<UnifiedClient, ClientError> {
        match backend_type {
            BackendType::OpenAI | BackendType::Custom(_) => {
                // OpenAI格式兼容大部分后端，包括自定义后端
                let client = OpenAIClient::with_base_url_and_timeout(base_url, timeout);
                Ok(UnifiedClient::OpenAI(client))
            }
            BackendType::Claude => {
                let client = ClaudeClient::with_base_url_and_timeout(base_url, timeout);
                Ok(UnifiedClient::Claude(client))
            }
            BackendType::Gemini => {
                // TODO: 实现Gemini客户端
                Err(ClientError::HeaderParseError(
                    "Gemini client not implemented yet".to_string()
                ))
            }
        }
    }

    /// 从base_url自动推断后端类型并创建客户端（已废弃，建议使用create_client_from_provider_type）
    #[deprecated(note = "Use create_client_from_provider_type instead to avoid hardcoded URL inference")]
    pub fn create_client_from_url(
        base_url: String,
        timeout: Duration,
    ) -> Result<UnifiedClient, ClientError> {
        let backend_type = BackendType::from_base_url(&base_url);
        Self::create_client(backend_type, base_url, timeout)
    }

    /// 创建OpenAI客户端
    pub fn create_openai_client(
        base_url: String,
        timeout: Duration,
    ) -> OpenAIClient {
        OpenAIClient::with_base_url_and_timeout(base_url, timeout)
    }

    /// 创建Claude客户端
    pub fn create_claude_client(
        base_url: String,
        timeout: Duration,
    ) -> ClaudeClient {
        ClaudeClient::with_base_url_and_timeout(base_url, timeout)
    }

    /// 获取支持的后端类型列表
    pub fn supported_backends() -> Vec<BackendType> {
        vec![
            BackendType::OpenAI,
            BackendType::Claude,
            // BackendType::Gemini, // TODO: 待实现
        ]
    }

    /// 检查后端类型是否受支持
    pub fn is_backend_supported(backend_type: &BackendType) -> bool {
        match backend_type {
            BackendType::OpenAI | BackendType::Claude => true,
            BackendType::Custom(_) => true, // 自定义后端使用OpenAI兼容格式
            BackendType::Gemini => false, // TODO: 待实现
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_type_inference() {
        assert_eq!(
            BackendType::from_base_url("https://api.openai.com/v1"),
            BackendType::OpenAI
        );
        assert_eq!(
            BackendType::from_base_url("https://api.anthropic.com"),
            BackendType::Claude
        );
        // 自定义URL现在默认使用OpenAI兼容格式
        assert_eq!(
            BackendType::from_base_url("https://custom-api.com/v1"),
            BackendType::OpenAI
        );
    }

    #[test]
    fn test_supported_backends() {
        let backends = ClientFactory::supported_backends();
        assert!(backends.contains(&BackendType::OpenAI));
        assert!(backends.contains(&BackendType::Claude));
    }

    #[test]
    fn test_backend_support_check() {
        assert!(ClientFactory::is_backend_supported(&BackendType::OpenAI));
        assert!(ClientFactory::is_backend_supported(&BackendType::Claude));
        assert!(ClientFactory::is_backend_supported(&BackendType::Custom("test".to_string())));
        assert!(!ClientFactory::is_backend_supported(&BackendType::Gemini));
    }

    #[tokio::test]
    async fn test_create_openai_client() {
        let client = ClientFactory::create_openai_client(
            "https://api.openai.com/v1".to_string(),
            Duration::from_secs(30),
        );
        assert_eq!(client.backend_type(), BackendType::OpenAI);
        assert_eq!(client.base_url(), "https://api.openai.com/v1");
    }

    #[tokio::test]
    async fn test_create_claude_client() {
        let client = ClientFactory::create_claude_client(
            "https://api.anthropic.com".to_string(),
            Duration::from_secs(30),
        );
        assert_eq!(client.backend_type(), BackendType::Claude);
        assert_eq!(client.base_url(), "https://api.anthropic.com");
    }

    #[tokio::test]
    async fn test_create_client_from_provider_type() {
        // 测试OpenAI类型
        let client = ClientFactory::create_client_from_provider_type(
            ProviderBackendType::OpenAI,
            "https://api.openai.com/v1".to_string(),
            Duration::from_secs(30),
        ).unwrap();
        assert_eq!(client.backend_type(), BackendType::OpenAI);

        // 测试Claude类型
        let client = ClientFactory::create_client_from_provider_type(
            ProviderBackendType::Claude,
            "https://api.anthropic.com".to_string(),
            Duration::from_secs(30),
        ).unwrap();
        assert_eq!(client.backend_type(), BackendType::Claude);

        // 测试自定义URL使用OpenAI格式
        let client = ClientFactory::create_client_from_provider_type(
            ProviderBackendType::OpenAI,
            "https://custom-api.com/v1".to_string(),
            Duration::from_secs(30),
        ).unwrap();
        assert_eq!(client.backend_type(), BackendType::OpenAI);
    }
}
