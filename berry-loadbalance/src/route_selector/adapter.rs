use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use super::traits::RouteSelector;
use super::types::{
    FailedRouteAttempt, RouteBackend, RouteDetail, RouteProvider, RouteResult, RouteSelectionError,
    RouteStats, SelectedRoute,
};

/// 基于现有LoadBalanceService的RouteSelector实现
pub struct LoadBalanceRouteSelector {
    service: Arc<crate::loadbalance::service::LoadBalanceService>,
}

impl LoadBalanceRouteSelector {
    /// 创建新的路由选择器
    pub fn new(service: Arc<crate::loadbalance::service::LoadBalanceService>) -> Self {
        Self { service }
    }

    /// 将Backend转换为RouteBackend
    fn convert_backend(backend: &berry_core::Backend) -> RouteBackend {
        RouteBackend {
            provider: backend.provider.clone(),
            model: backend.model.clone(),
            weight: backend.weight,
            enabled: backend.enabled,
            tags: backend.tags.clone(),
        }
    }

    /// 将Provider转换为RouteProvider
    fn convert_provider(provider: &berry_core::Provider) -> RouteProvider {
        RouteProvider {
            name: provider.name.clone(),
            base_url: provider.base_url.clone(),
            api_key: provider.api_key.clone(),
            headers: provider.headers.clone(),
            timeout_seconds: provider.timeout_seconds,
            backend_type: provider.backend_type.clone(),
        }
    }

    /// 将SelectedBackend转换为SelectedRoute
    fn convert_selected_backend(
        selected: crate::loadbalance::service::SelectedBackend,
    ) -> SelectedRoute {
        let route_id = format!("{}:{}", selected.backend.provider, selected.backend.model);

        SelectedRoute {
            route_id,
            provider: Self::convert_provider(&selected.provider),
            backend: Self::convert_backend(&selected.backend),
            selection_time: selected.selection_time,
        }
    }

    /// 将RouteResult转换为RequestResult
    fn convert_route_result(result: RouteResult) -> crate::loadbalance::service::RequestResult {
        match result {
            RouteResult::Success { latency } => {
                crate::loadbalance::service::RequestResult::Success { latency }
            }
            RouteResult::Failure { error, .. } => {
                crate::loadbalance::service::RequestResult::Failure { error }
            }
        }
    }

    /// 解析route_id获取provider和model
    fn parse_route_id(route_id: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = route_id.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid route_id format: {}", route_id);
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }
}

#[async_trait]
impl RouteSelector for LoadBalanceRouteSelector {
    async fn select_route(
        &self,
        model_name: &str,
        user_tags: Option<&[String]>,
    ) -> Result<SelectedRoute, RouteSelectionError> {
        match self
            .service
            .select_backend_with_user_tags(model_name, user_tags)
            .await
        {
            Ok(selected) => Ok(Self::convert_selected_backend(selected)),
            Err(e) => {
                // 尝试从错误中提取详细信息
                if let Some(detailed_error) =
                    e.downcast_ref::<crate::loadbalance::selector::BackendSelectionError>()
                {
                    let failed_attempts = detailed_error
                        .failed_attempts
                        .iter()
                        .map(|attempt| FailedRouteAttempt {
                            route_id: attempt.backend_key.clone(),
                            provider: attempt.provider.clone(),
                            model: attempt.model.clone(),
                            reason: attempt.reason.clone(),
                            is_healthy: attempt.is_healthy,
                        })
                        .collect();

                    Err(RouteSelectionError {
                        model_name: detailed_error.model_name.clone(),
                        message: detailed_error.error_message.clone(),
                        total_routes: detailed_error.total_backends,
                        healthy_routes: detailed_error.healthy_backends,
                        enabled_routes: detailed_error.enabled_backends,
                        failed_attempts,
                    })
                } else {
                    Err(RouteSelectionError {
                        model_name: model_name.to_string(),
                        message: e.to_string(),
                        total_routes: 0,
                        healthy_routes: 0,
                        enabled_routes: 0,
                        failed_attempts: vec![],
                    })
                }
            }
        }
    }

    async fn select_specific_route(
        &self,
        model_name: &str,
        provider_name: &str,
    ) -> Result<SelectedRoute, RouteSelectionError> {
        match self
            .service
            .select_specific_backend(model_name, provider_name)
            .await
        {
            Ok(selected) => Ok(Self::convert_selected_backend(selected)),
            Err(e) => Err(RouteSelectionError {
                model_name: model_name.to_string(),
                message: e.to_string(),
                total_routes: 0,
                healthy_routes: 0,
                enabled_routes: 0,
                failed_attempts: vec![],
            }),
        }
    }

    async fn report_result(&self, route_id: &str, result: RouteResult) {
        match Self::parse_route_id(route_id) {
            Ok((provider, model)) => {
                let request_result = Self::convert_route_result(result);
                self.service
                    .record_request_result(&provider, &model, request_result)
                    .await;
            }
            Err(e) => {
                tracing::error!("Failed to parse route_id '{}': {}", route_id, e);
            }
        }
    }

    async fn get_route_stats(&self) -> RouteStats {
        let service_health = self.service.get_service_health().await;
        let metrics = self.service.get_metrics();

        let mut route_details = std::collections::HashMap::new();

        // 获取所有请求计数
        let request_counts = metrics.get_all_request_counts();

        for (backend_key, request_count) in request_counts {
            if let Ok((provider, model)) = Self::parse_route_id(&backend_key) {
                let is_healthy = metrics.is_healthy(&provider, &model);
                let error_count = metrics.get_failure_count(&provider, &model) as u64;
                let average_latency = metrics.get_latency(&provider, &model);

                // 尝试获取当前权重（这需要知道模型名称，这里简化处理）
                let current_weight = 1.0; // 简化处理，实际应该从配置中获取

                route_details.insert(
                    backend_key.clone(),
                    RouteDetail {
                        route_id: backend_key,
                        provider,
                        model,
                        is_healthy,
                        request_count,
                        error_count,
                        average_latency,
                        current_weight,
                    },
                );
            }
        }

        RouteStats {
            total_requests: service_health.total_requests,
            successful_requests: service_health.successful_requests,
            route_details,
        }
    }
}
