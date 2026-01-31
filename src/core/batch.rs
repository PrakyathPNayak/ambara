//! Batch processing support for efficient multi-image operations.
//!
//! This module provides infrastructure for processing multiple images
//! in parallel, with automatic batching and memory management.

use crate::core::error::ExecutionError;
use crate::core::types::{ImageValue, Value};

/// Batch size configuration.
#[derive(Debug, Clone, Copy)]
pub enum BatchSize {
    /// Process all images at once
    Auto,
    /// Fixed batch size
    Fixed(usize),
    /// Dynamic based on available memory
    Dynamic,
}

impl Default for BatchSize {
    fn default() -> Self {
        BatchSize::Auto
    }
}

/// Batch processing mode for filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchMode {
    /// Filter processes images one at a time
    Sequential,
    /// Filter can process multiple images at once efficiently
    Parallel,
    /// Filter processes the entire batch as a single operation
    Batched,
}

/// Batch processing context.
///
/// Provides methods for filters to process multiple images efficiently.
#[derive(Debug, Clone)]
pub struct BatchContext {
    /// Current batch index
    pub batch_index: usize,
    /// Total number of batches
    pub total_batches: usize,
    /// Images in this batch
    pub images: Vec<ImageValue>,
    /// Batch mode being used
    pub mode: BatchMode,
}

impl BatchContext {
    /// Create a new batch context.
    pub fn new(images: Vec<ImageValue>, batch_index: usize, total_batches: usize) -> Self {
        Self {
            batch_index,
            total_batches,
            images,
            mode: BatchMode::Sequential,
        }
    }

    /// Get the number of images in this batch.
    pub fn len(&self) -> usize {
        self.images.len()
    }

    /// Check if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    /// Get progress percentage.
    pub fn progress(&self) -> f32 {
        if self.total_batches == 0 {
            1.0
        } else {
            (self.batch_index as f32 + 1.0) / self.total_batches as f32
        }
    }

    /// Split a single image input into batches.
    pub fn from_image_array(images: Vec<ImageValue>, batch_size: BatchSize) -> Vec<BatchContext> {
        let size = match batch_size {
            BatchSize::Auto => images.len(),
            BatchSize::Fixed(s) => s.max(1),
            BatchSize::Dynamic => estimate_batch_size(&images),
        };

        let total_batches = (images.len() + size - 1) / size;
        
        images
            .chunks(size)
            .enumerate()
            .map(|(i, chunk)| BatchContext::new(chunk.to_vec(), i, total_batches))
            .collect()
    }
}

/// Estimate optimal batch size based on image dimensions and available memory.
fn estimate_batch_size(images: &[ImageValue]) -> usize {
    if images.is_empty() {
        return 1;
    }

    // Estimate memory per image (bytes per pixel * dimensions)
    let first = &images[0];
    let bytes_per_image = (first.metadata.width * first.metadata.height * 4) as usize;

    // Target ~500MB per batch
    const TARGET_BATCH_MEMORY: usize = 500 * 1024 * 1024;
    
    let batch_size = (TARGET_BATCH_MEMORY / bytes_per_image).max(1);
    batch_size.min(32) // Cap at 32 images per batch
}

/// Extension trait for batch-aware filters.
pub trait BatchAware {
    /// Get the preferred batch mode for this filter.
    fn batch_mode(&self) -> BatchMode {
        BatchMode::Sequential
    }

    /// Execute on a batch of images.
    ///
    /// Only called if batch_mode() returns BatchMode::Batched.
    /// Default implementation processes images sequentially.
    fn execute_batch(
        &self,
        _ctx: &mut crate::core::context::ExecutionContext,
        _batch: &BatchContext,
    ) -> Result<Vec<Value>, ExecutionError> {
        Err(ExecutionError::NodeExecution {
            node_id: _ctx.node_id,
            error: "Batch execution not implemented for this filter".to_string(),
        })
    }

    /// Whether this filter can process images in parallel.
    fn supports_parallel(&self) -> bool {
        self.batch_mode() != BatchMode::Sequential
    }
}

/// Helper to create image batches from a Value.
pub fn extract_images_from_value(value: &Value) -> Result<Vec<ImageValue>, String> {
    match value {
        Value::Image(img) => Ok(vec![img.clone()]),
        Value::Array(arr) => {
            let mut images = Vec::new();
            for v in arr {
                if let Value::Image(img) = v {
                    images.push(img.clone());
                } else {
                    return Err(format!("Array contains non-image value: {:?}", v.get_type()));
                }
            }
            Ok(images)
        }
        _ => Err(format!("Expected image or image array, got {:?}", value.get_type())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{ImageMetadata, ImageFormat, ImageDataRef};
    use std::path::PathBuf;

    fn create_test_image(width: u32, height: u32) -> ImageValue {
        let metadata = ImageMetadata {
            width,
            height,
            format: ImageFormat::Png,
            has_alpha: true,
        };
        ImageValue::from_metadata(metadata, PathBuf::from("test.png"))
    }

    #[test]
    fn test_batch_context_creation() {
        let images = vec![
            create_test_image(100, 100),
            create_test_image(200, 200),
        ];
        
        let batch = BatchContext::new(images, 0, 1);
        assert_eq!(batch.len(), 2);
        assert_eq!(batch.progress(), 1.0);
    }

    #[test]
    fn test_batch_splitting_auto() {
        let images: Vec<_> = (0..10)
            .map(|_| create_test_image(100, 100))
            .collect();
        
        let batches = BatchContext::from_image_array(images, BatchSize::Auto);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 10);
    }

    #[test]
    fn test_batch_splitting_fixed() {
        let images: Vec<_> = (0..10)
            .map(|_| create_test_image(100, 100))
            .collect();
        
        let batches = BatchContext::from_image_array(images, BatchSize::Fixed(3));
        assert_eq!(batches.len(), 4); // 3 + 3 + 3 + 1
        assert_eq!(batches[0].len(), 3);
        assert_eq!(batches[3].len(), 1);
    }

    #[test]
    fn test_extract_single_image() {
        let img = create_test_image(100, 100);
        let value = Value::Image(img.clone());
        
        let images = extract_images_from_value(&value).unwrap();
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn test_extract_image_array() {
        let images = vec![
            Value::Image(create_test_image(100, 100)),
            Value::Image(create_test_image(200, 200)),
        ];
        let value = Value::Array(images);
        
        let extracted = extract_images_from_value(&value).unwrap();
        assert_eq!(extracted.len(), 2);
    }

    #[test]
    fn test_estimate_batch_size() {
        let images: Vec<_> = (0..100)
            .map(|_| create_test_image(1920, 1080))
            .collect();
        
        let size = estimate_batch_size(&images);
        assert!(size > 0 && size <= 32);
    }
}
