//! Systemd backend implementation.
//!
//! This backend uses systemctl to manage services.
//! It's designed for Linux hosts running systemd.

use crate::config::AclConfig;
use crate::error::{Result, ShikiError};
use crate::service::backend::{
    ServiceAction, ServiceBackend, ServiceOperationResult, ServiceState, ServiceStatus,
};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Systemd backend for service operations.
///
/// This backend uses systemctl to manage services on the local system.
pub struct SystemdBackend {
    /// Access control list for services.
    acl: AclConfig,
}

impl SystemdBackend {
    /// Creates a new systemd backend with the given ACL configuration.
    pub fn new(acl: AclConfig) -> Self {
        Self { acl }
    }

    /// Checks if a service is allowed by ACL.
    fn check_acl(&self, service: &str) -> Result<()> {
        if !self.acl.is_allowed(service) {
            return Err(ShikiError::ServiceDenied {
                service: service.to_string(),
                reason: "Service is not allowed by ACL".to_string(),
            });
        }
        Ok(())
    }

    /// Executes a systemctl command and returns the result.
    async fn systemctl(&self, args: &[&str]) -> Result<(bool, String)> {
        debug!(args = ?args, "Executing systemctl");

        let output = Command::new("systemctl")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                ShikiError::backend_with_source(format!("Failed to execute systemctl: {}", e), e)
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined_output = if stderr.is_empty() {
            stdout.to_string()
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        debug!(
            exit_code = output.status.code(),
            stdout = %stdout,
            stderr = %stderr,
            "systemctl completed"
        );

        Ok((output.status.success(), combined_output))
    }

    /// Checks if a service exists by trying to get its status.
    async fn service_exists(&self, service: &str) -> bool {
        // Use show to check if the service exists
        let result = self
            .systemctl(&["show", "--property=LoadState", service])
            .await;

        match result {
            Ok((_, output)) => {
                // If LoadState is not "not-found", the service exists
                !output.contains("LoadState=not-found")
            }
            Err(_) => false,
        }
    }

    /// Gets the current state of a service.
    async fn get_service_state(&self, service: &str) -> Result<ServiceState> {
        let (success, output) = self.systemctl(&["is-active", service]).await?;

        let state_str = output.trim();
        let state = match state_str {
            "active" => ServiceState::Running,
            "inactive" => ServiceState::Stopped,
            "failed" => ServiceState::Failed,
            "activating" => ServiceState::Running, // Consider activating as running
            "deactivating" => ServiceState::Stopped, // Consider deactivating as stopped
            _ => {
                if success {
                    ServiceState::Running
                } else {
                    ServiceState::Unknown
                }
            }
        };

        Ok(state)
    }
}

#[async_trait]
impl ServiceBackend for SystemdBackend {
    fn name(&self) -> &'static str {
        "systemd"
    }

    fn supports_service(&self, service: &str) -> bool {
        // For systemd, we need to check ACL
        self.acl.is_allowed(service)
    }

    async fn list_services(&self) -> Result<Vec<String>> {
        let (success, output) = self
            .systemctl(&[
                "list-unit-files",
                "--type=service",
                "--no-legend",
                "--no-pager",
            ])
            .await?;

        if !success {
            return Err(ShikiError::backend("Failed to list services"));
        }

        let services: Vec<String> = output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    let service = parts[0].strip_suffix(".service").unwrap_or(parts[0]);
                    if self.acl.is_allowed(service) {
                        Some(service.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok(services)
    }

    async fn status(&self, service: &str) -> Result<ServiceStatus> {
        self.check_acl(service)?;

        // Check if service exists
        if !self.service_exists(service).await {
            return Err(ShikiError::ServiceNotFound {
                service: service.to_string(),
            });
        }

        let state = self.get_service_state(service).await?;

        // Get description
        let (_, description_output) = self
            .systemctl(&["show", "--property=Description", service])
            .await?;

        let description = description_output
            .trim()
            .strip_prefix("Description=")
            .map(|s| s.to_string());

        Ok(ServiceStatus {
            name: service.to_string(),
            state,
            description,
        })
    }

    async fn start(&self, service: &str) -> Result<ServiceOperationResult> {
        self.check_acl(service)?;

        // Check if service exists
        if !self.service_exists(service).await {
            return Err(ShikiError::ServiceNotFound {
                service: service.to_string(),
            });
        }

        info!(service = service, "Starting service via systemd");

        // Check current state
        let current_state = self.get_service_state(service).await?;
        if current_state == ServiceState::Running {
            info!(service = service, "Service is already running");
            return Ok(ServiceOperationResult::success(
                service,
                ServiceAction::Start,
                ServiceState::Running,
            ));
        }

        // Start the service
        let (success, output) = self.systemctl(&["start", service]).await?;

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
        let new_state = self.get_service_state(service).await?;

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
        self.check_acl(service)?;

        // Check if service exists
        if !self.service_exists(service).await {
            return Err(ShikiError::ServiceNotFound {
                service: service.to_string(),
            });
        }

        info!(service = service, "Stopping service via systemd");

        // Check current state
        let current_state = self.get_service_state(service).await?;
        if current_state == ServiceState::Stopped {
            info!(service = service, "Service is already stopped");
            return Ok(ServiceOperationResult::success(
                service,
                ServiceAction::Stop,
                ServiceState::Stopped,
            ));
        }

        // Stop the service
        let (success, output) = self.systemctl(&["stop", service]).await?;

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
        let new_state = self.get_service_state(service).await?;

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
        self.check_acl(service)?;

        // Check if service exists
        if !self.service_exists(service).await {
            return Err(ShikiError::ServiceNotFound {
                service: service.to_string(),
            });
        }

        info!(service = service, "Restarting service via systemd");

        // Restart the service
        let (success, output) = self.systemctl(&["restart", service]).await?;

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
        let new_state = self.get_service_state(service).await?;

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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemd_backend_new() {
        let acl = AclConfig::default();
        let backend = SystemdBackend::new(acl);

        assert_eq!(backend.name(), "systemd");
    }

    #[test]
    fn test_supports_service_with_acl() {
        let acl = AclConfig {
            allowed: vec!["nginx".to_string(), "redis-*".to_string()],
            denied: vec!["redis-test".to_string()],
        };
        let backend = SystemdBackend::new(acl);

        assert!(backend.supports_service("nginx"));
        assert!(backend.supports_service("redis-server"));
        assert!(!backend.supports_service("redis-test")); // denied
        assert!(!backend.supports_service("postgres")); // not in allowed
    }

    #[test]
    fn test_check_acl() {
        let acl = AclConfig {
            allowed: vec!["nginx".to_string()],
            denied: vec![],
        };
        let backend = SystemdBackend::new(acl);

        assert!(backend.check_acl("nginx").is_ok());
        assert!(backend.check_acl("postgres").is_err());
    }

    // Note: Integration tests for systemd operations would require
    // a Linux system with systemd running. These are skipped in
    // the dev container environment.
}
