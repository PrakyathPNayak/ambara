//! Composite filters: Blend, Overlay

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::Rgba;

/// Register composite filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(Blend));
    registry.register(|| Box::new(Overlay));
}

/// Blends two images together.
#[derive(Debug, Clone)]
pub struct Blend;

impl FilterNode for Blend {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("blend", "Blend")
            .description("Blend two images together using various blend modes")
            .category(Category::Composite)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("base", PortType::Image)
                    .with_description("Base/background image")
            )
            .input(
                PortDefinition::input("blend", PortType::Image)
                    .with_description("Image to blend on top")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Blended result")
            )
            .parameter(
                ParameterDefinition::new("mode", PortType::String, Value::String("normal".to_string()))
                    .with_description("Blend mode")
                    .with_ui_hint(UiHint::Dropdown {
                        options: vec![
                            "normal".to_string(),
                            "multiply".to_string(),
                            "screen".to_string(),
                            "overlay".to_string(),
                            "darken".to_string(),
                            "lighten".to_string(),
                            "add".to_string(),
                            "subtract".to_string(),
                            "difference".to_string(),
                        ]
                    })
                    .with_constraint(Constraint::OneOf(vec![
                        Value::String("normal".to_string()),
                        Value::String("multiply".to_string()),
                        Value::String("screen".to_string()),
                        Value::String("overlay".to_string()),
                        Value::String("darken".to_string()),
                        Value::String("lighten".to_string()),
                        Value::String("add".to_string()),
                        Value::String("subtract".to_string()),
                        Value::String("difference".to_string()),
                    ])),
            )
            .parameter(
                ParameterDefinition::new("opacity", PortType::Float, Value::Float(1.0))
                    .with_description("Blend layer opacity")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 1.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let base_img = ctx.get_input_image("base")?;

        let blend_img = ctx.get_input_image("blend")?;

        let mode = ctx.get_string("mode").unwrap_or("normal");
        let opacity = ctx.get_float("opacity").unwrap_or(1.0) as f32;

        let base_data = base_img.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Base image has no data".to_string(),
        })?;

        let blend_data = blend_img.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Blend image has no data".to_string(),
        })?;

        let base_rgba = base_data.to_rgba8();
        let blend_rgba = blend_data.to_rgba8();
        let (width, height) = base_rgba.dimensions();
        let (blend_width, blend_height) = blend_rgba.dimensions();

        // Resize blend image if needed
        let blend_resized = if blend_width != width || blend_height != height {
            image::imageops::resize(&blend_rgba, width, height, image::imageops::FilterType::Lanczos3)
        } else {
            blend_rgba
        };

        let mut result = base_rgba;

        for (x, y, base_pixel) in result.enumerate_pixels_mut() {
            let blend_pixel = blend_resized.get_pixel(x, y);
            *base_pixel = blend_pixels(base_pixel, blend_pixel, mode, opacity);
        }

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(result));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Blend two pixels together.
fn blend_pixels(base: &Rgba<u8>, blend: &Rgba<u8>, mode: &str, opacity: f32) -> Rgba<u8> {
    let b = [
        base[0] as f32 / 255.0,
        base[1] as f32 / 255.0,
        base[2] as f32 / 255.0,
        base[3] as f32 / 255.0,
    ];
    let l = [
        blend[0] as f32 / 255.0,
        blend[1] as f32 / 255.0,
        blend[2] as f32 / 255.0,
        blend[3] as f32 / 255.0,
    ];

    let blended = match mode {
        "multiply" => [b[0] * l[0], b[1] * l[1], b[2] * l[2]],
        "screen" => [
            1.0 - (1.0 - b[0]) * (1.0 - l[0]),
            1.0 - (1.0 - b[1]) * (1.0 - l[1]),
            1.0 - (1.0 - b[2]) * (1.0 - l[2]),
        ],
        "overlay" => [
            overlay_channel(b[0], l[0]),
            overlay_channel(b[1], l[1]),
            overlay_channel(b[2], l[2]),
        ],
        "darken" => [b[0].min(l[0]), b[1].min(l[1]), b[2].min(l[2])],
        "lighten" => [b[0].max(l[0]), b[1].max(l[1]), b[2].max(l[2])],
        "add" => [
            (b[0] + l[0]).min(1.0),
            (b[1] + l[1]).min(1.0),
            (b[2] + l[2]).min(1.0),
        ],
        "subtract" => [
            (b[0] - l[0]).max(0.0),
            (b[1] - l[1]).max(0.0),
            (b[2] - l[2]).max(0.0),
        ],
        "difference" => [
            (b[0] - l[0]).abs(),
            (b[1] - l[1]).abs(),
            (b[2] - l[2]).abs(),
        ],
        _ => [l[0], l[1], l[2]], // normal
    };

    // Apply opacity and blend layer alpha
    let layer_alpha = l[3] * opacity;
    let final_r = b[0] * (1.0 - layer_alpha) + blended[0] * layer_alpha;
    let final_g = b[1] * (1.0 - layer_alpha) + blended[1] * layer_alpha;
    let final_b = b[2] * (1.0 - layer_alpha) + blended[2] * layer_alpha;
    let final_a = b[3] + layer_alpha * (1.0 - b[3]);

    Rgba([
        (final_r.clamp(0.0, 1.0) * 255.0) as u8,
        (final_g.clamp(0.0, 1.0) * 255.0) as u8,
        (final_b.clamp(0.0, 1.0) * 255.0) as u8,
        (final_a.clamp(0.0, 1.0) * 255.0) as u8,
    ])
}

