//! Graph serialization for saving and loading.

use crate::core::error::NodeId;
use crate::core::types::Value;
use crate::graph::connection::Connection;
use crate::graph::structure::{GraphMetadata, Position};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Serializable representation of a graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedNode {
    /// Node ID
    pub id: NodeId,
    /// Filter type ID (to look up in registry)
    pub filter_id: String,
    /// Position in UI
    pub position: Position,
    /// Parameter values (override defaults)
    pub parameters: HashMap<String, Value>,
    /// Optional display label
    pub label: Option<String>,
    /// Whether the node is disabled
    pub disabled: bool,
}

/// Serializable representation of a connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedConnection {
    /// From node ID
    pub from_node: NodeId,
    /// From port name
    pub from_port: String,
    /// To node ID
    pub to_node: NodeId,
    /// To port name
    pub to_port: String,
}

impl From<&Connection> for SerializedConnection {
    fn from(conn: &Connection) -> Self {
        Self {
            from_node: conn.from.node_id,
            from_port: conn.from.port_name.clone(),
            to_node: conn.to.node_id,
            to_port: conn.to.port_name.clone(),
        }
    }
}

/// Serializable representation of a complete graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedGraph {
    /// Graph format version
    pub version: String,
    /// Graph metadata
    pub metadata: GraphMetadata,
    /// All nodes
    pub nodes: Vec<SerializedNode>,
    /// All connections
    pub connections: Vec<SerializedConnection>,
}

impl SerializedGraph {
    /// Current format version.
    pub const VERSION: &'static str = "1.0.0";

    /// Create a new serialized graph.
    pub fn new() -> Self {
        Self {
            version: Self::VERSION.to_string(),
            metadata: GraphMetadata::default(),
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to compact JSON (no whitespace).
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl Default for SerializedGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_graph() {
        let mut graph = SerializedGraph::new();
        graph.metadata.name = Some("Test Graph".to_string());
        graph.nodes.push(SerializedNode {
            id: NodeId::new(),
            filter_id: "load_image".to_string(),
            position: Position::new(100.0, 100.0),
            parameters: HashMap::new(),
            label: None,
            disabled: false,
        });

        let json = graph.to_json().unwrap();
        assert!(json.contains("Test Graph"));
        assert!(json.contains("load_image"));

        let deserialized = SerializedGraph::from_json(&json).unwrap();
        assert_eq!(deserialized.metadata.name, Some("Test Graph".to_string()));
        assert_eq!(deserialized.nodes.len(), 1);
    }
}
