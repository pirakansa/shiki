//! HTTP request handlers.
//!
//! This module contains all the HTTP endpoint handlers for the shiki API.

use crate::error::ShikiError;
use crate::server::response::{
    AgentInfo, AgentState, ApiResponse, HealthData, HealthStatus, NotifyRequest,
    NotifyResponseData, ServerInfo, ServiceDetailData, ServiceInfo, ServiceOperationData,
    ServicesListData, StatsInfo, StatusData,
};
use crate::server::state::AppState;
use crate::service::ServiceAction;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use uuid::Uuid;

/// Version string for the application.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Health check handler.
///
/// GET /api/v1/health
pub async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.increment_requests();

    let data = HealthData {
        status: HealthStatus::Healthy,
        version: VERSION.to_string(),
        uptime_seconds: state.uptime_seconds(),
    };

    state.increment_success();
    (StatusCode::OK, Json(ApiResponse::success(data)))
}

/// Agent status handler.
///
/// GET /api/v1/status
pub async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.increment_requests();

    let stats_snapshot = state.stats.snapshot();

    let data = StatusData {
        agent: AgentInfo {
            name: state.agent_name.clone(),
            state: AgentState::Ready,
            mode: "standalone".to_string(),
            tags: state.agent_tags.clone(),
        },
        server: ServerInfo {
            bind: state.server_bind.clone(),
            port: state.server_port,
            tls_enabled: state.tls_enabled,
        },
        stats: StatsInfo {
            requests_total: stats_snapshot.requests_total,
            requests_success: stats_snapshot.requests_success,
            requests_failed: stats_snapshot.requests_failed,
            active_connections: 0, // TODO: Track active connections
        },
        version: VERSION.to_string(),
        uptime_seconds: state.uptime_seconds(),
    };

    state.increment_success();
    (StatusCode::OK, Json(ApiResponse::success(data)))
}

/// Notify handler - receives notifications and performs service operations.
///
/// POST /api/v1/notify
pub async fn notify(
    State(state): State<Arc<AppState>>,
    Json(request): Json<NotifyRequest>,
) -> impl IntoResponse {
    state.increment_requests();

    let request_id = Uuid::new_v4();
    let start_time = Instant::now();

    info!(
        request_id = %request_id,
        service = %request.service,
        action = %request.action,
        "Processing notify request"
    );

    // Parse the action
    let action = match request.action.to_lowercase().as_str() {
        "start" => ServiceAction::Start,
        "stop" => ServiceAction::Stop,
        "restart" => ServiceAction::Restart,
        _ => {
            state.increment_failed();
            let err = ShikiError::invalid_request(format!("Invalid action: {}", request.action));
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<NotifyResponseData>::from_error(&err)),
            );
        }
    };

    // Check if service is supported
    if !state.controller.supports_service(&request.service) {
        state.increment_failed();
        let err = ShikiError::ServiceNotFound {
            service: request.service.clone(),
        };
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<NotifyResponseData>::from_error(&err)),
        );
    }

    // Get previous status
    let previous_status = state
        .controller
        .status(&request.service)
        .await
        .ok()
        .map(|s| s.state.to_string());

    // Perform the action
    let result = state
        .controller
        .perform_action(&request.service, action)
        .await;

    match result {
        Ok(op_result) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;

            let data = NotifyResponseData {
                request_id,
                service: request.service,
                action: request.action,
                result: if op_result.success {
                    "completed".to_string()
                } else {
                    "failed".to_string()
                },
                previous_status,
                current_status: Some(op_result.state.to_string()),
                duration_ms: Some(duration_ms),
                message: op_result.message,
            };

            if op_result.success {
                state.increment_success();
                (StatusCode::OK, Json(ApiResponse::success(data)))
            } else {
                state.increment_failed();
                // Still return OK but with failed result in the data
                (StatusCode::OK, Json(ApiResponse::success(data)))
            }
        }
        Err(err) => {
            error!(
                request_id = %request_id,
                error = %err,
                "Notify request failed"
            );
            state.increment_failed();

            let status_code = match &err {
                ShikiError::ServiceNotFound { .. } => StatusCode::NOT_FOUND,
                ShikiError::ServiceDenied { .. } => StatusCode::FORBIDDEN,
                ShikiError::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };

            (
                status_code,
                Json(ApiResponse::<NotifyResponseData>::from_error(&err)),
            )
        }
    }
}

/// Query parameters for listing services.
#[derive(Debug, Deserialize)]
pub struct ListServicesQuery {
    /// Filter by status.
    pub status: Option<String>,
    /// Maximum number of results.
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Offset for pagination.
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    100
}

/// List services handler.
///
/// GET /api/v1/services
pub async fn list_services(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListServicesQuery>,
) -> impl IntoResponse {
    state.increment_requests();

    let services_result = state.controller.list_services().await;

    match services_result {
        Ok(service_names) => {
            let mut services = Vec::new();

            for name in &service_names {
                if let Ok(status) = state.controller.status(name).await {
                    // Apply status filter if provided
                    if let Some(ref filter_status) = query.status {
                        if status.state.to_string() != *filter_status {
                            continue;
                        }
                    }

                    services.push(ServiceInfo {
                        name: status.name,
                        status: status.state.to_string(),
                        description: status.description,
                    });
                }
            }

            let total = services.len();

            // Apply pagination
            let services: Vec<ServiceInfo> = services
                .into_iter()
                .skip(query.offset)
                .take(query.limit)
                .collect();

            let data = ServicesListData {
                services,
                total,
                limit: query.limit,
                offset: query.offset,
            };

            state.increment_success();
            (StatusCode::OK, Json(ApiResponse::success(data)))
        }
        Err(err) => {
            error!(error = %err, "Failed to list services");
            state.increment_failed();
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<ServicesListData>::from_error(&err)),
            )
        }
    }
}

