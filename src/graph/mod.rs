//! Graph module for managing processing graphs.
//!
//! A processing graph is a directed acyclic graph (DAG) where nodes represent
//! filter operations and edges represent data flow between them.

pub mod structure;
pub mod connection;
pub mod topology;
pub mod serialization;

// Re-export commonly used types
pub use structure::{ProcessingGraph, GraphNode, Position};
pub use connection::{Connection, Endpoint};
pub use topology::TopologyAnalyzer;
