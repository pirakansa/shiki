//! Application state management.
//!
//! This module manages the shared state across HTTP request handlers.

use crate::config::Config;
use crate::error::Result;
use crate::service::ServiceController;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Shared application state.
pub struct AppState {
    /// Service controller for managing services.
    pub controller: ServiceController,
    /// Application start time.
    pub start_time: Instant,
    /// Agent name.
    pub agent_name: String,
    /// Agent tags.
    pub agent_tags: Vec<String>,
    /// Server bind address.
    pub server_bind: String,
    /// Server port.
    pub server_port: u16,
    /// TLS enabled flag.
    pub tls_enabled: bool,
    /// Statistics counters.
    pub stats: Stats,
}

impl AppState {
    /// Creates a new application state from configuration.
    pub fn new(config: &Config) -> Result<Self> {
        let controller = ServiceController::from_config(config)?;

        Ok(Self {
            controller,
            start_time: Instant::now(),
            agent_name: config.agent_name(),
            agent_tags: config.agent.tags.clone(),
            server_bind: config.server.bind.clone(),
            server_port: config.server.port,
            tls_enabled: config.server.tls.enabled,
            stats: Stats::default(),
        })
    }

    /// Returns the uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Increments the total request counter.
    pub fn increment_requests(&self) {
        self.stats.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the successful request counter.
    pub fn increment_success(&self) {
        self.stats.requests_success.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the failed request counter.
    pub fn increment_failed(&self) {
        self.stats.requests_failed.fetch_add(1, Ordering::Relaxed);
    }
}

/// Statistics counters.
#[derive(Default)]
pub struct Stats {
    /// Total requests received.
    pub requests_total: AtomicU64,
    /// Successful requests.
    pub requests_success: AtomicU64,
    /// Failed requests.
    pub requests_failed: AtomicU64,
}

impl Stats {
    /// Gets the current statistics as a snapshot.
    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            requests_total: self.requests_total.load(Ordering::Relaxed),
            requests_success: self.requests_success.load(Ordering::Relaxed),
            requests_failed: self.requests_failed.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of statistics counters.
#[derive(Debug, Clone, Default)]
pub struct StatsSnapshot {
    /// Total requests received.
    pub requests_total: u64,
    /// Successful requests.
    pub requests_success: u64,
    /// Failed requests.
    pub requests_failed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Backend, ServiceDefinition};
    use std::collections::HashMap;

    fn create_test_config() -> Config {
        let mut config = Config::default();
        config.agent.backend = Backend::Exec;
        config.agent.name = Some("test-agent".to_string());
        config.agent.tags = vec!["test".to_string()];

        let mut services = HashMap::new();
        services.insert(
            "test-service".to_string(),
            ServiceDefinition {
                start: "true".to_string(),
                stop: "true".to_string(),
                status: "true".to_string(),
                restart: None,
                working_dir: None,
                env: vec![],
            },
        );
        config.services = services;

        config
    }

    #[test]
    fn test_app_state_new() {
        let config = create_test_config();
        let state = AppState::new(&config).unwrap();

        assert_eq!(state.agent_name, "test-agent");
        assert_eq!(state.agent_tags, vec!["test".to_string()]);
        assert_eq!(state.server_bind, "0.0.0.0");
        assert_eq!(state.server_port, 8080);
        assert!(!state.tls_enabled);
    }

    #[test]
    fn test_app_state_uptime() {
        let config = create_test_config();
        let state = AppState::new(&config).unwrap();

        // Uptime should be very small right after creation
        assert!(state.uptime_seconds() < 1);
    }

    #[test]
    fn test_stats_increment() {
        let config = create_test_config();
        let state = AppState::new(&config).unwrap();

        state.increment_requests();
        state.increment_requests();
        state.increment_success();
        state.increment_failed();

        let snapshot = state.stats.snapshot();
        assert_eq!(snapshot.requests_total, 2);
        assert_eq!(snapshot.requests_success, 1);
        assert_eq!(snapshot.requests_failed, 1);
    }

    #[test]
    fn test_stats_default() {
        let stats = Stats::default();
        let snapshot = stats.snapshot();

        assert_eq!(snapshot.requests_total, 0);
        assert_eq!(snapshot.requests_success, 0);
        assert_eq!(snapshot.requests_failed, 0);
    }
}
