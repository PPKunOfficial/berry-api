//! Berry Load Balance Library
//!
//! This library provides load balancing functionality for the Berry API system including:
//! - Backend selection strategies
//! - Health checking
//! - Metrics collection
//! - Load balance management

pub mod loadbalance;
pub mod route_selector;

// Re-export commonly used types from loadbalance module
pub use loadbalance::{
    BackendSelector, HealthChecker, HealthStats, HealthSummary, LoadBalanceManager,
    LoadBalanceService, MetricsCollector, RequestResult, SelectedBackend, ServiceHealth,
    SmartAiHealthChecker,
};

// Re-export route selector types
pub use route_selector::{
    FailedRouteAttempt, LoadBalanceRouteSelector, RouteBackend, RouteDetail, RouteErrorType,
    RouteProvider, RouteResult, RouteSelectionError, RouteSelector, RouteStats, SelectedRoute,
};
