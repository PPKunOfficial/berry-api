use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

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
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderBackendType {
    /// OpenAI兼容格式（默认）
    OpenAI,
    /// Anthropic Claude格式
    Claude,
    /// Google Gemini格式
    Gemini,
}

impl Default for ProviderBackendType {
    fn default() -> Self {
        ProviderBackendType::OpenAI
    }
}

/// 计费模式
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BillingMode {
    /// 按token计费 - 执行主动健康检查
    PerToken,
    /// 按请求计费 - 跳过主动检查，使用被动验证
    PerRequest,
}

impl Default for BillingMode {
    fn default() -> Self {
        BillingMode::PerToken
    }
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalanceStrategy {
    WeightedRandom,
    RoundRobin,
    LeastLatency,
    Failover,
    Random,
    WeightedFailover,
    /// 智能权重恢复策略 - 支持按请求计费的渐进式权重恢复
    SmartWeightedFailover,
    /// 智能AI负载均衡 - 基于客户流量的小流量健康检查，成本感知
    SmartAi,
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        Self::WeightedRandom
    }
}

impl Config {
    /// 验证配置的有效性
    pub fn validate(&self) -> Result<()> {
        // 验证providers
        for (provider_id, provider) in &self.providers {
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
        }

        // 验证models
        for (model_id, model) in &self.models {
            if model.name.is_empty() {
                anyhow::bail!("Model '{}' has empty name", model_id);
            }
            if model.backends.is_empty() {
                anyhow::bail!("Model '{}' has no backends defined", model_id);
            }

            // 验证backends
            for backend in &model.backends {
                if !self.providers.contains_key(&backend.provider) {
                    anyhow::bail!(
                        "Model '{}' references unknown provider '{}'",
                        model_id, backend.provider
                    );
                }

                let provider = &self.providers[&backend.provider];
                if !provider.models.contains(&backend.model) {
                    anyhow::bail!(
                        "Model '{}' backend references model '{}' not available in provider '{}'",
                        model_id, backend.model, backend.provider
                    );
                }

                if backend.weight <= 0.0 {
                    anyhow::bail!(
                        "Model '{}' backend has invalid weight: {}",
                        model_id, backend.weight
                    );
                }
            }
        }

        // 验证用户令牌
        for (user_id, user) in &self.users {
            if user.name.is_empty() {
                anyhow::bail!("User '{}' has empty name", user_id);
            }
            if user.token.is_empty() {
                anyhow::bail!("User '{}' has empty token", user_id);
            }

            // 验证允许的模型是否存在
            for model_name in &user.allowed_models {
                if !self.models.contains_key(model_name) {
                    anyhow::bail!(
                        "User '{}' references unknown model '{}'",
                        user_id, model_name
                    );
                }
            }
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
                            .map_or(false, |p| p.enabled)
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
