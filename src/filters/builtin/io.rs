//! Image I/O filters: LoadImage, SaveImage

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use std::path::Path;

/// Register I/O filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(LoadImage));
    registry.register(|| Box::new(LoadFolder));
    registry.register(|| Box::new(SaveImage));
}

/// Loads an image from disk.
#[derive(Debug, Clone)]
pub struct LoadImage;

impl FilterNode for LoadImage {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("load_image", "Load Image")
            .description("Load an image from a file path")
            .category(Category::Input)
            .author("Ambara")
            .version("1.0.0")
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("The loaded image")
            )
            .parameter(
                ParameterDefinition::new("path", PortType::String, Value::String(String::new()))
                    .with_description("Path to the image file")
                    .with_ui_hint(UiHint::FileChooser { filters: vec!["*.png".to_string(), "*.jpg".to_string(), "*.jpeg".to_string(), "*.gif".to_string(), "*.bmp".to_string(), "*.tiff".to_string(), "*.webp".to_string()] })
                    .with_constraint(Constraint::NotEmpty),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let path = ctx.get_string("path").unwrap_or("");
        
        if path.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "path".to_string(),
                error: "Path cannot be empty".to_string(),
            });
        }

        // Check if file exists
        if !Path::new(path).exists() {
            return Err(ValidationError::ResourceNotFound {
                node_id: ctx.node_id,
                resource: path.to_string(),
            });
        }

        // Validate extension
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let valid_extensions = ["png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp"];
        if !valid_extensions.contains(&extension.as_str()) {
            return Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: format!("Unsupported image format: {}", extension),
            });
        }

        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let path = ctx.get_string("path").map_err(|_| ExecutionError::MissingParameter {
            node_id: ctx.node_id,
            parameter: "path".to_string(),
        })?;

        // Load the image
        let img = image::open(path).map_err(|e| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: format!("Failed to load image: {}", e),
        })?;

        // Create ImageValue from DynamicImage
        let image_value = ImageValue::new(img);

        ctx.set_output("image", Value::Image(image_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Loads all images from a folder.
#[derive(Debug, Clone)]
pub struct LoadFolder;

impl FilterNode for LoadFolder {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("load_folder", "Load Folder")
            .description("Load all images from a folder (batch processing)")
            .category(Category::Input)
            .author("Ambara")
            .version("1.0.0")
            .output(
                PortDefinition::output("images", PortType::Array(Box::new(PortType::Image)))
                    .with_description("Array of loaded images")
            )
            .output(
                PortDefinition::output("count", PortType::Integer)
                    .with_description("Number of images loaded")
            )
            .parameter(
                ParameterDefinition::new("path", PortType::String, Value::String(String::new()))
                    .with_description("Path to the folder containing images")
                    .with_ui_hint(UiHint::FileChooser { filters: vec![] })
                    .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new("recursive", PortType::Boolean, Value::Boolean(false))
                    .with_description("Search subfolders recursively"),
            )
            .parameter(
                ParameterDefinition::new("pattern", PortType::String, Value::String("*".to_string()))
                    .with_description("Filename pattern (e.g., *.png, image_*.jpg)"),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let path = ctx.get_string("path").unwrap_or("");
        
        if path.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "path".to_string(),
                error: "Path cannot be empty".to_string(),
            });
        }

        let folder_path = Path::new(path);
        if !folder_path.exists() {
            return Err(ValidationError::ResourceNotFound {
                node_id: ctx.node_id,
                resource: path.to_string(),
            });
        }

        if !folder_path.is_dir() {
            return Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: "Path must be a directory".to_string(),
            });
        }

        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let path = ctx.get_string("path").map_err(|_| ExecutionError::MissingParameter {
            node_id: ctx.node_id,
            parameter: "path".to_string(),
        })?;
        
        let recursive = ctx.get_bool("recursive").unwrap_or(false);
        let pattern = ctx.get_string("pattern").unwrap_or("*");

        let folder_path = Path::new(path);
        let valid_extensions = ["png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp"];

        let mut images = Vec::new();

        if recursive {
            for entry in walkdir::WalkDir::new(folder_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                
                // Check extension
                let extension = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                if !valid_extensions.contains(&extension.as_str()) {
                    continue;
                }

                // Check pattern match
                if pattern != "*" {
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let pattern_glob = glob::Pattern::new(pattern).map_err(|e| ExecutionError::NodeExecution {
                        node_id: ctx.node_id,
                        error: format!("Invalid pattern: {}", e),
                    })?;
                    if !pattern_glob.matches(filename) {
                        continue;
                    }
                }

                // Load the image
                match image::open(&path) {
                    Ok(img) => {
                        let image_value = ImageValue::new(img);
                        images.push(Value::Image(image_value));
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load {}: {}", path.display(), e);
                    }
                }
            }
        } else {
            let entries = std::fs::read_dir(folder_path)
                .map_err(|e| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: format!("Failed to read directory: {}", e),
                })?;

            for entry in entries {
                let entry = entry.map_err(|e| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: format!("Failed to read directory entry: {}", e),
                })?;

                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                // Check extension
                let extension = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                if !valid_extensions.contains(&extension.as_str()) {
                    continue;
                }

                // Check pattern match
                if pattern != "*" {
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let pattern_glob = glob::Pattern::new(pattern).map_err(|e| ExecutionError::NodeExecution {
                        node_id: ctx.node_id,
                        error: format!("Invalid pattern: {}", e),
                    })?;
                    if !pattern_glob.matches(filename) {
                        continue;
                    }
                }

                // Load the image
                match image::open(&path) {
                    Ok(img) => {
                        let image_value = ImageValue::new(img);
                        images.push(Value::Image(image_value));
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load {}: {}", path.display(), e);
                    }
                }
            }
        }

        let count = images.len() as i64;
        ctx.set_output("images", Value::Array(images))?;
        ctx.set_output("count", Value::Integer(count))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Saves an image to disk.
