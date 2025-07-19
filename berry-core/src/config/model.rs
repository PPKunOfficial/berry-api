use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub providers: HashMap<String, Provider>,
    pub models: HashMap<String, ModelMapping>,
    pub users: HashMap<String, UserToken>,
    #[serde(default)]
    pub settings: GlobalSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GlobalSettings {
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_seconds: u64,
    #[serde(default = "default_request_timeout")]
    pub request_timeout_seconds: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_circuit_breaker_threshold")]
    pub circuit_breaker_failure_threshold: u32,
    #[serde(default = "default_circuit_breaker_timeout")]
    pub circuit_breaker_timeout_seconds: u64,
    // 新增健康检查相关配置
    #[serde(default = "default_recovery_check_interval")]
    pub recovery_check_interval_seconds: u64,
    #[serde(default = "default_max_internal_retries")]
    pub max_internal_retries: u32,
    #[serde(default = "default_health_check_timeout")]
    pub health_check_timeout_seconds: u64,
    // SmartAI 相关配置
    #[serde(default)]
    pub smart_ai: SmartAiSettings,
    // 数据库配置
    #[serde(default)]
    pub database: DatabaseSettings,
}

/// SmartAI 负载均衡配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SmartAiSettings {
    /// 初始信心度
    #[serde(default = "default_smart_ai_initial_confidence")]
    pub initial_confidence: f64,
    /// 最小信心度（保留恢复机会）
    #[serde(default = "default_smart_ai_min_confidence")]
    pub min_confidence: f64,
    /// 启用时间衰减
    #[serde(default = "default_true")]
    pub enable_time_decay: bool,
    /// 轻量级检查间隔（秒）
    #[serde(default = "default_smart_ai_lightweight_check_interval")]
    pub lightweight_check_interval_seconds: u64,
    /// 探索流量比例（用于测试其他后端）
    #[serde(default = "default_smart_ai_exploration_ratio")]
    pub exploration_ratio: f64,
    /// 非premium后端稳定性加成
    #[serde(default = "default_smart_ai_non_premium_stability_bonus")]
    pub non_premium_stability_bonus: f64,
    /// 信心度调整参数
    #[serde(default)]
    pub confidence_adjustments: SmartAiConfidenceAdjustments,
}

/// SmartAI 信心度调整参数
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SmartAiConfidenceAdjustments {
    #[serde(default = "default_smart_ai_success_boost")]
    pub success_boost: f64,
    #[serde(default = "default_smart_ai_network_error_penalty")]
    pub network_error_penalty: f64,
    #[serde(default = "default_smart_ai_auth_error_penalty")]
    pub auth_error_penalty: f64,
    #[serde(default = "default_smart_ai_rate_limit_penalty")]
    pub rate_limit_penalty: f64,
    #[serde(default = "default_smart_ai_server_error_penalty")]
    pub server_error_penalty: f64,
    #[serde(default = "default_smart_ai_model_error_penalty")]
    pub model_error_penalty: f64,
    #[serde(default = "default_smart_ai_timeout_penalty")]
    pub timeout_penalty: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseSettings {
    #[serde(default = "default_database_url")]
    pub url: String,
    #[serde(default = "default_database_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_database_timeout")]
    pub timeout_seconds: u64,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            max_connections: default_database_max_connections(),
            timeout_seconds: default_database_timeout(),
        }
    }
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            health_check_interval_seconds: default_health_check_interval(),
            request_timeout_seconds: default_request_timeout(),
            max_retries: default_max_retries(),
            circuit_breaker_failure_threshold: default_circuit_breaker_threshold(),
            circuit_breaker_timeout_seconds: default_circuit_breaker_timeout(),
            recovery_check_interval_seconds: default_recovery_check_interval(),
            max_internal_retries: default_max_internal_retries(),
            health_check_timeout_seconds: default_health_check_timeout(),
            smart_ai: SmartAiSettings::default(),
            database: DatabaseSettings::default(),
        }
    }
}

impl Default for SmartAiSettings {
    fn default() -> Self {
        Self {
            initial_confidence: default_smart_ai_initial_confidence(),
            min_confidence: default_smart_ai_min_confidence(),
            enable_time_decay: true,
            lightweight_check_interval_seconds: default_smart_ai_lightweight_check_interval(),
            exploration_ratio: default_smart_ai_exploration_ratio(),
            non_premium_stability_bonus: default_smart_ai_non_premium_stability_bonus(),
            confidence_adjustments: SmartAiConfidenceAdjustments::default(),
        }
    }
}

