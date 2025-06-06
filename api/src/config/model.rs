use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub providers: HashMap<String, Provider>,
    pub models: HashMap<String, ModelMapping>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Provider {
    pub name: String,
    pub base_url: String,
    pub api_key_env: String,
    pub models: Vec<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ModelMapping {
    pub name: String,
    pub backends: Vec<Backend>,
    #[serde(default)]
    pub strategy: LoadBalanceStrategy,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Backend {
    pub provider: String,
    pub model: String,
    pub weight: f64,
    #[serde(default)]
    pub priority: u8,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalanceStrategy {
    WeightedRandom,
    RoundRobin,
    LeastLatency,
    Failover,
}

impl Default for LoadBalanceStrategy {
    fn default() -> Self {
        Self::WeightedRandom
    }
}
