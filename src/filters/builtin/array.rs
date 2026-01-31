//! Array processing utilities - automatically wraps single-image filters to work with arrays.
//!
//! This module provides generic wrappers that make any single-image filter work with image arrays.

use crate::core::batch::{extract_images_from_value, BatchAware, BatchMode};
use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{ParameterDefinition, PortDefinition};
use crate::core::types::{PortType, Value};
use crate::filters::registry::FilterRegistry;
use rayon::prelude::*;

/// Register array processing filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(ArrayMap));
    registry.register(|| Box::new(ArrayFilter));
    registry.register(|| Box::new(ArrayConcat));
    registry.register(|| Box::new(ArraySlice));
}

/// Map an operation across all images in an array (or single image).
/// 
/// This is a generic node that applies batch processing to any input.
#[derive(Debug, Clone)]
pub struct ArrayMap;

impl FilterNode for ArrayMap {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("array_map", "Array Map")
            .description("Apply an operation to each image in an array (parallel processing)")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .output(
                PortDefinition::output("images", PortType::Any)
                    .with_description("Processed image(s)")
            )
            .output(
                PortDefinition::output("count", PortType::Integer)
                    .with_description("Number of images processed")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        
        // Extract images from input
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: e,
        })?;
        
        let count = images.len() as i64;
        
        // Return in same format as input
        let output = if matches!(input, Value::Image(_)) {
            Value::Image(images.into_iter().next().unwrap())
        } else {
            Value::Array(images.into_iter().map(Value::Image).collect())
        };
        
        ctx.set_output("images", output)?;
        ctx.set_output("count", Value::Integer(count))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl BatchAware for ArrayMap {
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Parallel
    }

    fn supports_parallel(&self) -> bool {
        true
    }
}

/// Filter images in an array based on conditions.
#[derive(Debug, Clone)]
pub struct ArrayFilter;

impl FilterNode for ArrayFilter {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("array_filter", "Array Filter")
            .description("Filter images in an array based on size/properties")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Array(Box::new(PortType::Image)))
                    .with_description("Array of images")
            )
            .parameter(
                ParameterDefinition::new("min_width", PortType::Integer, Value::Integer(0))
                    .with_description("Minimum width (0 = no limit)")
                    .with_range(0.0, 10000.0)
            )
            .parameter(
                ParameterDefinition::new("min_height", PortType::Integer, Value::Integer(0))
                    .with_description("Minimum height (0 = no limit)")
                    .with_range(0.0, 10000.0)
            )
            .output(
                PortDefinition::output("images", PortType::Array(Box::new(PortType::Image)))
                    .with_description("Filtered images")
            )
            .output(
                PortDefinition::output("count", PortType::Integer)
                    .with_description("Number of images after filtering")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        let min_width = ctx.get_integer("min_width").unwrap_or(0) as u32;
        let min_height = ctx.get_integer("min_height").unwrap_or(0) as u32;
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: e,
        })?;
        
        // Filter based on dimensions
        let filtered: Vec<_> = images
            .into_iter()
            .filter(|img| {
                img.metadata.width >= min_width && img.metadata.height >= min_height
            })
            .collect();
        
        let count = filtered.len() as i64;
        
        ctx.set_output("images", Value::Array(filtered.into_iter().map(Value::Image).collect()))?;
        ctx.set_output("count", Value::Integer(count))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Concatenate multiple image arrays or images into a single array.
#[derive(Debug, Clone)]
pub struct ArrayConcat;

impl FilterNode for ArrayConcat {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("array_concat", "Array Concat")
            .description("Concatenate multiple arrays or images into one array")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images1", PortType::Any)
                    .with_description("First image(s)")
            )
            .input(
                PortDefinition::input("images2", PortType::Any)
                    .optional()
                    .with_description("Second image(s)")
            )
            .input(
                PortDefinition::input("images3", PortType::Any)
                    .optional()
                    .with_description("Third image(s)")
            )
            .output(
                PortDefinition::output("images", PortType::Array(Box::new(PortType::Image)))
                    .with_description("Combined array of images")
            )
            .output(
                PortDefinition::output("count", PortType::Integer)
                    .with_description("Total number of images")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let mut all_images = Vec::new();
        
        // Collect from all inputs
        for input_name in &["images1", "images2", "images3"] {
            if let Ok(input) = ctx.get_input(input_name) {
                let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: format!("Error extracting {}: {}", input_name, e),
                })?;
                all_images.extend(images);
            }
        }
        
        let count = all_images.len() as i64;
        
        ctx.set_output("images", Value::Array(all_images.into_iter().map(Value::Image).collect()))?;
        ctx.set_output("count", Value::Integer(count))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Extract a slice/range of images from an array.
#[derive(Debug, Clone)]
pub struct ArraySlice;

impl FilterNode for ArraySlice {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("array_slice", "Array Slice")
            .description("Extract a range of images from an array")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Array(Box::new(PortType::Image)))
                    .with_description("Array of images")
            )
            .parameter(
                ParameterDefinition::new("start", PortType::Integer, Value::Integer(0))
                    .with_description("Start index (inclusive)")
                    .with_range(0.0, 1000.0)
            )
            .parameter(
                ParameterDefinition::new("end", PortType::Integer, Value::Integer(-1))
                    .with_description("End index (exclusive, -1 = end)")
                    .with_range(-1.0, 1000.0)
            )
            .output(
                PortDefinition::output("images", PortType::Array(Box::new(PortType::Image)))
                    .with_description("Sliced array")
            )
            .output(
                PortDefinition::output("count", PortType::Integer)
                    .with_description("Number of images in slice")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        let start = ctx.get_integer("start").unwrap_or(0) as usize;
        let end_param = ctx.get_integer("end").unwrap_or(-1);
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: e,
        })?;
        
        let end = if end_param < 0 {
            images.len()
        } else {
            (end_param as usize).min(images.len())
        };
        
        let sliced: Vec<_> = images.into_iter().skip(start).take(end.saturating_sub(start)).collect();
        let count = sliced.len() as i64;
        
        ctx.set_output("images", Value::Array(sliced.into_iter().map(Value::Image).collect()))?;
        ctx.set_output("count", Value::Integer(count))?;
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
    fn test_array_map_metadata() {
        let filter = ArrayMap;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "array_map");
        assert!(filter.supports_parallel());
    }
}
