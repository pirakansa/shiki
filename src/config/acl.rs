//! Access control list configuration.

use serde::{Deserialize, Serialize};

/// Access control list configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AclConfig {
    /// Allowed services (empty = allow all).
    pub allowed: Vec<String>,

    /// Denied services.
    pub denied: Vec<String>,
}

impl AclConfig {
    /// Checks if a service is allowed.
    /// Evaluation order: denied -> (allowed empty = allow all) -> allowed match -> deny
    pub fn is_allowed(&self, service: &str) -> bool {
        // Check denied list first
        for pattern in &self.denied {
            if glob_match::glob_match(pattern, service) {
                return false;
            }
        }

        // If allowed list is empty, allow all
        if self.allowed.is_empty() {
            return true;
        }

        // Check allowed list
        for pattern in &self.allowed {
            if glob_match::glob_match(pattern, service) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acl_is_allowed() {
        let acl = AclConfig {
            allowed: vec!["nginx".to_string(), "redis-*".to_string()],
            denied: vec!["redis-test".to_string()],
        };

        assert!(acl.is_allowed("nginx"));
        assert!(acl.is_allowed("redis-server"));
        assert!(acl.is_allowed("redis-sentinel"));
        assert!(!acl.is_allowed("redis-test")); // denied
        assert!(!acl.is_allowed("postgres")); // not in allowed
    }

    #[test]
    fn test_acl_empty_allowed() {
        let acl = AclConfig {
            allowed: vec![],
            denied: vec!["secret-*".to_string()],
        };

        assert!(acl.is_allowed("nginx"));
        assert!(acl.is_allowed("redis"));
        assert!(!acl.is_allowed("secret-service"));
    }

    #[test]
    fn test_acl_default() {
        let acl = AclConfig::default();
        assert!(acl.allowed.is_empty());
        assert!(acl.denied.is_empty());
        assert!(acl.is_allowed("any-service"));
    }
}
