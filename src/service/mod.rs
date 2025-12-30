//! Service module - Service management and backends.
//!
//! This module provides the service management layer for shiki,
//! including the backend trait and implementations for systemd and exec backends.

pub mod backend;
pub mod exec;
pub mod systemd;

#[cfg(test)]
mod exec_tests;

use crate::config::{Backend, Config};
use crate::error::{Result, ShikiError};
use exec::ExecBackend;
use std::sync::Arc;
use systemd::SystemdBackend;

// Re-exports for convenience
pub use backend::{
    ServiceAction, ServiceBackend, ServiceOperationResult, ServiceState, ServiceStatus,
};

/// Service controller that manages service operations.
///
/// The controller is responsible for routing service operations to the
/// appropriate backend based on configuration.
pub struct ServiceController {
    /// The active backend.
    backend: Arc<dyn ServiceBackend>,
    /// Backend type name.
    backend_type: Backend,
}

impl ServiceController {
    /// Creates a new service controller from configuration.
    pub fn from_config(config: &Config) -> Result<Self> {
        let backend: Arc<dyn ServiceBackend> = match config.agent.backend {
            Backend::Systemd => Arc::new(SystemdBackend::new(config.acl.clone())),
            Backend::Exec => {
                if config.services.is_empty() {
                    return Err(ShikiError::config(
                        "Exec backend requires at least one service definition",
                    ));
                }
                Arc::new(ExecBackend::new(config.services.clone()))
            }
        };

        Ok(Self {
            backend,
            backend_type: config.agent.backend,
        })
    }

    /// Returns the name of the active backend.
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    /// Returns the backend type.
    pub fn backend_type(&self) -> Backend {
        self.backend_type
    }

    /// Checks if a service is supported by the backend.
    pub fn supports_service(&self, service: &str) -> bool {
        self.backend.supports_service(service)
    }

    /// Lists all available services.
    pub async fn list_services(&self) -> Result<Vec<String>> {
        self.backend.list_services().await
    }

    /// Gets the status of a service.
    pub async fn status(&self, service: &str) -> Result<ServiceStatus> {
        self.backend.status(service).await
    }

    /// Starts a service.
    pub async fn start(&self, service: &str) -> Result<ServiceOperationResult> {
        self.backend.start(service).await
    }

    /// Stops a service.
    pub async fn stop(&self, service: &str) -> Result<ServiceOperationResult> {
        self.backend.stop(service).await
    }

    /// Restarts a service.
    pub async fn restart(&self, service: &str) -> Result<ServiceOperationResult> {
        self.backend.restart(service).await
    }

    /// Performs an action on a service.
    pub async fn perform_action(
        &self,
        service: &str,
        action: ServiceAction,
    ) -> Result<ServiceOperationResult> {
        self.backend.perform_action(service, action).await
    }
}

/// Creates a service backend from configuration.
///
/// This is a helper function for creating backends dynamically.
pub fn create_backend(config: &Config) -> Result<Arc<dyn ServiceBackend>> {
    match config.agent.backend {
        Backend::Systemd => Ok(Arc::new(SystemdBackend::new(config.acl.clone()))),
        Backend::Exec => {
            if config.services.is_empty() {
                return Err(ShikiError::config(
                    "Exec backend requires at least one service definition",
                ));
            }
            Ok(Arc::new(ExecBackend::new(config.services.clone())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServiceDefinition;
    use std::collections::HashMap;

    fn create_exec_config() -> Config {
        let mut config = Config::default();
        config.agent.backend = Backend::Exec;

        let mut services = HashMap::new();
        services.insert(
            "test-service".to_string(),
            ServiceDefinition {
                start: "echo starting".to_string(),
                stop: "echo stopping".to_string(),
                status: "true".to_string(),
                ..Default::default()
            },
        );
        config.services = services;

        config
    }

    fn create_systemd_config() -> Config {
        let mut config = Config::default();
        config.agent.backend = Backend::Systemd;
        config
    }

    #[test]
    fn test_service_controller_from_exec_config() {
        let config = create_exec_config();
        let controller = ServiceController::from_config(&config).unwrap();

        assert_eq!(controller.backend_name(), "exec");
        assert_eq!(controller.backend_type(), Backend::Exec);
        assert!(controller.supports_service("test-service"));
        assert!(!controller.supports_service("nonexistent"));
    }

    #[test]
    fn test_service_controller_from_systemd_config() {
        let config = create_systemd_config();
        let controller = ServiceController::from_config(&config).unwrap();

        assert_eq!(controller.backend_name(), "systemd");
        assert_eq!(controller.backend_type(), Backend::Systemd);
    }

    #[test]
    fn test_exec_backend_requires_services() {
        let mut config = Config::default();
        config.agent.backend = Backend::Exec;
        // No services defined

        let result = ServiceController::from_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_backend_exec() {
        let config = create_exec_config();
        let backend = create_backend(&config).unwrap();

        assert_eq!(backend.name(), "exec");
    }

    #[test]
    fn test_create_backend_systemd() {
        let config = create_systemd_config();
        let backend = create_backend(&config).unwrap();

        assert_eq!(backend.name(), "systemd");
    }

    #[tokio::test]
    async fn test_service_controller_operations() {
        let config = create_exec_config();
        let controller = ServiceController::from_config(&config).unwrap();

        // List services
        let services = controller.list_services().await.unwrap();
        assert!(services.contains(&"test-service".to_string()));

        // Get status
        let status = controller.status("test-service").await.unwrap();
        assert_eq!(status.name, "test-service");
        assert_eq!(status.state, ServiceState::Running);

        // Start (already running)
        let result = controller.start("test-service").await.unwrap();
        assert!(result.success);

        // Perform action
        let result = controller
            .perform_action("test-service", ServiceAction::Start)
            .await
            .unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_service_controller_not_found() {
        let config = create_exec_config();
        let controller = ServiceController::from_config(&config).unwrap();

        let result = controller.status("nonexistent").await;
        assert!(result.is_err());
    }
}
