//! Chunked/tiled image processing for larger-than-memory images.
//!
//! This module provides infrastructure for processing images that are too large
//! to fit in memory by dividing them into tiles and processing one tile at a time.
//!
//! # Architecture
//!
//! The chunked processing system uses a tile-based approach:
//! - Images are divided into rectangular tiles
//! - Each tile is loaded, processed, and written back
//! - Overlap regions handle spatial filters (blur, convolution)
//! - Memory usage is bounded by the configured memory limit
//!
//! # Example
//!
//! ```ignore
//! let config = ProcessingConfig::new()
//!     .with_memory_limit(500 * 1024 * 1024)  // 500MB
//!     .with_tile_size(256, 256);
//!
//! let result = process_chunked(&image_path, &filter, config)?;
//! ```

use crate::core::error::ExecutionError;
use crate::core::types::{ImageMetadata, ImageValue, ImageFormat};
use crate::core::error::NodeId;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Default memory limit (500 MB)
pub const DEFAULT_MEMORY_LIMIT: usize = 500 * 1024 * 1024;

/// Minimum tile size in pixels
pub const MIN_TILE_SIZE: u32 = 64;

/// Maximum tile size in pixels
pub const MAX_TILE_SIZE: u32 = 4096;

/// Represents a rectangular region within an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRegion {
    /// X offset from the image origin
    pub x: u32,
    /// Y offset from the image origin
    pub y: u32,
    /// Width of the region
    pub width: u32,
    /// Height of the region
    pub height: u32,
}

impl TileRegion {
    /// Create a new tile region.
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Get the right edge coordinate (exclusive).
    pub fn right(&self) -> u32 {
        self.x + self.width
    }

    /// Get the bottom edge coordinate (exclusive).
    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }

    /// Calculate the area of this region in pixels.
    pub fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Expand this region by the given overlap amount, clamping to image bounds.
    pub fn expand_with_overlap(&self, overlap: u32, image_width: u32, image_height: u32) -> Self {
        let new_x = self.x.saturating_sub(overlap);
        let new_y = self.y.saturating_sub(overlap);
        let new_right = (self.right() + overlap).min(image_width);
        let new_bottom = (self.bottom() + overlap).min(image_height);
        
        Self {
            x: new_x,
            y: new_y,
            width: new_right - new_x,
            height: new_bottom - new_y,
        }
    }

    /// Check if this region is entirely within the given bounds.
    pub fn is_within_bounds(&self, width: u32, height: u32) -> bool {
        self.right() <= width && self.bottom() <= height
    }
}

/// Configuration for chunked image processing.
#[derive(Debug, Clone)]
pub struct ProcessingConfig {
    /// Maximum memory to use in bytes.
    pub memory_limit: usize,
    /// Preferred tile width.
    pub tile_width: u32,
    /// Preferred tile height.
    pub tile_height: u32,
    /// Overlap for spatial filters (e.g., blur radius).
    pub overlap: u32,
    /// Whether to process tiles in parallel.
    pub parallel: bool,
    /// Number of worker threads (0 = auto).
    pub num_threads: usize,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            memory_limit: DEFAULT_MEMORY_LIMIT,
            tile_width: 512,
            tile_height: 512,
            overlap: 0,
            parallel: true,
            num_threads: 0,
        }
    }
}

