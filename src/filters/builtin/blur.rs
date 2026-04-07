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
    registry.register(|| Box::new(MedianBlur));
    registry.register(|| Box::new(MotionBlur));
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

/// Applies median blur for effective noise removal while preserving edges.
#[derive(Debug, Clone)]
pub struct MedianBlur;

impl FilterNode for MedianBlur {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("median_blur", "Median Blur")
            .description("Apply median blur to reduce salt-and-pepper noise while preserving edges")
            .category(Category::Blur)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Filtered image"),
            )
            .parameter(
                ParameterDefinition::new("radius", PortType::Integer, Value::Integer(2))
                    .with_description("Filter radius (kernel size = 2*radius+1)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 1.0, max: 20.0 }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let radius = ctx.get_integer("radius").unwrap_or(2);
        if radius < 1 {
            return Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: "Radius must be at least 1".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let radius = ctx.get_integer("radius").unwrap_or(2) as u32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let luma = img_data.to_luma8();
        let filtered = imageproc::filter::median_filter(&luma, radius, radius);
        let result = ImageValue::new(image::DynamicImage::ImageLuma8(filtered));

        ctx.set_output("image", Value::Image(result))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Applies directional motion blur to simulate camera or object movement.
#[derive(Debug, Clone)]
pub struct MotionBlur;

impl FilterNode for MotionBlur {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("motion_blur", "Motion Blur")
            .description("Apply directional motion blur to simulate camera or object movement")
            .category(Category::Blur)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Motion-blurred image"),
            )
            .parameter(
                ParameterDefinition::new("length", PortType::Integer, Value::Integer(10))
                    .with_description("Blur length in pixels")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 1.0, max: 100.0 }),
            )
            .parameter(
                ParameterDefinition::new("angle", PortType::Float, Value::Float(0.0))
                    .with_description("Blur direction in degrees (0=horizontal, 90=vertical)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 360.0 }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let length = ctx.get_integer("length").unwrap_or(10);
        if length < 1 {
            return Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: "Motion blur length must be at least 1".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let length = ctx.get_integer("length").unwrap_or(10) as usize;
        let angle_deg = ctx.get_float("angle").unwrap_or(0.0);

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img_data.to_rgba8();
        let (width, height) = (rgba.width() as usize, rgba.height() as usize);

        // Build 1D motion blur kernel along the angle direction
        let angle_rad = angle_deg.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        // Create kernel: list of (dx, dy) offsets along the motion direction
        let kernel_size = length.max(1);
        let half = kernel_size as f64 / 2.0;
        let weight = 1.0 / kernel_size as f64;

        let mut output = image::RgbaImage::new(width as u32, height as u32);

        for y in 0..height {
            for x in 0..width {
                let mut r_acc = 0.0_f64;
                let mut g_acc = 0.0_f64;
                let mut b_acc = 0.0_f64;
                let mut a_acc = 0.0_f64;

                for k in 0..kernel_size {
                    let offset = k as f64 - half;
                    let sx = (x as f64 + offset * cos_a).round() as i64;
                    let sy = (y as f64 + offset * sin_a).round() as i64;

                    // Clamp to image bounds
                    let sx = sx.clamp(0, width as i64 - 1) as u32;
                    let sy = sy.clamp(0, height as i64 - 1) as u32;

                    let pixel = rgba.get_pixel(sx, sy);
                    r_acc += pixel[0] as f64 * weight;
                    g_acc += pixel[1] as f64 * weight;
                    b_acc += pixel[2] as f64 * weight;
                    a_acc += pixel[3] as f64 * weight;
                }

                output.put_pixel(
                    x as u32,
                    y as u32,
                    image::Rgba([
                        r_acc.round() as u8,
                        g_acc.round() as u8,
                        b_acc.round() as u8,
                        a_acc.round() as u8,
                    ]),
                );
            }
        }

        let result = ImageValue::new(image::DynamicImage::ImageRgba8(output));
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

    #[test]
    fn test_median_blur_metadata() {
        let filter = MedianBlur;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "median_blur");
        assert_eq!(metadata.category, Category::Blur);
        assert_eq!(metadata.inputs.len(), 1);
        assert_eq!(metadata.outputs.len(), 1);
        assert_eq!(metadata.parameters.len(), 1);
    }

    #[test]
    fn test_motion_blur_metadata() {
        let filter = MotionBlur;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "motion_blur");
        assert_eq!(metadata.category, Category::Blur);
        assert_eq!(metadata.parameters.len(), 2);
    }

    // --- Execution tests ---

    fn make_test_image() -> crate::core::types::Value {
        use crate::core::types::{ImageValue, Value};
        let img = image::RgbaImage::from_fn(64, 64, |x, y| {
            image::Rgba([(x * 32) as u8, (y * 32) as u8, 128, 255])
        });
        Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(img)))
    }

    fn make_exec_ctx(filter: &dyn FilterNode, image: crate::core::types::Value) -> crate::core::context::ExecutionContext {
        use crate::core::error::NodeId;
        use crate::core::context::ExecutionContext;
        let mut ctx = ExecutionContext::new(NodeId::new());
        ctx.add_input("image", image);
        for p in &filter.metadata().parameters {
            ctx.add_parameter(p.name.clone(), p.default_value.clone());
        }
        ctx
    }

    #[test]
    fn test_gaussian_blur_execution() {
        let filter = GaussianBlur;
        let mut ctx = make_exec_ctx(&filter, make_test_image());
        assert!(filter.execute(&mut ctx).is_ok());
    }

    #[test]
    fn test_box_blur_execution() {
        let filter = BoxBlur;
        let mut ctx = make_exec_ctx(&filter, make_test_image());
        assert!(filter.execute(&mut ctx).is_ok());
    }

    #[test]
    fn test_median_blur_execution() {
        let filter = MedianBlur;
        let mut ctx = make_exec_ctx(&filter, make_test_image());
        assert!(filter.execute(&mut ctx).is_ok());
    }

    #[test]
    fn test_motion_blur_execution() {
        let filter = MotionBlur;
        let mut ctx = make_exec_ctx(&filter, make_test_image());
        assert!(filter.execute(&mut ctx).is_ok());
    }
}
