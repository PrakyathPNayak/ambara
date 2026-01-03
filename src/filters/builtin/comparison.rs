//! Comparison and logic operation nodes.

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::PortDefinition;
use crate::core::types::{PortType, Value};
use crate::filters::registry::FilterRegistry;

// Helper function to extract float from input
fn get_input_as_float(ctx: &ExecutionContext, port: &str) -> Result<f64, ExecutionError> {
    match ctx.get_input(port)? {
        Value::Float(v) => Ok(*v),
        Value::Integer(v) => Ok(*v as f64),
        _ => Err(ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: format!("Port '{}' expected number", port),
        }),
    }
}

/// Register comparison and logic nodes.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(Equal));
    registry.register(|| Box::new(NotEqual));
    registry.register(|| Box::new(LessThan));
    registry.register(|| Box::new(LessThanOrEqual));
    registry.register(|| Box::new(GreaterThan));
    registry.register(|| Box::new(GreaterThanOrEqual));
    registry.register(|| Box::new(And));
    registry.register(|| Box::new(Or));
    registry.register(|| Box::new(Not));
    registry.register(|| Box::new(Xor));
}

/// Check if two numbers are equal.
#[derive(Debug, Clone)]
pub struct Equal;

impl FilterNode for Equal {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("equal", "Equal")
            .description("Check if two numbers are equal (a == b)")
            .category(Category::Math)
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First value")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second value")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("True if a == b")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Boolean(a == b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Check if two numbers are not equal.
#[derive(Debug, Clone)]
pub struct NotEqual;

impl FilterNode for NotEqual {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("not_equal", "Not Equal")
            .description("Check if two numbers are not equal (a != b)")
            .category(Category::Math)
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First value")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second value")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("True if a != b")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Boolean(a != b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Check if a is less than b.
#[derive(Debug, Clone)]
pub struct LessThan;

impl FilterNode for LessThan {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("less_than", "Less Than")
            .description("Check if a is less than b (a < b)")
            .category(Category::Math)
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First value")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second value")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("True if a < b")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Boolean(a < b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Check if a is less than or equal to b.
#[derive(Debug, Clone)]
pub struct LessThanOrEqual;

impl FilterNode for LessThanOrEqual {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("less_than_or_equal", "Less Than or Equal")
            .description("Check if a is less than or equal to b (a <= b)")
            .category(Category::Math)
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First value")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second value")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("True if a <= b")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Boolean(a <= b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Check if a is greater than b.
#[derive(Debug, Clone)]
pub struct GreaterThan;

impl FilterNode for GreaterThan {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("greater_than", "Greater Than")
            .description("Check if a is greater than b (a > b)")
            .category(Category::Math)
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First value")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second value")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("True if a > b")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Boolean(a > b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Check if a is greater than or equal to b.
#[derive(Debug, Clone)]
pub struct GreaterThanOrEqual;

impl FilterNode for GreaterThanOrEqual {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("greater_than_or_equal", "Greater Than or Equal")
            .description("Check if a is greater than or equal to b (a >= b)")
            .category(Category::Math)
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First value")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second value")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("True if a >= b")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Boolean(a >= b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Logical AND operation.
#[derive(Debug, Clone)]
pub struct And;

impl FilterNode for And {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("and", "And")
            .description("Logical AND operation (a && b)")
            .category(Category::Math)
            .input(
                PortDefinition::input("a", PortType::Boolean)
                    .with_description("First boolean")
            )
            .input(
                PortDefinition::input("b", PortType::Boolean)
                    .with_description("Second boolean")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("True if both a and b are true")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = match ctx.get_input("a")? {
            Value::Boolean(v) => *v,
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Input 'a' must be boolean".to_string(),
            }),
        };
        let b = match ctx.get_input("b")? {
            Value::Boolean(v) => *v,
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Input 'b' must be boolean".to_string(),
            }),
        };
        ctx.set_output("result", Value::Boolean(a && b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Logical OR operation.
#[derive(Debug, Clone)]
pub struct Or;

impl FilterNode for Or {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("or", "Or")
            .description("Logical OR operation (a || b)")
            .category(Category::Math)
            .input(
                PortDefinition::input("a", PortType::Boolean)
                    .with_description("First boolean")
            )
            .input(
                PortDefinition::input("b", PortType::Boolean)
                    .with_description("Second boolean")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("True if either a or b is true")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = match ctx.get_input("a")? {
            Value::Boolean(v) => *v,
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Input 'a' must be boolean".to_string(),
            }),
        };
        let b = match ctx.get_input("b")? {
            Value::Boolean(v) => *v,
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Input 'b' must be boolean".to_string(),
            }),
        };
        ctx.set_output("result", Value::Boolean(a || b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Logical NOT operation.
#[derive(Debug, Clone)]
pub struct Not;

impl FilterNode for Not {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("not", "Not")
            .description("Logical NOT operation (!a)")
            .category(Category::Math)
            .input(
                PortDefinition::input("value", PortType::Boolean)
                    .with_description("Input boolean")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("True if value is false")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let value = match ctx.get_input("value")? {
            Value::Boolean(v) => *v,
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Input 'value' must be boolean".to_string(),
            }),
        };
        ctx.set_output("result", Value::Boolean(!value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Logical XOR operation.
#[derive(Debug, Clone)]
pub struct Xor;

impl FilterNode for Xor {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("xor", "Xor")
            .description("Logical XOR operation (a ^ b)")
            .category(Category::Math)
            .input(
                PortDefinition::input("a", PortType::Boolean)
                    .with_description("First boolean")
            )
            .input(
                PortDefinition::input("b", PortType::Boolean)
                    .with_description("Second boolean")
            )
            .output(
                PortDefinition::output("result", PortType::Boolean)
                    .with_description("True if exactly one of a or b is true")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = match ctx.get_input("a")? {
            Value::Boolean(v) => *v,
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Input 'a' must be boolean".to_string(),
            }),
        };
        let b = match ctx.get_input("b")? {
            Value::Boolean(v) => *v,
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Input 'b' must be boolean".to_string(),
            }),
        };
        ctx.set_output("result", Value::Boolean(a ^ b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}
