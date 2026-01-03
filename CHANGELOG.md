# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha.1] - 2026-01-03

### Added

#### Core Features
- Node-based image processing library with ComfyUI-style visual editor
- 28+ built-in filters across 16 categories
- Type-safe node connections with automatic validation
- Parallel execution engine for batch processing
- Graph serialization (save/load workflows)

#### Astrophotography Filters
- **Image Stack**: Combine multiple images using mean, median, sigma-clip, max, or min algorithms
- **Dark Frame Subtract**: Remove thermal noise using dark frame calibration
- **Flat Field Correct**: Remove vignetting and dust artifacts
- **Hot Pixel Removal**: Detect and remove hot/dead pixels using median filtering
- **Histogram Stretch**: Enhance faint details with adjustable black point, white point, and midtone

#### Image Preview
- **Image Preview Node**: Display thumbnails within the node graph
- Base64-encoded preview generation
- Collapsible preview area
- Shows original image dimensions

#### UI Features
- ReactFlow-based node editor
- Filter palette with search functionality
- Properties panel for parameter editing
- Connection management (auto-replace duplicate inputs)
- Edge deletion (Backspace/Delete keys)
- Clear graph button with confirmation
- File/directory dialogs for I/O operations

#### Filter Categories
- **Input**: LoadImage, LoadFolder
- **Output**: SaveImage (with directory, filename, format options)
- **Transform**: Resize, Rotate, Flip, Crop
- **Adjust**: Brightness, Contrast, Saturation
- **Blur**: GaussianBlur, BoxBlur
- **Sharpen**: Sharpen, UnsharpMask
- **Edge**: EdgeDetect, Sobel
- **Noise**: AddNoise, Denoise
- **Draw**: DrawRectangle, DrawCircle, DrawLine, DrawText
- **Text**: TextOverlay
- **Composite**: Blend, Overlay (with multiple blend modes)
- **Color**: Grayscale, Invert, HueShift, ColorBalance, Threshold
- **Analyze**: Histogram, ImageInfo
- **Math**: Add, Subtract, Multiply, Divide, Modulo, Power, Min, Max, Clamp
- **Utility**: Preview, SplitChannels, MergeChannels, Note, ImagePreview
- **Custom**: Astrophotography filters

#### Developer Features
- Comprehensive test suite (75+ tests passing)
- FilterRegistry for extensibility
- Strong typing with Rust's type system
- Documentation and examples

### Technical Details
- Rust 2021 edition
- Tauri 2.x for desktop UI
- React 19 + TypeScript
- ReactFlow for graph visualization
- Zustand for state management

### Known Limitations
- Preview nodes require execution to display thumbnails
- Sequential execution mode (parallel mode available but disabled by default)
- No undo/redo support yet

[0.1.0-alpha.1]: https://github.com/PrakyathPNayak/ambara/releases/tag/v0.1.0-alpha.1
