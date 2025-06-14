use std::time::Duration;
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use serde_json::Value;
use super::traits::{AIBackendClient, BackendType, ChatCompletionConfig};
use super::openai::OpenAIClient;
use super::claude::ClaudeClient;
use super::gemini::GeminiClient;
use super::types::{ClientError, ClientResponse};
use super::registry::get_global_registry;
use crate::config::model::ProviderBackendType;

/// 统一的客户端枚举，包装不同类型的AI后端客户端
#[derive(Clone)]
pub enum UnifiedClient {
    OpenAI(OpenAIClient),
    Claude(ClaudeClient),
    Gemini(GeminiClient),
}

#[async_trait]
impl AIBackendClient for UnifiedClient {
    fn backend_type(&self) -> BackendType {
        match self {
            UnifiedClient::OpenAI(client) => client.backend_type(),
            UnifiedClient::Claude(client) => client.backend_type(),
            UnifiedClient::Gemini(client) => client.backend_type(),
        }
    }

    fn base_url(&self) -> &str {
        match self {
            UnifiedClient::OpenAI(client) => client.base_url(),
            UnifiedClient::Claude(client) => client.base_url(),
            UnifiedClient::Gemini(client) => client.base_url(),
        }
    }

    fn with_timeout(self, timeout: Duration) -> Self {
        match self {
            UnifiedClient::OpenAI(client) => UnifiedClient::OpenAI(client.with_timeout(timeout)),
            UnifiedClient::Claude(client) => UnifiedClient::Claude(client.with_timeout(timeout)),
            UnifiedClient::Gemini(client) => UnifiedClient::Gemini(GeminiClient::with_base_url_and_timeout(client.base_url().to_string(), timeout)),
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
            UnifiedClient::Gemini(client) => client.build_request_headers(authorization, content_type),
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
            UnifiedClient::Gemini(client) => client.chat_completions(headers, body).await,
        }
    }

    async fn models(&self, token: &str) -> Result<ClientResponse, ClientError> {
        match self {
            UnifiedClient::OpenAI(client) => client.models(token).await,
            UnifiedClient::Claude(client) => client.models(token).await,
            UnifiedClient::Gemini(client) => {
                match client.models(token).await {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        let body = response.text().await.unwrap_or_default();
                        Ok(ClientResponse::new(status, body))
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    async fn health_check(&self, token: &str) -> Result<bool, ClientError> {
        match self {
            UnifiedClient::OpenAI(client) => client.health_check(token).await,
            UnifiedClient::Claude(client) => client.health_check(token).await,
            UnifiedClient::Gemini(client) => {
                match client.models(token).await {
                    Ok(response) => Ok(response.status().is_success()),
                    Err(_) => Ok(false),
                }
            }
        }
    }

    fn convert_config_to_json(&self, config: &ChatCompletionConfig) -> Value {
        match self {
            UnifiedClient::OpenAI(client) => client.convert_config_to_json(config),
            UnifiedClient::Claude(client) => client.convert_config_to_json(config),
            UnifiedClient::Gemini(client) => client.convert_config_to_json(config),
        }
    }

    fn supports_model(&self, model: &str) -> bool {
        match self {
            UnifiedClient::OpenAI(client) => client.supports_model(model),
            UnifiedClient::Claude(client) => client.supports_model(model),
            UnifiedClient::Gemini(client) => client.supports_model(model),
        }
    }

    fn supported_models(&self) -> Vec<String> {
        match self {
            UnifiedClient::OpenAI(client) => client.supported_models(),
            UnifiedClient::Claude(client) => client.supported_models(),
            UnifiedClient::Gemini(client) => client.supported_models(),
        }
    }
}

/// 客户端工厂
///
/// 现在使用插件化的客户端注册表系统，支持动态添加新的AI后端类型
pub struct ClientFactory;

impl ClientFactory {
    /// 根据配置的后端类型创建客户端（推荐使用）
    ///
    /// 现在使用全局客户端注册表来创建客户端，支持插件化扩展
    pub fn create_client_from_provider_type(
        provider_backend_type: ProviderBackendType,
        base_url: String,
        timeout: Duration,
    ) -> Result<UnifiedClient, ClientError> {
        get_global_registry().create_client(provider_backend_type, base_url, timeout)
    }

    /// 根据配置的后端类型创建客户端（使用自定义注册表）
    pub fn create_client_from_provider_type_with_registry(
        registry: &super::registry::ClientRegistry,
        provider_backend_type: ProviderBackendType,
        base_url: String,
        timeout: Duration,
    ) -> Result<UnifiedClient, ClientError> {
        registry.create_client(provider_backend_type, base_url, timeout)
    }

    /// 检查是否支持指定的后端类型
    pub fn supports_backend_type(provider_backend_type: &ProviderBackendType) -> bool {
        get_global_registry().supports_backend(provider_backend_type)
    }

    /// 获取所有支持的后端类型
    pub fn supported_backend_types() -> Vec<ProviderBackendType> {
        get_global_registry().supported_backends()
    }

    /// 获取注册的客户端类型数量
    pub fn registered_client_count() -> usize {
        get_global_registry().count()
    }

    /// 根据后端类型和配置创建客户端（兼容旧接口）
    #[deprecated(note = "Use create_client_from_provider_type instead for better type safety and plugin support")]
    pub fn create_client(
        backend_type: BackendType,
        base_url: String,
        timeout: Duration,
    ) -> Result<UnifiedClient, ClientError> {
        // 将旧的 BackendType 映射到新的 ProviderBackendType
        let provider_type = match backend_type {
            BackendType::OpenAI | BackendType::Custom(_) => ProviderBackendType::OpenAI,
            BackendType::Claude => ProviderBackendType::Claude,
            BackendType::Gemini => ProviderBackendType::Gemini,
        };

        Self::create_client_from_provider_type(provider_type, base_url, timeout)
    }

    /// 从base_url自动推断后端类型并创建客户端（已废弃，建议使用create_client_from_provider_type）
    #[deprecated(note = "Use create_client_from_provider_type instead to avoid hardcoded URL inference")]
    pub fn create_client_from_url(
        base_url: String,
        timeout: Duration,
    ) -> Result<UnifiedClient, ClientError> {
        let backend_type = BackendType::from_base_url(&base_url);
        // 将旧的 BackendType 映射到新的 ProviderBackendType
        let provider_type = match backend_type {
            BackendType::OpenAI | BackendType::Custom(_) => ProviderBackendType::OpenAI,
            BackendType::Claude => ProviderBackendType::Claude,
            BackendType::Gemini => ProviderBackendType::Gemini,
        };
        Self::create_client_from_provider_type(provider_type, base_url, timeout)
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

    /// 创建Gemini客户端
    pub fn create_gemini_client(
        base_url: String,
        timeout: Duration,
    ) -> GeminiClient {
        GeminiClient::with_base_url_and_timeout(base_url, timeout)
    }
}
