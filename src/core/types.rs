//! Core value types that flow through the processing graph.
//!
//! The type system uses an enum-based approach for several reasons:
//! - Closed set of types: Image processing has a finite set of data types
//! - Zero-cost pattern matching: Compiler optimizes to jump tables
//! - Serialization: serde handles enums natively
//! - Type safety: Exhaustive matching catches missing cases at compile time

use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

/// Core value types that can flow through the graph.
///
/// This enum represents all possible data types that can be passed between nodes.
/// Using an enum provides compile-time type safety and efficient pattern matching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum Value {
    /// Image data with metadata
    Image(ImageValue),
    /// 64-bit signed integer
    Integer(i64),
    /// 64-bit floating point number
    Float(f64),
    /// UTF-8 string
    String(String),
    /// Boolean value
    Boolean(bool),
    /// RGBA color value
    Color(Color),
    /// 2D vector (x, y)
    Vector2(f64, f64),
    /// 3D vector (x, y, z)
    Vector3(f64, f64, f64),
    /// Homogeneous array of values
    Array(Vec<Value>),
    /// Key-value map
    Map(HashMap<String, Value>),
    /// Represents absence of value
    None,
}

/// Image wrapper with metadata and smart memory management.
///
/// Images are stored using Arc for efficient sharing in the DAG structure.
/// Copy-on-write semantics are used when modifications are needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageValue {
    /// Image metadata (dimensions, format, etc.)
    pub metadata: ImageMetadata,
    /// Reference to the actual image data
    #[serde(skip)]
    data: Option<Arc<DynamicImage>>,
    /// Reference to original file or temporary storage
    pub data_ref: ImageDataRef,
}

impl PartialEq for ImageValue {
    fn eq(&self, other: &Self) -> bool {
        // Compare by metadata and data_ref (not the actual pixel data)
        self.metadata == other.metadata && self.data_ref == other.data_ref
    }
}

/// Image metadata without the pixel data.
///
/// This allows validation to check image properties without loading the full image.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageMetadata {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Image format
    pub format: ImageFormat,
    /// Whether the image has an alpha channel
    pub has_alpha: bool,
}

/// Reference to image data for serialization and lazy loading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "ref_type", content = "value")]
pub enum ImageDataRef {
    /// Path to an image file
    FilePath(PathBuf),
    /// Base64 encoded image data
    Base64(String),
    /// Reference to temporary storage
    Temporary(String),
    /// Image was created in memory (no external reference)
    InMemory,
}

/// Supported image formats.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    WebP,
    Tiff,
    Bmp,
    Unknown,
}

/// RGBA color value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Port types for type checking connections between nodes.
///
/// This enum defines what types of data a port can accept or produce.
/// The type system supports basic types, containers, and an Any wildcard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", content = "inner")]
pub enum PortType {
    Image,
    Integer,
    Float,
    String,
    Boolean,
    Color,
    Vector2,
    Vector3,
    /// Array of a specific type
    Array(Box<PortType>),
    /// Map with string keys and values of a specific type
    Map(Box<PortType>),
    /// Accepts any type (for generic filters)
    Any,
}

// ============================================================================
// Value Implementation
// ============================================================================

impl Value {
    /// Get the port type of this value.
    pub fn get_type(&self) -> PortType {
        match self {
            Value::Image(_) => PortType::Image,
            Value::Integer(_) => PortType::Integer,
            Value::Float(_) => PortType::Float,
            Value::String(_) => PortType::String,
            Value::Boolean(_) => PortType::Boolean,
            Value::Color(_) => PortType::Color,
            Value::Vector2(_, _) => PortType::Vector2,
            Value::Vector3(_, _, _) => PortType::Vector3,
            Value::Array(arr) => {
                if let Some(first) = arr.first() {
                    PortType::Array(Box::new(first.get_type()))
                } else {
                    PortType::Array(Box::new(PortType::Any))
                }
            }
            Value::Map(map) => {
                if let Some(first) = map.values().next() {
                    PortType::Map(Box::new(first.get_type()))
                } else {
                    PortType::Map(Box::new(PortType::Any))
                }
            }
            Value::None => PortType::Any,
        }
    }

    /// Try to get this value as an image reference.
    pub fn as_image(&self) -> Option<&ImageValue> {
        if let Value::Image(img) = self {
            Some(img)
        } else {
            None
        }
    }

    /// Try to get this value as a mutable image reference.
    pub fn as_image_mut(&mut self) -> Option<&mut ImageValue> {
        if let Value::Image(img) = self {
            Some(img)
        } else {
            None
        }
    }

