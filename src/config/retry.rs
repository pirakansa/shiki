//! Retry and timeout configuration types.

use serde::{Deserialize, Serialize};

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,

    /// Initial retry interval in milliseconds.
    pub initial_interval_ms: u64,

    /// Maximum retry interval in milliseconds.
    pub max_interval_ms: u64,

    /// Backoff multiplier.
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_interval_ms: 1000,
            max_interval_ms: 30000,
            multiplier: 2.0,
        }
    }
}

/// Timeout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeoutConfig {
    /// Service operation timeout in seconds.
    pub service_seconds: u64,

    /// HTTP request timeout in seconds.
    pub http_seconds: u64,

    /// Health check timeout in seconds.
    pub health_seconds: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            service_seconds: 60,
            http_seconds: 30,
            health_seconds: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_interval_ms, 1000);
        assert_eq!(config.max_interval_ms, 30000);
        assert!((config.multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_timeout_config_default() {
        let config = TimeoutConfig::default();
        assert_eq!(config.service_seconds, 60);
        assert_eq!(config.http_seconds, 30);
        assert_eq!(config.health_seconds, 5);
    }
}
