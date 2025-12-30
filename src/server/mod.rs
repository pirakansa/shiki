//! HTTP Server module - REST API server implementation.
//!
//! This module provides the HTTP server for shiki, including
//! routing, request handling, and response formatting.

pub mod handlers;
pub mod response;
pub mod state;

#[cfg(test)]
mod handlers_tests;

use crate::config::Config;
use crate::error::Result;
use axum::{
    routing::{get, post},
    Router,
};
use state::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

/// Creates the API router with all endpoints.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health and status endpoints
        .route("/api/v1/health", get(handlers::health))
        .route("/api/v1/status", get(handlers::status))
        // Notification endpoint
        .route("/api/v1/notify", post(handlers::notify))
        // Service endpoints
        .route("/api/v1/services", get(handlers::list_services))
        .route("/api/v1/services/:name", get(handlers::get_service))
        .route(
            "/api/v1/services/:name/start",
            post(handlers::start_service),
        )
        .route("/api/v1/services/:name/stop", post(handlers::stop_service))
        .route(
            "/api/v1/services/:name/restart",
            post(handlers::restart_service),
        )
        // Add tracing layer
        .layer(TraceLayer::new_for_http())
        // Add state
        .with_state(state)
}

/// Starts the HTTP server.
pub async fn serve(config: &Config) -> Result<()> {
    let state = Arc::new(AppState::new(config)?);
    let router = create_router(state);

    let addr = SocketAddr::new(
        config.server.bind.parse().map_err(|e| {
            crate::error::ShikiError::config(format!("Invalid bind address: {}", e))
        })?,
        config.server.port,
    );

    info!("Starting HTTP server on {}", addr);

    let listener = TcpListener::bind(addr).await.map_err(|e| {
        crate::error::ShikiError::backend_with_source(
            format!("Failed to bind to {}: {}", addr, e),
            e,
        )
    })?;

    axum::serve(listener, router).await.map_err(|e| {
        crate::error::ShikiError::backend_with_source(format!("Server error: {}", e), e)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Backend, ServiceDefinition};

    fn create_test_config() -> Config {
        let mut config = Config::default();
        config.agent.backend = Backend::Exec;
        config.services.insert(
            "test-service".to_string(),
            ServiceDefinition {
                start: "true".to_string(),
                stop: "true".to_string(),
                status: "true".to_string(),
                ..Default::default()
            },
        );
        config
    }

    #[test]
    fn test_create_router() {
        let config = create_test_config();
        let state = Arc::new(AppState::new(&config).unwrap());
        let _router = create_router(state);
        // Router creation should not panic
    }
}
