# Ambara v0.2.0 Release Notes

**Release Date:** January 2026

## 🚀 Major Feature: Chunked/Tiled Processing

Process images larger than available memory with intelligent tiled execution:

### New Core Infrastructure (`src/core/chunked.rs`)
- **TileRegion**: Define rectangular regions with overlap support for seamless blending
- **ProcessingConfig**: Configure tile sizes (64-4096px), overlap, and memory limits
- **SpatialExtent**: Track image dimensions and coordinate systems
- **TileIterator**: Efficient iteration over image tiles
- **MemoryTracker**: Real-time memory monitoring with configurable thresholds (100MB - 8GB)

### Traits for Large Image Handling
- **ChunkedImageSource**: Read tiles from disk-backed images (TIFF, large PNG, etc.)
- **ChunkedImageSink**: Write processed tiles with automatic stitching
- **SpatialAwareFilter**: Filters declare spatial requirements (pointwise, neighborhood, global)

### Processing Functions
- `process_chunked()`: Full tiled processing pipeline with progress callbacks
- `process_pointwise()`: Optimized path for pixel-independent operations
- Automatic overlap calculation based on filter spatial requirements

## 🎨 New UI: Execution Settings Panel

Added a new Settings component in the sidebar with:

- **Memory Limit Slider**: Configure max memory usage (100MB - 8GB)
- **Auto-Chunk Toggle**: Enable automatic tiled processing for large images
- **Tile Size Slider**: Adjust tile dimensions (256 - 4096 pixels)
- **Parallel Execution Toggle**: Enable/disable multi-threaded processing
- **Cache Toggle**: Enable/disable intermediate result caching
- Settings persist across sessions via localStorage

## 🔧 API Enhancements

### FilterNode Trait Extensions
- `spatial_extent()`: Filters declare their spatial requirements
- `supports_chunked_processing()`: Opt-in for chunked execution

### ExecutionOptions Builder
New builder methods for memory-aware execution:
```rust
ExecutionOptions::default()
    .with_memory_limit(1024 * 1024 * 1024) // 1GB
    .with_auto_chunk(true)
    .with_tile_size(512)
```

## 📊 Test Coverage

9 comprehensive tests for chunked processing:
- Memory tracking accuracy
- Tile iteration correctness
- Overlap handling
- Configuration validation
- Edge cases (small images, single tiles)

## 📝 Documentation

- **PITCH.md**: Comprehensive pitch document for companies
- **REPO_FILE_GUIDE.md**: Documentation of all important source files

## 🐛 Technical Changes

- Updated Tauri backend with ExecutionSettings integration
- Added Zustand store with localStorage persistence for settings
- All changes compile and tests pass
- DEB and RPM packages build successfully

## 📦 Installation

Download the appropriate binary for your platform from the assets below, or build from source:

```bash
git clone https://github.com/PrakyathPNayak/ambara.git
cd ambara
cargo build --release
```

For the desktop application:
```bash
cd ui
npm install
npm run tauri build
```

## 🔗 Links

- [Full Changelog](https://github.com/PrakyathPNayak/ambara/compare/v0.1.2...v0.2.0)
- [Documentation](https://github.com/PrakyathPNayak/ambara#readme)
- [Report Issues](https://github.com/PrakyathPNayak/ambara/issues)
