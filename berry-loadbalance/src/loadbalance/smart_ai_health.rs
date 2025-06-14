use berry_core::config::model::{Config, Provider};
use super::MetricsCollector;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// SmartAI 轻量级健康检查器
/// 执行免费的连通性检查，不发送付费的AI请求
pub struct SmartAiHealthChecker {
    config: Arc<Config>,
    metrics: Arc<MetricsCollector>,
    client: Client,
    check_interval: Duration,
}

impl SmartAiHealthChecker {
    pub fn new(config: Arc<Config>, metrics: Arc<MetricsCollector>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to create HTTP client for SmartAI health checker");

        Self {
            config,
            metrics,
            client,
            check_interval: Duration::from_secs(600), // 10分钟检查一次
        }
    }

    /// 启动健康检查服务
    pub async fn start(&self) -> Result<()> {
        info!("Starting SmartAI lightweight health checker");
        
        let mut interval = interval(self.check_interval);
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.check_all_providers().await {
                error!("SmartAI health check failed: {}", e);
            }
        }
    }

    /// 检查所有启用的provider
    async fn check_all_providers(&self) -> Result<()> {
        let enabled_providers: Vec<_> = self.config.providers.iter()
            .filter(|(_, provider)| provider.enabled)
            .collect();

        debug!("SmartAI health check for {} enabled providers", enabled_providers.len());

        for (provider_id, provider) in enabled_providers {
            self.check_provider_connectivity(provider_id, provider).await;
        }

        Ok(())
    }

    /// 检查单个provider的连通性
    async fn check_provider_connectivity(&self, provider_id: &str, provider: &Provider) {
        debug!("SmartAI connectivity check for provider: {}", provider_id);

        // 1. 基础URL连通性检查
        let base_connectivity = self.check_base_connectivity(&provider.base_url).await;
        
        // 2. Models API检查（通常免费）
        let models_api_ok = if base_connectivity {
            self.check_models_api(provider).await
        } else {
            false
        };

        let overall_connectivity = base_connectivity && models_api_ok;

        // 更新所有模型的连通性状态
        for model in &provider.models {
            let backend_key = format!("{}:{}", provider_id, model);
            self.metrics.update_smart_ai_connectivity(&backend_key, overall_connectivity);
            
            debug!(
                "SmartAI connectivity for {}: base={}, models_api={}, overall={}",
                backend_key, base_connectivity, models_api_ok, overall_connectivity
            );
        }

        if !overall_connectivity {
            warn!(
                "SmartAI connectivity failed for provider {}: base_connectivity={}, models_api={}",
                provider_id, base_connectivity, models_api_ok
            );
        }
    }

    /// 检查基础URL连通性
    async fn check_base_connectivity(&self, base_url: &str) -> bool {
        debug!("Checking base connectivity for: {}", base_url);

        match self.client
            .head(base_url)
            .timeout(Duration::from_secs(10))
            .send()
            .await 
        {
            Ok(response) => {
                let success = response.status().is_success() || response.status().as_u16() == 404;
                debug!("Base connectivity check for {}: status={}, success={}", 
                       base_url, response.status(), success);
                success
            }
            Err(e) => {
                debug!("Base connectivity failed for {}: {}", base_url, e);
                false
            }
        }
    }

    /// 检查Models API（通常免费）
    async fn check_models_api(&self, provider: &Provider) -> bool {
        let url = format!("{}/v1/models", provider.base_url);
        debug!("Checking models API: {}", url);

        if provider.api_key.is_empty() {
            debug!("Skipping models API check for {} (empty API key)", provider.base_url);
            return false;
        }

        let mut request = self.client.get(&url)
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .timeout(Duration::from_secs(15));

        // 添加自定义头部
        for (key, value) in &provider.headers {
            request = request.header(key, value);
        }

        match request.send().await {
            Ok(response) => {
                let success = response.status().is_success();
                debug!("Models API check for {}: status={}, success={}", 
                       url, response.status(), success);
                success
            }
            Err(e) => {
                debug!("Models API failed for {}: {}", url, e);
                false
            }
        }
    }

    /// 手动触发连通性检查
    pub async fn check_now(&self) -> Result<()> {
        info!("Manual SmartAI connectivity check triggered");
        self.check_all_providers().await
    }

    /// 检查特定provider的连通性
    pub async fn check_provider(&self, provider_id: &str) -> Result<()> {
        if let Some(provider) = self.config.providers.get(provider_id) {
            if provider.enabled {
                self.check_provider_connectivity(provider_id, provider).await;
                Ok(())
            } else {
                anyhow::bail!("Provider '{}' is disabled", provider_id);
            }
        } else {
            anyhow::bail!("Provider '{}' not found", provider_id);
        }
    }
}