fn overlay_channel(base: f32, blend: f32) -> f32 {
    if base < 0.5 {
        2.0 * base * blend
    } else {
        1.0 - 2.0 * (1.0 - base) * (1.0 - blend)
    }
}

/// Overlays one image on top of another at a specific position.
#[derive(Debug, Clone)]
pub struct Overlay;

impl FilterNode for Overlay {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("overlay", "Overlay")
            .description("Place one image on top of another at a specific position")
            .category(Category::Composite)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("base", PortType::Image)
                    .with_description("Base/background image")
            )
            .input(
                PortDefinition::input("overlay", PortType::Image)
                    .with_description("Image to place on top")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Composited result")
            )
            .parameter(
                ParameterDefinition::new("x", PortType::Integer, Value::Integer(0))
                    .with_description("X position of overlay")
                    .with_ui_hint(UiHint::SpinBox),
            )
            .parameter(
                ParameterDefinition::new("y", PortType::Integer, Value::Integer(0))
                    .with_description("Y position of overlay")
                    .with_ui_hint(UiHint::SpinBox),
            )
            .parameter(
                ParameterDefinition::new("opacity", PortType::Float, Value::Float(1.0))
                    .with_description("Overlay opacity")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 1.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let base_img = ctx.get_input_image("base")?;

        let overlay_img = ctx.get_input_image("overlay")?;

        let pos_x = ctx.get_integer("x").unwrap_or(0);
        let pos_y = ctx.get_integer("y").unwrap_or(0);
        let opacity = ctx.get_float("opacity").unwrap_or(1.0) as f32;

        let base_data = base_img.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Base image has no data".to_string(),
        })?;

        let overlay_data = overlay_img.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Overlay image has no data".to_string(),
        })?;

        let mut result = base_data.to_rgba8();
        let overlay_rgba = overlay_data.to_rgba8();
        let (base_width, base_height) = result.dimensions();
        let (overlay_width, overlay_height) = overlay_rgba.dimensions();

        // Composite the overlay onto the base
        for oy in 0..overlay_height {
            for ox in 0..overlay_width {
                let bx = pos_x + ox as i64;
                let by = pos_y + oy as i64;

                // Skip if outside bounds
                if bx < 0 || by < 0 || bx >= base_width as i64 || by >= base_height as i64 {
                    continue;
                }

                let bx = bx as u32;
                let by = by as u32;

                let base_pixel = result.get_pixel(bx, by);
                let overlay_pixel = overlay_rgba.get_pixel(ox, oy);

                // Alpha composite
                let src_a = (overlay_pixel[3] as f32 / 255.0) * opacity;
                let dst_a = base_pixel[3] as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);

                if out_a > 0.0 {
                    let r = (overlay_pixel[0] as f32 * src_a + base_pixel[0] as f32 * dst_a * (1.0 - src_a)) / out_a;
                    let g = (overlay_pixel[1] as f32 * src_a + base_pixel[1] as f32 * dst_a * (1.0 - src_a)) / out_a;
                    let b = (overlay_pixel[2] as f32 * src_a + base_pixel[2] as f32 * dst_a * (1.0 - src_a)) / out_a;

                    result.put_pixel(bx, by, Rgba([
                        r.clamp(0.0, 255.0) as u8,
                        g.clamp(0.0, 255.0) as u8,
                        b.clamp(0.0, 255.0) as u8,
                        (out_a * 255.0).clamp(0.0, 255.0) as u8,
                    ]));
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_metadata() {
        let filter = Blend;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "blend");
        assert_eq!(metadata.category, Category::Composite);
        assert_eq!(metadata.inputs.len(), 2);
    }

    #[test]
    fn test_overlay_metadata() {
        let filter = Overlay;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "overlay");
        assert_eq!(metadata.parameters.len(), 3);
    }
}
