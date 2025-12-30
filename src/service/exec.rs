//! Exec backend implementation.
//!
//! This backend executes user-defined commands for service operations.
//! It's designed for environments where systemd is not available,
//! such as Docker containers.

use crate::config::ServiceDefinition;
use crate::error::{Result, ShikiError};
use crate::service::backend::{
    ServiceAction, ServiceBackend, ServiceOperationResult, ServiceState, ServiceStatus,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Exec backend for service operations.
///
/// This backend executes user-defined commands to manage services.
/// Each service must have start, stop, and status commands defined
/// in the configuration file.
pub struct ExecBackend {
    /// Service definitions from configuration.
    services: HashMap<String, ServiceDefinition>,
}

impl ExecBackend {
    /// Creates a new exec backend with the given service definitions.
    pub fn new(services: HashMap<String, ServiceDefinition>) -> Self {
        Self { services }
    }

    /// Gets the service definition for a service.
    fn get_service(&self, name: &str) -> Result<&ServiceDefinition> {
        self.services
            .get(name)
            .ok_or_else(|| ShikiError::ServiceNotFound {
                service: name.to_string(),
            })
    }

    /// Executes a command and returns the exit status and output.
    async fn execute_command(
        &self,
        command: &str,
        service_name: &str,
        definition: &ServiceDefinition,
    ) -> Result<(bool, String)> {
        debug!(
            service = service_name,
            command = command,
            "Executing command"
        );

        // Parse command into program and arguments
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ShikiError::backend("Empty command"));
        }

        let program = parts[0];
        let args = &parts[1..];

        // Build the command
        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set working directory if specified
        if let Some(working_dir) = &definition.working_dir {
            cmd.current_dir(working_dir);
        }

        // Set environment variables
        for env_var in &definition.env {
            if let Some((key, value)) = env_var.split_once('=') {
                cmd.env(key, value);
            } else {
                warn!(
                    service = service_name,
                    env_var = env_var,
                    "Invalid environment variable format, expected KEY=VALUE"
                );
            }
        }

        // Execute the command
        let output = cmd.output().await.map_err(|e| {
            ShikiError::backend_with_source(
                format!("Failed to execute command '{}': {}", command, e),
                e,
            )
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined_output = if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        debug!(
            service = service_name,
            exit_code = output.status.code(),
            stdout = %stdout,
            stderr = %stderr,
            "Command completed"
        );

        Ok((output.status.success(), combined_output))
    }

    /// Gets the current state of a service by running its status command.
    async fn get_service_state(
        &self,
        service_name: &str,
        definition: &ServiceDefinition,
    ) -> Result<ServiceState> {
        let (success, _output) = self
            .execute_command(&definition.status, service_name, definition)
            .await?;

        // Exit code 0 means running, anything else means stopped
        if success {
            Ok(ServiceState::Running)
        } else {
            Ok(ServiceState::Stopped)
        }
    }
}

#[async_trait]
impl ServiceBackend for ExecBackend {
    fn name(&self) -> &'static str {
        "exec"
    }

    fn supports_service(&self, service: &str) -> bool {
        self.services.contains_key(service)
    }

    async fn list_services(&self) -> Result<Vec<String>> {
        let services: Vec<String> = self.services.keys().cloned().collect();
        Ok(services)
    }

    async fn status(&self, service: &str) -> Result<ServiceStatus> {
        let definition = self.get_service(service)?;
        let state = self.get_service_state(service, definition).await?;

        Ok(ServiceStatus::new(service, state))
    }

    async fn start(&self, service: &str) -> Result<ServiceOperationResult> {
        let definition = self.get_service(service)?;

        info!(service = service, "Starting service");

        // Check current state
        let current_state = self.get_service_state(service, definition).await?;
        if current_state == ServiceState::Running {
            info!(service = service, "Service is already running");
            return Ok(ServiceOperationResult::success(
                service,
                ServiceAction::Start,
                ServiceState::Running,
            ));
        }

        // Execute start command
        let (success, output) = self
            .execute_command(&definition.start, service, definition)
            .await?;

        if !success {
            error!(
                service = service,
                output = %output,
                "Failed to start service"
            );
            return Ok(ServiceOperationResult::failure(
                service,
                ServiceAction::Start,
                ServiceState::Failed,
                output,
            ));
        }

        // Verify the service started
        let new_state = self.get_service_state(service, definition).await?;

        if new_state == ServiceState::Running {
            info!(service = service, "Service started successfully");
            Ok(ServiceOperationResult::success(
                service,
                ServiceAction::Start,
                ServiceState::Running,
            ))
        } else {
            warn!(
                service = service,
                state = %new_state,
                "Service did not reach running state after start"
            );
            Ok(ServiceOperationResult::failure(
                service,
                ServiceAction::Start,
                new_state,
                "Service did not start properly",
            ))
        }
    }

    async fn stop(&self, service: &str) -> Result<ServiceOperationResult> {
        let definition = self.get_service(service)?;

        info!(service = service, "Stopping service");

        // Check current state
        let current_state = self.get_service_state(service, definition).await?;
        if current_state == ServiceState::Stopped {
            info!(service = service, "Service is already stopped");
            return Ok(ServiceOperationResult::success(
                service,
                ServiceAction::Stop,
                ServiceState::Stopped,
            ));
        }

        // Execute stop command
        let (success, output) = self
            .execute_command(&definition.stop, service, definition)
            .await?;

        if !success {
            error!(
                service = service,
                output = %output,
                "Failed to stop service"
            );
            return Ok(ServiceOperationResult::failure(
                service,
                ServiceAction::Stop,
                ServiceState::Failed,
                output,
            ));
        }

        // Verify the service stopped
        let new_state = self.get_service_state(service, definition).await?;

        if new_state == ServiceState::Stopped {
            info!(service = service, "Service stopped successfully");
            Ok(ServiceOperationResult::success(
                service,
                ServiceAction::Stop,
                ServiceState::Stopped,
            ))
        } else {
            warn!(
                service = service,
                state = %new_state,
                "Service did not reach stopped state after stop"
            );
            Ok(ServiceOperationResult::failure(
                service,
                ServiceAction::Stop,
                new_state,
                "Service did not stop properly",
            ))
        }
    }

    async fn restart(&self, service: &str) -> Result<ServiceOperationResult> {
        let definition = self.get_service(service)?;

        info!(service = service, "Restarting service");

        // If restart command is defined, use it
        if let Some(restart_cmd) = &definition.restart {
            let (success, output) = self
                .execute_command(restart_cmd, service, definition)
                .await?;

            if !success {
                error!(
                    service = service,
                    output = %output,
                    "Failed to restart service"
                );
                return Ok(ServiceOperationResult::failure(
                    service,
                    ServiceAction::Restart,
                    ServiceState::Failed,
                    output,
                ));
            }

            // Verify the service is running
            let new_state = self.get_service_state(service, definition).await?;

            if new_state == ServiceState::Running {
                info!(service = service, "Service restarted successfully");
                Ok(ServiceOperationResult::success(
                    service,
                    ServiceAction::Restart,
                    ServiceState::Running,
                ))
            } else {
                warn!(
                    service = service,
                    state = %new_state,
                    "Service did not reach running state after restart"
                );
                Ok(ServiceOperationResult::failure(
                    service,
                    ServiceAction::Restart,
                    new_state,
                    "Service did not restart properly",
                ))
            }
        } else {
            // No restart command, do stop then start
            debug!(
                service = service,
                "No restart command defined, using stop+start"
            );

            // Stop the service (ignore if already stopped)
            let stop_result = self.stop(service).await?;
            if !stop_result.success && stop_result.state != ServiceState::Stopped {
                return Ok(ServiceOperationResult::failure(
                    service,
                    ServiceAction::Restart,
                    stop_result.state,
                    stop_result
                        .message
                        .unwrap_or_else(|| "Failed to stop service".to_string()),
                ));
            }

            // Start the service
            let start_result = self.start(service).await?;
            if start_result.success {
                Ok(ServiceOperationResult::success(
                    service,
                    ServiceAction::Restart,
                    ServiceState::Running,
                ))
            } else {
                Ok(ServiceOperationResult::failure(
                    service,
                    ServiceAction::Restart,
                    start_result.state,
                    start_result
                        .message
                        .unwrap_or_else(|| "Failed to start service".to_string()),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_services() -> HashMap<String, ServiceDefinition> {
        let mut services = HashMap::new();

        // A simple test service using echo and true/false commands
        services.insert(
            "test-service".to_string(),
            ServiceDefinition {
                start: "echo starting".to_string(),
                stop: "echo stopping".to_string(),
                status: "true".to_string(), // Always returns running
                restart: None,
                working_dir: None,
                env: vec![],
            },
        );

        services.insert(
            "stopped-service".to_string(),
            ServiceDefinition {
                start: "echo starting".to_string(),
                stop: "echo stopping".to_string(),
                status: "false".to_string(), // Always returns stopped
                restart: None,
                working_dir: None,
                env: vec![],
            },
        );

        services.insert(
            "service-with-env".to_string(),
            ServiceDefinition {
                start: "echo $TEST_VAR".to_string(),
                stop: "echo stopping".to_string(),
                status: "true".to_string(),
                restart: Some("echo restarting".to_string()),
                working_dir: Some("/tmp".to_string()),
                env: vec!["TEST_VAR=hello".to_string()],
            },
        );

        services
    }

    #[test]
    fn test_exec_backend_new() {
        let services = create_test_services();
        let backend = ExecBackend::new(services);

        assert_eq!(backend.name(), "exec");
    }

    #[test]
    fn test_supports_service() {
        let services = create_test_services();
        let backend = ExecBackend::new(services);

        assert!(backend.supports_service("test-service"));
        assert!(backend.supports_service("stopped-service"));
        assert!(!backend.supports_service("nonexistent"));
    }

    #[tokio::test]
    async fn test_list_services() {
        let services = create_test_services();
        let backend = ExecBackend::new(services);

        let list = backend.list_services().await.unwrap();
        assert_eq!(list.len(), 3);
        assert!(list.contains(&"test-service".to_string()));
        assert!(list.contains(&"stopped-service".to_string()));
        assert!(list.contains(&"service-with-env".to_string()));
    }

    #[tokio::test]
    async fn test_status_running() {
        let services = create_test_services();
        let backend = ExecBackend::new(services);

        let status = backend.status("test-service").await.unwrap();
        assert_eq!(status.name, "test-service");
        assert_eq!(status.state, ServiceState::Running);
    }

    #[tokio::test]
    async fn test_status_stopped() {
        let services = create_test_services();
        let backend = ExecBackend::new(services);

        let status = backend.status("stopped-service").await.unwrap();
        assert_eq!(status.name, "stopped-service");
        assert_eq!(status.state, ServiceState::Stopped);
    }

    #[tokio::test]
    async fn test_status_not_found() {
        let services = create_test_services();
        let backend = ExecBackend::new(services);

        let result = backend.status("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_already_running() {
        let services = create_test_services();
        let backend = ExecBackend::new(services);

        // test-service status returns true (running)
        let result = backend.start("test-service").await.unwrap();
        assert!(result.success);
        assert_eq!(result.action, ServiceAction::Start);
        assert_eq!(result.state, ServiceState::Running);
    }

    #[tokio::test]
    async fn test_stop_already_stopped() {
        let services = create_test_services();
        let backend = ExecBackend::new(services);

        // stopped-service status returns false (stopped)
        let result = backend.stop("stopped-service").await.unwrap();
        assert!(result.success);
        assert_eq!(result.action, ServiceAction::Stop);
        assert_eq!(result.state, ServiceState::Stopped);
    }

    #[tokio::test]
    async fn test_restart_with_restart_command() {
        let services = create_test_services();
        let backend = ExecBackend::new(services);

        // service-with-env has a restart command
        let result = backend.restart("service-with-env").await.unwrap();
        assert!(result.success);
        assert_eq!(result.action, ServiceAction::Restart);
        assert_eq!(result.state, ServiceState::Running);
    }

    #[tokio::test]
    async fn test_perform_action() {
        let services = create_test_services();
        let backend = ExecBackend::new(services);

        // Test perform_action with different actions
        let result = backend
            .perform_action("test-service", ServiceAction::Start)
            .await
            .unwrap();
        assert!(result.success);

        let result = backend
            .perform_action("stopped-service", ServiceAction::Stop)
            .await
            .unwrap();
        assert!(result.success);
    }
}
