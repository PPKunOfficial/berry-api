//! Berry Core Library
//!
//! This library provides core functionality for the Berry API system including:
//! - Configuration management
//! - Authentication and authorization
//! - Shared types and utilities

pub mod auth;
pub mod client;
pub mod config;

// Re-export commonly used types
pub use auth::{AuthError, AuthMiddleware, AuthenticatedUser};
pub use client::{
    AIBackendClient, BackendType, ChatCompletionConfig, ChatMessage, ChatRole, ClientError,
    ClientFactory, ClientResponse, UnifiedClient,
};
pub use config::model::{
    Backend, BillingMode, Config, GlobalSettings, LoadBalanceStrategy, ModelMapping as Model,
    Provider, ProviderBackendType, UserToken,
};
