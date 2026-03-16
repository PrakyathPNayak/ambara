//! # Plugin Capability Sandbox
//!
//! The sandbox module defines a coarse-grained capability system that lets
//! users control what a plugin is allowed to do. Capabilities are declared
//! in the plugin manifest and must be explicitly granted by the host before
//! the plugin can use them.
//!
//! **Security note**: The capability system is advisory, not enforced at the
//! OS level. Loading an untrusted plugin is equivalent to running untrusted
//! native code. This system exists to communicate intent and provide
//! UI-level gating, not to sandbox the plugin process.
//!
//! ## Examples
//!
//! ```rust
//! use ambara::plugins::sandbox::CapabilitySet;
//!
//! let caps = CapabilitySet::from_manifest_flags(true, false, false, false);
//! assert!(caps.has(ambara::plugins::sandbox::Capability::Network));
//! assert!(!caps.has(ambara::plugins::sandbox::Capability::FilesystemWrite));
//! ```

use crate::plugins::manifest::PluginCapabilities;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Named capabilities a plugin can request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Make outbound network requests.
    Network,
    /// Read files from paths explicitly passed to the plugin.
    FilesystemRead,
    /// Write files to paths explicitly passed to the plugin.
    FilesystemWrite,
    /// Access GPU resources.
    Gpu,
}

impl Capability {
    /// Human-readable name for this capability.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Capability::Network => "Network Access",
            Capability::FilesystemRead => "Filesystem Read",
            Capability::FilesystemWrite => "Filesystem Write",
            Capability::Gpu => "GPU Access",
        }
    }

    /// Machine-readable identifier.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Capability::Network => "network",
            Capability::FilesystemRead => "filesystem_read",
            Capability::FilesystemWrite => "filesystem_write",
            Capability::Gpu => "gpu",
        }
    }
}

/// The set of capabilities granted to a loaded plugin.
///
/// Constructed from the manifest declarations; the host may further restrict
/// capabilities before calling `plugin_init`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    granted: HashSet<Capability>,
}

impl CapabilitySet {
    /// Create an empty (no capabilities granted) set.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            granted: HashSet::new(),
        }
    }

    /// Create a capability set from explicit flags.
    ///
    /// # Arguments
    ///
    /// * `network` - Whether network access is granted.
    /// * `fs_read` - Whether filesystem read access is granted.
    /// * `fs_write` - Whether filesystem write access is granted.
    /// * `gpu` - Whether GPU access is granted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ambara::plugins::sandbox::{CapabilitySet, Capability};
    /// let caps = CapabilitySet::from_manifest_flags(true, false, false, false);
    /// assert!(caps.has(Capability::Network));
    /// ```
    #[must_use]
    pub fn from_manifest_flags(network: bool, fs_read: bool, fs_write: bool, gpu: bool) -> Self {
        let mut granted = HashSet::new();
        if network {
            granted.insert(Capability::Network);
        }
        if fs_read {
            granted.insert(Capability::FilesystemRead);
        }
        if fs_write {
            granted.insert(Capability::FilesystemWrite);
        }
        if gpu {
            granted.insert(Capability::Gpu);
        }
        Self { granted }
    }

    /// Create a capability set from manifest capability declarations.
    #[must_use]
    pub fn from_manifest(caps: &PluginCapabilities) -> Self {
        Self::from_manifest_flags(
            caps.network,
            caps.filesystem_read,
            caps.filesystem_write,
            caps.gpu,
        )
    }

    /// Check whether a specific capability is granted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ambara::plugins::sandbox::{CapabilitySet, Capability};
    /// let caps = CapabilitySet::from_manifest_flags(false, true, false, false);
    /// assert!(caps.has(Capability::FilesystemRead));
    /// assert!(!caps.has(Capability::Network));
    /// ```
    #[must_use]
    pub fn has(&self, cap: Capability) -> bool {
        self.granted.contains(&cap)
    }

    /// Grant a capability.
    pub fn grant(&mut self, cap: Capability) {
        self.granted.insert(cap);
    }

    /// Revoke a capability.
    pub fn revoke(&mut self, cap: Capability) {
        self.granted.remove(&cap);
    }

    /// Return all granted capabilities.
    pub fn granted(&self) -> impl Iterator<Item = Capability> + '_ {
        self.granted.iter().copied()
    }

    /// Serialize the granted set to a JSON string for inclusion in the
    /// plugin-init config payload.
    #[must_use]
    pub fn to_config_json(&self) -> String {
        let caps: Vec<&str> = self.granted.iter().map(|c| c.id()).collect();
        serde_json::json!({ "granted_capabilities": caps }).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_has_no_capabilities() {
        let caps = CapabilitySet::empty();
        assert!(!caps.has(Capability::Network));
        assert!(!caps.has(Capability::FilesystemRead));
    }

    #[test]
    fn from_manifest_flags_grants_correctly() {
        let caps = CapabilitySet::from_manifest_flags(true, true, false, false);
        assert!(caps.has(Capability::Network));
        assert!(caps.has(Capability::FilesystemRead));
        assert!(!caps.has(Capability::FilesystemWrite));
        assert!(!caps.has(Capability::Gpu));
    }

    #[test]
    fn grant_revoke_roundtrip() {
        let mut caps = CapabilitySet::empty();
        caps.grant(Capability::Gpu);
        assert!(caps.has(Capability::Gpu));
        caps.revoke(Capability::Gpu);
        assert!(!caps.has(Capability::Gpu));
    }
}
