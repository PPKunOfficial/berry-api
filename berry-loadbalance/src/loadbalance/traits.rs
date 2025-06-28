use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use super::{HealthStats, MetricsCollector, RequestResult, SelectedBackend, ServiceHealth};

/// 负载均衡器接口
///
/// 这个trait定义了负载均衡器的核心功能，允许不同的实现策略
/// 并支持依赖注入和单元测试
#[async_trait]
pub trait LoadBalancer: Send + Sync {
    /// 为指定模型选择后端
    async fn select_backend(&self, model_name: &str) -> Result<SelectedBackend>;

    /// 为指定模型选择后端（支持用户标签过滤）
    async fn select_backend_with_user_tags(
        &self,
        model_name: &str,
        user_tags: Option<&[String]>,
    ) -> Result<SelectedBackend>;

    /// 选择指定的后端提供商
    async fn select_specific_backend(
        &self,
        model_name: &str,
        provider_name: &str,
    ) -> Result<SelectedBackend>;

    /// 记录请求结果
    async fn record_request_result(&self, provider: &str, model: &str, result: RequestResult);

    /// 获取指标收集器
    fn get_metrics(&self) -> Arc<dyn LoadBalancerMetrics>;

    /// 获取服务健康状态
    async fn get_service_health(&self) -> ServiceHealth;

    /// 手动触发健康检查
    async fn trigger_health_check(&self) -> Result<()>;

    /// 检查服务是否正在运行
    async fn is_running(&self) -> bool;

    /// 获取缓存统计信息
    async fn get_cache_stats(&self) -> Option<super::cache::CacheStats>;

    /// 获取模型权重信息（用于监控）
    async fn get_model_weights(&self, model_name: &str) -> Result<HashMap<String, f64>>;

    /// 获取健康状态统计
    async fn get_health_stats(&self) -> HashMap<String, HealthStats>;
}

/// 负载均衡器指标接口
///
/// 分离指标相关的功能，提供更好的模块化
pub trait LoadBalancerMetrics: Send + Sync {
    /// 获取总请求数
    fn get_total_requests(&self) -> u64;

    /// 获取成功请求数
    fn get_successful_requests(&self) -> u64;

    /// 检查后端是否在不健康列表中
    fn is_in_unhealthy_list(&self, backend_key: &str) -> bool;

    /// 获取后端延迟统计
    fn get_backend_latency(&self, backend_key: &str) -> Option<std::time::Duration>;

    /// 获取后端错误计数
    fn get_backend_error_count(&self, backend_key: &str) -> u64;

    /// 获取后端请求计数
    fn get_backend_request_count(&self, backend_key: &str) -> u64;

    /// 记录后端请求
    fn record_backend_request(&self, backend_key: &str);

    /// 记录后端错误
    fn record_backend_error(&self, backend_key: &str);

    /// 记录后端延迟
    fn record_backend_latency(&self, backend_key: &str, latency: std::time::Duration);

    /// 标记后端为不健康
    fn mark_backend_unhealthy(&self, backend_key: &str);

    /// 标记后端为健康
    fn mark_backend_healthy(&self, backend_key: &str);
}

/// 为现有的 MetricsCollector 实现 LoadBalancerMetrics trait
impl LoadBalancerMetrics for MetricsCollector {
    fn get_total_requests(&self) -> u64 {
        self.get_total_requests()
    }

    fn get_successful_requests(&self) -> u64 {
        self.get_successful_requests()
    }

    fn is_in_unhealthy_list(&self, backend_key: &str) -> bool {
        self.is_in_unhealthy_list(backend_key)
    }

    fn get_backend_latency(&self, backend_key: &str) -> Option<std::time::Duration> {
        self.get_latency_by_key(backend_key)
    }

    fn get_backend_error_count(&self, backend_key: &str) -> u64 {
        self.get_failure_count_by_key(backend_key) as u64
    }

    fn get_backend_request_count(&self, backend_key: &str) -> u64 {
        self.get_backend_request_count(backend_key)
    }

    fn record_backend_request(&self, backend_key: &str) {
        // MetricsCollector 没有直接的记录请求方法，使用成功记录
        self.record_success(backend_key);
    }

    fn record_backend_error(&self, backend_key: &str) {
        self.record_failure(backend_key);
    }

    fn record_backend_latency(&self, backend_key: &str, latency: std::time::Duration) {
        self.record_latency(backend_key, latency);
    }

    fn mark_backend_unhealthy(&self, backend_key: &str) {
        self.record_failure(backend_key);
    }

    fn mark_backend_healthy(&self, backend_key: &str) {
        self.record_success(backend_key);
    }
}
