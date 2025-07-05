//! 路由选择器模块
//!
//! 这个模块提供了简化的负载均衡接口，将复杂的负载均衡逻辑抽象为简单的线路选择和状态报告操作。
//!
//! # 核心概念
//!
//! - **RouteSelector**: 核心trait，定义了线路选择的接口
//! - **SelectedRoute**: 选中的线路信息，包含提供商和后端信息
//! - **RouteResult**: 请求结果，用于向选择器报告状态
//! - **RouteStats**: 统计信息，用于监控和调试
//!
//! # 使用示例
//!
//! ```rust
//! use berry_loadbalance::route_selector::{RouteSelector, LoadBalanceRouteSelector, RouteResult};
//! use berry_loadbalance::LoadBalanceService;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // 创建负载均衡服务
//! let load_balancer = Arc::new(LoadBalanceService::new(config)?);
//! load_balancer.start().await?;
//!
//! // 创建路由选择器
//! let route_selector: Arc<dyn RouteSelector> =
//!     Arc::new(LoadBalanceRouteSelector::new(load_balancer));
//!
//! // 选择路由
//! let route = route_selector.select_route("gpt-4", None).await?;
//!
//! // 使用路由信息
//! let api_url = route.get_api_url("v1/chat/completions");
//! let api_key = route.get_api_key()?;
//!
//! // 发送请求...
//! let result = send_request(&api_url, &api_key).await;
//!
//! // 报告结果
//! match result {
//!     Ok(_) => {
//!         route_selector.report_result(
//!             &route.route_id,
//!             RouteResult::Success { latency: std::time::Duration::from_millis(100) }
//!         ).await;
//!     }
//!     Err(e) => {
//!         route_selector.report_result(
//!             &route.route_id,
//!             RouteResult::Failure {
//!                 error: e.to_string(),
//!                 error_type: Some(RouteErrorType::Network)
//!             }
//!         ).await;
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod adapter;
pub mod traits;
pub mod types;

// 重新导出主要类型，方便使用
pub use adapter::LoadBalanceRouteSelector;
pub use traits::RouteSelector;
pub use types::{
    FailedRouteAttempt, RouteBackend, RouteDetail, RouteErrorType, RouteProvider, RouteResult,
    RouteSelectionError, RouteStats, SelectedRoute,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 创建一个简单的mock路由选择器用于测试
    pub struct MockRouteSelector {
        pub routes: Vec<SelectedRoute>,
        pub current_index: std::sync::atomic::AtomicUsize,
    }

    impl MockRouteSelector {
        pub fn new(routes: Vec<SelectedRoute>) -> Self {
            Self {
                routes,
                current_index: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        pub fn create_test_route(
            route_id: &str,
            provider_name: &str,
            model_name: &str,
        ) -> SelectedRoute {
            SelectedRoute {
                route_id: route_id.to_string(),
                provider: RouteProvider {
                    name: provider_name.to_string(),
                    base_url: "https://api.example.com".to_string(),
                    api_key: "test-key".to_string(),
                    headers: std::collections::HashMap::new(),
                    timeout_seconds: 30,
                    backend_type: berry_core::ProviderBackendType::OpenAI,
                },
                backend: RouteBackend {
                    provider: provider_name.to_string(),
                    model: model_name.to_string(),
                    weight: 1.0,
                    enabled: true,
                    tags: vec![],
                },
                selection_time: Duration::from_millis(10),
            }
        }
    }

    #[async_trait::async_trait]
    impl RouteSelector for MockRouteSelector {
        async fn select_route(
            &self,
            _model_name: &str,
            _user_tags: Option<&[String]>,
        ) -> anyhow::Result<SelectedRoute, RouteSelectionError> {
            if self.routes.is_empty() {
                return Err(RouteSelectionError {
                    model_name: _model_name.to_string(),
                    message: "No routes available".to_string(),
                    total_routes: 0,
                    healthy_routes: 0,
                    enabled_routes: 0,
                    failed_attempts: vec![],
                });
            }

            let index = self
                .current_index
                .load(std::sync::atomic::Ordering::Relaxed);
            let route = self.routes[index % self.routes.len()].clone();
            self.current_index
                .store(index + 1, std::sync::atomic::Ordering::Relaxed);

            Ok(route)
        }

        async fn select_specific_route(
            &self,
            _model_name: &str,
            provider_name: &str,
        ) -> anyhow::Result<SelectedRoute, RouteSelectionError> {
            for route in &self.routes {
                if route.provider.name == provider_name {
                    return Ok(route.clone());
                }
            }

            Err(RouteSelectionError {
                model_name: _model_name.to_string(),
                message: format!("Provider '{}' not found", provider_name),
                total_routes: self.routes.len(),
                healthy_routes: self.routes.len(),
                enabled_routes: self.routes.len(),
                failed_attempts: vec![],
            })
        }

        async fn report_result(&self, _route_id: &str, _result: RouteResult) {
            // Mock实现，不做任何操作
        }

        async fn get_route_stats(&self) -> RouteStats {
            RouteStats::default()
        }
    }

    #[tokio::test]
    async fn test_mock_route_selector() {
        let routes = vec![
            MockRouteSelector::create_test_route("test:gpt-4", "openai", "gpt-4"),
            MockRouteSelector::create_test_route("test:claude", "anthropic", "claude-3"),
        ];

        let selector = MockRouteSelector::new(routes);

        // 测试路由选择
        let route = selector.select_route("gpt-4", None).await.unwrap();
        assert_eq!(route.route_id, "test:gpt-4");
        assert_eq!(route.provider.name, "openai");

        // 测试特定提供商选择
        let route = selector
            .select_specific_route("claude", "anthropic")
            .await
            .unwrap();
        assert_eq!(route.route_id, "test:claude");
        assert_eq!(route.provider.name, "anthropic");

        // 测试结果报告（不会出错）
        selector
            .report_result(
                &route.route_id,
                RouteResult::Success {
                    latency: Duration::from_millis(100),
                },
            )
            .await;

        // 测试统计信息
        let stats = selector.get_route_stats().await;
        assert_eq!(stats.total_requests, 0); // Mock实现返回默认值
    }

    #[test]
    fn test_selected_route_methods() {
        let route = MockRouteSelector::create_test_route("test:gpt-4", "openai", "gpt-4");

        // 测试URL构建
        let url = route.get_api_url("v1/chat/completions");
        assert_eq!(url, "https://api.example.com/v1/chat/completions");

        // 测试API密钥获取
        let api_key = route.get_api_key().unwrap();
        assert_eq!(api_key, "test-key");

        // 测试超时设置
        let timeout = route.get_timeout();
        assert_eq!(timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_route_stats() {
        let mut stats = RouteStats::default();
        assert_eq!(stats.success_rate(), 0.0);
        assert_eq!(stats.healthy_routes_count(), 0);

        // 添加一些测试数据
        stats.total_requests = 100;
        stats.successful_requests = 80;
        assert_eq!(stats.success_rate(), 0.8);

        // 添加路由详情
        stats.route_details.insert(
            "test:route".to_string(),
            RouteDetail {
                route_id: "test:route".to_string(),
                provider: "test".to_string(),
                model: "test-model".to_string(),
                is_healthy: true,
                request_count: 50,
                error_count: 5,
                average_latency: Some(Duration::from_millis(100)),
                current_weight: 1.0,
            },
        );

        assert_eq!(stats.healthy_routes_count(), 1);
    }
}
