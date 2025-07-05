use super::selector::{RequestResult as SmartAiRequestResult, SmartAiErrorType};
use super::traits::{LoadBalancer, LoadBalancerMetrics};
use super::{HealthChecker, LoadBalanceManager, MetricsCollector};
use anyhow::Result;
use async_trait::async_trait;
use berry_core::{Backend, Config};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// 负载均衡服务
/// 整合负载均衡管理器和健康检查器，提供统一的服务接口
pub struct LoadBalanceService {
    manager: Arc<LoadBalanceManager>,
    health_checker: Arc<HealthChecker>,
    metrics: Arc<MetricsCollector>,
    is_running: Arc<RwLock<bool>>,
}

impl LoadBalanceService {
    /// 创建新的负载均衡服务
    pub fn new(config: Config) -> Result<Self> {
        // 验证配置
        config.validate()?;

        let manager = Arc::new(LoadBalanceManager::new(config.clone()));
        let metrics = manager.get_metrics();
        let health_checker = Arc::new(HealthChecker::new(manager.get_config(), metrics.clone()));

        Ok(Self {
            manager,
            health_checker,
            metrics,
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// 启动负载均衡服务
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.is_running.write().await;
            if *running {
                return Ok(());
            }
            *running = true;
        }

        info!("Starting load balance service");

        // 初始化管理器
        self.manager.initialize().await?;

        // 启动健康检查器
        let health_checker = self.health_checker.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            while *is_running.read().await {
                if let Err(e) = health_checker.check_now().await {
                    error!("Health check failed: {}", e);
                }

                // 等待下一次检查
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });

        // 启动恢复检查器
        let recovery_checker = self.health_checker.clone();
        let is_running_recovery = self.is_running.clone();

        tokio::spawn(async move {
            while *is_running_recovery.read().await {
                if let Err(e) = recovery_checker.check_recovery().await {
                    error!("Recovery check failed: {}", e);
                }

                // 等待下一次恢复检查（通常比健康检查间隔更长）
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });

        info!("Load balance service started successfully");
        Ok(())
    }

    /// 停止负载均衡服务
    pub async fn stop(&self) {
        let mut running = self.is_running.write().await;
        *running = false;
        info!("Load balance service stopped");
    }

    /// 直接选择指定的后端（用于调试和健康检查）
    pub async fn select_specific_backend(
        &self,
        model_name: &str,
        provider_name: &str,
    ) -> Result<SelectedBackend> {
        let start_time = Instant::now();

        debug!(
            "Selecting specific backend for model: {} from provider: {}",
            model_name, provider_name
        );

        // 获取配置
        let config = self.manager.get_config();

        // 查找模型配置（支持键名和显示名称）
        let (_found_key, model_mapping) = self
            .find_model_by_name(&config, model_name)
            .ok_or_else(|| anyhow::anyhow!("Model '{}' not found", model_name))?;

        if !model_mapping.enabled {
            anyhow::bail!("Model '{}' is disabled", model_name);
        }

        // 查找指定的后端
        let backend = model_mapping
            .backends
            .iter()
            .find(|b| b.provider == provider_name && b.enabled)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Backend '{}' not found or disabled for model '{}'",
                    provider_name,
                    model_name
                )
            })?;

        // 获取provider配置
        let provider = config
            .get_provider(&backend.provider)
            .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found", backend.provider))?;

        let selection_time = start_time.elapsed();

        debug!(
            "Selected specific backend for model '{}': provider='{}', model='{}', selection_time={}ms",
            model_name,
            backend.provider,
            backend.model,
            selection_time.as_millis()
        );

        Ok(SelectedBackend {
            backend: backend.clone(),
            provider: provider.clone(),
            selection_time,
        })
    }

    /// 为指定模型选择后端（带智能重试）
    pub async fn select_backend(&self, model_name: &str) -> Result<SelectedBackend> {
        self.select_backend_with_user_tags(model_name, None).await
    }

    /// 为指定模型选择后端（支持用户标签过滤）
    pub async fn select_backend_with_user_tags(
        &self,
        model_name: &str,
        user_tags: Option<&[String]>,
    ) -> Result<SelectedBackend> {
        let start_time = Instant::now();
        let max_retries = self.manager.get_config().settings.max_internal_retries;

        debug!(
            "Selecting backend for model: {} (max retries: {})",
            model_name, max_retries
        );

        for attempt in 0..=max_retries {
            debug!(
                "Backend selection attempt {} for model '{}'",
                attempt + 1,
                model_name
            );

            match self
                .manager
                .select_backend_with_user_tags(model_name, user_tags)
                .await
            {
                Ok(backend) => {
                    debug!(
                        "Load balancer selected backend: {}:{}",
                        backend.provider, backend.model
                    );

                    // 检查选中的backend是否健康
                    let is_healthy = self.metrics.is_healthy(&backend.provider, &backend.model);
                    debug!(
                        "Health check for {}:{}: {}",
                        backend.provider,
                        backend.model,
                        if is_healthy { "HEALTHY" } else { "UNHEALTHY" }
                    );

                    if is_healthy {
                        let selection_time = start_time.elapsed();

                        debug!(
                            "Selected healthy backend for model '{}': provider='{}', model='{}', selection_time={}ms",
                            model_name,
                            backend.provider,
                            backend.model,
                            selection_time.as_millis()
                        );

                        // 获取provider配置
                        let config = self.manager.get_config();
                        let provider = config.get_provider(&backend.provider).ok_or_else(|| {
                            anyhow::anyhow!("Provider '{}' not found", backend.provider)
                        })?;

                        debug!(
                            "Successfully resolved provider config for: {}",
                            backend.provider
                        );
                        return Ok(SelectedBackend {
                            backend,
                            provider: provider.clone(),
                            selection_time,
                        });
                    } else if attempt < max_retries {
                        debug!(
                            "Selected backend {}:{} is unhealthy, retrying... (attempt {}/{})",
                            backend.provider,
                            backend.model,
                            attempt + 1,
                            max_retries + 1
                        );
                        continue;
                    } else {
                        // 最后一次尝试，即使不健康也返回
                        warn!(
                            "All retries exhausted, returning unhealthy backend {}:{}",
                            backend.provider, backend.model
                        );
                        debug!("No more retry attempts available, using unhealthy backend as last resort");

                        let selection_time = start_time.elapsed();
                        let config = self.manager.get_config();
                        let provider = config.get_provider(&backend.provider).ok_or_else(|| {
                            anyhow::anyhow!("Provider '{}' not found", backend.provider)
                        })?;

                        return Ok(SelectedBackend {
                            backend,
                            provider: provider.clone(),
                            selection_time,
                        });
                    }
                }
                Err(e) => {
                    debug!("Backend selection failed: {}", e);
                    if attempt < max_retries {
                        debug!(
                            "Backend selection failed, retrying... (attempt {}/{}): {}",
                            attempt + 1,
                            max_retries + 1,
                            e
                        );
                        continue;
                    } else {
                        // 最后一次尝试失败，提供详细的错误信息
                        error!(
                            "All {} backend selection attempts failed for model '{}'. Final error: {}",
                            max_retries + 1,
                            model_name,
                            e
                        );

                        // 检查是否是我们的详细错误类型
                        if let Some(detailed_error) =
                            e.downcast_ref::<crate::loadbalance::selector::BackendSelectionError>()
                        {
                            // 如果是详细错误，直接返回
                            return Err(anyhow::anyhow!(
                                "Backend selection failed after {} internal retries for model '{}': {}. Total backends: {}, Enabled: {}, Healthy: {}. Please check backend health status or contact system administrator.",
                                max_retries + 1,
                                detailed_error.model_name,
                                detailed_error.error_message,
                                detailed_error.total_backends,
                                detailed_error.enabled_backends,
                                detailed_error.healthy_backends
                            ));
                        } else {
                            // 如果是其他类型的错误，包装成详细错误
                            return Err(anyhow::anyhow!(
                                "Backend selection failed after {} internal retries for model '{}': {}. This error occurred during the load balancing process. Please check your configuration and backend health status.",
                                max_retries + 1,
                                model_name,
                                e
                            ));
                        }
                    }
                }
            }
        }

        // 这行代码理论上不应该被执行到，但为了安全起见保留
        anyhow::bail!(
            "Unexpected error: Failed to select backend after {} attempts for model '{}'. This indicates a logic error in the retry mechanism.",
            max_retries + 1,
            model_name
        )
    }

    /// 记录SmartAI请求结果
    pub async fn record_smart_ai_request_result(
        &self,
        provider: &str,
        model: &str,
        success: bool,
        latency: Duration,
        error: Option<&anyhow::Error>,
    ) {
        let smart_ai_result = SmartAiRequestResult {
            success,
            latency,
            error_type: error.map(|e| self.classify_smart_ai_error(e)),
            timestamp: Instant::now(),
        };

        self.manager
            .record_smart_ai_request(provider, model, smart_ai_result);

        debug!(
            "Recorded SmartAI request for {}:{}: success={}, latency={}ms",
            provider,
            model,
            success,
            latency.as_millis()
        );
    }

    /// 分类SmartAI错误类型
    fn classify_smart_ai_error(&self, error: &anyhow::Error) -> SmartAiErrorType {
        LoadBalanceManager::classify_error(error)
    }

    /// 记录请求结果
    pub async fn record_request_result(&self, provider: &str, model: &str, result: RequestResult) {
        // 首先检查是否需要记录SmartAI信心度
        let config = self.manager.get_config();
        let mut is_smart_ai_model = false;
        let mut backend_billing_mode = berry_core::BillingMode::PerToken; // 默认值
        let mut found_backend = false;

        // 查找对应的backend配置和负载均衡策略
        for model_mapping in config.models.values() {
            for backend in &model_mapping.backends {
                if backend.provider == provider && backend.model == model {
                    backend_billing_mode = backend.billing_mode.clone();
                    is_smart_ai_model =
                        model_mapping.strategy == berry_core::LoadBalanceStrategy::SmartAi;
                    found_backend = true;
                    break;
                }
            }
            if found_backend {
                break;
            }
        }

        if !found_backend {
            warn!(
                "Backend configuration not found for {}:{}, using default per-token billing",
                provider, model
            );
        }

        // 如果是SmartAI模型，记录SmartAI信心度
        if is_smart_ai_model {
            let smart_ai_result = match &result {
                RequestResult::Success { latency } => super::selector::RequestResult {
                    success: true,
                    latency: *latency,
                    error_type: None,
                    timestamp: std::time::Instant::now(),
                },
                RequestResult::Failure { error } => {
                    let error_type =
                        LoadBalanceManager::classify_error(&anyhow::anyhow!("{}", error));
                    super::selector::RequestResult {
                        success: false,
                        latency: std::time::Duration::from_millis(0),
                        error_type: Some(error_type),
                        timestamp: std::time::Instant::now(),
                    }
                }
            };

            let is_success = smart_ai_result.success;
            self.manager
                .record_smart_ai_request(provider, model, smart_ai_result);
            debug!(
                "Recorded SmartAI request for {}:{}: success={}",
                provider, model, is_success
            );
        }

        match result {
            RequestResult::Success { latency } => {
                let backend_key = format!("{provider}:{model}");

                match backend_billing_mode {
                    berry_core::BillingMode::PerToken => {
                        // 按token计费：正常记录成功
                        self.manager.record_success(provider, model, latency);
                        debug!(
                            "Recorded success for per-token backend {}:{} with latency {}ms",
                            provider,
                            model,
                            latency.as_millis()
                        );
                    }
                    berry_core::BillingMode::PerRequest => {
                        // 按请求计费：检查是否在不健康列表中
                        if self.metrics.is_in_unhealthy_list(&backend_key) {
                            // 不健康的按请求计费backend：使用被动验证
                            self.metrics.record_passive_success(
                                &backend_key,
                                self.get_backend_original_weight(provider, model)
                                    .unwrap_or(1.0),
                            );
                            debug!(
                                "Recorded passive success for per-request backend {}:{} (weight recovery)",
                                provider, model
                            );
                        } else {
                            // 健康的按请求计费backend：正常记录
                            self.manager.record_success(provider, model, latency);
                            debug!(
                                "Recorded success for healthy per-request backend {}:{} with latency {}ms",
                                provider,
                                model,
                                latency.as_millis()
                            );
                        }
                    }
                }
            }
            RequestResult::Failure { error } => {
                // Chat请求失败应该使用Chat检查方式进行重试
                self.manager.record_failure_with_method(
                    provider,
                    model,
                    super::selector::HealthCheckMethod::Chat,
                );
                debug!(
                    "Recorded chat failure for {}:{} with error: {}",
                    provider, model, error
                );

                // 对于按请求计费的backend，失败时需要初始化权重恢复状态
                if found_backend && backend_billing_mode == berry_core::BillingMode::PerRequest {
                    let backend_key = format!("{provider}:{model}");
                    let original_weight = self
                        .get_backend_original_weight(provider, model)
                        .unwrap_or(1.0);
                    self.metrics
                        .initialize_per_request_recovery(&backend_key, original_weight);
                    debug!(
                        "Initialized per-request recovery for {}:{} with 10% weight",
                        provider, model
                    );
                }
            }
        }
    }

    /// 获取所有可用的模型列表
    pub fn get_available_models(&self) -> Vec<String> {
        self.manager.get_available_models()
    }

    /// 获取服务健康状态
    pub async fn get_service_health(&self) -> ServiceHealth {
        let health_summary = self.health_checker.get_health_summary();
        let model_stats = self.manager.get_health_stats().await;
        let is_running = *self.is_running.read().await;

        ServiceHealth {
            is_running,
            health_summary,
            model_stats,
            total_requests: self.metrics.get_total_requests(),
            successful_requests: self.metrics.get_successful_requests(),
        }
    }

    /// 手动触发健康检查
    pub async fn trigger_health_check(&self) -> Result<()> {
        self.health_checker.check_now().await
    }

    /// 获取指标收集器
    pub fn get_metrics(&self) -> Arc<MetricsCollector> {
        self.metrics.clone()
    }

    /// 检查服务是否正在运行
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// 获取缓存统计信息
    pub async fn get_cache_stats(&self) -> Option<super::cache::CacheStats> {
        self.manager.get_cache_stats().await
    }

    /// 获取模型权重信息（用于监控）
    pub async fn get_model_weights(
        &self,
        model_name: &str,
    ) -> Result<std::collections::HashMap<String, f64>> {
        // 委托给manager来获取权重信息
        self.manager.get_model_weights(model_name).await
    }

    /// 获取backend的原始权重
    fn get_backend_original_weight(&self, provider: &str, model: &str) -> Option<f64> {
        let config = self.manager.get_config();

        // 遍历所有模型映射，找到匹配的backend
        for model_mapping in config.models.values() {
            for backend in &model_mapping.backends {
                if backend.provider == provider && backend.model == model {
                    return Some(backend.weight);
                }
            }
        }

        None
    }

    /// 通过模型名称查找模型（支持键名和显示名称）
    fn find_model_by_name<'a>(
        &self,
        config: &'a berry_core::Config,
        model_name: &str,
    ) -> Option<(String, &'a berry_core::Model)> {
        // 首先尝试直接通过键名查找
        if let Some(mapping) = config.models.get(model_name) {
            return Some((model_name.to_string(), mapping));
        }

        // 然后尝试通过显示名称查找
        for (key, mapping) in &config.models {
            if mapping.name == model_name {
                return Some((key.clone(), mapping));
            }
        }

        None
    }
}