#[derive(Debug, Clone)]
pub struct SaveImage;

impl FilterNode for SaveImage {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("save_image", "Save Image")
            .description("Save an image to a file path")
            .category(Category::Output)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("The image to save")
            )
            .output(
                PortDefinition::output("path", PortType::String)
                    .with_description("The path where the image was saved")
            )
            .parameter(
                ParameterDefinition::new("path", PortType::String, Value::String(String::new()))
                    .with_description("Output file path")
                    .with_ui_hint(UiHint::FileChooser { filters: vec!["*.png".to_string(), "*.jpg".to_string(), "*.jpeg".to_string()] })
                    .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new("quality", PortType::Integer, Value::Integer(90))
                    .with_description("JPEG quality (1-100)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 1.0, max: 100.0 }),
            )
            .parameter(
                ParameterDefinition::new("create_dirs", PortType::Boolean, Value::Boolean(true))
                    .with_description("Create parent directories if they don't exist"),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let path = ctx.get_string("path").unwrap_or("");
        
        if path.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "path".to_string(),
                error: "Path cannot be empty".to_string(),
            });
        }

        // Validate extension
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let valid_extensions = ["png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp"];
        if !valid_extensions.contains(&extension.as_str()) {
            return Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: format!("Unsupported output format: {}", extension),
            });
        }

        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let path = ctx.get_string("path").map_err(|_| ExecutionError::MissingParameter {
            node_id: ctx.node_id,
            parameter: "path".to_string(),
        })?.to_string();

        let image = ctx.get_input_image("image")?;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let create_dirs = ctx.get_bool("create_dirs").unwrap_or(true);
        let quality = ctx.get_integer("quality").unwrap_or(90) as u8;

        // Create parent directories if needed
        if create_dirs {
            if let Some(parent) = Path::new(&path).parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| ExecutionError::NodeExecution {
                        node_id: ctx.node_id,
                        error: format!("Failed to create directories: {}", e),
                    })?;
                }
            }
        }

        // Determine format from extension
        let extension = Path::new(&path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_lowercase();

        // Get the raw image buffer
        let buffer = img_data.to_rgba8();

        // Save based on format
        match extension.as_str() {
            "jpg" | "jpeg" => {
                let rgb = image::DynamicImage::ImageRgba8(buffer.clone()).to_rgb8();
                let mut output = std::io::BufWriter::new(
                    std::fs::File::create(&path).map_err(|e| ExecutionError::NodeExecution {
                        node_id: ctx.node_id,
                        error: format!("Failed to create file: {}", e),
                    })?,
                );
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, quality);
                encoder.encode(
                    &rgb,
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                ).map_err(|e| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: format!("Failed to encode JPEG: {}", e),
                })?;
            }
            _ => {
                buffer.save(&path).map_err(|e| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: format!("Failed to save image: {}", e),
                })?;
            }
        }

        ctx.set_output("path", Value::String(path))?;
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
    fn test_load_image_metadata() {
        let filter = LoadImage;
        let metadata = filter.metadata();
        
        assert_eq!(metadata.id, "load_image");
        assert_eq!(metadata.category, Category::Input);
        assert_eq!(metadata.outputs.len(), 1);
        assert_eq!(metadata.parameters.len(), 1);
    }

    #[test]
    fn test_save_image_metadata() {
        let filter = SaveImage;
        let metadata = filter.metadata();
        
        assert_eq!(metadata.id, "save_image");
        assert_eq!(metadata.category, Category::Output);
        assert_eq!(metadata.inputs.len(), 1);
        assert_eq!(metadata.parameters.len(), 3);
    }
}
