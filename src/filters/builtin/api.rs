//! API and external service integration filters.
//!
//! These filters enable Ambara pipelines to fetch images from URLs,
//! call image generation APIs (Stable Diffusion, etc.), classify images
//! via external services, and run generic model inference.

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use std::io::Cursor;
use std::io::Read;

/// Register API filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(HttpImageFetch));
    registry.register(|| Box::new(StableDiffusionGenerate));
    registry.register(|| Box::new(ImageClassify));
    registry.register(|| Box::new(ModelInference));
    registry.register(|| Box::new(StyleTransfer));
}

// ============================================================================
// HttpImageFetch — fetch an image from a URL
// ============================================================================

/// Fetch an image from an HTTP(S) URL.
#[derive(Debug, Clone)]
pub struct HttpImageFetch;

impl FilterNode for HttpImageFetch {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("http_image_fetch", "HTTP Image Fetch")
            .description("Fetch an image from an HTTP or HTTPS URL")
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("The fetched image"),
            )
            .parameter(
                ParameterDefinition::new("url", PortType::String, Value::String(String::new()))
                    .with_description("URL of the image to fetch (http:// or https://)")
                    .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "timeout_secs",
                    PortType::Integer,
                    Value::Integer(30),
                )
                .with_description("Request timeout in seconds")
                .with_constraint(Constraint::Range { min: 1.0, max: 120.0 }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let url = ctx.get_string("url").unwrap_or("");
        if url.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "url".to_string(),
                error: "URL cannot be empty".to_string(),
            });
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "url".to_string(),
                error: "URL must start with http:// or https://".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let url = ctx.get_string("url")?;
        let timeout = ctx.get_integer("timeout_secs").unwrap_or(30) as u64;

        let resp = ureq::agent()
            .get(url)
            .timeout(std::time::Duration::from_secs(timeout))
            .call()
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("HTTP request failed: {}", e),
            })?;

        let content_type = resp
            .header("Content-Type")
            .unwrap_or("")
            .to_lowercase();

        if !content_type.contains("image") {
            return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Response is not an image (Content-Type: {})", content_type),
            });
        }

        let mut bytes = Vec::new();
        resp.into_reader()
            .take(50 * 1024 * 1024) // 50 MB limit
            .read_to_end(&mut bytes)
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to read response body: {}", e),
            })?;

        let img = image::load_from_memory(&bytes).map_err(|e| {
            ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to decode image: {}", e),
            }
        })?;

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(img.to_rgba8()))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// StableDiffusionGenerate — text-to-image via Stable Diffusion API
// ============================================================================

/// Generate an image using a Stable Diffusion API endpoint.
///
/// Supports any API that follows the Automatic1111/ComfyUI-style text-to-image
/// interface (POST with prompt, returns base64-encoded image).
#[derive(Debug, Clone)]
pub struct StableDiffusionGenerate;

