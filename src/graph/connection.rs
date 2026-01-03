//! Connection types for the graph.

use crate::core::error::{ConnectionId, NodeId};
use serde::{Deserialize, Serialize};

/// An endpoint of a connection (node + port).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Endpoint {
    /// The node ID.
    pub node_id: NodeId,
    /// The port name on that node.
    pub port_name: String,
}

impl Endpoint {
    /// Create a new endpoint.
    pub fn new(node_id: NodeId, port_name: impl Into<String>) -> Self {
        Self {
            node_id,
            port_name: port_name.into(),
        }
    }
}

/// A connection between two ports in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Unique identifier for this connection.
    pub id: ConnectionId,
    /// Source endpoint (output port).
    pub from: Endpoint,
    /// Target endpoint (input port).
    pub to: Endpoint,
}

impl Connection {
    /// Create a new connection.
    pub fn new(from: Endpoint, to: Endpoint) -> Self {
        Self {
            id: ConnectionId::new(),
            from,
            to,
        }
    }

    /// Create with a specific ID.
    pub fn with_id(mut self, id: ConnectionId) -> Self {
        self.id = id;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint() {
        let node_id = NodeId::new();
        let endpoint = Endpoint::new(node_id, "output");
        
        assert_eq!(endpoint.node_id, node_id);
        assert_eq!(endpoint.port_name, "output");
    }

    #[test]
    fn test_connection() {
        let node1 = NodeId::new();
        let node2 = NodeId::new();
        
        let conn = Connection::new(
            Endpoint::new(node1, "output"),
            Endpoint::new(node2, "input"),
        );
        
        assert_eq!(conn.from.node_id, node1);
        assert_eq!(conn.to.node_id, node2);
    }
}
