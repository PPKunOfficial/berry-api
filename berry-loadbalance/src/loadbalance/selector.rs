use berry_core::config::model::{Backend, LoadBalanceStrategy, ModelMapping};
use anyhow::Result;
use rand::Rng;
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// 后端选择错误类型
#[derive(Debug, Clone)]
pub struct BackendSelectionError {
    pub model_name: String,
    pub total_backends: usize,
    pub healthy_backends: usize,
    pub enabled_backends: usize,
    pub failed_attempts: Vec<FailedAttempt>,
    pub error_message: String,
}

/// 失败尝试记录
#[derive(Debug, Clone)]
pub struct FailedAttempt {
    pub backend_key: String,
    pub provider: String,
    pub model: String,
    pub reason: String,
    pub is_healthy: bool,
    pub failure_count: u32,
    pub last_failure_time: Option<Instant>,
}

impl std::fmt::Display for BackendSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Backend selection failed for model '{}': {}", self.model_name, self.error_message)
    }
}

impl std::error::Error for BackendSelectionError {}

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
    // 新增：不健康列表管理
    unhealthy_backends: Arc<std::sync::RwLock<HashMap<String, UnhealthyBackend>>>,
    recovery_attempts: Arc<std::sync::RwLock<HashMap<String, u32>>>,
    // 新增：权重恢复状态管理
    weight_recovery_states: Arc<std::sync::RwLock<HashMap<String, WeightRecoveryState>>>,
    // SmartAI 相关字段
    smart_ai_health: Arc<std::sync::RwLock<HashMap<String, SmartAiBackendHealth>>>,
    // 请求计数
    total_requests: Arc<std::sync::atomic::AtomicU64>,
    successful_requests: Arc<std::sync::atomic::AtomicU64>,
    request_counts: Arc<std::sync::RwLock<HashMap<String, u64>>>,
}

/// 不健康后端信息
#[derive(Debug, Clone)]
pub struct UnhealthyBackend {
    pub backend_key: String,
    pub first_failure_time: Instant,
    pub last_failure_time: Instant,
    pub failure_count: u32,
    pub last_recovery_attempt: Option<Instant>,
    pub recovery_attempts: u32,
}

/// 权重恢复状态
#[derive(Debug, Clone)]
pub struct WeightRecoveryState {
    pub backend_key: String,
    pub original_weight: f64,
    pub current_weight: f64,
    pub recovery_stage: RecoveryStage,
    pub last_success_time: Instant,
    pub success_count: u32,
}

/// 恢复阶段
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStage {
    /// 不健康状态，使用10%权重
    Unhealthy,
    /// 恢复中第一阶段，使用30%权重
    RecoveryStage1,
    /// 恢复中第二阶段，使用50%权重
    RecoveryStage2,
    /// 完全恢复，使用100%权重
    FullyRecovered,
}

/// SmartAI 后端健康状态
#[derive(Debug, Clone)]
pub struct SmartAiBackendHealth {
    /// 信心度评分 (0.0-1.0)
    pub confidence_score: f64,
    /// 总请求数
    pub total_requests: u32,
    /// 连续成功次数
    pub consecutive_successes: u32,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 最后请求时间
    pub last_request_time: Instant,
    /// 最后成功时间
    pub last_success_time: Option<Instant>,
    /// 最后失败时间
    pub last_failure_time: Option<Instant>,
    /// 错误统计
    pub error_counts: HashMap<SmartAiErrorType, u32>,
    /// 最后连通性检查时间
    pub last_connectivity_check: Option<Instant>,
    /// 连通性状态
    pub connectivity_ok: bool,
}

/// SmartAI 错误类型
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum SmartAiErrorType {
    NetworkError,    // 连接超时、DNS失败
    AuthError,       // 401、403、API密钥无效
    RateLimitError,  // 429 Too Many Requests
    ServerError,     // 5xx错误
    ModelError,      // 模型不存在、参数错误
    TimeoutError,    // 请求超时
}

/// 请求结果记录
#[derive(Debug, Clone)]
pub struct RequestResult {
    pub success: bool,
    pub latency: Duration,
    pub error_type: Option<SmartAiErrorType>,
    pub timestamp: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            latencies: Arc::new(std::sync::RwLock::new(HashMap::new())),
            health_status: Arc::new(std::sync::RwLock::new(HashMap::new())),
            failure_counts: Arc::new(std::sync::RwLock::new(HashMap::new())),
            last_health_check: Arc::new(std::sync::RwLock::new(HashMap::new())),
            unhealthy_backends: Arc::new(std::sync::RwLock::new(HashMap::new())),
            recovery_attempts: Arc::new(std::sync::RwLock::new(HashMap::new())),
            weight_recovery_states: Arc::new(std::sync::RwLock::new(HashMap::new())),
            smart_ai_health: Arc::new(std::sync::RwLock::new(HashMap::new())),
            total_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            successful_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            request_counts: Arc::new(std::sync::RwLock::new(HashMap::new())),
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
        let now = Instant::now();
        tracing::debug!("Recording failure for backend: {}", backend_key);

        // 增加总请求计数
        self.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 增加后端请求计数
        if let Ok(mut counts) = self.request_counts.write() {
            *counts.entry(backend_key.to_string()).or_insert(0) += 1;
        }

