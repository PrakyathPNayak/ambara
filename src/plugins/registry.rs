//! # Plugin Registry
//!
//! The `PluginRegistry` manages the lifecycle of loaded plugins: discovery,
//! loading, unloading, health checking, and routing filter execution to the
//! correct plugin.
//!
//! It also provides [`PluginFilterNode`], a [`FilterNode`] adapter that wraps
//! a plugin-contributed filter behind the standard Ambara node interface.
//! Instances of `PluginFilterNode` are registered in the [`FilterRegistry`]
//! when plugins are loaded.
//!
//! ## Thread Safety
//!
//! Each `LoadedPlugin` is wrapped in an `Arc<Mutex<LoadedPlugin>>`. You may
//! safely share a `&PluginRegistry` across threads for read operations; write
//! operations (load/unload) require `&mut PluginRegistry`.
//!
//! ## Examples
//!
//! ```rust,ignore
//! use ambara::plugins::registry::{PluginRegistry, PluginSystemConfig};
//! use ambara::filters::registry::FilterRegistry;
//!
//! let mut registry = PluginRegistry::new("/path/to/plugins", PluginSystemConfig::default());
//! // Discover and load all plugins
//! let results = registry.load_all();
//! for (path, result) in results {
//!     match result {
//!         Ok(id) => println!("Loaded plugin {id} from {}", path.display()),
//!         Err(e) => eprintln!("Failed to load {}: {e}", path.display()),
//!     }
//! }
//!
//! // Register all plugin filters in the filter registry
//! let mut filter_registry = FilterRegistry::with_builtins();
//! registry.register_all_in_filter_registry(&mut filter_registry).unwrap();
//! ```

use crate::core::context::{ExecutionContext, ValidationContext};
use crate::core::error::{ExecutionError, PluginError, ValidationError};
use crate::core::node::{FilterNode, NodeMetadata};
use crate::filters::registry::{FilterRegistry, FilterSource};
use crate::plugins::health::HealthReport;
use crate::plugins::loader::LoadedPlugin;
use crate::plugins::manifest::PluginManifest;

use indexmap::IndexMap;
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Configuration for the plugin system.
#[derive(Debug, Clone)]
pub struct PluginSystemConfig {
    /// Maximum number of plugins that may be loaded simultaneously.
    pub max_plugins: usize,
    /// Whether to automatically load all plugins in the plugin directory on
    /// `PluginRegistry::load_all`.
    pub auto_load: bool,
    /// Host configuration JSON passed to each plugin on initialisation.
    pub host_config: serde_json::Value,
}

impl Default for PluginSystemConfig {
    fn default() -> Self {
        Self {
            max_plugins: 64,
            auto_load: false,
            host_config: serde_json::json!({}),
        }
    }
}

/// Registry managing all loaded plugins.
///
/// Use [`PluginRegistry::new`] to create an instance, then call
/// [`PluginRegistry::load_plugin`] or [`PluginRegistry::load_all`] to
/// populate it.
pub struct PluginRegistry {
    /// Loaded plugins, keyed by their manifest ID.
    plugins: IndexMap<String, Arc<Mutex<LoadedPlugin>>>,
    /// Directory where plugins are discovered.
    plugin_dir: PathBuf,
    /// System configuration.
    config: PluginSystemConfig,
}

impl PluginRegistry {
    /// Create a new, empty plugin registry.
    ///
    /// No plugins are loaded at construction time.
    ///
    /// # Arguments
    ///
    /// * `plugin_dir` - The directory to scan for plugins.
    /// * `config` - System-wide configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use ambara::plugins::registry::{PluginRegistry, PluginSystemConfig};
    /// let registry = PluginRegistry::new("/tmp/plugins", PluginSystemConfig::default());
    /// assert_eq!(registry.loaded_plugin_count(), 0);
    /// ```
    #[must_use]
    pub fn new(plugin_dir: impl Into<PathBuf>, config: PluginSystemConfig) -> Self {
        Self {
            plugins: IndexMap::new(),
            plugin_dir: plugin_dir.into(),
            config,
        }
    }

