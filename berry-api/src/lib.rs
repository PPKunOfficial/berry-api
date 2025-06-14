//! Berry API Server Library
//! 
//! This library provides the main API server functionality for the Berry API system

pub mod app;
pub mod router;
pub mod static_files;
pub mod observability;
pub mod middleware;

// Re-export the main server function
pub use app::start_server;