    /// Try to get this value as an integer.
    pub fn as_integer(&self) -> Option<i64> {
        if let Value::Integer(i) = self {
            Some(*i)
        } else {
            None
        }
    }

    /// Try to get this value as a float.
    /// Integers are automatically converted to floats.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to get this value as a string reference.
    pub fn as_string(&self) -> Option<&str> {
        if let Value::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    /// Try to get this value as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Boolean(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// Try to get this value as a color.
    pub fn as_color(&self) -> Option<Color> {
        if let Value::Color(c) = self {
            Some(*c)
        } else {
            None
        }
    }

    /// Try to get this value as a 2D vector.
    pub fn as_vector2(&self) -> Option<(f64, f64)> {
        if let Value::Vector2(x, y) = self {
            Some((*x, *y))
        } else {
            None
        }
    }

    /// Try to get this value as a 3D vector.
    pub fn as_vector3(&self) -> Option<(f64, f64, f64)> {
        if let Value::Vector3(x, y, z) = self {
            Some((*x, *y, *z))
        } else {
            None
        }
    }

    /// Try to get this value as an array reference.
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        if let Value::Array(arr) = self {
            Some(arr)
        } else {
            None
        }
    }

    /// Try to get this value as a map reference.
    pub fn as_map(&self) -> Option<&HashMap<String, Value>> {
        if let Value::Map(map) = self {
            Some(map)
        } else {
            None
        }
    }

    /// Check if this value is None.
    pub fn is_none(&self) -> bool {
        matches!(self, Value::None)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Image(img) => write!(f, "Image({}x{})", img.metadata.width, img.metadata.height),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{:.4}", fl),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Color(c) => write!(f, "Color({}, {}, {}, {})", c.r, c.g, c.b, c.a),
            Value::Vector2(x, y) => write!(f, "Vec2({:.2}, {:.2})", x, y),
            Value::Vector3(x, y, z) => write!(f, "Vec3({:.2}, {:.2}, {:.2})", x, y, z),
            Value::Array(arr) => write!(f, "Array[{}]", arr.len()),
            Value::Map(map) => write!(f, "Map{{{} entries}}", map.len()),
            Value::None => write!(f, "None"),
        }
    }
}

// ============================================================================
// PortType Implementation
// ============================================================================

impl PortType {
    /// Check if a value matches this port type.
    pub fn matches(&self, value: &Value) -> bool {
        match (self, value) {
            // Any accepts everything
            (PortType::Any, _) => true,
            // Direct matches
            (PortType::Image, Value::Image(_)) => true,
            (PortType::Integer, Value::Integer(_)) => true,
            (PortType::Float, Value::Float(_)) => true,
            // Integer can be used where float is expected (implicit conversion)
            (PortType::Float, Value::Integer(_)) => true,
            (PortType::String, Value::String(_)) => true,
            (PortType::Boolean, Value::Boolean(_)) => true,
            (PortType::Color, Value::Color(_)) => true,
            (PortType::Vector2, Value::Vector2(_, _)) => true,
            (PortType::Vector3, Value::Vector3(_, _, _)) => true,
            // Container types - check inner type
            (PortType::Array(inner), Value::Array(arr)) => {
                arr.is_empty() || arr.iter().all(|v| inner.matches(v))
            }
            (PortType::Map(inner), Value::Map(map)) => {
                map.is_empty() || map.values().all(|v| inner.matches(v))
            }
            _ => false,
        }
    }

    /// Check if this type is compatible with another (for connections).
    ///
    /// This is used when validating if two ports can be connected.
    pub fn compatible_with(&self, other: &PortType) -> bool {
        match (self, other) {
            // Any is compatible with everything
            (PortType::Any, _) | (_, PortType::Any) => true,
            // Integer output can connect to Float input (implicit conversion)
            (PortType::Integer, PortType::Float) => true,
            // Container covariance
            (PortType::Array(a), PortType::Array(b)) => a.compatible_with(b),
            (PortType::Map(a), PortType::Map(b)) => a.compatible_with(b),
            // Direct match
            (a, b) => a == b,
        }
    }

    /// Get a human-readable name for this type.
    pub fn display_name(&self) -> String {
        match self {
            PortType::Image => "Image".to_string(),
            PortType::Integer => "Integer".to_string(),
            PortType::Float => "Float".to_string(),
            PortType::String => "String".to_string(),
            PortType::Boolean => "Boolean".to_string(),
            PortType::Color => "Color".to_string(),
            PortType::Vector2 => "Vector2".to_string(),
            PortType::Vector3 => "Vector3".to_string(),
            PortType::Array(inner) => format!("Array<{}>", inner.display_name()),
            PortType::Map(inner) => format!("Map<String, {}>", inner.display_name()),
            PortType::Any => "Any".to_string(),
        }
    }
}

