//! Graph structure and node management.
//!
//! The ProcessingGraph is the central data structure that holds all nodes
//! and their connections. It uses a centralized approach for:
//! - Easy serialization
//! - Graph-wide validation
//! - Execution planning

use crate::core::error::{ConnectionId, GraphError, GraphResult, NodeId};
use crate::core::node::FilterNode;
use crate::core::types::Value;
use crate::graph::connection::{Connection, Endpoint};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Position of a node in the UI (for serialization).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A node instance in the graph.
///
/// Contains the filter implementation, position, and parameter values.
#[derive(Clone)]
pub struct GraphNode {
    /// Unique identifier
    pub id: NodeId,
    /// The filter implementation
    pub filter: Box<dyn FilterNode>,
    /// Position in the UI
    pub position: Position,
    /// Current parameter values (overriding defaults)
    pub parameters: HashMap<String, Value>,
    /// Optional display name override
    pub label: Option<String>,
    /// Whether this node is disabled
    pub disabled: bool,
}

impl std::fmt::Debug for GraphNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphNode")
            .field("id", &self.id)
            .field("filter", &self.filter.metadata().name)
            .field("position", &self.position)
            .field("parameters", &self.parameters)
            .field("label", &self.label)
            .field("disabled", &self.disabled)
            .finish()
    }
}

impl GraphNode {
    /// Create a new graph node with a filter.
    pub fn new(filter: Box<dyn FilterNode>) -> Self {
        Self {
            id: NodeId::new(),
            filter,
            position: Position::default(),
            parameters: HashMap::new(),
            label: None,
            disabled: false,
        }
    }

    /// Create with a specific ID.
    pub fn with_id(mut self, id: NodeId) -> Self {
        self.id = id;
        self
    }

    /// Set the position.
    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position = Position::new(x, y);
        self
    }

    /// Set a parameter value.
    pub fn with_parameter(mut self, name: impl Into<String>, value: Value) -> Self {
        self.parameters.insert(name.into(), value);
        self
    }

    /// Set the display label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Get the display name (label or filter name).
    pub fn display_name(&self) -> String {
        self.label
            .clone()
            .unwrap_or_else(|| self.filter.metadata().name.clone())
    }

    /// Get a parameter value, falling back to default.
    pub fn get_parameter(&self, name: &str) -> Option<Value> {
        // Check custom value first
        if let Some(value) = self.parameters.get(name) {
            return Some(value.clone());
        }
        // Fall back to default
        self.filter
            .metadata()
            .get_parameter(name)
            .map(|p| p.default_value.clone())
    }

    /// Set a parameter value.
    pub fn set_parameter(&mut self, name: impl Into<String>, value: Value) {
        self.parameters.insert(name.into(), value);
    }
}

/// Metadata about the graph itself.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphMetadata {
    /// Optional name for this graph.
    pub name: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Author information.
    pub author: Option<String>,
    /// Version string.
    pub version: Option<String>,
    /// Additional tags.
    pub tags: Vec<String>,
    /// Creation timestamp.
    pub created_at: Option<String>,
    /// Last modified timestamp.
    pub modified_at: Option<String>,
}

/// The main processing graph structure.
///
/// Uses IndexMap to maintain insertion order for consistent iteration.
#[derive(Debug, Clone)]
pub struct ProcessingGraph {
    /// All nodes in the graph, indexed by ID.
    nodes: IndexMap<NodeId, GraphNode>,
    /// All connections in the graph.
    connections: Vec<Connection>,
    /// Graph metadata.
    pub metadata: GraphMetadata,
}

