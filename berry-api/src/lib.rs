//! Berry API Server Library
//!
//! This library provides the main API server functionality for the Berry API system

pub mod app;
pub mod database;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod observability;
pub mod repositories;
pub mod router;
pub mod static_files;

// Re-export the main server function
pub use app::start_server;
