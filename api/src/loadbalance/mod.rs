pub mod selector;
pub mod manager;
pub mod health_checker;
pub mod service;
pub mod smart_ai_health;

#[cfg(test)]
mod manager_tests;

pub use selector::{BackendSelector, MetricsCollector};
pub use manager::{LoadBalanceManager, HealthStats};
pub use health_checker::{HealthChecker, HealthSummary};
pub use service::{LoadBalanceService, SelectedBackend, RequestResult, ServiceHealth};
pub use smart_ai_health::SmartAiHealthChecker;
