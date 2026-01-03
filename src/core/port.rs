//! Port definitions and constraints for node inputs/outputs.
//!
//! Ports define the interface of a node - what data it accepts and produces.
//! Each port has a type and optional constraints for validation.

use crate::core::types::{ImageFormat, PortType, Value};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Direction of a port (input or output).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PortDirection {
    Input,
    Output,
}

/// Definition of a node port (input or output).
///
/// Ports are the connection points of a node. Each port has a name, type,
/// and optional constraints that values must satisfy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDefinition {
    /// Unique name within the node (used in code)
    pub name: String,
    /// Human-readable name (used in UI)
    pub display_name: String,
    /// Type of data this port accepts/produces
    pub port_type: PortType,
    /// Direction (input or output)
    pub direction: PortDirection,
    /// Default value (for optional inputs)
    pub default_value: Option<Value>,
    /// Whether this port is optional
    pub optional: bool,
    /// Description for documentation and tooltips
    pub description: String,
    /// Constraints that values must satisfy
    pub constraints: Vec<Constraint>,
}

/// UI hints for parameter display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "widget", content = "options")]
pub enum UiHint {
    /// Default input widget based on type
    Default,
    /// Slider for numeric values
    Slider {
        /// Whether to use logarithmic scale
        logarithmic: bool,
    },
    /// Dropdown for selecting from options
    Dropdown {
        /// Available options
        options: Vec<String>,
    },
    /// Color picker widget
    ColorPicker,
    /// File chooser dialog
    FileChooser {
        /// File type filters (e.g., ["*.png", "*.jpg"])
        filters: Vec<String>,
    },
    /// Text input field
    TextInput {
        /// Allow multiple lines
        multiline: bool,
        /// Placeholder text
        placeholder: Option<String>,
    },
    /// Checkbox for booleans
    Checkbox,
    /// Spin box for integers
    SpinBox,
    /// Angle input (with circular widget)
    Angle,
    /// 2D position picker
    Position2D,
}

/// Definition of a node parameter (configuration).
///
/// Parameters differ from inputs: they are configured in the UI property panel
/// rather than connected to other nodes. They remain constant during batch
/// processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    /// Unique name within the node
    pub name: String,
    /// Human-readable name
    pub display_name: String,
    /// Type of the parameter
    pub param_type: PortType,
    /// Default value (required for parameters)
    pub default_value: Value,
    /// Description for documentation
    pub description: String,
    /// Constraints for validation
    pub constraints: Vec<Constraint>,
    /// UI widget hint
    pub ui_hint: UiHint,
    /// Group name for organizing parameters in UI
    pub group: Option<String>,
}

/// Constraints that can be applied to port/parameter values.
///
/// Constraints are checked during validation before execution begins.
/// This allows catching errors early before processing starts.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum Constraint {
    /// Numeric value must be within range [min, max]
    Range { min: f64, max: f64 },
    /// Numeric value must be >= min
    MinValue(f64),
    /// Numeric value must be <= max
    MaxValue(f64),
    /// Numeric value must be a multiple of step
    Step(f64),

    /// String/array length must be >= min
    MinLength(usize),
    /// String/array length must be <= max
    MaxLength(usize),
    /// String must match regex pattern
    Pattern(String),
    /// String must not be empty
    NotEmpty,

    /// Image dimensions must be >= (width, height)
    ImageMinDimensions { width: u32, height: u32 },
    /// Image dimensions must be <= (width, height)
    ImageMaxDimensions { width: u32, height: u32 },
    /// Image aspect ratio must be within tolerance of target
    ImageAspectRatio { ratio: f64, tolerance: f64 },
    /// Image must be one of the specified formats
    ImageFormat(Vec<ImageFormat>),
    /// Image must have alpha channel
    ImageRequiresAlpha,

    /// Value must be one of the specified options
    OneOf(Vec<Value>),
    /// Integer must be positive (> 0)
    Positive,
    /// Integer must be non-negative (>= 0)
    NonNegative,

    /// Custom constraint with validation function
    /// Note: The closure is skipped during serialization
    #[serde(skip)]
    Custom {
        name: String,
        description: String,
        validator: Arc<dyn Fn(&Value) -> Result<(), String> + Send + Sync>,
    },
}

