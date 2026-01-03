//! FilterNode trait and node metadata.
//!
//! The FilterNode trait is the core abstraction for all image processing
//! operations. It uses a two-phase design: validation (before execution)
//! and execution (processing).

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::port::{ParameterDefinition, PortDefinition};
use crate::core::types::Color;
use serde::{Deserialize, Serialize};

/// Category for organizing filters in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Input nodes (load images, folders, etc.)
    Input,
    /// Output nodes (save, preview, etc.)
    Output,
    /// Basic transformations (resize, crop, rotate)
    Transform,
    /// Color adjustments (brightness, contrast, etc.)
    Adjust,
    /// Blur effects
    Blur,
    /// Sharpening effects
    Sharpen,
    /// Edge detection and effects
    Edge,
    /// Noise operations
    Noise,
    /// Drawing operations
    Draw,
    /// Text operations
    Text,
    /// Compositing operations
    Composite,
    /// Color manipulation
    Color,
    /// Analysis and measurement
    Analyze,
    /// Mathematical operations
    Math,
    /// Utility nodes
    Utility,
    /// Custom/user-defined
    Custom,
}

impl Category {
    /// Get the display name for this category.
    pub fn display_name(&self) -> &'static str {
        match self {
            Category::Input => "Input",
            Category::Output => "Output",
            Category::Transform => "Transform",
            Category::Adjust => "Adjust",
            Category::Blur => "Blur",
            Category::Sharpen => "Sharpen",
            Category::Edge => "Edge",
            Category::Noise => "Noise",
            Category::Draw => "Draw",
            Category::Text => "Text",
            Category::Composite => "Composite",
            Category::Color => "Color",
            Category::Analyze => "Analyze",
            Category::Math => "Math",
            Category::Utility => "Utility",
            Category::Custom => "Custom",
        }
    }

    /// Get all categories in display order.
    pub fn all() -> &'static [Category] {
        &[
            Category::Input,
            Category::Output,
            Category::Transform,
            Category::Adjust,
            Category::Blur,
            Category::Sharpen,
            Category::Edge,
            Category::Noise,
            Category::Draw,
            Category::Text,
            Category::Composite,
            Category::Color,
            Category::Analyze,
            Category::Utility,
            Category::Custom,
        ]
    }
}

impl Default for Category {
    fn default() -> Self {
        Category::Custom
    }
}

/// Metadata describing a filter node.
///
/// This struct contains all information needed to:
/// - Display the node in the UI
/// - Validate connections
/// - Generate property panels
/// - Document the filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// Unique identifier for this filter type (e.g., "gaussian_blur")
    pub id: String,
    /// Human-readable name (e.g., "Gaussian Blur")
    pub name: String,
    /// Category for UI organization
    pub category: Category,
    /// Detailed description
    pub description: String,
    /// Version string
    pub version: String,
    /// Author or source
    pub author: String,

    /// Input port definitions
    pub inputs: Vec<PortDefinition>,
    /// Output port definitions
    pub outputs: Vec<PortDefinition>,
    /// Parameter definitions
    pub parameters: Vec<ParameterDefinition>,

    /// Searchable tags
    pub tags: Vec<String>,
    /// Optional color hint for UI
    pub color: Option<Color>,
    /// Whether this filter supports progress reporting
    pub supports_progress: bool,
    /// Whether this filter is deterministic (same inputs always give same outputs)
    pub deterministic: bool,
}

impl NodeMetadata {
    /// Create a new metadata builder.
    pub fn builder(id: impl Into<String>, name: impl Into<String>) -> NodeMetadataBuilder {
        NodeMetadataBuilder::new(id, name)
    }

    /// Get all input port names.
    pub fn input_names(&self) -> Vec<&str> {
        self.inputs.iter().map(|p| p.name.as_str()).collect()
    }

    /// Get all output port names.
    pub fn output_names(&self) -> Vec<&str> {
        self.outputs.iter().map(|p| p.name.as_str()).collect()
    }

