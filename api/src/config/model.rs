use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub providers: HashMap<String, Provider>,
    pub models: HashMap<String, ModelMapping>,
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
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            health_check_interval_seconds: default_health_check_interval(),
            request_timeout_seconds: default_request_timeout(),
            max_retries: default_max_retries(),
            circuit_breaker_failure_threshold: default_circuit_breaker_threshold(),
            circuit_breaker_timeout_seconds: default_circuit_breaker_timeout(),
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalanceStrategy {
    WeightedRandom,
    RoundRobin,
    LeastLatency,
    Failover,
    Random,
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
}
