//! Error types and error handling for shiki.
//!
//! This module defines all error types used throughout the application,
//! including error codes, error responses for the API, and CLI exit codes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// Error codes as defined in the specification.
/// Each error has a unique code for identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    /// E001: Configuration file is invalid
    #[serde(rename = "E001")]
    ConfigInvalid,

    /// E002: Target service does not exist
    #[serde(rename = "E002")]
    ServiceNotFound,

    /// E003: Service operation is not permitted
    #[serde(rename = "E003")]
    ServiceDenied,

    /// E004: Backend operation failed
    #[serde(rename = "E004")]
    BackendError,

    /// E005: Operation timed out
    #[serde(rename = "E005")]
    Timeout,

    /// E006: Failed to connect to remote agent
    #[serde(rename = "E006")]
    ConnectionError,

    /// E007: Authentication failed
    #[serde(rename = "E007")]
    AuthFailed,

    /// E008: Request is invalid
    #[serde(rename = "E008")]
    InvalidRequest,

    /// E009: Agent is busy
    #[serde(rename = "E009")]
    AgentBusy,
}

impl ErrorCode {
    /// Returns the error code as a string (e.g., "E001").
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::ConfigInvalid => "E001",
            ErrorCode::ServiceNotFound => "E002",
            ErrorCode::ServiceDenied => "E003",
            ErrorCode::BackendError => "E004",
            ErrorCode::Timeout => "E005",
            ErrorCode::ConnectionError => "E006",
            ErrorCode::AuthFailed => "E007",
            ErrorCode::InvalidRequest => "E008",
            ErrorCode::AgentBusy => "E009",
        }
    }

    /// Returns the default message for this error code.
    pub fn default_message(&self) -> &'static str {
        match self {
            ErrorCode::ConfigInvalid => "Configuration file is invalid",
            ErrorCode::ServiceNotFound => "Service not found",
            ErrorCode::ServiceDenied => "Service operation is not permitted",
            ErrorCode::BackendError => "Backend operation failed",
            ErrorCode::Timeout => "Operation timed out",
            ErrorCode::ConnectionError => "Failed to connect to remote agent",
            ErrorCode::AuthFailed => "Authentication failed",
            ErrorCode::InvalidRequest => "Request is invalid",
            ErrorCode::AgentBusy => "Agent is busy",
        }
    }

    /// Returns the HTTP status code for this error.
    pub fn http_status(&self) -> u16 {
        match self {
            ErrorCode::ConfigInvalid => 500,
            ErrorCode::ServiceNotFound => 404,
            ErrorCode::ServiceDenied => 403,
            ErrorCode::BackendError => 500,
            ErrorCode::Timeout => 504,
            ErrorCode::ConnectionError => 502,
            ErrorCode::AuthFailed => 401,
            ErrorCode::InvalidRequest => 400,
            ErrorCode::AgentBusy => 503,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// CLI exit codes as defined in the specification.
pub mod exit_code {
    /// Success
    pub const SUCCESS: i32 = 0;
    /// General error
    pub const GENERAL_ERROR: i32 = 1;
    /// Configuration error
    pub const CONFIG_ERROR: i32 = 2;
    /// Connection error
    pub const CONNECTION_ERROR: i32 = 3;
    /// Timeout error
    pub const TIMEOUT_ERROR: i32 = 4;
    /// Authentication error
    pub const AUTH_ERROR: i32 = 5;
    /// Command line argument error
    pub const CLI_ERROR: i32 = 64;
}

/// The main error type for shiki.
#[derive(Debug, Error)]
pub enum ShikiError {
    /// Configuration file is invalid or cannot be loaded.
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Target service does not exist.
    #[error("Service not found: {service}")]
    ServiceNotFound { service: String },

    /// Service operation is not permitted by ACL.
    #[error("Service operation denied: {service}")]
    ServiceDenied { service: String, reason: String },

    /// Backend operation failed.
    #[error("Backend error: {message}")]
    Backend {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Operation timed out.
    #[error("Timeout: {operation} (waited {seconds}s)")]
    Timeout { operation: String, seconds: u64 },

    /// Failed to connect to remote agent.
    #[error("Connection error: {target}")]
    Connection {
        target: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Authentication failed.
    #[error("Authentication failed: {reason}")]
    AuthFailed { reason: String },

    /// Request is invalid.
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    /// Agent is busy.
    #[error("Agent is busy: {reason}")]
    AgentBusy { reason: String },

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error.
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// JSON parsing error.
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ShikiError {
    /// Returns the error code for this error.
    pub fn code(&self) -> ErrorCode {
        match self {
            ShikiError::Config { .. } => ErrorCode::ConfigInvalid,
            ShikiError::ServiceNotFound { .. } => ErrorCode::ServiceNotFound,
            ShikiError::ServiceDenied { .. } => ErrorCode::ServiceDenied,
            ShikiError::Backend { .. } => ErrorCode::BackendError,
            ShikiError::Timeout { .. } => ErrorCode::Timeout,
            ShikiError::Connection { .. } => ErrorCode::ConnectionError,
            ShikiError::AuthFailed { .. } => ErrorCode::AuthFailed,
            ShikiError::InvalidRequest { .. } => ErrorCode::InvalidRequest,
            ShikiError::AgentBusy { .. } => ErrorCode::AgentBusy,
            ShikiError::Io(_) => ErrorCode::BackendError,
            ShikiError::Yaml(_) => ErrorCode::ConfigInvalid,
            ShikiError::Json(_) => ErrorCode::InvalidRequest,
        }
    }

    /// Returns the CLI exit code for this error.
    pub fn exit_code(&self) -> i32 {
        match self {
            ShikiError::Config { .. } | ShikiError::Yaml(_) => exit_code::CONFIG_ERROR,
            ShikiError::Connection { .. } => exit_code::CONNECTION_ERROR,
            ShikiError::Timeout { .. } => exit_code::TIMEOUT_ERROR,
            ShikiError::AuthFailed { .. } => exit_code::AUTH_ERROR,
            _ => exit_code::GENERAL_ERROR,
        }
    }

    /// Creates a configuration error with a message.
    pub fn config(message: impl Into<String>) -> Self {
        ShikiError::Config {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a configuration error with a message and source.
    pub fn config_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ShikiError::Config {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates a backend error with a message.
    pub fn backend(message: impl Into<String>) -> Self {
        ShikiError::Backend {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a backend error with a message and source.
    pub fn backend_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ShikiError::Backend {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates a connection error.
    pub fn connection(target: impl Into<String>) -> Self {
        ShikiError::Connection {
            target: target.into(),
            source: None,
        }
    }

    /// Creates a connection error with a source.
    pub fn connection_with_source(
        target: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        ShikiError::Connection {
            target: target.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates an invalid request error.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        ShikiError::InvalidRequest {
            message: message.into(),
        }
    }
}

/// Error details for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Additional context fields.
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

impl ErrorDetails {
    /// Creates empty error details.
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Adds a field to the error details.
    pub fn with_field(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
}

impl Default for ErrorDetails {
    fn default() -> Self {
        Self::new()
    }
}

/// Error response structure for the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code (e.g., "E001").
    pub code: ErrorCode,

    /// Human-readable error message.
    pub message: String,

    /// Additional error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorDetails>,
}

impl ErrorResponse {
    /// Creates a new error response.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Creates an error response with details.
    pub fn with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: ErrorDetails,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details),
        }
    }

    /// Creates an error response from a ShikiError.
    pub fn from_error(error: &ShikiError) -> Self {
        let code = error.code();
        let message = error.to_string();

        let details = match error {
            ShikiError::ServiceNotFound { service } => Some(
                ErrorDetails::new()
                    .with_field("service", service.clone())
                    .with_field(
                        "suggestion",
                        "Check if the service is installed and the name is correct",
                    ),
            ),
            ShikiError::ServiceDenied { service, reason } => Some(
                ErrorDetails::new()
                    .with_field("service", service.clone())
                    .with_field("reason", reason.clone()),
            ),
            ShikiError::Timeout { operation, seconds } => Some(
                ErrorDetails::new()
                    .with_field("operation", operation.clone())
                    .with_field("timeout_seconds", *seconds),
            ),
            ShikiError::Connection { target, .. } => {
                Some(ErrorDetails::new().with_field("target", target.clone()))
            }
            _ => None,
        };

        Self {
            code,
            message,
            details,
        }
    }
}

/// Result type alias for shiki operations.
pub type Result<T> = std::result::Result<T, ShikiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_as_str() {
        assert_eq!(ErrorCode::ConfigInvalid.as_str(), "E001");
        assert_eq!(ErrorCode::ServiceNotFound.as_str(), "E002");
        assert_eq!(ErrorCode::ServiceDenied.as_str(), "E003");
        assert_eq!(ErrorCode::BackendError.as_str(), "E004");
        assert_eq!(ErrorCode::Timeout.as_str(), "E005");
        assert_eq!(ErrorCode::ConnectionError.as_str(), "E006");
        assert_eq!(ErrorCode::AuthFailed.as_str(), "E007");
        assert_eq!(ErrorCode::InvalidRequest.as_str(), "E008");
        assert_eq!(ErrorCode::AgentBusy.as_str(), "E009");
    }

    #[test]
    fn test_error_code_http_status() {
        assert_eq!(ErrorCode::ConfigInvalid.http_status(), 500);
        assert_eq!(ErrorCode::ServiceNotFound.http_status(), 404);
        assert_eq!(ErrorCode::ServiceDenied.http_status(), 403);
        assert_eq!(ErrorCode::BackendError.http_status(), 500);
        assert_eq!(ErrorCode::Timeout.http_status(), 504);
        assert_eq!(ErrorCode::ConnectionError.http_status(), 502);
        assert_eq!(ErrorCode::AuthFailed.http_status(), 401);
        assert_eq!(ErrorCode::InvalidRequest.http_status(), 400);
        assert_eq!(ErrorCode::AgentBusy.http_status(), 503);
    }

    #[test]
    fn test_shiki_error_code() {
        let err = ShikiError::ServiceNotFound {
            service: "nginx".to_string(),
        };
        assert_eq!(err.code(), ErrorCode::ServiceNotFound);

        let err = ShikiError::config("invalid yaml");
        assert_eq!(err.code(), ErrorCode::ConfigInvalid);

        let err = ShikiError::Timeout {
            operation: "start".to_string(),
            seconds: 60,
        };
        assert_eq!(err.code(), ErrorCode::Timeout);
    }

    #[test]
    fn test_shiki_error_exit_code() {
        let err = ShikiError::config("invalid yaml");
        assert_eq!(err.exit_code(), exit_code::CONFIG_ERROR);

        let err = ShikiError::connection("localhost:8080");
        assert_eq!(err.exit_code(), exit_code::CONNECTION_ERROR);

        let err = ShikiError::Timeout {
            operation: "wait".to_string(),
            seconds: 30,
        };
        assert_eq!(err.exit_code(), exit_code::TIMEOUT_ERROR);

        let err = ShikiError::AuthFailed {
            reason: "invalid token".to_string(),
        };
        assert_eq!(err.exit_code(), exit_code::AUTH_ERROR);
    }

    #[test]
    fn test_error_response_from_error() {
        let err = ShikiError::ServiceNotFound {
            service: "nginx".to_string(),
        };
        let response = ErrorResponse::from_error(&err);

        assert_eq!(response.code, ErrorCode::ServiceNotFound);
        assert!(response.message.contains("nginx"));
        assert!(response.details.is_some());

        let details = response.details.unwrap();
        assert_eq!(
            details.fields.get("service"),
            Some(&serde_json::Value::String("nginx".to_string()))
        );
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse::new(ErrorCode::ServiceNotFound, "Service not found: nginx");
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"code\":\"E002\""));
        assert!(json.contains("Service not found: nginx"));
    }

    #[test]
    fn test_error_details_builder() {
        let details = ErrorDetails::new()
            .with_field("service", "nginx")
            .with_field("timeout", 60);

        assert_eq!(
            details.fields.get("service"),
            Some(&serde_json::Value::String("nginx".to_string()))
        );
        assert_eq!(
            details.fields.get("timeout"),
            Some(&serde_json::Value::Number(60.into()))
        );
    }

    #[test]
    fn test_error_display() {
        let err = ShikiError::ServiceNotFound {
            service: "nginx".to_string(),
        };
        assert_eq!(format!("{}", err), "Service not found: nginx");

        let err = ShikiError::Timeout {
            operation: "start service".to_string(),
            seconds: 60,
        };
        assert_eq!(format!("{}", err), "Timeout: start service (waited 60s)");
    }
}
