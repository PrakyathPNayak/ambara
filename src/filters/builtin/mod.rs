//! Built-in filter implementations.
//!
//! This module contains the standard filters that ship with Ambara.

mod io;
mod blur;
mod color;
mod transform;
mod composite;
mod utility;
mod constants;
mod math;
mod comparison;
mod conversion;
mod astro;
mod batch;
mod array;
mod sharpen;
mod edge;
mod noise;
mod draw;
mod text;

use crate::filters::registry::FilterRegistry;

/// Register all built-in filters.
pub fn register_all(registry: &mut FilterRegistry) {
    io::register(registry);
    blur::register(registry);
    color::register(registry);
    transform::register(registry);
    composite::register(registry);
    utility::register(registry);
    constants::register(registry);
    math::register(registry);
    comparison::register(registry);
    conversion::register(registry);
    astro::register(registry);
    batch::register(registry);
    array::register(registry);
    sharpen::register(registry);
    edge::register(registry);
    noise::register(registry);
    draw::register(registry);
    text::register(registry);
}

// Re-export for direct access
pub use io::{LoadImage, LoadFolder, SaveImage};
pub use blur::{GaussianBlur, BoxBlur};
pub use color::{Brightness, Contrast, Saturation, Grayscale, Invert, Sepia, HueRotate, Threshold, Posterize, GammaCorrection, ColorBalance};
pub use transform::{Resize, Rotate, Flip, Crop};
pub use composite::{Blend, Overlay};
pub use utility::Preview;
pub use constants::{IntegerConstant, FloatConstant, StringConstant, BooleanConstant, ColorConstant};
pub use math::{Add, Subtract, Multiply, Divide, Modulo, Power, Min, Max, Clamp};
pub use astro::{ImageStack, DarkFrameSubtract, FlatFieldCorrect, HotPixelRemoval, HistogramStretch};
pub use batch::{BatchBrightness, BatchResize, BatchContrast};
pub use sharpen::{UnsharpMask, Sharpen};
pub use edge::{EdgeDetect, Emboss};
pub use noise::{AddNoise, Denoise};
pub use draw::{DrawRectangle, DrawCircle, DrawLine};
pub use text::TextOverlay;
