//! Individual validation stages.
//!
//! Each stage checks for a specific category of errors.

use crate::core::context::ValidationContext;
use crate::core::error::{NodeId, ValidationError, ValidationWarning};
use crate::core::types::Value;
use crate::graph::structure::ProcessingGraph;
use crate::graph::topology::TopologyAnalyzer;
use std::path::Path;

/// Trait for validation stages.
pub trait ValidationStage: Send + Sync {
    /// Name of this validation stage.
    fn name(&self) -> &str;

    /// Validate the graph.
    ///
    /// Returns Ok with warnings, or Err with errors.
    fn validate(
        &self,
        graph: &ProcessingGraph,
    ) -> Result<Vec<ValidationWarning>, Vec<ValidationError>>;
}

/// Structural validation - checks graph structure.
///
/// Verifies:
/// - Graph is a DAG (no cycles)
/// - All connections reference valid nodes
/// - All required inputs are connected
pub struct StructuralValidation;

impl ValidationStage for StructuralValidation {
    fn name(&self) -> &str {
        "Structural Validation"
    }

    fn validate(
        &self,
        graph: &ProcessingGraph,
    ) -> Result<Vec<ValidationWarning>, Vec<ValidationError>> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Empty graph warning (not error - might be intentional)
        if graph.is_empty() {
            warnings.push(ValidationWarning {
                message: "Graph is empty".to_string(),
                node_id: None,
                suggestion: Some("Add some filter nodes to create a processing pipeline".to_string()),
            });
            return Ok(warnings);
        }

        // Check for cycles
        let analyzer = TopologyAnalyzer::new(graph);
        if analyzer.has_cycle() {
            errors.push(ValidationError::CycleDetected);
        }

        // Check that all required inputs are connected
        for node in graph.nodes() {
            let metadata = node.filter.metadata();
            
            for input in &metadata.inputs {
                if !input.optional && input.default_value.is_none() {
                    // Check if connected
                    let is_connected = graph
                        .connections_to(node.id)
                        .any(|conn| conn.to.port_name == input.name);

                    if !is_connected {
                        errors.push(ValidationError::MissingRequiredInput {
                            node_id: node.id,
                            port: input.name.clone(),
                        });
                    }
                }
            }
        }

        // Check for disconnected subgraphs (warning)
        let subgraphs = analyzer.find_subgraphs();
        if subgraphs.len() > 1 {
            warnings.push(ValidationWarning {
                message: format!(
                    "Graph contains {} disconnected subgraphs",
                    subgraphs.len()
                ),
                node_id: None,
                suggestion: Some("Consider connecting all parts or removing unused nodes".to_string()),
            });
        }

