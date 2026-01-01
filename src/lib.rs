//! shiki - Lightweight service coordination agent
//!
//! This crate provides functionality for coordinating service startup order
//! across multiple machines and containers via HTTP.
//!
//! # Overview
//!
//! shiki is designed to solve the problem of service dependency management
//! in distributed environments where systemd's built-in dependency mechanism
//! cannot reach across machine boundaries.
//!
//! # Modules
//!
//! - [`cli`] - Command-line interface definitions
//! - [`client`] - HTTP client for communicating with agents
//! - [`config`] - Configuration file parsing and validation
//! - [`error`] - Error types and error handling
//! - [`server`] - HTTP server and API handlers
//! - [`service`] - Service management and backends

pub mod cli;
pub mod client;
pub mod config;
pub mod error;
pub mod server;
pub mod service;

// Re-exports for convenience
pub use cli::Cli;
pub use client::ShikiClient;
pub use config::Config;
pub use error::{ErrorCode, Result, ShikiError};
pub use server::serve;
pub use service::{ServiceController, ServiceOperationResult, ServiceState, ServiceStatus};
