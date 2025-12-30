//! API response types and formatting.
//!
//! This module defines the standard API response format used by all endpoints.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ErrorResponse, ShikiError};

/// Standard API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Whether the request was successful.
    pub success: bool,
    /// Response data (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error information (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorResponse>,
    /// Response timestamp.
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    /// Creates a successful response with data.
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    /// Creates a failed response with an error.
    pub fn error(error: ErrorResponse) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error),
            timestamp: Utc::now(),
        }
    }

    /// Creates a failed response from a ShikiError.
    pub fn from_error(err: &ShikiError) -> ApiResponse<T> {
        Self::error(ErrorResponse::from_error(err))
    }
}

/// Health check response data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthData {
    /// Health status.
    pub status: HealthStatus,
    /// Application version.
    pub version: String,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
}

/// Health status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// System is healthy.
    Healthy,
    /// System is degraded but operational.
    Degraded,
    /// System is unhealthy.
    Unhealthy,
}

/// Agent status response data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusData {
    /// Agent information.
    pub agent: AgentInfo,
    /// Server information.
    pub server: ServerInfo,
    /// Statistics.
    pub stats: StatsInfo,
    /// Application version.
    pub version: String,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
}

/// Agent information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent name.
    pub name: String,
    /// Agent state.
    pub state: AgentState,
    /// Operation mode.
    pub mode: String,
    /// Agent tags.
    pub tags: Vec<String>,
}

/// Agent state enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentState {
    /// Agent is ready to accept requests.
    Ready,
    /// Agent is starting up.
    Starting,
    /// Agent is shutting down.
    ShuttingDown,
    /// Agent is in error state.
    Error,
}

/// Server information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Bind address.
    pub bind: String,
    /// Port number.
    pub port: u16,
    /// Whether TLS is enabled.
    pub tls_enabled: bool,
}

/// Statistics information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsInfo {
    /// Total requests received.
    pub requests_total: u64,
    /// Successful requests.
    pub requests_success: u64,
    /// Failed requests.
    pub requests_failed: u64,
    /// Active connections.
    pub active_connections: u64,
}

/// Notify request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyRequest {
    /// Action to perform.
    pub action: String,
    /// Target service name.
    pub service: String,
    /// Optional configuration.
    #[serde(default)]
    pub options: NotifyOptions,
}

/// Notify request options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyOptions {
    /// Whether to wait for completion.
    #[serde(default = "default_wait")]
    pub wait: bool,
    /// Timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_wait() -> bool {
    true
}

fn default_timeout() -> u64 {
    60
}

impl Default for NotifyOptions {
    fn default() -> Self {
        Self {
            wait: default_wait(),
            timeout_seconds: default_timeout(),
        }
    }
}

/// Notify response data for synchronous (wait=true) requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyResponseData {
    /// Request ID.
    pub request_id: Uuid,
    /// Service name.
    pub service: String,
    /// Action performed.
    pub action: String,
    /// Result status.
    pub result: String,
    /// Previous service status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_status: Option<String>,
    /// Current service status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_status: Option<String>,
    /// Duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Message (for accepted/error responses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Service list response data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesListData {
    /// List of services.
    pub services: Vec<ServiceInfo>,
    /// Total number of services.
    pub total: usize,
    /// Limit used in query.
    pub limit: usize,
    /// Offset used in query.
    pub offset: usize,
}

/// Basic service information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name.
    pub name: String,
    /// Service status.
    pub status: String,
    /// Service description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Service detail response data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDetailData {
    /// Service name.
    pub name: String,
    /// Service status.
    pub status: String,
    /// Service description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Service operation response data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceOperationData {
    /// Service name.
    pub service: String,
    /// Action performed.
    pub action: String,
    /// Whether the operation was successful.
    pub success: bool,
    /// Previous state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_state: Option<String>,
    /// Current state.
    pub current_state: String,
    /// Operation message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response: ApiResponse<String> = ApiResponse::success("test data".to_string());

        assert!(response.success);
        assert_eq!(response.data, Some("test data".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        use crate::error::ErrorCode;
        let error = ErrorResponse {
            code: ErrorCode::ConfigInvalid,
            message: "Test error".to_string(),
            details: None,
        };
        let response: ApiResponse<String> = ApiResponse::error(error.clone());

        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error.unwrap().code, ErrorCode::ConfigInvalid);
    }

    #[test]
    fn test_health_status_serialization() {
        let status = HealthStatus::Healthy;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"healthy\"");

        let status = HealthStatus::Degraded;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"degraded\"");
    }

    #[test]
    fn test_notify_options_default() {
        let options = NotifyOptions::default();
        assert!(options.wait);
        assert_eq!(options.timeout_seconds, 60);
    }

    #[test]
    fn test_notify_request_deserialization() {
        let json = r#"{"action": "start", "service": "nginx"}"#;
        let request: NotifyRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.action, "start");
        assert_eq!(request.service, "nginx");
        assert!(request.options.wait);
        assert_eq!(request.options.timeout_seconds, 60);
    }

    #[test]
    fn test_notify_request_with_options() {
        let json = r#"{
            "action": "start",
            "service": "nginx",
            "options": {
                "wait": false,
                "timeout_seconds": 30
            }
        }"#;
        let request: NotifyRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.action, "start");
        assert_eq!(request.service, "nginx");
        assert!(!request.options.wait);
        assert_eq!(request.options.timeout_seconds, 30);
    }

    #[test]
    fn test_service_info_serialization() {
        let info = ServiceInfo {
            name: "nginx".to_string(),
            status: "running".to_string(),
            description: Some("Web server".to_string()),
        };

        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["name"], "nginx");
        assert_eq!(json["status"], "running");
        assert_eq!(json["description"], "Web server");
    }

    #[test]
    fn test_agent_state_serialization() {
        assert_eq!(
            serde_json::to_string(&AgentState::Ready).unwrap(),
            "\"ready\""
        );
        assert_eq!(
            serde_json::to_string(&AgentState::Starting).unwrap(),
            "\"starting\""
        );
        assert_eq!(
            serde_json::to_string(&AgentState::ShuttingDown).unwrap(),
            "\"shuttingdown\""
        );
        assert_eq!(
            serde_json::to_string(&AgentState::Error).unwrap(),
            "\"error\""
        );
    }
}