/// Get service details handler.
///
/// GET /api/v1/services/:name
pub async fn get_service(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    state.increment_requests();

    let status_result = state.controller.status(&name).await;

    match status_result {
        Ok(status) => {
            let data = ServiceDetailData {
                name: status.name,
                status: status.state.to_string(),
                description: status.description,
            };

            state.increment_success();
            (StatusCode::OK, Json(ApiResponse::success(data)))
        }
        Err(err) => {
            state.increment_failed();

            let status_code = match &err {
                ShikiError::ServiceNotFound { .. } => StatusCode::NOT_FOUND,
                ShikiError::ServiceDenied { .. } => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };

            (
                status_code,
                Json(ApiResponse::<ServiceDetailData>::from_error(&err)),
            )
        }
    }
}

/// Start service handler.
///
/// POST /api/v1/services/:name/start
pub async fn start_service(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    service_action(state, name, ServiceAction::Start).await
}

/// Stop service handler.
///
/// POST /api/v1/services/:name/stop
pub async fn stop_service(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    service_action(state, name, ServiceAction::Stop).await
}

/// Restart service handler.
///
/// POST /api/v1/services/:name/restart
pub async fn restart_service(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    service_action(state, name, ServiceAction::Restart).await
}

/// Common service action handler.
async fn service_action(
    state: Arc<AppState>,
    service: String,
    action: ServiceAction,
) -> impl IntoResponse {
    state.increment_requests();

    info!(
        service = %service,
        action = %action,
        "Processing service action"
    );

    // Get previous status
    let previous_state = state
        .controller
        .status(&service)
        .await
        .ok()
        .map(|s| s.state.to_string());

    // Perform the action
    let result = state.controller.perform_action(&service, action).await;

    match result {
        Ok(op_result) => {
            let data = ServiceOperationData {
                service,
                action: action.to_string(),
                success: op_result.success,
                previous_state,
                current_state: op_result.state.to_string(),
                message: op_result.message,
            };

            state.increment_success();
            (StatusCode::OK, Json(ApiResponse::success(data)))
        }
        Err(err) => {
            error!(error = %err, "Service action failed");
            state.increment_failed();

            let status_code = match &err {
                ShikiError::ServiceNotFound { .. } => StatusCode::NOT_FOUND,
                ShikiError::ServiceDenied { .. } => StatusCode::FORBIDDEN,
                ShikiError::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };

            (
                status_code,
                Json(ApiResponse::<ServiceOperationData>::from_error(&err)),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Backend, ServiceDefinition};
    use crate::server::state::AppState;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::{get, post},
        Router,
    };
    use std::collections::HashMap;
    use tower::ServiceExt;

    fn create_test_state() -> Arc<AppState> {
        let mut config = crate::config::Config::default();
        config.agent.backend = Backend::Exec;
        config.agent.name = Some("test-agent".to_string());

        let mut services = HashMap::new();
        services.insert(
            "test-service".to_string(),
            ServiceDefinition {
                start: "true".to_string(),
                stop: "true".to_string(),
                status: "true".to_string(),
                restart: None,
                working_dir: None,
                env: vec![],
            },
        );
        config.services = services;

        Arc::new(AppState::new(&config).unwrap())
    }

    fn create_test_router(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/api/v1/health", get(health))
            .route("/api/v1/status", get(status))
            .route("/api/v1/notify", post(notify))
            .route("/api/v1/services", get(list_services))
            .route("/api/v1/services/:name", get(get_service))
            .route("/api/v1/services/:name/start", post(start_service))
            .route("/api/v1/services/:name/stop", post(stop_service))
            .route("/api/v1/services/:name/restart", post(restart_service))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = create_test_state();
        let app = create_test_router(state);

        let request = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_status_endpoint() {
        let state = create_test_state();
        let app = create_test_router(state);

        let request = Request::builder()
            .uri("/api/v1/status")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_services_endpoint() {
        let state = create_test_state();
        let app = create_test_router(state);

        let request = Request::builder()
            .uri("/api/v1/services")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_service_endpoint() {
        let state = create_test_state();
        let app = create_test_router(state);

        let request = Request::builder()
            .uri("/api/v1/services/test-service")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_service_not_found() {
        let state = create_test_state();
        let app = create_test_router(state);

        let request = Request::builder()
            .uri("/api/v1/services/nonexistent")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_start_service_endpoint() {
        let state = create_test_state();
        let app = create_test_router(state);

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/services/test-service/start")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_stop_service_endpoint() {
        let state = create_test_state();
        let app = create_test_router(state);

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/services/test-service/stop")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_restart_service_endpoint() {
        let state = create_test_state();
        let app = create_test_router(state);

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/services/test-service/restart")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_notify_endpoint() {
        let state = create_test_state();
        let app = create_test_router(state);

        let body = r#"{"action": "start", "service": "test-service"}"#;

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/notify")
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_notify_invalid_action() {
        let state = create_test_state();
        let app = create_test_router(state);

        let body = r#"{"action": "invalid", "service": "test-service"}"#;

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/notify")
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_notify_service_not_found() {
        let state = create_test_state();
        let app = create_test_router(state);

        let body = r#"{"action": "start", "service": "nonexistent"}"#;

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/notify")
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
