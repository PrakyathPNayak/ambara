//! Color adjustment filters with optional GPU acceleration

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::gpu::{GpuAccelerated, GpuFilters, GpuPool};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::Pixel;

/// Register color filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(Brightness));
    registry.register(|| Box::new(Contrast));
    registry.register(|| Box::new(Saturation));
    registry.register(|| Box::new(Grayscale));
    registry.register(|| Box::new(Invert));
}

/// Adjusts image brightness.
#[derive(Debug, Clone)]
pub struct Brightness;

impl FilterNode for Brightness {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("brightness", "Brightness")
            .description("Adjust the brightness of an image")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Adjusted image")
            )
            .parameter(
                ParameterDefinition::new("amount", PortType::Float, Value::Float(0.0))
                    .with_description("Brightness adjustment (-1.0 to 1.0)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: -1.0, max: 1.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let amount = ctx.get_float("amount").unwrap_or(0.0) as f32;
        let adjustment = (amount * 255.0) as i32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let mut result = img_data.to_rgba8();
        
        for pixel in result.pixels_mut() {
            let channels = pixel.channels_mut();
            for i in 0..3 {
                channels[i] = (channels[i] as i32 + adjustment).clamp(0, 255) as u8;
            }
        }

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(result));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Adjusts image contrast.
#[derive(Debug, Clone)]
pub struct Contrast;

impl FilterNode for Contrast {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("contrast", "Contrast")
            .description("Adjust the contrast of an image")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Adjusted image")
            )
            .parameter(
                ParameterDefinition::new("amount", PortType::Float, Value::Float(1.0))
                    .with_description("Contrast multiplier (0.0 to 3.0)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 3.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let factor = ctx.get_float("amount").unwrap_or(1.0) as f32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img_data.to_rgba8();
        
        // Apply contrast adjustment manually
        // Contrast is applied as: new_value = (old_value - 128) * factor + 128
        let result = image::ImageBuffer::from_fn(rgba.width(), rgba.height(), |x, y| {
            let pixel = rgba.get_pixel(x, y);
            let adjust = |v: u8| -> u8 {
                let adjusted = ((v as f32 - 128.0) * factor + 128.0).clamp(0.0, 255.0);
                adjusted as u8
            };
            image::Rgba([adjust(pixel[0]), adjust(pixel[1]), adjust(pixel[2]), pixel[3]])
        });

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(result));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Adjusts image saturation.
#[derive(Debug, Clone)]
pub struct Saturation;

impl FilterNode for Saturation {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("saturation", "Saturation")
            .description("Adjust the color saturation of an image")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Adjusted image")
            )
            .parameter(
                ParameterDefinition::new("amount", PortType::Float, Value::Float(1.0))
                    .with_description("Saturation multiplier (0.0 = grayscale, 1.0 = original)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 3.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let saturation = ctx.get_float("amount").unwrap_or(1.0) as f32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let mut result = img_data.to_rgba8();

        for pixel in result.pixels_mut() {
            let channels = pixel.channels_mut();
            let r = channels[0] as f32 / 255.0;
            let g = channels[1] as f32 / 255.0;
            let b = channels[2] as f32 / 255.0;

            // Calculate luminance
            let luma = 0.299 * r + 0.587 * g + 0.114 * b;

            // Interpolate between grayscale and original
            let new_r = luma + (r - luma) * saturation;
            let new_g = luma + (g - luma) * saturation;
            let new_b = luma + (b - luma) * saturation;

            channels[0] = (new_r.clamp(0.0, 1.0) * 255.0) as u8;
            channels[1] = (new_g.clamp(0.0, 1.0) * 255.0) as u8;
            channels[2] = (new_b.clamp(0.0, 1.0) * 255.0) as u8;
        }

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(result));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Converts image to grayscale with GPU acceleration.
#[derive(Debug, Clone)]
pub struct Grayscale;

impl FilterNode for Grayscale {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("grayscale", "Grayscale")
            .description("Convert an image to grayscale (GPU accelerated when available)")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Grayscale image")
            )
            .parameter(
                ParameterDefinition::new("use_gpu", PortType::Boolean, Value::Boolean(true))
                    .with_description("Use GPU acceleration if available"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let use_gpu = ctx.get_bool("use_gpu").unwrap_or(true);

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        // Try GPU acceleration
        if use_gpu {
            if let Some(device) = GpuPool::global().device() {
                if let Ok(filters) = GpuFilters::new(device) {
                    if let Ok(result) = filters.grayscale(img_data) {
                        let result_value = ImageValue::new(result);
                        ctx.set_output("image", Value::Image(result_value))?;
                        return Ok(());
                    }
                }
            }
        }

        // Fallback to CPU
        let gray = img_data.grayscale();
        let rgba = gray.to_rgba8();

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(rgba));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl GpuAccelerated for Grayscale {
    fn supports_gpu(&self) -> bool {
        true
    }
}

/// Inverts image colors with GPU acceleration.
#[derive(Debug, Clone)]
pub struct Invert;

impl FilterNode for Invert {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("invert", "Invert Colors")
            .description("Invert the colors of an image (GPU accelerated when available)")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Inverted image")
            )
            .parameter(
                ParameterDefinition::new("invert_alpha", PortType::Boolean, Value::Boolean(false))
                    .with_description("Also invert the alpha channel"),
            )
            .parameter(
                ParameterDefinition::new("use_gpu", PortType::Boolean, Value::Boolean(true))
                    .with_description("Use GPU acceleration if available"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let invert_alpha = ctx.get_bool("invert_alpha").unwrap_or(false);
        let use_gpu = ctx.get_bool("use_gpu").unwrap_or(true);

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        // Try GPU acceleration (only if not inverting alpha, GPU shader doesn't support that)
        if use_gpu && !invert_alpha {
            if let Some(device) = GpuPool::global().device() {
                if let Ok(filters) = GpuFilters::new(device) {
                    if let Ok(result) = filters.invert(img_data) {
                        let result_value = ImageValue::new(result);
                        ctx.set_output("image", Value::Image(result_value))?;
                        return Ok(());
                    }
                }
            }
        }

        // Fallback to CPU
        let mut result = img_data.to_rgba8();

        for pixel in result.pixels_mut() {
            let channels = pixel.channels_mut();
            channels[0] = 255 - channels[0];
            channels[1] = 255 - channels[1];
            channels[2] = 255 - channels[2];
            if invert_alpha {
                channels[3] = 255 - channels[3];
            }
        }

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(result));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl GpuAccelerated for Invert {
    fn supports_gpu(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brightness_metadata() {
        let filter = Brightness;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "brightness");
        assert_eq!(metadata.category, Category::Color);
    }

    #[test]
    fn test_grayscale_metadata() {
        let filter = Grayscale;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "grayscale");
        // Now has use_gpu parameter
        assert_eq!(metadata.parameters.len(), 1);
    }

    #[test]
    fn test_invert_metadata() {
        let filter = Invert;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "invert");
        // Now has invert_alpha and use_gpu parameters
        assert_eq!(metadata.parameters.len(), 2);
    }

    #[test]
    fn test_gpu_support() {
        assert!(Grayscale.supports_gpu());
        assert!(Invert.supports_gpu());
    }
}
