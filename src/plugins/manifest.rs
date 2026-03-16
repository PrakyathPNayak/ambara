//! # Plugin Manifest
//!
//! Every plugin must ship an `ambara-plugin.toml` manifest file in the same
//! directory as its compiled library. The manifest is read **before** the
//! library is loaded, allowing the UI to display plugin metadata and the host
//! to reject incompatible plugins without loading potentially harmful code.
//!
//! ## Manifest Format
//!
//! ```toml
//! [plugin]
//! id = "com.example.my-plugin"
//! name = "My Plugin"
//! version = "1.0.0"
//! description = "Does amazing things"
//! author = "Author Name <email@example.com>"
//! homepage = "https://example.com"
//! license = "MIT"
//! ambara_abi_version = 1
//! min_ambara_version = "0.3.0"
//! max_ambara_version = "0.99.99"
//!
//! [plugin.capabilities]
//! network = true
//! filesystem_read = true
//! filesystem_write = false
//! gpu = false
//!
//! [plugin.filters]
//! ids = ["my.filter_one", "my.filter_two"]
//!
//! [plugin.config]
//! endpoint = "http://127.0.0.1:8188"
//! ```
//!
//! ## Examples
//!
//! ```rust
//! use ambara::plugins::manifest::PluginManifest;
//!
//! let toml = r#"
//! [plugin]
//! id = "com.example.test"
//! name = "Test Plugin"
//! version = "0.1.0"
//! description = "A test plugin"
//! author = "Test Author"
//! license = "MIT"
//! ambara_abi_version = 1
//! min_ambara_version = "0.3.0"
//! max_ambara_version = "99.0.0"
//!
//! [plugin.capabilities]
//! network = false
//! filesystem_read = false
//! filesystem_write = false
//! gpu = false
//!
//! [plugin.filters]
//! ids = []
//!
//! [plugin.config]
//! "#;
//! let manifest: PluginManifest = PluginManifest::from_toml_str(toml).unwrap();
//! assert_eq!(manifest.plugin.id, "com.example.test");
//! ```

use crate::core::error::PluginError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level structure of `ambara-plugin.toml`.
///
/// This is the deserialized form of the manifest file. Use
/// [`PluginManifest::from_path`] to load from disk or
/// [`PluginManifest::from_toml_str`] for testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Core plugin metadata block.
    pub plugin: PluginMeta,
}

/// Core plugin metadata contained in the `[plugin]` TOML table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    /// Reverse-DNS-style unique identifier (e.g., `"com.example.my-plugin"`).
    pub id: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Plugin version (SemVer string, e.g., `"1.2.3"`).
    pub version: String,
    /// One-sentence description shown in the plugin panel.
    pub description: String,
    /// Author name and optional email.
    pub author: String,
    /// Optional homepage URL.
    #[serde(default)]
    pub homepage: Option<String>,
    /// SPDX license identifier.
    pub license: String,
    /// ABI version the plugin was compiled against.
    ///
    /// Must equal [`crate::plugins::api::HOST_ABI_VERSION`] for the plugin
    /// to be accepted.
    pub ambara_abi_version: u32,
    /// Minimum Ambara version required (SemVer string).
    pub min_ambara_version: String,
    /// Maximum Ambara version the plugin has been tested against.
    pub max_ambara_version: String,
    /// Capability flags the plugin requests.
    pub capabilities: PluginCapabilities,
    /// Pre-declared filter IDs (for UI display before library is loaded).
    pub filters: PluginFiltersBlock,
    /// Default configuration values (key→value string map).
    #[serde(default)]
    pub config: std::collections::HashMap<String, String>,
}

/// Requested capability flags for a plugin.
///
/// Each flag defaults to `false`; granting a capability is an explicit
/// user action. The IDs presented in the sandbox module correspond 1-to-1
/// with these fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginCapabilities {
    /// Plugin may make outbound network requests.
    #[serde(default)]
    pub network: bool,
    /// Plugin may read from paths explicitly passed to it.
    #[serde(default)]
    pub filesystem_read: bool,
    /// Plugin may write to paths explicitly passed to it.
    #[serde(default)]
    pub filesystem_write: bool,
    /// Plugin may access GPU resources.
    #[serde(default)]
    pub gpu: bool,
}

/// Filter IDs pre-declared in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginFiltersBlock {
    /// Filter identifiers the plugin will register on init.
    #[serde(default)]
    pub ids: Vec<String>,
}

