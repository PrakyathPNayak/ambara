//! # Ambara Plugin System
//!
//! This module implements the external plugin system for Ambara.
//! Third-party developers can write Rust crates that compile to shared
//! libraries (`.so` / `.dll` / `.dylib`), export a stable C ABI vtable,
//! and have their `FilterNode` implementations loaded and executed as
//! first-class citizens in any Ambara processing graph.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        Ambara Host                          │
//! │                                                             │
//! │  PluginRegistry ──── discovers ──► ambara-plugin.toml      │
//! │       │                                                     │
//! │       └── loads ──► libmy_plugin.so                        │
//! │                          │                                  │
//! │                    PluginVTable ◄─── ambara_plugin_vtable   │
//! │                          │                                  │
//! │                    PluginFilterNode                         │
//! │                    (FilterNode impl)                        │
//! │                          │                                  │
//! │                    FilterRegistry ──► ExecutionEngine       │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Plugin Discovery
//!
//! The registry scans a directory for subdirectories (or files) containing:
//! 1. An `ambara-plugin.toml` manifest file
//! 2. A `.so` / `.dll` / `.dylib` library file
//!
//! ## Quick Start (Plugin Author)
//!
//! See `docs/writing-a-plugin.md` for a complete step-by-step guide.
//!
//! ## Security
//!
//! Plugins run in the same process. Loading an untrusted plugin is equivalent
//! to running untrusted native code. Only load plugins from sources you trust.
//! The capability system in `sandbox.rs` provides coarse-grained permission
//! flags, but does not constitute a security boundary.

#![warn(missing_docs)]

pub mod api;
pub mod error;
pub mod health;
pub mod loader;
pub mod manifest;
pub mod registry;
pub mod sandbox;

pub use api::{AbiResult, AbiStr, PluginHandle, PluginVTable, HOST_ABI_VERSION};
pub use error::PluginError;
pub use health::HealthReport;
pub use loader::LoadedPlugin;
pub use manifest::{PluginCapabilities, PluginManifest};
pub use registry::{PluginFilterNode, PluginRegistry, PluginSystemConfig};
pub use sandbox::CapabilitySet;
