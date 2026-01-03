//! Error types for Ambara.
//!
//! Uses thiserror for structured errors with context. Errors are designed to:
//! - Be serializable for sending to frontend
//! - Include actionable information (which node, what to fix)
//! - Support error chaining for context

use crate::core::types::PortType;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

/// Unique identifier for a node in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Create a new random node ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a node ID from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0.to_string()[..8])
    }
}

/// Unique identifier for a connection in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub Uuid);

impl ConnectionId {
    /// Create a new random connection ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0.to_string()[..8])
    }
}

/// Top-level error type for Ambara.
///
/// This enum encompasses all error categories and enables automatic
/// conversion between specific error types.
#[derive(Error, Debug)]
pub enum AmbaraError {
    #[error("Graph error: {0}")]
    Graph(#[from] GraphError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),

    #[error("Plugin error: {0}")]
    Plugin(#[from] PluginError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

/// Errors related to graph structure and operations.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum GraphError {
    #[error("Node {0} not found")]
    NodeNotFound(NodeId),

    #[error("Connection {0} not found")]
    ConnectionNotFound(ConnectionId),

    #[error("Port '{port}' not found on node {node_id}")]
    PortNotFound { node_id: NodeId, port: String },

    #[error("Cycle detected in graph involving nodes: {nodes:?}")]
    CycleDetected { nodes: Vec<NodeId> },

    #[error("Invalid connection: {reason}")]
    InvalidConnection { reason: String },

    #[error("Cannot connect {from_type} to {to_type}")]
    TypeMismatch { from_type: PortType, to_type: PortType },

    #[error("Port '{port}' on node {node_id} is already connected")]
    PortAlreadyConnected { node_id: NodeId, port: String },

    #[error("Cannot delete node {0}: it has active connections")]
    NodeHasConnections(NodeId),

    #[error("Graph is empty")]
    EmptyGraph,
}

/// Errors from the validation phase.
///
/// Validation errors are caught before execution begins, allowing users
/// to fix issues before wasting time on processing.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: PortType, got: PortType },

    #[error("Missing required input '{port}' on node {node_id}")]
    MissingRequiredInput { node_id: NodeId, port: String },

    #[error("Constraint violation on node {node_id}, parameter '{parameter}': {error}")]
    ConstraintViolation {
        node_id: NodeId,
        parameter: String,
        error: String,
    },

    #[error("Custom validation failed on node {node_id}: {error}")]
    CustomValidation { node_id: NodeId, error: String },

    #[error("Resource not found: {resource} (referenced by node {node_id})")]
    ResourceNotFound { node_id: NodeId, resource: String },

    #[error("Insufficient memory: need {required} bytes, have {available} bytes")]
    InsufficientMemory { required: usize, available: usize },

    #[error("Graph contains a cycle")]
    CycleDetected,

    #[error("No output nodes found in graph")]
    NoOutputNodes,

    #[error("Unreachable node: {0}")]
    UnreachableNode(NodeId),

    #[error("{0}")]
    Other(String),
}

/// Errors during graph execution.
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Node {node_id} execution failed: {error}")]
    NodeExecution { node_id: NodeId, error: String },

    #[error("Missing input '{port}' for node {node_id}")]
    MissingInput { node_id: NodeId, port: String },

    #[error("Missing parameter '{parameter}' for node {node_id}")]
    MissingParameter { node_id: NodeId, parameter: String },

    #[error("Output '{port}' was not set by node {node_id}")]
    OutputNotSet { node_id: NodeId, port: String },

    #[error("Script error in node {node_id}: {error}")]
    ScriptError { node_id: NodeId, error: String },

    #[error("Out of memory during execution")]
    OutOfMemory,

    #[error("Execution cancelled by user")]
    Cancelled,

    #[error("Timeout after {duration_secs} seconds")]
    Timeout { duration_secs: u64 },

    #[error("Image processing error: {0}")]
    ImageProcessing(String),

    #[error("{0}")]
    Other(String),
}

/// Errors from the plugin system.
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Failed to load plugin from {path}: {error}")]
    LoadFailed { path: String, error: String },

    #[error("Plugin version mismatch: host expects API v{host_version}, plugin has v{plugin_version}")]
    IncompatibleVersion {
        host_version: u32,
        plugin_version: u32,
    },

    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Plugin '{name}' not found")]
    PluginNotFound { name: String },

    #[error("Plugin '{name}' is already loaded")]
    AlreadyLoaded { name: String },

    #[error("Script compilation error: {0}")]
    ScriptCompilation(String),

    #[error("Plugin execution error: {0}")]
    Execution(String),
}

/// Errors during batch processing.
#[derive(Error, Debug)]
pub enum BatchError {
    #[error("Batch validation failed: {0}")]
    ValidationFailed(ValidationError),

    #[error("Failed to process item {index}: {error}")]
    ItemFailed { index: usize, error: ExecutionError },

    #[error("No input files found matching pattern: {pattern}")]
    NoInputsFound { pattern: String },

    #[error("Output directory does not exist: {path}")]
    OutputDirectoryMissing { path: String },

    #[error("Batch cancelled after processing {completed}/{total} items")]
    Cancelled { completed: usize, total: usize },