impl ProcessingConfig {
    /// Create a new processing configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the memory limit in bytes.
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = limit;
        self
    }

    /// Set the memory limit in megabytes.
    pub fn with_memory_limit_mb(mut self, mb: usize) -> Self {
        self.memory_limit = mb * 1024 * 1024;
        self
    }

    /// Set the tile size.
    pub fn with_tile_size(mut self, width: u32, height: u32) -> Self {
        self.tile_width = width.clamp(MIN_TILE_SIZE, MAX_TILE_SIZE);
        self.tile_height = height.clamp(MIN_TILE_SIZE, MAX_TILE_SIZE);
        self
    }

    /// Set the overlap for spatial filters.
    pub fn with_overlap(mut self, overlap: u32) -> Self {
        self.overlap = overlap;
        self
    }

    /// Enable or disable parallel processing.
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Set the number of worker threads.
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.num_threads = threads;
        self
    }

    /// Calculate optimal tile size based on memory limit and image dimensions.
    pub fn calculate_optimal_tile_size(&self, image_width: u32, image_height: u32) -> (u32, u32) {
        // Each pixel takes 4 bytes (RGBA)
        const BYTES_PER_PIXEL: usize = 4;
        
        // We need memory for: input tile + output tile + overlap buffer
        // Assuming 3x multiplier for working memory
        let working_memory_factor = 3;
        
        let max_tile_pixels = self.memory_limit / (BYTES_PER_PIXEL * working_memory_factor);
        
        // If image fits entirely in memory, return full image dimensions
        let total_pixels = (image_width as usize) * (image_height as usize);
        if total_pixels <= max_tile_pixels {
            return (image_width, image_height);
        }

        // Calculate tile size that fits in memory while maintaining aspect ratio
        let aspect = self.tile_width as f64 / self.tile_height as f64;
        let tile_height = ((max_tile_pixels as f64 / aspect).sqrt() as u32)
            .clamp(MIN_TILE_SIZE, MAX_TILE_SIZE);
        let tile_width = ((tile_height as f64 * aspect) as u32)
            .clamp(MIN_TILE_SIZE, MAX_TILE_SIZE);

        (tile_width, tile_height)
    }

    /// Check if an image needs chunked processing.
    pub fn needs_chunking(&self, image_width: u32, image_height: u32) -> bool {
        const BYTES_PER_PIXEL: usize = 4;
        let image_size = (image_width as usize) * (image_height as usize) * BYTES_PER_PIXEL;
        // Need chunking if image is larger than half the memory limit
        // (to leave room for output)
        image_size > self.memory_limit / 2
    }
}

/// Spatial extent required by a filter for proper processing.
///
/// Spatial filters (blur, sharpen, edge detection) need additional
/// pixels around each tile to compute correct results at tile boundaries.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpatialExtent {
    /// Pixels needed to the left.
    pub left: u32,
    /// Pixels needed to the right.
    pub right: u32,
    /// Pixels needed above.
    pub top: u32,
    /// Pixels needed below.
    pub bottom: u32,
}

impl SpatialExtent {
    /// Create a symmetric extent (same on all sides).
    pub fn symmetric(radius: u32) -> Self {
        Self {
            left: radius,
            right: radius,
            top: radius,
            bottom: radius,
        }
    }

    /// Create an asymmetric extent.
    pub fn asymmetric(left: u32, right: u32, top: u32, bottom: u32) -> Self {
        Self { left, right, top, bottom }
    }

    /// Get the maximum extent in any direction.
    pub fn max_extent(&self) -> u32 {
        self.left.max(self.right).max(self.top).max(self.bottom)
    }

    /// Check if this filter requires any overlap.
    pub fn needs_overlap(&self) -> bool {
        self.left > 0 || self.right > 0 || self.top > 0 || self.bottom > 0
    }

