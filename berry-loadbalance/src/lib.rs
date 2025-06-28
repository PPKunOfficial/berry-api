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
};