    #[error("{0}")]
    Other(String),
}

// ============================================================================
// Error Utilities
// ============================================================================

impl ValidationError {
    /// Check if this is a fatal error that should stop validation.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            ValidationError::CycleDetected
                | ValidationError::InsufficientMemory { .. }
                | ValidationError::NoOutputNodes
        )
    }

    /// Get suggestion for fixing this error.
    pub fn suggested_fix(&self) -> Option<String> {
        match self {
            ValidationError::TypeMismatch { expected, got } => Some(format!(
                "Insert a conversion node to convert {} to {}",
                got, expected
            )),
            ValidationError::MissingRequiredInput { port, .. } => {
                Some(format!("Connect an output to the '{}' input", port))
            }
            ValidationError::ResourceNotFound { resource, .. } => {
                Some(format!("Check that the file '{}' exists", resource))
            }
            ValidationError::ConstraintViolation { parameter, error, .. } => {
                Some(format!("Adjust '{}': {}", parameter, error))
            }
            _ => None,
        }
    }

    /// Get list of affected node IDs.
    pub fn affected_nodes(&self) -> Vec<NodeId> {
        match self {
            ValidationError::MissingRequiredInput { node_id, .. }
            | ValidationError::ConstraintViolation { node_id, .. }
            | ValidationError::CustomValidation { node_id, .. }
            | ValidationError::ResourceNotFound { node_id, .. }
            | ValidationError::UnreachableNode(node_id) => vec![*node_id],
            _ => vec![],
        }
    }
}

impl ExecutionError {
    /// Get the node ID that caused this error, if applicable.
    pub fn node_id(&self) -> Option<NodeId> {
        match self {
            ExecutionError::NodeExecution { node_id, .. }
            | ExecutionError::MissingInput { node_id, .. }
            | ExecutionError::MissingParameter { node_id, .. }
            | ExecutionError::OutputNotSet { node_id, .. }
            | ExecutionError::ScriptError { node_id, .. } => Some(*node_id),
            _ => None,
        }
    }

    /// Check if this error is recoverable (can continue with other items).
    pub fn is_recoverable(&self) -> bool {
        !matches!(
            self,
            ExecutionError::OutOfMemory | ExecutionError::Cancelled | ExecutionError::Timeout { .. }
        )
    }
}

/// Result type alias for Ambara operations.
pub type AmbaraResult<T> = Result<T, AmbaraError>;

/// Result type alias for graph operations.
pub type GraphResult<T> = Result<T, GraphError>;

/// Result type alias for validation operations.
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Result type alias for execution operations.
pub type ExecutionResult<T> = Result<T, ExecutionError>;

// ============================================================================
// Validation Report
// ============================================================================

/// Comprehensive validation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether validation passed without errors.
    pub success: bool,
    /// List of errors found.
    pub errors: Vec<ValidationError>,
    /// List of warnings (non-fatal issues).
    pub warnings: Vec<ValidationWarning>,
    /// Time taken for validation in milliseconds.
    pub duration_ms: u64,
}

/// Non-fatal validation warning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning message.
    pub message: String,
    /// Node that triggered the warning, if applicable.
    pub node_id: Option<NodeId>,
    /// Suggestion for addressing the warning.
    pub suggestion: Option<String>,
}

impl ValidationReport {
    /// Create a new empty report (success).
    pub fn new() -> Self {
        Self {
            success: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Add an error to the report.
    pub fn add_error(&mut self, error: ValidationError) {
        self.success = false;
        self.errors.push(error);
    }

    /// Add a warning to the report.
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Check if the graph can be executed.
    pub fn can_execute(&self) -> bool {
        self.success
    }

    /// Get a human-readable summary.
    pub fn summary(&self) -> String {
        if self.success {
            if self.warnings.is_empty() {
                "✓ Graph is valid and ready to execute".to_string()
            } else {
                format!(
                    "✓ Graph is valid with {} warning(s)",
                    self.warnings.len()
                )
            }
        } else {
            format!(
                "✗ Validation failed with {} error(s)",
                self.errors.len()
            )
        }
    }

    /// Get detailed error messages with suggestions.
    pub fn detailed_errors(&self) -> Vec<String> {
        self.errors
            .iter()
            .enumerate()
            .map(|(i, error)| {
                let mut msg = format!("{}. {}", i + 1, error);
                if let Some(fix) = error.suggested_fix() {
                    msg.push_str(&format!("\n   → Suggestion: {}", fix));
                }
                msg
            })
            .collect()
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_display() {
        let id = NodeId::new();
        let display = format!("{}", id);
        assert_eq!(display.len(), 8);
    }

    #[test]
    fn test_validation_error_suggestions() {
        let error = ValidationError::MissingRequiredInput {
            node_id: NodeId::new(),
            port: "image".to_string(),
        };
        assert!(error.suggested_fix().is_some());
        assert!(error.suggested_fix().unwrap().contains("image"));
    }

    #[test]
    fn test_validation_report() {
        let mut report = ValidationReport::new();
        assert!(report.can_execute());

        report.add_error(ValidationError::CycleDetected);
        assert!(!report.can_execute());
        assert_eq!(report.errors.len(), 1);
    }
}
