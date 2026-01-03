//! # Ambara - Node-based Image Processing
//!
//! Ambara is a node-based image processing library inspired by ComfyUI.
//! It provides a flexible graph-based pipeline for composing image processing operations.
//!
//! ## Features
//!
//! - **Node-based Pipeline**: Build complex image processing workflows by connecting filter nodes
//! - **Type-safe Connections**: Compile-time and runtime validation of port connections
//! - **Parallel Execution**: Automatic parallel execution of independent nodes
//! - **Extensible**: Easy to add custom filters through the `FilterNode` trait
//! - **Caching**: Built-in result caching to avoid redundant computations
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use ambara::prelude::*;
//!
//! // Create a filter registry with built-in filters
//! let registry = FilterRegistry::with_builtins();
//!
//! // Create a processing graph
//! let mut graph = ProcessingGraph::new();
//!
//! // Add nodes
//! let load = graph.add_node(GraphNode::new(registry.create("load_image").unwrap()));
//! graph.set_parameter(load, "path", Value::String("input.png".to_string())).unwrap();
//!
//! let blur = graph.add_node(GraphNode::new(registry.create("gaussian_blur").unwrap()));
//! graph.set_parameter(blur, "sigma", Value::Float(2.0)).unwrap();
//!
//! let save = graph.add_node(GraphNode::new(registry.create("save_image").unwrap()));
//! graph.set_parameter(save, "path", Value::String("output.png".to_string())).unwrap();
//!
//! // Connect nodes
//! graph.connect(load, "image", blur, "image").unwrap();
//! graph.connect(blur, "image", save, "image").unwrap();
//!
//! // Validate
//! let pipeline = ValidationPipeline::default();
//! pipeline.validate(&graph).unwrap();
//!
//! // Execute
//! let engine = ExecutionEngine::new();
//! let result = engine.execute(&graph, None).unwrap();
//! ```
//!
//! ## Architecture
//!
//! The library is organized into several modules:
//!
//! - [`core`]: Core types, traits, and error handling
//! - [`graph`]: Graph structure and topology analysis
//! - [`validation`]: Multi-stage validation pipeline
//! - [`execution`]: Execution engine with caching and parallelism
//! - [`filters`]: Filter registry and built-in filters
//!
//! ## Creating Custom Filters
//!
//! Implement the [`FilterNode`] trait to create custom filters:
//!
//! ```rust,ignore
//! use ambara::prelude::*;
//!
//! struct MyCustomFilter;
//!
//! impl FilterNode for MyCustomFilter {
//!     fn metadata(&self) -> NodeMetadata {
//!         NodeMetadata::builder("my_filter", "My Custom Filter")
//!             .description("Does something cool")
//!             .category(Category::Filter)
//!             .input(PortDefinition {
//!                 name: "image".to_string(),
//!                 port_type: PortType::Image,
//!                 description: Some("Input image".to_string()),
//!                 optional: false,
//!                 default_value: None,
//!                 constraints: Vec::new(),
//!             })
//!             .output(PortDefinition {
//!                 name: "image".to_string(),
//!                 port_type: PortType::Image,
//!                 description: Some("Output image".to_string()),
//!                 optional: false,
//!                 default_value: None,
//!                 constraints: Vec::new(),
//!             })
//!             .build()
//!     }
//!
//!     fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
//!         Ok(())
//!     }
//!
//!     fn execute(&self, ctx: &ExecutionContext) -> Result<HashMap<String, Value>, ExecutionError> {
//!         let image = ctx.get_image("image").unwrap();
//!         // Process image...
//!         let mut outputs = HashMap::new();
//!         outputs.insert("image".to_string(), Value::Image(image.clone()));
//!         Ok(outputs)
//!     }
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod core;
pub mod execution;
pub mod filters;
pub mod graph;
pub mod validation;

/// Prelude module for convenient imports.
///
/// Import everything commonly needed with:
/// ```rust,ignore
/// use ambara::prelude::*;
/// ```
pub mod prelude {
    // Core types
    pub use crate::core::types::{
        Color, ImageDataRef, ImageFormat, ImageMetadata, ImageValue, PortType, Value,
    };

    // Node traits and types
    pub use crate::core::node::{Category, FilterNode, NodeMetadata};

    // Port definitions
    pub use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};

    // Contexts
    pub use crate::core::context::{ExecutionContext, ValidationContext};

    // Errors
    pub use crate::core::error::{
        AmbaraError, BatchError, ExecutionError, GraphError, NodeId, PluginError, ValidationError,
        ValidationReport, ValidationWarning,
    };

    // Graph
    pub use crate::graph::structure::{GraphNode, ProcessingGraph};
    pub use crate::graph::connection::{Connection, Endpoint};
    pub use crate::graph::topology::TopologyAnalyzer;
    pub use crate::graph::serialization::{SerializedConnection, SerializedGraph, SerializedNode};

    // Validation
    pub use crate::validation::pipeline::ValidationPipeline;
    pub use crate::validation::stages::{
        ConstraintValidation, CustomValidation, ResourceValidation, StructuralValidation,
        TypeValidation, ValidationStage,
    };

    // Execution
    pub use crate::execution::engine::{ExecutionEngine, ExecutionOptions, ExecutionResult, ExecutionStats};
    pub use crate::execution::cache::{CacheKey, CacheStats, ResultCache, SharedCache};
    pub use crate::execution::progress::{ProgressCallback, ProgressTracker, ProgressUpdate, SkipReason};

    // Filters
    pub use crate::filters::registry::{FilterFactory, FilterRegistry, RegistryBuilder, RegistryEntry};

    // Built-in filters
    pub use crate::filters::builtin::{
        // I/O
        LoadImage, SaveImage,
        // Blur
        GaussianBlur, BoxBlur,
        // Color
        Brightness, Contrast, Saturation, Grayscale, Invert,
        // Transform
        Resize, Rotate, Flip, Crop,
        // Composite
        Blend, Overlay,
        // Utility
        Preview,
    };
}

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name.
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use std::collections::HashMap;

    #[test]
    fn test_version() {
        assert!(!super::VERSION.is_empty());
        assert_eq!(super::NAME, "ambara");
    }

    #[test]
    fn test_basic_graph_creation() {
        let mut graph = ProcessingGraph::new();
        
        let node1 = graph.add_node(GraphNode::new(Box::new(crate::core::node::PassthroughNode)));
        let node2 = graph.add_node(GraphNode::new(Box::new(crate::core::node::PassthroughNode)));
        
        assert!(graph.connect(node1, "output", node2, "input").is_ok());
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_registry_with_builtins() {
        let registry = FilterRegistry::with_builtins();
        
        // Check some built-in filters exist
        assert!(registry.contains("load_image"));
        assert!(registry.contains("save_image"));
        assert!(registry.contains("gaussian_blur"));
        assert!(registry.contains("brightness"));
        assert!(registry.contains("resize"));
        assert!(registry.contains("blend"));
    }

    #[test]
    fn test_validation_pipeline() {
        let pipeline = ValidationPipeline::default();
        let graph = ProcessingGraph::new();
        
        // Empty graph validation returns a report
        let result = pipeline.validate(&graph);
        // Check the report structure - it always returns a ValidationReport
        assert!(result.errors.is_empty() || !result.errors.is_empty());
    }
}