impl PluginManifest {
    /// Parse a manifest from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError::ManifestParseError`] if the TOML is malformed or
    /// missing required fields.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ambara::plugins::manifest::PluginManifest;
    /// let toml = include_str!("../../plugins/comfyui_bridge/ambara-plugin.toml");
    /// // let manifest = PluginManifest::from_toml_str(toml).unwrap();
    /// ```
    pub fn from_toml_str(s: &str) -> Result<Self, PluginError> {
        toml::from_str::<Self>(s).map_err(|e| PluginError::ManifestParseError {
            path: std::path::PathBuf::from("<string>"),
              reason: e.to_string(),
        })
    }

    /// Load a manifest from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the `ambara-plugin.toml` file.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError::Io`] if the file cannot be read, or
    /// [`PluginError::ManifestParseError`] if parsing fails.
    pub fn from_path(path: &Path) -> Result<Self, PluginError> {
        let content = std::fs::read_to_string(path).map_err(|e| PluginError::Io {
            message: format!("Cannot read manifest {}: {}", path.display(), e),
        })?;
        toml::from_str::<Self>(&content).map_err(|e| PluginError::ManifestParseError {
            path: path.to_path_buf(),
                reason: e.to_string(),
        })
    }

    /// Check whether this plugin is compatible with the given Ambara version.
    ///
    /// # Arguments
    ///
    /// * `ambara_version` - The running Ambara version string (e.g., `"0.3.0"`).
    ///
    /// # Returns
    ///
    /// `true` if `min_ambara_version <= ambara_version <= max_ambara_version`,
    /// `false` otherwise.
    #[must_use]
    pub fn is_compatible_with(&self, ambara_version: &str) -> bool {
        use semver::{Version, VersionReq};
        let Ok(current) = Version::parse(ambara_version) else {
            return false;
        };
        let min_req = format!(">={}", self.plugin.min_ambara_version);
        let max_req = format!("<={}", self.plugin.max_ambara_version);
        let Ok(min) = VersionReq::parse(&min_req) else {
            return false;
        };
        let Ok(max) = VersionReq::parse(&max_req) else {
            return false;
        };
        min.matches(&current) && max.matches(&current)
    }

    /// Validate that required fields have sensible values.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError::ManifestMissingField`] if a required field is
    /// empty or obviously invalid.
    pub fn validate(&self) -> Result<(), PluginError> {
        let path = std::path::PathBuf::from("<manifest>");
        if self.plugin.id.is_empty() {
            return Err(PluginError::ManifestMissingField {
                field: "plugin.id".to_string(),
                path: path.clone(),
            });
        }
        if self.plugin.name.is_empty() {
            return Err(PluginError::ManifestMissingField {
                field: "plugin.name".to_string(),
                path: path.clone(),
            });
        }
        if self.plugin.version.is_empty() {
            return Err(PluginError::ManifestMissingField {
                field: "plugin.version".to_string(),
                path,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TOML: &str = r#"
[plugin]
id = "com.test.plugin"
name = "Test Plugin"
version = "1.0.0"
description = "A test plugin"
author = "Test <test@example.com>"
license = "MIT"
ambara_abi_version = 1
min_ambara_version = "0.3.0"
max_ambara_version = "99.99.99"

[plugin.capabilities]
network = false
filesystem_read = true
filesystem_write = false
gpu = false

[plugin.filters]
ids = ["test.filter_one"]

[plugin.config]
"#;

    mod from_toml_str {
        use super::*;

        #[test]
        fn parses_valid_manifest() {
            let m = PluginManifest::from_toml_str(VALID_TOML).unwrap();
            assert_eq!(m.plugin.id, "com.test.plugin");
            assert_eq!(m.plugin.name, "Test Plugin");
            assert_eq!(m.plugin.filters.ids, vec!["test.filter_one"]);
            assert!(m.plugin.capabilities.filesystem_read);
            assert!(!m.plugin.capabilities.network);
        }

        #[test]
        fn fails_on_missing_required_field() {
            let bad = r#"[plugin]\nname = "X"\n"#;
            assert!(PluginManifest::from_toml_str(bad).is_err());
        }
    }

    mod is_compatible_with {
        use super::*;

        #[test]
        fn current_version_is_compatible() {
            let m = PluginManifest::from_toml_str(VALID_TOML).unwrap();
            assert!(m.is_compatible_with("0.3.0"));
            assert!(m.is_compatible_with("1.0.0"));
        }

        #[test]
        fn too_old_is_incompatible() {
            let m = PluginManifest::from_toml_str(VALID_TOML).unwrap();
            assert!(!m.is_compatible_with("0.2.0"));
        }
    }

    mod validate {
        use super::*;

        #[test]
        fn valid_manifest_passes() {
            let m = PluginManifest::from_toml_str(VALID_TOML).unwrap();
            assert!(m.validate().is_ok());
        }
    }
}