impl FilterNode for StableDiffusionGenerate {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("stable_diffusion_generate", "Stable Diffusion Generate")
            .description("Generate an image from text using a Stable Diffusion API (local or remote)")
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("The generated image"),
            )
            .parameter(
                ParameterDefinition::new(
                    "api_url",
                    PortType::String,
                    Value::String("http://127.0.0.1:7860/sdapi/v1/txt2img".to_string()),
                )
                .with_description("Stable Diffusion API endpoint URL")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "prompt",
                    PortType::String,
                    Value::String("a beautiful landscape, high quality, 4k".to_string()),
                )
                .with_description("Text prompt for image generation")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "negative_prompt",
                    PortType::String,
                    Value::String("low quality, blurry, distorted".to_string()),
                )
                .with_description("Negative prompt to avoid undesired features"),
            )
            .parameter(
                ParameterDefinition::new("width", PortType::Integer, Value::Integer(512))
                    .with_description("Output image width in pixels")
                    .with_constraint(Constraint::Range {
                        min: 64.0,
                        max: 2048.0,
                    })
                    .with_ui_hint(UiHint::Slider {
                        logarithmic: false,
                    }),
            )
            .parameter(
                ParameterDefinition::new("height", PortType::Integer, Value::Integer(512))
                    .with_description("Output image height in pixels")
                    .with_constraint(Constraint::Range {
                        min: 64.0,
                        max: 2048.0,
                    })
                    .with_ui_hint(UiHint::Slider {
                        logarithmic: false,
                    }),
            )
            .parameter(
                ParameterDefinition::new("steps", PortType::Integer, Value::Integer(20))
                    .with_description("Number of denoising steps")
                    .with_constraint(Constraint::Range {
                        min: 1.0,
                        max: 150.0,
                    })
                    .with_ui_hint(UiHint::Slider {
                        logarithmic: false,
                    }),
            )
            .parameter(
                ParameterDefinition::new("cfg_scale", PortType::Float, Value::Float(7.0))
                    .with_description("Classifier-free guidance scale (how closely to follow the prompt)")
                    .with_constraint(Constraint::Range {
                        min: 1.0,
                        max: 30.0,
                    })
                    .with_ui_hint(UiHint::Slider {
                        logarithmic: false,
                    }),
            )
            .parameter(
                ParameterDefinition::new("seed", PortType::Integer, Value::Integer(-1))
                    .with_description("Random seed (-1 for random)")
                    .with_constraint(Constraint::Range {
                        min: -1.0,
                        max: 2147483647.0,
                    }),
            )
            .parameter(
                ParameterDefinition::new(
                    "timeout_secs",
                    PortType::Integer,
                    Value::Integer(120),
                )
                .with_description("API request timeout in seconds")
                .with_constraint(Constraint::Range { min: 10.0, max: 600.0 }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let api_url = ctx.get_string("api_url").unwrap_or("");
        if api_url.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "api_url".to_string(),
                error: "API URL cannot be empty".to_string(),
            });
        }
        let prompt = ctx.get_string("prompt").unwrap_or("");
        if prompt.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "prompt".to_string(),
                error: "Prompt cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let api_url = ctx.get_string("api_url")?;
        let prompt = ctx.get_string("prompt").unwrap_or("");
        let negative_prompt = ctx.get_string("negative_prompt").unwrap_or("");
        let width = ctx.get_integer("width").unwrap_or(512);
        let height = ctx.get_integer("height").unwrap_or(512);
        let steps = ctx.get_integer("steps").unwrap_or(20);
        let cfg_scale = ctx.get_float("cfg_scale").unwrap_or(7.0);
        let seed = ctx.get_integer("seed").unwrap_or(-1);
        let timeout = ctx.get_integer("timeout_secs").unwrap_or(120) as u64;

        let body = serde_json::json!({
            "prompt": prompt,
            "negative_prompt": negative_prompt,
            "width": width,
            "height": height,
            "steps": steps,
            "cfg_scale": cfg_scale,
            "seed": seed,
        });

        let resp = ureq::agent()
            .post(api_url)
            .timeout(std::time::Duration::from_secs(timeout))
            .send_json(&body)
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Stable Diffusion API request failed: {}", e),
            })?;

        let resp_json: serde_json::Value =
            resp.into_json().map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to parse API response: {}", e),
            })?;

        // Extract first base64 image from the response
        let images = resp_json
            .get("images")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "API response missing 'images' array".to_string(),
            })?;

        let b64 = images
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "No images in API response".to_string(),
            })?;

        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to decode base64 image: {}", e),
            })?;

        let img = image::load_from_memory(&decoded).map_err(|e| {
            ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to decode generated image: {}", e),
            }
        })?;

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(img.to_rgba8()))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// ImageClassify — classify an image via an external API
// ============================================================================

/// Classify an image by sending it to an external classification API.
///
/// Accepts any API that takes a base64-encoded image and returns JSON
/// with classification results.
#[derive(Debug, Clone)]
pub struct ImageClassify;

