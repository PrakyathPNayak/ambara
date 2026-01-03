//! Astrophotography filters: Image Stacking, Dark Frame Subtraction, Flat Field Correction

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

/// Register astrophotography filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(ImageStack));
    registry.register(|| Box::new(DarkFrameSubtract));
    registry.register(|| Box::new(FlatFieldCorrect));
    registry.register(|| Box::new(HotPixelRemoval));
    registry.register(|| Box::new(HistogramStretch));
}

/// Stack multiple images using various algorithms.
/// 
/// Common astrophotography technique to reduce noise and increase signal.
#[derive(Debug, Clone)]
pub struct ImageStack;

impl FilterNode for ImageStack {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("image_stack", "Image Stack")
            .description("Stack multiple images to reduce noise and increase signal-to-noise ratio")
            .category(Category::Custom) // We'll use Custom for Astro
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("images", PortType::Array(Box::new(PortType::Image)))
                    .with_description("List of images to stack")
            )
            .parameter(
                ParameterDefinition::new("method", PortType::String, Value::String("mean".to_string()))
                    .with_description("Stacking method: mean, median, sigma_clip, max, min")
                    .with_ui_hint(UiHint::Dropdown { options: vec![
                        "mean".to_string(),
                        "median".to_string(),
                        "sigma_clip".to_string(),
                        "max".to_string(),
                        "min".to_string(),
                    ]})
            )
            .parameter(
                ParameterDefinition::new("sigma", PortType::Float, Value::Float(2.0))
                    .with_description("Sigma value for sigma clipping (only used with sigma_clip method)")
                    .with_range(0.5, 5.0)
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Stacked result image")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let images = ctx.get_input("images")?;

        let image_list = match images {
            Value::Array(arr) => arr,
            _ => return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Expected image list".to_string(),
            }),
        };

        if image_list.is_empty() {
            return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Image list is empty".to_string(),
            });
        }

        // Convert to DynamicImages
        let mut dynamic_images: Vec<DynamicImage> = Vec::new();
        for val in image_list {
            if let Value::Image(img_val) = val {
                if let Some(img) = img_val.get_image() {
                    dynamic_images.push(img.clone());
                }
            }
        }

        if dynamic_images.is_empty() {
            return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "No valid images in list".to_string(),
            });
        }

        let method = ctx.get_string("method").unwrap_or("mean");
        let sigma = ctx.get_float("sigma").unwrap_or(2.0);

        // Get dimensions from first image
        let (width, height) = dynamic_images[0].dimensions();
        
        // Convert all to RGBA and check dimensions match
        let rgba_images: Vec<RgbaImage> = dynamic_images.iter()
            .filter_map(|img| {
                if img.dimensions() == (width, height) {
                    Some(img.to_rgba8())
                } else {
                    None
                }
            })
            .collect();

        if rgba_images.len() < 2 {
            return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Need at least 2 images with matching dimensions".to_string(),
            });
        }

        let result = match method {
            "mean" => stack_mean(&rgba_images, width, height),
            "median" => stack_median(&rgba_images, width, height),
            "sigma_clip" => stack_sigma_clip(&rgba_images, width, height, sigma),
            "max" => stack_max(&rgba_images, width, height),
            "min" => stack_min(&rgba_images, width, height),
            _ => stack_mean(&rgba_images, width, height),
        };

        ctx.set_output("image", Value::Image(ImageValue::new(DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

fn stack_mean(images: &[RgbaImage], width: u32, height: u32) -> RgbaImage {
    let mut result = RgbaImage::new(width, height);
    let n = images.len() as f64;

    for y in 0..height {
        for x in 0..width {
            let mut r_sum = 0.0;
            let mut g_sum = 0.0;
            let mut b_sum = 0.0;
            let mut a_sum = 0.0;

            for img in images {
                let pixel = img.get_pixel(x, y);
                r_sum += pixel[0] as f64;
                g_sum += pixel[1] as f64;
                b_sum += pixel[2] as f64;
                a_sum += pixel[3] as f64;
            }

            result.put_pixel(x, y, Rgba([
                (r_sum / n).round() as u8,
                (g_sum / n).round() as u8,
                (b_sum / n).round() as u8,
                (a_sum / n).round() as u8,
            ]));
        }
    }

    result
}

fn stack_median(images: &[RgbaImage], width: u32, height: u32) -> RgbaImage {
    let mut result = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let mut r_vals: Vec<u8> = images.iter().map(|img| img.get_pixel(x, y)[0]).collect();
            let mut g_vals: Vec<u8> = images.iter().map(|img| img.get_pixel(x, y)[1]).collect();
            let mut b_vals: Vec<u8> = images.iter().map(|img| img.get_pixel(x, y)[2]).collect();
            let mut a_vals: Vec<u8> = images.iter().map(|img| img.get_pixel(x, y)[3]).collect();

            r_vals.sort();
            g_vals.sort();
            b_vals.sort();
            a_vals.sort();

            let mid = r_vals.len() / 2;
            result.put_pixel(x, y, Rgba([
                r_vals[mid],
                g_vals[mid],
                b_vals[mid],
                a_vals[mid],
            ]));
        }
    }

    result
}

fn stack_sigma_clip(images: &[RgbaImage], width: u32, height: u32, sigma: f64) -> RgbaImage {
    let mut result = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let r_vals: Vec<f64> = images.iter().map(|img| img.get_pixel(x, y)[0] as f64).collect();
            let g_vals: Vec<f64> = images.iter().map(|img| img.get_pixel(x, y)[1] as f64).collect();
            let b_vals: Vec<f64> = images.iter().map(|img| img.get_pixel(x, y)[2] as f64).collect();
            let a_vals: Vec<f64> = images.iter().map(|img| img.get_pixel(x, y)[3] as f64).collect();

            result.put_pixel(x, y, Rgba([
                sigma_clip_mean(&r_vals, sigma) as u8,
                sigma_clip_mean(&g_vals, sigma) as u8,
                sigma_clip_mean(&b_vals, sigma) as u8,
                sigma_clip_mean(&a_vals, sigma) as u8,
            ]));
        }
    }

    result
}

