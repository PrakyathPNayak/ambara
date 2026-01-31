//! GPU acceleration infrastructure.
//!
//! This module provides GPU-accelerated image processing using wgpu
//! for cross-platform compute shader support.

use image::{DynamicImage, RgbaImage};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// GPU execution backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackend {
    /// Use Vulkan (Linux, Windows)
    Vulkan,
    /// Use Metal (macOS)
    Metal,
    /// Use DirectX 12 (Windows)
    Dx12,
    /// Use WebGPU (web, fallback)
    WebGpu,
    /// No GPU acceleration
    None,
}

impl From<wgpu::Backend> for GpuBackend {
    fn from(backend: wgpu::Backend) -> Self {
        match backend {
            wgpu::Backend::Vulkan => GpuBackend::Vulkan,
            wgpu::Backend::Metal => GpuBackend::Metal,
            wgpu::Backend::Dx12 => GpuBackend::Dx12,
            wgpu::Backend::BrowserWebGpu => GpuBackend::WebGpu,
            _ => GpuBackend::None,
        }
    }
}

/// GPU device manager with actual wgpu implementation.
pub struct GpuDevice {
    device: wgpu::Device,
    queue: wgpu::Queue,
    backend: GpuBackend,
}

impl GpuDevice {
    /// Initialize GPU device.
    ///
    /// Returns None if no suitable GPU is available.
    pub fn new() -> Option<Self> {
        pollster::block_on(Self::new_async())
    }

    /// Async initialization of GPU device.
    pub async fn new_async() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;

        let backend = adapter.get_info().backend.into();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Ambara GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .ok()?;

        Some(Self {
            device,
            queue,
            backend,
        })
    }

    /// Get the backend being used.
    pub fn backend(&self) -> GpuBackend {
        self.backend
    }

    /// Check if GPU acceleration is available.
    pub fn is_available(&self) -> bool {
        self.backend != GpuBackend::None
    }

    /// Get reference to the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get reference to the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Upload image to GPU texture.
    pub fn upload_image(&self, image: &DynamicImage) -> Result<GpuImage, GpuError> {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Input Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            texture_size,
        );

        Ok(GpuImage {
            texture: Arc::new(texture),
            width,
            height,
            format: ImageFormat::Rgba8,
        })
    }

    /// Create an output texture for compute operations.
    pub fn create_output_texture(&self, width: u32, height: u32) -> GpuImage {
        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Output Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        GpuImage {
            texture: Arc::new(texture),
            width,
            height,
            format: ImageFormat::Rgba8,
        }
    }

    /// Download image from GPU texture.
    pub fn download_image(&self, gpu_image: &GpuImage) -> Result<DynamicImage, GpuError> {
        let buffer_size = (gpu_image.width * gpu_image.height * 4) as u64;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Download Encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &gpu_image.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * gpu_image.width),
                    rows_per_image: Some(gpu_image.height),
                },
            },
            wgpu::Extent3d {
                width: gpu_image.width,
                height: gpu_image.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|_| GpuError::Execution("Failed to receive buffer map result".to_string()))?
            .map_err(|e| GpuError::Execution(format!("Buffer mapping failed: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let rgba_image = RgbaImage::from_raw(gpu_image.width, gpu_image.height, data.to_vec())
            .ok_or_else(|| GpuError::Execution("Failed to create image from buffer".to_string()))?;

        Ok(DynamicImage::ImageRgba8(rgba_image))
    }

    /// Compile a compute shader.
    pub fn compile_shader(&self, source: &str, label: &str) -> Result<GpuShader, GpuError> {
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

        Ok(GpuShader {
            module: Arc::new(module),
            entry_point: "main".to_string(),
        })
    }

    /// Execute a compute shader on input image.
    pub fn execute_compute<T: bytemuck::Pod>(
        &self,
        shader: &GpuShader,
        input: &GpuImage,
        output: &GpuImage,
        params: Option<&T>,
    ) -> Result<(), GpuError> {
        let input_view = input.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create bind group layout entries
        let mut bind_group_layout_entries = vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
        ];

        // Create bind group entries
        let mut bind_group_entries: Vec<wgpu::BindGroupEntry> = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&input_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&output_view),
            },
        ];

        // Add params buffer if provided
        let params_buffer = params.map(|p| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Params Buffer"),
                    contents: bytemuck::bytes_of(p),
                    usage: wgpu::BufferUsages::UNIFORM,
                })
        });

        if params_buffer.is_some() {
            bind_group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
        }

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Compute Bind Group Layout"),
                    entries: &bind_group_layout_entries,
                });

        if let Some(ref buffer) = params_buffer {
            bind_group_entries.push(wgpu::BindGroupEntry {
                binding: 2,
                resource: buffer.as_entire_binding(),
            });
        }

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &bind_group_layout,
            entries: &bind_group_entries,
        });

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader.module,
                entry_point: Some(&shader.entry_point),
                compilation_options: Default::default(),
                cache: None,
            });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Compute Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (8x8 workgroup size)
            let workgroups_x = (output.width + 7) / 8;
            let workgroups_y = (output.height + 7) / 8;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);

        Ok(())
    }
}