impl fmt::Display for PortType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// ImageValue Implementation
// ============================================================================

impl ImageValue {
    /// Create a new ImageValue from a DynamicImage.
    pub fn new(image: DynamicImage) -> Self {
        let width = image.width();
        let height = image.height();
        let has_alpha = matches!(
            image,
            DynamicImage::ImageRgba8(_)
                | DynamicImage::ImageRgba16(_)
                | DynamicImage::ImageRgba32F(_)
                | DynamicImage::ImageLumaA8(_)
                | DynamicImage::ImageLumaA16(_)
        );

        Self {
            metadata: ImageMetadata {
                width,
                height,
                format: ImageFormat::Unknown,
                has_alpha,
            },
            data: Some(Arc::new(image)),
            data_ref: ImageDataRef::InMemory,
        }
    }

    /// Load an image from a file path.
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self, image::ImageError> {
        let path = path.into();
        let image = image::open(&path)?;
        let width = image.width();
        let height = image.height();
        let has_alpha = matches!(
            image,
            DynamicImage::ImageRgba8(_)
                | DynamicImage::ImageRgba16(_)
                | DynamicImage::ImageRgba32F(_)
                | DynamicImage::ImageLumaA8(_)
                | DynamicImage::ImageLumaA16(_)
        );

        Ok(Self {
            metadata: ImageMetadata {
                width,
                height,
                format: ImageFormat::from_path(&path),
                has_alpha,
            },
            data: Some(Arc::new(image)),
            data_ref: ImageDataRef::FilePath(path),
        })
    }

    /// Create ImageValue with only metadata (for validation without loading).
    pub fn from_metadata(metadata: ImageMetadata, path: PathBuf) -> Self {
        Self {
            metadata,
            data: None,
            data_ref: ImageDataRef::FilePath(path),
        }
    }

    /// Get a shared reference to the underlying image.
    ///
    /// Returns None if the image hasn't been loaded yet.
    pub fn get_image(&self) -> Option<&DynamicImage> {
        self.data.as_ref().map(|arc| arc.as_ref())
    }

    /// Get a mutable reference to the underlying image.
    ///
    /// Uses copy-on-write semantics: if the image is shared,
    /// it will be cloned before modification.
    pub fn get_image_mut(&mut self) -> Option<&mut DynamicImage> {
        self.data.as_mut().map(|arc| Arc::make_mut(arc))
    }

    /// Take ownership of the underlying image.
    ///
    /// If the image is shared, this will clone it.
    pub fn into_image(self) -> Option<DynamicImage> {
        self.data.map(|arc| Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone()))
    }

    /// Check if the image data is currently loaded.
    pub fn is_loaded(&self) -> bool {
        self.data.is_some()
    }

    /// Get the estimated memory size of this image in bytes.
    pub fn estimated_memory_size(&self) -> usize {
        // RGBA = 4 bytes per pixel
        (self.metadata.width as usize) * (self.metadata.height as usize) * 4
    }
}

impl Default for ImageValue {
    fn default() -> Self {
        Self {
            metadata: ImageMetadata {
                width: 0,
                height: 0,
                format: ImageFormat::Unknown,
                has_alpha: false,
            },
            data: None,
            data_ref: ImageDataRef::InMemory,
        }
    }
}

// ============================================================================
// ImageFormat Implementation
// ============================================================================

impl ImageFormat {
    /// Determine image format from file path extension.
    pub fn from_path(path: &std::path::Path) -> Self {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "png" => ImageFormat::Png,
            "gif" => ImageFormat::Gif,
            "webp" => ImageFormat::WebP,
            "tiff" | "tif" => ImageFormat::Tiff,
            "bmp" => ImageFormat::Bmp,
            _ => ImageFormat::Unknown,
        }
    }

    /// Convert to image crate's ImageFormat.
    pub fn to_image_format(&self) -> Option<image::ImageFormat> {
        match self {
            ImageFormat::Jpeg => Some(image::ImageFormat::Jpeg),
            ImageFormat::Png => Some(image::ImageFormat::Png),
            ImageFormat::Gif => Some(image::ImageFormat::Gif),
            ImageFormat::WebP => Some(image::ImageFormat::WebP),
            ImageFormat::Tiff => Some(image::ImageFormat::Tiff),
            ImageFormat::Bmp => Some(image::ImageFormat::Bmp),
            ImageFormat::Unknown => None,
        }
    }

    /// Get the typical file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Gif => "gif",
            ImageFormat::WebP => "webp",
            ImageFormat::Tiff => "tiff",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Unknown => "bin",
        }
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageFormat::Jpeg => write!(f, "JPEG"),
            ImageFormat::Png => write!(f, "PNG"),
            ImageFormat::Gif => write!(f, "GIF"),
            ImageFormat::WebP => write!(f, "WebP"),
            ImageFormat::Tiff => write!(f, "TIFF"),
            ImageFormat::Bmp => write!(f, "BMP"),
            ImageFormat::Unknown => write!(f, "Unknown"),
        }
    }
}

