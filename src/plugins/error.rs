//! # Plugin Error Re-exports
//!
//! This module re-exports [`PluginError`] from [`crate::core::error`] for
//! ergonomic use within the `plugins` sub-tree. Plugin authors who need to
//! convert their errors into the host error type import from here.
//!
//! ## Examples
//!
//! ```rust,ignore
//! use ambara::plugins::error::PluginError;
//!
//! fn load() -> Result<(), PluginError> {
//!     Err(PluginError::PluginNotFound {
//!         plugin_id: "com.example.missing".to_string(),
//!     })
//! }
//! ```

/// Re-export the canonical `PluginError` from the core error module.
pub use crate::core::error::PluginError;
