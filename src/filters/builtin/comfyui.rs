//! ComfyUI workflow nodes for Ambara.
//!
//! These filters mirror the core ComfyUI nodes (checkpoint loading, CLIP text
//! encoding, KSampler, VAE decode, LoRA loading, upscaling) by sending
//! structured prompts to a running ComfyUI server via its REST API.
//!
//! All nodes communicate with ComfyUI at a configurable base URL (default
//! `http://127.0.0.1:8188`) and use the `/prompt` and `/history` endpoints.

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, ValidationError};
use crate::core::node::{Category, FilterNode, NodeMetadata};
use crate::core::port::{Constraint, ParameterDefinition, PortDefinition, UiHint};
use crate::core::types::{ImageValue, PortType, Value};
use crate::filters::registry::FilterRegistry;
use std::io::Cursor;
use std::io::Read;

/// Register all ComfyUI workflow filters.
pub fn register(registry: &mut FilterRegistry) {
    registry.register(|| Box::new(ComfyCheckpointLoader));
    registry.register(|| Box::new(ComfyClipTextEncode));
    registry.register(|| Box::new(ComfyKSampler));
    registry.register(|| Box::new(ComfyVaeDecode));
    registry.register(|| Box::new(ComfyLoraLoader));
    registry.register(|| Box::new(ComfyImageUpscale));
    registry.register(|| Box::new(ComfyControlNetApply));
    registry.register(|| Box::new(ComfyWorkflowRunner));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Poll ComfyUI `/history/{prompt_id}` until the prompt is done.
fn poll_comfyui_result(
    base_url: &str,
    prompt_id: &str,
    timeout_secs: u64,
    node_id: crate::core::error::NodeId,
) -> Result<serde_json::Value, ExecutionError> {
    let url = format!("{}/history/{}", base_url, prompt_id);
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    loop {
        if std::time::Instant::now() > deadline {
            return Err(ExecutionError::NodeExecution {
                node_id,
                error: format!(
                    "ComfyUI did not finish within {} seconds",
                    timeout_secs
                ),
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(500));

        let resp = ureq::agent()
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .call()
            .map_err(|e| ExecutionError::NodeExecution {
                node_id,
                error: format!("ComfyUI history poll failed: {}", e),
            })?;

        let history: serde_json::Value =
            resp.into_json().map_err(|e| ExecutionError::NodeExecution {
                node_id,
                error: format!("Failed to parse history response: {}", e),
            })?;

        if let Some(entry) = history.get(prompt_id) {
            return Ok(entry.clone());
        }
    }
}

/// Submit a ComfyUI workflow prompt and return the prompt_id.
fn submit_prompt(
    base_url: &str,
    workflow: &serde_json::Value,
    node_id: crate::core::error::NodeId,
) -> Result<String, ExecutionError> {
    let body = serde_json::json!({ "prompt": workflow });
    let resp = ureq::agent()
        .post(&format!("{}/prompt", base_url))
        .timeout(std::time::Duration::from_secs(30))
        .send_json(&body)
        .map_err(|e| ExecutionError::NodeExecution {
            node_id,
            error: format!("ComfyUI prompt submission failed: {}", e),
        })?;

    let resp_json: serde_json::Value =
        resp.into_json().map_err(|e| ExecutionError::NodeExecution {
            node_id,
            error: format!("Failed to parse prompt response: {}", e),
        })?;

    resp_json
        .get("prompt_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ExecutionError::NodeExecution {
            node_id,
            error: "ComfyUI response missing prompt_id".to_string(),
        })
}

/// Download an image from ComfyUI's `/view` endpoint.
fn download_comfyui_image(
    base_url: &str,
    filename: &str,
    subfolder: &str,
    folder_type: &str,
    node_id: crate::core::error::NodeId,
) -> Result<image::DynamicImage, ExecutionError> {
    let url = format!(
        "{}/view?filename={}&subfolder={}&type={}",
        base_url, filename, subfolder, folder_type,
    );
    let resp = ureq::agent()
        .get(&url)
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| ExecutionError::NodeExecution {
            node_id,
            error: format!("ComfyUI image download failed: {}", e),
        })?;

    let mut bytes = Vec::new();
    resp.into_reader()
        .take(100 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| ExecutionError::NodeExecution {
            node_id,
            error: format!("Failed to read image bytes: {}", e),
        })?;

    image::load_from_memory(&bytes).map_err(|e| ExecutionError::NodeExecution {
        node_id,
        error: format!("Failed to decode ComfyUI image: {}", e),
    })
}

/// Extract the first output image filename from a ComfyUI history entry.
fn first_output_image(
    history: &serde_json::Value,
) -> Option<(String, String, String)> {
    let outputs = history.get("outputs")?;
    for (_node_id, node_output) in outputs.as_object()? {
        if let Some(images) = node_output.get("images").and_then(|v| v.as_array()) {
            if let Some(img) = images.first() {
                let filename = img.get("filename")?.as_str()?.to_string();
                let subfolder = img
                    .get("subfolder")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let folder_type = img
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("output")
                    .to_string();
                return Some((filename, subfolder, folder_type));
            }
        }
    }
    None
}

fn validate_comfyui_url(ctx: &ValidationContext, param: &str) -> Result<(), ValidationError> {
    let url = ctx.get_string(param).unwrap_or("");
    if url.is_empty() {
        return Err(ValidationError::ConstraintViolation {
            node_id: ctx.node_id,
            parameter: param.to_string(),
            error: "ComfyUI server URL cannot be empty".to_string(),
        });
    }
    Ok(())
}

// ============================================================================
// ComfyCheckpointLoader — load a Stable Diffusion checkpoint
// ============================================================================

/// Load a Stable Diffusion checkpoint model through ComfyUI.
///
/// This node sends a workflow to ComfyUI that loads a checkpoint and outputs
/// the model name reference for downstream nodes to use.
#[derive(Debug, Clone)]
pub struct ComfyCheckpointLoader;

impl FilterNode for ComfyCheckpointLoader {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("comfy_checkpoint_loader", "ComfyUI Checkpoint Loader")
            .description(
                "Load a Stable Diffusion checkpoint through ComfyUI. \
                 Outputs model/CLIP/VAE references for downstream ComfyUI nodes.",
            )
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .tag("comfyui")
            .tag("model")
            .tag("checkpoint")
            .output(
                PortDefinition::output("model_ref", PortType::String)
                    .with_description("Model reference name for downstream nodes"),
            )
            .output(
                PortDefinition::output("clip_ref", PortType::String)
                    .with_description("CLIP model reference"),
            )
            .output(
                PortDefinition::output("vae_ref", PortType::String)
                    .with_description("VAE model reference"),
            )
            .parameter(
                ParameterDefinition::new(
                    "comfyui_url",
                    PortType::String,
                    Value::String("http://127.0.0.1:8188".to_string()),
                )
                .with_description("ComfyUI server URL")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "checkpoint_name",
                    PortType::String,
                    Value::String("v1-5-pruned-emaonly.safetensors".to_string()),
                )
                .with_description(
                    "Checkpoint filename as it appears in ComfyUI's models/checkpoints folder",
                )
                .with_constraint(Constraint::NotEmpty),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        validate_comfyui_url(ctx, "comfyui_url")?;
        let ckpt = ctx.get_string("checkpoint_name").unwrap_or("");
        if ckpt.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "checkpoint_name".to_string(),
                error: "Checkpoint name cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let comfyui_url = ctx.get_string("comfyui_url")?;
        let checkpoint_name = ctx.get_string("checkpoint_name")?;

        // Verify ComfyUI is reachable
        ureq::agent()
            .get(&format!("{}/system_stats", comfyui_url))
            .timeout(std::time::Duration::from_secs(5))
            .call()
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("ComfyUI not reachable at {}: {}", comfyui_url, e),
            })?;

        // Produce reference strings that downstream nodes can use
        let model_ref = format!("ckpt:{}", checkpoint_name);
        let clip_ref = format!("clip:{}", checkpoint_name);
        let vae_ref = format!("vae:{}", checkpoint_name);

        ctx.set_output("model_ref", Value::String(model_ref))?;
        ctx.set_output("clip_ref", Value::String(clip_ref))?;
        ctx.set_output("vae_ref", Value::String(vae_ref))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// ComfyClipTextEncode — CLIP text encoding