impl FilterNode for ImageClassify {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("image_classify", "Image Classify")
            .description("Classify an image using an external classification API endpoint")
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Image to classify"),
            )
            .output(
                PortDefinition::output("result", PortType::String)
                    .with_description("Classification result as JSON string"),
            )
            .output(
                PortDefinition::output("top_label", PortType::String)
                    .with_description("Top predicted label"),
            )
            .output(
                PortDefinition::output("confidence", PortType::Float)
                    .with_description("Confidence score of top prediction (0.0-1.0)"),
            )
            .parameter(
                ParameterDefinition::new(
                    "api_url",
                    PortType::String,
                    Value::String("http://127.0.0.1:5000/classify".to_string()),
                )
                .with_description("Classification API endpoint URL")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "timeout_secs",
                    PortType::Integer,
                    Value::Integer(30),
                )
                .with_description("API request timeout in seconds")
                .with_constraint(Constraint::Range { min: 5.0, max: 120.0 }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let api_url = ctx.get_string("api_url").unwrap_or("");
        if api_url.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "api_url".to_string(),
                error: "API URL cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let api_url = ctx.get_string("api_url")?;
        let timeout = ctx.get_integer("timeout_secs").unwrap_or(30) as u64;

        // Encode image as PNG base64
        let rgba = image
            .get_image()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Input image data is not loaded".to_string(),
            })?
            .to_rgba8();
        let mut png_bytes = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(Cursor::new(&mut png_bytes));
            use image::ImageEncoder;
            encoder
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: format!("Failed to encode image as PNG: {}", e),
                })?;
        }

        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        let body = serde_json::json!({
            "image": b64,
        });

        let resp = ureq::agent()
            .post(api_url)
            .timeout(std::time::Duration::from_secs(timeout))
            .send_json(&body)
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Classification API request failed: {}", e),
            })?;

        let resp_json: serde_json::Value =
            resp.into_json().map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to parse classification response: {}", e),
            })?;

        let result_str = serde_json::to_string_pretty(&resp_json).unwrap_or_default();

        // Try to extract top label and confidence from common response formats
        let top_label = resp_json
            .get("label")
            .or_else(|| resp_json.get("class"))
            .or_else(|| resp_json.get("prediction"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let confidence = resp_json
            .get("confidence")
            .or_else(|| resp_json.get("score"))
            .or_else(|| resp_json.get("probability"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        ctx.set_output("result", Value::String(result_str))?;
        ctx.set_output("top_label", Value::String(top_label))?;
        ctx.set_output("confidence", Value::Float(confidence))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// ModelInference — generic model inference via API
// ============================================================================

/// Run generic model inference by sending an image to any REST API
/// and receiving a processed image back.
#[derive(Debug, Clone)]
pub struct ModelInference;

impl FilterNode for ModelInference {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("model_inference", "Model Inference")
            .description("Send an image to a model inference API and receive a processed image back")
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image to process"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Processed output image"),
            )
            .output(
                PortDefinition::output("raw_response", PortType::String)
                    .with_description("Raw API response as string (for debugging)"),
            )
            .parameter(
                ParameterDefinition::new(
                    "api_url",
                    PortType::String,
                    Value::String("http://127.0.0.1:5000/predict".to_string()),
                )
                .with_description("Model inference API endpoint URL")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "model_name",
                    PortType::String,
                    Value::String(String::new()),
                )
                .with_description("Optional model name to include in the request"),
            )
            .parameter(
                ParameterDefinition::new(
                    "extra_params",
                    PortType::String,
                    Value::String(String::new()),
                )
                .with_description("Extra parameters as JSON string (merged into request body)"),
            )
            .parameter(
                ParameterDefinition::new(
                    "timeout_secs",
                    PortType::Integer,
                    Value::Integer(60),
                )
                .with_description("API request timeout in seconds")
                .with_constraint(Constraint::Range { min: 5.0, max: 600.0 }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let api_url = ctx.get_string("api_url").unwrap_or("");
        if api_url.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "api_url".to_string(),
                error: "API URL cannot be empty".to_string(),
            });
        }

        // Validate extra_params is valid JSON if provided
        let extra = ctx.get_string("extra_params").unwrap_or("");
        if !extra.is_empty() {
            serde_json::from_str::<serde_json::Value>(extra).map_err(|e| {
                ValidationError::ConstraintViolation {
                    node_id: ctx.node_id,
                    parameter: "extra_params".to_string(),
                    error: format!("Invalid JSON: {}", e),
                }
            })?;
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let image = ctx.get_input_image("image")?;
        let api_url = ctx.get_string("api_url")?;
        let model_name = ctx.get_string("model_name").unwrap_or("");
        let extra_params_str = ctx.get_string("extra_params").unwrap_or("");
        let timeout = ctx.get_integer("timeout_secs").unwrap_or(60) as u64;

        // Encode image as PNG base64
        let rgba = image
            .get_image()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Input image data is not loaded".to_string(),
            })?
            .to_rgba8();
        let mut png_bytes = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(Cursor::new(&mut png_bytes));
            use image::ImageEncoder;
            encoder
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: format!("Failed to encode image: {}", e),
                })?;
        }

        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        let mut body = serde_json::json!({
            "image": b64,
        });

        if !model_name.is_empty() {
            body["model"] = serde_json::Value::String(model_name.to_string());
        }

        // Merge extra parameters if provided
        if !extra_params_str.is_empty() {
            if let Ok(extra) = serde_json::from_str::<serde_json::Value>(extra_params_str) {
                if let (Some(base), Some(ext)) = (body.as_object_mut(), extra.as_object()) {
                    for (k, v) in ext {
                        base.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        let resp = ureq::agent()
            .post(api_url)
            .timeout(std::time::Duration::from_secs(timeout))
            .send_json(&body)
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Model inference API request failed: {}", e),
            })?;

        let resp_json: serde_json::Value =
            resp.into_json().map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to parse inference response: {}", e),
            })?;

        let raw_str = serde_json::to_string_pretty(&resp_json).unwrap_or_default();
        ctx.set_output("raw_response", Value::String(raw_str))?;

        // Try to extract image from response
        let img_b64 = resp_json
            .get("image")
            .or_else(|| resp_json.get("output"))
            .or_else(|| {
                resp_json
                    .get("images")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
            })
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "API response does not contain an image field".to_string(),
            })?;

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(img_b64)
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to decode base64 image from response: {}", e),
            })?;

        let out_img = image::load_from_memory(&decoded).map_err(|e| {
            ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to decode output image: {}", e),
            }
        })?;

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(
                out_img.to_rgba8(),
            ))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// StyleTransfer — apply artistic style transfer via API
