//! Execution and validation contexts.
//!
//! Contexts provide access to inputs, parameters, and outputs during
//! node validation and execution. They encapsulate the data flow.

use crate::core::error::{ExecutionError, NodeId, ValidationError};
use crate::core::types::{Color, ImageValue, Value};
use std::collections::HashMap;

/// Context provided during node validation.
///
/// ValidationContext contains metadata about inputs (without necessarily
/// loading the full data) to allow validation before execution begins.
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// ID of the node being validated.
    pub node_id: NodeId,
    /// Input values or metadata.
    inputs: HashMap<String, Value>,
    /// Parameter values.
    parameters: HashMap<String, Value>,
}

impl ValidationContext {
    /// Create a new validation context.
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            inputs: HashMap::new(),
            parameters: HashMap::new(),
        }
    }

    /// Add an input value to the context.
    pub fn add_input(&mut self, name: impl Into<String>, value: Value) {
        self.inputs.insert(name.into(), value);
    }

    /// Add a parameter value to the context.
    pub fn add_parameter(&mut self, name: impl Into<String>, value: Value) {
        self.parameters.insert(name.into(), value);
    }

    /// Get all inputs.
    pub fn inputs(&self) -> &HashMap<String, Value> {
        &self.inputs
    }

    /// Get all parameters.
    pub fn parameters(&self) -> &HashMap<String, Value> {
        &self.parameters
    }

    // ========================================================================
    // Input Getters
    // ========================================================================

    /// Get an input value by name.
    pub fn get_input(&self, name: &str) -> Result<&Value, ValidationError> {
        self.inputs.get(name).ok_or_else(|| ValidationError::MissingRequiredInput {
            node_id: self.node_id,
            port: name.to_string(),
        })
    }

    /// Get an input as an image.
    pub fn get_input_image(&self, name: &str) -> Result<&ImageValue, ValidationError> {
        self.get_input(name)?
            .as_image()
            .ok_or_else(|| ValidationError::TypeMismatch {
                expected: crate::core::types::PortType::Image,
                got: self.inputs.get(name).map(|v| v.get_type()).unwrap_or(crate::core::types::PortType::Any),
            })
    }

    /// Check if an input exists.
    pub fn has_input(&self, name: &str) -> bool {
        self.inputs.contains_key(name)
    }

    // ========================================================================
    // Parameter Getters
    // ========================================================================

    /// Get a parameter value by name.
    pub fn get_parameter(&self, name: &str) -> Result<&Value, ValidationError> {
        self.parameters.get(name).ok_or_else(|| ValidationError::ConstraintViolation {
            node_id: self.node_id,
            parameter: name.to_string(),
            error: "Parameter not set".to_string(),
        })
    }

    /// Get a parameter as an integer.
    pub fn get_integer(&self, name: &str) -> Result<i64, ValidationError> {
        self.get_parameter(name)?
            .as_integer()
            .ok_or_else(|| ValidationError::TypeMismatch {
                expected: crate::core::types::PortType::Integer,
                got: self.parameters.get(name).map(|v| v.get_type()).unwrap_or(crate::core::types::PortType::Any),
            })
    }

    /// Get a parameter as a float.
    pub fn get_float(&self, name: &str) -> Result<f64, ValidationError> {
        self.get_parameter(name)?
            .as_float()
            .ok_or_else(|| ValidationError::TypeMismatch {
                expected: crate::core::types::PortType::Float,
                got: self.parameters.get(name).map(|v| v.get_type()).unwrap_or(crate::core::types::PortType::Any),
            })
    }

    /// Get a parameter as a string.
    pub fn get_string(&self, name: &str) -> Result<&str, ValidationError> {
        self.get_parameter(name)?
            .as_string()
            .ok_or_else(|| ValidationError::TypeMismatch {
                expected: crate::core::types::PortType::String,
                got: self.parameters.get(name).map(|v| v.get_type()).unwrap_or(crate::core::types::PortType::Any),
            })
    }

    /// Get a parameter as a boolean.
    pub fn get_bool(&self, name: &str) -> Result<bool, ValidationError> {
        self.get_parameter(name)?
            .as_bool()
            .ok_or_else(|| ValidationError::TypeMismatch {
                expected: crate::core::types::PortType::Boolean,
                got: self.parameters.get(name).map(|v| v.get_type()).unwrap_or(crate::core::types::PortType::Any),
            })
    }

    /// Get a parameter as a color.
    pub fn get_color(&self, name: &str) -> Result<Color, ValidationError> {
        self.get_parameter(name)?
            .as_color()
            .ok_or_else(|| ValidationError::TypeMismatch {
                expected: crate::core::types::PortType::Color,
                got: self.parameters.get(name).map(|v| v.get_type()).unwrap_or(crate::core::types::PortType::Any),
            })
    }

    /// Check if a parameter exists.
    pub fn has_parameter(&self, name: &str) -> bool {
        self.parameters.contains_key(name)
    }
}