    /// Combine with another extent (take maximum of each).
    pub fn combine(&self, other: &SpatialExtent) -> SpatialExtent {
        SpatialExtent {
            left: self.left.max(other.left),
            right: self.right.max(other.right),
            top: self.top.max(other.top),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

/// Iterator over tiles in an image.
pub struct TileIterator {
    image_width: u32,
    image_height: u32,
    tile_width: u32,
    tile_height: u32,
    current_x: u32,
    current_y: u32,
}

impl TileIterator {
    /// Create a new tile iterator.
    pub fn new(image_width: u32, image_height: u32, tile_width: u32, tile_height: u32) -> Self {
        Self {
            image_width,
            image_height,
            tile_width,
            tile_height,
            current_x: 0,
            current_y: 0,
        }
    }

    /// Get the total number of tiles.
    pub fn tile_count(&self) -> usize {
        let tiles_x = (self.image_width + self.tile_width - 1) / self.tile_width;
        let tiles_y = (self.image_height + self.tile_height - 1) / self.tile_height;
        (tiles_x * tiles_y) as usize
    }
}

impl Iterator for TileIterator {
    type Item = TileRegion;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_y >= self.image_height {
            return None;
        }

        let x = self.current_x;
        let y = self.current_y;
        let width = (self.tile_width).min(self.image_width - x);
        let height = (self.tile_height).min(self.image_height - y);

        // Move to next tile
        self.current_x += self.tile_width;
        if self.current_x >= self.image_width {
            self.current_x = 0;
            self.current_y += self.tile_height;
        }

        Some(TileRegion::new(x, y, width, height))
    }
}

/// Memory tracker for chunked processing.
#[derive(Debug)]
pub struct MemoryTracker {
    /// Current memory usage in bytes.
    current: AtomicUsize,
    /// Peak memory usage in bytes.
    peak: AtomicUsize,
    /// Memory limit in bytes.
    limit: usize,
}

impl MemoryTracker {
    /// Create a new memory tracker with the given limit.
    pub fn new(limit: usize) -> Self {
        Self {
            current: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            limit,
        }
    }

    /// Try to allocate memory. Returns true if successful.
    pub fn try_allocate(&self, bytes: usize) -> bool {
        let mut current = self.current.load(Ordering::Relaxed);
        loop {
            if current + bytes > self.limit {
                return false;
            }
            match self.current.compare_exchange_weak(
                current,
                current + bytes,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // Update peak if necessary
                    let new_current = current + bytes;
                    let mut peak = self.peak.load(Ordering::Relaxed);
                    while new_current > peak {
                        match self.peak.compare_exchange_weak(
                            peak,
                            new_current,
                            Ordering::SeqCst,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => break,
                            Err(p) => peak = p,
                        }
                    }
                    return true;
                }
                Err(c) => current = c,
            }
        }
    }

    /// Release allocated memory.
    pub fn release(&self, bytes: usize) {
        self.current.fetch_sub(bytes, Ordering::SeqCst);
    }

    /// Get current memory usage.
    pub fn current_usage(&self) -> usize {
        self.current.load(Ordering::Relaxed)
    }

    /// Get peak memory usage.
    pub fn peak_usage(&self) -> usize {
        self.peak.load(Ordering::Relaxed)
    }

    /// Get remaining available memory.
    pub fn available(&self) -> usize {
        self.limit.saturating_sub(self.current.load(Ordering::Relaxed))
    }

    /// Get the memory limit.
    pub fn limit(&self) -> usize {
        self.limit
    }
}

/// A tile buffer for processing.
#[derive(Debug)]
pub struct TileBuffer {
    /// The image data for this tile.
    pub data: ImageBuffer<Rgba<u8>, Vec<u8>>,
    /// The region this tile covers in the original image.
    pub region: TileRegion,
    /// The region with overlap (for spatial filters).
    pub overlap_region: TileRegion,
}

impl TileBuffer {
    /// Create a new tile buffer.
    pub fn new(data: ImageBuffer<Rgba<u8>, Vec<u8>>, region: TileRegion, overlap_region: TileRegion) -> Self {
        Self { data, region, overlap_region }
    }

    /// Get the memory size of this buffer in bytes.
    pub fn memory_size(&self) -> usize {
        self.data.as_raw().len()
    }

    /// Extract the core region (without overlap) from the processed tile.
    pub fn extract_core(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let offset_x = self.region.x - self.overlap_region.x;
        let offset_y = self.region.y - self.overlap_region.y;
        
        let mut result = ImageBuffer::new(self.region.width, self.region.height);
        
        for y in 0..self.region.height {
            for x in 0..self.region.width {
                let src_x = x + offset_x;
                let src_y = y + offset_y;
                if src_x < self.data.width() && src_y < self.data.height() {
                    result.put_pixel(x, y, *self.data.get_pixel(src_x, src_y));
                }
            }
        }
        
        result
    }
}

/// Trait for chunked image sources.
pub trait ChunkedImageSource: Send + Sync {
    /// Get image metadata without loading pixel data.
    fn metadata(&self) -> &ImageMetadata;

    /// Read a tile from the image.
    fn read_tile(&self, region: TileRegion) -> Result<TileBuffer, ExecutionError>;

    /// Get the file path if this is a file-based source.
    fn file_path(&self) -> Option<&Path>;
}

/// Trait for chunked image destinations.
pub trait ChunkedImageSink: Send + Sync {
    /// Initialize the destination with image metadata.
    fn initialize(&mut self, metadata: &ImageMetadata) -> Result<(), ExecutionError>;

    /// Write a tile to the destination.
    fn write_tile(&mut self, tile: &TileBuffer) -> Result<(), ExecutionError>;

