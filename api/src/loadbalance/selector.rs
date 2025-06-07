use crate::config::model::{Backend, LoadBalanceStrategy, ModelMapping};
use anyhow::Result;
use rand::distributions::{Distribution, WeightedIndex};
use rand::{thread_rng, Rng};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;

pub struct BackendSelector {
    mapping: ModelMapping,
    round_robin_counter: AtomicUsize,
    metrics: Arc<MetricsCollector>,
}

/// 指标收集器，用于收集后端性能数据
pub struct MetricsCollector {
    latencies: Arc<std::sync::RwLock<HashMap<String, Duration>>>,
    health_status: Arc<std::sync::RwLock<HashMap<String, bool>>>,
    failure_counts: Arc<std::sync::RwLock<HashMap<String, u32>>>,
    last_health_check: Arc<std::sync::RwLock<HashMap<String, Instant>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            latencies: Arc::new(std::sync::RwLock::new(HashMap::new())),
            health_status: Arc::new(std::sync::RwLock::new(HashMap::new())),
            failure_counts: Arc::new(std::sync::RwLock::new(HashMap::new())),
            last_health_check: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// 记录请求延迟
    pub fn record_latency(&self, backend_key: &str, latency: Duration) {
        if let Ok(mut latencies) = self.latencies.write() {
            latencies.insert(backend_key.to_string(), latency);
        }
    }

    /// 记录请求失败
    pub fn record_failure(&self, backend_key: &str) {
        if let Ok(mut failures) = self.failure_counts.write() {
            let count = failures.entry(backend_key.to_string()).or_insert(0);
            *count += 1;
        }

        // 标记为不健康
        if let Ok(mut health) = self.health_status.write() {
            health.insert(backend_key.to_string(), false);
        }
    }

    /// 记录请求成功
    pub fn record_success(&self, backend_key: &str) {
        // 重置失败计数
        if let Ok(mut failures) = self.failure_counts.write() {
            failures.insert(backend_key.to_string(), 0);
        }

        // 标记为健康
        if let Ok(mut health) = self.health_status.write() {
            health.insert(backend_key.to_string(), true);
        }
    }

    /// 检查后端是否健康
    pub fn is_healthy(&self, provider: &str, model: &str) -> bool {
        let backend_key = format!("{}:{}", provider, model);

        if let Ok(health) = self.health_status.read() {
            health.get(&backend_key).copied().unwrap_or(true) // 默认认为是健康的
        } else {
            true
        }
    }

    /// 获取后端延迟
    pub fn get_latency(&self, provider: &str, model: &str) -> Option<Duration> {
        let backend_key = format!("{}:{}", provider, model);

        if let Ok(latencies) = self.latencies.read() {
            latencies.get(&backend_key).copied()
        } else {
            None
        }
    }

    /// 获取失败计数
    pub fn get_failure_count(&self, provider: &str, model: &str) -> u32 {
        let backend_key = format!("{}:{}", provider, model);

        if let Ok(failures) = self.failure_counts.read() {
            failures.get(&backend_key).copied().unwrap_or(0)
        } else {
            0
        }
    }

    /// 更新健康检查时间
    pub fn update_health_check(&self, backend_key: &str) {
        if let Ok(mut last_check) = self.last_health_check.write() {
            last_check.insert(backend_key.to_string(), Instant::now());
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendSelector {
    pub fn new(mapping: ModelMapping, metrics: Arc<MetricsCollector>) -> Self {
        Self {
            mapping,
            round_robin_counter: AtomicUsize::new(0),
            metrics,
        }
    }

    /// 获取模型映射的引用
    pub fn get_mapping(&self) -> &ModelMapping {
        &self.mapping
    }

    /// 获取模型名称
    pub fn get_model_name(&self) -> &str {
        &self.mapping.name
    }

    pub fn select(&self) -> Result<Backend> {
        let enabled_backends: Vec<Backend> = self.mapping.backends
            .iter()
            .filter(|b| b.enabled)
            .cloned()
            .collect();

        if enabled_backends.is_empty() {
            anyhow::bail!("No enabled backends for model {}", self.mapping.name);
        }

        match self.mapping.strategy {
            LoadBalanceStrategy::WeightedRandom => {
                self.select_weighted_random(&enabled_backends)
            }
            LoadBalanceStrategy::RoundRobin => {
                self.select_round_robin(&enabled_backends)
            }
            LoadBalanceStrategy::LeastLatency => {
                self.select_least_latency(&enabled_backends)
            }
            LoadBalanceStrategy::Failover => {
                self.select_failover(&enabled_backends)
            }
            LoadBalanceStrategy::Random => {
                self.select_random(&enabled_backends)
            }
        }
    }

    fn select_weighted_random(&self, backends: &[Backend]) -> Result<Backend> {
        let weights: Vec<f64> = backends.iter().map(|b| b.weight).collect();
        let dist = WeightedIndex::new(&weights)?;
        let mut rng = thread_rng();
        Ok(backends[dist.sample(&mut rng)].clone())
    }

    fn select_round_robin(&self, backends: &[Backend]) -> Result<Backend> {
        let index = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % backends.len();
        Ok(backends[index].clone())
    }

    fn select_least_latency(&self, backends: &[Backend]) -> Result<Backend> {
        // 根据metrics选择延迟最低的后端
        let mut best_backend = &backends[0];
        let mut best_latency = self.metrics.get_latency(&best_backend.provider, &best_backend.model)
            .unwrap_or(Duration::from_secs(999)); // 默认很高的延迟

        for backend in backends.iter().skip(1) {
            let latency = self.metrics.get_latency(&backend.provider, &backend.model)
                .unwrap_or(Duration::from_secs(999));

            if latency < best_latency {
                best_backend = backend;
                best_latency = latency;
            }
        }

        Ok(best_backend.clone())
    }

    fn select_failover(&self, backends: &[Backend]) -> Result<Backend> {
        // 按优先级排序，选择第一个可用的
        let mut sorted = backends.to_vec();
        sorted.sort_by_key(|b| b.priority);

        for backend in &sorted {
            if self.metrics.is_healthy(&backend.provider, &backend.model) {
                return Ok(backend.clone());
            }
        }

        // 如果都不健康，返回优先级最高的
        Ok(sorted[0].clone())
    }

    fn select_random(&self, backends: &[Backend]) -> Result<Backend> {
        let mut rng = thread_rng();
        let index = rng.gen_range(0..backends.len());
        Ok(backends[index].clone())
    }
}
