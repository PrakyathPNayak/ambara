//! Sharpening filters: UnsharpMask, Sharpen

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::Pixel;

/// Register sharpen filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(UnsharpMask));
    registry.register(|| Box::new(Sharpen));
}

/// Unsharp mask sharpening: sharpens an image by subtracting a blurred version.
#[derive(Debug, Clone)]
pub struct UnsharpMask;

impl FilterNode for UnsharpMask {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("unsharp_mask", "Unsharp Mask")
            .description("Sharpen an image using unsharp masking (blur + subtract)")
            .category(Category::Sharpen)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Sharpened image"),
            )
            .parameter(
                ParameterDefinition::new("sigma", PortType::Float, Value::Float(1.0))
                    .with_description("Blur radius for the mask (higher = stronger effect)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.1, max: 20.0 }),
            )
            .parameter(
                ParameterDefinition::new("amount", PortType::Float, Value::Float(1.0))
                    .with_description("Sharpening strength (0.0 to 5.0)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 5.0 }),
            )
            .parameter(
                ParameterDefinition::new("threshold", PortType::Integer, Value::Integer(0))
                    .with_description("Minimum difference to sharpen (reduces noise sharpening)")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let sigma = ctx.get_float("sigma").unwrap_or(1.0) as f32;
        let amount = ctx.get_float("amount").unwrap_or(1.0) as f32;
        let threshold = ctx.get_integer("threshold").unwrap_or(0) as i32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img_data.to_rgba8();
        let blurred = imageproc::filter::gaussian_blur_f32(&rgba, sigma);

        let (w, h) = rgba.dimensions();
        let mut result = image::RgbaImage::new(w, h);

        for (x, y, orig) in rgba.enumerate_pixels() {
            let blur_px = blurred.get_pixel(x, y);
            let orig_ch = orig.channels();
            let blur_ch = blur_px.channels();
            let mut out = [0u8; 4];
            for i in 0..3 {
                let diff = (orig_ch[i] as i32) - (blur_ch[i] as i32);
                if diff.abs() >= threshold {
                    out[i] = ((orig_ch[i] as f32) + amount * diff as f32)
                        .clamp(0.0, 255.0) as u8;
                } else {
                    out[i] = orig_ch[i];
                }
            }
            out[3] = orig_ch[3]; // preserve alpha
            result.put_pixel(x, y, image::Rgba(out));
        }

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(result))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Simple 3×3 sharpen kernel convolution.
#[derive(Debug, Clone)]
pub struct Sharpen;

impl FilterNode for Sharpen {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("sharpen", "Sharpen")
            .description("Apply a sharpening convolution kernel to an image")
            .category(Category::Sharpen)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Sharpened image"),
            )
            .parameter(
                ParameterDefinition::new("strength", PortType::Float, Value::Float(1.0))
                    .with_description("Sharpening strength (0.0 = none, 1.0 = standard)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 5.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let strength = ctx.get_float("strength").unwrap_or(1.0) as f32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img_data.to_rgba8();
        let (w, h) = rgba.dimensions();

        // Sharpen kernel: [0 -s 0; -s 1+4s -s; 0 -s 0]
        let mut result = image::RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let orig = rgba.get_pixel(x, y).0;
                if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                    result.put_pixel(x, y, image::Rgba(orig));
                    continue;
                }
                let top = rgba.get_pixel(x, y - 1).0;
                let bot = rgba.get_pixel(x, y + 1).0;
                let lft = rgba.get_pixel(x - 1, y).0;
                let rgt = rgba.get_pixel(x + 1, y).0;

                let mut out = [0u8; 4];
                for i in 0..3 {
                    let center = orig[i] as f32;
                    let neighbors = top[i] as f32 + bot[i] as f32 + lft[i] as f32 + rgt[i] as f32;
                    let sharpened = center + strength * (4.0 * center - neighbors);
                    out[i] = sharpened.clamp(0.0, 255.0) as u8;
                }
                out[3] = orig[3];
                result.put_pixel(x, y, image::Rgba(out));
            }
        }

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(result))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}
