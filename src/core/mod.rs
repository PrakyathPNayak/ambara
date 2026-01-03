//! Core types and traits for the Ambara image processing system.
//!
//! This module contains the foundational types that make up the image processing
//! pipeline including:
//! - Value types (Image, Integer, Float, etc.)
//! - Port definitions and constraints
//! - Node traits and metadata
//! - Error types
//! - Execution and validation contexts

pub mod types;
pub mod port;
pub mod error;
pub mod context;
pub mod node;

// Re-export commonly used types
pub use types::{Value, PortType, ImageValue, ImageFormat, Color, ImageMetadata};
pub use port::{PortDefinition, PortDirection, Constraint};
pub use error::{AmbaraError, GraphError, ValidationError, ExecutionError};
pub use context::{ValidationContext, ExecutionContext};
pub use node::{FilterNode, NodeMetadata, Category};