impl Default for SmartAiConfidenceAdjustments {
    fn default() -> Self {
        Self {
            success_boost: default_smart_ai_success_boost(),
            network_error_penalty: default_smart_ai_network_error_penalty(),
            auth_error_penalty: default_smart_ai_auth_error_penalty(),
            rate_limit_penalty: default_smart_ai_rate_limit_penalty(),
            server_error_penalty: default_smart_ai_server_error_penalty(),
            model_error_penalty: default_smart_ai_model_error_penalty(),
            timeout_penalty: default_smart_ai_timeout_penalty(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Provider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub models: Vec<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_request_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 后端类型，明确指定使用什么API格式
    #[serde(default)]
    pub backend_type: ProviderBackendType,
}

/// Provider的后端类型
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderBackendType {
    /// OpenAI兼容格式（默认）
    #[default]
    OpenAI,
    /// Anthropic Claude格式
    Claude,
    /// Google Gemini格式
    Gemini,
}

/// 计费模式
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BillingMode {
    /// 按token计费 - 执行主动健康检查
    #[default]
    PerToken,
    /// 按请求计费 - 跳过主动检查，使用被动验证
    PerRequest,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModelMapping {
    pub name: String,
    pub backends: Vec<Backend>,
    #[serde(default)]
    pub strategy: LoadBalanceStrategy,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Backend {
    pub provider: String,
    pub model: String,
    #[serde(default = "default_weight")]
    pub weight: f64,
    #[serde(default)]
    pub priority: u8,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub billing_mode: BillingMode,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserToken {
    pub name: String,
    pub token: String,
    #[serde(default)]
    pub allowed_models: Vec<String>, // 空表示允许所有模型
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub rate_limit: Option<RateLimit>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
}

// Default value functions
fn default_true() -> bool {
    true
}

fn default_weight() -> f64 {
    1.0
}

fn default_health_check_interval() -> u64 {
    30
}

fn default_request_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

fn default_circuit_breaker_threshold() -> u32 {
    5
}

fn default_circuit_breaker_timeout() -> u64 {
    60
}

fn default_recovery_check_interval() -> u64 {
    120 // 2分钟检查一次恢复
}

fn default_max_internal_retries() -> u32 {
    2 // 内部最多重试2次
}

fn default_health_check_timeout() -> u64 {
    10 // 健康检查超时10秒
}

// SmartAI 默认值函数
fn default_smart_ai_initial_confidence() -> f64 {
    0.8
}

fn default_smart_ai_min_confidence() -> f64 {
    0.05
}

fn default_smart_ai_lightweight_check_interval() -> u64 {
    600 // 10分钟
}

fn default_smart_ai_exploration_ratio() -> f64 {
    0.2
}

fn default_smart_ai_non_premium_stability_bonus() -> f64 {
    1.1
}

fn default_smart_ai_success_boost() -> f64 {
    0.1
}

fn default_smart_ai_network_error_penalty() -> f64 {
    0.3
}

fn default_smart_ai_auth_error_penalty() -> f64 {
    0.8
}

fn default_smart_ai_rate_limit_penalty() -> f64 {
    0.1
}

fn default_smart_ai_server_error_penalty() -> f64 {
    0.2
}

fn default_smart_ai_model_error_penalty() -> f64 {
    0.3
}

fn default_smart_ai_timeout_penalty() -> f64 {
    0.2
}

// 数据库默认值函数
fn default_database_url() -> String {
    "postgres://localhost/berry".to_string()
}

fn default_database_max_connections() -> u32 {
    5
}

fn default_database_timeout() -> u64 {
    30
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalanceStrategy {
    /// 智能AI负载均衡 - 基于客户流量的小流量健康检查，成本感知
    SmartAi,
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        Self::SmartAi
    }
}

impl Config {
    /// 验证配置的有效性
    pub fn validate(&self) -> Result<()> {
        // 验证providers
        for (provider_id, provider) in &self.providers {
            self.validate_provider_config(provider_id, provider)?;
        }

        // 验证models
        for (model_id, model) in &self.models {
            self.validate_model_config(model_id, model)?;
        }

        // 验证用户令牌
        for (user_id, user) in &self.users {
            self.validate_user_config(user_id, user)?;
        }

        Ok(())
    }

    /// 验证单个Provider配置的有效性
    fn validate_provider_config(&self, provider_id: &str, provider: &Provider) -> Result<()> {
        // 基本字段验证
        if provider.name.is_empty() {
            anyhow::bail!("Provider '{}' has empty name", provider_id);
        }

        if provider.base_url.is_empty() {
            anyhow::bail!("Provider '{}' has empty base_url", provider_id);
        }

        if provider.api_key.is_empty() {
            anyhow::bail!("Provider '{}' has empty api_key", provider_id);
        }

        if provider.models.is_empty() {
            anyhow::bail!("Provider '{}' has no models defined", provider_id);
        }

        // URL格式验证
        if !provider.base_url.starts_with("http://") && !provider.base_url.starts_with("https://") {
            anyhow::bail!("Provider '{}' has invalid base_url format: '{}'. Must start with http:// or https://",
                provider_id, provider.base_url);
        }

        // API密钥格式验证（基本长度检查）
        if provider.api_key.len() < 10 {
            anyhow::bail!(
                "Provider '{}' has API key that is too short (minimum 10 characters)",
                provider_id
            );
        }

        // 超时值验证
        if provider.timeout_seconds == 0 {
            anyhow::bail!(
                "Provider '{}' has invalid timeout_seconds: cannot be 0",
                provider_id
            );
        }

        if provider.timeout_seconds > 300 {
            anyhow::bail!(
                "Provider '{}' has timeout_seconds too large: {} (maximum 300 seconds)",
                provider_id,
                provider.timeout_seconds
            );
        }

        // 重试次数验证
        if provider.max_retries > 10 {
            anyhow::bail!(
                "Provider '{}' has max_retries too large: {} (maximum 10)",
                provider_id,
                provider.max_retries
            );
        }

        // 验证模型名称不为空
        for model_name in &provider.models {
            if model_name.is_empty() {
                anyhow::bail!(
                    "Provider '{}' has empty model name in models list",
                    provider_id
                );
            }
        }

        // 验证自定义头部格式
        for (header_name, header_value) in &provider.headers {
            if header_name.is_empty() {
                anyhow::bail!("Provider '{}' has empty header name", provider_id);
            }
            if header_value.is_empty() {
                anyhow::bail!(
                    "Provider '{}' has empty header value for header '{}'",
                    provider_id,
                    header_name
                );
            }
        }

        Ok(())
    }

    /// 验证单个Model配置的有效性
    fn validate_model_config(&self, model_id: &str, model: &ModelMapping) -> Result<()> {
        // 基本字段验证
        if model.name.is_empty() {
            anyhow::bail!("Model '{}' has empty name", model_id);
        }

        if model.backends.is_empty() {
            anyhow::bail!("Model '{}' has no backends defined", model_id);
        }

        // 验证模型名称格式（不能包含特殊字符）
        if model.name.contains(' ') || model.name.contains('\t') || model.name.contains('\n') {
            anyhow::bail!(
                "Model '{}' has invalid name format: '{}' (cannot contain whitespace)",
                model_id,
                model.name
            );
        }

        // 验证backends
        let mut total_weight = 0.0;
        for backend in &model.backends {
            self.validate_backend_config(model_id, backend)?;
            if backend.enabled {
                total_weight += backend.weight;
            }
        }

        // 检查是否有可用的后端
        if total_weight <= 0.0 {
            anyhow::bail!(
                "Model '{}' has no enabled backends with positive weight",
                model_id
            );
        }

        Ok(())
    }

    /// 验证单个Backend配置的有效性
    fn validate_backend_config(&self, model_id: &str, backend: &Backend) -> Result<()> {
        // 验证provider引用
        if !self.providers.contains_key(&backend.provider) {
            anyhow::bail!(
                "Model '{}' references unknown provider '{}'",
                model_id,
                backend.provider
            );
        }

        let provider = &self.providers[&backend.provider];

        // 验证模型在provider中存在
        if !provider.models.contains(&backend.model) {
            anyhow::bail!(
                "Model '{}' backend references model '{}' not available in provider '{}'",
                model_id,
                backend.model,
                backend.provider
            );
        }

        // 验证权重
        if backend.weight <= 0.0 {
            anyhow::bail!(
                "Model '{}' backend has invalid weight: {} (must be positive)",
                model_id,
                backend.weight
            );
        }

        if backend.weight > 100.0 {
            anyhow::bail!(
                "Model '{}' backend has weight too large: {} (maximum 100.0)",
                model_id,
                backend.weight
            );
        }

        // 验证优先级
        if backend.priority > 10 {
            anyhow::bail!(
                "Model '{}' backend has priority too high: {} (maximum 10)",
                model_id,
                backend.priority
            );
        }

        // 验证标签格式
        for tag in &backend.tags {
            if tag.is_empty() {
                anyhow::bail!("Model '{}' backend has empty tag", model_id);
            }
            if tag.contains(' ') {
                anyhow::bail!(
                    "Model '{}' backend has invalid tag format: '{}' (cannot contain spaces)",
                    model_id,
                    tag
                );
            }
        }

        Ok(())
    }

    /// 验证单个User配置的有效性
    fn validate_user_config(&self, user_id: &str, user: &UserToken) -> Result<()> {
        // 基本字段验证
        if user.name.is_empty() {
            anyhow::bail!("User '{}' has empty name", user_id);
        }

        if user.token.is_empty() {
            anyhow::bail!("User '{}' has empty token", user_id);
        }

        // Token格式验证（基本长度和格式检查）
        if user.token.len() < 16 {
            anyhow::bail!(
                "User '{}' has token that is too short (minimum 16 characters)",
                user_id
            );
        }

        if user.token.contains(' ') || user.token.contains('\t') || user.token.contains('\n') {
            anyhow::bail!(
                "User '{}' has invalid token format (cannot contain whitespace)",
                user_id
            );
        }

        // 验证允许的模型是否存在
        for model_name in &user.allowed_models {
            if model_name.is_empty() {
                anyhow::bail!("User '{}' has empty model name in allowed_models", user_id);
            }

            if !self.models.contains_key(model_name) {
                anyhow::bail!(
                    "User '{}' references unknown model '{}'",
                    user_id,
                    model_name
                );
            }
        }

        // 验证速率限制配置
        if let Some(rate_limit) = &user.rate_limit {
            self.validate_rate_limit_config(user_id, rate_limit)?;
        }

        // 验证标签格式
        for tag in &user.tags {
            if tag.is_empty() {
                anyhow::bail!("User '{}' has empty tag", user_id);
            }
            if tag.contains(' ') {
                anyhow::bail!(
                    "User '{}' has invalid tag format: '{}' (cannot contain spaces)",
                    user_id,
                    tag
                );
            }
        }

        Ok(())
    }

    /// 验证速率限制配置的有效性
    fn validate_rate_limit_config(&self, user_id: &str, rate_limit: &RateLimit) -> Result<()> {
        if rate_limit.requests_per_minute == 0 {
            anyhow::bail!(
                "User '{}' has invalid requests_per_minute: cannot be 0",
                user_id
            );
        }

        if rate_limit.requests_per_hour == 0 {
            anyhow::bail!(
                "User '{}' has invalid requests_per_hour: cannot be 0",
                user_id
            );
        }

        if rate_limit.requests_per_day == 0 {
            anyhow::bail!(
                "User '{}' has invalid requests_per_day: cannot be 0",
                user_id
            );
        }

        // 逻辑一致性检查
        if rate_limit.requests_per_minute > rate_limit.requests_per_hour {
            anyhow::bail!("User '{}' has inconsistent rate limits: requests_per_minute ({}) > requests_per_hour ({})",
                user_id, rate_limit.requests_per_minute, rate_limit.requests_per_hour);
        }

        if rate_limit.requests_per_hour > rate_limit.requests_per_day {
            anyhow::bail!("User '{}' has inconsistent rate limits: requests_per_hour ({}) > requests_per_day ({})",
                user_id, rate_limit.requests_per_hour, rate_limit.requests_per_day);
        }

        // 合理性检查（防止过大的值）
        if rate_limit.requests_per_minute > 1000 {
            anyhow::bail!(
                "User '{}' has requests_per_minute too large: {} (maximum 1000)",
                user_id,
                rate_limit.requests_per_minute
            );
        }

        if rate_limit.requests_per_hour > 60000 {
            anyhow::bail!(
                "User '{}' has requests_per_hour too large: {} (maximum 60000)",
                user_id,
                rate_limit.requests_per_hour
            );
        }

        if rate_limit.requests_per_day > 1440000 {
            anyhow::bail!(
                "User '{}' has requests_per_day too large: {} (maximum 1440000)",
                user_id,
                rate_limit.requests_per_day
            );
        }

        Ok(())
    }

    /// 获取指定模型的所有可用后端
    pub fn get_available_backends(&self, model_name: &str) -> Option<Vec<&Backend>> {
        self.models.get(model_name).map(|model| {
            model
                .backends
                .iter()
                .filter(|backend| {
                    backend.enabled
                        && self
                            .providers
                            .get(&backend.provider)
                            .is_some_and(|p| p.enabled)
                })
                .collect()
        })
    }

    /// 获取指定provider的配置
    pub fn get_provider(&self, provider_id: &str) -> Option<&Provider> {
        self.providers.get(provider_id)
    }

    /// 获取指定model的配置
    pub fn get_model(&self, model_name: &str) -> Option<&ModelMapping> {
        self.models.get(model_name)
    }

    /// 获取所有可用的模型名称
    pub fn get_available_models(&self) -> Vec<String> {
        self.models
            .iter()
            .filter(|(_, model)| model.enabled)
            .map(|(_, model)| model.name.clone())
            .collect()
    }

    /// 验证用户令牌
    pub fn validate_user_token(&self, token: &str) -> Option<&UserToken> {
        self.users
            .values()
            .find(|user| user.enabled && user.token == token)
    }

    /// 检查用户是否有权限访问指定模型（通过模型名称）
    pub fn user_can_access_model(&self, user: &UserToken, model_name: &str) -> bool {
        // 如果allowed_models为空，表示允许访问所有模型
        if user.allowed_models.is_empty() {
            return true;
        }

        // 需要找到模型名称对应的模型ID，然后检查权限
        for (model_id, model) in &self.models {
            if model.name == model_name && model.enabled {
                return user.allowed_models.contains(model_id);
            }
        }

        false
    }

    /// 获取用户信息
    pub fn get_user(&self, user_id: &str) -> Option<&UserToken> {
        self.users.get(user_id)
    }

    /// 检查用户是否有指定标签
    pub fn user_has_tag(&self, user: &UserToken, tag: &str) -> bool {
        user.tags.contains(&tag.to_string())
    }

    /// 根据用户标签过滤后端
    pub fn filter_backends_by_user_tags(
        &self,
        backends: &[Backend],
        user: &UserToken,
    ) -> Vec<Backend> {
        // 如果用户没有标签，返回所有后端
        if user.tags.is_empty() {
            return backends.to_vec();
        }

        // 过滤出与用户标签匹配的后端
        backends
            .iter()
            .filter(|backend| {
                // 如果后端没有标签，允许所有用户访问
                if backend.tags.is_empty() {
                    return true;
                }

                // 检查是否有共同标签
                backend
                    .tags
                    .iter()
                    .any(|backend_tag| user.tags.contains(backend_tag))
            })
            .cloned()
            .collect()
    }

    /// 获取具有指定标签的用户列表
    pub fn get_users_with_tag(&self, tag: &str) -> Vec<&UserToken> {
        self.users
            .values()
            .filter(|user| user.tags.contains(&tag.to_string()))
            .collect()
    }

    /// 获取具有指定标签的后端列表
    pub fn get_backends_with_tag(&self, model_name: &str, tag: &str) -> Option<Vec<&Backend>> {
        self.models.get(model_name).map(|model| {
            model
                .backends
                .iter()
                .filter(|backend| backend.tags.contains(&tag.to_string()))
                .collect()
        })
    }

    /// 获取用户可访问的模型列表
    pub fn get_user_available_models(&self, user: &UserToken) -> Vec<String> {
        if user.allowed_models.is_empty() {
            // 如果没有限制，返回所有可用模型的名称（面向客户的名称）
            self.get_available_models()
        } else {
            // 返回用户允许的且系统中存在的模型的面向客户名称
            user.allowed_models
                .iter()
                .filter_map(|model_id| {
                    // 检查模型ID是否存在且启用
                    self.models
                        .get(model_id)
                        .filter(|model| model.enabled)
                        .map(|model| model.name.clone()) // 返回面向客户的模型名称
                })
                .collect()
        }
    }
}