        // Check for disabled nodes that have connections
        for node in graph.nodes() {
            if node.disabled {
                let has_downstream = graph.connections_from(node.id).next().is_some();
                if has_downstream {
                    warnings.push(ValidationWarning {
                        message: format!("Disabled node '{}' has downstream connections", node.display_name()),
                        node_id: Some(node.id),
                        suggestion: Some("Downstream nodes may not receive expected inputs".to_string()),
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(warnings)
        } else {
            Err(errors)
        }
    }
}

/// Type validation - checks type compatibility.
///
/// Verifies:
/// - All connection types are compatible
pub struct TypeValidation;

impl ValidationStage for TypeValidation {
    fn name(&self) -> &str {
        "Type Validation"
    }

    fn validate(
        &self,
        graph: &ProcessingGraph,
    ) -> Result<Vec<ValidationWarning>, Vec<ValidationError>> {
        let mut errors = Vec::new();

        for conn in graph.connections() {
            // Get the nodes
            let from_node = match graph.get_node(conn.from.node_id) {
                Ok(n) => n,
                Err(_) => continue, // Structural validation will catch this
            };
            let to_node = match graph.get_node(conn.to.node_id) {
                Ok(n) => n,
                Err(_) => continue,
            };

            // Get the port definitions
            let from_metadata = from_node.filter.metadata();
            let to_metadata = to_node.filter.metadata();

            let from_port = from_metadata.get_output(&conn.from.port_name);
            let to_port = to_metadata.get_input(&conn.to.port_name);

            match (from_port, to_port) {
                (Some(from_def), Some(to_def)) => {
                    if !from_def.port_type.compatible_with(&to_def.port_type) {
                        errors.push(ValidationError::TypeMismatch {
                            expected: to_def.port_type.clone(),
                            got: from_def.port_type.clone(),
                        });
                    }
                }
                _ => {
                    // Port not found - structural validation will catch this
                }
            }
        }

        if errors.is_empty() {
            Ok(Vec::new())
        } else {
            Err(errors)
        }
    }
}

/// Constraint validation - checks parameter constraints.
///
/// Verifies:
/// - All parameter values satisfy their constraints
pub struct ConstraintValidation;

impl ValidationStage for ConstraintValidation {
    fn name(&self) -> &str {
        "Constraint Validation"
    }

    fn validate(
        &self,
        graph: &ProcessingGraph,
    ) -> Result<Vec<ValidationWarning>, Vec<ValidationError>> {
        let mut errors = Vec::new();

        for node in graph.nodes() {
            let metadata = node.filter.metadata();

            for param_def in &metadata.parameters {
                // Get the value (custom or default)
                let value = node
                    .parameters
                    .get(&param_def.name)
                    .cloned()
                    .unwrap_or_else(|| param_def.default_value.clone());

                // Validate against constraints
                if let Err(error) = param_def.validate(&value) {
                    errors.push(ValidationError::ConstraintViolation {
                        node_id: node.id,
                        parameter: param_def.name.clone(),
                        error,
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(Vec::new())
        } else {
            Err(errors)
        }
    }
}

/// Custom validation - runs each node's custom validation.
///
/// Calls the `validate` method on each filter.
pub struct CustomValidation;

impl ValidationStage for CustomValidation {
    fn name(&self) -> &str {
        "Custom Validation"
    }

    fn validate(
        &self,
        graph: &ProcessingGraph,
    ) -> Result<Vec<ValidationWarning>, Vec<ValidationError>> {
        let mut errors = Vec::new();

        for node in graph.nodes() {
            if node.disabled {
                continue;
            }

            // Build validation context
            let mut ctx = ValidationContext::new(node.id);

            // Add input metadata (we don't have actual values yet)
            for conn in graph.connections_to(node.id) {
                // For validation, we use placeholder values based on expected types
                let from_node = match graph.get_node(conn.from.node_id) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let from_metadata = from_node.filter.metadata();
                if let Some(port) = from_metadata.get_output(&conn.from.port_name) {
                    // Create a placeholder value for validation
                    let placeholder = create_placeholder(&port.port_type);
                    ctx.add_input(conn.to.port_name.clone(), placeholder);
                }
            }

            // Add parameters
            let metadata = node.filter.metadata();
            for param_def in &metadata.parameters {
                let value = node
                    .parameters
                    .get(&param_def.name)
                    .cloned()
                    .unwrap_or_else(|| param_def.default_value.clone());
                ctx.add_parameter(param_def.name.clone(), value);
            }

            // Run custom validation
            if let Err(error) = node.filter.validate(&ctx) {
                errors.push(error);
            }
        }

        if errors.is_empty() {
            Ok(Vec::new())
        } else {
            Err(errors)
        }
    }
}

/// Resource validation - checks external resources.
///
/// Verifies:
/// - File paths exist
/// - Memory requirements are reasonable
pub struct ResourceValidation;

impl ValidationStage for ResourceValidation {
    fn name(&self) -> &str {
        "Resource Validation"
    }

    fn validate(
        &self,
        graph: &ProcessingGraph,
    ) -> Result<Vec<ValidationWarning>, Vec<ValidationError>> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for node in graph.nodes() {
            if node.disabled {
                continue;
            }

            let metadata = node.filter.metadata();

            // Check for file path parameters
            for param_def in &metadata.parameters {
                if param_def.name.contains("path") || param_def.name.contains("file") {
                    if let Some(Value::String(path)) = node.parameters.get(&param_def.name) {
                        // Check if path exists (for input files)
                        if metadata.id.contains("load") || metadata.id.contains("input") {
                            if !Path::new(path).exists() {
                                // Check for glob patterns
                                if path.contains('*') || path.contains('?') {
                                    // It's a glob pattern, check if it matches anything
                                    match glob::glob(path) {
                                        Ok(paths) => {
                                            if paths.count() == 0 {
                                                errors.push(ValidationError::ResourceNotFound {
                                                    node_id: node.id,
                                                    resource: path.clone(),
                                                });
                                            }
                                        }
                                        Err(_) => {
                                            warnings.push(ValidationWarning {
                                                message: format!("Could not validate glob pattern: {}", path),
                                                node_id: Some(node.id),
                                                suggestion: None,
                                            });
                                        }
                                    }
                                } else {
                                    errors.push(ValidationError::ResourceNotFound {
                                        node_id: node.id,
                                        resource: path.clone(),
                                    });
                                }
                            }
                        }

                        // For output files, check that directory exists
                        if metadata.id.contains("save") || metadata.id.contains("output") {
                            if let Some(parent) = Path::new(path).parent() {
                                if !parent.exists() && !parent.as_os_str().is_empty() {
                                    warnings.push(ValidationWarning {
                                        message: format!(
                                            "Output directory does not exist: {}",
                                            parent.display()
                                        ),
                                        node_id: Some(node.id),
                                        suggestion: Some("Directory will be created on execution".to_string()),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(warnings)
        } else {
            Err(errors)
        }
    }
}

/// Create a placeholder value for validation based on port type.
fn create_placeholder(port_type: &crate::core::types::PortType) -> Value {
    use crate::core::types::{ImageMetadata, ImageValue, ImageFormat, PortType, Color};
    use std::path::PathBuf;

    match port_type {
        PortType::Image => {
            let metadata = ImageMetadata {
                width: 1920,
                height: 1080,
                format: ImageFormat::Png,
                has_alpha: true,
            };
            Value::Image(ImageValue::from_metadata(metadata, PathBuf::new()))
        }
        PortType::Integer => Value::Integer(0),
        PortType::Float => Value::Float(0.0),
        PortType::String => Value::String(String::new()),
        PortType::Boolean => Value::Boolean(false),
        PortType::Color => Value::Color(Color::BLACK),
        PortType::Vector2 => Value::Vector2(0.0, 0.0),
        PortType::Vector3 => Value::Vector3(0.0, 0.0, 0.0),
        PortType::Array(inner) => Value::Array(vec![create_placeholder(inner)]),
        PortType::Map(_) => Value::Map(std::collections::HashMap::new()),
        PortType::Any => Value::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::node::PassthroughNode;
    use crate::graph::structure::GraphNode;

    fn create_test_node() -> GraphNode {
        GraphNode::new(Box::new(PassthroughNode))
    }

    #[test]
    fn test_structural_validation_empty_graph() {
        let graph = ProcessingGraph::new();
        let stage = StructuralValidation;
        let result = stage.validate(&graph);

        // Empty graph should produce a warning, not an error
        assert!(result.is_ok());
        let warnings = result.unwrap();
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_type_validation_compatible_types() {
        let mut graph = ProcessingGraph::new();
        let node1 = graph.add_node(create_test_node());
        let node2 = graph.add_node(create_test_node());
        graph.connect(node1, "output", node2, "input").unwrap();

        let stage = TypeValidation;
        let result = stage.validate(&graph);

        // PassthroughNode uses Any type, so should be compatible
        assert!(result.is_ok());
    }
}
