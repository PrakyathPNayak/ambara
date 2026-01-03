//! Utility filters: Preview, PassthroughNode (re-exported)

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata, PassthroughNode};
use crate::core::port::PortDefinition;
use crate::core::types::{PortType, Value};
use crate::filters::registry::FilterRegistry;

/// Register utility filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(PassthroughNode));
    registry.register(|| Box::new(Preview));
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
