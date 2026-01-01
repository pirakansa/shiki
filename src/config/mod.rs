//! Configuration module for shiki.
//!
//! This module provides all configuration types and loading functionality.
//! Configuration can be loaded from YAML files and environment variables.

mod acl;
mod agent;
mod cluster;
mod logging;
mod retry;
mod server;

pub use acl::AclConfig;
pub use agent::{AgentConfig, AgentMode, Backend, ServiceDefinition};
pub use cluster::{ClusterConfig, PeerConfig};
pub use logging::{LogFormat, LogLevel, LogOutput, LoggingConfig};
pub use retry::{RetryConfig, TimeoutConfig};
pub use server::{AuthConfig, AuthMethod, ServerConfig, TlsConfig};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::ShikiError;

/// Application configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Server configuration.
    pub server: ServerConfig,

    /// Authentication configuration.
    pub auth: AuthConfig,

    /// Logging configuration.
    pub logging: LoggingConfig,

    /// Agent configuration.
    pub agent: AgentConfig,

    /// Retry configuration.
    pub retry: RetryConfig,

    /// Timeout configuration.
    pub timeout: TimeoutConfig,

    /// Access control configuration.
    pub acl: AclConfig,

    /// Cluster configuration.
    pub cluster: ClusterConfig,

    /// Service definitions (for exec backend).
    #[serde(default)]
    pub services: HashMap<String, ServiceDefinition>,
}

impl Config {
    /// Loads configuration from an optional path.
    /// If path is None, uses default search paths.
    pub fn load<P: AsRef<Path>>(path: Option<P>) -> Result<Self, ShikiError> {
        match path {
            Some(p) => Self::load_from_path(p),
            None => {
                // Try default paths
                let default_paths = [
                    "/etc/shiki/config.yaml",
                    "/etc/shiki/config.yml",
                    "config.yaml",
                    "config.yml",
                ];

                for path in &default_paths {
                    if std::path::Path::new(path).exists() {
                        return Self::load_from_path(path);
                    }
                }

                // No config file found, use defaults
                Ok(Self::default())
            }
        }
    }

