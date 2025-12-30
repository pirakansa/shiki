//! Configuration management for shiki.
//!
//! This module handles loading, parsing, and validating configuration files.
//! Configuration can be loaded from YAML files and overridden by environment variables.

use crate::error::{Result, ShikiError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

/// Default configuration file path.
pub const DEFAULT_CONFIG_PATH: &str = "/etc/shiki/config.yaml";

/// Environment variable for configuration file path.
pub const ENV_CONFIG_PATH: &str = "SHIKI_CONFIG";

/// Root configuration structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// HTTP server configuration.
    pub server: ServerConfig,

    /// Authentication configuration.
    pub auth: AuthConfig,

    /// Logging configuration.
    pub logging: LoggingConfig,

    /// Agent configuration.
    pub agent: AgentConfig,

    /// Service definitions (for exec backend).
    #[serde(default)]
    pub services: HashMap<String, ServiceDefinition>,

    /// Retry configuration.
    pub retry: RetryConfig,

    /// Timeout configuration.
    pub timeout: TimeoutConfig,

    /// Access control list configuration.
    pub acl: AclConfig,

    /// Cluster configuration.
    pub cluster: ClusterConfig,
}

impl Config {
    /// Loads configuration from the specified path.
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            ShikiError::config_with_source(
                format!("Failed to read config file: {}", path.display()),
                e,
            )
        })?;

        let config: Config = serde_yaml::from_str(&content).map_err(|e| {
            ShikiError::config_with_source(
                format!("Failed to parse config file: {}", path.display()),
                e,
            )
        })?;

        config.validate()?;
        Ok(config)
    }

    /// Loads configuration with the following priority:
    /// 1. Explicit path (if provided)
    /// 2. SHIKI_CONFIG environment variable
    /// 3. Default path (/etc/shiki/config.yaml)
    ///
    /// Returns default config if no file exists.
    pub fn load(explicit_path: Option<&Path>) -> Result<Self> {
        let path = Self::resolve_config_path(explicit_path);

        if let Some(path) = path {
            if path.exists() {
                let mut config = Self::load_from_path(&path)?;
                config.apply_env_overrides();
                config.validate()?;
                return Ok(config);
            } else if explicit_path.is_some() {
                // Explicit path was provided but doesn't exist
                return Err(ShikiError::config(format!(
                    "Configuration file not found: {}",
                    path.display()
                )));
            }
        }

        // No config file found, use defaults with env overrides
        let mut config = Config::default();
        config.apply_env_overrides();
        config.validate()?;
        Ok(config)
    }

    /// Resolves the configuration file path based on priority.
    fn resolve_config_path(explicit_path: Option<&Path>) -> Option<PathBuf> {
        if let Some(path) = explicit_path {
            return Some(path.to_path_buf());
        }

        if let Ok(env_path) = env::var(ENV_CONFIG_PATH) {
            return Some(PathBuf::from(env_path));
        }

        Some(PathBuf::from(DEFAULT_CONFIG_PATH))
    }

    /// Applies environment variable overrides to the configuration.
    pub fn apply_env_overrides(&mut self) {
        // Server settings
        if let Ok(bind) = env::var("SHIKI_SERVER_BIND") {
            self.server.bind = bind;
        }
        if let Ok(port) = env::var("SHIKI_SERVER_PORT") {
            if let Ok(port) = port.parse() {
                self.server.port = port;
            }
        }

        // Auth settings
        if let Ok(enabled) = env::var("SHIKI_AUTH_ENABLED") {
            self.auth.enabled = enabled.parse().unwrap_or(false);
        }
        if let Ok(token) = env::var("SHIKI_AUTH_TOKEN") {
            self.auth.token = Some(token);
        }

        // Logging settings
        if let Ok(level) = env::var("SHIKI_LOG_LEVEL") {
            if let Ok(level) = level.parse() {
                self.logging.level = level;
            }
        }
        if let Ok(format) = env::var("SHIKI_LOG_FORMAT") {
            if let Ok(format) = format.parse() {
                self.logging.format = format;
            }
        }

        // Agent settings
        if let Ok(name) = env::var("SHIKI_AGENT_NAME") {
            self.agent.name = Some(name);
        }
        if let Ok(backend) = env::var("SHIKI_AGENT_BACKEND") {
            if let Ok(backend) = backend.parse() {
                self.agent.backend = backend;
            }
        }
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<()> {
        // Validate port range
        if self.server.port == 0 {
            return Err(ShikiError::config(
                "Server port must be between 1 and 65535",
            ));
        }

        // Validate TLS configuration
        if self.server.tls.enabled {
            if self.server.tls.cert_path.is_none() {
                return Err(ShikiError::config(
                    "TLS is enabled but cert_path is not specified",
                ));
            }
            if self.server.tls.key_path.is_none() {
                return Err(ShikiError::config(
                    "TLS is enabled but key_path is not specified",
                ));
            }
        }

        // Validate logging file path
        if self.logging.output == LogOutput::File && self.logging.file_path.is_none() {
            return Err(ShikiError::config(
                "Log output is 'file' but file_path is not specified",
            ));
        }

        // Validate exec backend requires services
        if self.agent.backend == Backend::Exec && self.services.is_empty() {
            return Err(ShikiError::config(
                "Backend is 'exec' but no services are defined",
            ));
        }

        // Validate service definitions
        for (name, service) in &self.services {
            service.validate(name)?;
        }

        // Validate retry configuration
        if self.retry.max_attempts == 0 {
            return Err(ShikiError::config("retry.max_attempts must be at least 1"));
        }

        Ok(())
    }

    /// Returns the effective agent name (configured or hostname).
    pub fn agent_name(&self) -> String {
        self.agent.name.clone().unwrap_or_else(|| {
            hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        })
    }
}

/// HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Bind address.
    pub bind: String,

    /// Listen port.
    pub port: u16,

    /// TLS configuration.
    pub tls: TlsConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0".to_string(),
            port: 8080,
            tls: TlsConfig::default(),
        }
    }
}

/// TLS configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TlsConfig {
    /// Enable TLS.
    pub enabled: bool,

    /// Path to server certificate.
    pub cert_path: Option<String>,

    /// Path to private key.
    pub key_path: Option<String>,
}

/// Authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Enable authentication.
    pub enabled: bool,

    /// Authentication method.
    pub method: AuthMethod,

    /// Bearer token (for token auth).
    pub token: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            method: AuthMethod::Token,
            token: None,
        }
    }
}

/// Authentication method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    /// Bearer token authentication.
    #[default]
    Token,
    /// Mutual TLS authentication.
    Mtls,
}

impl std::str::FromStr for AuthMethod {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "token" => Ok(AuthMethod::Token),
            "mtls" => Ok(AuthMethod::Mtls),
            _ => Err(format!("Invalid auth method: {}", s)),
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level.
    pub level: LogLevel,

    /// Log format.
    pub format: LogFormat,

    /// Log output destination.
    pub output: LogOutput,

    /// Log file path (when output is "file").
    pub file_path: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Json,
            output: LogOutput::Stdout,
            file_path: None,
        }
    }
}

/// Log level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

impl LogLevel {
    /// Converts to tracing level filter.
    pub fn to_tracing_level(&self) -> tracing::Level {
        match self {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Log format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Json,
    Text,
}

impl std::str::FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(LogFormat::Json),
            "text" => Ok(LogFormat::Text),
            _ => Err(format!("Invalid log format: {}", s)),
        }
    }
}

/// Log output destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    #[default]
    Stdout,
    Stderr,
    File,
}

impl std::str::FromStr for LogOutput {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stdout" => Ok(LogOutput::Stdout),
            "stderr" => Ok(LogOutput::Stderr),
            "file" => Ok(LogOutput::File),
            _ => Err(format!("Invalid log output: {}", s)),
        }
    }
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Agent name (defaults to hostname).
    pub name: Option<String>,

    /// Operation mode.
    pub mode: AgentMode,

    /// Backend type.
    pub backend: Backend,

    /// Tags for filtering.
    pub tags: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: None,
            mode: AgentMode::Standalone,
            backend: Backend::Systemd,
            tags: Vec::new(),
        }
    }
}

/// Agent operation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    #[default]
    Standalone,
    Cluster,
}

impl std::str::FromStr for AgentMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "standalone" => Ok(AgentMode::Standalone),
            "cluster" => Ok(AgentMode::Cluster),
            _ => Err(format!("Invalid agent mode: {}", s)),
        }
    }
}

/// Backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    #[default]
    Systemd,
    Exec,
}

