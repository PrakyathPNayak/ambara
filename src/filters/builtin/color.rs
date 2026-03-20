//! Color adjustment filters with optional GPU acceleration

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::gpu::{GpuAccelerated, GpuFilters, GpuPool};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::Pixel;

/// Register color filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(Brightness));
    registry.register(|| Box::new(Contrast));
    registry.register(|| Box::new(Saturation));
    registry.register(|| Box::new(Grayscale));
    registry.register(|| Box::new(Invert));
    registry.register(|| Box::new(Sepia));
    registry.register(|| Box::new(HueRotate));
    registry.register(|| Box::new(Threshold));
    registry.register(|| Box::new(Posterize));
    registry.register(|| Box::new(GammaCorrection));
    registry.register(|| Box::new(ColorBalance));
}

/// Adjusts image brightness.
#[derive(Debug, Clone)]
pub struct Brightness;

impl FilterNode for Brightness {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("brightness", "Brightness")
            .description("Adjust the brightness of an image")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Adjusted image")
            )
            .parameter(
                ParameterDefinition::new("amount", PortType::Float, Value::Float(0.0))
                    .with_description("Brightness adjustment (-1.0 to 1.0)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: -1.0, max: 1.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let amount = ctx.get_float("amount").unwrap_or(0.0) as f32;
        let adjustment = (amount * 255.0) as i32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let mut result = img_data.to_rgba8();
        
        for pixel in result.pixels_mut() {
            let channels = pixel.channels_mut();
            for i in 0..3 {
                channels[i] = (channels[i] as i32 + adjustment).clamp(0, 255) as u8;
            }
        }

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(result));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Adjusts image contrast.
#[derive(Debug, Clone)]
pub struct Contrast;

impl FilterNode for Contrast {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("contrast", "Contrast")
            .description("Adjust the contrast of an image")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Adjusted image")
            )
            .parameter(
                ParameterDefinition::new("amount", PortType::Float, Value::Float(1.0))
                    .with_description("Contrast multiplier (0.0 to 3.0)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 3.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let factor = ctx.get_float("amount").unwrap_or(1.0) as f32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img_data.to_rgba8();
        
        // Apply contrast adjustment manually
        // Contrast is applied as: new_value = (old_value - 128) * factor + 128
        let result = image::ImageBuffer::from_fn(rgba.width(), rgba.height(), |x, y| {
            let pixel = rgba.get_pixel(x, y);
            let adjust = |v: u8| -> u8 {
                let adjusted = ((v as f32 - 128.0) * factor + 128.0).clamp(0.0, 255.0);
                adjusted as u8
            };
            image::Rgba([adjust(pixel[0]), adjust(pixel[1]), adjust(pixel[2]), pixel[3]])
        });

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(result));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Adjusts image saturation.
#[derive(Debug, Clone)]
pub struct Saturation;

impl FilterNode for Saturation {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("saturation", "Saturation")
            .description("Adjust the color saturation of an image")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Adjusted image")
            )
            .parameter(
                ParameterDefinition::new("amount", PortType::Float, Value::Float(1.0))
                    .with_description("Saturation multiplier (0.0 = grayscale, 1.0 = original)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 3.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;

        let saturation = ctx.get_float("amount").unwrap_or(1.0) as f32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let mut result = img_data.to_rgba8();

        for pixel in result.pixels_mut() {
            let channels = pixel.channels_mut();
            let r = channels[0] as f32 / 255.0;
            let g = channels[1] as f32 / 255.0;
            let b = channels[2] as f32 / 255.0;

            // Calculate luminance
            let luma = 0.299 * r + 0.587 * g + 0.114 * b;

            // Interpolate between grayscale and original
            let new_r = luma + (r - luma) * saturation;
            let new_g = luma + (g - luma) * saturation;
            let new_b = luma + (b - luma) * saturation;

            channels[0] = (new_r.clamp(0.0, 1.0) * 255.0) as u8;
            channels[1] = (new_g.clamp(0.0, 1.0) * 255.0) as u8;
            channels[2] = (new_b.clamp(0.0, 1.0) * 255.0) as u8;
        }

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(result));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Converts image to grayscale with GPU acceleration.
#[derive(Debug, Clone)]
pub struct Grayscale;

impl FilterNode for Grayscale {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("grayscale", "Grayscale")
            .description("Convert an image to grayscale (GPU accelerated when available)")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Grayscale image")
            )
            .parameter(
                ParameterDefinition::new("use_gpu", PortType::Boolean, Value::Boolean(true))
                    .with_description("Use GPU acceleration if available"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let use_gpu = ctx.get_bool("use_gpu").unwrap_or(true);

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        // Try GPU acceleration
        if use_gpu {
            if let Some(device) = GpuPool::global().device() {
                if let Ok(filters) = GpuFilters::new(device) {
                    if let Ok(result) = filters.grayscale(img_data) {
                        let result_value = ImageValue::new(result);
                        ctx.set_output("image", Value::Image(result_value))?;
                        return Ok(());
                    }
                }
            }
        }

        // Fallback to CPU
        let gray = img_data.grayscale();
        let rgba = gray.to_rgba8();

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(rgba));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl GpuAccelerated for Grayscale {
    fn supports_gpu(&self) -> bool {
        true
    }
}

/// Inverts image colors with GPU acceleration.
#[derive(Debug, Clone)]
pub struct Invert;

impl FilterNode for Invert {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("invert", "Invert Colors")
            .description("Invert the colors of an image (GPU accelerated when available)")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Inverted image")
            )
            .parameter(
                ParameterDefinition::new("invert_alpha", PortType::Boolean, Value::Boolean(false))
                    .with_description("Also invert the alpha channel"),
            )
            .parameter(
                ParameterDefinition::new("use_gpu", PortType::Boolean, Value::Boolean(true))
                    .with_description("Use GPU acceleration if available"),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let invert_alpha = ctx.get_bool("invert_alpha").unwrap_or(false);
        let use_gpu = ctx.get_bool("use_gpu").unwrap_or(true);

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        // Try GPU acceleration (only if not inverting alpha, GPU shader doesn't support that)
        if use_gpu && !invert_alpha {
            if let Some(device) = GpuPool::global().device() {
                if let Ok(filters) = GpuFilters::new(device) {
                    if let Ok(result) = filters.invert(img_data) {
                        let result_value = ImageValue::new(result);
                        ctx.set_output("image", Value::Image(result_value))?;
                        return Ok(());
                    }
                }
            }
        }

        // Fallback to CPU
        let mut result = img_data.to_rgba8();

        for pixel in result.pixels_mut() {
            let channels = pixel.channels_mut();
            channels[0] = 255 - channels[0];
            channels[1] = 255 - channels[1];
            channels[2] = 255 - channels[2];
            if invert_alpha {
                channels[3] = 255 - channels[3];
            }
        }

        let result_value = ImageValue::new(image::DynamicImage::ImageRgba8(result));

        ctx.set_output("image", Value::Image(result_value))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

impl GpuAccelerated for Invert {
    fn supports_gpu(&self) -> bool {
        true
    }
}

/// Apply a sepia tone to an image.
#[derive(Debug, Clone)]
pub struct Sepia;

impl FilterNode for Sepia {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("sepia", "Sepia")
            .description("Apply a warm sepia tone to an image")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(PortDefinition::input("image", PortType::Image).with_description("Input image"))
            .output(PortDefinition::output("image", PortType::Image).with_description("Sepia-toned image"))
            .parameter(
                ParameterDefinition::new("intensity", PortType::Float, Value::Float(1.0))
                    .with_description("Sepia intensity (0.0 = original, 1.0 = full sepia)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 1.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> { Ok(()) }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let intensity = ctx.get_float("intensity").unwrap_or(1.0) as f32;
        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id, error: "Image has no data".to_string(),
        })?;
        let mut result = img_data.to_rgba8();
        for pixel in result.pixels_mut() {
            let ch = pixel.channels_mut();
            let r = ch[0] as f32; let g = ch[1] as f32; let b = ch[2] as f32;
            let sr = (0.393 * r + 0.769 * g + 0.189 * b).min(255.0);
            let sg = (0.349 * r + 0.686 * g + 0.168 * b).min(255.0);
            let sb = (0.272 * r + 0.534 * g + 0.131 * b).min(255.0);
            ch[0] = (r + (sr - r) * intensity).clamp(0.0, 255.0) as u8;
            ch[1] = (g + (sg - g) * intensity).clamp(0.0, 255.0) as u8;
            ch[2] = (b + (sb - b) * intensity).clamp(0.0, 255.0) as u8;
        }
        ctx.set_output("image", Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> { Box::new(self.clone()) }
}

/// Rotate the hue of all pixels.
#[derive(Debug, Clone)]
pub struct HueRotate;

impl FilterNode for HueRotate {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("hue_rotate", "Hue Rotate")
            .description("Rotate the hue of all pixels by a given angle")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(PortDefinition::input("image", PortType::Image).with_description("Input image"))
            .output(PortDefinition::output("image", PortType::Image).with_description("Hue-shifted image"))
            .parameter(
                ParameterDefinition::new("angle", PortType::Float, Value::Float(0.0))
                    .with_description("Hue rotation angle in degrees")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: -360.0, max: 360.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> { Ok(()) }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let angle = ctx.get_float("angle").unwrap_or(0.0) as f32;
        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id, error: "Image has no data".to_string(),
        })?;
        let mut result = img_data.to_rgba8();
        for pixel in result.pixels_mut() {
            let ch = pixel.channels_mut();
            let (h, s, l) = rgb_to_hsl(ch[0], ch[1], ch[2]);
            let new_h = (h + angle).rem_euclid(360.0);
            let (r, g, b) = hsl_to_rgb(new_h, s, l);
            ch[0] = r; ch[1] = g; ch[2] = b;
        }
        ctx.set_output("image", Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> { Box::new(self.clone()) }
}

/// Apply a binary threshold to an image.
#[derive(Debug, Clone)]
pub struct Threshold;

impl FilterNode for Threshold {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("threshold", "Threshold")
            .description("Convert image to black and white using a threshold value")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(PortDefinition::input("image", PortType::Image).with_description("Input image"))
            .output(PortDefinition::output("image", PortType::Image).with_description("Thresholded image"))
            .parameter(
                ParameterDefinition::new("level", PortType::Integer, Value::Integer(128))
                    .with_description("Threshold level (0-255)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 255.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> { Ok(()) }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let level = ctx.get_integer("level").unwrap_or(128) as u8;
        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id, error: "Image has no data".to_string(),
        })?;
        let mut result = img_data.to_rgba8();
        for pixel in result.pixels_mut() {
            let ch = pixel.channels_mut();
            let luma = (0.299 * ch[0] as f32 + 0.587 * ch[1] as f32 + 0.114 * ch[2] as f32) as u8;
            let v = if luma >= level { 255u8 } else { 0u8 };
            ch[0] = v; ch[1] = v; ch[2] = v;
        }
        ctx.set_output("image", Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> { Box::new(self.clone()) }
}

/// Reduce the number of distinct color levels (posterize effect).
#[derive(Debug, Clone)]
pub struct Posterize;

impl FilterNode for Posterize {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("posterize", "Posterize")
            .description("Reduce the number of color levels in an image")
            .category(Category::Color)
            .author("Ambara")
            .version("1.0.0")
            .input(PortDefinition::input("image", PortType::Image).with_description("Input image"))
            .output(PortDefinition::output("image", PortType::Image).with_description("Posterized image"))
            .parameter(
                ParameterDefinition::new("levels", PortType::Integer, Value::Integer(4))
                    .with_description("Number of color levels per channel (2-256)")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::Range { min: 2.0, max: 256.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> { Ok(()) }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let levels = ctx.get_integer("levels").unwrap_or(4).max(2) as f32;
        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id, error: "Image has no data".to_string(),
        })?;
        let mut result = img_data.to_rgba8();
        let step = 255.0 / (levels - 1.0);
        for pixel in result.pixels_mut() {
            let ch = pixel.channels_mut();
            for i in 0..3 {
                let bucket = (ch[i] as f32 / 255.0 * (levels - 1.0)).round();
                ch[i] = (bucket * step).clamp(0.0, 255.0) as u8;
            }
        }
        ctx.set_output("image", Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> { Box::new(self.clone()) }
}

/// Apply gamma correction to an image.
#[derive(Debug, Clone)]
pub struct GammaCorrection;

impl FilterNode for GammaCorrection {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("gamma", "Gamma Correction")
            .description("Apply gamma correction to adjust image luminance")
            .category(Category::Adjust)
            .author("Ambara")
            .version("1.0.0")
            .input(PortDefinition::input("image", PortType::Image).with_description("Input image"))
            .output(PortDefinition::output("image", PortType::Image).with_description("Gamma-corrected image"))
            .parameter(
                ParameterDefinition::new("gamma", PortType::Float, Value::Float(1.0))
                    .with_description("Gamma value (< 1.0 brightens, > 1.0 darkens)")
                    .with_ui_hint(UiHint::Slider { logarithmic: true })
                    .with_constraint(Constraint::Range { min: 0.1, max: 5.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> { Ok(()) }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let gamma = ctx.get_float("gamma").unwrap_or(1.0) as f32;
        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id, error: "Image has no data".to_string(),
        })?;
        let mut result = img_data.to_rgba8();
        let inv_gamma = 1.0 / gamma;
        // Build lookup table
        let lut: Vec<u8> = (0..256).map(|i| {
            ((i as f32 / 255.0).powf(inv_gamma) * 255.0).clamp(0.0, 255.0) as u8
        }).collect();
        for pixel in result.pixels_mut() {
            let ch = pixel.channels_mut();
            ch[0] = lut[ch[0] as usize];
            ch[1] = lut[ch[1] as usize];
            ch[2] = lut[ch[2] as usize];
        }
        ctx.set_output("image", Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> { Box::new(self.clone()) }
}

/// Adjust RGB channels independently.
#[derive(Debug, Clone)]
pub struct ColorBalance;

impl FilterNode for ColorBalance {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("color_balance", "Color Balance")
            .description("Adjust red, green, and blue channel levels independently")
            .category(Category::Adjust)
            .author("Ambara")
            .version("1.0.0")
            .input(PortDefinition::input("image", PortType::Image).with_description("Input image"))
            .output(PortDefinition::output("image", PortType::Image).with_description("Color-balanced image"))
            .parameter(
                ParameterDefinition::new("red", PortType::Float, Value::Float(1.0))
                    .with_description("Red channel multiplier")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 3.0 }),
            )
            .parameter(
                ParameterDefinition::new("green", PortType::Float, Value::Float(1.0))
                    .with_description("Green channel multiplier")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 3.0 }),
            )
            .parameter(
                ParameterDefinition::new("blue", PortType::Float, Value::Float(1.0))
                    .with_description("Blue channel multiplier")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 3.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> { Ok(()) }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let r_mult = ctx.get_float("red").unwrap_or(1.0) as f32;
        let g_mult = ctx.get_float("green").unwrap_or(1.0) as f32;
        let b_mult = ctx.get_float("blue").unwrap_or(1.0) as f32;
        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id, error: "Image has no data".to_string(),
        })?;
        let mut result = img_data.to_rgba8();
        for pixel in result.pixels_mut() {
            let ch = pixel.channels_mut();
            ch[0] = (ch[0] as f32 * r_mult).clamp(0.0, 255.0) as u8;
            ch[1] = (ch[1] as f32 * g_mult).clamp(0.0, 255.0) as u8;
            ch[2] = (ch[2] as f32 * b_mult).clamp(0.0, 255.0) as u8;
        }
        ctx.set_output("image", Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> { Box::new(self.clone()) }
}

// --- HSL color space utilities ---

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - r).abs() < 1e-6 {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-6 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    if s.abs() < 1e-6 {
        let v = (l * 255.0).clamp(0.0, 255.0) as u8;
        return (v, v, v);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let hk = h / 360.0;
    let to_rgb = |t: f32| -> u8 {
        let t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (v * 255.0).clamp(0.0, 255.0) as u8
    };
    (to_rgb(hk + 1.0 / 3.0), to_rgb(hk), to_rgb(hk - 1.0 / 3.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brightness_metadata() {
        let filter = Brightness;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "brightness");
        assert_eq!(metadata.category, Category::Color);
    }

    #[test]
    fn test_grayscale_metadata() {
        let filter = Grayscale;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "grayscale");
        // Now has use_gpu parameter
        assert_eq!(metadata.parameters.len(), 1);
    }

    #[test]
    fn test_invert_metadata() {
        let filter = Invert;
        let metadata = filter.metadata();
        assert_eq!(metadata.id, "invert");
        // Now has invert_alpha and use_gpu parameters
        assert_eq!(metadata.parameters.len(), 2);
    }

    #[test]
    fn test_gpu_support() {
        assert!(Grayscale.supports_gpu());
        assert!(Invert.supports_gpu());
    }
}
