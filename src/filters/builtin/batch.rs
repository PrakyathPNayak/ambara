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
    registry.register(|| Box::new(BatchGaussianBlur));
    registry.register(|| Box::new(BatchSaturation));
    registry.register(|| Box::new(BatchRotate));
    registry.register(|| Box::new(BatchGrayscale));
    registry.register(|| Box::new(BatchInvert));
    registry.register(|| Box::new(BatchCrop));
    registry.register(|| Box::new(BatchFlip));
}

/// Batch brightness adjustment - processes multiple images efficiently.
#[derive(Debug, Clone)]
pub struct BatchBrightness;

impl FilterNode for BatchBrightness {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_brightness", "Batch Brightness")
            .description("Adjust brightness for single image or batch of images")
            .category(Category::Adjust)
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
        
        let was_single = matches!(input, Value::Image(_));
        
        // Extract images from input (handles both single image and arrays)
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id.clone(),
            error: e,
        })?;
        
        // Process in parallel
        let adjusted: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id.clone(),
                    error: "Failed to get image data".to_string(),
                })?;
                
                let adjusted = adjust_brightness(img, brightness);
                Ok(img_val.with_image(adjusted))
            })
            .collect();
        
        let adjusted = adjusted?;
        
        // Return in same format as input
        let output = if was_single {
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
        
        let was_single = matches!(input, Value::Image(_));
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id.clone(),
            error: e,
        })?;
        
        // Process in parallel
        let resized: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id.clone(),
                    error: "Failed to get image data".to_string(),
                })?;
                
                let resized = img.resize_exact(width, height, imageops::FilterType::Lanczos3);
                Ok(img_val.with_image(resized))
            })
            .collect();
        
        let resized = resized?;
        
        let output = if was_single {
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
            .category(Category::Adjust)
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
        
        let was_single = matches!(input, Value::Image(_));
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id.clone(),
            error: e,
        })?;
        
        let adjusted: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id.clone(),
                    error: "Failed to get image data".to_string(),
                })?;
                
                let adjusted = adjust_contrast(img, contrast);
                Ok(img_val.with_image(adjusted))
            })
            .collect();
        
        let adjusted = adjusted?;
        
        let output = if was_single {
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

/// Batch Gaussian blur.
#[derive(Debug, Clone)]
pub struct BatchGaussianBlur;

impl FilterNode for BatchGaussianBlur {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_gaussian_blur", "Batch Gaussian Blur")
            .description("Apply Gaussian blur to single image or batch of images")
            .category(Category::Blur)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .parameter(
                ParameterDefinition::new("sigma", PortType::Float, Value::Float(1.0))
                    .with_description("Blur sigma (standard deviation)")
                    .with_range(0.1, 10.0)
            )
            .output(
                PortDefinition::output("images", PortType::Any)
                    .with_description("Blurred image(s)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        let sigma = ctx.get_float("sigma").unwrap_or(1.0) as f32;
        
        let was_single = matches!(input, Value::Image(_));
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id.clone(),
            error: e,
        })?;
        
        let blurred: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id.clone(),
                    error: "Failed to get image data".to_string(),
                })?;
                
                let blurred = img.blur(sigma);
                Ok(img_val.with_image(blurred))
            })
            .collect();
        
        let blurred = blurred?;
        
        let output = if was_single {
            Value::Image(blurred.into_iter().next().unwrap())
        } else {
            Value::Array(blurred.into_iter().map(Value::Image).collect())
        };
        
        ctx.set_output("images", output)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl BatchAware for BatchGaussianBlur {
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Parallel
    }

    fn supports_parallel(&self) -> bool {
        true
    }
}

/// Batch saturation adjustment.
#[derive(Debug, Clone)]
pub struct BatchSaturation;

impl FilterNode for BatchSaturation {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_saturation", "Batch Saturation")
            .description("Adjust saturation for single image or batch of images")
            .category(Category::Adjust)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .parameter(
                ParameterDefinition::new("saturation", PortType::Float, Value::Float(1.0))
                    .with_description("Saturation multiplier (0.0 to 2.0)")
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
        let saturation = ctx.get_float("saturation").unwrap_or(1.0);
        
        let was_single = matches!(input, Value::Image(_));
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id.clone(),
            error: e,
        })?;
        
        let adjusted: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id.clone(),
                    error: "Failed to get image data".to_string(),
                })?;
                
                let adjusted = adjust_saturation(img, saturation);
                Ok(img_val.with_image(adjusted))
            })
            .collect();
        
        let adjusted = adjusted?;
        
        let output = if was_single {
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

impl BatchAware for BatchSaturation {
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Parallel
    }

    fn supports_parallel(&self) -> bool {
        true
    }
}

/// Batch rotate.
#[derive(Debug, Clone)]
pub struct BatchRotate;