impl std::fmt::Debug for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constraint::Range { min, max } => f.debug_struct("Range")
                .field("min", min)
                .field("max", max)
                .finish(),
            Constraint::MinValue(v) => f.debug_tuple("MinValue").field(v).finish(),
            Constraint::MaxValue(v) => f.debug_tuple("MaxValue").field(v).finish(),
            Constraint::Step(v) => f.debug_tuple("Step").field(v).finish(),
            Constraint::MinLength(v) => f.debug_tuple("MinLength").field(v).finish(),
            Constraint::MaxLength(v) => f.debug_tuple("MaxLength").field(v).finish(),
            Constraint::Pattern(v) => f.debug_tuple("Pattern").field(v).finish(),
            Constraint::NotEmpty => write!(f, "NotEmpty"),
            Constraint::ImageMinDimensions { width, height } => f.debug_struct("ImageMinDimensions")
                .field("width", width)
                .field("height", height)
                .finish(),
            Constraint::ImageMaxDimensions { width, height } => f.debug_struct("ImageMaxDimensions")
                .field("width", width)
                .field("height", height)
                .finish(),
            Constraint::ImageAspectRatio { ratio, tolerance } => f.debug_struct("ImageAspectRatio")
                .field("ratio", ratio)
                .field("tolerance", tolerance)
                .finish(),
            Constraint::ImageFormat(v) => f.debug_tuple("ImageFormat").field(v).finish(),
            Constraint::ImageRequiresAlpha => write!(f, "ImageRequiresAlpha"),
            Constraint::OneOf(v) => f.debug_tuple("OneOf").field(v).finish(),
            Constraint::Positive => write!(f, "Positive"),
            Constraint::NonNegative => write!(f, "NonNegative"),
            Constraint::Custom { name, description, .. } => f.debug_struct("Custom")
                .field("name", name)
                .field("description", description)
                .field("validator", &"<closure>")
                .finish(),
        }
    }
}

// ============================================================================
// PortDefinition Builder Pattern
// ============================================================================

impl PortDefinition {
    /// Create a new input port definition.
    pub fn input(name: impl Into<String>, port_type: PortType) -> Self {
        let name = name.into();
        Self {
            display_name: Self::name_to_display(&name),
            name,
            port_type,
            direction: PortDirection::Input,
            default_value: None,
            optional: false,
            description: String::new(),
            constraints: Vec::new(),
        }
    }

    /// Create a new output port definition.
    pub fn output(name: impl Into<String>, port_type: PortType) -> Self {
        let name = name.into();
        Self {
            display_name: Self::name_to_display(&name),
            name,
            port_type,
            direction: PortDirection::Output,
            default_value: None,
            optional: false,
            description: String::new(),
            constraints: Vec::new(),
        }
    }

    /// Set the display name.
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    /// Set the default value.
    pub fn with_default(mut self, value: Value) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Mark this port as optional.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Add a range constraint.
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.constraints.push(Constraint::Range { min, max });
        self
    }

    /// Add a minimum value constraint.
    pub fn with_min(mut self, min: f64) -> Self {
        self.constraints.push(Constraint::MinValue(min));
        self
    }

    /// Add a maximum value constraint.
    pub fn with_max(mut self, max: f64) -> Self {
        self.constraints.push(Constraint::MaxValue(max));
        self
    }

    /// Add a constraint.
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Convert snake_case name to Title Case display name.
    fn name_to_display(name: &str) -> String {
        name.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().chain(chars).collect()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Validate a value against this port's type and constraints.
    pub fn validate(&self, value: &Value) -> Result<(), String> {
        // Type check
        if !self.port_type.matches(value) {
            return Err(format!(
                "Type mismatch for port '{}': expected {}, got {}",
                self.name,
                self.port_type,
                value.get_type()
            ));
        }

        // Constraint checks
        for constraint in &self.constraints {
            constraint.validate(value)?;
        }

        Ok(())
    }
}

// ============================================================================
// ParameterDefinition Builder Pattern
// ============================================================================

impl ParameterDefinition {
    /// Create a new parameter definition.
    pub fn new(name: impl Into<String>, param_type: PortType, default_value: Value) -> Self {
        let name = name.into();
        Self {
            display_name: PortDefinition::name_to_display(&name),
            name,
            param_type,
            default_value,
            description: String::new(),
            constraints: Vec::new(),
            ui_hint: UiHint::Default,
            group: None,
        }
    }

