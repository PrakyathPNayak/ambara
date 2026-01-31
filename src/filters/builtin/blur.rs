//! Blur filters: Gaussian, Box blur with optional GPU acceleration

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::gpu::{GpuAccelerated, GpuFilters, GpuPool};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;

/// Register blur filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(GaussianBlur));
    registry.register(|| Box::new(BoxBlur));
}

/// Applies Gaussian blur to an image with optional GPU acceleration.
#[derive(Debug, Clone)]
pub struct GaussianBlur;

impl FilterNode for GaussianBlur {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("gaussian_blur", "Gaussian Blur")
            .description("Apply a Gaussian blur effect to an image (GPU accelerated when available)")
            .category(Category::Blur)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Blurred image")
            )
            .parameter(
                ParameterDefinition::new("sigma", PortType::Float, Value::Float(1.0))
                    .with_description("Blur intensity (standard deviation)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.1, max: 100.0 }),
            )
            .parameter(
                ParameterDefinition::new("use_gpu", PortType::Boolean, Value::Boolean(true))
                    .with_description("Use GPU acceleration if available"),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let sigma = ctx.get_float("sigma").unwrap_or(1.0);
        
        if sigma <= 0.0 {
            return Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: "Sigma must be positive".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let sigma = ctx.get_float("sigma").unwrap_or(1.0) as f32;
        let use_gpu = ctx.get_bool("use_gpu").unwrap_or(true);

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        // Try GPU acceleration
        if use_gpu {
            if let Some(device) = GpuPool::global().device() {
                if let Ok(filters) = GpuFilters::new(device) {
                    let radius = (sigma * 3.0).ceil();
                    if let Ok(blurred) = filters.gaussian_blur(img_data, radius, sigma) {
                        let result = ImageValue::new(blurred);
                        ctx.set_output("image", Value::Image(result))?;
                        return Ok(());
                    }
                }
            }
        }

        // Fallback to CPU
        let rgba = img_data.to_rgba8();
        let blurred = imageproc::filter::gaussian_blur_f32(&rgba, sigma);

        let result = ImageValue::new(image::DynamicImage::ImageRgba8(blurred));

        ctx.set_output("image", Value::Image(result))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl GpuAccelerated for GaussianBlur {
    fn supports_gpu(&self) -> bool {
        true
    }
}

/// Applies box blur to an image.
#[derive(Debug, Clone)]
pub struct BoxBlur;

impl FilterNode for BoxBlur {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("box_blur", "Box Blur")
            .description("Apply a fast box blur effect to an image")
            .category(Category::Blur)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Blurred image")
            )
            .parameter(
                ParameterDefinition::new("radius_x", PortType::Integer, Value::Integer(3))
                    .with_description("Horizontal blur radius")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 1.0, max: 50.0 }),
            )
            .parameter(
                ParameterDefinition::new("radius_y", PortType::Integer, Value::Integer(3))
                    .with_description("Vertical blur radius")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 1.0, max: 50.0 }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let radius_x = ctx.get_integer("radius_x").unwrap_or(3);
        let radius_y = ctx.get_integer("radius_y").unwrap_or(3);

        if radius_x < 1 || radius_y < 1 {
            return Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: "Blur radius must be at least 1".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let radius_x = ctx.get_integer("radius_x").unwrap_or(3) as u32;
        let radius_y = ctx.get_integer("radius_y").unwrap_or(3) as u32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        // Apply box blur using separable filter
        // Create box blur kernels
        let h_kernel_size = (2 * radius_x + 1) as usize;
        let v_kernel_size = (2 * radius_y + 1) as usize;
        let h_value = 1.0 / h_kernel_size as f32;
        let v_value = 1.0 / v_kernel_size as f32;
        let h_kernel: Vec<f32> = vec![h_value; h_kernel_size];
        let v_kernel: Vec<f32> = vec![v_value; v_kernel_size];
        
        let rgba = img_data.to_rgba8();
        let blurred = imageproc::filter::separable_filter(&rgba, &h_kernel, &v_kernel);

        let result = ImageValue::new(image::DynamicImage::ImageRgba8(blurred));

        ctx.set_output("image", Value::Image(result))?;
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
    fn test_gaussian_blur_metadata() {
        let filter = GaussianBlur;
        let metadata = filter.metadata();
        
        assert_eq!(metadata.id, "gaussian_blur");
        assert_eq!(metadata.category, Category::Blur);
    }

    #[test]
    fn test_box_blur_metadata() {
        let filter = BoxBlur;
        let metadata = filter.metadata();
        
        assert_eq!(metadata.id, "box_blur");
        assert_eq!(metadata.parameters.len(), 2);
    }
}
