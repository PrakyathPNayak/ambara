//! Drawing filters: DrawRectangle, DrawCircle, DrawLine

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;

/// Register draw filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(DrawRectangle));
    registry.register(|| Box::new(DrawCircle));
    registry.register(|| Box::new(DrawLine));
}

/// Draw a rectangle on an image.
#[derive(Debug, Clone)]
pub struct DrawRectangle;

impl FilterNode for DrawRectangle {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("draw_rectangle", "Draw Rectangle")
            .description("Draw a filled or outlined rectangle on an image")
            .category(Category::Draw)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Image with rectangle drawn"),
            )
            .parameter(
                ParameterDefinition::new("x", PortType::Integer, Value::Integer(10))
                    .with_description("Top-left X coordinate")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::MinValue(0.0)),
            )
            .parameter(
                ParameterDefinition::new("y", PortType::Integer, Value::Integer(10))
                    .with_description("Top-left Y coordinate")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::MinValue(0.0)),
            )
            .parameter(
                ParameterDefinition::new("width", PortType::Integer, Value::Integer(100))
                    .with_description("Rectangle width")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::MinValue(1.0)),
            )
            .parameter(
                ParameterDefinition::new("height", PortType::Integer, Value::Integer(100))
                    .with_description("Rectangle height")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::MinValue(1.0)),
            )
            .parameter(
                ParameterDefinition::new("color_r", PortType::Integer, Value::Integer(255))
                    .with_description("Red component (0-255)")
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .parameter(
                ParameterDefinition::new("color_g", PortType::Integer, Value::Integer(0))
                    .with_description("Green component (0-255)")
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .parameter(
                ParameterDefinition::new("color_b", PortType::Integer, Value::Integer(0))
                    .with_description("Blue component (0-255)")
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .parameter(
                ParameterDefinition::new("filled", PortType::Boolean, Value::Boolean(true))
                    .with_description("Fill the rectangle (false = outline only)"),
            )
            .parameter(
                ParameterDefinition::new("thickness", PortType::Integer, Value::Integer(2))
                    .with_description("Line thickness for outline mode")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::Range { min: 1.0, max: 50.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let rx = ctx.get_integer("x").unwrap_or(10).max(0) as u32;
        let ry = ctx.get_integer("y").unwrap_or(10).max(0) as u32;
        let rw = ctx.get_integer("width").unwrap_or(100).max(1) as u32;
        let rh = ctx.get_integer("height").unwrap_or(100).max(1) as u32;
        let r = ctx.get_integer("color_r").unwrap_or(255) as u8;
        let g = ctx.get_integer("color_g").unwrap_or(0) as u8;
        let b = ctx.get_integer("color_b").unwrap_or(0) as u8;
        let filled = ctx.get_bool("filled").unwrap_or(true);
        let thickness = ctx.get_integer("thickness").unwrap_or(2).max(1) as u32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let mut rgba = img_data.to_rgba8();
        let (w, h) = rgba.dimensions();
        let color = image::Rgba([r, g, b, 255]);

        if filled {
            for py in ry..=(ry + rh).min(h - 1) {
                for px in rx..=(rx + rw).min(w - 1) {
                    rgba.put_pixel(px, py, color);
                }
            }
        } else {
            // Draw outline with thickness
            let t = thickness;
            for py in ry..(ry + rh).min(h) {
                for px in rx..(rx + rw).min(w) {
                    let on_top = py < ry + t;
                    let on_bottom = py >= (ry + rh).saturating_sub(t);
                    let on_left = px < rx + t;
                    let on_right = px >= (rx + rw).saturating_sub(t);
                    if on_top || on_bottom || on_left || on_right {
                        rgba.put_pixel(px, py, color);
                    }
                }
            }
        }

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(rgba))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Draw a circle on an image.
#[derive(Debug, Clone)]
pub struct DrawCircle;

impl FilterNode for DrawCircle {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("draw_circle", "Draw Circle")
            .description("Draw a filled or outlined circle on an image")
            .category(Category::Draw)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Image with circle drawn"),
            )
            .parameter(
                ParameterDefinition::new("center_x", PortType::Integer, Value::Integer(100))
                    .with_description("Center X coordinate")
                    .with_ui_hint(UiHint::SpinBox),
            )
            .parameter(
                ParameterDefinition::new("center_y", PortType::Integer, Value::Integer(100))
                    .with_description("Center Y coordinate")
                    .with_ui_hint(UiHint::SpinBox),
            )
            .parameter(
                ParameterDefinition::new("radius", PortType::Integer, Value::Integer(50))
                    .with_description("Circle radius")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::MinValue(1.0)),
            )
            .parameter(
                ParameterDefinition::new("color_r", PortType::Integer, Value::Integer(0))
                    .with_description("Red component (0-255)")
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .parameter(
                ParameterDefinition::new("color_g", PortType::Integer, Value::Integer(255))
                    .with_description("Green component (0-255)")
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .parameter(
                ParameterDefinition::new("color_b", PortType::Integer, Value::Integer(0))
                    .with_description("Blue component (0-255)")
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .parameter(
                ParameterDefinition::new("filled", PortType::Boolean, Value::Boolean(true))
                    .with_description("Fill the circle"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let cx = ctx.get_integer("center_x").unwrap_or(100) as i32;
        let cy = ctx.get_integer("center_y").unwrap_or(100) as i32;
        let radius = ctx.get_integer("radius").unwrap_or(50).max(1) as i32;
        let r = ctx.get_integer("color_r").unwrap_or(0) as u8;
        let g = ctx.get_integer("color_g").unwrap_or(255) as u8;
        let b = ctx.get_integer("color_b").unwrap_or(0) as u8;
        let filled = ctx.get_bool("filled").unwrap_or(true);

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let mut rgba = img_data.to_rgba8();
        let (w, h) = rgba.dimensions();
        let color = image::Rgba([r, g, b, 255]);
        let r2 = (radius * radius) as i64;

        for py in (cy - radius).max(0)..(cy + radius + 1).min(h as i32) {
            for px in (cx - radius).max(0)..(cx + radius + 1).min(w as i32) {
                let dx = (px - cx) as i64;
                let dy = (py - cy) as i64;
                let dist2 = dx * dx + dy * dy;
                if filled {
                    if dist2 <= r2 {
                        rgba.put_pixel(px as u32, py as u32, color);
                    }
                } else {
                    // Outline: within 2 pixels of the circumference
                    let outer = ((radius + 2) * (radius + 2)) as i64;
                    let inner = ((radius - 2).max(0) * (radius - 2).max(0)) as i64;
                    if dist2 <= outer && dist2 >= inner {
                        rgba.put_pixel(px as u32, py as u32, color);
                    }
                }
            }
        }

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(rgba))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Draw a line on an image using Bresenham's algorithm.
#[derive(Debug, Clone)]
pub struct DrawLine;

impl FilterNode for DrawLine {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("draw_line", "Draw Line")
            .description("Draw a straight line between two points on an image")
            .category(Category::Draw)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Image with line drawn"),
            )
            .parameter(
                ParameterDefinition::new("x1", PortType::Integer, Value::Integer(0))
                    .with_description("Start X coordinate"),
            )
            .parameter(
                ParameterDefinition::new("y1", PortType::Integer, Value::Integer(0))
                    .with_description("Start Y coordinate"),
            )
            .parameter(
                ParameterDefinition::new("x2", PortType::Integer, Value::Integer(100))
                    .with_description("End X coordinate"),
            )
            .parameter(
                ParameterDefinition::new("y2", PortType::Integer, Value::Integer(100))
                    .with_description("End Y coordinate"),
            )
            .parameter(
                ParameterDefinition::new("color_r", PortType::Integer, Value::Integer(255))
                    .with_description("Red component (0-255)")
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .parameter(
                ParameterDefinition::new("color_g", PortType::Integer, Value::Integer(255))
                    .with_description("Green component (0-255)")
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .parameter(
                ParameterDefinition::new("color_b", PortType::Integer, Value::Integer(255))
                    .with_description("Blue component (0-255)")
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .parameter(
                ParameterDefinition::new("thickness", PortType::Integer, Value::Integer(2))
                    .with_description("Line thickness in pixels")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::Range { min: 1.0, max: 50.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let x1 = ctx.get_integer("x1").unwrap_or(0) as i32;
        let y1 = ctx.get_integer("y1").unwrap_or(0) as i32;
        let x2 = ctx.get_integer("x2").unwrap_or(100) as i32;
        let y2 = ctx.get_integer("y2").unwrap_or(100) as i32;
        let r = ctx.get_integer("color_r").unwrap_or(255) as u8;
        let g = ctx.get_integer("color_g").unwrap_or(255) as u8;
        let b = ctx.get_integer("color_b").unwrap_or(255) as u8;
        let thickness = ctx.get_integer("thickness").unwrap_or(2).max(1) as i32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let mut rgba = img_data.to_rgba8();
        let (w, h) = rgba.dimensions();
        let color = image::Rgba([r, g, b, 255]);
        let half_t = thickness / 2;

        // Bresenham's line algorithm
        let dx = (x2 - x1).abs();
        let dy = -(y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut cx = x1;
        let mut cy = y1;

        loop {
            // Draw a square around the point for thickness
            for ty in -half_t..=half_t {
                for tx in -half_t..=half_t {
                    let px = cx + tx;
                    let py = cy + ty;
                    if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                        rgba.put_pixel(px as u32, py as u32, color);
                    }
                }
            }

            if cx == x2 && cy == y2 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                err += dx;
                cy += sy;
            }
        }

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(rgba))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}