    /// Set the display name.
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a range constraint and set UI hint to slider.
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.constraints.push(Constraint::Range { min, max });
        if matches!(self.ui_hint, UiHint::Default) {
            self.ui_hint = UiHint::Slider { logarithmic: false };
        }
        self
    }

    /// Set UI hint to logarithmic slider.
    pub fn logarithmic(mut self) -> Self {
        self.ui_hint = UiHint::Slider { logarithmic: true };
        self
    }

    /// Add a constraint.
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Set the UI hint.
    pub fn with_ui_hint(mut self, ui_hint: UiHint) -> Self {
        self.ui_hint = ui_hint;
        self
    }

    /// Set the parameter group.
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Validate a value against this parameter's type and constraints.
    pub fn validate(&self, value: &Value) -> Result<(), String> {
        // Type check
        if !self.param_type.matches(value) {
            return Err(format!(
                "Type mismatch for parameter '{}': expected {}, got {}",
                self.name,
                self.param_type,
                value.get_type()
            ));
        }

        // Constraint checks
        for constraint in &self.constraints {
            constraint.validate(value)?;
        }

        Ok(())
    }
}

// ============================================================================
// Constraint Validation
// ============================================================================

impl Constraint {
    /// Validate a value against this constraint.
    pub fn validate(&self, value: &Value) -> Result<(), String> {
        match self {
            Constraint::Range { min, max } => {
                if let Some(num) = value.as_float() {
                    if num < *min || num > *max {
                        return Err(format!(
                            "Value {} is out of range [{}, {}]",
                            num, min, max
                        ));
                    }
                }
            }

            Constraint::MinValue(min) => {
                if let Some(num) = value.as_float() {
                    if num < *min {
                        return Err(format!("Value {} is below minimum {}", num, min));
                    }
                }
            }

            Constraint::MaxValue(max) => {
                if let Some(num) = value.as_float() {
                    if num > *max {
                        return Err(format!("Value {} is above maximum {}", num, max));
                    }
                }
            }

            Constraint::Step(step) => {
                if let Some(num) = value.as_float() {
                    let remainder = num % step;
                    if remainder.abs() > f64::EPSILON {
                        return Err(format!(
                            "Value {} must be a multiple of {}",
                            num, step
                        ));
                    }
                }
            }

            Constraint::MinLength(min_len) => {
                let len = match value {
                    Value::String(s) => s.len(),
                    Value::Array(arr) => arr.len(),
                    _ => 0,
                };
                if len < *min_len {
                    return Err(format!(
                        "Length {} is below minimum {}",
                        len, min_len
                    ));
                }
            }

            Constraint::MaxLength(max_len) => {
                let len = match value {
                    Value::String(s) => s.len(),
                    Value::Array(arr) => arr.len(),
                    _ => 0,
                };
                if len > *max_len {
                    return Err(format!(
                        "Length {} is above maximum {}",
                        len, max_len
                    ));
                }
            }

            Constraint::NotEmpty => {
                let is_empty = match value {
                    Value::String(s) => s.is_empty(),
                    Value::Array(arr) => arr.is_empty(),
                    Value::Map(map) => map.is_empty(),
                    _ => false,
                };
                if is_empty {
                    return Err("Value cannot be empty".to_string());
                }
            }

            Constraint::Pattern(pattern) => {
                if let Value::String(s) = value {
                    // Simple contains check; for full regex, add regex crate
                    if !s.contains(pattern) {
                        return Err(format!(
                            "String '{}' does not match pattern '{}'",
                            s, pattern
                        ));
                    }
                }
            }

            Constraint::ImageMinDimensions { width, height } => {
                if let Some(img) = value.as_image() {
                    if img.metadata.width < *width || img.metadata.height < *height {
                        return Err(format!(
                            "Image dimensions {}x{} are below minimum {}x{}",
                            img.metadata.width, img.metadata.height, width, height
                        ));
                    }
                }
            }

            Constraint::ImageMaxDimensions { width, height } => {
                if let Some(img) = value.as_image() {
                    if img.metadata.width > *width || img.metadata.height > *height {
                        return Err(format!(
                            "Image dimensions {}x{} are above maximum {}x{}",
                            img.metadata.width, img.metadata.height, width, height
                        ));
                    }
                }
            }

            Constraint::ImageAspectRatio { ratio, tolerance } => {
                if let Some(img) = value.as_image() {
                    let actual_ratio = img.metadata.width as f64 / img.metadata.height as f64;
                    let diff = (actual_ratio - ratio).abs();
                    if diff > *tolerance {
                        return Err(format!(
                            "Image aspect ratio {:.2} is not within tolerance {} of target {:.2}",
                            actual_ratio, tolerance, ratio
                        ));
                    }
                }
            }

            Constraint::ImageFormat(formats) => {
                if let Some(img) = value.as_image() {
                    if !formats.contains(&img.metadata.format) {
                        return Err(format!(
                            "Image format {} is not one of {:?}",
                            img.metadata.format, formats
                        ));
                    }
                }
            }

            Constraint::ImageRequiresAlpha => {
                if let Some(img) = value.as_image() {
                    if !img.metadata.has_alpha {
                        return Err("Image must have an alpha channel".to_string());
                    }
                }
            }

            Constraint::OneOf(options) => {
                let matches = options.iter().any(|opt| {
                    // Compare discriminants for type matching
                    std::mem::discriminant(value) == std::mem::discriminant(opt)
                });
                if !matches {
                    return Err("Value is not one of the allowed options".to_string());
                }
            }

            Constraint::Positive => {
                if let Some(num) = value.as_float() {
                    if num <= 0.0 {
                        return Err(format!("Value {} must be positive", num));
                    }
                }
            }

            Constraint::NonNegative => {
                if let Some(num) = value.as_float() {
                    if num < 0.0 {
                        return Err(format!("Value {} must be non-negative", num));
                    }
                }
            }

            Constraint::Custom { name, validator, .. } => {
                validator(value).map_err(|e| format!("{}: {}", name, e))?;
            }
        }

        Ok(())
    }