    /// Scan the plugin directory for valid plugin manifests without loading
    /// the libraries.
    ///
    /// A valid plugin directory entry is a directory (or the plugin dir itself)
    /// containing an `ambara-plugin.toml` file.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError::Io`] if the directory cannot be read.
    pub fn discover(&self) -> Result<Vec<(PathBuf, PluginManifest)>, PluginError> {
        if !self.plugin_dir.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let read_dir =
            std::fs::read_dir(&self.plugin_dir).map_err(|e| PluginError::Io {
                message: format!("Cannot read plugin dir {}: {e}", self.plugin_dir.display()),
            })?;

        for entry in read_dir.flatten() {
            let path = entry.path();
            // Look for ambara-plugin.toml in subdirectories
            let manifest_path = if path.is_dir() {
                path.join("ambara-plugin.toml")
            } else {
                continue;
            };

            if manifest_path.exists() {
                match PluginManifest::from_path(&manifest_path) {
                    Ok(m) => {
                        // Find the library file in the same directory
                        let lib_path = Self::find_library_in(&path);
                        if let Some(lib) = lib_path {
                            results.push((lib, m));
                        }
                    }
                    Err(e) => {
                        log::warn!("Skipping malformed manifest {}: {e}", manifest_path.display());
                    }
                }
            }
        }
        Ok(results)
    }

