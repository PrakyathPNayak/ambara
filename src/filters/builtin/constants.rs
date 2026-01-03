//! Constant value nodes for providing literal values to the graph.

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{Color, PortType, Value};
use crate::filters::registry::FilterRegistry;

/// Register constant nodes.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(IntegerConstant));
    registry.register(|| Box::new(FloatConstant));
    registry.register(|| Box::new(StringConstant));
    registry.register(|| Box::new(BooleanConstant));
    registry.register(|| Box::new(ColorConstant));
}

/// Integer constant node.
#[derive(Debug, Clone)]
pub struct IntegerConstant;

impl FilterNode for IntegerConstant {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("integer_constant", "Integer")
            .description("Provides a constant integer value")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .output(
                PortDefinition::output("value", PortType::Integer)
                    .with_description("The integer value")
            )
            .parameter(
                ParameterDefinition::new("value", PortType::Integer, Value::Integer(0))
                    .with_description("Integer value"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let value = ctx.get_integer("value").unwrap_or(0);
        ctx.set_output("value", Value::Integer(value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Float constant node.
#[derive(Debug, Clone)]
pub struct FloatConstant;

impl FilterNode for FloatConstant {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("float_constant", "Float")
            .description("Provides a constant floating point value")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .output(
                PortDefinition::output("value", PortType::Float)
                    .with_description("The float value")
            )
            .parameter(
                ParameterDefinition::new("value", PortType::Float, Value::Float(0.0))
                    .with_description("Float value"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let value = ctx.get_float("value").unwrap_or(0.0);
        ctx.set_output("value", Value::Float(value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// String constant node.
#[derive(Debug, Clone)]
pub struct StringConstant;

impl FilterNode for StringConstant {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("string_constant", "String")
            .description("Provides a constant string value")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .output(
                PortDefinition::output("value", PortType::String)
                    .with_description("The string value")
            )
            .parameter(
                ParameterDefinition::new("value", PortType::String, Value::String(String::new()))
                    .with_description("String value")
                    .with_ui_hint(UiHint::TextInput { multiline: true, placeholder: None }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let value = ctx.get_string("value").unwrap_or("").to_string();
        ctx.set_output("value", Value::String(value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Boolean constant node.
#[derive(Debug, Clone)]
pub struct BooleanConstant;

impl FilterNode for BooleanConstant {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("boolean_constant", "Boolean")
            .description("Provides a constant boolean value")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .output(
                PortDefinition::output("value", PortType::Boolean)
                    .with_description("The boolean value")
            )
            .parameter(
                ParameterDefinition::new("value", PortType::Boolean, Value::Boolean(false))
                    .with_description("Boolean value"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let value = ctx.get_bool("value").unwrap_or(false);
        ctx.set_output("value", Value::Boolean(value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Color constant node.
#[derive(Debug, Clone)]
pub struct ColorConstant;

impl FilterNode for ColorConstant {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("color_constant", "Color")
            .description("Provides a constant color value")
            .category(Category::Utility)
            .author("Ambara")
            .version("1.0.0")
            .output(
                PortDefinition::output("value", PortType::Color)
                    .with_description("The color value")
            )
            .parameter(
                ParameterDefinition::new(
                    "value",
                    PortType::Color,
                    Value::Color(Color { r: 255, g: 255, b: 255, a: 255 })
                )
                .with_description("Color value (RGBA, 0.0-1.0)"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let value = ctx.get_color("value").unwrap_or(Color { r: 255, g: 255, b: 255, a: 255 });
        ctx.set_output("value", Value::Color(value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}
