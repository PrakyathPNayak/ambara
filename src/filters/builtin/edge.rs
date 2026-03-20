//! Edge detection filters: EdgeDetect, Emboss

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::Pixel;

/// Register edge detection filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(EdgeDetect));
    registry.register(|| Box::new(Emboss));
}

/// Sobel edge detection.
#[derive(Debug, Clone)]
pub struct EdgeDetect;

impl FilterNode for EdgeDetect {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("edge_detect", "Edge Detect")
            .description("Detect edges using the Sobel operator")
            .category(Category::Edge)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Edge-detected image"),
            )
            .parameter(
                ParameterDefinition::new("method", PortType::String, Value::String("sobel".to_string()))
                    .with_description("Edge detection method")
                    .with_ui_hint(UiHint::Dropdown {
                        options: vec![
                            "sobel".to_string(),
                            "prewitt".to_string(),
                        ],
                    }),
            )
            .parameter(
                ParameterDefinition::new("invert", PortType::Boolean, Value::Boolean(false))
                    .with_description("Invert edges (white background, dark edges)"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let method = ctx.get_string("method").unwrap_or("sobel");
        let invert = ctx.get_bool("invert").unwrap_or(false);

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let gray = img_data.to_luma8();
        let (w, h) = gray.dimensions();

        // Sobel or Prewitt kernels
        let (kx, ky): ([[i32; 3]; 3], [[i32; 3]; 3]) = match method.as_ref() {
            "prewitt" => (
                [[-1, 0, 1], [-1, 0, 1], [-1, 0, 1]],
                [[-1, -1, -1], [0, 0, 0], [1, 1, 1]],
            ),
            _ => (
                [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]],
                [[-1, -2, -1], [0, 0, 0], [1, 2, 1]],
            ),
        };

        let mut result = image::RgbaImage::new(w, h);
        for y in 1..h.saturating_sub(1) {
            for x in 1..w.saturating_sub(1) {
                let mut gx: i32 = 0;
                let mut gy: i32 = 0;
                for ky_idx in 0..3 {
                    for kx_idx in 0..3 {
                        let px = gray.get_pixel(
                            x + kx_idx - 1,
                            y + ky_idx - 1,
                        )
                        .channels()[0] as i32;
                        gx += px * kx[ky_idx as usize][kx_idx as usize];
                        gy += px * ky[ky_idx as usize][kx_idx as usize];
                    }
                }
                let magnitude = ((gx * gx + gy * gy) as f64).sqrt().min(255.0) as u8;
                let val = if invert { 255 - magnitude } else { magnitude };
                result.put_pixel(x, y, image::Rgba([val, val, val, 255]));
            }
        }

        // Fill border pixels
        for x in 0..w {
            let bg = if invert { 255u8 } else { 0u8 };
            result.put_pixel(x, 0, image::Rgba([bg, bg, bg, 255]));
            if h > 1 {
                result.put_pixel(x, h - 1, image::Rgba([bg, bg, bg, 255]));
            }
        }
        for y in 0..h {
            let bg = if invert { 255u8 } else { 0u8 };
            result.put_pixel(0, y, image::Rgba([bg, bg, bg, 255]));
            if w > 1 {
                result.put_pixel(w - 1, y, image::Rgba([bg, bg, bg, 255]));
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

/// Emboss effect using a directional convolution kernel.
#[derive(Debug, Clone)]
pub struct Emboss;

impl FilterNode for Emboss {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("emboss", "Emboss")
            .description("Apply an emboss/relief effect to an image")
            .category(Category::Edge)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Embossed image"),
            )
            .parameter(
                ParameterDefinition::new("direction", PortType::String, Value::String("top_left".to_string()))
                    .with_description("Light direction for emboss")
                    .with_ui_hint(UiHint::Dropdown {
                        options: vec![
                            "top_left".to_string(),
                            "top_right".to_string(),
                            "bottom_left".to_string(),
                            "bottom_right".to_string(),
                        ],
                    }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let direction = ctx.get_string("direction").unwrap_or("top_left");

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img_data.to_rgba8();
        let (w, h) = rgba.dimensions();

        // Emboss kernels based on direction
        let kernel: [[i32; 3]; 3] = match direction.as_ref() {
            "top_right" => [
                [0, 1, 2],
                [-1, 1, 1],
                [-2, -1, 0],
            ],
            "bottom_left" => [
                [0, -1, -2],
                [1, 1, -1],
                [2, 1, 0],
            ],
            "bottom_right" => [
                [-2, -1, 0],
                [-1, 1, 1],
                [0, 1, 2],
            ],
            _ => [
                [2, 1, 0],
                [1, 1, -1],
                [0, -1, -2],
            ],
        };

        let mut result = image::RgbaImage::new(w, h);
        for y in 1..h.saturating_sub(1) {
            for x in 1..w.saturating_sub(1) {
                let mut out = [128u8; 4];
                for c in 0..3 {
                    let mut sum: i32 = 0;
                    for ky in 0..3 {
                        for kx in 0..3 {
                            let px = rgba.get_pixel(x + kx - 1, y + ky - 1).channels()[c] as i32;
                            sum += px * kernel[ky as usize][kx as usize];
                        }
                    }
                    out[c] = (sum + 128).clamp(0, 255) as u8;
                }
                out[3] = rgba.get_pixel(x, y).channels()[3];
                result.put_pixel(x, y, image::Rgba(out));
            }
        }

        // Copy border from original
        for x in 0..w {
            result.put_pixel(x, 0, *rgba.get_pixel(x, 0));
            if h > 1 {
                result.put_pixel(x, h - 1, *rgba.get_pixel(x, h - 1));
            }
        }
        for y in 0..h {
            result.put_pixel(0, y, *rgba.get_pixel(0, y));
            if w > 1 {
                result.put_pixel(w - 1, y, *rgba.get_pixel(w - 1, y));
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
