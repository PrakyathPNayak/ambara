//! Noise filters: AddNoise, Denoise (median filter)

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use image::Pixel;

/// Register noise filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(AddNoise));
    registry.register(|| Box::new(Denoise));
}

/// Add random noise to an image (Gaussian or salt-and-pepper).
#[derive(Debug, Clone)]
pub struct AddNoise;

impl FilterNode for AddNoise {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("add_noise", "Add Noise")
            .description("Add random noise to an image (Gaussian or salt-and-pepper)")
            .category(Category::Noise)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Noisy image"),
            )
            .parameter(
                ParameterDefinition::new("noise_type", PortType::String, Value::String("gaussian".to_string()))
                    .with_description("Type of noise to add")
                    .with_ui_hint(UiHint::Dropdown {
                        options: vec!["gaussian".to_string(), "salt_pepper".to_string()],
                    }),
            )
            .parameter(
                ParameterDefinition::new("amount", PortType::Float, Value::Float(0.1))
                    .with_description("Noise intensity (0.0 to 1.0)")
                    .with_ui_hint(UiHint::Slider { logarithmic: false })
                    .with_constraint(Constraint::Range { min: 0.0, max: 1.0 }),
            )
            .parameter(
                ParameterDefinition::new("seed", PortType::Integer, Value::Integer(42))
                    .with_description("Random seed for reproducibility")
                    .with_ui_hint(UiHint::SpinBox),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let noise_type = ctx.get_string("noise_type").unwrap_or("gaussian");
        let amount = ctx.get_float("amount").unwrap_or(0.1) as f32;
        let seed = ctx.get_integer("seed").unwrap_or(42) as u64;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let mut rgba = img_data.to_rgba8();

        // Simple LCG pseudo-random number generator (deterministic, no external deps)
        let mut rng_state = seed;
        let mut next_rand = move || -> f32 {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((rng_state >> 33) as f32) / (u32::MAX as f32)
        };

        match noise_type.as_ref() {
            "salt_pepper" => {
                for pixel in rgba.pixels_mut() {
                    let r = next_rand();
                    if r < amount * 0.5 {
                        let ch = pixel.channels_mut();
                        ch[0] = 0;
                        ch[1] = 0;
                        ch[2] = 0;
                    } else if r < amount {
                        let ch = pixel.channels_mut();
                        ch[0] = 255;
                        ch[1] = 255;
                        ch[2] = 255;
                    }
                }
            }
            _ => {
                // Gaussian-like noise via Box-Muller approximation
                let intensity = amount * 128.0;
                for pixel in rgba.pixels_mut() {
                    let ch = pixel.channels_mut();
                    for i in 0..3 {
                        let u1 = next_rand().max(1e-10);
                        let u2 = next_rand();
                        let gauss = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
                        let noise = gauss * intensity;
                        ch[i] = (ch[i] as f32 + noise).clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(rgba))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

/// Denoise an image using a median filter.
#[derive(Debug, Clone)]
pub struct Denoise;

impl FilterNode for Denoise {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("denoise", "Denoise")
            .description("Reduce noise using a median filter")
            .category(Category::Noise)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Noisy input image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Denoised image"),
            )
            .parameter(
                ParameterDefinition::new("radius", PortType::Integer, Value::Integer(1))
                    .with_description("Filter radius (1 = 3×3 kernel, 2 = 5×5, etc.)")
                    .with_ui_hint(UiHint::SpinBox)
                    .with_constraint(Constraint::Range { min: 1.0, max: 5.0 }),
            )
            .build()
    }

    fn validate(&self, _ctx: &ValidationContext) -> Result<(), ValidationError> {
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let radius = ctx.get_integer("radius").unwrap_or(1).max(1) as u32;

        let img_data = image.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: ctx.node_id,
            error: "Image has no data".to_string(),
        })?;

        let rgba = img_data.to_rgba8();
        let (w, h) = rgba.dimensions();
        let mut result = image::RgbaImage::new(w, h);

        let r = radius as i32;
        for y in 0..h {
            for x in 0..w {
                let mut channels: [Vec<u8>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                for ky in -r..=r {
                    for kx in -r..=r {
                        let nx = (x as i32 + kx).clamp(0, w as i32 - 1) as u32;
                        let ny = (y as i32 + ky).clamp(0, h as i32 - 1) as u32;
                        let px = rgba.get_pixel(nx, ny).channels();
                        for c in 0..4 {
                            channels[c].push(px[c]);
                        }
                    }
                }
                let mut out = [0u8; 4];
                for c in 0..4 {
                    channels[c].sort_unstable();
                    out[c] = channels[c][channels[c].len() / 2];
                }
                result.put_pixel(x, y, image::Rgba(out));
            }
        }

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(result))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}
