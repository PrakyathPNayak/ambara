# Ambara v0.1.2 Release Notes

**Release Date:** January 4, 2026

## 🎨 New Features

### Batch Save Images Node
Save multiple images at once with intelligent filename management:
- **Auto-incrementing filenames** with configurable padding (e.g., `image_001.png`, `image_002.png`, `image_003.png`)
- **Customizable prefix** for organized output
- **Multi-format support**: PNG, JPG, WebP, BMP, TIFF
- **Quality control** for lossy formats (JPEG/WebP)
- **Outputs**: Returns array of saved paths and total count for verification

### Output Value Display
Non-image output values now appear directly in nodes after execution:
- **Numbers**: Formatted with 2 decimal places
- **Booleans**: Shown as ✓ or ✗
- **Strings**: Truncated if too long with ellipsis
- **Arrays**: Displays item count `[n]`
- Values appear in **green badges** next to output ports
- Images excluded to keep nodes compact

## 🎯 Improvements

### Enhanced Node Colors
Nodes now have **distinct solid background colors** by category for superior minimap visibility:
- 🟢 **Input**: Green (#2d4a2d)
- 🔴 **Output**: Red (#4a2d2d)
- 🔵 **Transform**: Blue (#2d3a4a)
- 🩷 **Color/Adjust**: Pink (#4a2d3d)
- 🟣 **Blur/Sharpen/Edge/Noise**: Purple (#3d2d4a)
- ⚫ **Draw/Text/Utility**: Gray (#35414a)
- 🔷 **Math**: Cyan (#2d3d4a)
- 🟠 **Composite/Analyze**: Orange (#4a3d2d)

#### Blur & Sharpen
- GaussianBlur, BoxBlur, Sharpen, UnsharpMask

#### Edge Detection
- EdgeDetect, Sobel

#### Noise Processing
- AddNoise, Denoise

#### Drawing & Text
- DrawRectangle, DrawCircle, DrawLine, DrawText with color/size options

#### Composite Operations
### Zoom Capability
- **10x increased zoom out** range (minZoom: 0.05 vs previous 0.5)
- MaxZoom set to 4 for better control
- Perfect for viewing large, complex graphs

### Load Folder Improvements
- Parameter renamed from `path` to `directory` for clearer UI
- **Automatically opens directory picker** dialog
- Better integration with file system browser
- Maintains recursive search and pattern matching features

### Minimap Enhancements
- All category colors now properly mapped and visible
- Improved node stroke styling for better visibility
- Category colors clearly visible at a glance
- Easier navigation in complex graphs

## 🐛 Bug Fixes

- Fixed compilation errors in batch save implementation
- Corrected type handling for array inputs
- Improved error messages for batch operations
- Fixed minimap colors to match all node categories
- Updated Linux dependencies for modern Ubuntu versions

## 📝 Technical Details

- Version changed to numeric format (0.1.2) for MSI bundler compatibility
- All changes tested and compiled successfully (release build)
- Frontend and backend version synchronized
- CHANGELOG.md updated with detailed change history

## 📦 Installation

Download the appropriate binary for your platform from the assets below, or build from source:

```bash
git clone https://github.com/PrakyathPNayak/ambara.git
cd ambara
cargo build --release
```

## 🔗 Links

- [Full Changelog](https://github.com/PrakyathPNayak/ambara/compare/v0.1.0-alpha.1...v0.1.2)
- [Documentation](https://github.com/PrakyathPNayak/ambara#readme)
- [Report Issues](https://github.com/PrakyathPNayak/ambara/issues)
