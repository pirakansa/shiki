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
//! - [`config`] - Configuration file parsing and validation
//! - [`error`] - Error types and error handling

pub mod cli;
pub mod config;
pub mod error;

// Re-exports for convenience
pub use cli::Cli;
pub use config::Config;
pub use error::{ErrorCode, Result, ShikiError};
