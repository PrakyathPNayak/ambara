# Ambara v0.1.0-alpha.1 Release Notes

**Release Date:** January 3, 2026

## 🎉 Welcome to Ambara Alpha!

Ambara is a node-based image processing application with a ComfyUI-like interface, designed for professional image workflows including astrophotography processing.

## ✨ Key Features

### Core Capabilities
- **Node-based Processing**: Create complex image processing workflows using a visual node editor
- **28+ Built-in Filters**: Comprehensive filter library across 16 categories
- **Type-Safe Connections**: Automatic validation ensures correct data flow between nodes
- **Parallel Execution**: High-performance image processing with batch support
- **Workflow Persistence**: Save and load your processing graphs as JSON files

### Astrophotography Tools
- **Image Stacking**: Combine multiple exposures using multiple algorithms (mean, median, sigma-clip, max, min)
- **Dark Frame Subtraction**: Remove thermal noise with dark frame calibration
- **Flat Field Correction**: Eliminate vignetting and dust artifacts
- **Hot Pixel Removal**: Detect and remove stuck/hot pixels
- **Histogram Stretching**: Enhance faint details with advanced controls

### Image Preview
- **Live Thumbnails**: See preview thumbnails directly in the node graph
- **Real-time Updates**: Previews update as you execute the graph
- **Dimension Display**: View original image dimensions alongside previews

### User Interface
- **Filter Palette**: Search and browse 28+ filters organized by category
- **Properties Panel**: Edit node parameters in real-time
- **Intuitive Controls**: 
  - Delete connections with Backspace/Delete keys
  - Clear entire graph with confirmation
  - File dialogs for image import/export

## 📦 What's Included

### Filter Categories

#### Input/Output
- LoadImage, LoadFolder, SaveImage (with customizable format, quality, and output path)

#### Transformation
- Resize, Rotate, Flip, Crop with full control

#### Color Adjustments
- Brightness, Contrast, Saturation, Grayscale, Invert

#### Blur & Sharpen
- GaussianBlur, BoxBlur, Sharpen, UnsharpMask

#### Edge Detection
- EdgeDetect, Sobel

#### Noise Processing
- AddNoise, Denoise

#### Drawing & Text
- DrawRectangle, DrawCircle, DrawLine, DrawText with color/size options

#### Composite Operations
- Blend, Overlay with adjustable opacity

#### Utility
- Preview, SplitChannels, MergeChannels, Note, ImageInfo, ImagePreview

#### Analysis
- ImageInfo for detailed image statistics

#### Mathematical Operations
- Add, Subtract, Multiply, Divide, Modulo, Power, Min, Max, Clamp

#### Comparison & Logic
- Equal, NotEqual, Less, Greater, And, Or, Not

#### Type Conversion
- ToInteger, ToFloat, ToString, ToBoolean

#### Astrophotography (NEW!)
- ImageStack, DarkFrameSubtract, FlatFieldCorrect, HotPixelRemoval, HistogramStretch

## 🚀 Getting Started

1. **Launch Ambara**: Open the application from your applications menu
2. **Load an Image**: Use LoadImage node from the Input category
3. **Add Filters**: Click on filters in the palette to add them to your workflow
4. **Connect Nodes**: Drag connections between output and input ports
5. **Execute**: Click "Execute" to process your image
6. **Save Results**: Use SaveImage node to export your processed image
7. **Save Workflow**: Use File → Save to store your entire workflow for later

## 🎬 Example Workflows

### Basic Image Enhancement
```
LoadImage → Brightness → Contrast → Saturation → SaveImage
```

### Astrophotography Processing
```
LoadFolder → ImageStack (median) → DarkFrameSubtract → FlatFieldCorrect → 
HistogramStretch → ImagePreview → SaveImage
```

### Edge Detection & Blur
```
LoadImage → EdgeDetect → GaussianBlur → SaveImage
```

## 🐛 Known Limitations

- Alpha release - expect rough edges and occasional bugs
- Large image batches (100+ images) may use significant RAM
- Some advanced features from the roadmap not yet implemented
- No undo/redo yet (planned for v0.2)

## 📋 System Requirements

- **OS**: Linux (primary), Windows and macOS in progress
- **RAM**: 4GB minimum, 8GB+ recommended for batch processing
- **Storage**: 200MB for application + space for your image files

## 🔮 Roadmap

- **v0.2.0**: Undo/redo support, image comparison view, advanced histogram tools
- **v0.3.0**: Custom node creation, JavaScript scripting support
- **v0.4.0**: GPU acceleration, real-time preview in node graph
- **v1.0.0**: Production-ready release with stability guarantees

## 🤝 Contributing

Contributions welcome! The project is open source on GitHub:
https://github.com/PrakyathPNayak/ambara

## 📄 License

MIT License - See LICENSE file for details

## 💬 Feedback

Have questions, found bugs, or have feature suggestions? 
- Open an issue on GitHub
- Check the documentation at the project wiki
- Join the community discussions

---

**Thank you for trying Ambara!** Your feedback helps us build a better image processing tool.