    /// Finalize the destination (flush buffers, close files, etc.).
    fn finalize(&mut self) -> Result<(), ExecutionError>;
}

/// File-based chunked image source.
pub struct FileImageSource {
    path: PathBuf,
    metadata: ImageMetadata,
    image: Option<DynamicImage>,
}

impl FileImageSource {
    /// Open an image file as a chunked source.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ExecutionError> {
        let path = path.as_ref().to_path_buf();
        
        // Try to read just metadata first
        let image = image::open(&path).map_err(|e| ExecutionError::ImageProcessing(
            format!("Failed to open image '{}': {}", path.display(), e)
        ))?;
        
        let metadata = ImageMetadata {
            width: image.width(),
            height: image.height(),
            format: ImageFormat::from_path(&path),
            has_alpha: matches!(
                image,
                DynamicImage::ImageRgba8(_) | DynamicImage::ImageRgba16(_) | 
                DynamicImage::ImageRgba32F(_) | DynamicImage::ImageLumaA8(_) | 
                DynamicImage::ImageLumaA16(_)
            ),
        };
        
        Ok(Self {
            path,
            metadata,
            image: Some(image),
        })
    }

    /// Get the image metadata.
    pub fn get_metadata(&self) -> &ImageMetadata {
        &self.metadata
    }
}

impl ChunkedImageSource for FileImageSource {
    fn metadata(&self) -> &ImageMetadata {
        &self.metadata
    }

    fn read_tile(&self, region: TileRegion) -> Result<TileBuffer, ExecutionError> {
        let image = self.image.as_ref().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: NodeId::new(),
            error: "Image not loaded".to_string(),
        })?;

        // Clamp region to image bounds
        let region = TileRegion::new(
            region.x.min(self.metadata.width),
            region.y.min(self.metadata.height),
            region.width.min(self.metadata.width.saturating_sub(region.x)),
            region.height.min(self.metadata.height.saturating_sub(region.y)),
        );

        let sub_image = image.crop_imm(region.x, region.y, region.width, region.height);
        let rgba = sub_image.to_rgba8();

        Ok(TileBuffer::new(rgba, region, region))
    }

    fn file_path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}

/// In-memory chunked image source.
pub struct MemoryImageSource {
    image: Arc<DynamicImage>,
    metadata: ImageMetadata,
}

impl MemoryImageSource {
    /// Create from an existing DynamicImage.
    pub fn new(image: DynamicImage) -> Self {
        let metadata = ImageMetadata {
            width: image.width(),
            height: image.height(),
            format: ImageFormat::Unknown,
            has_alpha: matches!(
                image,
                DynamicImage::ImageRgba8(_) | DynamicImage::ImageRgba16(_) | 
                DynamicImage::ImageRgba32F(_) | DynamicImage::ImageLumaA8(_) | 
                DynamicImage::ImageLumaA16(_)
            ),
        };
        
        Self {
            image: Arc::new(image),
            metadata,
        }
    }

    /// Create from an ImageValue.
    pub fn from_image_value(value: &ImageValue) -> Result<Self, ExecutionError> {
        let image = value.get_image().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: NodeId::new(),
            error: "Image data not loaded".to_string(),
        })?;
        
        Ok(Self {
            image: Arc::new(image.clone()),
            metadata: value.metadata,
        })
    }
}

impl ChunkedImageSource for MemoryImageSource {
    fn metadata(&self) -> &ImageMetadata {
        &self.metadata
    }

    fn read_tile(&self, region: TileRegion) -> Result<TileBuffer, ExecutionError> {
        // Clamp region to image bounds
        let region = TileRegion::new(
            region.x.min(self.metadata.width),
            region.y.min(self.metadata.height),
            region.width.min(self.metadata.width.saturating_sub(region.x)),
            region.height.min(self.metadata.height.saturating_sub(region.y)),
        );

        let sub_image = self.image.crop_imm(region.x, region.y, region.width, region.height);
        let rgba = sub_image.to_rgba8();

        Ok(TileBuffer::new(rgba, region, region))
    }

    fn file_path(&self) -> Option<&Path> {
        None
    }
}

/// In-memory chunked image sink that assembles tiles into a final image.
pub struct MemoryImageSink {
    output: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    metadata: Option<ImageMetadata>,
}

impl MemoryImageSink {
    /// Create a new memory sink.
    pub fn new() -> Self {
        Self {
            output: None,
            metadata: None,
        }
    }