    /// Find the first `.so`/`.dll`/`.dylib` file in `dir`.
    fn find_library_in(dir: &Path) -> Option<PathBuf> {
        let extensions = &["so", "dll", "dylib"];
        std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
            let p = e.path();
            if p.is_file() {
                if let Some(ext) = p.extension() {
                    if extensions.iter().any(|&ex| ext == ex) {
                        return Some(p);
                    }
                }
            }
            None
        })
    }

    /// Load a single plugin from the given library path.
    ///
    /// # Arguments
    ///
    /// * `library_path` - Path to the compiled plugin `.so`/`.dll`/`.dylib`.
    ///
    /// # Returns
    ///
    /// The plugin's manifest ID on success.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError`] if loading, ABI verification, or init fails,
    /// or if a plugin with the same ID is already loaded.
    pub fn load_plugin(&mut self, library_path: &Path) -> Result<String, PluginError> {
        let plugin = LoadedPlugin::load(library_path, &self.config.host_config)?;
        let plugin_id = plugin.id().to_string();

        if self.plugins.contains_key(&plugin_id) {
            return Err(PluginError::PluginAlreadyLoaded {
                plugin_id: plugin_id.clone(),
            });
        }

        self.plugins
            .insert(plugin_id.clone(), Arc::new(Mutex::new(plugin)));
        log::info!("Plugin '{}' loaded from {}", plugin_id, library_path.display());
        Ok(plugin_id)
    }

    /// Unload a plugin by ID.
    ///
    /// Removes the plugin from the registry. The `Arc` will be dropped when
    /// all `PluginFilterNode` instances backed by this plugin are dropped too.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError::PluginNotFound`] if no plugin with that ID exists.
    pub fn unload_plugin(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        self.plugins
            .shift_remove(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound {
                plugin_id: plugin_id.to_string(),
            })?;
        log::info!("Plugin '{plugin_id}' unloaded");
        Ok(())
    }

    /// Load all plugins discovered in the plugin directory.
    ///
    /// Returns one `(path, result)` entry per discovered plugin library.
    /// Failures are not fatal; the caller decides whether to surface them.
    pub fn load_all(&mut self) -> Vec<(PathBuf, Result<String, PluginError>)> {
        let discovered = match self.discover() {
            Ok(d) => d,
            Err(e) => {
                return vec![(
                    self.plugin_dir.clone(),
                    Err(PluginError::Io {
                        message: e.to_string(),
                    }),
                )];
            }
        };

        discovered
            .into_iter()
            .map(|(lib_path, _manifest)| {
                let result = self.load_plugin(&lib_path);
                (lib_path, result)
            })
            .collect()
    }

    /// Return a reference to the `Arc<Mutex<LoadedPlugin>>` for a given plugin ID.
    #[must_use]
    pub fn get_plugin_arc(&self, plugin_id: &str) -> Option<Arc<Mutex<LoadedPlugin>>> {
        self.plugins.get(plugin_id).map(Arc::clone)
    }

    /// Return a reference to a loaded plugin for inspection.
    ///
    /// Locks the mutex. Prefer [`get_plugin_arc`] when holding the lock
    /// across multiple operations.
    pub fn with_plugin<F, R>(&self, plugin_id: &str, f: F) -> Option<R>
    where
        F: FnOnce(&LoadedPlugin) -> R,
    {
        let arc = self.plugins.get(plugin_id)?;
        let guard = arc.lock();
        Some(f(&*guard))
    }

    /// Return the number of currently loaded plugins.
    #[must_use]
    pub fn loaded_plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Iterate over all loaded plugin IDs.
    pub fn plugin_ids(&self) -> impl Iterator<Item = &str> {
        self.plugins.keys().map(String::as_str)
    }

    /// Run health checks on all loaded plugins.
    ///
    /// Returns a `HealthReport` for each plugin.
    pub fn health_check_all(&self) -> Vec<HealthReport> {
        self.plugins
            .values()
            .map(|arc| arc.lock().health_check())
            .collect()
    }

    /// Register all filters from all loaded plugins into the given
    /// [`FilterRegistry`].
    ///
    /// Call this after loading plugins to make their filters available.
    ///
    /// # Errors
    ///
    /// Returns the first [`PluginError`] encountered (other plugins continue
    /// to be registered).
    pub fn register_all_in_filter_registry(
        &self,
        filter_registry: &mut FilterRegistry,
    ) -> Result<(), PluginError> {
        for plugin_id in self.plugins.keys().cloned().collect::<Vec<_>>() {
            self.register_plugin_in_filter_registry(&plugin_id, filter_registry)?;
        }
        Ok(())
    }

    /// Register filters from a specific plugin into the filter registry.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError`] if the plugin is not loaded or filter
    /// metadata cannot be retrieved.
    pub fn register_plugin_in_filter_registry(
        &self,
        plugin_id: &str,
        filter_registry: &mut FilterRegistry,
    ) -> Result<(), PluginError> {
        let arc = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound {
                plugin_id: plugin_id.to_string(),
            })?;

        let filter_ids: Vec<String> = arc.lock().filter_ids();

        for filter_id in filter_ids {
            let metadata = {
                let plugin = arc.lock();
                plugin.filter_metadata(&filter_id)?
            };

            let plugin_arc = Arc::clone(arc);
            let fid = filter_id.clone();
            let pid = plugin_id.to_string();
            let plugin_version = arc.lock().manifest.plugin.version.clone();

            let source = FilterSource::Plugin {
                plugin_id: pid,
                plugin_version,
            };

            let meta_clone = metadata.clone();
            filter_registry.register_plugin_filter(
                move || {
                    Box::new(PluginFilterNode {
                        plugin: Arc::clone(&plugin_arc),
                        filter_id: fid.clone(),
                        metadata: meta_clone.clone(),
                    })
                },
                metadata,
                source,
            );
        }

        Ok(())
    }

    /// Execute a filter from a named plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - ID of the plugin owning the filter.
    /// * `filter_id` - ID of the filter to execute.
    /// * `inputs` - Named input values.
    /// * `params` - Named parameter values.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError`] if the plugin is not loaded or execution fails.
    pub fn execute_filter(
        &self,
        plugin_id: &str,
        filter_id: &str,
        inputs: &[(&str, crate::core::types::Value)],
        params: &[(&str, crate::core::types::Value)],
    ) -> Result<std::collections::HashMap<String, crate::core::types::Value>, PluginError> {
        let arc =
            self.plugins
                .get(plugin_id)
                .ok_or_else(|| PluginError::PluginNotFound {
                    plugin_id: plugin_id.to_string(),
                })?;
        arc.lock().execute_filter(filter_id, inputs, params)
    }
}