// ============================================================================
// Color Implementation
// ============================================================================

impl Color {
    /// Create a new color from RGBA components.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a color from RGB components (alpha = 255).
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Parse a hex color string.
    ///
    /// Supports formats: "#RGB", "#RGBA", "#RRGGBB", "#RRGGBBAA"
    pub fn from_hex(hex: &str) -> Result<Self, String> {
        let hex = hex.trim_start_matches('#');

        match hex.len() {
            3 => {
                // #RGB format
                let r = u8::from_str_radix(&hex[0..1], 16).map_err(|e| e.to_string())? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).map_err(|e| e.to_string())? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).map_err(|e| e.to_string())? * 17;
                Ok(Self::rgb(r, g, b))
            }
            4 => {
                // #RGBA format
                let r = u8::from_str_radix(&hex[0..1], 16).map_err(|e| e.to_string())? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).map_err(|e| e.to_string())? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).map_err(|e| e.to_string())? * 17;
                let a = u8::from_str_radix(&hex[3..4], 16).map_err(|e| e.to_string())? * 17;
                Ok(Self::new(r, g, b, a))
            }
            6 => {
                // #RRGGBB format
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| e.to_string())?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| e.to_string())?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| e.to_string())?;
                Ok(Self::rgb(r, g, b))
            }
            8 => {
                // #RRGGBBAA format
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| e.to_string())?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| e.to_string())?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| e.to_string())?;
                let a = u8::from_str_radix(&hex[6..8], 16).map_err(|e| e.to_string())?;
                Ok(Self::new(r, g, b, a))
            }
            _ => Err(format!(
                "Invalid hex color format: expected 3, 4, 6, or 8 characters, got {}",
                hex.len()
            )),
        }
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }

    /// Convert to image crate's Rgba type.
    pub fn to_rgba(&self) -> image::Rgba<u8> {
        image::Rgba([self.r, self.g, self.b, self.a])
    }

    /// Create from image crate's Rgba type.
    pub fn from_rgba(rgba: image::Rgba<u8>) -> Self {
        Self::new(rgba[0], rgba[1], rgba[2], rgba[3])
    }

    /// Common colors
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const TRANSPARENT: Color = Color::new(0, 0, 0, 0);
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_type_matching() {
        assert!(PortType::Integer.matches(&Value::Integer(42)));
        assert!(PortType::Float.matches(&Value::Float(3.14)));
        assert!(PortType::Float.matches(&Value::Integer(42))); // Implicit conversion
        assert!(!PortType::Integer.matches(&Value::Float(3.14))); // No downcast
        assert!(PortType::Any.matches(&Value::String("test".to_string())));
    }

    #[test]
    fn test_port_type_compatibility() {
        assert!(PortType::Integer.compatible_with(&PortType::Integer));
        assert!(PortType::Integer.compatible_with(&PortType::Float)); // Can connect int to float
        assert!(PortType::Any.compatible_with(&PortType::Image));
        assert!(!PortType::String.compatible_with(&PortType::Integer));
    }

    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#FF0000").unwrap();
        assert_eq!(color, Color::RED);

        let color = Color::from_hex("#00FF00FF").unwrap();
        assert_eq!(color, Color::new(0, 255, 0, 255));

        let color = Color::from_hex("F00").unwrap();
        assert_eq!(color, Color::rgb(255, 0, 0));
    }

    #[test]
    fn test_color_to_hex() {
        assert_eq!(Color::RED.to_hex(), "#FF0000");
        assert_eq!(Color::new(0, 255, 0, 128).to_hex(), "#00FF0080");
    }

    #[test]
    fn test_value_type_inference() {
        assert_eq!(Value::Integer(42).get_type(), PortType::Integer);
        assert_eq!(Value::Float(3.14).get_type(), PortType::Float);
        assert_eq!(
            Value::Array(vec![Value::Integer(1), Value::Integer(2)]).get_type(),
            PortType::Array(Box::new(PortType::Integer))
        );
    }
}