fn sigma_clip_mean(values: &[f64], sigma: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let std_dev = variance.sqrt();

    let threshold = sigma * std_dev;
    let filtered: Vec<f64> = values.iter()
        .filter(|&&v| (v - mean).abs() <= threshold)
        .copied()
        .collect();

    if filtered.is_empty() {
        mean
    } else {
        filtered.iter().sum::<f64>() / filtered.len() as f64
    }
}

fn stack_max(images: &[RgbaImage], width: u32, height: u32) -> RgbaImage {
    let mut result = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let r_max = images.iter().map(|img| img.get_pixel(x, y)[0]).max().unwrap_or(0);
            let g_max = images.iter().map(|img| img.get_pixel(x, y)[1]).max().unwrap_or(0);
            let b_max = images.iter().map(|img| img.get_pixel(x, y)[2]).max().unwrap_or(0);
            let a_max = images.iter().map(|img| img.get_pixel(x, y)[3]).max().unwrap_or(255);

            result.put_pixel(x, y, Rgba([r_max, g_max, b_max, a_max]));
        }
    }

    result
}

fn stack_min(images: &[RgbaImage], width: u32, height: u32) -> RgbaImage {
    let mut result = RgbaImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let r_min = images.iter().map(|img| img.get_pixel(x, y)[0]).min().unwrap_or(0);
            let g_min = images.iter().map(|img| img.get_pixel(x, y)[1]).min().unwrap_or(0);
            let b_min = images.iter().map(|img| img.get_pixel(x, y)[2]).min().unwrap_or(0);
            let a_min = images.iter().map(|img| img.get_pixel(x, y)[3]).min().unwrap_or(255);

            result.put_pixel(x, y, Rgba([r_min, g_min, b_min, a_min]));
        }
    }

    result
}

/// Dark frame subtraction for noise reduction.
/// 
/// Subtracts a dark frame (image taken with lens cap on) to remove thermal noise.
#[derive(Debug, Clone)]
pub struct DarkFrameSubtract;

