//! Batch-aware filter examples.
//!
//! These filters demonstrate how to implement efficient batch processing
//! for operations on multiple images.

use crate::core::batch::{BatchAware, BatchMode, extract_images_from_value};
use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{ParameterDefinition, PortDefinition};
use crate::core::types::{PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::{DynamicImage, imageops};
use rayon::prelude::*;

/// Register batch-aware filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(BatchBrightness));
    registry.register(|| Box::new(BatchResize));
    registry.register(|| Box::new(BatchContrast));
}

/// Batch brightness adjustment - processes multiple images efficiently.
#[derive(Debug, Clone)]
pub struct BatchBrightness;

impl FilterNode for BatchBrightness {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_brightness", "Batch Brightness")
            .description("Adjust brightness for single image or batch of images")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .parameter(
                ParameterDefinition::new("brightness", PortType::Float, Value::Float(0.0))
                    .with_description("Brightness adjustment (-1.0 to 1.0)")
                    .with_range(-1.0, 1.0)
            )
            .output(
                PortDefinition::output("images", PortType::Any)
                    .with_description("Adjusted image(s)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        let brightness = ctx.get_float("brightness").unwrap_or(0.0);
        
        // Extract images from input (handles both single image and arrays)
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: e,
        })?;
        
        // Process in parallel
        let adjusted: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: "Failed to get image data".to_string(),
                })?;
                
                let adjusted = adjust_brightness(img, brightness);
                Ok(img_val.with_image(adjusted))
            })
            .collect();
        
        let adjusted = adjusted?;
        
        // Return in same format as input
        let output = if matches!(input, Value::Image(_)) {
            Value::Image(adjusted.into_iter().next().unwrap())
        } else {
            Value::Array(adjusted.into_iter().map(Value::Image).collect())
        };
        
        ctx.set_output("images", output)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl BatchAware for BatchBrightness {
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Parallel
    }

    fn supports_parallel(&self) -> bool {
        true
    }
}

/// Batch resize - resizes multiple images efficiently.
#[derive(Debug, Clone)]
pub struct BatchResize;

impl FilterNode for BatchResize {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_resize", "Batch Resize")
            .description("Resize single image or batch of images")
            .category(Category::Transform)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .parameter(
                ParameterDefinition::new("width", PortType::Integer, Value::Integer(800))
                    .with_description("Target width in pixels")
                    .with_range(1.0, 10000.0)
            )
            .parameter(
                ParameterDefinition::new("height", PortType::Integer, Value::Integer(600))
                    .with_description("Target height in pixels")
                    .with_range(1.0, 10000.0)
            )
            .output(
                PortDefinition::output("images", PortType::Any)
                    .with_description("Resized image(s)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        let width = ctx.get_integer("width").unwrap_or(800) as u32;
        let height = ctx.get_integer("height").unwrap_or(600) as u32;
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: e,
        })?;
        
        // Process in parallel
        let resized: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: "Failed to get image data".to_string(),
                })?;
                
                let resized = DynamicImage::resize_exact(img, width, height, imageops::FilterType::Lanczos3);
                Ok(img_val.with_image(resized))
            })
            .collect();
        
        let resized = resized?;
        
        let output = if matches!(input, Value::Image(_)) {
            Value::Image(resized.into_iter().next().unwrap())
        } else {
            Value::Array(resized.into_iter().map(Value::Image).collect())
        };
        
        ctx.set_output("images", output)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl BatchAware for BatchResize {
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Parallel
    }

    fn supports_parallel(&self) -> bool {
        true
    }
}

/// Batch contrast adjustment.
#[derive(Debug, Clone)]
pub struct BatchContrast;

impl FilterNode for BatchContrast {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_contrast", "Batch Contrast")
            .description("Adjust contrast for single image or batch of images")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .parameter(
                ParameterDefinition::new("contrast", PortType::Float, Value::Float(1.0))
                    .with_description("Contrast multiplier (0.0 to 2.0)")
                    .with_range(0.0, 2.0)
            )
            .output(
                PortDefinition::output("images", PortType::Any)
                    .with_description("Adjusted image(s)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        let contrast = ctx.get_float("contrast").unwrap_or(1.0);
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: e,
        })?;
        
        let adjusted: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: "Failed to get image data".to_string(),
                })?;
                
                let adjusted = adjust_contrast(img, contrast);
                Ok(img_val.with_image(adjusted))
            })
            .collect();
        
        let adjusted = adjusted?;
        
        let output = if matches!(input, Value::Image(_)) {
            Value::Image(adjusted.into_iter().next().unwrap())
        } else {
            Value::Array(adjusted.into_iter().map(Value::Image).collect())
        };
        
        ctx.set_output("images", output)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl BatchAware for BatchContrast {
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Parallel
    }

    fn supports_parallel(&self) -> bool {
        true
    }
}

// Helper functions

fn adjust_brightness(img: &DynamicImage, brightness: f64) -> DynamicImage {
    let mut output = img.to_rgba8();
    let adjustment = (brightness * 255.0) as i32;
    
    for pixel in output.pixels_mut() {
        pixel[0] = (pixel[0] as i32 + adjustment).clamp(0, 255) as u8;
        pixel[1] = (pixel[1] as i32 + adjustment).clamp(0, 255) as u8;
        pixel[2] = (pixel[2] as i32 + adjustment).clamp(0, 255) as u8;
    }
    
    DynamicImage::ImageRgba8(output)
}

fn adjust_contrast(img: &DynamicImage, contrast: f64) -> DynamicImage {
    let mut output = img.to_rgba8();
    let factor = contrast;
    
    for pixel in output.pixels_mut() {
        for i in 0..3 {
            let value = pixel[i] as f64 / 255.0;
            let adjusted = ((value - 0.5) * factor + 0.5) * 255.0;
            pixel[i] = adjusted.clamp(0.0, 255.0) as u8;
        }
    }
    
    DynamicImage::ImageRgba8(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_brightness_metadata() {
        let filter = BatchBrightness;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "batch_brightness");
        assert_eq!(filter.batch_mode(), BatchMode::Parallel);
    }

    #[test]
    fn test_batch_supports_parallel() {
        let filter = BatchBrightness;
        assert!(filter.supports_parallel());
    }
}
