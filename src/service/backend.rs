//! Service backend trait and common types.
//!
//! This module defines the `ServiceBackend` trait that all service backends
//! (systemd, exec) must implement, along with common types for service operations.

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Service state as reported by the backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    /// Service is running.
    Running,
    /// Service is stopped.
    Stopped,
    /// Service has failed.
    Failed,
    /// Service state is unknown.
    Unknown,
}

impl std::fmt::Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceState::Running => write!(f, "running"),
            ServiceState::Stopped => write!(f, "stopped"),
            ServiceState::Failed => write!(f, "failed"),
            ServiceState::Unknown => write!(f, "unknown"),
        }
    }
}

/// Service action to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceAction {
    /// Start the service.
    Start,
    /// Stop the service.
    Stop,
    /// Restart the service.
    Restart,
}

impl std::fmt::Display for ServiceAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceAction::Start => write!(f, "start"),
            ServiceAction::Stop => write!(f, "stop"),
            ServiceAction::Restart => write!(f, "restart"),
        }
    }
}

impl std::str::FromStr for ServiceAction {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "start" => Ok(ServiceAction::Start),
            "stop" => Ok(ServiceAction::Stop),
            "restart" => Ok(ServiceAction::Restart),
            _ => Err(format!("Invalid service action: {}", s)),
        }
    }
}

/// Result of a service operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceOperationResult {
    /// The service name.
    pub service: String,
    /// The action performed.
    pub action: ServiceAction,
    /// Whether the operation succeeded.
    pub success: bool,
    /// The resulting state of the service.
    pub state: ServiceState,
    /// Optional message (e.g., error details).
    pub message: Option<String>,
}

impl ServiceOperationResult {
    /// Creates a successful operation result.
    pub fn success(service: impl Into<String>, action: ServiceAction, state: ServiceState) -> Self {
        Self {
            service: service.into(),
            action,
            success: true,
            state,
            message: None,
        }
    }

    /// Creates a failed operation result.
    pub fn failure(
        service: impl Into<String>,
        action: ServiceAction,
        state: ServiceState,
        message: impl Into<String>,
    ) -> Self {
        Self {
            service: service.into(),
            action,
            success: false,
            state,
            message: Some(message.into()),
        }
    }
}

/// Service status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    /// The service name.
    pub name: String,
    /// Current state.
    pub state: ServiceState,
    /// Optional description or message.
    pub description: Option<String>,
}

impl ServiceStatus {
    /// Creates a new service status.
    pub fn new(name: impl Into<String>, state: ServiceState) -> Self {
        Self {
            name: name.into(),
            state,
            description: None,
        }
    }

    /// Creates a service status with a description.
    pub fn with_description(
        name: impl Into<String>,
        state: ServiceState,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            state,
            description: Some(description.into()),
        }
    }
}

/// Trait for service backends.
///
/// This trait defines the interface that all service backends must implement.
/// Backends are responsible for starting, stopping, and checking the status
/// of services on the local system.
#[async_trait]
pub trait ServiceBackend: Send + Sync {
    /// Returns the name of this backend.
    fn name(&self) -> &'static str;

    /// Checks if the backend supports the given service.
    fn supports_service(&self, service: &str) -> bool;

    /// Gets the list of available services.
    async fn list_services(&self) -> Result<Vec<String>>;

    /// Gets the status of a service.
    async fn status(&self, service: &str) -> Result<ServiceStatus>;

    /// Starts a service.
    async fn start(&self, service: &str) -> Result<ServiceOperationResult>;

    /// Stops a service.
    async fn stop(&self, service: &str) -> Result<ServiceOperationResult>;

    /// Restarts a service.
    async fn restart(&self, service: &str) -> Result<ServiceOperationResult>;

    /// Performs an action on a service.
    async fn perform_action(
        &self,
        service: &str,
        action: ServiceAction,
    ) -> Result<ServiceOperationResult> {
        match action {
            ServiceAction::Start => self.start(service).await,
            ServiceAction::Stop => self.stop(service).await,
            ServiceAction::Restart => self.restart(service).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_state_display() {
        assert_eq!(format!("{}", ServiceState::Running), "running");
        assert_eq!(format!("{}", ServiceState::Stopped), "stopped");
        assert_eq!(format!("{}", ServiceState::Failed), "failed");
        assert_eq!(format!("{}", ServiceState::Unknown), "unknown");
    }

    #[test]
    fn test_service_action_display() {
        assert_eq!(format!("{}", ServiceAction::Start), "start");
        assert_eq!(format!("{}", ServiceAction::Stop), "stop");
        assert_eq!(format!("{}", ServiceAction::Restart), "restart");
    }

    #[test]
    fn test_service_action_parse() {
        assert_eq!(
            "start".parse::<ServiceAction>().unwrap(),
            ServiceAction::Start
        );
        assert_eq!(
            "STOP".parse::<ServiceAction>().unwrap(),
            ServiceAction::Stop
        );
        assert_eq!(
            "Restart".parse::<ServiceAction>().unwrap(),
            ServiceAction::Restart
        );
        assert!("invalid".parse::<ServiceAction>().is_err());
    }

    #[test]
    fn test_service_operation_result_success() {
        let result =
            ServiceOperationResult::success("nginx", ServiceAction::Start, ServiceState::Running);

        assert_eq!(result.service, "nginx");
        assert_eq!(result.action, ServiceAction::Start);
        assert!(result.success);
        assert_eq!(result.state, ServiceState::Running);
        assert!(result.message.is_none());
    }

    #[test]
    fn test_service_operation_result_failure() {
        let result = ServiceOperationResult::failure(
            "nginx",
            ServiceAction::Start,
            ServiceState::Failed,
            "Permission denied",
        );

        assert_eq!(result.service, "nginx");
        assert_eq!(result.action, ServiceAction::Start);
        assert!(!result.success);
        assert_eq!(result.state, ServiceState::Failed);
        assert_eq!(result.message, Some("Permission denied".to_string()));
    }

    #[test]
    fn test_service_status() {
        let status = ServiceStatus::new("nginx", ServiceState::Running);
        assert_eq!(status.name, "nginx");
        assert_eq!(status.state, ServiceState::Running);
        assert!(status.description.is_none());

        let status =
            ServiceStatus::with_description("nginx", ServiceState::Running, "Active and running");
        assert_eq!(status.description, Some("Active and running".to_string()));
    }

    #[test]
    fn test_service_state_serialization() {
        let state = ServiceState::Running;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"running\"");

        let deserialized: ServiceState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ServiceState::Running);
    }

    #[test]
    fn test_service_action_serialization() {
        let action = ServiceAction::Start;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, "\"start\"");

        let deserialized: ServiceAction = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ServiceAction::Start);
    }
}
