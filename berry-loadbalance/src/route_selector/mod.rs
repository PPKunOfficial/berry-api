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