/// GPU image handle with actual wgpu texture.
pub struct GpuImage {
    pub texture: Arc<wgpu::Texture>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

impl Clone for GpuImage {
    fn clone(&self) -> Self {
        Self {
            texture: self.texture.clone(),
            width: self.width,
            height: self.height,
            format: self.format,
        }
    }
}

/// Image format for GPU operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// 8-bit RGBA
    Rgba8,
    /// 16-bit RGBA
    Rgba16,
    /// 32-bit float RGBA
    Rgba32Float,
    /// 32-bit float single channel
    R32Float,
}

/// Compiled GPU shader.
pub struct GpuShader {
    module: Arc<wgpu::ShaderModule>,
    entry_point: String,
}

impl GpuShader {
    /// Create a new shader from WGSL source.
    pub fn from_wgsl(device: &GpuDevice, source: &str) -> Result<Self, GpuError> {
        device.compile_shader(source, "Custom Shader")
    }
}

/// GPU-related errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum GpuError {
    /// GPU not available
    #[error("GPU not available")]
    NotAvailable,

    /// GPU operation not implemented yet
    #[error("GPU operation not implemented yet")]
    NotImplemented,

    /// Shader compilation failed
    #[error("Shader compilation failed: {0}")]
    ShaderCompilation(String),

    /// GPU execution failed
    #[error("GPU execution failed: {0}")]
    Execution(String),

    /// Memory allocation failed
    #[error("Memory allocation failed")]
    OutOfMemory,
}

/// GPU acceleration capability trait.
pub trait GpuAccelerated {
    /// Whether this filter supports GPU acceleration.
    fn supports_gpu(&self) -> bool {
        false
    }

    /// Execute on GPU if available.
    fn execute_gpu(
        &self,
        _device: &GpuDevice,
        _ctx: &mut crate::core::context::ExecutionContext,
    ) -> Result<(), crate::core::error::ExecutionError> {
        Err(crate::core::error::ExecutionError::NodeExecution {
            node_id: _ctx.node_id,
            error: "GPU execution not implemented for this filter".to_string(),
        })
    }
}

/// Global GPU device pool for sharing across filters.
pub struct GpuPool {
    device: Option<Arc<GpuDevice>>,
}

impl GpuPool {
    /// Get or initialize the global GPU pool.
    pub fn global() -> &'static Self {
        static POOL: std::sync::OnceLock<GpuPool> = std::sync::OnceLock::new();
        POOL.get_or_init(|| {
            let device = GpuDevice::new().map(Arc::new);
            GpuPool { device }
        })
    }

    /// Get the GPU device if available.
    pub fn device(&self) -> Option<Arc<GpuDevice>> {
        self.device.clone()
    }

    /// Check if GPU is available.
    pub fn is_available(&self) -> bool {
        self.device.is_some()
    }
}

// ============================================================================
// GPU Filter Implementations
// ============================================================================

/// Parameters for Gaussian blur shader.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlurParams {
    pub radius: f32,
    pub sigma: f32,
    pub _padding: [f32; 2],
}

/// Parameters for brightness/contrast shader.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BrightnessContrastParams {
    pub brightness: f32,
    pub contrast: f32,
    pub _padding: [f32; 2],
}

/// Parameters for HSV adjustment shader.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HsvParams {
    pub hue_shift: f32,
    pub saturation: f32,
    pub value: f32,
    pub _padding: f32,
}

/// GPU-accelerated image filter operations.
pub struct GpuFilters {
    device: Arc<GpuDevice>,
    blur_shader: GpuShader,
    brightness_contrast_shader: GpuShader,
    grayscale_shader: GpuShader,
    invert_shader: GpuShader,
    hsv_shader: GpuShader,
}

