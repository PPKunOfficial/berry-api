//! Berry Load Balance Library
//!
//! This library provides load balancing functionality for the Berry API system including:
//! - Backend selection strategies
//! - Health checking
//! - Metrics collection
//! - Load balance management

pub mod loadbalance;

// Re-export commonly used types
pub use loadbalance::{
    BackendSelector,
    FailedRouteAttempt,
    HealthChecker,
    HealthStats,
    HealthSummary,
    LoadBalanceManager,
    // 新的路由选择器相关类型
    LoadBalanceRouteSelector,
    LoadBalanceService,
    MetricsCollector,
    RequestResult,
    RouteDetail,
    RouteErrorType,
    RouteResult,
    RouteSelectionError,
    RouteSelector,
    RouteStats,
    SelectedBackend,
    SelectedRoute,
    ServiceHealth,
    SmartAiHealthChecker,
};