    /// Get a human-readable description of this constraint.
    pub fn description(&self) -> String {
        match self {
            Constraint::Range { min, max } => format!("Must be between {} and {}", min, max),
            Constraint::MinValue(min) => format!("Must be at least {}", min),
            Constraint::MaxValue(max) => format!("Must be at most {}", max),
            Constraint::Step(step) => format!("Must be a multiple of {}", step),
            Constraint::MinLength(len) => format!("Minimum length: {}", len),
            Constraint::MaxLength(len) => format!("Maximum length: {}", len),
            Constraint::NotEmpty => "Cannot be empty".to_string(),
            Constraint::Pattern(p) => format!("Must match pattern: {}", p),
            Constraint::ImageMinDimensions { width, height } => {
                format!("Minimum dimensions: {}x{}", width, height)
            }
            Constraint::ImageMaxDimensions { width, height } => {
                format!("Maximum dimensions: {}x{}", width, height)
            }
            Constraint::ImageAspectRatio { ratio, tolerance } => {
                format!("Aspect ratio: {} ± {}", ratio, tolerance)
            }
            Constraint::ImageFormat(formats) => format!("Allowed formats: {:?}", formats),
            Constraint::ImageRequiresAlpha => "Must have alpha channel".to_string(),
            Constraint::OneOf(options) => format!("One of {} options", options.len()),
            Constraint::Positive => "Must be positive".to_string(),
            Constraint::NonNegative => "Must be non-negative".to_string(),
            Constraint::Custom { description, .. } => description.clone(),
        }
    }
}

impl Default for UiHint {
    fn default() -> Self {
        UiHint::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_definition_builder() {
        let port = PortDefinition::input("my_input", PortType::Float)
            .with_display_name("My Input")
            .with_description("A test input")
            .with_range(0.0, 100.0)
            .with_default(Value::Float(50.0));

        assert_eq!(port.name, "my_input");
        assert_eq!(port.display_name, "My Input");
        assert_eq!(port.port_type, PortType::Float);
        assert!(matches!(port.direction, PortDirection::Input));
        assert_eq!(port.constraints.len(), 1);
    }

    #[test]
    fn test_constraint_range_validation() {
        let constraint = Constraint::Range { min: 0.0, max: 100.0 };

        assert!(constraint.validate(&Value::Float(50.0)).is_ok());
        assert!(constraint.validate(&Value::Float(0.0)).is_ok());
        assert!(constraint.validate(&Value::Float(100.0)).is_ok());
        assert!(constraint.validate(&Value::Float(-1.0)).is_err());
        assert!(constraint.validate(&Value::Float(101.0)).is_err());
    }

    #[test]
    fn test_constraint_not_empty() {
        let constraint = Constraint::NotEmpty;

        assert!(constraint.validate(&Value::String("hello".to_string())).is_ok());
        assert!(constraint.validate(&Value::String("".to_string())).is_err());
        assert!(constraint.validate(&Value::Array(vec![Value::Integer(1)])).is_ok());
        assert!(constraint.validate(&Value::Array(vec![])).is_err());
    }

    #[test]
    fn test_name_to_display() {
        assert_eq!(PortDefinition::name_to_display("my_input"), "My Input");
        assert_eq!(PortDefinition::name_to_display("blur_radius"), "Blur Radius");
        assert_eq!(PortDefinition::name_to_display("image"), "Image");
    }
}
