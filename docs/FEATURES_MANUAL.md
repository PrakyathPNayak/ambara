# Ambara Features Manual

> **Version 0.5.0** — Node-based Image Processing with Plugin Support and ComfyUI Integration

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Filter Reference](#filter-reference)
   - [Blur](#blur)
   - [Color](#color)
   - [Adjust](#adjust)
   - [Edge Detection](#edge-detection)
   - [Transform](#transform)
   - [Composite](#composite)
   - [Sharpen](#sharpen)
   - [Noise](#noise)
   - [Draw](#draw)
   - [Text](#text)
   - [Astrophotography](#astrophotography)
   - [Input / Output](#input--output)
   - [Utility](#utility)
   - [Constants](#constants)
   - [Type Conversion](#type-conversion)
   - [Math](#math)
   - [Comparison & Logic](#comparison--logic)
   - [Array Operations](#array-operations)
   - [Batch Processing](#batch-processing)
   - [ComfyUI Integration](#comfyui-integration)
   - [External API](#external-api)
4. [Plugin System](#plugin-system)
5. [GPU Acceleration](#gpu-acceleration)
6. [User Interface](#user-interface)
7. [CLI Usage](#cli-usage)

---

## Overview

Ambara is a Rust-based, node-graph image processing application inspired by ComfyUI. Users build processing pipelines by connecting filter nodes in a directed acyclic graph (DAG). The engine validates connections, resolves execution order via topological sort, caches intermediate results, and executes independent branches in parallel.

**Key capabilities:**

- **111 built-in filter nodes** across 18 categories
- **GPU-accelerated** blur, grayscale, and invert via wgpu
- **Parallel batch processing** with Rayon
- **External API integration** — Stable Diffusion, ComfyUI, style transfer, classification
- **Plugin system** — load third-party Rust shared libraries (.so/.dll/.dylib) at runtime
- **Tauri + React UI** with visual node graph, chat panel, plugin manager, and live preview
- **CLI** for headless graph execution, filter listing, and single-image processing

---

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                     Ambara Host                       │
│                                                       │
│  FilterRegistry ── discovers ──► builtin filters      │
│       │                                               │
│  PluginRegistry ── loads ──► .so/.dll/.dylib plugins  │
│       │                                               │
│  ProcessingGraph ── connects ──► GraphNodes            │
│       │                                               │
│  ValidationPipeline ── validates ──► type safety       │
│       │                                               │
│  ExecutionEngine ── executes ──► parallel + cached     │
└──────────────────────────────────────────────────────┘
```

**Core trait — `FilterNode`:**

Every filter implements three methods:
- `metadata()` — declares ID, name, description, category, inputs, outputs, and parameters
- `validate()` — checks parameter constraints before execution
- `execute()` — performs the image processing operation

---

## Filter Reference

### Blur

#### `gaussian_blur` — Gaussian Blur

Apply Gaussian blur to an image. **GPU-accelerated** via wgpu when available.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Blurred image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `sigma` | Float | 1.0 | 0.1 – 100.0 | Blur radius (standard deviation) |
| `use_gpu` | Boolean | true | — | Use GPU acceleration if available |

---

#### `box_blur` — Box Blur

Fast box (average) blur with independent X/Y radii.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Blurred image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `radius_x` | Integer | 3 | 1 – 50 | Horizontal blur radius |
| `radius_y` | Integer | 3 | 1 – 50 | Vertical blur radius |

---

#### `median_blur` — Median Blur

Non-linear median filter — excellent for salt-and-pepper noise removal while preserving edges.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Filtered image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `radius` | Integer | 2 | 1 – 20 | Filter kernel radius |

---

#### `motion_blur` — Motion Blur

Simulates directional motion blur at a specified angle.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Motion-blurred image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `length` | Integer | 10 | 1 – 100 | Blur streak length in pixels |
| `angle` | Float | 0.0 | 0.0 – 360.0 | Direction angle in degrees |

---

### Color

#### `brightness` — Brightness

Adjust image brightness by adding/subtracting a uniform value from all pixels.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Adjusted image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `amount` | Float | 0.0 | -1.0 – 1.0 | Brightness adjustment (negative = darker) |

---

#### `contrast` — Contrast

Adjust image contrast by scaling pixel values around the midpoint.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Adjusted image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `amount` | Float | 1.0 | 0.0 – 3.0 | Contrast multiplier (1.0 = no change) |

---

#### `saturation` — Saturation

Adjust color saturation.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Adjusted image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `amount` | Float | 1.0 | 0.0 – 3.0 | Saturation multiplier (0 = grayscale, 1 = no change) |

---

#### `grayscale` — Grayscale

Convert image to grayscale. **GPU-accelerated** via wgpu when available.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Grayscale image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `use_gpu` | Boolean | true | — | Use GPU acceleration if available |

---

#### `invert` — Invert Colors

Invert all pixel color values. **GPU-accelerated** via wgpu when available.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Inverted image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `invert_alpha` | Boolean | false | — | Also invert the alpha channel |
| `use_gpu` | Boolean | true | — | Use GPU acceleration if available |

---

#### `sepia` — Sepia

Apply a warm sepia tone effect.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Sepia-toned image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `intensity` | Float | 1.0 | 0.0 – 1.0 | Sepia effect strength |

---

#### `hue_rotate` — Hue Rotate

Rotate the hue component of all pixels in HSL color space.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Hue-rotated image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `angle` | Float | 0.0 | -360.0 – 360.0 | Hue rotation in degrees |

---

#### `threshold` — Threshold

Binary threshold — pixels above the level become white, below become black.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Binary image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `level` | Integer | 128 | 0 – 255 | Threshold cutoff value |

---

#### `posterize` — Posterize

Reduce the number of distinct color levels per channel, creating a poster-like effect.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Posterized image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `levels` | Integer | 4 | 2 – 256 | Number of color levels per channel |

---

#### `levels_adjust` — Levels Adjust

Remap the tonal range by setting black point, white point, and gamma.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Levels-adjusted image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `black_point` | Float | 0.0 | 0.0 – 255.0 | Input shadows clamp |
| `white_point` | Float | 255.0 | 0.0 – 255.0 | Input highlights clamp |
| `gamma` | Float | 1.0 | 0.1 – 10.0 | Midtone gamma (logarithmic scale) |

---

#### `channel_mixer` — Channel Mixer

Mix RGB channels using a 3×3 weight matrix. Each output channel is a weighted sum of all input channels.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Channel-mixed image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `rr` | Float | 1.0 | -2.0 – 2.0 | Red → Red weight |
| `rg` | Float | 0.0 | -2.0 – 2.0 | Green → Red weight |
| `rb` | Float | 0.0 | -2.0 – 2.0 | Blue → Red weight |
| `gr` | Float | 0.0 | -2.0 – 2.0 | Red → Green weight |
| `gg` | Float | 1.0 | -2.0 – 2.0 | Green → Green weight |
| `gb` | Float | 0.0 | -2.0 – 2.0 | Blue → Green weight |
| `br` | Float | 0.0 | -2.0 – 2.0 | Red → Blue weight |
| `bg` | Float | 0.0 | -2.0 – 2.0 | Green → Blue weight |
| `bb` | Float | 1.0 | -2.0 – 2.0 | Blue → Blue weight |

---

#### `vibrance` — Vibrance

Selectively boost saturation of less-saturated colors while protecting already-vivid colors.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Vibrance-adjusted image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `amount` | Float | 0.5 | -1.0 – 1.0 | Vibrance intensity (negative = desaturate) |

---

### Adjust

#### `gamma` — Gamma Correction

Apply gamma correction to adjust midtone luminance.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Gamma-corrected image |

| Parameter | Type | Default | Range | Scale | Description |
|-----------|------|---------|-------|-------|-------------|
| `gamma` | Float | 1.0 | 0.1 – 5.0 | Logarithmic | Gamma value (< 1 brightens, > 1 darkens) |

---

#### `color_balance` — Color Balance

Independently scale each RGB channel.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Color-balanced image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `red` | Float | 1.0 | 0.0 – 3.0 | Red channel multiplier |
| `green` | Float | 1.0 | 0.0 – 3.0 | Green channel multiplier |
| `blue` | Float | 1.0 | 0.0 – 3.0 | Blue channel multiplier |

---

### Edge Detection

#### `edge_detect` — Edge Detect

Detect edges using Sobel or Prewitt operators.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Edge map |

| Parameter | Type | Default | Options | Description |
|-----------|------|---------|---------|-------------|
| `method` | String | `"sobel"` | `sobel`, `prewitt` | Edge detection algorithm |
| `invert` | Boolean | false | — | Invert the edge map |

---

#### `emboss` — Emboss

Create a raised relief / emboss effect.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Embossed image |

| Parameter | Type | Default | Options | Description |
|-----------|------|---------|---------|-------------|
| `direction` | String | `"top_left"` | `top_left`, `top_right`, `bottom_left`, `bottom_right` | Light direction |

---

#### `canny_edge` — Canny Edge Detection

Multi-stage Canny algorithm with hysteresis thresholding for clean, thin edge detection.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Edge map |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `low_threshold` | Float | 50.0 | 0.0 – 255.0 | Lower hysteresis threshold |
| `high_threshold` | Float | 150.0 | 0.0 – 255.0 | Upper hysteresis threshold |

---

#### `laplacian` — Laplacian Edge Detection

Second-derivative edge detection using the Laplacian operator.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Edge map |

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `normalize` | Boolean | true | Normalize output to 0–255 range |

---

### Transform

#### `resize` — Resize

Resize an image with configurable interpolation.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Resized image |

| Parameter | Type | Default | Range | Options | Description |
|-----------|------|---------|-------|---------|-------------|
| `width` | Integer | 1920 | 1 – 16384 | — | Target width in pixels |
| `height` | Integer | 1080 | 1 – 16384 | — | Target height in pixels |
| `preserve_aspect` | Boolean | true | — | — | Maintain aspect ratio |
| `filter` | String | `"lanczos3"` | — | `nearest`, `triangle`, `catmullrom`, `gaussian`, `lanczos3` | Resampling algorithm |

---

#### `rotate` — Rotate

Rotate image by 90° increments.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Rotated image |

| Parameter | Type | Default | Options | Description |
|-----------|------|---------|---------|-------------|
| `angle` | String | `"90"` | `90`, `180`, `270` | Rotation angle (clockwise) |

---

#### `flip` — Flip

Flip image horizontally and/or vertically.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Flipped image |

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `horizontal` | Boolean | true | Mirror left ↔ right |
| `vertical` | Boolean | false | Mirror top ↔ bottom |

---

#### `crop` — Crop

Extract a rectangular region from an image.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Cropped region |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `x` | Integer | 0 | 0 – ∞ | Left edge X coordinate |
| `y` | Integer | 0 | 0 – ∞ | Top edge Y coordinate |
| `width` | Integer | 100 | 1 – ∞ | Region width |
| `height` | Integer | 100 | 1 – ∞ | Region height |

---

### Composite

#### `blend` — Blend

Blend two images using standard blending modes.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `base` | Input | Image | Base (bottom) image |
| `blend` | Input | Image | Blend (top) image |
| `image` | Output | Image | Composited result |

| Parameter | Type | Default | Options | Range | Description |
|-----------|------|---------|---------|-------|-------------|
| `mode` | String | `"normal"` | `normal`, `multiply`, `screen`, `overlay`, `darken`, `lighten`, `add`, `subtract`, `difference` | — | Blend mode |
| `opacity` | Float | 1.0 | — | 0.0 – 1.0 | Blend layer opacity |

---

#### `overlay` — Overlay

Place one image on top of another at a specified position.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `base` | Input | Image | Background image |
| `overlay` | Input | Image | Foreground image |
| `image` | Output | Image | Composited result |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `x` | Integer | 0 | — | Horizontal offset |
| `y` | Integer | 0 | — | Vertical offset |
| `opacity` | Float | 1.0 | 0.0 – 1.0 | Overlay opacity |

---

### Sharpen

#### `unsharp_mask` — Unsharp Mask

Classic unsharp masking — sharpen by subtracting a blurred copy from the original.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Sharpened image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `sigma` | Float | 1.0 | 0.1 – 20.0 | Blur radius for the mask |
| `amount` | Float | 1.0 | 0.0 – 5.0 | Sharpening strength |
| `threshold` | Integer | 0 | 0 – 255 | Minimum edge contrast to sharpen |

---

#### `sharpen` — Sharpen

Apply a 3×3 convolution sharpening kernel.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Sharpened image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `strength` | Float | 1.0 | 0.0 – 5.0 | Kernel strength multiplier |

---

### Noise

#### `add_noise` — Add Noise

Inject synthetic noise into an image.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Noisy image |

| Parameter | Type | Default | Options | Range | Description |
|-----------|------|---------|---------|-------|-------------|
| `noise_type` | String | `"gaussian"` | `gaussian`, `salt_pepper` | — | Noise distribution type |
| `amount` | Float | 0.1 | — | 0.0 – 1.0 | Noise intensity |
| `seed` | Integer | 42 | — | — | Random seed for reproducibility |

---

#### `denoise` — Denoise

Remove noise using a median filter.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Noisy image |
| `image` | Output | Image | Denoised image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `radius` | Integer | 1 | 1 – 5 | Median filter kernel radius |

---

### Draw

#### `draw_rectangle` — Draw Rectangle

Draw a filled or outlined rectangle onto an image.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Annotated image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `x` | Integer | 10 | 0 – ∞ | Top-left X coordinate |
| `y` | Integer | 10 | 0 – ∞ | Top-left Y coordinate |
| `width` | Integer | 100 | 1 – ∞ | Rectangle width |
| `height` | Integer | 100 | 1 – ∞ | Rectangle height |
| `color_r` | Integer | 255 | 0 – 255 | Red component |
| `color_g` | Integer | 0 | 0 – 255 | Green component |
| `color_b` | Integer | 0 | 0 – 255 | Blue component |
| `filled` | Boolean | true | — | Fill the rectangle |
| `thickness` | Integer | 2 | 1 – 50 | Outline stroke width (when not filled) |

---

#### `draw_circle` — Draw Circle

Draw a filled or outlined circle.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Annotated image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `center_x` | Integer | 100 | — | Circle center X |
| `center_y` | Integer | 100 | — | Circle center Y |
| `radius` | Integer | 50 | 1 – ∞ | Circle radius |
| `color_r` | Integer | 0 | 0 – 255 | Red component |
| `color_g` | Integer | 255 | 0 – 255 | Green component |
| `color_b` | Integer | 0 | 0 – 255 | Blue component |
| `filled` | Boolean | true | — | Fill the circle |

---

#### `draw_line` — Draw Line

Draw a straight line using Bresenham's algorithm.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Annotated image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `x1` | Integer | 0 | — | Start X |
| `y1` | Integer | 0 | — | Start Y |
| `x2` | Integer | 100 | — | End X |
| `y2` | Integer | 100 | — | End Y |
| `color_r` | Integer | 255 | 0 – 255 | Red component |
| `color_g` | Integer | 255 | 0 – 255 | Green component |
| `color_b` | Integer | 255 | 0 – 255 | Blue component |
| `thickness` | Integer | 2 | 1 – 50 | Line width |

---

### Text

#### `text_overlay` — Text Overlay

Render text onto an image using a built-in 8×13 bitmap font.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Annotated image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `text` | String | `"Hello Ambara"` | — | Text content to render |
| `x` | Integer | 10 | 0 – ∞ | Text position X |
| `y` | Integer | 10 | 0 – ∞ | Text position Y |
| `scale` | Integer | 2 | 1 – 10 | Pixel scaling factor for the bitmap font |
| `color_r` | Integer | 255 | 0 – 255 | Red component |
| `color_g` | Integer | 255 | 0 – 255 | Green component |
| `color_b` | Integer | 255 | 0 – 255 | Blue component |

---

### Astrophotography

Specialized filters for astronomical image processing.

#### `image_stack` — Image Stack

Stack multiple exposures to reduce noise. Supports statistical combination methods commonly used in astrophotography.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `images` | Input | Array\<Image\> | Stack of input exposures |
| `image` | Output | Image | Combined result |

| Parameter | Type | Default | Options | Range | Description |
|-----------|------|---------|---------|-------|-------------|
| `method` | String | `"mean"` | `mean`, `median`, `sigma_clip`, `max`, `min` | — | Stacking algorithm |
| `sigma` | Float | 2.0 | — | 0.5 – 5.0 | Sigma-clip rejection threshold (only for `sigma_clip`) |

---

#### `dark_frame_subtract` — Dark Frame Subtract

Remove thermal noise by subtracting a dark frame (exposure with lens cap on).

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Light frame |
| `dark` | Input | Image | Dark frame |
| `image` | Output | Image | Calibrated image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `scale` | Float | 1.0 | 0.0 – 2.0 | Dark frame scaling factor |

---

#### `flat_field_correct` — Flat Field Correct

Remove vignetting and dust shadows by dividing by a flat-field frame.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Light frame |
| `flat` | Input | Image | Flat-field frame |
| `image` | Output | Image | Corrected image |

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `normalize` | Boolean | true | Normalize the flat field before dividing |

---

#### `hot_pixel_removal` — Hot Pixel Removal

Detect and replace stuck hot pixels using local neighborhood analysis.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Cleaned image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `threshold` | Float | 50.0 | 10.0 – 200.0 | Deviation threshold for hot pixel detection |

---

#### `histogram_stretch` — Histogram Stretch

Stretch the histogram to enhance faint details — essential for deep-sky imaging.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Stretched image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `black_point` | Float | 0.0 | 0.0 – 0.5 | Shadow clipping point (fraction) |
| `white_point` | Float | 1.0 | 0.5 – 1.0 | Highlight clipping point (fraction) |
| `midtone` | Float | 0.5 | 0.0 – 1.0 | Midtone transfer function bias |

---

### Input / Output

#### `load_image` — Load Image

Load a single image from disk.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Output | Image | Loaded image |

| Parameter | Type | Default | Constraints | Description |
|-----------|------|---------|-------------|-------------|
| `path` | String | `""` | NotEmpty, FileChooser (png/jpg/jpeg/gif/bmp/tiff/webp) | File path |

---

#### `load_folder` — Load Folder

Load all images from a directory into an array.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `images` | Output | Array\<Image\> | All loaded images |
| `count` | Output | Integer | Number of images found |

| Parameter | Type | Default | Constraints | Description |
|-----------|------|---------|-------------|-------------|
| `directory` | String | `""` | NotEmpty, FileChooser | Folder path |
| `recursive` | Boolean | false | — | Scan subdirectories |
| `pattern` | String | `"*"` | — | Glob pattern for filtering filenames |

---

#### `save_image` — Save Image

Save a single image to disk.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Image to save |
| `path` | Output | String | Written file path |

| Parameter | Type | Default | Options | Range | Description |
|-----------|------|---------|---------|-------|-------------|
| `directory` | String | `"./output"` | — | — | Output directory |
| `filename` | String | `"output"` | — | — | Base filename (without extension) |
| `format` | String | `"png"` | `png`, `jpg`, `webp`, `bmp`, `tiff` | — | Image format |
| `quality` | Integer | 90 | — | 1 – 100 | Compression quality (JPEG/WebP) |
| `create_dirs` | Boolean | true | — | — | Create output directory if missing |
| `overwrite` | Boolean | true | — | — | Overwrite existing file |

---

#### `batch_save_images` — Batch Save Images

Save multiple images with sequential numbering.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `images` | Input | Array\<Image\> | Images to save |
| `paths` | Output | Array\<String\> | Written file paths |
| `count` | Output | Integer | Number of files written |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `directory` | String | `"./output"` | — | Output directory |
| `prefix` | String | `"image_"` | — | Filename prefix |
| `format` | String | `"png"` | — | Image format |
| `start_index` | Integer | 1 | 0 – 9999 | First sequence number |
| `digits` | Integer | 3 | 1 – 6 | Zero-padded digit count (e.g., 3 → `001`) |
| `quality` | Integer | 90 | 1 – 100 | Compression quality |
| `create_dirs` | Boolean | true | — | Create output directory if missing |

---

### Utility

#### `preview` — Preview

Display image dimensions and metadata. Passes the image through unchanged.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Same image (passthrough) |
| `width` | Output | Integer | Image width |
| `height` | Output | Integer | Image height |
| `has_alpha` | Output | Boolean | Whether image has alpha channel |
| `info` | Output | String | Human-readable info string |

---

#### `image_info` — Image Info

Extract detailed metadata from an image.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `width` | Output | Integer | Width in pixels |
| `height` | Output | Integer | Height in pixels |
| `channels` | Output | Integer | Number of color channels |
| `has_alpha` | Output | Boolean | Has alpha channel |
| `pixel_count` | Output | Integer | Total pixel count |
| `aspect_ratio` | Output | Float | Width / Height ratio |

*Category: Analyze*

---

#### `image_preview` — Image Preview

Generate a thumbnail suitable for UI display, plus metadata.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source image |
| `image` | Output | Image | Same image (passthrough) |
| `thumbnail` | Output | String | Base64-encoded PNG thumbnail |
| `width` | Output | Integer | Original width |
| `height` | Output | Integer | Original height |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `max_size` | Integer | 200 | 50 – 400 | Maximum thumbnail dimension |

---

#### `split_channels` — Split Channels

Split an RGBA image into four separate grayscale channel images.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Source RGBA image |
| `red` | Output | Image | Red channel |
| `green` | Output | Image | Green channel |
| `blue` | Output | Image | Blue channel |
| `alpha` | Output | Image | Alpha channel |

---

#### `merge_channels` — Merge Channels

Recombine separate channel images back into a single RGBA image.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `red` | Input | Image | Red channel |
| `green` | Input | Image | Green channel |
| `blue` | Input | Image | Blue channel |
| `alpha` | Input | Image (optional) | Alpha channel |
| `image` | Output | Image | Merged RGBA image |

---

#### `collect_images` — Collect Images

Gather 1–4 individual images into a single array.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image1` | Input | Image | First image (required) |
| `image2` | Input | Image (optional) | Second image |
| `image3` | Input | Image (optional) | Third image |
| `image4` | Input | Image (optional) | Fourth image |
| `images` | Output | Array\<Image\> | Collected array |
| `count` | Output | Integer | Number of images collected |

---

#### `get_image_from_array` — Get Image From Array

Extract a single image from an array by index.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `images` | Input | Array\<Image\> | Image array |
| `image` | Output | Image | Extracted image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `index` | Integer | 0 | 0 – 100 | Zero-based index |

---

#### `array_length` — Array Length

Return the number of items in any array.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `array` | Input | Any | Input array |
| `length` | Output | Integer | Item count |

---

#### `value_display` — Value Display

Debug utility — display any value's content and type.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `value` | Input | Any | Value to inspect |
| `value` | Output | Any | Same value (passthrough) |
| `display` | Output | String | Human-readable representation |
| `type` | Output | String | Type name |

---

#### `note` — Note

Graph annotation node — no processing, just text for documenting your workflow.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `text` | String | `"Add your notes here..."` | Note content |

---

### Constants

Nodes that output fixed constant values for use as graph parameters.

#### `integer_constant` — Integer

| Output | Type | | Parameter | Type | Default |
|--------|------|-|-----------|------|---------|
| `value` | Integer | | `value` | Integer | 0 |

#### `float_constant` — Float

| Output | Type | | Parameter | Type | Default |
|--------|------|-|-----------|------|---------|
| `value` | Float | | `value` | Float | 0.0 |

#### `string_constant` — String

| Output | Type | | Parameter | Type | Default | UI |
|--------|------|-|-----------|------|---------|-----|
| `value` | String | | `value` | String | `""` | Multiline TextInput |

#### `boolean_constant` — Boolean

| Output | Type | | Parameter | Type | Default |
|--------|------|-|-----------|------|---------|
| `value` | Boolean | | `value` | Boolean | false |

#### `color_constant` — Color

| Output | Type | | Parameter | Type | Default |
|--------|------|-|-----------|------|---------|
| `value` | Color | | `value` | Color | rgba(255, 255, 255, 255) |

---

### Type Conversion

Nodes for converting values between types.

| Node ID | Name | Input | Output |
|---------|------|-------|--------|
| `to_integer` | To Integer | `value` (Any) | `result` (Integer) |
| `to_float` | To Float | `value` (Any) | `result` (Float) |
| `to_string` | To String | `value` (Any) | `result` (String) |
| `to_boolean` | To Boolean | `value` (Any) | `result` (Boolean) |

---

### Math

All math nodes take two Float inputs (`a`, `b`) and produce a Float output (`result`), unless otherwise noted.

| Node ID | Name | Operation |
|---------|------|-----------|
| `add` | Add | a + b |
| `subtract` | Subtract | a − b |
| `multiply` | Multiply | a × b |
| `divide` | Divide | a ÷ b |
| `modulo` | Modulo | a mod b |
| `power` | Power | a^b |
| `min` | Min | min(a, b) |
| `max` | Max | max(a, b) |

#### `clamp` — Clamp

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `value` | Input | Float | Value to clamp |
| `min` | Input | Float | Lower bound |
| `max` | Input | Float | Upper bound |
| `result` | Output | Float | Clamped value |

---

### Comparison & Logic

#### Comparison (Float → Boolean)

All comparison nodes take two Float inputs (`a`, `b`) and produce a Boolean `result`.

| Node ID | Name | Operation |
|---------|------|-----------|
| `equal` | Equal | a == b |
| `not_equal` | Not Equal | a != b |
| `less_than` | Less Than | a < b |
| `less_than_or_equal` | Less Than or Equal | a <= b |
| `greater_than` | Greater Than | a > b |
| `greater_than_or_equal` | Greater Than or Equal | a >= b |

#### Logic (Boolean → Boolean)

| Node ID | Name | Inputs | Operation |
|---------|------|--------|-----------|
| `and` | And | `a`, `b` (Boolean) | a && b |
| `or` | Or | `a`, `b` (Boolean) | a \|\| b |
| `not` | Not | `value` (Boolean) | !value |
| `xor` | Xor | `a`, `b` (Boolean) | a ^ b |

---

### Array Operations

#### `array_map` — Array Map

Pass through an image array with count. Serves as a routing / identity node for arrays.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `images` | Input | Any | Image array |
| `images` | Output | Any | Same array |
| `count` | Output | Integer | Array length |

---

#### `array_filter` — Array Filter

Filter images by minimum dimensions.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `images` | Input | Array\<Image\> | Input images |
| `images` | Output | Array\<Image\> | Filtered images |
| `count` | Output | Integer | Remaining count |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `min_width` | Integer | 0 | 0 – 10000 | Minimum width to keep |
| `min_height` | Integer | 0 | 0 – 10000 | Minimum height to keep |

---

#### `array_concat` — Array Concat

Concatenate up to three image arrays.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `images1` | Input | Any | First array (required) |
| `images2` | Input | Any (optional) | Second array |
| `images3` | Input | Any (optional) | Third array |
| `images` | Output | Array\<Image\> | Concatenated result |
| `count` | Output | Integer | Total item count |

---

#### `array_slice` — Array Slice

Extract a sub-range from an image array.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `images` | Input | Array\<Image\> | Source array |
| `images` | Output | Array\<Image\> | Sliced result |
| `count` | Output | Integer | Slice length |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `start` | Integer | 0 | 0 – 1000 | Start index (inclusive) |
| `end` | Integer | -1 | -1 – 1000 | End index (exclusive, -1 = end of array) |

---

### Batch Processing

All batch nodes implement the `BatchAware` trait with `BatchMode::Parallel` and use Rayon for multi-threaded execution. Each accepts either a single image or an array and returns the same shape.

| Node ID | Name | Category | Parameters |
|---------|------|----------|------------|
| `batch_brightness` | Batch Brightness | Adjust | `brightness` (Float, default 0.0, range -1.0 – 1.0) |
| `batch_contrast` | Batch Contrast | Adjust | `contrast` (Float, default 1.0, range 0.0 – 2.0) |
| `batch_saturation` | Batch Saturation | Adjust | `saturation` (Float, default 1.0, range 0.0 – 2.0) |
| `batch_grayscale` | Batch Grayscale | Adjust | *(none)* |
| `batch_invert` | Batch Invert | Adjust | *(none)* |
| `batch_gaussian_blur` | Batch Gaussian Blur | Blur | `sigma` (Float, default 1.0, range 0.1 – 10.0) |
| `batch_resize` | Batch Resize | Transform | `width` (Integer, default 800, range 1 – 10000), `height` (Integer, default 600, range 1 – 10000) |
| `batch_rotate` | Batch Rotate | Transform | `angle` (Float, default 90.0) |
| `batch_crop` | Batch Crop | Transform | `x` (Integer, default 0, range 0 – 10000), `y` (Integer, default 0, range 0 – 10000), `width` (Integer, default 100, range 1 – 10000), `height` (Integer, default 100, range 1 – 10000) |
| `batch_flip` | Batch Flip | Transform | `direction` (String, default `"horizontal"`) |

**Common ports for all batch nodes:**

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `images` | Input | Any | Single image or image array |
| `images` | Output | Any | Processed image(s) |

---

### ComfyUI Integration

Nodes that communicate with a ComfyUI server via REST API. Requires a running ComfyUI instance (default: `http://127.0.0.1:8188`).

#### `comfy_checkpoint_loader` — ComfyUI Checkpoint Loader

Load a Stable Diffusion checkpoint model.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `model_ref` | Output | String | Model reference token |
| `clip_ref` | Output | String | CLIP reference token |
| `vae_ref` | Output | String | VAE reference token |

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `comfyui_url` | String | `"http://127.0.0.1:8188"` | ComfyUI server URL |
| `checkpoint_name` | String | `"v1-5-pruned-emaonly.safetensors"` | Checkpoint filename |

---

#### `comfy_clip_text_encode` — ComfyUI CLIP Text Encode

Encode a text prompt into CLIP conditioning.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `clip_ref` | Input | String | CLIP model reference |
| `conditioning` | Output | String | Encoded conditioning token |

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `text` | String | `"a beautiful landscape, high quality"` | Text prompt |

---

#### `comfy_ksampler` — ComfyUI KSampler

Core sampling / denoising step for Stable Diffusion generation.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `model_ref` | Input | String | Model reference |
| `positive` | Input | String | Positive conditioning |
| `negative` | Input | String (optional) | Negative conditioning |
| `latent_ref` | Output | String | Generated latent reference |

| Parameter | Type | Default | Range | Options | Description |
|-----------|------|---------|-------|---------|-------------|
| `comfyui_url` | String | `"http://127.0.0.1:8188"` | — | — | Server URL |
| `seed` | Integer | 0 | 0 – 2,147,483,647 | — | Random seed |
| `steps` | Integer | 20 | 1 – 150 | — | Sampling steps |
| `cfg_scale` | Float | 7.0 | 1.0 – 30.0 | — | Classifier-free guidance scale |
| `sampler_name` | String | `"euler"` | — | `euler`, `euler_ancestral`, `heun`, `heunpp2`, `dpm_2`, `dpm_2_ancestral`, `lms`, `dpm_fast`, `dpm_adaptive`, `dpmpp_2s_ancestral`, `dpmpp_2m` | Sampler algorithm |
| `scheduler` | String | `"normal"` | — | `normal`, `karras`, `exponential`, `sgm_uniform`, `simple`, `ddim_uniform` | Noise schedule |
| `denoise` | Float | 1.0 | 0.0 – 1.0 | — | Denoising strength |
| `width` | Integer | 512 | 64 – 2048 | — | Output width |
| `height` | Integer | 512 | 64 – 2048 | — | Output height |
| `batch_size` | Integer | 1 | 1 – 16 | — | Number of images per batch |
| `timeout_secs` | Integer | 300 | 10 – 3600 | — | Operation timeout |

---

#### `comfy_vae_decode` — ComfyUI VAE Decode

Decode a latent representation into pixel-space using a VAE.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `latent_ref` | Input | String | Latent reference from KSampler |
| `vae_ref` | Input | String (optional) | VAE reference (uses default if empty) |
| `image` | Output | Image | Decoded pixel image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `comfyui_url` | String | `"http://127.0.0.1:8188"` | — | Server URL |
| `timeout_secs` | Integer | 300 | — | Operation timeout |

---

#### `comfy_lora_loader` — ComfyUI LoRA Loader

Apply a LoRA (Low-Rank Adaptation) model to modify the base model and CLIP.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `model_ref` | Input | String | Base model reference |
| `clip_ref` | Input | String | CLIP reference |
| `model_ref` | Output | String | LoRA-modified model reference |
| `clip_ref` | Output | String | LoRA-modified CLIP reference |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `comfyui_url` | String | `"http://127.0.0.1:8188"` | — | Server URL |
| `lora_name` | String | — | — | LoRA filename |
| `model_strength` | Float | 1.0 | -2.0 – 2.0 | Model weight influence |
| `clip_strength` | Float | 1.0 | -2.0 – 2.0 | CLIP weight influence |

---

#### `comfy_image_upscale` — ComfyUI Image Upscale

Upscale an image using an AI upscaling model via ComfyUI.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Low-resolution input |
| `image` | Output | Image | Upscaled result |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `comfyui_url` | String | `"http://127.0.0.1:8188"` | — | Server URL |
| `upscale_model` | String | `"RealESRGAN_x4plus.pth"` | — | Upscale model name |
| `timeout_secs` | Integer | 300 | — | Operation timeout |

---

#### `comfy_controlnet_apply` — ComfyUI ControlNet Apply

Apply a ControlNet conditioning model to guide image generation.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `conditioning` | Input | String | Existing conditioning |
| `control_image` | Input | Image | Control image (e.g., depth map, edge map) |
| `conditioning` | Output | String | ControlNet-augmented conditioning |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `controlnet_name` | String | — | — | ControlNet model name |
| `strength` | Float | 1.0 | 0.0 – 2.0 | ControlNet influence strength |

---

#### `comfy_workflow_runner` — ComfyUI Workflow Runner

Execute an arbitrary ComfyUI workflow from raw JSON. Maximum flexibility for custom pipelines.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image (optional) | Optional input image for the workflow |
| `image` | Output | Image | First output image from the workflow |
| `raw_history` | Output | String | Raw JSON history response |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `comfyui_url` | String | `"http://127.0.0.1:8188"` | — | Server URL |
| `workflow_json` | String | — | — | Complete ComfyUI workflow as JSON |
| `timeout_secs` | Integer | 600 | 10 – 7200 | Operation timeout |

---

### External API

#### `http_image_fetch` — HTTP Image Fetch

Download an image from a URL.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Output | Image | Downloaded image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `url` | String | — | — | Image URL |
| `timeout_secs` | Integer | 30 | 1 – 120 | Request timeout |

---

#### `stable_diffusion_generate` — Stable Diffusion Generate

Generate an image from a text prompt using the Automatic1111 / SD WebUI API.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Output | Image | Generated image |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `api_url` | String | `"http://127.0.0.1:7860/sdapi/v1/txt2img"` | — | SD API endpoint |
| `prompt` | String | — | — | Text prompt |
| `negative_prompt` | String | — | — | Negative prompt |
| `width` | Integer | 512 | 64 – 2048 | Image width |
| `height` | Integer | 512 | 64 – 2048 | Image height |
| `steps` | Integer | 20 | 1 – 150 | Sampling steps |
| `cfg_scale` | Float | 7.0 | 1.0 – 30.0 | CFG scale |
| `seed` | Integer | -1 | — | Seed (-1 = random) |
| `timeout_secs` | Integer | 120 | — | Request timeout |

---

#### `image_classify` — Image Classify

Classify an image via an external classification API.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Image to classify |
| `result` | Output | String | Full classification result (JSON) |
| `top_label` | Output | String | Highest-confidence class label |
| `confidence` | Output | Float | Confidence score |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `api_url` | String | — | — | Classification API endpoint |
| `timeout_secs` | Integer | 30 | — | Request timeout |

---

#### `model_inference` — Model Inference

Run inference on a generic model API endpoint. Sends an image and receives a processed image back.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `image` | Input | Image | Input image |
| `image` | Output | Image | Processed image |
| `raw_response` | Output | String | Raw API response |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `api_url` | String | — | — | Model API endpoint |
| `model_name` | String | — | — | Model identifier |
| `extra_params` | String | — | — | Additional JSON parameters |
| `timeout_secs` | Integer | 60 | — | Request timeout |

---

#### `style_transfer` — Style Transfer

Apply neural style transfer by sending content and style images to an API.

| Port | Direction | Type | Description |
|------|-----------|------|-------------|
| `content` | Input | Image | Content image |
| `style` | Input | Image | Style reference image |
| `image` | Output | Image | Stylized result |

| Parameter | Type | Default | Range | Description |
|-----------|------|---------|-------|-------------|
| `api_url` | String | — | — | Style transfer API endpoint |
| `strength` | Float | 0.8 | 0.0 – 1.0 | Style influence strength |
| `timeout_secs` | Integer | 120 | — | Request timeout |

---

## Plugin System

Ambara supports loading third-party filters at runtime through a native plugin system.

### Architecture

Plugins are compiled Rust shared libraries (`.so` / `.dll` / `.dylib`) that export a C ABI vtable. The host loads them via `libloading`, verifies ABI compatibility, and wraps each plugin filter as a `PluginFilterNode` that implements the standard `FilterNode` trait.

### Plugin Structure

Each plugin directory must contain:

1. **`ambara-plugin.toml`** — Manifest file
2. **Shared library** — Compiled `.so`/`.dll`/`.dylib`

### Manifest Format

```toml
[plugin]
id = "com.example.my-plugin"
name = "My Plugin"
version = "1.0.0"
description = "Does amazing things"
author = "Author Name <email@example.com>"
homepage = "https://example.com"
license = "MIT"
ambara_abi_version = 1
min_ambara_version = "0.3.0"
max_ambara_version = "0.99.99"

[plugin.capabilities]
network = true
filesystem_read = true
filesystem_write = false
gpu = false

[plugin.filters]
ids = ["my.filter_one", "my.filter_two"]

[plugin.config]
endpoint = "http://127.0.0.1:8188"
```

### Capability System

Plugins declare the capabilities they need. The host grants or denies each:

| Capability | Description |
|------------|-------------|
| `network` | Make outbound network requests |
| `filesystem_read` | Read files from explicitly passed paths |
| `filesystem_write` | Write files to explicitly passed paths |
| `gpu` | Access GPU resources |

> **Security note:** The capability system is advisory. Loading a plugin is equivalent to running native code. Only load plugins from trusted sources.

### Plugin Lifecycle

1. **Discovery** — `PluginRegistry::discover()` scans the plugin directory for `ambara-plugin.toml` files
2. **Manifest validation** — Checks ABI version, Ambara version compatibility, and manifest integrity
3. **Loading** — `LoadedPlugin::load()` opens the shared library, locates the `ambara_plugin_vtable` symbol
4. **Initialization** — Calls `plugin_create()` then `plugin_init()` with merged config JSON
5. **Registration** — Plugin filters are registered in the `FilterRegistry` as `PluginFilterNode` instances
6. **Health checking** — `health_check_all()` polls all loaded plugins for liveness
7. **Unloading** — `unload_plugin()` calls `plugin_destroy()` and drops the library handle

### Plugin System Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `max_plugins` | 64 | Maximum simultaneously loaded plugins |
| `auto_load` | false | Auto-load all plugins on startup |
| `host_config` | `{}` | JSON config passed to all plugins on init |

---

## GPU Acceleration

Three filters support GPU acceleration via wgpu:

- **Gaussian Blur** (`gaussian_blur`)
- **Grayscale** (`grayscale`)
- **Invert Colors** (`invert`)

These filters implement the `GpuAccelerated` trait and use `GpuPool::global()` for device management. GPU execution is enabled by default (parameter `use_gpu = true`) and falls back to CPU automatically if no compatible GPU is available.

---

## User Interface

Ambara ships with a **Tauri + React** desktop application built with TypeScript and Vite.

### Components

| Component | Purpose |
|-----------|---------|
| **GraphCanvas** | Visual node graph editor — drag, connect, validate, execute, save/load workflows |
| **FilterPalette** | Searchable sidebar listing all available filters by category; drag to add |
| **PropertiesPanel** | Inspect and edit parameters of the currently selected node |
| **PluginPanel** | Browse, load, unload, and refresh plugins |
| **ChatPanel** | AI chat interface with markdown rendering, syntax highlighting, image attachment, and graph-insertion capability |
| **FilterNode** | Visual representation of a filter node in the canvas |
| **PreviewNode** | Image preview display within the graph |
| **ValueDisplayNode** | Debug value display within the graph |
| **Settings** | Application settings panel |
| **ConfirmDialog** | Confirmation dialog component |
| **Toast** | Notification toast component |

### Key UI Features

- **Visual node graph** with drag-and-drop node placement and wire connections
- **Real-time validation** and execution feedback
- **Save / Load** graph workflows as JSON
- **Chat panel** with AI-assisted graph building, markdown rendering, code syntax highlighting, and image attachment
- **Automation pipelines** — chatbot can build, validate, and execute pipelines end-to-end
- **Image ingestion** — attach image files via native Tauri file dialog for pipeline inputs
- **Plugin management** UI for loading/unloading third-party filter plugins
- **Live image preview** nodes with thumbnail generation

---

## AI Chatbot System

Ambara includes a **generative AI chatbot** powered by a ReAct (Reason + Act) agentic pipeline. The chatbot translates natural language into validated image-processing graphs.

### Architecture

```
User Message → WebSocket → FastAPI → Agent (ReAct Loop) → Tool Calls → Response
                                         │
                                    LLM (Ollama/OpenAI/Anthropic/Groq)
                                         │
                              ┌──────────┼──────────┐
                              │          │          │
                         CodeRetriever  GraphGen  GraphValidator
                         (Code-as-RAG) (Pipeline) (Topology Check)
```

### Agent Tools

| Tool | Purpose |
|------|---------|
| `search_filters` | Keyword search across the filter library |
| `get_filter_details` | Detailed info on a specific filter (ports, params, constraints) |
| `list_categories` | List all filter categories |
| `get_compatible_filters` | Find filters that can connect after a given filter |
| `generate_graph` | Generate a processing graph from natural language |
| `explain_filter` | Explain what a filter does with use cases |
| `suggest_pipeline` | Suggest a sequence of filters for a goal |
| `explain_graph` | Step-by-step explanation of an existing graph |
| `validate_graph` | Validate graph JSON (schema, IDs, ports, topology) |
| `set_input_image` | Register an image path for use in generated graphs |
| `execute_pipeline` | Execute a generated graph through the Ambara engine |

### Key Features

- **ReAct reasoning loop**: Agent reasons about intent, selects tools, observes results, and iterates
- **Code-as-RAG retrieval**: Parses Rust source code directly — no manually maintained corpus
- **Graph validation**: Schema compliance, filter ID verification, port type checking, cycle/orphan detection
- **Markdown rendering**: Chat responses render with full markdown, code blocks, and syntax highlighting
- **Image ingestion**: Attach image files via native OS dialog; paths flow into generated `load_image` nodes
- **Automation pipelines**: Agent can build → validate → execute pipelines end-to-end
- **Multi-backend LLM**: Supports Ollama (local), OpenAI, Anthropic, Groq
- **WebSocket streaming**: Word-by-word response streaming with typing indicator
- **Session persistence**: Chat history and session ID preserved across page reloads

### Guardrails

- **Agent timeout**: 90-second hard limit on the entire agent run
- **WebSocket timeout**: 120-second `asyncio.wait_for()` on each request
- **Response sanitization**: Leaked JSON tool-call artifacts are stripped before reaching the UI
- **Tool deduplication**: Identical tool calls within a session are skipped
- **Prompt hardening**: System prompt instructs the model to never dump raw JSON in answers

### Prompting Techniques

The chatbot implements seven prompting strategies documented in `papers/08_small_model_prompt_refining.md`:

1. **Chain-of-thought scaffolding** — 4-step reasoning (Understand → Classify → Plan → Verify)
2. **Structured persona-task-format** — Role, tools, rules, format in hierarchical sections
3. **Negative example guardrails** — Explicit "do NOT" instructions for common failure modes
4. **Self-verification loop** — `validate_graph` called after `generate_graph`
5. **Retrieval-augmented context** — Only matched filters included, not all 111
6. **Hierarchical prompt sectioning** — Tabular decision rules with priority ordering
7. **Response format anchoring** — Exact JSON templates with "EXACTLY ONE" emphasis

---

## CLI Usage

The `ambara` binary provides a headless command-line interface.

### Commands

```bash
# List all available filters
ambara list
ambara list --json

# Show detailed info for a specific filter
ambara info <filter_id>

# Process a single image with inline filters
ambara process <input> <output> [--blur <sigma>] [--brightness <amount>]

# Load and execute a graph JSON file
ambara load-graph <graph.json> --execute
ambara load-graph <graph.json> --dry-run
```

---

## Filter Count Summary

| Category | Count |
|----------|-------|
| Blur | 4 |
| Color | 10 |
| Adjust | 2 |
| Edge Detection | 4 |
| Transform | 4 |
| Composite | 2 |
| Sharpen | 2 |
| Noise | 2 |
| Draw | 3 |
| Text | 1 |
| Astrophotography | 5 |
| Input / Output | 4 |
| Utility | 10 |
| Constants | 5 |
| Type Conversion | 4 |
| Math | 8 |
| Comparison & Logic | 10 |
| Array Operations | 4 |
| Batch Processing | 10 |
| ComfyUI Integration | 8 |
| External API | 5 |
| **Total** | **~107** |

---

*Generated from source code in `src/filters/builtin/` and `src/plugins/`.*
