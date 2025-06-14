//! Berry Relay Library
//! 
//! This library provides request relay functionality for the Berry API system including:
//! - Request handlers
//! - Client implementations
//! - Protocol adapters

pub mod relay;

// Re-export commonly used types
pub use relay::handler::LoadBalancedHandler;