impl ProcessingGraph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Self {
            nodes: IndexMap::new(),
            connections: Vec::new(),
            metadata: GraphMetadata::default(),
        }
    }

    /// Create with metadata.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.metadata.name = Some(name.into());
        self
    }

    // ========================================================================
    // Node Management
    // ========================================================================

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: GraphNode) -> NodeId {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    /// Add a node from a filter and return the node ID.
    pub fn add_filter(&mut self, filter: Box<dyn FilterNode>) -> NodeId {
        self.add_node(GraphNode::new(filter))
    }

    /// Remove a node from the graph.
    ///
    /// Also removes all connections involving this node.
    pub fn remove_node(&mut self, id: NodeId) -> GraphResult<GraphNode> {
        // Remove all connections involving this node
        self.connections.retain(|conn| {
            conn.from.node_id != id && conn.to.node_id != id
        });

        self.nodes
            .shift_remove(&id)
            .ok_or(GraphError::NodeNotFound(id))
    }

    /// Get a reference to a node.
    pub fn get_node(&self, id: NodeId) -> GraphResult<&GraphNode> {
        self.nodes.get(&id).ok_or(GraphError::NodeNotFound(id))
    }

    /// Get a mutable reference to a node.
    pub fn get_node_mut(&mut self, id: NodeId) -> GraphResult<&mut GraphNode> {
        self.nodes.get_mut(&id).ok_or(GraphError::NodeNotFound(id))
    }

    /// Check if a node exists.
    pub fn has_node(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Get all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values()
    }

    /// Get all nodes mutably.
    pub fn nodes_mut(&mut self) -> impl Iterator<Item = &mut GraphNode> {
        self.nodes.values_mut()
    }

    /// Get all node IDs.
    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    // ========================================================================
    // Connection Management
    // ========================================================================

    /// Create a connection between two ports.
    pub fn connect(
        &mut self,
        from_node: NodeId,
        from_port: impl Into<String>,
        to_node: NodeId,
        to_port: impl Into<String>,
    ) -> GraphResult<ConnectionId> {
        let from_port = from_port.into();
        let to_port = to_port.into();

        // Verify nodes exist
        let from_node_data = self.get_node(from_node)?;
        let to_node_data = self.get_node(to_node)?;

        // Verify ports exist
        let from_metadata = from_node_data.filter.metadata();
        let to_metadata = to_node_data.filter.metadata();

        let from_port_def = from_metadata
            .get_output(&from_port)
            .ok_or_else(|| GraphError::PortNotFound {
                node_id: from_node,
                port: from_port.clone(),
            })?;

        let to_port_def = to_metadata
            .get_input(&to_port)
            .ok_or_else(|| GraphError::PortNotFound {
                node_id: to_node,
                port: to_port.clone(),
            })?;

        // Check type compatibility
        if !from_port_def.port_type.compatible_with(&to_port_def.port_type) {
            return Err(GraphError::TypeMismatch {
                from_type: from_port_def.port_type.clone(),
                to_type: to_port_def.port_type.clone(),
            });
        }

        // Check if input port is already connected
        if self.is_input_connected(to_node, &to_port) {
            return Err(GraphError::PortAlreadyConnected {
                node_id: to_node,
                port: to_port,
            });
        }

        // Check for cycles
        if self.would_create_cycle(from_node, to_node) {
            return Err(GraphError::CycleDetected {
                nodes: vec![from_node, to_node],
            });
        }

        // Create connection
        let connection = Connection {
            id: ConnectionId::new(),
            from: Endpoint {
                node_id: from_node,
                port_name: from_port,
            },
            to: Endpoint {
                node_id: to_node,
                port_name: to_port,
            },
        };

        let id = connection.id;
        self.connections.push(connection);
        Ok(id)
    }

    /// Remove a connection by ID.
    pub fn disconnect(&mut self, id: ConnectionId) -> GraphResult<Connection> {
        let pos = self
            .connections
            .iter()
            .position(|c| c.id == id)
            .ok_or(GraphError::ConnectionNotFound(id))?;

        Ok(self.connections.remove(pos))
    }

    /// Remove all connections to a specific input port.
    pub fn disconnect_input(&mut self, node_id: NodeId, port: &str) {
        self.connections.retain(|conn| {
            !(conn.to.node_id == node_id && conn.to.port_name == port)
        });
    }

    /// Get a connection by ID.
    pub fn get_connection(&self, id: ConnectionId) -> GraphResult<&Connection> {
        self.connections
            .iter()
            .find(|c| c.id == id)
            .ok_or(GraphError::ConnectionNotFound(id))
    }

    /// Get all connections.
    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }

    /// Get all connections from a node.
    pub fn connections_from(&self, node_id: NodeId) -> impl Iterator<Item = &Connection> {
        self.connections
            .iter()
            .filter(move |c| c.from.node_id == node_id)
    }

    /// Get all connections to a node.
    pub fn connections_to(&self, node_id: NodeId) -> impl Iterator<Item = &Connection> {
        self.connections
            .iter()
            .filter(move |c| c.to.node_id == node_id)
    }

    /// Check if an input port is already connected.
    pub fn is_input_connected(&self, node_id: NodeId, port: &str) -> bool {
        self.connections
            .iter()
            .any(|c| c.to.node_id == node_id && c.to.port_name == port)
    }

    /// Get the number of connections.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    // ========================================================================
    // Graph Analysis
    // ========================================================================

    /// Check if connecting from_node to to_node would create a cycle.
    fn would_create_cycle(&self, from_node: NodeId, to_node: NodeId) -> bool {
        // If from_node is reachable from to_node, adding this edge creates a cycle
        self.is_reachable(to_node, from_node)
    }

    /// Check if `target` is reachable from `start` following connections.
    pub fn is_reachable(&self, start: NodeId, target: NodeId) -> bool {
        if start == target {
            return true;
        }

        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            if current == target {
                return true;
            }

            if visited.insert(current) {
                for conn in self.connections_from(current) {
                    queue.push_back(conn.to.node_id);
                }
            }
        }

        false
    }

    /// Get all nodes that depend on the given node (downstream).
    pub fn get_downstream(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Start with immediate dependencies
        for conn in self.connections_from(node_id) {
            queue.push_back(conn.to.node_id);
        }

        while let Some(current) = queue.pop_front() {
            if visited.insert(current) {
                result.push(current);
                for conn in self.connections_from(current) {
                    queue.push_back(conn.to.node_id);
                }
            }
        }

        result
    }

    /// Get all nodes that the given node depends on (upstream).
    pub fn get_upstream(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        // Start with immediate dependencies
        for conn in self.connections_to(node_id) {
            queue.push_back(conn.from.node_id);
        }

        while let Some(current) = queue.pop_front() {
            if visited.insert(current) {
                result.push(current);
                for conn in self.connections_to(current) {
                    queue.push_back(conn.from.node_id);
                }
            }
        }

        result
    }

    /// Get nodes with no incoming connections (source nodes).
    pub fn get_source_nodes(&self) -> Vec<NodeId> {
        self.nodes
            .keys()
            .filter(|&id| !self.connections.iter().any(|c| c.to.node_id == *id))
            .copied()
            .collect()
    }

    /// Get nodes with no outgoing connections (sink nodes).
    pub fn get_sink_nodes(&self) -> Vec<NodeId> {
        self.nodes
            .keys()
            .filter(|&id| !self.connections.iter().any(|c| c.from.node_id == *id))
            .copied()
            .collect()
    }

    /// Check if the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Clear all nodes and connections.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.connections.clear();
    }
}