/// 选中的后端信息
#[derive(Debug, Clone)]
pub struct SelectedBackend {
    pub backend: Backend,
    pub provider: berry_core::Provider,
    pub selection_time: Duration,
}

impl SelectedBackend {
    /// 获取完整的API URL
    pub fn get_api_url(&self, endpoint: &str) -> String {
        format!(
            "{}/{}",
            self.provider.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/')
        )
    }

    /// 获取API密钥
    pub fn get_api_key(&self) -> Result<String> {
        if self.provider.api_key.is_empty() {
            anyhow::bail!("API key is empty for provider: {}", self.provider.name);
        }
        Ok(self.provider.api_key.clone())
    }

    /// 获取请求头
    pub fn get_headers(&self) -> std::collections::HashMap<String, String> {
        self.provider.headers.clone()
    }

    /// 获取超时设置
    pub fn get_timeout(&self) -> Duration {
        Duration::from_secs(self.provider.timeout_seconds)
    }
}

/// 请求结果
#[derive(Debug, Clone)]
pub enum RequestResult {
    Success { latency: Duration },
    Failure { error: String },
}

/// 服务健康状态
#[derive(Debug, Clone)]
pub struct ServiceHealth {
    pub is_running: bool,
    pub health_summary: super::health_checker::HealthSummary,
    pub model_stats: std::collections::HashMap<String, super::manager::HealthStats>,
    pub total_requests: u64,
    pub successful_requests: u64,
}

