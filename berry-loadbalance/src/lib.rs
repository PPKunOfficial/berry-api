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
    BackendSelector, HealthChecker, HealthStats, HealthSummary, LoadBalanceManager,
    LoadBalanceService, MetricsCollector, RequestResult, SelectedBackend, ServiceHealth,
    SmartAiHealthChecker,
    // 新的路由选择器相关类型
    LoadBalanceRouteSelector, RouteSelector, SelectedRoute, RouteResult, RouteStats,
    RouteSelectionError, RouteErrorType, RouteDetail, FailedRouteAttempt,
};
