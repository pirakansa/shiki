//! Server configuration types.
//!
//! Contains configurations for HTTP server, TLS, and authentication.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::error::ShikiError;

/// HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Listen address.
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

    /// Certificate file path.
    pub cert_path: Option<String>,

    /// Private key file path.
    pub key_path: Option<String>,
}

/// Authentication configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Enable authentication.
    pub enabled: bool,

    /// Authentication method.
    pub method: AuthMethod,

    /// Static token for token auth.
    pub token: Option<String>,

    /// API key list for key auth.
    pub api_keys: Vec<String>,
}

/// Authentication method.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    /// No authentication.
    #[default]
    None,

    /// Token-based authentication.
    Token,

    /// API key authentication.
    ApiKey,
}

impl FromStr for AuthMethod {
    type Err = ShikiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(AuthMethod::None),
            "token" => Ok(AuthMethod::Token),
            "apikey" | "api_key" => Ok(AuthMethod::ApiKey),
            _ => Err(ShikiError::config(format!("Unknown auth method: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.bind, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert!(!config.tls.enabled);
    }

    #[test]
    fn test_auth_method_parse() {
        assert_eq!("none".parse::<AuthMethod>().unwrap(), AuthMethod::None);
        assert_eq!("token".parse::<AuthMethod>().unwrap(), AuthMethod::Token);
        assert_eq!("apikey".parse::<AuthMethod>().unwrap(), AuthMethod::ApiKey);
        assert!("invalid".parse::<AuthMethod>().is_err());
    }
}
