//! Utility filters: Preview, PassthroughNode, SplitChannels, MergeChannels, Note

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata, PassthroughNode};
use crate::core::port::{ParameterDefinition, PortDefinition};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::{DynamicImage, GrayImage, Rgba, RgbaImage};

/// Register utility filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(PassthroughNode));
    registry.register(|| Box::new(Preview));
    registry.register(|| Box::new(SplitChannels));
    registry.register(|| Box::new(MergeChannels));
    registry.register(|| Box::new(Note));
    registry.register(|| Box::new(ImageInfo));
}

/// Preview node - displays image info without modifying it.
///
/// This is useful for debugging and inspection in the pipeline.
#[derive(Debug, Clone)]
pub struct Preview;

impl FilterNode for Preview {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("preview", "Preview")
            .description("Preview an image and display its information")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Image to preview")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Passthrough of input image")
            )
            .output(
                PortDefinition::output("width", PortType::Integer)
                    .with_description("Image width in pixels")
            )
            .output(
                PortDefinition::output("height", PortType::Integer)
                    .with_description("Image height in pixels")
            )
            .output(
                PortDefinition::output("has_alpha", PortType::Boolean)
                    .with_description("Whether image has alpha channel")
            )
            .output(
                PortDefinition::output("info", PortType::String)
                    .with_description("Human-readable image information")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let width = image.metadata.width;
        let height = image.metadata.height;
        let has_alpha = image.metadata.has_alpha;
        let format = &image.metadata.format;

        let info = format!(
            "Image: {}x{} {:?}, Alpha: {}",
            width, height, format, has_alpha
        );

        ctx.set_output("image", Value::Image(image.clone()))?;
        ctx.set_output("width", Value::Integer(width as i64))?;
        ctx.set_output("height", Value::Integer(height as i64))?;
        ctx.set_output("has_alpha", Value::Boolean(has_alpha))?;
        ctx.set_output("info", Value::String(info))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Split an image into its RGBA channels.
#[derive(Debug, Clone)]
pub struct SplitChannels;

impl FilterNode for SplitChannels {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("split_channels", "Split Channels")
            .description("Split an image into its Red, Green, Blue, and Alpha channels")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Image to split")
            )
            .output(
                PortDefinition::output("red", PortType::Image)
                    .with_description("Red channel as grayscale image")
            )
            .output(
                PortDefinition::output("green", PortType::Image)
                    .with_description("Green channel as grayscale image")
            )
            .output(
                PortDefinition::output("blue", PortType::Image)
                    .with_description("Blue channel as grayscale image")
            )
            .output(
                PortDefinition::output("alpha", PortType::Image)
                    .with_description("Alpha channel as grayscale image")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let img = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        // Extract each channel
        let mut red = GrayImage::new(width, height);
        let mut green = GrayImage::new(width, height);
        let mut blue = GrayImage::new(width, height);
        let mut alpha = GrayImage::new(width, height);

        for (x, y, pixel) in rgba.enumerate_pixels() {
            red.put_pixel(x, y, image::Luma([pixel[0]]));
            green.put_pixel(x, y, image::Luma([pixel[1]]));
            blue.put_pixel(x, y, image::Luma([pixel[2]]));
            alpha.put_pixel(x, y, image::Luma([pixel[3]]));
        }

        ctx.set_output("red", Value::Image(ImageValue::new(DynamicImage::ImageLuma8(red))))?;
        ctx.set_output("green", Value::Image(ImageValue::new(DynamicImage::ImageLuma8(green))))?;
        ctx.set_output("blue", Value::Image(ImageValue::new(DynamicImage::ImageLuma8(blue))))?;
        ctx.set_output("alpha", Value::Image(ImageValue::new(DynamicImage::ImageLuma8(alpha))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Merge separate channels into an RGBA image.
#[derive(Debug, Clone)]
pub struct MergeChannels;

impl FilterNode for MergeChannels {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("merge_channels", "Merge Channels")
            .description("Merge separate Red, Green, Blue, and Alpha channels into one image")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("red", PortType::Image)
                    .with_description("Red channel")
            )
            .input(
                PortDefinition::input("green", PortType::Image)
                    .with_description("Green channel")
            )
            .input(
                PortDefinition::input("blue", PortType::Image)
                    .with_description("Blue channel")
            )
            .input(
                PortDefinition::input("alpha", PortType::Image)
                    .optional()
                    .with_description("Alpha channel (optional, defaults to opaque)")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Combined RGBA image")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let red_img = ctx.get_input_image("red")?;
        let green_img = ctx.get_input_image("green")?;
        let blue_img = ctx.get_input_image("blue")?;
        let alpha_img = ctx.get_input_image_optional("alpha");

        let red = red_img.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Red channel has no data".to_string(),
        })?.to_luma8();

        let green = green_img.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Green channel has no data".to_string(),
        })?.to_luma8();

        let blue = blue_img.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Blue channel has no data".to_string(),
        })?.to_luma8();

        let (width, height) = red.dimensions();
        
        // Check dimensions match
        if green.dimensions() != (width, height) || blue.dimensions() != (width, height) {
            return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Channel dimensions must match".to_string(),
            });
        }

        let alpha = alpha_img.and_then(|img| img.get_image().map(|i| i.to_luma8()));

        let mut result = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let r = red.get_pixel(x, y)[0];
                let g = green.get_pixel(x, y)[0];
                let b = blue.get_pixel(x, y)[0];
                let a = alpha.as_ref().map(|img| img.get_pixel(x, y)[0]).unwrap_or(255);
                result.put_pixel(x, y, Rgba([r, g, b, a]));
            }
        }

        ctx.set_output("image", Value::Image(ImageValue::new(DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// A note/comment node for documenting the graph.
#[derive(Debug, Clone)]
pub struct Note;

impl FilterNode for Note {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("note", "Note")
            .description("Add a note or comment to document your graph")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .parameter(
                ParameterDefinition::new("text", PortType::String, Value::String("Add your notes here...".to_string()))
                    .with_description("Note content")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, _ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        // Note nodes don't process anything
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Get detailed information about an image.
#[derive(Debug, Clone)]
pub struct ImageInfo;

impl FilterNode for ImageInfo {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("image_info", "Image Info")
            .description("Get detailed information about an image")
            .category(Category::Analyze)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Image to analyze")
            )
            .output(
                PortDefinition::output("width", PortType::Integer)
                    .with_description("Image width in pixels")
            )
            .output(
                PortDefinition::output("height", PortType::Integer)
                    .with_description("Image height in pixels")
            )
            .output(
                PortDefinition::output("channels", PortType::Integer)
                    .with_description("Number of color channels")
            )
            .output(
                PortDefinition::output("has_alpha", PortType::Boolean)
                    .with_description("Whether image has alpha channel")
            )
            .output(
                PortDefinition::output("pixel_count", PortType::Integer)
                    .with_description("Total number of pixels")
            )
            .output(
                PortDefinition::output("aspect_ratio", PortType::Float)
                    .with_description("Width/Height aspect ratio")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let width = image.metadata.width;
        let height = image.metadata.height;
        let has_alpha = image.metadata.has_alpha;
        let channels = if has_alpha { 4 } else { 3 };
        let pixel_count = (width * height) as i64;
        let aspect_ratio = width as f64 / height as f64;

        ctx.set_output("width", Value::Integer(width as i64))?;
        ctx.set_output("height", Value::Integer(height as i64))?;
        ctx.set_output("channels", Value::Integer(channels))?;
        ctx.set_output("has_alpha", Value::Boolean(has_alpha))?;
        ctx.set_output("pixel_count", Value::Integer(pixel_count))?;
        ctx.set_output("aspect_ratio", Value::Float(aspect_ratio))?;
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
    fn test_preview_metadata() {
        let filter = Preview;
        let metadata = filter.metadata();
        
        assert_eq!(metadata.id, "preview");
        assert_eq!(metadata.category, Category::Utility);
        assert_eq!(metadata.inputs.len(), 1);
        assert_eq!(metadata.outputs.len(), 5);
    }
}