impl GpuFilters {
    /// Create new GPU filters instance.
    pub fn new(device: Arc<GpuDevice>) -> Result<Self, GpuError> {
        let blur_shader = device.compile_shader(shaders::GAUSSIAN_BLUR, "Gaussian Blur")?;
        let brightness_contrast_shader =
            device.compile_shader(shaders::BRIGHTNESS_CONTRAST, "Brightness Contrast")?;
        let grayscale_shader = device.compile_shader(shaders::GRAYSCALE, "Grayscale")?;
        let invert_shader = device.compile_shader(shaders::INVERT, "Invert")?;
        let hsv_shader = device.compile_shader(shaders::HSV_ADJUST, "HSV Adjust")?;

        Ok(Self {
            device,
            blur_shader,
            brightness_contrast_shader,
            grayscale_shader,
            invert_shader,
            hsv_shader,
        })
    }

    /// Apply Gaussian blur to an image.
    pub fn gaussian_blur(
        &self,
        image: &DynamicImage,
        radius: f32,
        sigma: f32,
    ) -> Result<DynamicImage, GpuError> {
        let input = self.device.upload_image(image)?;
        let output = self.device.create_output_texture(input.width, input.height);

        let params = BlurParams {
            radius,
            sigma,
            _padding: [0.0; 2],
        };

        self.device
            .execute_compute(&self.blur_shader, &input, &output, Some(&params))?;
        self.device.download_image(&output)
    }

    /// Adjust brightness and contrast.
    pub fn brightness_contrast(
        &self,
        image: &DynamicImage,
        brightness: f32,
        contrast: f32,
    ) -> Result<DynamicImage, GpuError> {
        let input = self.device.upload_image(image)?;
        let output = self.device.create_output_texture(input.width, input.height);

        let params = BrightnessContrastParams {
            brightness,
            contrast,
            _padding: [0.0; 2],
        };

        self.device.execute_compute(
            &self.brightness_contrast_shader,
            &input,
            &output,
            Some(&params),
        )?;
        self.device.download_image(&output)
    }

    /// Convert to grayscale.
    pub fn grayscale(&self, image: &DynamicImage) -> Result<DynamicImage, GpuError> {
        let input = self.device.upload_image(image)?;
        let output = self.device.create_output_texture(input.width, input.height);

        self.device
            .execute_compute::<[f32; 4]>(&self.grayscale_shader, &input, &output, None)?;
        self.device.download_image(&output)
    }

    /// Invert colors.
    pub fn invert(&self, image: &DynamicImage) -> Result<DynamicImage, GpuError> {
        let input = self.device.upload_image(image)?;
        let output = self.device.create_output_texture(input.width, input.height);

        self.device
            .execute_compute::<[f32; 4]>(&self.invert_shader, &input, &output, None)?;
        self.device.download_image(&output)
    }

    /// Adjust HSV values.
    pub fn hsv_adjust(
        &self,
        image: &DynamicImage,
        hue_shift: f32,
        saturation: f32,
        value: f32,
    ) -> Result<DynamicImage, GpuError> {
        let input = self.device.upload_image(image)?;
        let output = self.device.create_output_texture(input.width, input.height);

        let params = HsvParams {
            hue_shift,
            saturation,
            value,
            _padding: 0.0,
        };

        self.device
            .execute_compute(&self.hsv_shader, &input, &output, Some(&params))?;
        self.device.download_image(&output)
    }
}

// ============================================================================
// Compute Shaders (WGSL)
// ============================================================================

/// WGSL compute shaders for image processing.
pub mod shaders {
    /// Gaussian blur compute shader.
    pub const GAUSSIAN_BLUR: &str = r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: BlurParams;

struct BlurParams {
    radius: f32,
    sigma: f32,
    _padding: vec2<f32>,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    let coords = vec2<i32>(global_id.xy);
    
    if (coords.x >= i32(dims.x) || coords.y >= i32(dims.y)) {
        return;
    }

    var color = vec4<f32>(0.0);
    var weight_sum = 0.0;

    let radius = i32(params.radius);
    let sigma2 = params.sigma * params.sigma;

    for (var dy = -radius; dy <= radius; dy = dy + 1) {
        for (var dx = -radius; dx <= radius; dx = dx + 1) {
            let sample_x = clamp(coords.x + dx, 0, i32(dims.x) - 1);
            let sample_y = clamp(coords.y + dy, 0, i32(dims.y) - 1);
            let sample_coords = vec2<i32>(sample_x, sample_y);
            
            let dist = f32(dx * dx + dy * dy);
            let weight = exp(-dist / (2.0 * sigma2));
            
            color += textureLoad(input_texture, sample_coords, 0) * weight;
            weight_sum += weight;
        }
    }

    textureStore(output_texture, coords, color / weight_sum);
}
"#;

