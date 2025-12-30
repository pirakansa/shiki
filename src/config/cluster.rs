//! Cluster configuration types.

use serde::{Deserialize, Serialize};

/// Cluster configuration (future implementation).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClusterConfig {
    /// Enable cluster mode.
    pub enabled: bool,

    /// Peer agents.
    pub peers: Vec<PeerConfig>,
}

/// Peer agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    /// Peer name.
    pub name: String,

    /// Peer address (host:port).
    pub address: String,

    /// Peer tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_config_default() {
        let config = ClusterConfig::default();
        assert!(!config.enabled);
        assert!(config.peers.is_empty());
    }

    #[test]
    fn test_peer_config() {
        let peer = PeerConfig {
            name: "peer1".to_string(),
            address: "192.168.1.100:8080".to_string(),
            tags: vec!["production".to_string()],
        };

        assert_eq!(peer.name, "peer1");
        assert_eq!(peer.address, "192.168.1.100:8080");
        assert_eq!(peer.tags.len(), 1);
    }
}
