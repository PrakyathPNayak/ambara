//! Filter module.
//!
//! Contains the filter registry and built-in filter implementations.

pub mod registry;
pub mod builtin;

pub use registry::{FilterRegistry, FilterFactory};