    /// Brightness/contrast adjustment shader.
    pub const BRIGHTNESS_CONTRAST: &str = r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: AdjustParams;

struct AdjustParams {
    brightness: f32,
    contrast: f32,
    _padding: vec2<f32>,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    let coords = vec2<i32>(global_id.xy);
    
    if (coords.x >= i32(dims.x) || coords.y >= i32(dims.y)) {
        return;
    }

    var color = textureLoad(input_texture, coords, 0);
    
    // Apply brightness (additive)
    color = vec4<f32>(
        color.r + params.brightness,
        color.g + params.brightness,
        color.b + params.brightness,
        color.a
    );
    
    // Apply contrast (multiplicative around 0.5)
    color = vec4<f32>(
        (color.r - 0.5) * params.contrast + 0.5,
        (color.g - 0.5) * params.contrast + 0.5,
        (color.b - 0.5) * params.contrast + 0.5,
        color.a
    );
    
    // Clamp to valid range
    color = clamp(color, vec4<f32>(0.0, 0.0, 0.0, 0.0), vec4<f32>(1.0, 1.0, 1.0, 1.0));
    
    textureStore(output_texture, coords, color);
}
"#;

    /// Grayscale conversion shader.
    pub const GRAYSCALE: &str = r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    let coords = vec2<i32>(global_id.xy);
    
    if (coords.x >= i32(dims.x) || coords.y >= i32(dims.y)) {
        return;
    }

    let color = textureLoad(input_texture, coords, 0);
    
    // ITU-R BT.709 luma coefficients
    let gray = 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;
    
    textureStore(output_texture, coords, vec4<f32>(gray, gray, gray, color.a));
}
"#;

    /// Color inversion shader.
    pub const INVERT: &str = r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    let coords = vec2<i32>(global_id.xy);
    
    if (coords.x >= i32(dims.x) || coords.y >= i32(dims.y)) {
        return;
    }

    let color = textureLoad(input_texture, coords, 0);
    
    textureStore(output_texture, coords, vec4<f32>(1.0 - color.r, 1.0 - color.g, 1.0 - color.b, color.a));
}
"#;

    /// HSV adjustment shader.
    pub const HSV_ADJUST: &str = r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: HsvParams;

struct HsvParams {
    hue_shift: f32,
    saturation: f32,
    value: f32,
    _padding: f32,
}

fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32> {
    let c_max = max(max(rgb.r, rgb.g), rgb.b);
    let c_min = min(min(rgb.r, rgb.g), rgb.b);
    let delta = c_max - c_min;
    
    var h: f32 = 0.0;
    if (delta > 0.0) {
        if (c_max == rgb.r) {
            h = ((rgb.g - rgb.b) / delta) % 6.0;
        } else if (c_max == rgb.g) {
            h = (rgb.b - rgb.r) / delta + 2.0;
        } else {
            h = (rgb.r - rgb.g) / delta + 4.0;
        }
        h = h / 6.0;
        if (h < 0.0) {
            h = h + 1.0;
        }
    }
    
    var s: f32 = 0.0;
    if (c_max > 0.0) {
        s = delta / c_max;
    }
    
    return vec3<f32>(h, s, c_max);
}

fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x * 6.0;
    let s = hsv.y;
    let v = hsv.z;
    
    let c = v * s;
    let x = c * (1.0 - abs(h % 2.0 - 1.0));
    let m = v - c;
    
    var rgb: vec3<f32>;
    let hi = i32(floor(h));
    switch (hi) {
        case 0: { rgb = vec3<f32>(c, x, 0.0); }
        case 1: { rgb = vec3<f32>(x, c, 0.0); }
        case 2: { rgb = vec3<f32>(0.0, c, x); }
        case 3: { rgb = vec3<f32>(0.0, x, c); }
        case 4: { rgb = vec3<f32>(x, 0.0, c); }
        case 5, default: { rgb = vec3<f32>(c, 0.0, x); }
    }
    
    return rgb + vec3<f32>(m, m, m);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    let coords = vec2<i32>(global_id.xy);
    
    if (coords.x >= i32(dims.x) || coords.y >= i32(dims.y)) {
        return;
    }

    let color = textureLoad(input_texture, coords, 0);
    var hsv = rgb_to_hsv(color.rgb);
    
    // Apply adjustments
    hsv.x = (hsv.x + params.hue_shift) % 1.0;
    if (hsv.x < 0.0) {
        hsv.x = hsv.x + 1.0;
    }
    hsv.y = clamp(hsv.y * params.saturation, 0.0, 1.0);
    hsv.z = clamp(hsv.z * params.value, 0.0, 1.0);
    
    let rgb = hsv_to_rgb(hsv);
    textureStore(output_texture, coords, vec4<f32>(rgb, color.a));
}
"#;

    /// Sharpen filter shader.
    pub const SHARPEN: &str = r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: SharpenParams;