impl Default for ProcessingGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::node::PassthroughNode;

    fn create_test_node() -> GraphNode {
        GraphNode::new(Box::new(PassthroughNode))
    }

    #[test]
    fn test_add_remove_node() {
        let mut graph = ProcessingGraph::new();

        let id = graph.add_node(create_test_node());
        assert_eq!(graph.node_count(), 1);
        assert!(graph.has_node(id));

        graph.remove_node(id).unwrap();
        assert_eq!(graph.node_count(), 0);
        assert!(!graph.has_node(id));
    }

    #[test]
    fn test_connect_nodes() {
        let mut graph = ProcessingGraph::new();

        let node1 = graph.add_node(create_test_node());
        let node2 = graph.add_node(create_test_node());

        let conn_id = graph.connect(node1, "output", node2, "input").unwrap();
        assert_eq!(graph.connection_count(), 1);

        graph.disconnect(conn_id).unwrap();
        assert_eq!(graph.connection_count(), 0);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = ProcessingGraph::new();

        let node1 = graph.add_node(create_test_node());
        let node2 = graph.add_node(create_test_node());
        let node3 = graph.add_node(create_test_node());

        graph.connect(node1, "output", node2, "input").unwrap();
        graph.connect(node2, "output", node3, "input").unwrap();

        // This should fail - would create cycle
        let result = graph.connect(node3, "output", node1, "input");
        assert!(matches!(result, Err(GraphError::CycleDetected { .. })));
    }

    #[test]
    fn test_source_sink_nodes() {
        let mut graph = ProcessingGraph::new();

        let node1 = graph.add_node(create_test_node());
        let node2 = graph.add_node(create_test_node());
        let node3 = graph.add_node(create_test_node());

        graph.connect(node1, "output", node2, "input").unwrap();
        graph.connect(node2, "output", node3, "input").unwrap();

        let sources = graph.get_source_nodes();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains(&node1));

        let sinks = graph.get_sink_nodes();
        assert_eq!(sinks.len(), 1);
        assert!(sinks.contains(&node3));
    }

    #[test]
    fn test_upstream_downstream() {
        let mut graph = ProcessingGraph::new();

        let node1 = graph.add_node(create_test_node());
        let node2 = graph.add_node(create_test_node());
        let node3 = graph.add_node(create_test_node());

        graph.connect(node1, "output", node2, "input").unwrap();
        graph.connect(node2, "output", node3, "input").unwrap();

        let downstream = graph.get_downstream(node1);
        assert_eq!(downstream.len(), 2);
        assert!(downstream.contains(&node2));
        assert!(downstream.contains(&node3));

        let upstream = graph.get_upstream(node3);
        assert_eq!(upstream.len(), 2);
        assert!(upstream.contains(&node1));
        assert!(upstream.contains(&node2));
    }
}