    /// Loads configuration from a YAML file.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, ShikiError> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            ShikiError::config(format!(
                "Failed to read config file '{}': {}",
                path.as_ref().display(),
                e
            ))
        })?;

        Self::load_from_str(&content)
    }

    /// Loads configuration from a YAML string.
    pub fn load_from_str(content: &str) -> Result<Self, ShikiError> {
        let config: Config = serde_yaml::from_str(content)
            .map_err(|e| ShikiError::config(format!("Failed to parse config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Validates configuration.
    fn validate(&self) -> Result<(), ShikiError> {
        // Validate port
        if self.server.port == 0 {
            return Err(ShikiError::config("server.port must be > 0"));
        }

        // Validate TLS
        if self.server.tls.enabled {
            if self.server.tls.cert_path.is_none() {
                return Err(ShikiError::config(
                    "server.tls.cert_path is required when TLS is enabled",
                ));
            }
            if self.server.tls.key_path.is_none() {
                return Err(ShikiError::config(
                    "server.tls.key_path is required when TLS is enabled",
                ));
            }
        }

        // Validate auth
        if self.auth.enabled {
            match self.auth.method {
                AuthMethod::Token if self.auth.token.is_none() => {
                    return Err(ShikiError::config(
                        "auth.token is required when using token authentication",
                    ));
                }
                AuthMethod::ApiKey if self.auth.api_keys.is_empty() => {
                    return Err(ShikiError::config(
                        "auth.api_keys is required when using API key authentication",
                    ));
                }
                _ => {}
            }
        }

        // Validate exec backend requires service definitions
        if self.agent.backend == Backend::Exec && self.services.is_empty() {
            return Err(ShikiError::config(
                "services section is required when using exec backend",
            ));
        }

        // Validate service definitions
        for (name, def) in &self.services {
            if def.start.is_empty() {
                return Err(ShikiError::config(format!(
                    "services.{}.start is required",
                    name
                )));
            }
            if def.stop.is_empty() {
                return Err(ShikiError::config(format!(
                    "services.{}.stop is required",
                    name
                )));
            }
            if def.status.is_empty() {
                return Err(ShikiError::config(format!(
                    "services.{}.status is required",
                    name
                )));
            }
        }

        // Validate logging
        if self.logging.output == LogOutput::File && self.logging.file_path.is_none() {
            return Err(ShikiError::config(
                "logging.file_path is required when output is file",
            ));
        }

        // Validate retry
        if self.retry.max_attempts == 0 {
            return Err(ShikiError::config("retry.max_attempts must be > 0"));
        }

        // Validate cluster
        if self.cluster.enabled && self.cluster.peers.is_empty() {
            return Err(ShikiError::config(
                "cluster.peers is required when cluster is enabled",
            ));
        }

        Ok(())
    }

    /// Returns the agent name (configured name or hostname).
    pub fn agent_name(&self) -> String {
        self.agent.name.clone().unwrap_or_else(|| {
            hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.server.bind, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert!(!config.server.tls.enabled);
        assert!(!config.auth.enabled);
        assert_eq!(config.logging.level, LogLevel::Info);
        assert_eq!(config.logging.format, LogFormat::Json);
        assert_eq!(config.agent.backend, Backend::Systemd);
        assert_eq!(config.retry.max_attempts, 3);
        assert_eq!(config.timeout.service_seconds, 60);
    }

    #[test]
    fn test_load_from_yaml() {
        let yaml = r#"
server:
  bind: "127.0.0.1"
  port: 9090

logging:
  level: debug
  format: text

agent:
  name: "test-agent"
  backend: systemd
  tags:
    - production
    - web
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::load_from_path(file.path()).unwrap();

        assert_eq!(config.server.bind, "127.0.0.1");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.logging.level, LogLevel::Debug);
        assert_eq!(config.logging.format, LogFormat::Text);
        assert_eq!(config.agent.name, Some("test-agent".to_string()));
        assert_eq!(config.agent.tags, vec!["production", "web"]);
    }

    #[test]
    fn test_load_exec_backend() {
        let yaml = r#"
agent:
  backend: exec

services:
  nginx:
    start: "/usr/sbin/nginx"
    stop: "/usr/sbin/nginx -s quit"
    status: "pgrep -x nginx"
  
  redis:
    start: "redis-server --daemonize yes"
    stop: "redis-cli shutdown"
    status: "redis-cli ping"
    working_dir: "/var/lib/redis"
    env:
      - "REDIS_PORT=6379"
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::load_from_path(file.path()).unwrap();

        assert_eq!(config.agent.backend, Backend::Exec);
        assert_eq!(config.services.len(), 2);

        let nginx = config.services.get("nginx").unwrap();
        assert_eq!(nginx.start, "/usr/sbin/nginx");
        assert_eq!(nginx.stop, "/usr/sbin/nginx -s quit");

        let redis = config.services.get("redis").unwrap();
        assert_eq!(redis.working_dir, Some("/var/lib/redis".to_string()));
        assert_eq!(redis.env, vec!["REDIS_PORT=6379"]);
    }

    #[test]
    fn test_validation_port_zero() {
        let yaml = r#"
server:
  port: 0
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = Config::load_from_path(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("port"));
    }

    #[test]
    fn test_validation_tls_without_cert() {
        let yaml = r#"
server:
  tls:
    enabled: true
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = Config::load_from_path(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cert_path"));
    }

    #[test]
    fn test_validation_exec_without_services() {
        let yaml = r#"
agent:
  backend: exec
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = Config::load_from_path(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("services"));
    }

    #[test]
    fn test_validation_service_missing_command() {
        let yaml = r#"
agent:
  backend: exec

services:
  broken:
    start: "echo start"
    stop: ""
    status: "echo status"
"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let result = Config::load_from_path(file.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("stop"));
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();

        assert!(yaml.contains("bind:"));
        assert!(yaml.contains("port:"));
        assert!(yaml.contains("level:"));
    }

    #[test]
    fn test_agent_name_default_to_hostname() {
        let config = Config::default();
        let name = config.agent_name();

        // Should return hostname or "unknown"
        assert!(!name.is_empty());
    }

    #[test]
    fn test_agent_name_configured() {
        let mut config = Config::default();
        config.agent.name = Some("my-agent".to_string());

        assert_eq!(config.agent_name(), "my-agent");
    }
}
