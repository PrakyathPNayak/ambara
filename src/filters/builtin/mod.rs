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
}

// Re-export for direct access
pub use io::{LoadImage, LoadFolder, SaveImage};
pub use blur::{GaussianBlur, BoxBlur};
pub use color::{Brightness, Contrast, Saturation, Grayscale, Invert};
pub use transform::{Resize, Rotate, Flip, Crop};
pub use composite::{Blend, Overlay};
pub use utility::Preview;
pub use constants::{IntegerConstant, FloatConstant, StringConstant, BooleanConstant, ColorConstant};
pub use math::{Add, Subtract, Multiply, Divide, Modulo, Power, Min, Max, Clamp};
