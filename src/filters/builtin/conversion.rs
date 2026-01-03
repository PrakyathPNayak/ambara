//! Type conversion nodes.

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::PortDefinition;
use crate::core::types::{PortType, Value};
use crate::filters::registry::FilterRegistry;

/// Register type conversion nodes.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(ToInteger));
    registry.register(|| Box::new(ToFloat));
    registry.register(|| Box::new(ToString));
    registry.register(|| Box::new(ToBoolean));
}

/// Convert value to integer.
#[derive(Debug, Clone)]
pub struct ToInteger;

impl FilterNode for ToInteger {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("to_integer", "To Integer")
            .description("Convert a value to an integer")
            .category(Category::Utility)
            .input(
                PortDefinition::input("value", PortType::Any)
                    .with_description("Value to convert")
            )
            .output(
                PortDefinition::output("result", PortType::Integer)
                    .with_description("Integer value")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let result = match ctx.get_input("value")? {
            Value::Integer(v) => *v,
            Value::Float(v) => *v as i64,
            Value::Boolean(v) => if *v { 1 } else { 0 },
            Value::String(s) => s.parse().unwrap_or(0),
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Cannot convert value to integer".to_string(),
            }),
        };
        ctx.set_output("result", Value::Integer(result))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Convert value to float.
#[derive(Debug, Clone)]
pub struct ToFloat;

impl FilterNode for ToFloat {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("to_float", "To Float")
            .description("Convert a value to a floating-point number")
            .category(Category::Utility)
            .input(
                PortDefinition::input("value", PortType::Any)
                    .with_description("Value to convert")
            )
            .output(
                PortDefinition::output("result", PortType::Float)
                    .with_description("Float value")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let result = match ctx.get_input("value")? {
            Value::Integer(v) => *v as f64,
            Value::Float(v) => *v,
            Value::Boolean(v) => if *v { 1.0 } else { 0.0 },
            Value::String(s) => s.parse().unwrap_or(0.0),
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Cannot convert value to float".to_string(),
            }),
        };
        ctx.set_output("result", Value::Float(result))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Convert value to string.
#[derive(Debug, Clone)]
pub struct ToString;

impl FilterNode for ToString {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("to_string", "To String")
            .description("Convert a value to a string")
            .category(Category::Utility)
            .input(
                PortDefinition::input("value", PortType::Any)
                    .with_description("Value to convert")
            )
            .output(
                PortDefinition::output("result", PortType::String)
                    .with_description("String value")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let result = match ctx.get_input("value")? {
            Value::Integer(v) => v.to_string(),
            Value::Float(v) => v.to_string(),
            Value::Boolean(v) => v.to_string(),
            Value::String(s) => s.clone(),
            Value::Color(c) => format!("rgba({}, {}, {}, {})", c.r, c.g, c.b, c.a),
            Value::Vector2(x, y) => format!("[{}, {}]", x, y),
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Cannot convert value to string".to_string(),
            }),
        };
        ctx.set_output("result", Value::String(result))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Convert value to boolean.
#[derive(Debug, Clone)]
pub struct ToBoolean;

impl FilterNode for ToBoolean {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("to_boolean", "To Boolean")
            .description("Convert a value to a boolean")
            .category(Category::Utility)
            .input(
                PortDefinition::input("value", PortType::Any)
                    .with_description("Value to convert")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("Boolean value")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let result = match ctx.get_input("value")? {
            Value::Integer(v) => *v != 0,
            Value::Float(v) => *v != 0.0,
            Value::Boolean(v) => *v,
            Value::String(s) => !s.is_empty() && s != "false" && s != "0",
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Cannot convert value to boolean".to_string(),
            }),
        };
        ctx.set_output("result", Value::Boolean(result))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}