        if let Ok(mut failures) = self.failure_counts.write() {
            let count = failures.entry(backend_key.to_string()).or_insert(0);
            *count += 1;
            tracing::debug!("Updated failure count for {}: {}", backend_key, *count);
        }

        // 标记为不健康
        if let Ok(mut health) = self.health_status.write() {
            health.insert(backend_key.to_string(), false);
            tracing::debug!("Marked backend {} as unhealthy", backend_key);
        }

        // 添加到不健康列表
        if let Ok(mut unhealthy) = self.unhealthy_backends.write() {
            match unhealthy.get_mut(backend_key) {
                Some(backend) => {
                    backend.last_failure_time = now;
                    backend.failure_count += 1;
                    tracing::debug!(
                        "Updated existing unhealthy backend {}: failure_count={}",
                        backend_key,
                        backend.failure_count
                    );
                }
                None => {
                    tracing::debug!("Adding new backend {} to unhealthy list", backend_key);
                    unhealthy.insert(
                        backend_key.to_string(),
                        UnhealthyBackend {
                            backend_key: backend_key.to_string(),
                            first_failure_time: now,
                            last_failure_time: now,
                            failure_count: 1,
                            last_recovery_attempt: None,
                            recovery_attempts: 0,
                        },
                    );
                }
            }
        }