    /// Get all parameter names.
    pub fn parameter_names(&self) -> Vec<&str> {
        self.parameters.iter().map(|p| p.name.as_str()).collect()
    }

    /// Find an input port by name.
    pub fn get_input(&self, name: &str) -> Option<&PortDefinition> {
        self.inputs.iter().find(|p| p.name == name)
    }

    /// Find an output port by name.
    pub fn get_output(&self, name: &str) -> Option<&PortDefinition> {
        self.outputs.iter().find(|p| p.name == name)
    }

    /// Find a parameter by name.
    pub fn get_parameter(&self, name: &str) -> Option<&ParameterDefinition> {
        self.parameters.iter().find(|p| p.name == name)
    }
}

/// Builder for NodeMetadata.
pub struct NodeMetadataBuilder {
    id: String,
    name: String,
    category: Category,
    description: String,
    version: String,
    author: String,
    inputs: Vec<PortDefinition>,
    outputs: Vec<PortDefinition>,
    parameters: Vec<ParameterDefinition>,
    tags: Vec<String>,
    color: Option<Color>,
    supports_progress: bool,
    deterministic: bool,
}

impl NodeMetadataBuilder {
    /// Create a new builder with required fields.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: Category::Custom,
            description: String::new(),
            version: "1.0.0".to_string(),
            author: "Ambara".to_string(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            parameters: Vec::new(),
            tags: Vec::new(),
            color: None,
            supports_progress: false,
            deterministic: true,
        }
    }

    /// Set the category.
    pub fn category(mut self, category: Category) -> Self {
        self.category = category;
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the author.
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    /// Add an input port.
    pub fn input(mut self, port: PortDefinition) -> Self {
        self.inputs.push(port);
        self
    }

    /// Add an output port.
    pub fn output(mut self, port: PortDefinition) -> Self {
        self.outputs.push(port);
        self
    }

    /// Add a parameter.
    pub fn parameter(mut self, param: ParameterDefinition) -> Self {
        self.parameters.push(param);
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags.
    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Set the color hint.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Mark as supporting progress reporting.
    pub fn supports_progress(mut self) -> Self {
        self.supports_progress = true;
        self
    }

    /// Mark as non-deterministic.
    pub fn non_deterministic(mut self) -> Self {
        self.deterministic = false;
        self
    }

    /// Build the metadata.
    pub fn build(self) -> NodeMetadata {
        NodeMetadata {
            id: self.id,
            name: self.name,
            category: self.category,
            description: self.description,
            version: self.version,
            author: self.author,
            inputs: self.inputs,
            outputs: self.outputs,
            parameters: self.parameters,
            tags: self.tags,
            color: self.color,
            supports_progress: self.supports_progress,
            deterministic: self.deterministic,
        }
    }
}

/// The core trait for filter nodes.
///
/// # Design
///
/// The trait uses a two-phase design:
///
/// 1. **Validation Phase** (`validate`): Called once before execution begins.
///    Checks that all inputs and parameters are valid. This allows catching
///    errors early before wasting time on batch processing.
///
/// 2. **Execution Phase** (`execute`): Called once per image to actually
///    perform the processing.
///
/// # Thread Safety
///
/// `Send + Sync` bounds enable parallel execution across threads.
///
/// # Example Implementation
///
/// ```ignore
/// struct GaussianBlur;
///
/// impl FilterNode for GaussianBlur {
///     fn metadata(&self) -> NodeMetadata {
///         NodeMetadata::builder("gaussian_blur", "Gaussian Blur")
///             .category(Category::Blur)
///             .description("Apply Gaussian blur to an image")
///             .input(PortDefinition::input("image", PortType::Image))
///             .output(PortDefinition::output("result", PortType::Image))
///             .parameter(
///                 ParameterDefinition::new("radius", PortType::Float, Value::Float(5.0))
///                     .with_range(0.1, 100.0)
///             )
///             .build()
///     }
///
///     fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
///         // Check radius is reasonable for the image size
///         let image = ctx.get_input_image("image")?;
///         let radius = ctx.get_float("radius")?;
///         if radius > image.metadata.width as f64 / 2.0 {
///             return Err(ValidationError::ConstraintViolation {
///                 node_id: ctx.node_id,
///                 parameter: "radius".to_string(),
///                 error: "Radius too large for image".to_string(),
///             });
///         }
///         Ok(())
///     }
///
///     fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
///         let image = ctx.take_input_image("image")?;
///         let radius = ctx.get_float("radius")?;
///         
///         let blurred = apply_gaussian_blur(image, radius);
///         ctx.set_output_image("result", blurred)?;
///         Ok(())
///     }
/// }
/// ```
pub trait FilterNode: Send + Sync {
    /// Get the metadata for this filter.
    ///
    /// This is called during registration and should return consistent values.
    fn metadata(&self) -> NodeMetadata;

    /// Validate the node configuration.
    ///
    /// Called once before execution begins. Should verify:
    /// - All required inputs are connected
    /// - All parameters are within valid ranges
    /// - Custom constraints are satisfied
    ///
    /// The validation context contains metadata about inputs, not necessarily
    /// the actual pixel data. This allows validation without loading images.
    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError>;

    /// Execute the node.
    ///
    /// Called once per image (or once per batch for batch-aware nodes).
    /// Should:
    /// - Read inputs from the context
    /// - Perform the processing
    /// - Set outputs in the context
    /// - Optionally report progress
    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError>;

    /// Reset any internal state.
    ///
    /// Called between batch executions for filters that maintain state.
    /// Default implementation does nothing.
    fn reset(&mut self) {}

    /// Clone this node into a boxed trait object.
    ///
    /// Required for graph cloning and parallel execution.
    fn clone_box(&self) -> Box<dyn FilterNode>;
}

// Allow cloning Box<dyn FilterNode>
impl Clone for Box<dyn FilterNode> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A simple passthrough node that copies input to output.
///
/// Useful for debugging and as a template for new filters.
#[derive(Debug, Clone)]
pub struct PassthroughNode;

impl FilterNode for PassthroughNode {
    fn metadata(&self) -> NodeMetadata {
        use crate::core::types::PortType;
        
        NodeMetadata::builder("passthrough", "Passthrough")
            .category(Category::Utility)
            .description("Passes the input through unchanged")
            .input(PortDefinition::input("input", PortType::Any))
            .output(PortDefinition::output("output", PortType::Any))
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        // No special validation needed
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let value = ctx.take_input("input")?;
        ctx.set_output("output", value)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_builder() {
        use crate::core::types::PortType;

        let metadata = NodeMetadata::builder("test_filter", "Test Filter")
            .category(Category::Utility)
            .description("A test filter")
            .input(PortDefinition::input("input", PortType::Image))
            .output(PortDefinition::output("output", PortType::Image))
            .tags(["test", "debug"])
            .build();

        assert_eq!(metadata.id, "test_filter");
        assert_eq!(metadata.name, "Test Filter");
        assert_eq!(metadata.category, Category::Utility);
        assert_eq!(metadata.inputs.len(), 1);
        assert_eq!(metadata.outputs.len(), 1);
        assert_eq!(metadata.tags.len(), 2);
    }

    #[test]
    fn test_passthrough_node() {
        use crate::core::error::NodeId;
        use crate::core::types::Value;

        let node = PassthroughNode;
        let metadata = node.metadata();

        assert_eq!(metadata.id, "passthrough");
        assert_eq!(metadata.inputs.len(), 1);
        assert_eq!(metadata.outputs.len(), 1);

        // Test execution
        let mut ctx = ExecutionContext::new(NodeId::new());
        ctx.add_input("input", Value::Integer(42));

        node.execute(&mut ctx).unwrap();

        let outputs = ctx.take_outputs();
        assert_eq!(outputs.get("output"), Some(&Value::Integer(42)));
    }

    #[test]
    fn test_category_display() {
        assert_eq!(Category::Blur.display_name(), "Blur");
        assert_eq!(Category::Transform.display_name(), "Transform");
    }
}