impl FilterNode for DarkFrameSubtract {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("dark_frame_subtract", "Dark Frame Subtract")
            .description("Subtract dark frame to remove thermal noise from astrophotos")
            .category(Category::Custom)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Light frame (your photo)")
            )
            .input(
                PortDefinition::input("dark", PortType::Image)
                    .with_description("Dark frame (taken with lens cap on, same exposure)")
            )
            .parameter(
                ParameterDefinition::new("scale", PortType::Float, Value::Float(1.0))
                    .with_description("Scale factor for dark frame subtraction")
                    .with_range(0.0, 2.0)
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Calibrated image")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let light = ctx.get_input_image("image")?;
        let dark = ctx.get_input_image("dark")?;
        let scale = ctx.get_float("scale").unwrap_or(1.0);

        let light_img = light.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Light frame has no data".to_string(),
        })?;

        let dark_img = dark.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Dark frame has no data".to_string(),
        })?;

        let light_rgba = light_img.to_rgba8();
        let dark_rgba = dark_img.to_rgba8();
        let (width, height) = light_rgba.dimensions();

        if dark_rgba.dimensions() != (width, height) {
            return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Dark frame dimensions must match light frame".to_string(),
            });
        }

        let mut result = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let l = light_rgba.get_pixel(x, y);
                let d = dark_rgba.get_pixel(x, y);

                let r = (l[0] as f64 - d[0] as f64 * scale).max(0.0).min(255.0) as u8;
                let g = (l[1] as f64 - d[1] as f64 * scale).max(0.0).min(255.0) as u8;
                let b = (l[2] as f64 - d[2] as f64 * scale).max(0.0).min(255.0) as u8;

                result.put_pixel(x, y, Rgba([r, g, b, l[3]]));
            }
        }

        ctx.set_output("image", Value::Image(ImageValue::new(DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Flat field correction to remove vignetting and dust shadows.
#[derive(Debug, Clone)]
pub struct FlatFieldCorrect;

impl FilterNode for FlatFieldCorrect {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("flat_field_correct", "Flat Field Correct")
            .description("Apply flat field correction to remove vignetting and dust artifacts")
            .category(Category::Custom)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Calibrated light frame")
            )
            .input(
                PortDefinition::input("flat", PortType::Image)
                    .with_description("Flat field (even illumination photo)")
            )
            .parameter(
                ParameterDefinition::new("normalize", PortType::Boolean, Value::Boolean(true))
                    .with_description("Normalize flat field to mean value")
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Corrected image")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let flat = ctx.get_input_image("flat")?;
        let normalize = ctx.get_bool("normalize").unwrap_or(true);

        let img = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let flat_img = flat.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Flat field has no data".to_string(),
        })?;

        let img_rgba = img.to_rgba8();
        let flat_rgba = flat_img.to_rgba8();
        let (width, height) = img_rgba.dimensions();

        if flat_rgba.dimensions() != (width, height) {
            return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Flat field dimensions must match image".to_string(),
            });
        }

        // Calculate flat field mean if normalizing
        let flat_mean = if normalize {
            let sum: f64 = flat_rgba.pixels()
                .map(|p| (p[0] as f64 + p[1] as f64 + p[2] as f64) / 3.0)
                .sum();
            sum / (width * height) as f64
        } else {
            1.0
        };

        let mut result = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let p = img_rgba.get_pixel(x, y);
                let f = flat_rgba.get_pixel(x, y);

                // Avoid division by zero
                let fr = (f[0] as f64 / 255.0).max(0.001);
                let fg = (f[1] as f64 / 255.0).max(0.001);
                let fb = (f[2] as f64 / 255.0).max(0.001);

                let scale = if normalize { flat_mean / 255.0 } else { 1.0 };

                let r = ((p[0] as f64 / fr) * scale).min(255.0).max(0.0) as u8;
                let g = ((p[1] as f64 / fg) * scale).min(255.0).max(0.0) as u8;
                let b = ((p[2] as f64 / fb) * scale).min(255.0).max(0.0) as u8;

                result.put_pixel(x, y, Rgba([r, g, b, p[3]]));
            }
        }

        ctx.set_output("image", Value::Image(ImageValue::new(DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Remove hot pixels (stuck pixels) from an image.
#[derive(Debug, Clone)]
pub struct HotPixelRemoval;

impl FilterNode for HotPixelRemoval {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("hot_pixel_removal", "Hot Pixel Removal")
            .description("Detect and remove hot/dead pixels using median filtering")
            .category(Category::Custom)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Image to clean")
            )
            .parameter(
                ParameterDefinition::new("threshold", PortType::Float, Value::Float(50.0))
                    .with_description("Deviation threshold to detect hot pixels")
                    .with_range(10.0, 200.0)
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Cleaned image")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let threshold = ctx.get_float("threshold").unwrap_or(50.0);

        let img = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut result = rgba.clone();

        // 3x3 neighborhood hot pixel detection
        for y in 1..height-1 {
            for x in 1..width-1 {
                let center = rgba.get_pixel(x, y);
                
                // Get 8-neighborhood
                let mut neighbors_r = Vec::with_capacity(8);
                let mut neighbors_g = Vec::with_capacity(8);
                let mut neighbors_b = Vec::with_capacity(8);

                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 { continue; }
                        let nx = (x as i32 + dx) as u32;
                        let ny = (y as i32 + dy) as u32;
                        let n = rgba.get_pixel(nx, ny);
                        neighbors_r.push(n[0]);
                        neighbors_g.push(n[1]);
                        neighbors_b.push(n[2]);
                    }
                }

                neighbors_r.sort();
                neighbors_g.sort();
                neighbors_b.sort();

                let median_r = neighbors_r[4];
                let median_g = neighbors_g[4];
                let median_b = neighbors_b[4];

                // Check if center pixel deviates significantly
                let dev_r = (center[0] as f64 - median_r as f64).abs();
                let dev_g = (center[1] as f64 - median_g as f64).abs();
                let dev_b = (center[2] as f64 - median_b as f64).abs();

                if dev_r > threshold || dev_g > threshold || dev_b > threshold {
                    result.put_pixel(x, y, Rgba([median_r, median_g, median_b, center[3]]));
                }
            }
        }

        ctx.set_output("image", Value::Image(ImageValue::new(DynamicImage::ImageRgba8(result))))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Histogram stretch to enhance faint details.
#[derive(Debug, Clone)]
pub struct HistogramStretch;

impl FilterNode for HistogramStretch {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("histogram_stretch", "Histogram Stretch")
            .description("Stretch histogram to enhance faint details in astrophotos")
            .category(Category::Custom)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Image to stretch")
            )
            .parameter(
                ParameterDefinition::new("black_point", PortType::Float, Value::Float(0.0))
                    .with_description("Black point (0-1)")
                    .with_range(0.0, 0.5)
            )
            .parameter(
                ParameterDefinition::new("white_point", PortType::Float, Value::Float(1.0))
                    .with_description("White point (0-1)")
                    .with_range(0.5, 1.0)
            )
            .parameter(
                ParameterDefinition::new("midtone", PortType::Float, Value::Float(0.5))
                    .with_description("Midtone balance (0-1, 0.5 is neutral)")
                    .with_range(0.0, 1.0)
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Stretched image")
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let black_point = ctx.get_float("black_point").unwrap_or(0.0);
        let white_point = ctx.get_float("white_point").unwrap_or(1.0);
        let midtone = ctx.get_float("midtone").unwrap_or(0.5);

        let img = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut result = RgbaImage::new(width, height);

        let bp = (black_point * 255.0) as u8;
        let wp = (white_point * 255.0) as u8;
        let range = (wp - bp) as f64;

        // Midtone transfer function (simplified curves adjustment)
        let gamma = if midtone > 0.001 {
            (0.5f64).ln() / (midtone).ln()
        } else {
            1.0
        };

        for y in 0..height {
            for x in 0..width {
                let p = rgba.get_pixel(x, y);
                
                let stretch = |v: u8| -> u8 {
                    let clamped = v.max(bp).min(wp);
                    let normalized = (clamped - bp) as f64 / range;
                    let adjusted = normalized.powf(gamma);
                    (adjusted * 255.0).min(255.0).max(0.0) as u8
                };

                result.put_pixel(x, y, Rgba([
                    stretch(p[0]),
                    stretch(p[1]),
                    stretch(p[2]),
                    p[3],
                ]));
            }
        }

        ctx.set_output("image", Value::Image(ImageValue::new(DynamicImage::ImageRgba8(result))))?;
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
    fn test_image_stack_metadata() {
        let filter = ImageStack;
        let metadata = filter.metadata();
        
        assert_eq!(metadata.id, "image_stack");
        assert_eq!(metadata.inputs.len(), 1);
        assert_eq!(metadata.outputs.len(), 1);
    }

    #[test]
    fn test_dark_subtract_metadata() {
        let filter = DarkFrameSubtract;
        let metadata = filter.metadata();
        
        assert_eq!(metadata.id, "dark_frame_subtract");
        assert_eq!(metadata.inputs.len(), 2);
    }

    #[test]
    fn test_histogram_stretch_metadata() {
        let filter = HistogramStretch;
        let metadata = filter.metadata();
        
        assert_eq!(metadata.id, "histogram_stretch");
        assert!(metadata.parameters.len() >= 3);
    }
}