    /// Take the final image.
    pub fn take_image(self) -> Option<DynamicImage> {
        self.output.map(DynamicImage::ImageRgba8)
    }

    /// Get the final image as ImageValue.
    pub fn into_image_value(self) -> Option<ImageValue> {
        let _metadata = self.metadata?;
        let image = self.output?;
        
        Some(ImageValue::new(DynamicImage::ImageRgba8(image)))
    }
}

impl Default for MemoryImageSink {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkedImageSink for MemoryImageSink {
    fn initialize(&mut self, metadata: &ImageMetadata) -> Result<(), ExecutionError> {
        self.metadata = Some(*metadata);
        self.output = Some(ImageBuffer::new(metadata.width, metadata.height));
        Ok(())
    }

    fn write_tile(&mut self, tile: &TileBuffer) -> Result<(), ExecutionError> {
        let output = self.output.as_mut().ok_or_else(|| ExecutionError::NodeExecution {
            node_id: NodeId::new(),
            error: "Sink not initialized".to_string(),
        })?;

        // Extract core region and copy to output
        let core = tile.extract_core();
        
        for y in 0..tile.region.height {
            for x in 0..tile.region.width {
                let dst_x = tile.region.x + x;
                let dst_y = tile.region.y + y;
                if dst_x < output.width() && dst_y < output.height() {
                    output.put_pixel(dst_x, dst_y, *core.get_pixel(x, y));
                }
            }
        }

        Ok(())
    }

