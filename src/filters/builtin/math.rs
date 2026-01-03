//! Mathematical operation nodes.

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

/// Register math operation nodes.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(Add));
    registry.register(|| Box::new(Subtract));
    registry.register(|| Box::new(Multiply));
    registry.register(|| Box::new(Divide));
    registry.register(|| Box::new(Modulo));
    registry.register(|| Box::new(Power));
    registry.register(|| Box::new(Min));
    registry.register(|| Box::new(Max));
    registry.register(|| Box::new(Clamp));
}

/// Add two numbers.
#[derive(Debug, Clone)]
pub struct Add;

impl FilterNode for Add {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("add", "Add")
            .description("Add two numbers (a + b)")
            .category(Category::Math)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First number")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second number")
            )
            .output(
                PortDefinition::output("result", PortType::Float)
                    .with_description("Sum (a + b)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Float(a + b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Subtract two numbers.
#[derive(Debug, Clone)]
pub struct Subtract;

impl FilterNode for Subtract {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("subtract", "Subtract")
            .description("Subtract two numbers (a - b)")
            .category(Category::Math)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First number")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second number")
            )
            .output(
                PortDefinition::output("result", PortType::Float)
                    .with_description("Difference (a - b)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Float(a - b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Multiply two numbers.
#[derive(Debug, Clone)]
pub struct Multiply;

impl FilterNode for Multiply {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("multiply", "Multiply")
            .description("Multiply two numbers (a × b)")
            .category(Category::Math)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First number")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second number")
            )
            .output(
                PortDefinition::output("result", PortType::Float)
                    .with_description("Product (a × b)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Float(a * b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Divide two numbers.
#[derive(Debug, Clone)]
pub struct Divide;

impl FilterNode for Divide {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("divide", "Divide")
            .description("Divide two numbers (a ÷ b)")
            .category(Category::Math)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("Dividend")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Divisor")
            )
            .output(
                PortDefinition::output("result", PortType::Float)
                    .with_description("Quotient (a ÷ b)")
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        if let Ok(b) = ctx.get_input("b") {
            if let Value::Float(val) = b {
                if *val == 0.0 {
                    return Err(ValidationError::CustomValidation {
                        node_id: ctx.node_id,
                        error: "Division by zero".to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        
        if b == 0.0 {
            return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Division by zero".to_string(),
            });
        }
        
        ctx.set_output("result", Value::Float(a / b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Modulo operation.
#[derive(Debug, Clone)]
pub struct Modulo;

impl FilterNode for Modulo {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("modulo", "Modulo")
            .description("Modulo operation (a % b)")
            .category(Category::Math)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("Dividend")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Divisor")
            )
            .output(
                PortDefinition::output("result", PortType::Float)
                    .with_description("Remainder (a % b)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Float(a % b))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Power operation.
#[derive(Debug, Clone)]
pub struct Power;

impl FilterNode for Power {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("power", "Power")
            .description("Raise a to the power of b (a^b)")
            .category(Category::Math)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("Base")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Exponent")
            )
            .output(
                PortDefinition::output("result", PortType::Float)
                    .with_description("Result (a^b)")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Float(a.powf(b)))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Minimum of two numbers.
#[derive(Debug, Clone)]
pub struct Min;

impl FilterNode for Min {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("min", "Min")
            .description("Return the minimum of two numbers")
            .category(Category::Math)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First number")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second number")
            )
            .output(
                PortDefinition::output("result", PortType::Float)
                    .with_description("Minimum value")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Float(a.min(b)))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Maximum of two numbers.
#[derive(Debug, Clone)]
pub struct Max;

impl FilterNode for Max {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("max", "Max")
            .description("Return the maximum of two numbers")
            .category(Category::Math)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("a", PortType::Float)
                    .with_description("First number")
            )
            .input(
                PortDefinition::input("b", PortType::Float)
                    .with_description("Second number")
            )
            .output(
                PortDefinition::output("result", PortType::Float)
                    .with_description("Maximum value")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let a = get_input_as_float(ctx, "a")?;
        let b = get_input_as_float(ctx, "b")?;
        ctx.set_output("result", Value::Float(a.max(b)))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Clamp a value between min and max.
#[derive(Debug, Clone)]
pub struct Clamp;

impl FilterNode for Clamp {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("clamp", "Clamp")
            .description("Clamp a value between min and max")
            .category(Category::Math)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("value", PortType::Float)
                    .with_description("Value to clamp")
            )
            .input(
                PortDefinition::input("min", PortType::Float)
                    .with_description("Minimum value")
            )
            .input(
                PortDefinition::input("max", PortType::Float)
                    .with_description("Maximum value")
            )
            .output(
                PortDefinition::output("result", PortType::Float)
                    .with_description("Clamped value")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let value = get_input_as_float(ctx, "value")?;
        let min = get_input_as_float(ctx, "min")?;
        let max = get_input_as_float(ctx, "max")?;
        ctx.set_output("result", Value::Float(value.clamp(min, max)))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}