// ============================================================================

/// Apply neural style transfer by sending a content image and style reference
/// to an external style transfer API.
#[derive(Debug, Clone)]
pub struct StyleTransfer;

impl FilterNode for StyleTransfer {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("style_transfer", "Style Transfer")
            .description("Apply neural style transfer using an external API (content + style images)")
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .input(
                PortDefinition::input("content", PortType::Image)
                    .with_description("Content image to stylize"),
            )
            .input(
                PortDefinition::input("style", PortType::Image)
                    .with_description("Style reference image"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Stylized output image"),
            )
            .parameter(
                ParameterDefinition::new(
                    "api_url",
                    PortType::String,
                    Value::String("http://127.0.0.1:5000/style-transfer".to_string()),
                )
                .with_description("Style transfer API endpoint URL")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new("strength", PortType::Float, Value::Float(0.8))
                    .with_description("Style strength (0.0 = no style, 1.0 = full style)")
                    .with_constraint(Constraint::Range { min: 0.0, max: 1.0 })
                    .with_ui_hint(UiHint::Slider {
                        logarithmic: false,
                    }),
            )
            .parameter(
                ParameterDefinition::new(
                    "timeout_secs",
                    PortType::Integer,
                    Value::Integer(120),
                )
                .with_description("API request timeout in seconds")
                .with_constraint(Constraint::Range { min: 10.0, max: 600.0 }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let api_url = ctx.get_string("api_url").unwrap_or("");
        if api_url.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "api_url".to_string(),
                error: "API URL cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let content = ctx.get_input_image("content")?;
        let style = ctx.get_input_image("style")?;
        let api_url = ctx.get_string("api_url")?;
        let strength = ctx.get_float("strength").unwrap_or(0.8);
        let timeout = ctx.get_integer("timeout_secs").unwrap_or(120) as u64;

        // Encode both images
        use base64::Engine;
        let encode_image = |img: &ImageValue| -> Result<String, ExecutionError> {
            let rgba = img
                .get_image()
                .ok_or_else(|| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: "Input image data is not loaded".to_string(),
                })?
                .to_rgba8();
            let mut png_bytes = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(Cursor::new(&mut png_bytes));
            use image::ImageEncoder;
            encoder
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| ExecutionError::NodeExecution {
                    node_id: ctx.node_id,
                    error: format!("Failed to encode image: {}", e),
                })?;
            Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
        };

        let content_b64 = encode_image(content)?;
        let style_b64 = encode_image(style)?;

        let body = serde_json::json!({
            "content_image": content_b64,
            "style_image": style_b64,
            "strength": strength,
        });

        let resp = ureq::agent()
            .post(api_url)
            .timeout(std::time::Duration::from_secs(timeout))
            .send_json(&body)
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Style transfer API request failed: {}", e),
            })?;

        let resp_json: serde_json::Value =
            resp.into_json().map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to parse style transfer response: {}", e),
            })?;

        let img_b64 = resp_json
            .get("image")
            .or_else(|| resp_json.get("output"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Style transfer response missing image field".to_string(),
            })?;

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(img_b64)
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to decode base64 image: {}", e),
            })?;

        let out_img = image::load_from_memory(&decoded).map_err(|e| {
            ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to decode stylized image: {}", e),
            }
        })?;

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(
                out_img.to_rgba8(),
            ))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

