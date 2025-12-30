//! Agent and service configuration types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

use crate::error::ShikiError;

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Agent name (defaults to hostname).
    pub name: Option<String>,

    /// Agent mode.
    pub mode: AgentMode,

    /// Service control backend.
    pub backend: Backend,

    /// Agent tags.
    pub tags: Vec<String>,

    /// Metadata.
    pub metadata: HashMap<String, String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: None,
            mode: AgentMode::Standalone,
            backend: Backend::Systemd,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Agent operating mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    /// Standalone mode.
    #[default]
    Standalone,

    /// Cluster mode.
    Cluster,
}

impl FromStr for AgentMode {
    type Err = ShikiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "standalone" => Ok(AgentMode::Standalone),
            "cluster" => Ok(AgentMode::Cluster),
            _ => Err(ShikiError::config(format!("Unknown agent mode: {}", s))),
        }
    }
}

/// Service control backend type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    /// systemd backend.
    #[default]
    Systemd,

    /// Command execution backend.
    Exec,
}

impl FromStr for Backend {
    type Err = ShikiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "systemd" => Ok(Backend::Systemd),
            "exec" => Ok(Backend::Exec),
            _ => Err(ShikiError::config(format!("Unknown backend: {}", s))),
        }
    }
}

/// Service definition for exec backend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ServiceDefinition {
    /// Start command.
    pub start: String,

    /// Stop command.
    pub stop: String,

    /// Status check command.
    pub status: String,

    /// Reload command (optional).
    pub reload: Option<String>,

    /// Restart command (optional, defaults to stop + start).
    pub restart: Option<String>,

    /// Working directory.
    pub working_dir: Option<String>,

    /// Environment variables.
    pub env: Vec<String>,

    /// Command timeout in seconds.
    pub timeout: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert!(config.name.is_none());
        assert_eq!(config.mode, AgentMode::Standalone);
        assert_eq!(config.backend, Backend::Systemd);
        assert!(config.tags.is_empty());
    }

    #[test]
    fn test_backend_parse() {
        assert_eq!("systemd".parse::<Backend>().unwrap(), Backend::Systemd);
        assert_eq!("EXEC".parse::<Backend>().unwrap(), Backend::Exec);
        assert!("invalid".parse::<Backend>().is_err());
    }

    #[test]
    fn test_agent_mode_parse() {
        assert_eq!(
            "standalone".parse::<AgentMode>().unwrap(),
            AgentMode::Standalone
        );
        assert_eq!("cluster".parse::<AgentMode>().unwrap(), AgentMode::Cluster);
        assert!("invalid".parse::<AgentMode>().is_err());
    }

    #[test]
    fn test_service_definition_default() {
        let def = ServiceDefinition::default();
        assert!(def.start.is_empty());
        assert!(def.stop.is_empty());
        assert!(def.status.is_empty());
        assert!(def.reload.is_none());
        assert!(def.working_dir.is_none());
        assert!(def.env.is_empty());
    }
}