struct SharpenParams {
    strength: f32,
    _padding: vec3<f32>,
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    let coords = vec2<i32>(global_id.xy);
    
    if (coords.x >= i32(dims.x) || coords.y >= i32(dims.y)) {
        return;
    }

    let center = textureLoad(input_texture, coords, 0);
    
    // Sample neighbors with bounds checking
    let left = textureLoad(input_texture, vec2<i32>(max(coords.x - 1, 0), coords.y), 0);
    let right = textureLoad(input_texture, vec2<i32>(min(coords.x + 1, i32(dims.x) - 1), coords.y), 0);
    let up = textureLoad(input_texture, vec2<i32>(coords.x, max(coords.y - 1, 0)), 0);
    let down = textureLoad(input_texture, vec2<i32>(coords.x, min(coords.y + 1, i32(dims.y) - 1)), 0);
    
    // Laplacian kernel: -1 -1 -1 / -1 8 -1 / -1 -1 -1 simplified to 4-neighbor
    let edge = center * 5.0 - left - right - up - down;
    
    // Blend based on strength
    var result = center + (edge - center) * params.strength;
    result = clamp(result, vec4<f32>(0.0), vec4<f32>(1.0));
    result.a = center.a;
    
    textureStore(output_texture, coords, result);
}
"#;

    /// Edge detection (Sobel) shader.
    pub const EDGE_DETECT: &str = r#"
@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba8unorm, write>;

fn luminance(color: vec4<f32>) -> f32 {
    return 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dims = textureDimensions(input_texture);
    let coords = vec2<i32>(global_id.xy);
    
    if (coords.x >= i32(dims.x) || coords.y >= i32(dims.y)) {
        return;
    }

    // Sample 3x3 neighborhood
    var samples: array<f32, 9>;
    var idx = 0;
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            let sx = clamp(coords.x + dx, 0, i32(dims.x) - 1);
            let sy = clamp(coords.y + dy, 0, i32(dims.y) - 1);
            samples[idx] = luminance(textureLoad(input_texture, vec2<i32>(sx, sy), 0));
            idx = idx + 1;
        }
    }

    // Sobel kernels
    let gx = samples[2] + 2.0 * samples[5] + samples[8] - samples[0] - 2.0 * samples[3] - samples[6];
    let gy = samples[6] + 2.0 * samples[7] + samples[8] - samples[0] - 2.0 * samples[1] - samples[2];
    
    let edge = sqrt(gx * gx + gy * gy);
    let result = clamp(edge, 0.0, 1.0);
    
    textureStore(output_texture, coords, vec4<f32>(result, result, result, 1.0));
}
"#;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_device_creation() {
        // This may fail on CI without GPU, so we just test it doesn't panic
        let _device = GpuDevice::new();
    }

    #[test]
    fn test_gpu_pool() {
        let pool = GpuPool::global();
        // Should not panic
        let _ = pool.is_available();
    }

    #[test]
    fn test_blur_params_layout() {
        // Verify params are correctly sized for GPU
        assert_eq!(std::mem::size_of::<BlurParams>(), 16);
        assert_eq!(std::mem::size_of::<BrightnessContrastParams>(), 16);
        assert_eq!(std::mem::size_of::<HsvParams>(), 16);
    }

    #[test]
    #[ignore] // Requires GPU
    fn test_gpu_grayscale() {
        let device = GpuDevice::new().expect("GPU required");
        let device = Arc::new(device);
        let filters = GpuFilters::new(device).expect("Failed to create filters");

        // Create a test image
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            100,
            100,
            image::Rgba([255, 128, 64, 255]),
        ));

        let result = filters.grayscale(&img).expect("Grayscale failed");
        assert_eq!(result.width(), 100);
        assert_eq!(result.height(), 100);
    }

    #[test]
    #[ignore] // Requires GPU
    fn test_gpu_blur() {
        let device = GpuDevice::new().expect("GPU required");
        let device = Arc::new(device);
        let filters = GpuFilters::new(device).expect("Failed to create filters");

        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            100,
            100,
            image::Rgba([255, 128, 64, 255]),
        ));

        let result = filters
            .gaussian_blur(&img, 3.0, 1.5)
            .expect("Blur failed");
        assert_eq!(result.width(), 100);
        assert_eq!(result.height(), 100);
    }
}
