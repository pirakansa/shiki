//! Shiki HTTP client API.
//!
//! This module provides the client for communicating with shiki agents.

use crate::error::{Result, ShikiError};
use crate::server::response::{
    ApiResponse, HealthData, NotifyOptions, NotifyRequest, NotifyResponseData, ServiceDetailData,
    ServicesListData, StatusData,
};
use crate::service::ServiceAction;
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, error, info};

/// Default timeout for HTTP requests.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Shiki HTTP client for communicating with agents.
#[derive(Debug, Clone)]
pub struct ShikiClient {
    /// HTTP client.
    client: Client,
    /// Base URL of the target agent.
    base_url: String,
}

impl ShikiClient {
    /// Creates a new client for the specified agent URL.
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the agent (e.g., "http://localhost:8080")
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|e| {
                ShikiError::backend_with_source("Failed to create HTTP client".to_string(), e)
            })?;

        Ok(Self {
            client,
            base_url: base_url.into(),
        })
    }

    /// Creates a new client with custom timeout.
    pub fn with_timeout(base_url: impl Into<String>, timeout: Duration) -> Result<Self> {
        let client = Client::builder().timeout(timeout).build().map_err(|e| {
            ShikiError::backend_with_source("Failed to create HTTP client".to_string(), e)
        })?;

        Ok(Self {
            client,
            base_url: base_url.into(),
        })
    }

    /// Checks the health of the target agent.
    ///
    /// # Returns
    /// Health status data from the agent.
    pub async fn health(&self) -> Result<HealthData> {
        let url = format!("{}/api/v1/health", self.base_url);
        debug!(url = %url, "Checking agent health");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ShikiError::connection_with_source(&self.base_url, e))?;

        let api_response: ApiResponse<HealthData> = response.json().await.map_err(|e| {
            ShikiError::backend_with_source("Failed to parse health response".to_string(), e)
        })?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| ShikiError::backend("Health response missing data".to_string()))
        } else {
            Err(Self::extract_error(&api_response))
        }
    }

    /// Gets the status of the target agent.
    ///
    /// # Returns
    /// Detailed status data from the agent.
    pub async fn status(&self) -> Result<StatusData> {
        let url = format!("{}/api/v1/status", self.base_url);
        debug!(url = %url, "Getting agent status");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ShikiError::connection_with_source(&self.base_url, e))?;

        let api_response: ApiResponse<StatusData> = response.json().await.map_err(|e| {
            ShikiError::backend_with_source("Failed to parse status response".to_string(), e)
        })?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| ShikiError::backend("Status response missing data".to_string()))
        } else {
            Err(Self::extract_error(&api_response))
        }
    }

    /// Sends a notification to the target agent to perform a service operation.
    ///
    /// # Arguments
    /// * `service` - Name of the service to operate on
    /// * `action` - Action to perform (start, stop, restart)
    /// * `wait` - Whether to wait for the operation to complete
    /// * `timeout_seconds` - Timeout for the operation
    ///
    /// # Returns
    /// Response data from the notify operation.
    pub async fn notify(
        &self,
        service: &str,
        action: ServiceAction,
        wait: bool,
        timeout_seconds: u64,
    ) -> Result<NotifyResponseData> {
        let url = format!("{}/api/v1/notify", self.base_url);

        let request = NotifyRequest {
            action: action.to_string(),
            service: service.to_string(),
            options: NotifyOptions {
                wait,
                timeout_seconds,
            },
        };

        info!(
            url = %url,
            service = %service,
            action = %action,
            "Sending notify request"
        );

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ShikiError::connection_with_source(&self.base_url, e))?;

        let api_response: ApiResponse<NotifyResponseData> = response.json().await.map_err(|e| {
            ShikiError::backend_with_source("Failed to parse notify response".to_string(), e)
        })?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| ShikiError::backend("Notify response missing data".to_string()))
        } else {
            Err(Self::extract_error(&api_response))
        }
    }

    /// Lists all services on the target agent.
    ///
    /// # Arguments
    /// * `status_filter` - Optional status filter (running, stopped, failed)
    /// * `limit` - Maximum number of results
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    /// List of services from the agent.
    pub async fn list_services(
        &self,
        status_filter: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<ServicesListData> {
        let mut url = format!("{}/api/v1/services", self.base_url);
        let mut params = Vec::new();

        if let Some(status) = status_filter {
            params.push(format!("status={}", status));
        }
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }

        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        debug!(url = %url, "Listing services");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ShikiError::connection_with_source(&self.base_url, e))?;

        let api_response: ApiResponse<ServicesListData> = response.json().await.map_err(|e| {
            ShikiError::backend_with_source("Failed to parse services list response".to_string(), e)
        })?;

        if api_response.success {
            api_response.data.ok_or_else(|| {
                ShikiError::backend("Services list response missing data".to_string())
            })
        } else {
            Err(Self::extract_error(&api_response))
        }
    }

    /// Gets the details of a specific service.
    ///
    /// # Arguments
    /// * `name` - Name of the service
    ///
    /// # Returns
    /// Service details from the agent.
    pub async fn get_service(&self, name: &str) -> Result<ServiceDetailData> {
        let url = format!("{}/api/v1/services/{}", self.base_url, name);
        debug!(url = %url, service = %name, "Getting service details");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ShikiError::connection_with_source(&self.base_url, e))?;

        let api_response: ApiResponse<ServiceDetailData> = response.json().await.map_err(|e| {
            ShikiError::backend_with_source(
                "Failed to parse service detail response".to_string(),
                e,
            )
        })?;

        if api_response.success {
            api_response.data.ok_or_else(|| {
                ShikiError::backend("Service detail response missing data".to_string())
            })
        } else {
            Err(Self::extract_error(&api_response))
        }
    }

    /// Starts a service on the target agent.
    ///
    /// # Arguments
    /// * `name` - Name of the service to start
    pub async fn start_service(&self, name: &str) -> Result<NotifyResponseData> {
        self.notify(name, ServiceAction::Start, true, DEFAULT_TIMEOUT_SECS)
            .await
    }

    /// Stops a service on the target agent.
    ///
    /// # Arguments
    /// * `name` - Name of the service to stop
    pub async fn stop_service(&self, name: &str) -> Result<NotifyResponseData> {
        self.notify(name, ServiceAction::Stop, true, DEFAULT_TIMEOUT_SECS)
            .await
    }

    /// Restarts a service on the target agent.
    ///
    /// # Arguments
    /// * `name` - Name of the service to restart
    pub async fn restart_service(&self, name: &str) -> Result<NotifyResponseData> {
        self.notify(name, ServiceAction::Restart, true, DEFAULT_TIMEOUT_SECS)
            .await
    }

    /// Waits for a service to reach a specific state.
    ///
    /// # Arguments
    /// * `name` - Name of the service
    /// * `target_status` - The desired status (running, stopped)
    /// * `timeout` - Maximum time to wait
    /// * `poll_interval` - Time between status checks
    ///
    /// # Returns
    /// Ok(()) if the service reaches the target state, Err if timeout.
    pub async fn wait_for_service(
        &self,
        name: &str,
        target_status: &str,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        info!(
            service = %name,
            target_status = %target_status,
            timeout_secs = %timeout.as_secs(),
            "Waiting for service to reach target state"
        );

        loop {
            match self.get_service(name).await {
                Ok(service) => {
                    if service.status == target_status {
                        info!(
                            service = %name,
                            status = %service.status,
                            "Service reached target state"
                        );
                        return Ok(());
                    }

                    debug!(
                        service = %name,
                        current_status = %service.status,
                        target_status = %target_status,
                        "Service not yet in target state"
                    );
                }
                Err(e) => {
                    error!(
                        service = %name,
                        error = %e,
                        "Failed to get service status while waiting"
                    );
                }
            }

            if start.elapsed() >= timeout {
                return Err(ShikiError::Timeout {
                    operation: format!("wait for {} to be {}", name, target_status),
                    seconds: timeout.as_secs(),
                });
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Extracts an error from an API response.
    fn extract_error<T>(response: &ApiResponse<T>) -> ShikiError {
        if let Some(err) = &response.error {
            ShikiError::backend(format!("[{}] {}", err.code, err.message))
        } else {
            ShikiError::backend("Unknown error".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ShikiClient::new("http://localhost:8080").unwrap();
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_client_with_timeout() {
        let client =
            ShikiClient::with_timeout("http://localhost:8080", Duration::from_secs(60)).unwrap();
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    // Integration tests would require a running server
    // These are marked as ignored by default
    #[tokio::test]
    #[ignore]
    async fn test_health_check_integration() {
        let client = ShikiClient::new("http://localhost:8080").unwrap();
        let health = client.health().await.unwrap();
        assert_eq!(
            health.status,
            crate::server::response::HealthStatus::Healthy
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_status_check_integration() {
        let client = ShikiClient::new("http://localhost:8080").unwrap();
        let status = client.status().await.unwrap();
        assert!(!status.agent.name.is_empty());
    }
}