impl FilterNode for BatchRotate {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_rotate", "Batch Rotate")
            .description("Rotate single image or batch of images")
            .category(Category::Transform)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .parameter(
                ParameterDefinition::new("angle", PortType::Float, Value::Float(90.0))
                    .with_description("Rotation angle in degrees (90, 180, 270)")
            )
            .output(
                PortDefinition::output("images", PortType::Any)
                    .with_description("Rotated image(s)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        let angle = ctx.get_float("angle").unwrap_or(90.0);
        
        let was_single = matches!(input, Value::Image(_));
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id.clone(),
            error: e,
        })?;
        
        let rotated: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id.clone(),
                    error: "Failed to get image data".to_string(),
                })?;
                
                let rotated = rotate_image(img, angle);
                Ok(img_val.with_image(rotated))
            })
            .collect();
        
        let rotated = rotated?;
        
        let output = if was_single {
            Value::Image(rotated.into_iter().next().unwrap())
        } else {
            Value::Array(rotated.into_iter().map(Value::Image).collect())
        };
        
        ctx.set_output("images", output)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl BatchAware for BatchRotate {
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Parallel
    }

    fn supports_parallel(&self) -> bool {
        true
    }
}

/// Batch grayscale conversion.
#[derive(Debug, Clone)]
pub struct BatchGrayscale;

impl FilterNode for BatchGrayscale {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_grayscale", "Batch Grayscale")
            .description("Convert single image or batch of images to grayscale")
            .category(Category::Adjust)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .output(
                PortDefinition::output("images", PortType::Any)
                    .with_description("Grayscale image(s)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        
        let was_single = matches!(input, Value::Image(_));
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id.clone(),
            error: e,
        })?;
        
        let grayscale: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id.clone(),
                    error: "Failed to get image data".to_string(),
                })?;
                
                let gray = img.grayscale();
                Ok(img_val.with_image(gray))
            })
            .collect();
        
        let grayscale = grayscale?;
        
        let output = if was_single {
            Value::Image(grayscale.into_iter().next().unwrap())
        } else {
            Value::Array(grayscale.into_iter().map(Value::Image).collect())
        };
        
        ctx.set_output("images", output)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl BatchAware for BatchGrayscale {
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Parallel
    }

    fn supports_parallel(&self) -> bool {
        true
    }
}

/// Batch invert.
#[derive(Debug, Clone)]
pub struct BatchInvert;

impl FilterNode for BatchInvert {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_invert", "Batch Invert")
            .description("Invert colors of single image or batch of images")
            .category(Category::Adjust)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .output(
                PortDefinition::output("images", PortType::Any)
                    .with_description("Inverted image(s)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        
        let was_single = matches!(input, Value::Image(_));
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id.clone(),
            error: e,
        })?;
        
        let inverted: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id.clone(),
                    error: "Failed to get image data".to_string(),
                })?;
                
                let inverted = invert_image(img);
                Ok(img_val.with_image(inverted))
            })
            .collect();
        
        let inverted = inverted?;
        
        let output = if was_single {
            Value::Image(inverted.into_iter().next().unwrap())
        } else {
            Value::Array(inverted.into_iter().map(Value::Image).collect())
        };
        
        ctx.set_output("images", output)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl BatchAware for BatchInvert {
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Parallel
    }

    fn supports_parallel(&self) -> bool {
        true
    }
}

/// Batch crop.
#[derive(Debug, Clone)]
pub struct BatchCrop;

impl FilterNode for BatchCrop {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_crop", "Batch Crop")
            .description("Crop single image or batch of images")
            .category(Category::Transform)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .parameter(
                ParameterDefinition::new("x", PortType::Integer, Value::Integer(0))
                    .with_description("X coordinate of crop region")
                    .with_range(0.0, 10000.0)
            )
            .parameter(
                ParameterDefinition::new("y", PortType::Integer, Value::Integer(0))
                    .with_description("Y coordinate of crop region")
                    .with_range(0.0, 10000.0)
            )
            .parameter(
                ParameterDefinition::new("width", PortType::Integer, Value::Integer(100))
                    .with_description("Width of crop region")
                    .with_range(1.0, 10000.0)
            )
            .parameter(
                ParameterDefinition::new("height", PortType::Integer, Value::Integer(100))
                    .with_description("Height of crop region")
                    .with_range(1.0, 10000.0)
            )
            .output(
                PortDefinition::output("images", PortType::Any)
                    .with_description("Cropped image(s)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        let x = ctx.get_integer("x").unwrap_or(0) as u32;
        let y = ctx.get_integer("y").unwrap_or(0) as u32;
        let width = ctx.get_integer("width").unwrap_or(100) as u32;
        let height = ctx.get_integer("height").unwrap_or(100) as u32;
        
        let was_single = matches!(input, Value::Image(_));
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id.clone(),
            error: e,
        })?;
        
