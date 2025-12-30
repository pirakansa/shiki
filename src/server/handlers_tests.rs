//! Tests for HTTP request handlers.

#[cfg(test)]
mod tests {
    use crate::config::{Backend, ServiceDefinition};
    use crate::server::handlers::{
        get_service, health, list_services, notify, restart_service, start_service, status,
        stop_service,
    };
    use crate::server::state::AppState;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::{get, post},
        Router,
    };
    use std::collections::HashMap;
    use std::sync::Arc;
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
                ..Default::default()
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
