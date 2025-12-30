//! Tests for ExecBackend.

#[cfg(test)]
mod tests {
    use crate::config::ServiceDefinition;
    use crate::service::backend::{ServiceAction, ServiceBackend, ServiceState};
    use crate::service::exec::ExecBackend;
    use std::collections::HashMap;

    fn create_test_services() -> HashMap<String, ServiceDefinition> {
        let mut services = HashMap::new();

        // A simple test service using echo and true/false commands
        services.insert(
            "test-service".to_string(),
            ServiceDefinition {
                start: "echo starting".to_string(),
                stop: "echo stopping".to_string(),
                status: "true".to_string(), // Always returns running
                ..Default::default()
            },
        );

        services.insert(
            "stopped-service".to_string(),
            ServiceDefinition {
                start: "echo starting".to_string(),
                stop: "echo stopping".to_string(),
                status: "false".to_string(), // Always returns stopped
                ..Default::default()
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
                ..Default::default()
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