impl ServiceHealth {
    /// 检查服务是否健康
    pub fn is_healthy(&self) -> bool {
        self.is_running && self.health_summary.is_system_healthy()
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            self.successful_requests as f64 / self.total_requests as f64
        } else {
            0.0
        }
    }
}

/// 为 LoadBalanceService 实现 LoadBalancer trait
#[async_trait]
impl LoadBalancer for LoadBalanceService {
    async fn select_backend(&self, model_name: &str) -> Result<SelectedBackend> {
        LoadBalanceService::select_backend(self, model_name).await
    }

    async fn select_backend_with_user_tags(
        &self,
        model_name: &str,
        user_tags: Option<&[String]>,
    ) -> Result<SelectedBackend> {
        LoadBalanceService::select_backend_with_user_tags(self, model_name, user_tags).await
    }

    async fn select_specific_backend(
        &self,
        model_name: &str,
        provider_name: &str,
    ) -> Result<SelectedBackend> {
        LoadBalanceService::select_specific_backend(self, model_name, provider_name).await
    }

    async fn record_request_result(&self, provider: &str, model: &str, result: RequestResult) {
        LoadBalanceService::record_request_result(self, provider, model, result).await;
    }

    fn get_metrics(&self) -> Arc<dyn LoadBalancerMetrics> {
        self.metrics.clone()
    }

    async fn get_service_health(&self) -> ServiceHealth {
        LoadBalanceService::get_service_health(self).await
    }

    async fn trigger_health_check(&self) -> Result<()> {
        LoadBalanceService::trigger_health_check(self).await
    }

    async fn is_running(&self) -> bool {
        LoadBalanceService::is_running(self).await
    }

    async fn get_cache_stats(&self) -> Option<super::cache::CacheStats> {
        LoadBalanceService::get_cache_stats(self).await
    }

    async fn get_model_weights(
        &self,
        model_name: &str,
    ) -> Result<std::collections::HashMap<String, f64>> {
        LoadBalanceService::get_model_weights(self, model_name).await
    }

    async fn get_health_stats(
        &self,
    ) -> std::collections::HashMap<String, super::manager::HealthStats> {
        self.manager.get_health_stats().await
    }
}