        let cropped: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id.clone(),
                    error: "Failed to get image data".to_string(),
                })?;
                
                let cropped = img.crop_imm(x, y, width, height);
                Ok(img_val.with_image(cropped))
            })
            .collect();
        
        let cropped = cropped?;
        
        let output = if was_single {
            Value::Image(cropped.into_iter().next().unwrap())
        } else {
            Value::Array(cropped.into_iter().map(Value::Image).collect())
        };
        
        ctx.set_output("images", output)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl BatchAware for BatchCrop {
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Parallel
    }

    fn supports_parallel(&self) -> bool {
        true
    }
}

/// Batch flip.
#[derive(Debug, Clone)]
pub struct BatchFlip;

impl FilterNode for BatchFlip {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("batch_flip", "Batch Flip")
            .description("Flip single image or batch of images")
            .category(Category::Transform)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Any)
                    .with_description("Image or array of images")
            )
            .parameter(
                ParameterDefinition::new("direction", PortType::String, Value::String("horizontal".to_string()))
                    .with_description("Flip direction: 'horizontal' or 'vertical'")
            )
            .output(
                PortDefinition::output("images", PortType::Any)
                    .with_description("Flipped image(s)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let input = ctx.get_input("images")?;
        let direction = ctx.get_string("direction").unwrap_or("horizontal");
        
        let was_single = matches!(input, Value::Image(_));
        
        let images = extract_images_from_value(input).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id.clone(),
            error: e,
        })?;
        
        let flipped: Result<Vec<_>, _> = images
            .par_iter()
            .map(|img_val| {
                let img = img_val.get_image().ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id.clone(),
                    error: "Failed to get image data".to_string(),
                })?;
                
                let flipped = match direction {
                    "vertical" => img.flipv(),
                    _ => img.fliph(),
                };
                Ok(img_val.with_image(flipped))
            })
            .collect();
        
        let flipped = flipped?;
        
        let output = if was_single {
            Value::Image(flipped.into_iter().next().unwrap())
        } else {
            Value::Array(flipped.into_iter().map(Value::Image).collect())
        };
        
        ctx.set_output("images", output)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl BatchAware for BatchFlip {
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

fn adjust_saturation(img: &DynamicImage, saturation: f64) -> DynamicImage {
    let mut output = img.to_rgba8();
    
    for pixel in output.pixels_mut() {
        let r = pixel[0] as f64 / 255.0;
        let g = pixel[1] as f64 / 255.0;
        let b = pixel[2] as f64 / 255.0;
        
        // Convert to HSL
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        
        if (max - min).abs() < 0.001 {
            // Grayscale, no saturation change needed
            continue;
        }
        
        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        
        let h = if (max - r).abs() < 0.001 {
            (g - b) / d + (if g < b { 6.0 } else { 0.0 })
        } else if (max - g).abs() < 0.001 {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        } / 6.0;
        
        // Adjust saturation
        let new_s = (s * saturation).clamp(0.0, 1.0);
        
        // Convert back to RGB
        let (new_r, new_g, new_b) = hsl_to_rgb(h, new_s, l);
        
        pixel[0] = (new_r * 255.0) as u8;
        pixel[1] = (new_g * 255.0) as u8;
        pixel[2] = (new_b * 255.0) as u8;
    }
    
    DynamicImage::ImageRgba8(output)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    
    let (r, g, b) = match (h * 6.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    
    (r + m, g + m, b + m)
}

fn rotate_image(img: &DynamicImage, angle: f64) -> DynamicImage {
    let angle_normalized = ((angle % 360.0) + 360.0) % 360.0;
    
    match angle_normalized as i32 {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img.clone(), // For other angles, would need more complex rotation
    }
}

fn invert_image(img: &DynamicImage) -> DynamicImage {
    let mut output = img.to_rgba8();
    
    for pixel in output.pixels_mut() {
        pixel[0] = 255 - pixel[0];
        pixel[1] = 255 - pixel[1];
        pixel[2] = 255 - pixel[2];
        // Keep alpha unchanged
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

    #[test]
    fn test_all_batch_filters_metadata() {
        let filters: Vec<Box<dyn FilterNode>> = vec![
            Box::new(BatchBrightness),
            Box::new(BatchResize),
            Box::new(BatchContrast),
            Box::new(BatchGaussianBlur),
            Box::new(BatchSaturation),
            Box::new(BatchRotate),
            Box::new(BatchGrayscale),
            Box::new(BatchInvert),
            Box::new(BatchCrop),
            Box::new(BatchFlip),
        ];
        
        for filter in filters {
            let metadata = filter.metadata();
            assert!(!metadata.id.is_empty());
            assert!(!metadata.name.is_empty());
        }
    }
}