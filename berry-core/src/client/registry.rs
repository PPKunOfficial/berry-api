use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

// use super::traits::{AIBackendClient, BackendType}; // 暂时不需要，但保留以备将来使用
use super::claude::ClaudeClient;
use super::factory::UnifiedClient;
use super::gemini::GeminiClient;
use super::openai::OpenAIClient;
use super::types::ClientError;
use crate::config::model::ProviderBackendType;

/// 客户端构建器函数类型
///
/// 接受 base_url 和 timeout，返回一个 UnifiedClient
pub type ClientBuilder =
    Box<dyn Fn(String, Duration) -> Result<UnifiedClient, ClientError> + Send + Sync>;

/// 客户端注册表
///
/// 支持动态注册和创建不同类型的AI后端客户端
/// 提供插件化的客户端管理机制
pub struct ClientRegistry {
    builders: Arc<RwLock<HashMap<ProviderBackendType, ClientBuilder>>>,
}

impl ClientRegistry {
    /// 创建新的客户端注册表
    pub fn new() -> Self {
        let registry = Self {
            builders: Arc::new(RwLock::new(HashMap::new())),
        };

        // 注册默认的客户端类型
        registry.register_default_clients();
        registry
    }

    /// 注册默认的客户端类型
    fn register_default_clients(&self) {
        // 注册 OpenAI 客户端
        self.register_client(
            ProviderBackendType::OpenAI,
            Box::new(|base_url, timeout| {
                let client = OpenAIClient::with_base_url_and_timeout(base_url, timeout);
                Ok(UnifiedClient::OpenAI(client))
            }),
        );

        // 注册 Claude 客户端
        self.register_client(
            ProviderBackendType::Claude,
            Box::new(|base_url, timeout| {
                let client = ClaudeClient::with_base_url_and_timeout(base_url, timeout);
                Ok(UnifiedClient::Claude(client))
            }),
        );

        // 注册 Gemini 客户端
        self.register_client(
            ProviderBackendType::Gemini,
            Box::new(|base_url, timeout| {
                let client = GeminiClient::with_base_url_and_timeout(base_url, timeout);
                Ok(UnifiedClient::Gemini(client))
            }),
        );
    }

    /// 注册新的客户端类型
    ///
    /// # 参数
    /// * `backend_type` - 后端类型
    /// * `builder` - 客户端构建器函数
    pub fn register_client(&self, backend_type: ProviderBackendType, builder: ClientBuilder) {
        if let Ok(mut builders) = self.builders.write() {
            tracing::info!(
                "Registered client builder for backend type: {:?}",
                backend_type
            );
            builders.insert(backend_type, builder);
        } else {
            tracing::error!("Failed to acquire write lock for client registry");
        }
    }

    /// 创建客户端
    ///
    /// # 参数
    /// * `backend_type` - 后端类型
    /// * `base_url` - 基础URL
    /// * `timeout` - 超时时间
    ///
    /// # 返回
    /// 成功时返回 UnifiedClient，失败时返回 ClientError
    pub fn create_client(
        &self,
        backend_type: ProviderBackendType,
        base_url: String,
        timeout: Duration,
    ) -> Result<UnifiedClient, ClientError> {
        let builders = self.builders.read().map_err(|e| {
            ClientError::HeaderParseError(format!("Failed to acquire read lock: {}", e))
        })?;

        let builder = builders.get(&backend_type).ok_or_else(|| {
            ClientError::HeaderParseError(format!(
                "No client builder registered for backend type: {:?}",
                backend_type
            ))
        })?;

        builder(base_url, timeout)
    }

    /// 检查是否支持指定的后端类型
    pub fn supports_backend(&self, backend_type: &ProviderBackendType) -> bool {
        if let Ok(builders) = self.builders.read() {
            builders.contains_key(backend_type)
        } else {
            false
        }
    }

    /// 获取所有支持的后端类型
    pub fn supported_backends(&self) -> Vec<ProviderBackendType> {
        if let Ok(builders) = self.builders.read() {
            builders.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// 移除客户端类型注册
    pub fn unregister_client(&self, backend_type: &ProviderBackendType) -> bool {
        if let Ok(mut builders) = self.builders.write() {
            let removed = builders.remove(backend_type).is_some();
            if removed {
                tracing::info!(
                    "Unregistered client builder for backend type: {:?}",
                    backend_type
                );
            }
            removed
        } else {
            false
        }
    }

    /// 获取注册的客户端类型数量
    pub fn count(&self) -> usize {
        if let Ok(builders) = self.builders.read() {
            builders.len()
        } else {
            0
        }
    }
}

impl Default for ClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局客户端注册表实例
static GLOBAL_REGISTRY: std::sync::OnceLock<ClientRegistry> = std::sync::OnceLock::new();

/// 获取全局客户端注册表
pub fn get_global_registry() -> &'static ClientRegistry {
    GLOBAL_REGISTRY.get_or_init(ClientRegistry::new)
}

/// 注册全局客户端类型
///
/// 这是一个便利函数，用于向全局注册表注册新的客户端类型
pub fn register_global_client(backend_type: ProviderBackendType, builder: ClientBuilder) {
    get_global_registry().register_client(backend_type, builder);
}
