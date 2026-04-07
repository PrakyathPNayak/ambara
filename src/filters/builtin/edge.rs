//! Edge detection filters: EdgeDetect, Emboss

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::Pixel;

/// Register edge detection filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(EdgeDetect));
    registry.register(|| Box::new(Emboss));
    registry.register(|| Box::new(CannyEdge));
    registry.register(|| Box::new(Laplacian));
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
        let (kx, ky): ([[i32; 3]; 3], [[i32; 3]; 3]) = match method {
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
        let kernel: [[i32; 3]; 3] = match direction {
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
                for (c, out_ch) in out.iter_mut().enumerate().take(3) {
                    let mut sum: i32 = 0;
                    for ky in 0..3 {
                        for kx in 0..3 {
                            let px = rgba.get_pixel(x + kx - 1, y + ky - 1).channels()[c] as i32;
                            sum += px * kernel[ky as usize][kx as usize];
                        }
                    }
                    *out_ch = (sum + 128).clamp(0, 255) as u8;
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

/// Canny edge detection with adjustable thresholds.
#[derive(Debug, Clone)]
pub struct CannyEdge;

impl FilterNode for CannyEdge {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("canny_edge", "Canny Edge Detection")
            .description("Detect edges using the Canny algorithm with adjustable thresholds")
            .category(Category::Edge)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Edge map (white edges on black)"),
            )
            .parameter(
                ParameterDefinition::new("low_threshold", PortType::Float, Value::Float(50.0))
                    .with_description("Lower hysteresis threshold")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .parameter(
                ParameterDefinition::new("high_threshold", PortType::Float, Value::Float(150.0))
                    .with_description("Upper hysteresis threshold")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let low = ctx.get_float("low_threshold").unwrap_or(50.0);
        let high = ctx.get_float("high_threshold").unwrap_or(150.0);
        if low >= high {
            return Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: "Low threshold must be less than high threshold".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let low = ctx.get_float("low_threshold").unwrap_or(50.0) as f32;
        let high = ctx.get_float("high_threshold").unwrap_or(150.0) as f32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let luma = img_data.to_luma8();
        let edges = imageproc::edges::canny(&luma, low, high);

        let result = ImageValue::new(image::DynamicImage::ImageLuma8(edges));
        ctx.set_output("image", Value::Image(result))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Laplacian edge detection for second-derivative edge highlighting.
#[derive(Debug, Clone)]
pub struct Laplacian;

impl FilterNode for Laplacian {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("laplacian", "Laplacian Edge Detection")
            .description("Detect edges using the Laplacian second-derivative operator")
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
                ParameterDefinition::new("normalize", PortType::Boolean, Value::Boolean(true))
                    .with_description("Normalize output to full 0-255 range"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let normalize = ctx.get_bool("normalize").unwrap_or(true);

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img_data.to_rgba8();
        let (w, h) = rgba.dimensions();

        let kernel: [[i32; 3]; 3] = [
            [0, 1, 0],
            [1, -4, 1],
            [0, 1, 0],
        ];

        let mut buffer: Vec<i32> = vec![0; (w * h) as usize];
        let mut min_val = i32::MAX;
        let mut max_val = i32::MIN;

        for y in 1..h.saturating_sub(1) {
            for x in 1..w.saturating_sub(1) {
                let mut sum: i32 = 0;
                for ky in 0..3u32 {
                    for kx in 0..3u32 {
                        let luma = {
                            let p = rgba.get_pixel(x + kx - 1, y + ky - 1).channels();
                            (p[0] as i32 * 299 + p[1] as i32 * 587 + p[2] as i32 * 114) / 1000
                        };
                        sum += luma * kernel[ky as usize][kx as usize];
                    }
                }
                let idx = (y * w + x) as usize;
                buffer[idx] = sum;
                min_val = min_val.min(sum);
                max_val = max_val.max(sum);
            }
        }

        let mut result = image::RgbaImage::new(w, h);
        let range = (max_val - min_val).max(1) as f64;

        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let val = if normalize {
                    ((buffer[idx] - min_val) as f64 / range * 255.0).round() as u8
                } else {
                    buffer[idx].unsigned_abs().min(255) as u8
                };
                result.put_pixel(x, y, image::Rgba([val, val, val, 255]));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_detect_metadata() {
        let filter = EdgeDetect;
        let m = filter.metadata();
        assert_eq!(m.id, "edge_detect");
        assert_eq!(m.category, Category::Edge);
    }

    #[test]
    fn test_canny_edge_metadata() {
        let filter = CannyEdge;
        let m = filter.metadata();
        assert_eq!(m.id, "canny_edge");
        assert_eq!(m.category, Category::Edge);
        assert_eq!(m.parameters.len(), 2);
    }

    #[test]
    fn test_laplacian_metadata() {
        let filter = Laplacian;
        let m = filter.metadata();
        assert_eq!(m.id, "laplacian");
        assert_eq!(m.category, Category::Edge);
    }

    // --- Execution tests ---

    fn make_test_image() -> crate::core::types::Value {
        use crate::core::types::{ImageValue, Value};
        let img = image::RgbaImage::from_fn(64, 64, |x, y| {
            // Create a pattern with sharp edges for edge detection
            let v = if x < 8 && y < 8 { 255u8 } else { 0u8 };
            image::Rgba([v, v, v, 255])
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
    fn test_edge_detect_execution() {
        let filter = EdgeDetect;
        let mut ctx = make_exec_ctx(&filter, make_test_image());
        assert!(filter.execute(&mut ctx).is_ok());
    }

    #[test]
    fn test_canny_edge_execution() {
        let filter = CannyEdge;
        let mut ctx = make_exec_ctx(&filter, make_test_image());
        assert!(filter.execute(&mut ctx).is_ok());
    }

    #[test]
    fn test_laplacian_execution() {
        let filter = Laplacian;
        let mut ctx = make_exec_ctx(&filter, make_test_image());
        assert!(filter.execute(&mut ctx).is_ok());
    }

    #[test]
    fn test_emboss_execution() {
        let filter = Emboss;
        let mut ctx = make_exec_ctx(&filter, make_test_image());
        assert!(filter.execute(&mut ctx).is_ok());
    }
}
