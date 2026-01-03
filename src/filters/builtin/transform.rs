//! Transform filters: Resize, Rotate, Flip, Crop

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::imageops::FilterType;
use image::GenericImageView;

/// Register transform filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(Resize));
    registry.register(|| Box::new(Rotate));
    registry.register(|| Box::new(Flip));
    registry.register(|| Box::new(Crop));
}

/// Resizes an image.
#[derive(Debug, Clone)]
pub struct Resize;

impl FilterNode for Resize {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("resize", "Resize")
            .description("Resize an image to specified dimensions")
            .category(Category::Transform)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Resized image")
            )
            .parameter(
                ParameterDefinition::new("width", PortType::Integer, Value::Integer(1920))
                    .with_description("Target width in pixels")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::Range { min: 1.0, max: 16384.0 }),
            )
            .parameter(
                ParameterDefinition::new("height", PortType::Integer, Value::Integer(1080))
                    .with_description("Target height in pixels")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::Range { min: 1.0, max: 16384.0 }),
            )
            .parameter(
                ParameterDefinition::new("preserve_aspect", PortType::Boolean, Value::Boolean(true))
                    .with_description("Preserve aspect ratio (uses width as primary)"),
            )
            .parameter(
                ParameterDefinition::new("filter", PortType::String, Value::String("lanczos3".to_string()))
                    .with_description("Resampling filter")
                    .with_ui_hint(UiHint::Dropdown)
                    .with_constraint(Constraint::OneOf(vec![
                        Value::String("nearest".to_string()),
                        Value::String("triangle".to_string()),
                        Value::String("catmullrom".to_string()),
                        Value::String("gaussian".to_string()),
                        Value::String("lanczos3".to_string()),
                    ])),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let width = ctx.get_integer("width").unwrap_or(1920);
        let height = ctx.get_integer("height").unwrap_or(1080);

        if width < 1 || height < 1 {
            return Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: "Width and height must be at least 1".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let target_width = ctx.get_integer("width").unwrap_or(1920) as u32;
        let target_height = ctx.get_integer("height").unwrap_or(1080) as u32;
        let preserve_aspect = ctx.get_bool("preserve_aspect").unwrap_or(true);
        let filter_name = ctx.get_string("filter").unwrap_or("lanczos3");

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let (src_width, src_height) = img_data.dimensions();

        // Calculate dimensions
        let (final_width, final_height) = if preserve_aspect {
            let aspect = src_width as f32 / src_height as f32;
            let new_height = (target_width as f32 / aspect) as u32;
            if new_height <= target_height {
                (target_width, new_height)
            } else {
                let new_width = (target_height as f32 * aspect) as u32;
                (new_width, target_height)
            }
        } else {
            (target_width, target_height)
        };

        // Select filter
        let filter = match filter_name {
            "nearest" => FilterType::Nearest,
            "triangle" => FilterType::Triangle,
            "catmullrom" => FilterType::CatmullRom,
            "gaussian" => FilterType::Gaussian,
            _ => FilterType::Lanczos3,
        };

        let rgba = img_data.to_rgba8();
        let resized = image::imageops::resize(&rgba, final_width, final_height, filter);

        let result = ImageValue::new(image::DynamicImage::ImageRgba8(resized));

        ctx.set_output("image", Value::Image(result))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Rotates an image.
#[derive(Debug, Clone)]
pub struct Rotate;

impl FilterNode for Rotate {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("rotate", "Rotate")
            .description("Rotate an image by a specified angle")
            .category(Category::Transform)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Rotated image")
            )
            .parameter(
                ParameterDefinition::new("angle", PortType::String, Value::String("90".to_string()))
                    .with_description("Rotation angle")
                    .with_ui_hint(UiHint::Dropdown)
                    .with_constraint(Constraint::OneOf(vec![
                        Value::String("90".to_string()),
                        Value::String("180".to_string()),
                        Value::String("270".to_string()),
                    ])),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let angle = ctx.get_string("angle").unwrap_or("90");

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img_data.to_rgba8();
        let rotated = match angle {
            "90" => image::imageops::rotate90(&rgba),
            "180" => image::imageops::rotate180(&rgba),
            "270" => image::imageops::rotate270(&rgba),
            _ => rgba,
        };

        let result = ImageValue::new(image::DynamicImage::ImageRgba8(rotated));

        ctx.set_output("image", Value::Image(result))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Flips an image.
#[derive(Debug, Clone)]
pub struct Flip;

impl FilterNode for Flip {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("flip", "Flip")
            .description("Flip an image horizontally or vertically")
            .category(Category::Transform)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Flipped image")
            )
            .parameter(
                ParameterDefinition::new("horizontal", PortType::Boolean, Value::Boolean(true))
                    .with_description("Flip horizontally"),
            )
            .parameter(
                ParameterDefinition::new("vertical", PortType::Boolean, Value::Boolean(false))
                    .with_description("Flip vertically"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let horizontal = ctx.get_bool("horizontal").unwrap_or(true);
        let vertical = ctx.get_bool("vertical").unwrap_or(false);

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let mut result = img_data.to_rgba8();

        if horizontal {
            result = image::imageops::flip_horizontal(&result);
        }
        if vertical {
            result = image::imageops::flip_vertical(&result);
        }

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(result));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Crops an image.
#[derive(Debug, Clone)]
pub struct Crop;

impl FilterNode for Crop {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("crop", "Crop")
            .description("Crop a region from an image")
            .category(Category::Transform)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Cropped image")
            )
            .parameter(
                ParameterDefinition::new("x", PortType::Integer, Value::Integer(0))
                    .with_description("Left edge X coordinate")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::MinValue(0.0)),
            )
            .parameter(
                ParameterDefinition::new("y", PortType::Integer, Value::Integer(0))
                    .with_description("Top edge Y coordinate")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::MinValue(0.0)),
            )
            .parameter(
                ParameterDefinition::new("width", PortType::Integer, Value::Integer(100))
                    .with_description("Crop width")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::MinValue(1.0)),
            )
            .parameter(
                ParameterDefinition::new("height", PortType::Integer, Value::Integer(100))
                    .with_description("Crop height")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::MinValue(1.0)),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let width = ctx.get_integer("width").unwrap_or(100);
        let height = ctx.get_integer("height").unwrap_or(100);

        if width < 1 || height < 1 {
            return Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: "Crop dimensions must be at least 1x1".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let x = ctx.get_integer("x").unwrap_or(0) as u32;
        let y = ctx.get_integer("y").unwrap_or(0) as u32;
        let width = ctx.get_integer("width").unwrap_or(100) as u32;
        let height = ctx.get_integer("height").unwrap_or(100) as u32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img_data.to_rgba8();
        let (src_width, src_height) = rgba.dimensions();

        // Validate crop bounds
        if x >= src_width || y >= src_height {
            return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!(
                    "Crop origin ({}, {}) is outside image bounds ({}x{})",
                    x, y, src_width, src_height
                ),
            });
        }

        // Clamp crop dimensions to image bounds
        let crop_width = width.min(src_width - x);
        let crop_height = height.min(src_height - y);

        let cropped = image::imageops::crop_imm(&rgba, x, y, crop_width, crop_height).to_image();

        let result = ImageValue::new(image::DynamicImage::ImageRgba8(cropped));

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
    fn test_resize_metadata() {
        let filter = Resize;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "resize");
        assert_eq!(metadata.category, Category::Transform);
        assert_eq!(metadata.parameters.len(), 4);
    }

    #[test]
    fn test_rotate_metadata() {
        let filter = Rotate;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "rotate");
    }

    #[test]
    fn test_flip_metadata() {
        let filter = Flip;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "flip");
        assert_eq!(metadata.parameters.len(), 2);
    }

    #[test]
    fn test_crop_metadata() {
        let filter = Crop;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "crop");
        assert_eq!(metadata.parameters.len(), 4);
    }
}