/// Context provided during node execution.
///
/// ExecutionContext contains actual data values and allows nodes to
/// set their output values.
#[derive(Debug)]
pub struct ExecutionContext {
    /// ID of the node being executed.
    pub node_id: NodeId,
    /// Input values.
    inputs: HashMap<String, Value>,
    /// Parameter values.
    parameters: HashMap<String, Value>,
    /// Output values set by the node.
    outputs: HashMap<String, Value>,
    /// Current progress (0.0 to 1.0).
    progress: f32,
    /// Whether execution should be cancelled.
    cancelled: bool,
    /// Memory limit in bytes for processing.
    memory_limit: usize,
    /// Whether auto-chunking is enabled for large images.
    auto_chunk: bool,
    /// Preferred tile size for chunked processing.
    tile_size: (u32, u32),
}

impl ExecutionContext {
    /// Create a new execution context.
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            inputs: HashMap::new(),
            parameters: HashMap::new(),
            outputs: HashMap::new(),
            progress: 0.0,
            cancelled: false,
            memory_limit: 500 * 1024 * 1024, // 500MB default
            auto_chunk: true,
            tile_size: (512, 512),
        }
    }

    /// Create a new execution context with memory settings.
    pub fn with_memory_settings(
        node_id: NodeId,
        memory_limit: usize,
        auto_chunk: bool,
        tile_size: (u32, u32),
    ) -> Self {
        Self {
            node_id,
            inputs: HashMap::new(),
            parameters: HashMap::new(),
            outputs: HashMap::new(),
            progress: 0.0,
            cancelled: false,
            memory_limit,
            auto_chunk,
            tile_size,
        }
    }

    /// Add an input value to the context.
    pub fn add_input(&mut self, name: impl Into<String>, value: Value) {
        self.inputs.insert(name.into(), value);
    }

    /// Add a parameter value to the context.
    pub fn add_parameter(&mut self, name: impl Into<String>, value: Value) {
        self.parameters.insert(name.into(), value);
    }

    /// Get all inputs.
    pub fn inputs(&self) -> &HashMap<String, Value> {
        &self.inputs
    }

    /// Get all parameters.
    pub fn parameters(&self) -> &HashMap<String, Value> {
        &self.parameters
    }

    /// Get all outputs.
    pub fn outputs(&self) -> &HashMap<String, Value> {
        &self.outputs
    }

    /// Take ownership of all outputs.
    pub fn take_outputs(self) -> HashMap<String, Value> {
        self.outputs
    }

    // ========================================================================
    // Input Getters
    // ========================================================================

    /// Get an input value by name.
    pub fn get_input(&self, name: &str) -> Result<&Value, ExecutionError> {
        self.inputs.get(name).ok_or_else(|| ExecutionError::MissingInput {
            node_id: self.node_id,
            port: name.to_string(),
        })
    }

    /// Get an input as an image.
    pub fn get_input_image(&self, name: &str) -> Result<&ImageValue, ExecutionError> {
        self.get_input(name)?
            .as_image()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: self.node_id,
                error: format!("Input '{}' is not an image", name),
            })
    }

    /// Get an optional input as an image.
    /// Returns None if the input doesn't exist, or Some(&ImageValue) if it does.
    pub fn get_input_image_optional(&self, name: &str) -> Option<&ImageValue> {
        self.inputs.get(name).and_then(|v| v.as_image())
    }

    /// Get an input as a mutable image.
    pub fn get_input_image_mut(&mut self, name: &str) -> Result<&mut ImageValue, ExecutionError> {
        let node_id = self.node_id;
        self.inputs
            .get_mut(name)
            .ok_or_else(|| ExecutionError::MissingInput {
                node_id,
                port: name.to_string(),
            })?
            .as_image_mut()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id,
                error: format!("Input '{}' is not an image", name),
            })
    }

    /// Take ownership of an input value.
    pub fn take_input(&mut self, name: &str) -> Result<Value, ExecutionError> {
        self.inputs.remove(name).ok_or_else(|| ExecutionError::MissingInput {
            node_id: self.node_id,
            port: name.to_string(),
        })
    }

    /// Take ownership of an input as an ImageValue.
    pub fn take_input_image(&mut self, name: &str) -> Result<ImageValue, ExecutionError> {
        let value = self.take_input(name)?;
        match value {
            Value::Image(img) => Ok(img),
            _ => Err(ExecutionError::NodeExecution {
                node_id: self.node_id,
                error: format!("Input '{}' is not an image", name),
            }),
        }
    }

    /// Check if an input exists.
    pub fn has_input(&self, name: &str) -> bool {
        self.inputs.contains_key(name)
    }

    // ========================================================================
    // Parameter Getters
    // ========================================================================

    /// Get a parameter value by name.
    pub fn get_parameter(&self, name: &str) -> Result<&Value, ExecutionError> {
        self.parameters.get(name).ok_or_else(|| ExecutionError::MissingParameter {
            node_id: self.node_id,
            parameter: name.to_string(),
        })
    }

    /// Get a parameter as an integer.
    pub fn get_integer(&self, name: &str) -> Result<i64, ExecutionError> {
        self.get_parameter(name)?
            .as_integer()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: self.node_id,
                error: format!("Parameter '{}' is not an integer", name),
            })
    }

    /// Get a parameter as a float.
    pub fn get_float(&self, name: &str) -> Result<f64, ExecutionError> {
        self.get_parameter(name)?
            .as_float()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: self.node_id,
                error: format!("Parameter '{}' is not a float", name),
            })
    }

    /// Get a parameter as a string.
    pub fn get_string(&self, name: &str) -> Result<&str, ExecutionError> {
        self.get_parameter(name)?
            .as_string()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: self.node_id,
                error: format!("Parameter '{}' is not a string", name),
            })
    }

    /// Get a parameter as a boolean.
    pub fn get_bool(&self, name: &str) -> Result<bool, ExecutionError> {
        self.get_parameter(name)?
            .as_bool()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: self.node_id,
                error: format!("Parameter '{}' is not a boolean", name),
            })
    }

    /// Get a parameter as a color.
    pub fn get_color(&self, name: &str) -> Result<Color, ExecutionError> {
        self.get_parameter(name)?
            .as_color()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: self.node_id,
                error: format!("Parameter '{}' is not a color", name),
            })
    }

    /// Get a parameter as a 2D vector.
    pub fn get_vector2(&self, name: &str) -> Result<(f64, f64), ExecutionError> {
        self.get_parameter(name)?
            .as_vector2()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: self.node_id,
                error: format!("Parameter '{}' is not a vector2", name),
            })
    }

    /// Check if a parameter exists.
    pub fn has_parameter(&self, name: &str) -> bool {
        self.parameters.contains_key(name)
    }

    // ========================================================================
    // Output Setters
    // ========================================================================

    /// Set an output value.
    pub fn set_output(&mut self, name: impl Into<String>, value: Value) -> Result<(), ExecutionError> {
        self.outputs.insert(name.into(), value);
        Ok(())
    }

    /// Set an output image value.
    pub fn set_output_image(&mut self, name: impl Into<String>, image: ImageValue) -> Result<(), ExecutionError> {
        self.set_output(name, Value::Image(image))
    }

    /// Check if an output has been set.
    pub fn has_output(&self, name: &str) -> bool {
        self.outputs.contains_key(name)
    }

    // ========================================================================
    // Memory Settings
    // ========================================================================

    /// Get the memory limit in bytes.
    pub fn memory_limit(&self) -> usize {
        self.memory_limit
    }

    /// Get the memory limit in megabytes.
    pub fn memory_limit_mb(&self) -> usize {
        self.memory_limit / (1024 * 1024)
    }

    /// Check if auto-chunking is enabled.
    pub fn auto_chunk(&self) -> bool {
        self.auto_chunk
    }

    /// Get the preferred tile size for chunked processing.
    pub fn tile_size(&self) -> (u32, u32) {
        self.tile_size
    }

    /// Check if an image of given dimensions would need chunked processing.
    /// Returns true if the image memory footprint exceeds half the memory limit.
    pub fn needs_chunking(&self, width: u32, height: u32) -> bool {
        const BYTES_PER_PIXEL: usize = 4;
        let image_size = (width as usize) * (height as usize) * BYTES_PER_PIXEL;
        image_size > self.memory_limit / 2
    }

    /// Calculate the memory needed for an image of given dimensions.
    pub fn calculate_image_memory(width: u32, height: u32) -> usize {
        const BYTES_PER_PIXEL: usize = 4;
        (width as usize) * (height as usize) * BYTES_PER_PIXEL
    }

    // ========================================================================
    // Progress and Cancellation
    // ========================================================================

    /// Set the current progress (0.0 to 1.0).
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    /// Get the current progress.
    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// Mark execution as cancelled.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Check if execution should be cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Check cancellation and return error if cancelled.
    pub fn check_cancelled(&self) -> Result<(), ExecutionError> {
        if self.cancelled {
            Err(ExecutionError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// Convert ValidationContext to ExecutionContext.
impl From<ValidationContext> for ExecutionContext {
    fn from(val_ctx: ValidationContext) -> Self {
        let mut exec_ctx = ExecutionContext::new(val_ctx.node_id);
        for (name, value) in val_ctx.inputs {
            exec_ctx.add_input(name, value);
        }
        for (name, value) in val_ctx.parameters {
            exec_ctx.add_parameter(name, value);
        }
        exec_ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Value;

    #[test]
    fn test_validation_context_inputs() {
        let mut ctx = ValidationContext::new(NodeId::new());
        ctx.add_input("test", Value::Integer(42));

        assert!(ctx.has_input("test"));
        assert!(!ctx.has_input("nonexistent"));
        assert_eq!(ctx.get_integer("test").ok(), None); // integer is in inputs, not parameters
    }

    #[test]
    fn test_validation_context_parameters() {
        let mut ctx = ValidationContext::new(NodeId::new());
        ctx.add_parameter("radius", Value::Float(5.0));

        assert!(ctx.has_parameter("radius"));
        assert_eq!(ctx.get_float("radius").unwrap(), 5.0);
    }

    #[test]
    fn test_execution_context_outputs() {
        let mut ctx = ExecutionContext::new(NodeId::new());
        ctx.set_output("result", Value::Integer(100)).unwrap();

        assert!(ctx.has_output("result"));
        let outputs = ctx.take_outputs();
        assert_eq!(outputs.get("result"), Some(&Value::Integer(100)));
    }

    #[test]
    fn test_execution_context_progress() {
        let mut ctx = ExecutionContext::new(NodeId::new());
        
        ctx.set_progress(0.5);
        assert_eq!(ctx.progress(), 0.5);

        ctx.set_progress(1.5); // Should clamp to 1.0
        assert_eq!(ctx.progress(), 1.0);

        ctx.set_progress(-0.5); // Should clamp to 0.0
        assert_eq!(ctx.progress(), 0.0);
    }

    #[test]
    fn test_execution_context_cancellation() {
        let mut ctx = ExecutionContext::new(NodeId::new());
        
        assert!(!ctx.is_cancelled());
        assert!(ctx.check_cancelled().is_ok());

        ctx.cancel();
        assert!(ctx.is_cancelled());
        assert!(ctx.check_cancelled().is_err());
    }
}