// ============================================================================
// PluginFilterNode — adapts a plugin filter to the FilterNode trait
// ============================================================================

/// A [`FilterNode`] adaptor that routes execution to a plugin via the C ABI.
///
/// Instances are created by [`PluginRegistry::register_plugin_in_filter_registry`]
/// and registered in the [`FilterRegistry`] as first-class nodes. From the
/// execution engine's perspective they are indistinguishable from builtin nodes.
#[derive(Clone)]
pub struct PluginFilterNode {
    /// Shared handle to the owning plugin (protected by a mutex).
    plugin: Arc<Mutex<LoadedPlugin>>,
    /// The specific filter ID this node represents.
    filter_id: String,
    /// Cached metadata (does not require locking the plugin).
    metadata: NodeMetadata,
}

impl FilterNode for PluginFilterNode {
    fn metadata(&self) -> NodeMetadata {
        self.metadata.clone()
    }

    fn validate(&self, ctx: &ValidationContext) -> Result<(), ValidationError> {
        // Collect inputs and parameters from the context
        let inputs: Vec<(&str, crate::core::types::Value)> = self
            .metadata
            .inputs
            .iter()
            .filter_map(|p| {
                ctx.inputs().get(&p.name).map(|v| (p.name.as_str(), v.clone()))
            })
            .collect();
        let params: Vec<(&str, crate::core::types::Value)> = self
            .metadata
            .parameters
            .iter()
            .filter_map(|p| {
                ctx.parameters().get(&p.name).map(|v| (p.name.as_str(), v.clone()))
            })
            .collect();

        // Borrows are temporaries, rebuild with stable references
        let input_pairs: Vec<(&str, crate::core::types::Value)> = inputs.clone();
        let param_pairs: Vec<(&str, crate::core::types::Value)> = params.clone();

        let errors = self
            .plugin
            .lock()
            .validate_filter(&self.filter_id, &input_pairs, &param_pairs);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::CustomValidation {
                node_id: ctx.node_id,
                error: errors.join("; "),
            })
        }
    }

    fn execute(&self, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        // Collect all inputs from the context
        let inputs: Vec<(String, crate::core::types::Value)> = self
            .metadata
            .inputs
            .iter()
            .filter_map(|p| {
                ctx.inputs().get(&p.name).map(|v| (p.name.clone(), v.clone()))
            })
            .collect();
        let params: Vec<(String, crate::core::types::Value)> = self
            .metadata
            .parameters
            .iter()
            .filter_map(|p| {
                ctx.parameters().get(&p.name).map(|v| (p.name.clone(), v.clone()))
            })
            .collect();

        let input_refs: Vec<(&str, crate::core::types::Value)> =
            inputs.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        let param_refs: Vec<(&str, crate::core::types::Value)> =
            params.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();

        let outputs = self
            .plugin
            .lock()
            .execute_filter(&self.filter_id, &input_refs, &param_refs)
            .map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Plugin filter '{}' failed: {e}", self.filter_id),
            })?;

        for (name, value) in outputs {
            ctx.set_output(name, value).map_err(|e| ExecutionError::NodeExecution {
                node_id: ctx.node_id,
                error: format!("Failed to set output: {e}"),
            })?;
        }
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn FilterNode> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod plugin_registry {
        use super::*;

        #[test]
        fn new_registry_is_empty() {
            let reg = PluginRegistry::new("/nonexistent", PluginSystemConfig::default());
            assert_eq!(reg.loaded_plugin_count(), 0);
        }

        #[test]
        fn discover_nonexistent_dir_returns_empty() {
            let reg = PluginRegistry::new("/nonexistent_dir_xyz", PluginSystemConfig::default());
            let found = reg.discover().unwrap();
            assert!(found.is_empty());
        }

        #[test]
        fn unload_nonexistent_plugin_errors() {
            let mut reg = PluginRegistry::new("/tmp", PluginSystemConfig::default());
            let result = reg.unload_plugin("com.example.nonexistent");
            assert!(matches!(result, Err(PluginError::PluginNotFound { .. })));
        }
    }
}