// ============================================================================

/// Encode text prompts via CLIP through ComfyUI.
#[derive(Debug, Clone)]
pub struct ComfyClipTextEncode;

impl FilterNode for ComfyClipTextEncode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("comfy_clip_text_encode", "ComfyUI CLIP Text Encode")
            .description(
                "Encode a text prompt into CLIP conditioning using ComfyUI. \
                 Connect the clip_ref from a Checkpoint Loader.",
            )
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .tag("comfyui")
            .tag("clip")
            .tag("prompt")
            .input(
                PortDefinition::input("clip_ref", PortType::String)
                    .with_description("CLIP model reference from Checkpoint Loader"),
            )
            .output(
                PortDefinition::output("conditioning", PortType::String)
                    .with_description("Encoded conditioning reference"),
            )
            .parameter(
                ParameterDefinition::new(
                    "text",
                    PortType::String,
                    Value::String("a beautiful landscape, high quality".to_string()),
                )
                .with_description("Text prompt to encode")
                .with_constraint(Constraint::NotEmpty),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let text = ctx.get_string("text").unwrap_or("");
        if text.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "text".to_string(),
                error: "Text prompt cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let clip_ref = ctx
            .get_input("clip_ref")
            .ok()
            .and_then(|v| {
                if let Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let text = ctx.get_string("text")?;

        // Produce a conditioning reference that KSampler can consume
        let cond_ref = format!("cond:{}:{}", clip_ref, text);
        ctx.set_output("conditioning", Value::String(cond_ref))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// ComfyKSampler — denoise/sample latent image
// ============================================================================

/// KSampler node — the core sampling/denoising step in a Stable Diffusion
/// workflow, orchestrated through ComfyUI.
#[derive(Debug, Clone)]
pub struct ComfyKSampler;

impl FilterNode for ComfyKSampler {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("comfy_ksampler", "ComfyUI KSampler")
            .description(
                "Run the KSampler (denoising) step through ComfyUI with full control \
                 over sampler, scheduler, steps, CFG scale, and seed.",
            )
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .tag("comfyui")
            .tag("sampler")
            .tag("generation")
            .input(
                PortDefinition::input("model_ref", PortType::String)
                    .with_description("Model reference from Checkpoint Loader"),
            )
            .input(
                PortDefinition::input("positive", PortType::String)
                    .with_description("Positive conditioning from CLIP Text Encode"),
            )
            .input(
                PortDefinition::input("negative", PortType::String)
                    .with_description("Negative conditioning from CLIP Text Encode")
                    .optional(),
            )
            .output(
                PortDefinition::output("latent_ref", PortType::String)
                    .with_description("Latent image reference for VAE Decode"),
            )
            .parameter(
                ParameterDefinition::new(
                    "comfyui_url",
                    PortType::String,
                    Value::String("http://127.0.0.1:8188".to_string()),
                )
                .with_description("ComfyUI server URL")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new("seed", PortType::Integer, Value::Integer(0))
                    .with_description("Random seed (0 = random)")
                    .with_constraint(Constraint::Range {
                        min: 0.0,
                        max: 2147483647.0,
                    }),
            )
            .parameter(
                ParameterDefinition::new("steps", PortType::Integer, Value::Integer(20))
                    .with_description("Number of sampling steps")
                    .with_constraint(Constraint::Range {
                        min: 1.0,
                        max: 150.0,
                    })
                    .with_ui_hint(UiHint::Slider { logarithmic: false }),
            )
            .parameter(
                ParameterDefinition::new("cfg_scale", PortType::Float, Value::Float(7.0))
                    .with_description("Classifier-free guidance scale")
                    .with_constraint(Constraint::Range {
                        min: 1.0,
                        max: 30.0,
                    })
                    .with_ui_hint(UiHint::Slider { logarithmic: false }),
            )
            .parameter(
                ParameterDefinition::new(
                    "sampler_name",
                    PortType::String,
                    Value::String("euler".to_string()),
                )
                .with_description("Sampler algorithm (euler, euler_ancestral, dpmpp_2m, etc.)")
                .with_constraint(Constraint::OneOf(vec![
                    Value::String("euler".to_string()),
                    Value::String("euler_ancestral".to_string()),
                    Value::String("heun".to_string()),
                    Value::String("dpm_2".to_string()),
                    Value::String("dpm_2_ancestral".to_string()),
                    Value::String("lms".to_string()),
                    Value::String("dpmpp_2m".to_string()),
                    Value::String("dpmpp_2m_sde".to_string()),
                    Value::String("dpmpp_sde".to_string()),
                    Value::String("ddim".to_string()),
                    Value::String("uni_pc".to_string()),
                ]))
                .with_ui_hint(UiHint::Dropdown {
                    options: vec![
                        "euler".to_string(), "euler_ancestral".to_string(),
                        "heun".to_string(), "dpm_2".to_string(),
                        "dpm_2_ancestral".to_string(), "lms".to_string(),
                        "dpmpp_2m".to_string(), "dpmpp_2m_sde".to_string(),
                        "dpmpp_sde".to_string(), "ddim".to_string(),
                        "uni_pc".to_string(),
                    ],
                }),
            )
            .parameter(
                ParameterDefinition::new(
                    "scheduler",
                    PortType::String,
                    Value::String("normal".to_string()),
                )
                .with_description("Noise scheduler")
                .with_constraint(Constraint::OneOf(vec![
                    Value::String("normal".to_string()),
                    Value::String("karras".to_string()),
                    Value::String("exponential".to_string()),
                    Value::String("sgm_uniform".to_string()),
                    Value::String("simple".to_string()),
                    Value::String("ddim_uniform".to_string()),
                ]))
                .with_ui_hint(UiHint::Dropdown {
                    options: vec![
                        "normal".to_string(), "karras".to_string(),
                        "exponential".to_string(), "sgm_uniform".to_string(),
                        "simple".to_string(), "ddim_uniform".to_string(),
                    ],
                }),
            )
            .parameter(
                ParameterDefinition::new("denoise", PortType::Float, Value::Float(1.0))
                    .with_description("Denoise strength (1.0 = full denoise)")
                    .with_constraint(Constraint::Range {
                        min: 0.0,
                        max: 1.0,
                    })
                    .with_ui_hint(UiHint::Slider { logarithmic: false }),
            )
            .parameter(
                ParameterDefinition::new("width", PortType::Integer, Value::Integer(512))
                    .with_description("Latent image width")
                    .with_constraint(Constraint::Range {
                        min: 64.0,
                        max: 2048.0,
                    })
                    .with_ui_hint(UiHint::Slider { logarithmic: false }),
            )
            .parameter(
                ParameterDefinition::new("height", PortType::Integer, Value::Integer(512))
                    .with_description("Latent image height")
                    .with_constraint(Constraint::Range {
                        min: 64.0,
                        max: 2048.0,
                    })
                    .with_ui_hint(UiHint::Slider { logarithmic: false }),
            )
            .parameter(
                ParameterDefinition::new("batch_size", PortType::Integer, Value::Integer(1))
                    .with_description("Number of images to generate per batch")
                    .with_constraint(Constraint::Range {
                        min: 1.0,
                        max: 16.0,
                    }),
            )
            .parameter(
                ParameterDefinition::new(
                    "timeout_secs",
                    PortType::Integer,
                    Value::Integer(300),
                )
                .with_description("Maximum time to wait for ComfyUI to finish")
                .with_constraint(Constraint::Range {
                    min: 10.0,
                    max: 3600.0,
                }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        validate_comfyui_url(ctx, "comfyui_url")
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let comfyui_url = ctx.get_string("comfyui_url")?;
        let model_ref = ctx
            .get_input("model_ref")
            .ok()
            .and_then(|v| {
                if let Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let positive = ctx
            .get_input("positive")
            .ok()
            .and_then(|v| {
                if let Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let negative = ctx
            .get_input("negative")
            .ok()
            .and_then(|v| {
                if let Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let seed = ctx.get_integer("seed").unwrap_or(0);
        let steps = ctx.get_integer("steps").unwrap_or(20);
        let cfg_scale = ctx.get_float("cfg_scale").unwrap_or(7.0);
        let sampler_name = ctx.get_string("sampler_name").unwrap_or("euler");
        let scheduler = ctx.get_string("scheduler").unwrap_or("normal");
        let denoise = ctx.get_float("denoise").unwrap_or(1.0);
        let width = ctx.get_integer("width").unwrap_or(512);
        let height = ctx.get_integer("height").unwrap_or(512);
        let batch_size = ctx.get_integer("batch_size").unwrap_or(1);
        let _timeout = ctx.get_integer("timeout_secs").unwrap_or(300) as u64;

        // Extract checkpoint name from model_ref
        let ckpt = model_ref
            .strip_prefix("ckpt:")
            .unwrap_or(&model_ref)
            .to_string();

        // Extract prompt text from conditioning refs
        let pos_text = positive
            .rsplit_once(':')
            .map(|(_, t)| t)
            .unwrap_or(&positive)
            .to_string();
        let neg_text = negative
            .rsplit_once(':')
            .map(|(_, t)| t)
            .unwrap_or(&negative)
            .to_string();

        // Build ComfyUI workflow JSON
        let workflow = serde_json::json!({
            "1": {
                "class_type": "CheckpointLoaderSimple",
                "inputs": { "ckpt_name": ckpt }
            },
            "2": {
                "class_type": "CLIPTextEncode",
                "inputs": { "text": pos_text, "clip": ["1", 1] }
            },
            "3": {
                "class_type": "CLIPTextEncode",
                "inputs": { "text": neg_text, "clip": ["1", 1] }
            },
            "4": {
                "class_type": "EmptyLatentImage",
                "inputs": { "width": width, "height": height, "batch_size": batch_size }
            },
            "5": {
                "class_type": "KSampler",
                "inputs": {
                    "model": ["1", 0],
                    "positive": ["2", 0],
                    "negative": ["3", 0],
                    "latent_image": ["4", 0],
                    "seed": seed,
                    "steps": steps,
                    "cfg": cfg_scale,
                    "sampler_name": sampler_name,
                    "scheduler": scheduler,
                    "denoise": denoise,
                }
            },
            "6": {
                "class_type": "VAEDecode",
                "inputs": { "samples": ["5", 0], "vae": ["1", 2] }
            },
            "7": {
                "class_type": "SaveImage",
                "inputs": { "images": ["6", 0], "filename_prefix": "ambara" }
            }
        });

        let prompt_id = submit_prompt(&comfyui_url, &workflow, ctx.node_id)?;

        ctx.set_output(
            "latent_ref",
            Value::String(format!("prompt:{}:5", prompt_id)),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// ComfyVaeDecode — decode latent image to pixels
// ============================================================================

/// Decode a latent image to pixels through ComfyUI, producing an image output.
#[derive(Debug, Clone)]
pub struct ComfyVaeDecode;

impl FilterNode for ComfyVaeDecode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("comfy_vae_decode", "ComfyUI VAE Decode")
            .description(
                "Decode a latent image produced by KSampler into a pixel image \
                 through ComfyUI. Retrieves the generated image from the server.",
            )
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .tag("comfyui")
            .tag("vae")
            .tag("decode")
            .input(
                PortDefinition::input("latent_ref", PortType::String)
                    .with_description("Latent image reference from KSampler"),
            )
            .input(
                PortDefinition::input("vae_ref", PortType::String)
                    .with_description("VAE model reference from Checkpoint Loader")
                    .optional(),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Decoded pixel image"),
            )
            .parameter(
                ParameterDefinition::new(
                    "comfyui_url",
                    PortType::String,
                    Value::String("http://127.0.0.1:8188".to_string()),
                )
                .with_description("ComfyUI server URL")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "timeout_secs",
                    PortType::Integer,
                    Value::Integer(300),
                )
                .with_description("Maximum time to wait for result")
                .with_constraint(Constraint::Range {
                    min: 10.0,
                    max: 3600.0,
                }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        validate_comfyui_url(ctx, "comfyui_url")
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let comfyui_url = ctx.get_string("comfyui_url")?.to_string();
        let timeout = ctx.get_integer("timeout_secs").unwrap_or(300) as u64;
        let latent_ref = ctx
            .get_input("latent_ref")
            .ok()
            .and_then(|v| {
                if let Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        // Extract prompt_id from latent_ref (format: "prompt:<id>:<node>")
        let prompt_id = latent_ref
            .strip_prefix("prompt:")
            .and_then(|rest| rest.split(':').next())
            .unwrap_or(&latent_ref)
            .to_string();

        if prompt_id.is_empty() {
            return Err(ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "No latent reference provided — connect KSampler output".to_string(),
            });
        }

        // Poll for completion
        let history = poll_comfyui_result(&comfyui_url, &prompt_id, timeout, ctx.node_id)?;

        // Download the output image
        let (filename, subfolder, folder_type) =
            first_output_image(&history).ok_or_else(|| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "ComfyUI produced no output images".to_string(),
            })?;

        let img = download_comfyui_image(
            &comfyui_url,
            &filename,
            &subfolder,
            &folder_type,
            ctx.node_id,
        )?;

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(
                img.to_rgba8(),
            ))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// ComfyLoraLoader — load a LoRA model
// ============================================================================

/// Load a LoRA (Low-Rank Adaptation) model through ComfyUI.
#[derive(Debug, Clone)]
pub struct ComfyLoraLoader;

impl FilterNode for ComfyLoraLoader {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("comfy_lora_loader", "ComfyUI LoRA Loader")
            .description(
                "Load a LoRA (Low-Rank Adaptation) model to modify a base checkpoint. \
                 Adjusts both the model and CLIP weights.",
            )
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .tag("comfyui")
            .tag("lora")
            .tag("model")
            .input(
                PortDefinition::input("model_ref", PortType::String)
                    .with_description("Base model reference from Checkpoint Loader"),
            )
            .input(
                PortDefinition::input("clip_ref", PortType::String)
                    .with_description("CLIP model reference from Checkpoint Loader"),
            )
            .output(
                PortDefinition::output("model_ref", PortType::String)
                    .with_description("Model reference with LoRA applied"),
            )
            .output(
                PortDefinition::output("clip_ref", PortType::String)
                    .with_description("CLIP reference with LoRA applied"),
            )
            .parameter(
                ParameterDefinition::new(
                    "comfyui_url",
                    PortType::String,
                    Value::String("http://127.0.0.1:8188".to_string()),
                )
                .with_description("ComfyUI server URL")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "lora_name",
                    PortType::String,
                    Value::String(String::new()),
                )
                .with_description(
                    "LoRA filename in ComfyUI's models/loras folder (e.g. detail_tweaker.safetensors)",
                )
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "model_strength",
                    PortType::Float,
                    Value::Float(1.0),
                )
                .with_description("LoRA strength for the model (UNet) weights")
                .with_constraint(Constraint::Range {
                    min: -2.0,
                    max: 2.0,
                })
                .with_ui_hint(UiHint::Slider { logarithmic: false }),
            )
            .parameter(
                ParameterDefinition::new(
                    "clip_strength",
                    PortType::Float,
                    Value::Float(1.0),
                )
                .with_description("LoRA strength for the CLIP weights")
                .with_constraint(Constraint::Range {
                    min: -2.0,
                    max: 2.0,
                })
                .with_ui_hint(UiHint::Slider { logarithmic: false }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        validate_comfyui_url(ctx, "comfyui_url")?;
        let lora = ctx.get_string("lora_name").unwrap_or("");
        if lora.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "lora_name".to_string(),
                error: "LoRA name cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let model_ref = ctx
            .get_input("model_ref")
            .ok()
            .and_then(|v| {
                if let Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let clip_ref = ctx
            .get_input("clip_ref")
            .ok()
            .and_then(|v| {
                if let Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let lora_name = ctx.get_string("lora_name")?;
        let model_strength = ctx.get_float("model_strength").unwrap_or(1.0);
        let clip_strength = ctx.get_float("clip_strength").unwrap_or(1.0);

        // Produce augmented references
        let new_model = format!(
            "{}+lora:{}:{}",
            model_ref, lora_name, model_strength
        );
        let new_clip = format!(
            "{}+lora:{}:{}",
            clip_ref, lora_name, clip_strength
        );

        ctx.set_output("model_ref", Value::String(new_model))?;
        ctx.set_output("clip_ref", Value::String(new_clip))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// ComfyImageUpscale — upscale an image through ComfyUI
// ============================================================================

/// Upscale an image using an upscale model through ComfyUI.
#[derive(Debug, Clone)]
pub struct ComfyImageUpscale;

impl FilterNode for ComfyImageUpscale {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("comfy_image_upscale", "ComfyUI Image Upscale")
            .description(
                "Upscale an image using a model-based upscaler (RealESRGAN, etc.) \
                 through ComfyUI.",
            )
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .tag("comfyui")
            .tag("upscale")
            .tag("super-resolution")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Input image to upscale"),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("Upscaled output image"),
            )
            .parameter(
                ParameterDefinition::new(
                    "comfyui_url",
                    PortType::String,
                    Value::String("http://127.0.0.1:8188".to_string()),
                )
                .with_description("ComfyUI server URL")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "upscale_model",
                    PortType::String,
                    Value::String("RealESRGAN_x4plus.pth".to_string()),
                )
                .with_description("Upscale model filename in ComfyUI's upscale_models folder")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "timeout_secs",
                    PortType::Integer,
                    Value::Integer(300),
                )
                .with_description("Maximum time to wait for result")
                .with_constraint(Constraint::Range {
                    min: 10.0,
                    max: 3600.0,
                }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        validate_comfyui_url(ctx, "comfyui_url")
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let comfyui_url = ctx.get_string("comfyui_url")?;
        let upscale_model = ctx.get_string("upscale_model")?;
        let timeout = ctx.get_integer("timeout_secs").unwrap_or(300) as u64;
        let input_image = ctx.get_input_image("image")?;

        // Encode input image as PNG
        let rgba = input_image
            .get_image()
            .ok_or_else(|| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Input image data not loaded".to_string(),
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
                    error: format!("Failed to encode input image: {}", e),
                })?;
        }

        // Upload image to ComfyUI
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);

        let upload_body = serde_json::json!({
            "image": b64,
            "type": "input",
            "overwrite": true,
        });

        let upload_resp = ureq::agent()
            .post(&format!("{}/upload/image", comfyui_url))
            .timeout(std::time::Duration::from_secs(30))
            .send_json(&upload_body)
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Image upload to ComfyUI failed: {}", e),
            })?;

        let upload_json: serde_json::Value =
            upload_resp.into_json().map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to parse upload response: {}", e),
            })?;

        let uploaded_name = upload_json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("ambara_input.png");

        // Build upscale workflow
        let workflow = serde_json::json!({
            "1": {
                "class_type": "LoadImage",
                "inputs": { "image": uploaded_name }
            },
            "2": {
                "class_type": "UpscaleModelLoader",
                "inputs": { "model_name": upscale_model }
            },
            "3": {
                "class_type": "ImageUpscaleWithModel",
                "inputs": {
                    "upscale_model": ["2", 0],
                    "image": ["1", 0],
                }
            },
            "4": {
                "class_type": "SaveImage",
                "inputs": { "images": ["3", 0], "filename_prefix": "ambara_upscale" }
            }
        });

        let prompt_id = submit_prompt(&comfyui_url, &workflow, ctx.node_id)?;
        let history = poll_comfyui_result(&comfyui_url, &prompt_id, timeout, ctx.node_id)?;

        let (filename, subfolder, folder_type) =
            first_output_image(&history).ok_or_else(|| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "ComfyUI upscale produced no output".to_string(),
            })?;

        let img = download_comfyui_image(
            &comfyui_url,
            &filename,
            &subfolder,
            &folder_type,
            ctx.node_id,
        )?;

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(
                img.to_rgba8(),
            ))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// ComfyControlNetApply — apply a ControlNet to conditioning
// ============================================================================

/// Apply a ControlNet model to conditioning through ComfyUI.
#[derive(Debug, Clone)]
pub struct ComfyControlNetApply;

impl FilterNode for ComfyControlNetApply {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("comfy_controlnet_apply", "ComfyUI ControlNet Apply")
            .description(
                "Apply a ControlNet to guide image generation using a control image \
                 (edges, depth, pose, etc.) through ComfyUI.",
            )
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .tag("comfyui")
            .tag("controlnet")
            .tag("guided-generation")
            .input(
                PortDefinition::input("conditioning", PortType::String)
                    .with_description("Conditioning from CLIP Text Encode"),
            )
            .input(
                PortDefinition::input("control_image", PortType::Image)
                    .with_description("Control image (edge map, depth map, pose, etc.)"),
            )
            .output(
                PortDefinition::output("conditioning", PortType::String)
                    .with_description("Conditioning with ControlNet applied"),
            )
            .parameter(
                ParameterDefinition::new(
                    "controlnet_name",
                    PortType::String,
                    Value::String(String::new()),
                )
                .with_description(
                    "ControlNet model filename (e.g. control_v11p_sd15_canny.pth)",
                )
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new("strength", PortType::Float, Value::Float(1.0))
                    .with_description("ControlNet influence strength")
                    .with_constraint(Constraint::Range {
                        min: 0.0,
                        max: 2.0,
                    })
                    .with_ui_hint(UiHint::Slider { logarithmic: false }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        let cn = ctx.get_string("controlnet_name").unwrap_or("");
        if cn.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "controlnet_name".to_string(),
                error: "ControlNet model name cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let conditioning = ctx
            .get_input("conditioning")
            .ok()
            .and_then(|v| {
                if let Value::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let controlnet_name = ctx.get_string("controlnet_name")?;
        let strength = ctx.get_float("strength").unwrap_or(1.0);

        // Augment conditioning reference with ControlNet info
        let new_cond = format!(
            "{}+cn:{}:{}",
            conditioning, controlnet_name, strength
        );
        ctx.set_output("conditioning", Value::String(new_cond))?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

// ============================================================================
// ComfyWorkflowRunner — run arbitrary ComfyUI workflows
// ============================================================================

/// Run an arbitrary ComfyUI workflow JSON and retrieve the output image.
///
/// This is the most flexible node — you paste an entire ComfyUI workflow
/// (exported as API format JSON) and it runs it, returning the first output image.
#[derive(Debug, Clone)]
pub struct ComfyWorkflowRunner;

impl FilterNode for ComfyWorkflowRunner {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata::builder("comfy_workflow_runner", "ComfyUI Workflow Runner")
            .description(
                "Run an arbitrary ComfyUI workflow (API-format JSON) and retrieve \
                 the first output image. Paste the exported workflow JSON.",
            )
            .category(Category::Api)
            .author("Ambara")
            .version("1.0.0")
            .tag("comfyui")
            .tag("workflow")
            .tag("advanced")
            .input(
                PortDefinition::input("image", PortType::Image)
                    .with_description("Optional input image (uploaded to ComfyUI for the workflow)")
                    .optional(),
            )
            .output(
                PortDefinition::output("image", PortType::Image)
                    .with_description("First output image from the workflow"),
            )
            .output(
                PortDefinition::output("raw_history", PortType::String)
                    .with_description("Full ComfyUI history JSON for debugging"),
            )
            .parameter(
                ParameterDefinition::new(
                    "comfyui_url",
                    PortType::String,
                    Value::String("http://127.0.0.1:8188".to_string()),
                )
                .with_description("ComfyUI server URL")
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "workflow_json",
                    PortType::String,
                    Value::String(String::new()),
                )
                .with_description(
                    "ComfyUI workflow in API format JSON (use 'Save (API format)' in ComfyUI)",
                )
                .with_constraint(Constraint::NotEmpty),
            )
            .parameter(
                ParameterDefinition::new(
                    "timeout_secs",
                    PortType::Integer,
                    Value::Integer(600),
                )
                .with_description("Maximum time to wait for workflow completion")
                .with_constraint(Constraint::Range {
                    min: 10.0,
                    max: 7200.0,
                }),
            )
            .build()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        validate_comfyui_url(ctx, "comfyui_url")?;
        let wf = ctx.get_string("workflow_json").unwrap_or("");
        if wf.is_empty() {
            return Err(ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "workflow_json".to_string(),
                error: "Workflow JSON cannot be empty".to_string(),
            });
        }
        // Validate it's valid JSON
        serde_json::from_str::<serde_json::Value>(wf).map_err(|e| {
            ValidationError::ConstraintViolation {
                node_id: ctx.node_id,
                parameter: "workflow_json".to_string(),
                error: format!("Invalid workflow JSON: {}", e),
            }
        })?;
        Ok(())
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        let comfyui_url = ctx.get_string("comfyui_url")?.to_string();
        let workflow_str = ctx.get_string("workflow_json")?.to_string();
        let timeout = ctx.get_integer("timeout_secs").unwrap_or(600) as u64;

        let workflow: serde_json::Value =
            serde_json::from_str(&workflow_str).map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Invalid workflow JSON: {}", e),
            })?;

        let prompt_id = submit_prompt(&comfyui_url, &workflow, ctx.node_id)?;
        let history = poll_comfyui_result(&comfyui_url, &prompt_id, timeout, ctx.node_id)?;

        let raw = serde_json::to_string_pretty(&history).unwrap_or_default();
        ctx.set_output("raw_history", Value::String(raw))?;

        let (filename, subfolder, folder_type) =
            first_output_image(&history).ok_or_else(|| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: "Workflow produced no output images".to_string(),
            })?;

        let img = download_comfyui_image(
            &comfyui_url,
            &filename,
            &subfolder,
            &folder_type,
            ctx.node_id,
        )?;

        ctx.set_output(
            "image",
            Value::Image(ImageValue::new(image::DynamicImage::ImageRgba8(
                img.to_rgba8(),
            ))),
        )?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}