    fn finalize(&mut self) -> Result<(), ExecutionError> {
        Ok(())
    }
}

/// Process an image in chunks using a processing function.
///
/// This is the main entry point for chunked processing.
pub fn process_chunked<F>(
    source: &dyn ChunkedImageSource,
    sink: &mut dyn ChunkedImageSink,
    config: &ProcessingConfig,
    process_tile: F,
) -> Result<(), ExecutionError>
where
    F: Fn(&TileBuffer) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, ExecutionError> + Send + Sync,
{
    let metadata = source.metadata();
    
    // Initialize sink
    sink.initialize(metadata)?;

    // Calculate optimal tile size
    let (tile_width, tile_height) = config.calculate_optimal_tile_size(metadata.width, metadata.height);
    
    // Create memory tracker
    let tracker = MemoryTracker::new(config.memory_limit);
    
    // Iterate over tiles
    let tiles: Vec<_> = TileIterator::new(metadata.width, metadata.height, tile_width, tile_height).collect();
    
    for tile_region in tiles {
        // Expand region for overlap if needed
        let overlap_region = tile_region.expand_with_overlap(
            config.overlap,
            metadata.width,
            metadata.height,
        );
        
        // Read tile with overlap
        let mut tile = source.read_tile(overlap_region)?;
        tile.region = tile_region;
        tile.overlap_region = overlap_region;
        
        // Estimate memory needed
        let tile_memory = tile.memory_size() * 2; // Input + output
        if !tracker.try_allocate(tile_memory) {
            return Err(ExecutionError::OutOfMemory);
        }
        
        // Process tile
        let processed = process_tile(&tile)?;
        
        // Create processed tile buffer
        let processed_tile = TileBuffer::new(processed, tile_region, overlap_region);
        
        // Write to sink
        sink.write_tile(&processed_tile)?;
        
        // Release memory
        tracker.release(tile_memory);
    }
    
    // Finalize
    sink.finalize()?;
    
    Ok(())
}

/// Simple point-wise processing that applies a function to each pixel.
pub fn process_pointwise<F>(
    source: &dyn ChunkedImageSource,
    config: &ProcessingConfig,
    process_pixel: F,
) -> Result<ImageValue, ExecutionError>
where
    F: Fn(Rgba<u8>) -> Rgba<u8> + Send + Sync + Copy,
{
    let mut sink = MemoryImageSink::new();
    
    process_chunked(source, &mut sink, config, |tile| {
        let mut output = ImageBuffer::new(tile.data.width(), tile.data.height());
        
        for (x, y, pixel) in tile.data.enumerate_pixels() {
            output.put_pixel(x, y, process_pixel(*pixel));
        }
        
        Ok(output)
    })?;
    
    sink.into_image_value().ok_or_else(|| ExecutionError::NodeExecution {
        node_id: NodeId::new(),
        error: "Failed to produce output image".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_region() {
        let region = TileRegion::new(10, 20, 100, 200);
        assert_eq!(region.right(), 110);
        assert_eq!(region.bottom(), 220);
        assert_eq!(region.area(), 20000);
    }

    #[test]
    fn test_tile_region_expand() {
        let region = TileRegion::new(50, 50, 100, 100);
        let expanded = region.expand_with_overlap(10, 200, 200);
        assert_eq!(expanded.x, 40);
        assert_eq!(expanded.y, 40);
        assert_eq!(expanded.width, 120);
        assert_eq!(expanded.height, 120);
    }

    #[test]
    fn test_tile_region_expand_at_boundary() {
        let region = TileRegion::new(0, 0, 100, 100);
        let expanded = region.expand_with_overlap(10, 200, 200);
        assert_eq!(expanded.x, 0);
        assert_eq!(expanded.y, 0);
        assert_eq!(expanded.width, 110);
        assert_eq!(expanded.height, 110);
    }

    #[test]
    fn test_tile_iterator() {
        let iter = TileIterator::new(1000, 1000, 256, 256);
        assert_eq!(iter.tile_count(), 16); // 4x4 tiles

        let tiles: Vec<_> = TileIterator::new(100, 100, 64, 64).collect();
        assert_eq!(tiles.len(), 4); // 2x2 tiles
        
        // Check first tile
        assert_eq!(tiles[0].x, 0);
        assert_eq!(tiles[0].y, 0);
        assert_eq!(tiles[0].width, 64);
        assert_eq!(tiles[0].height, 64);
        
        // Check last tile (should be smaller)
        assert_eq!(tiles[3].x, 64);
        assert_eq!(tiles[3].y, 64);
        assert_eq!(tiles[3].width, 36);
        assert_eq!(tiles[3].height, 36);
    }

    #[test]
    fn test_processing_config() {
        let config = ProcessingConfig::new()
            .with_memory_limit_mb(512)
            .with_tile_size(256, 256);
        
        assert_eq!(config.memory_limit, 512 * 1024 * 1024);
        assert_eq!(config.tile_width, 256);
        assert_eq!(config.tile_height, 256);
    }

    #[test]
    fn test_needs_chunking() {
        let config = ProcessingConfig::new().with_memory_limit_mb(100);
        
        // Small image - no chunking needed
        assert!(!config.needs_chunking(100, 100));
        
        // Large image - chunking needed
        assert!(config.needs_chunking(10000, 10000));
    }

    #[test]
    fn test_spatial_extent() {
        let extent = SpatialExtent::symmetric(5);
        assert_eq!(extent.max_extent(), 5);
        assert!(extent.needs_overlap());

        let extent2 = SpatialExtent::asymmetric(1, 2, 3, 4);
        assert_eq!(extent2.max_extent(), 4);
        
        let combined = extent.combine(&extent2);
        assert_eq!(combined.left, 5);
        assert_eq!(combined.right, 5);
        assert_eq!(combined.top, 5);
        assert_eq!(combined.bottom, 5);
    }

    #[test]
    fn test_memory_tracker() {
        let tracker = MemoryTracker::new(1000);
        
        assert!(tracker.try_allocate(500));
        assert_eq!(tracker.current_usage(), 500);
        assert_eq!(tracker.available(), 500);
        
        assert!(tracker.try_allocate(400));
        assert_eq!(tracker.current_usage(), 900);
        
        // Should fail - not enough memory
        assert!(!tracker.try_allocate(200));
        
        tracker.release(500);
        assert_eq!(tracker.current_usage(), 400);
        assert_eq!(tracker.peak_usage(), 900);
    }

    #[test]
    fn test_calculate_optimal_tile_size() {
        let config = ProcessingConfig::new()
            .with_memory_limit_mb(100)
            .with_tile_size(512, 512);
        
        // Small image - should return full dimensions
        let (w, h) = config.calculate_optimal_tile_size(100, 100);
        assert_eq!((w, h), (100, 100));
        
        // Large image - should return constrained tile size
        let (w, h) = config.calculate_optimal_tile_size(10000, 10000);
        assert!(w <= MAX_TILE_SIZE);
        assert!(h <= MAX_TILE_SIZE);
    }
}