        // 清理权重恢复状态（如果存在）
        if let Ok(mut recovery_states) = self.weight_recovery_states.write() {
            if recovery_states.remove(backend_key).is_some() {
                tracing::debug!(
                    "Cleared weight recovery state for failed backend {}",
                    backend_key
                );
            }
        }
    }

    /// 记录请求成功
    pub fn record_success(&self, backend_key: &str) {
        tracing::debug!("Recording success for backend: {}", backend_key);

        // 增加总请求计数和成功请求计数
        self.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.successful_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // 增加后端请求计数
        if let Ok(mut counts) = self.request_counts.write() {
            *counts.entry(backend_key.to_string()).or_insert(0) += 1;
        }

        // 重置失败计数
        if let Ok(mut failures) = self.failure_counts.write() {
            failures.insert(backend_key.to_string(), 0);
            tracing::debug!("Reset failure count for {} to 0", backend_key);
        }

        // 标记为健康
        if let Ok(mut health) = self.health_status.write() {
            health.insert(backend_key.to_string(), true);
            tracing::debug!("Marked backend {} as healthy", backend_key);
        }

        // 从不健康列表中移除
        if let Ok(mut unhealthy) = self.unhealthy_backends.write() {
            if unhealthy.remove(backend_key).is_some() {
                tracing::debug!("Removed backend {} from unhealthy list", backend_key);
            }
        }

        // 重置恢复尝试计数
        if let Ok(mut recovery) = self.recovery_attempts.write() {
            if recovery.remove(backend_key).is_some() {
                tracing::debug!("Reset recovery attempts for backend {}", backend_key);
            }
        }

        // 清理权重恢复状态
        if let Ok(mut recovery_states) = self.weight_recovery_states.write() {
            if recovery_states.remove(backend_key).is_some() {
                tracing::debug!(
                    "Cleared weight recovery state for recovered backend {}",
                    backend_key
                );
            }
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

    /// 获取所有不健康的后端
    pub fn get_unhealthy_backends(&self) -> Vec<UnhealthyBackend> {
        if let Ok(unhealthy) = self.unhealthy_backends.read() {
            unhealthy.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// 检查后端是否需要恢复检查
    pub fn needs_recovery_check(&self, backend_key: &str, recovery_interval: Duration) -> bool {
        if let Ok(unhealthy) = self.unhealthy_backends.read() {
            if let Some(backend) = unhealthy.get(backend_key) {
                match backend.last_recovery_attempt {
                    Some(last_attempt) => last_attempt.elapsed() >= recovery_interval,
                    None => true, // 从未尝试过恢复
                }
            } else {
                false // 不在不健康列表中
            }
        } else {
            false
        }
    }

    /// 记录恢复尝试
    pub fn record_recovery_attempt(&self, backend_key: &str) {
        let now = Instant::now();
        tracing::debug!("Recording recovery attempt for backend: {}", backend_key);

        if let Ok(mut unhealthy) = self.unhealthy_backends.write() {
            if let Some(backend) = unhealthy.get_mut(backend_key) {
                backend.last_recovery_attempt = Some(now);
                backend.recovery_attempts += 1;
                tracing::debug!(
                    "Updated recovery attempt for {}: attempt #{}",
                    backend_key,
                    backend.recovery_attempts
                );
            } else {
                tracing::warn!(
                    "Attempted to record recovery for backend {} not in unhealthy list",
                    backend_key
                );
            }
        }

        if let Ok(mut recovery) = self.recovery_attempts.write() {
            let count = recovery.entry(backend_key.to_string()).or_insert(0);
            *count += 1;
            tracing::debug!(
                "Updated global recovery count for {}: {}",
                backend_key,
                *count
            );
        }
    }

    /// 检查后端是否在不健康列表中
    pub fn is_in_unhealthy_list(&self, backend_key: &str) -> bool {
        if let Ok(unhealthy) = self.unhealthy_backends.read() {
            unhealthy.contains_key(backend_key)
        } else {
            false
        }
    }

    /// 记录按请求计费provider的被动验证成功
    pub fn record_passive_success(&self, backend_key: &str, original_weight: f64) {
        tracing::debug!(
            "Recording passive success for per-request backend: {}",
            backend_key
        );

        if let Ok(mut recovery_states) = self.weight_recovery_states.write() {
            match recovery_states.get_mut(backend_key) {
                Some(state) => {
                    state.last_success_time = Instant::now();
                    state.success_count += 1;

                    // 根据成功次数逐步提高权重
                    let new_stage = match state.success_count {
                        1..=2 => RecoveryStage::RecoveryStage1, // 30%权重
                        3..=4 => RecoveryStage::RecoveryStage2, // 50%权重
                        _ => RecoveryStage::FullyRecovered,     // 100%权重
                    };

                    if new_stage != state.recovery_stage {
                        state.recovery_stage = new_stage.clone();
                        state.current_weight = match new_stage {
                            RecoveryStage::RecoveryStage1 => original_weight * 0.3,
                            RecoveryStage::RecoveryStage2 => original_weight * 0.5,
                            RecoveryStage::FullyRecovered => original_weight,
                            _ => state.current_weight,
                        };

                        tracing::debug!(
                            "Backend {} advanced to stage {:?} with weight {:.2}",
                            backend_key,
                            new_stage,
                            state.current_weight
                        );

                        // 如果完全恢复，从不健康列表中移除并标记为健康
                        if new_stage == RecoveryStage::FullyRecovered {
                            if let Ok(mut unhealthy) = self.unhealthy_backends.write() {
                                unhealthy.remove(backend_key);
                                tracing::debug!(
                                    "Removed fully recovered backend {} from unhealthy list",
                                    backend_key
                                );
                            }

                            if let Ok(mut health) = self.health_status.write() {
                                health.insert(backend_key.to_string(), true);
                                tracing::debug!(
                                    "Marked fully recovered backend {} as healthy",
                                    backend_key
                                );
                            }
                        }
                    }
                }
                None => {
                    // 首次被动成功，创建恢复状态
                    let recovery_state = WeightRecoveryState {
                        backend_key: backend_key.to_string(),
                        original_weight,
                        current_weight: original_weight * 0.3, // 从30%开始
                        recovery_stage: RecoveryStage::RecoveryStage1,
                        last_success_time: Instant::now(),
                        success_count: 1,
                    };

                    recovery_states.insert(backend_key.to_string(), recovery_state);
                    tracing::debug!(
                        "Created recovery state for backend {} starting at 30% weight",
                        backend_key
                    );
                }
            }
        }
    }

    /// 获取backend的当前权重（考虑恢复状态）
    pub fn get_effective_weight(&self, backend_key: &str, original_weight: f64) -> f64 {
        if let Ok(recovery_states) = self.weight_recovery_states.read() {
            if let Some(state) = recovery_states.get(backend_key) {
                return state.current_weight;
            }
        }

        // 检查是否在不健康列表中
        if self.is_in_unhealthy_list(backend_key) {
            // 不健康的按请求计费provider使用10%权重
            return original_weight * 0.1;
        }

        // 默认使用原始权重
        original_weight
    }

    /// 初始化按请求计费provider的权重恢复状态
    pub fn initialize_per_request_recovery(&self, backend_key: &str, original_weight: f64) {
        tracing::debug!(
            "Initializing per-request recovery for backend: {} with 10% weight",
            backend_key
        );

        if let Ok(mut recovery_states) = self.weight_recovery_states.write() {
            let recovery_state = WeightRecoveryState {
                backend_key: backend_key.to_string(),
                original_weight,
                current_weight: original_weight * 0.1, // 从10%开始
                recovery_stage: RecoveryStage::Unhealthy,
                last_success_time: Instant::now(),
                success_count: 0,
            };

            recovery_states.insert(backend_key.to_string(), recovery_state);
        }
    }

    // SmartAI 相关方法

    /// 记录SmartAI请求结果
    pub fn record_smart_ai_request(&self, backend_key: &str, result: RequestResult) {
        tracing::debug!("Recording SmartAI request result for backend: {}", backend_key);

        if let Ok(mut smart_health) = self.smart_ai_health.write() {
            let health = smart_health.entry(backend_key.to_string()).or_insert_with(|| {
                SmartAiBackendHealth {
                    confidence_score: 0.8, // 初始信心度
                    total_requests: 0,
                    consecutive_successes: 0,
                    consecutive_failures: 0,
                    last_request_time: result.timestamp,
                    last_success_time: None,
                    last_failure_time: None,
                    error_counts: HashMap::new(),
                    last_connectivity_check: None,
                    connectivity_ok: true,
                }
            });

            // 更新基本统计
            health.total_requests += 1;
            health.last_request_time = result.timestamp;

            if result.success {
                health.consecutive_successes += 1;
                health.consecutive_failures = 0;
                health.last_success_time = Some(result.timestamp);

                // 成功时提升信心度
                health.confidence_score = (health.confidence_score + 0.1).min(1.0);

                tracing::debug!(
                    "SmartAI success for {}: confidence={:.3}, consecutive_successes={}",
                    backend_key, health.confidence_score, health.consecutive_successes
                );
            } else {
                health.consecutive_failures += 1;
                health.consecutive_successes = 0;
                health.last_failure_time = Some(result.timestamp);

                // 根据错误类型调整信心度
                if let Some(error_type) = &result.error_type {
                    let penalty = match error_type {
                        SmartAiErrorType::NetworkError => 0.3,
                        SmartAiErrorType::AuthError => 0.8,
                        SmartAiErrorType::RateLimitError => 0.1,
                        SmartAiErrorType::ServerError => 0.2,
                        SmartAiErrorType::ModelError => 0.3,
                        SmartAiErrorType::TimeoutError => 0.2,
                    };

                    health.confidence_score = (health.confidence_score - penalty).max(0.05);

                    // 更新错误统计
                    *health.error_counts.entry(error_type.clone()).or_insert(0) += 1;

                    tracing::debug!(
                        "SmartAI failure for {}: error={:?}, penalty={:.2}, confidence={:.3}",
                        backend_key, error_type, penalty, health.confidence_score
                    );
                }
            }
        }
    }

    /// 获取SmartAI后端信心度
    pub fn get_smart_ai_confidence(&self, backend_key: &str) -> f64 {
        if let Ok(smart_health) = self.smart_ai_health.read() {
            if let Some(health) = smart_health.get(backend_key) {
                // 应用时间衰减
                self.apply_time_decay(health.confidence_score, health.last_request_time)
            } else {
                0.8 // 新后端默认信心度
            }
        } else {
            0.8
        }
    }

    /// 应用时间衰减
    fn apply_time_decay(&self, confidence: f64, last_request: Instant) -> f64 {
        let hours_since_last = last_request.elapsed().as_secs() / 3600;

        let decay_factor = match hours_since_last {
            0..=1 => 1.0,      // 1小时内：无衰减
            2..=6 => 0.95,     // 2-6小时：轻微衰减
            7..=24 => 0.9,     // 7-24小时：中等衰减
            25..=72 => 0.8,    // 1-3天：较大衰减
            _ => 0.7,          // 3天以上：大幅衰减
        };

        (confidence * decay_factor).max(0.5) // 长期无流量时保持基础信心度
    }

    /// 更新连通性检查结果
    pub fn update_smart_ai_connectivity(&self, backend_key: &str, connectivity_ok: bool) {
        if let Ok(mut smart_health) = self.smart_ai_health.write() {
            let health = smart_health.entry(backend_key.to_string()).or_insert_with(|| {
                SmartAiBackendHealth {
                    confidence_score: 0.8,
                    total_requests: 0,
                    consecutive_successes: 0,
                    consecutive_failures: 0,
                    last_request_time: Instant::now(),
                    last_success_time: None,
                    last_failure_time: None,
                    error_counts: HashMap::new(),
                    last_connectivity_check: None,
                    connectivity_ok: true,
                }
            });

            health.last_connectivity_check = Some(Instant::now());
            health.connectivity_ok = connectivity_ok;

            if !connectivity_ok {
                // 连通性失败时降低信心度
                health.confidence_score = (health.confidence_score * 0.5).max(0.05);
                tracing::debug!(
                    "SmartAI connectivity failed for {}: confidence={:.3}",
                    backend_key, health.confidence_score
                );
            }
        }
    }

    /// 获取SmartAI后端的详细健康信息
    pub fn get_smart_ai_health_details(&self, backend_key: &str) -> Option<SmartAiBackendHealth> {
        if let Ok(smart_health) = self.smart_ai_health.read() {
            smart_health.get(backend_key).cloned()
        } else {
            None
        }
    }

    /// 获取所有SmartAI后端的健康信息
    pub fn get_all_smart_ai_health(&self) -> HashMap<String, SmartAiBackendHealth> {
        if let Ok(smart_health) = self.smart_ai_health.read() {
            smart_health.clone()
        } else {
            HashMap::new()
        }
    }

    /// 获取总请求数
    pub fn get_total_requests(&self) -> u64 {
        self.total_requests.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 获取成功请求数
    pub fn get_successful_requests(&self) -> u64 {
        self.successful_requests.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 获取特定后端的请求数
    pub fn get_backend_request_count(&self, backend_key: &str) -> u64 {
        if let Ok(counts) = self.request_counts.read() {
            counts.get(backend_key).copied().unwrap_or(0)
        } else {
            0
        }
    }

    /// 获取所有后端的请求计数
    pub fn get_all_request_counts(&self) -> HashMap<String, u64> {
        if let Ok(counts) = self.request_counts.read() {
            counts.clone()
        } else {
            HashMap::new()
        }
    }

    /// 根据backend key获取延迟
    pub fn get_latency_by_key(&self, backend_key: &str) -> Option<Duration> {
        if let Ok(latencies) = self.latencies.read() {
            latencies.get(backend_key).copied()
        } else {
            None
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
        let enabled_backends: Vec<Backend> = self
            .mapping
            .backends
            .iter()
            .filter(|b| b.enabled)
            .cloned()
            .collect();

        self.select_with_user_filter(&enabled_backends, None)
    }

    /// 根据用户标签过滤后端并选择
    pub fn select_with_user_tags(&self, user_tags: &[String]) -> Result<Backend> {
        let enabled_backends: Vec<Backend> = self
            .mapping
            .backends
            .iter()
            .filter(|b| b.enabled)
            .cloned()
            .collect();

        self.select_with_user_filter(&enabled_backends, Some(user_tags))
    }

    /// 内部选择方法，支持用户标签过滤
    fn select_with_user_filter(&self, enabled_backends: &[Backend], user_tags: Option<&[String]>) -> Result<Backend> {
        // 根据用户标签过滤后端
        let filtered_backends = if let Some(tags) = user_tags {
            self.filter_backends_by_tags(enabled_backends, tags)
        } else {
            enabled_backends.to_vec()
        };

        if filtered_backends.is_empty() {
            let error_msg = if user_tags.is_some() {
                "No backends available matching user tags"
            } else {
                "No enabled backends available"
            };
            return Err(self.create_detailed_error(
                error_msg,
                &self.mapping.backends,
                &[],
            ).into());
        }

        let result = match self.mapping.strategy {
            LoadBalanceStrategy::WeightedRandom => self.select_weighted_random(&filtered_backends),
            LoadBalanceStrategy::RoundRobin => self.select_round_robin(&filtered_backends),
            LoadBalanceStrategy::LeastLatency => self.select_least_latency(&filtered_backends),
            LoadBalanceStrategy::Failover => self.select_failover(&filtered_backends),
            LoadBalanceStrategy::Random => self.select_random(&filtered_backends),
            LoadBalanceStrategy::WeightedFailover => {
                self.select_weighted_failover(&filtered_backends)
            }
            LoadBalanceStrategy::SmartWeightedFailover => {
                self.select_smart_weighted_failover(&filtered_backends)
            }
            LoadBalanceStrategy::SmartAi => {
                self.select_smart_ai(&filtered_backends)
            }
        };

        // 如果选择失败，创建详细的错误信息
        if let Err(e) = &result {
            tracing::error!(
                "Backend selection failed for model '{}' using strategy '{:?}': {}",
                self.mapping.name,
                self.mapping.strategy,
                e
            );
        }

        result
    }

    /// 根据用户标签过滤后端
    fn filter_backends_by_tags(&self, backends: &[Backend], user_tags: &[String]) -> Vec<Backend> {
        // 如果用户没有标签，返回所有后端
        if user_tags.is_empty() {
            return backends.to_vec();
        }

        // 过滤出与用户标签匹配的后端
        backends.iter()
            .filter(|backend| {
                // 如果后端没有标签，允许所有用户访问
                if backend.tags.is_empty() {
                    return true;
                }

                // 检查是否有共同标签
                backend.tags.iter().any(|backend_tag| user_tags.contains(backend_tag))
            })
            .cloned()
            .collect()
    }

    fn select_weighted_random(&self, backends: &[Backend]) -> Result<Backend> {
        let weights: Vec<f64> = backends.iter().map(|b| b.weight).collect();
        let dist = WeightedIndex::new(&weights)?;
        let mut rng = rand::rng();
        Ok(backends[dist.sample(&mut rng)].clone())
    }

    fn select_round_robin(&self, backends: &[Backend]) -> Result<Backend> {
        let index = self.round_robin_counter.fetch_add(1, Ordering::Relaxed) % backends.len();
        Ok(backends[index].clone())
    }

    fn select_least_latency(&self, backends: &[Backend]) -> Result<Backend> {
        // 根据metrics选择延迟最低的后端
        let mut best_backend = &backends[0];
        let mut best_latency = self
            .metrics
            .get_latency(&best_backend.provider, &best_backend.model)
            .unwrap_or(Duration::from_secs(999)); // 默认很高的延迟

        for backend in backends.iter().skip(1) {
            let latency = self
                .metrics
                .get_latency(&backend.provider, &backend.model)
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

        // 尝试找到第一个健康的后端
        for backend in &sorted {
            if self.metrics.is_healthy(&backend.provider, &backend.model) {
                tracing::debug!(
                    "Failover selected healthy backend {}:{} (priority: {}) for model '{}'",
                    backend.provider,
                    backend.model,
                    backend.priority,
                    self.mapping.name
                );
                return Ok(backend.clone());
            }
        }

        // 如果都不健康，返回优先级最高的作为最后尝试
        if !sorted.is_empty() {
            let fallback_backend = &sorted[0];
            tracing::warn!(
                "Failover: no healthy backends available for model '{}', using highest priority backend {}:{} as fallback",
                self.mapping.name,
                fallback_backend.provider,
                fallback_backend.model
            );
            return Ok(fallback_backend.clone());
        }

        // 如果没有任何后端，返回详细错误
        let failed_attempts = self.collect_backend_status(backends);
        Err(self.create_detailed_error(
            "Failover selection failed - no backends available",
            backends,
            &failed_attempts,
        ).into())
    }

    fn select_random(&self, backends: &[Backend]) -> Result<Backend> {
        let mut rng = rand::rng();
        let index = rng.random_range(0..backends.len());
        Ok(backends[index].clone())
    }

    fn select_weighted_failover(&self, backends: &[Backend]) -> Result<Backend> {
        // 首先过滤出健康的后端
        let healthy_backends: Vec<Backend> = backends
            .iter()
            .filter(|b| self.metrics.is_healthy(&b.provider, &b.model))
            .cloned()
            .collect();

        // 如果有健康的后端，使用权重随机选择
        if !healthy_backends.is_empty() {
            return self.select_weighted_random(&healthy_backends);
        }

        // 如果没有健康的后端，记录详细信息并仍然尝试选择
        tracing::warn!(
            "No healthy backends available for weighted failover on model '{}', attempting selection from all backends",
            self.mapping.name
        );

        // 收集失败信息
        let failed_attempts = self.collect_backend_status(backends);

        // 尝试从所有后端中选择（作为最后的尝试）
        match self.select_weighted_random(backends) {
            Ok(backend) => {
                tracing::warn!(
                    "Selected unhealthy backend {}:{} as last resort for model '{}'",
                    backend.provider,
                    backend.model,
                    self.mapping.name
                );
                Ok(backend)
            }
            Err(_) => {
                // 如果连选择都失败了，返回详细错误
                Err(self.create_detailed_error(
                    "Weighted failover selection failed - no healthy backends available and fallback selection failed",
                    backends,
                    &failed_attempts,
                ).into())
            }
        }
    }

    fn select_smart_weighted_failover(&self, backends: &[Backend]) -> Result<Backend> {
        // 智能权重故障转移：考虑权重恢复状态
        let mut adjusted_backends = Vec::new();
        let mut total_effective_weight = 0.0;

        for backend in backends {
            let backend_key = format!("{}:{}", backend.provider, backend.model);
            let effective_weight = self
                .metrics
                .get_effective_weight(&backend_key, backend.weight);

            // 创建调整权重后的backend副本
            let mut adjusted_backend = backend.clone();
            adjusted_backend.weight = effective_weight;
            adjusted_backends.push(adjusted_backend);
            total_effective_weight += effective_weight;

            tracing::debug!(
                "Backend {} effective weight: {:.3} (original: {:.3})",
                backend_key,
                effective_weight,
                backend.weight
            );
        }

        // 过滤出权重大于0的后端
        let valid_backends: Vec<Backend> = adjusted_backends
            .into_iter()
            .filter(|b| b.weight > 0.0)
            .collect();

        if valid_backends.is_empty() {
            // 收集详细的失败信息
            let failed_attempts = self.collect_backend_status(backends);

            tracing::error!(
                "Smart weighted failover failed for model '{}': no backends with positive weight (total effective weight: {:.3})",
                self.mapping.name,
                total_effective_weight
            );

            return Err(self.create_detailed_error(
                "Smart weighted failover failed - no backends with positive effective weight available",
                backends,
                &failed_attempts,
            ).into());
        }

        // 使用调整后的权重进行选择
        match self.select_weighted_random(&valid_backends) {
            Ok(backend) => {
                tracing::debug!(
                    "Smart weighted failover selected backend {}:{} for model '{}'",
                    backend.provider,
                    backend.model,
                    self.mapping.name
                );
                Ok(backend)
            }
            Err(e) => {
                let failed_attempts = self.collect_backend_status(backends);
                tracing::error!(
                    "Smart weighted failover selection failed for model '{}': {}",
                    self.mapping.name,
                    e
                );

                Err(self.create_detailed_error(
                    &format!("Smart weighted failover selection failed: {}", e),
                    backends,
                    &failed_attempts,
                ).into())
            }
        }
    }

    /// SmartAI 负载均衡选择
    fn select_smart_ai(&self, backends: &[Backend]) -> Result<Backend> {
        tracing::debug!("SmartAI selection for model '{}' with {} backends", self.mapping.name, backends.len());

        // 计算每个后端的有效权重
        let mut weighted_backends: Vec<(Backend, f64)> = Vec::new();

        for backend in backends {
            let backend_key = format!("{}:{}", backend.provider, backend.model);
            let confidence = self.metrics.get_smart_ai_confidence(&backend_key);
            let effective_weight = self.calculate_smart_ai_effective_weight(backend, confidence);

            if effective_weight > 0.01 {
                weighted_backends.push((backend.clone(), effective_weight));
                tracing::debug!(
                    "SmartAI backend {}: confidence={:.3}, effective_weight={:.3} (original={:.3})",
                    backend_key, confidence, effective_weight, backend.weight
                );
            } else {
                tracing::debug!(
                    "SmartAI backend {} excluded: confidence={:.3}, effective_weight={:.3}",
                    backend_key, confidence, effective_weight
                );
            }
        }

        if weighted_backends.is_empty() {
            let failed_attempts = self.collect_backend_status(backends);
            tracing::error!(
                "SmartAI selection failed for model '{}': no backends with positive weight",
                self.mapping.name
            );

            return Err(self.create_detailed_error(
                "SmartAI selection failed - no backends with positive effective weight available",
                backends,
                &failed_attempts,
            ).into());
        }

        // 小流量优化的选择策略
        let selected_backend = if weighted_backends.len() == 1 {
            // 只有一个可用后端
            weighted_backends[0].0.clone()
        } else {
            // 80%选择最佳，20%探索其他（避免过度集中）
            if rand::random::<f64>() < 0.8 {
                // 选择权重最高的
                let best = weighted_backends.iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .unwrap();
                best.0.clone()
            } else {
                // 加权随机选择（探索其他后端）
                self.weighted_random_select_smart_ai(&weighted_backends)?
            }
        };

        tracing::debug!(
            "SmartAI selected backend {}:{} for model '{}'",
            selected_backend.provider,
            selected_backend.model,
            self.mapping.name
        );

        Ok(selected_backend)
    }

    /// 计算SmartAI的有效权重
    fn calculate_smart_ai_effective_weight(&self, backend: &Backend, confidence: f64) -> f64 {
        let base_weight = backend.weight;

        // 检查是否为premium后端
        let is_premium = backend.tags.contains(&"premium".to_string());

        // 信心度到权重的映射
        let confidence_weight = match confidence {
            c if c >= 0.8 => c,           // 高信心度：按比例
            c if c >= 0.6 => c * 0.8,     // 中等信心度：适度降权
            c if c >= 0.3 => c * 0.5,     // 低信心度：大幅降权
            _ => 0.05,                    // 极低信心度：保留恢复机会
        };

        // 只有非premium后端才能获得稳定性加成
        let stability_bonus = if !is_premium && confidence > 0.9 {
            1.1  // 非premium后端稳定时给予10%加成
        } else {
            1.0  // premium后端不给加成，凭原始权重竞争
        };

        let effective_weight = base_weight * confidence_weight * stability_bonus;

        tracing::debug!(
            "SmartAI weight calculation for {}: base={:.3}, confidence={:.3}, confidence_weight={:.3}, stability_bonus={:.2}, effective={:.3}, is_premium={}",
            format!("{}:{}", backend.provider, backend.model),
            base_weight, confidence, confidence_weight, stability_bonus, effective_weight, is_premium
        );

        effective_weight
    }

    /// SmartAI加权随机选择
    fn weighted_random_select_smart_ai(&self, backends: &[(Backend, f64)]) -> Result<Backend> {
        let total_weight: f64 = backends.iter().map(|(_, w)| w).sum();
        let mut random_value = rand::random::<f64>() * total_weight;

        for (backend, weight) in backends {
            random_value -= weight;
            if random_value <= 0.0 {
                return Ok(backend.clone());
            }
        }

        // 兜底返回第一个
        Ok(backends[0].0.clone())
    }

    /// 创建详细的错误信息
    fn create_detailed_error(
        &self,
        base_message: &str,
        all_backends: &[Backend],
        failed_attempts: &[FailedAttempt],
    ) -> BackendSelectionError {
        let total_backends = all_backends.len();
        let enabled_backends = all_backends.iter().filter(|b| b.enabled).count();
        let mut healthy_backends = 0;

        // 统计健康的后端数量
        for backend in all_backends {
            if backend.enabled && self.metrics.is_healthy(&backend.provider, &backend.model) {
                healthy_backends += 1;
            }
        }

        // 构建详细的错误消息
        let mut error_message = format!("{} for model '{}'", base_message, self.mapping.name);

        if total_backends == 0 {
            error_message.push_str(". No backends configured for this model.");
        } else if enabled_backends == 0 {
            error_message.push_str(&format!(
                ". All {} configured backends are disabled.",
                total_backends
            ));
        } else if healthy_backends == 0 {
            error_message.push_str(&format!(
                ". All {} enabled backends are currently unhealthy.",
                enabled_backends
            ));

            // 添加恢复建议
            error_message.push_str(" Please check backend health status and wait for automatic recovery, or contact system administrator.");
        } else {
            error_message.push_str(&format!(
                ". {} total backends, {} enabled, {} healthy.",
                total_backends, enabled_backends, healthy_backends
            ));
        }

        // 添加策略信息
        error_message.push_str(&format!(
            " Load balance strategy: {:?}.",
            self.mapping.strategy
        ));

        // 如果有失败尝试，添加详细信息
        if !failed_attempts.is_empty() {
            error_message.push_str(" Recent failures: ");
            for (i, attempt) in failed_attempts.iter().enumerate() {
                if i > 0 {
                    error_message.push_str(", ");
                }
                error_message.push_str(&format!(
                    "{}:{} ({})",
                    attempt.provider,
                    attempt.model,
                    attempt.reason
                ));
            }
        }

        BackendSelectionError {
            model_name: self.mapping.name.clone(),
            total_backends,
            healthy_backends,
            enabled_backends,
            failed_attempts: failed_attempts.to_vec(),
            error_message,
        }
    }

    /// 收集后端状态信息用于错误报告
    fn collect_backend_status(&self, backends: &[Backend]) -> Vec<FailedAttempt> {
        let mut attempts = Vec::new();

        for backend in backends {
            let backend_key = format!("{}:{}", backend.provider, backend.model);
            let is_healthy = self.metrics.is_healthy(&backend.provider, &backend.model);
            let failure_count = self.metrics.get_failure_count(&backend.provider, &backend.model);

            let reason = if !backend.enabled {
                "Backend disabled".to_string()
            } else if !is_healthy {
                format!("Unhealthy (failures: {})", failure_count)
            } else {
                "Available".to_string()
            };

            // 从metrics中获取真实的last_failure_time
            let last_failure_time = if let Ok(unhealthy_backends) = self.metrics.unhealthy_backends.read() {
                unhealthy_backends.get(&backend_key).map(|ub| ub.last_failure_time)
            } else {
                None
            };

            attempts.push(FailedAttempt {
                backend_key,
                provider: backend.provider.clone(),
                model: backend.model.clone(),
                reason,
                is_healthy,
                failure_count,
                last_failure_time,
            });
        }

        attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use berry_core::config::model::{BillingMode, LoadBalanceStrategy, ModelMapping};

    fn create_test_backends() -> Vec<Backend> {
        vec![
            Backend {
                provider: "provider1".to_string(),
                model: "model1".to_string(),
                weight: 0.6,
                priority: 1,
                enabled: true,
                tags: vec![],
                billing_mode: BillingMode::PerToken,
            },
            Backend {
                provider: "provider2".to_string(),
                model: "model2".to_string(),
                weight: 0.3,
                priority: 2,
                enabled: true,
                tags: vec![],
                billing_mode: BillingMode::PerRequest,
            },
            Backend {
                provider: "provider3".to_string(),
                model: "model3".to_string(),
                weight: 0.1,
                priority: 3,
                enabled: true,
                tags: vec![],
                billing_mode: BillingMode::PerToken,
            },
        ]
    }

    fn create_test_mapping() -> ModelMapping {
        ModelMapping {
            name: "test-model".to_string(),
            backends: create_test_backends(),
            strategy: LoadBalanceStrategy::WeightedFailover,
            enabled: true,
        }
    }

    #[test]
    fn test_weighted_failover_all_healthy() {
        let metrics = Arc::new(MetricsCollector::new());
        let mapping = create_test_mapping();
        let selector = BackendSelector::new(mapping, metrics.clone());

        // 标记所有后端为健康
        metrics.record_success("provider1:model1");
        metrics.record_success("provider2:model2");
        metrics.record_success("provider3:model3");

        // 多次选择，验证权重分布
        let mut selections = std::collections::HashMap::new();
        for _ in 0..1000 {
            let backend = selector.select().unwrap();
            let key = format!("{}:{}", backend.provider, backend.model);
            *selections.entry(key).or_insert(0) += 1;
        }

        // 验证选择分布大致符合权重比例
        assert!(selections.contains_key("provider1:model1"));
        assert!(selections.contains_key("provider2:model2"));
        assert!(selections.contains_key("provider3:model3"));

        // provider1应该被选择最多（权重0.6）
        let provider1_count = selections.get("provider1:model1").unwrap_or(&0);
        let provider2_count = selections.get("provider2:model2").unwrap_or(&0);
        let provider3_count = selections.get("provider3:model3").unwrap_or(&0);

        assert!(provider1_count > provider2_count);
        assert!(provider2_count > provider3_count);
    }

    #[test]
    fn test_weighted_failover_partial_failure() {
        let metrics = Arc::new(MetricsCollector::new());
        let mapping = create_test_mapping();
        let selector = BackendSelector::new(mapping, metrics.clone());

        // 标记provider1为不健康，其他为健康
        metrics.record_failure("provider1:model1");
        metrics.record_success("provider2:model2");
        metrics.record_success("provider3:model3");

        // 多次选择，验证只选择健康的后端
        let mut selections = std::collections::HashMap::new();
        for _ in 0..100 {
            let backend = selector.select().unwrap();
            let key = format!("{}:{}", backend.provider, backend.model);
            *selections.entry(key).or_insert(0) += 1;
        }

        // 不应该选择不健康的provider1
        assert!(!selections.contains_key("provider1:model1"));
        // 应该选择健康的provider2和provider3
        assert!(selections.contains_key("provider2:model2"));
        assert!(selections.contains_key("provider3:model3"));
    }

    #[test]
    fn test_weighted_failover_all_failed() {
        let metrics = Arc::new(MetricsCollector::new());
        let mapping = create_test_mapping();
        let selector = BackendSelector::new(mapping, metrics.clone());

        // 标记所有后端为不健康
        metrics.record_failure("provider1:model1");
        metrics.record_failure("provider2:model2");
        metrics.record_failure("provider3:model3");

        // WeightedFailover策略在所有后端都不健康时，仍然会基于权重进行选择
        // 多次选择验证权重分布（即使后端不健康）
        let mut selections = std::collections::HashMap::new();
        for _ in 0..1000 {
            let backend = selector.select().unwrap();
            let key = format!("{}:{}", backend.provider, backend.model);
            *selections.entry(key).or_insert(0) += 1;
        }

        // 应该仍然选择所有后端，但provider1被选择最多（权重0.6最高）
        assert!(selections.contains_key("provider1:model1"));
        assert!(selections.contains_key("provider2:model2"));
        assert!(selections.contains_key("provider3:model3"));

        let provider1_count = selections.get("provider1:model1").unwrap_or(&0);
        let provider2_count = selections.get("provider2:model2").unwrap_or(&0);
        let provider3_count = selections.get("provider3:model3").unwrap_or(&0);

        // provider1应该被选择最多（权重0.6）
        assert!(provider1_count > provider2_count);
        assert!(provider2_count > provider3_count);
    }

    #[test]
    fn test_smart_ai_selection() {
        let metrics = Arc::new(MetricsCollector::new());
        let mut mapping = create_test_mapping();
        mapping.strategy = LoadBalanceStrategy::SmartAi; // 使用SmartAI策略
        let selector = BackendSelector::new(mapping, metrics.clone());

        // 模拟一些请求结果来建立信心度
        let backend_key1 = "provider1:model1";
        let backend_key2 = "provider2:model2";

        // 记录一些成功的请求
        for _ in 0..5 {
            metrics.record_smart_ai_request(backend_key1, RequestResult {
                success: true,
                latency: std::time::Duration::from_millis(100),
                error_type: None,
                timestamp: std::time::Instant::now(),
            });
        }

        // provider2有一些失败
        metrics.record_smart_ai_request(backend_key2, RequestResult {
            success: false,
            latency: std::time::Duration::from_millis(200),
            error_type: Some(SmartAiErrorType::NetworkError),
            timestamp: std::time::Instant::now(),
        });

        // 进行选择测试
        let backend = selector.select().unwrap();

        // 应该能够成功选择一个后端
        assert!(backend.enabled);
        assert!(!backend.provider.is_empty());
        assert!(!backend.model.is_empty());
    }
}