impl std::str::FromStr for Backend {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "systemd" => Ok(Backend::Systemd),
            "exec" => Ok(Backend::Exec),
            _ => Err(format!("Invalid backend: {}", s)),
        }
    }
}

/// Service definition for exec backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    /// Start command.
    pub start: String,

    /// Stop command.
    pub stop: String,

    /// Status check command (exit code 0 = running).
    pub status: String,

    /// Restart command (optional, defaults to stop then start).
    pub restart: Option<String>,

    /// Working directory.
    pub working_dir: Option<String>,

    /// Environment variables (KEY=VALUE format).
    #[serde(default)]
    pub env: Vec<String>,
}

impl ServiceDefinition {
    /// Validates the service definition.
    pub fn validate(&self, name: &str) -> Result<()> {
        if self.start.is_empty() {
            return Err(ShikiError::config(format!(
                "Service '{}': start command is required",
                name
            )));
        }
        if self.stop.is_empty() {
            return Err(ShikiError::config(format!(
                "Service '{}': stop command is required",
                name
            )));
        }
        if self.status.is_empty() {
            return Err(ShikiError::config(format!(
                "Service '{}': status command is required",
                name
            )));
        }
        Ok(())
    }
}

/// Retry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,

    /// Initial retry delay in milliseconds.
    pub delay_ms: u64,

    /// Exponential backoff factor.
    pub backoff_factor: f64,

    /// Maximum retry delay in milliseconds.
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            delay_ms: 1000,
            backoff_factor: 2.0,
            max_delay_ms: 30000,
        }
    }
}

/// Timeout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeoutConfig {
    /// TCP connection timeout in seconds.
    pub connect_seconds: u64,

    /// HTTP response read timeout in seconds.
    pub read_seconds: u64,

    /// Service operation timeout in seconds.
    pub service_seconds: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_seconds: 5,
            read_seconds: 30,
            service_seconds: 60,
        }
    }
}

/// Access control list configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AclConfig {
    /// Allowed services (empty = allow all).
    pub allowed: Vec<String>,

    /// Denied services.
    pub denied: Vec<String>,
}

impl AclConfig {
    /// Checks if a service is allowed.
    /// Evaluation order: denied -> (allowed empty = allow all) -> allowed match -> deny
    pub fn is_allowed(&self, service: &str) -> bool {
        // Check denied list first
        for pattern in &self.denied {
            if glob_match::glob_match(pattern, service) {
                return false;
            }
        }

        // If allowed list is empty, allow all
        if self.allowed.is_empty() {
            return true;
        }

        // Check allowed list
        for pattern in &self.allowed {
            if glob_match::glob_match(pattern, service) {
                return true;
            }
        }

        false
    }
}

/// Cluster configuration (future implementation).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClusterConfig {
    /// Enable cluster mode.
    pub enabled: bool,

    /// Peer agents.
    pub peers: Vec<PeerConfig>,
}

/// Peer agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    /// Peer name.
    pub name: String,

    /// Peer address (host:port).
    pub address: String,

    /// Peer tags.
    #[serde(default)]
    pub tags: Vec<String>,
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
    fn test_acl_is_allowed() {
        let acl = AclConfig {
            allowed: vec!["nginx".to_string(), "redis-*".to_string()],
            denied: vec!["redis-test".to_string()],
        };

        assert!(acl.is_allowed("nginx"));
        assert!(acl.is_allowed("redis-server"));
        assert!(acl.is_allowed("redis-sentinel"));
        assert!(!acl.is_allowed("redis-test")); // denied
        assert!(!acl.is_allowed("postgres")); // not in allowed
    }

    #[test]
    fn test_acl_empty_allowed() {
        let acl = AclConfig {
            allowed: vec![],
            denied: vec!["secret-*".to_string()],
        };

        assert!(acl.is_allowed("nginx"));
        assert!(acl.is_allowed("redis"));
        assert!(!acl.is_allowed("secret-service"));
    }

    #[test]
    fn test_log_level_parse() {
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert_eq!("DEBUG".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("Info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_backend_parse() {
        assert_eq!("systemd".parse::<Backend>().unwrap(), Backend::Systemd);
        assert_eq!("EXEC".parse::<Backend>().unwrap(), Backend::Exec);
        assert!("invalid".parse::<Backend>().is_err());
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
