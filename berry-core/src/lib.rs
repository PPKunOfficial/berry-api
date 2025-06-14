//! Berry Core Library
//!
//! This library provides core functionality for the Berry API system including:
//! - Configuration management
//! - Authentication and authorization
//! - Shared types and utilities

pub mod config;
pub mod auth;

// Re-export commonly used types
pub use config::model::{
    Config, Provider, ModelMapping as Model, UserToken, Backend,
    LoadBalanceStrategy, BillingMode, GlobalSettings, ProviderBackendType
};
pub use auth::{AuthenticatedUser, AuthError, AuthMiddleware};
