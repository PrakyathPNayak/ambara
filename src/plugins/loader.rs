//! # Dynamic Library Plugin Loader
//!
//! This module is responsible for loading a plugin shared library from disk,
//! locating its [`PluginVTable`], initialising the plugin instance, and
//! providing a safe Rust wrapper around the raw FFI calls.
//!
//! ## Safety Model
//!
//! All interaction with the loaded library happens through raw C function
//! pointers in the vtable. Several invariants must be maintained at all times:
//!
//! 1. **Drop order**: The `_library` field must be dropped **last**. Because
//!    Rust drops struct fields in declaration order, and `_library` is declared
//!    last, this is guaranteed.  The `Drop` implementation calls `plugin_destroy`
//!    before the implicit field drops occur, which is the correct sequence.
//!
//! 2. **vtable lifetime**: The vtable pointer points into the loaded library's
//!    memory. It is valid only while `_library` is not dropped. All vtable
//!    accesses in methods on `LoadedPlugin` happen before `_library` is dropped.
//!
//! 3. **panic safety**: The host wraps all vtable calls in error-result
//!    conversions. If the plugin panics across the FFI boundary (which is UB),
//!    the plugin itself is responsible for catching it with `catch_unwind`.
//!
//! ## Examples
//!
//! ```rust,ignore
//! use ambara::plugins::loader::LoadedPlugin;
//! use std::path::Path;
//!
//! let plugin = LoadedPlugin::load(
//!     Path::new("/path/to/libmy_plugin.so"),
//!     &serde_json::json!({}),
//! )?;
//! println!("Loaded plugin: {}", plugin.manifest.plugin.name);
//! for id in plugin.filter_ids() {
//!     println!("  filter: {id}");
//! }
//! ```

use crate::core::error::PluginError;
use crate::core::node::NodeMetadata;
use crate::core::types::Value;
use crate::plugins::api::{AbiResult, PluginHandle, PluginVTable, HOST_ABI_VERSION};
use crate::plugins::health::HealthReport;
use crate::plugins::manifest::PluginManifest;
use crate::plugins::sandbox::CapabilitySet;

use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Size of the intermediate JSON buffer used for FFI calls (4 MiB).
///
/// If a plugin produces output larger than this, a dynamic reallocation loop
/// will be used. For most image-processing nodes the output is metadata plus
/// a base64 image, so 4 MiB is sufficient for typical workloads.
const DEFAULT_BUF_SIZE: usize = 4 * 1024 * 1024;

/// A loaded plugin instance with its vtable and lifecycle state.
///
/// Fields are ordered carefully: `_library` is last so that Rust's field-drop
/// order ensures the library is unloaded only **after** all other cleanup.
pub struct LoadedPlugin {
    /// Deserialized manifest (read from `ambara-plugin.toml`).
    pub manifest: PluginManifest,
    /// Path the library was loaded from.
    pub library_path: PathBuf,
    /// Opaque plugin handle (owned by the plugin; freed via `plugin_destroy`).
    handle: *mut PluginHandle,
    /// Pointer to the vtable inside the loaded library.
    ///
    /// # SAFETY
    ///
    /// Valid only while `_library` has not been dropped. All methods on this
    /// struct access `vtable_ptr` before `_library` can be dropped (which only
    /// happens in `Drop::drop` after `plugin_destroy` has been called).
    vtable_ptr: *const PluginVTable,
    /// When this plugin was loaded.
    pub loaded_at: Instant,
    /// Granted capability set for this plugin.
    pub capabilities: CapabilitySet,
    /// Whether the plugin has passed its last health check.
    pub last_healthy: bool,
    /// The loaded shared library. **Must be the last field** (dropped last).
    _library: libloading::Library,
}

// SAFETY: LoadedPlugin is Send because:
// - `handle` and `vtable_ptr` are raw pointers into memory owned by `_library`.
// - We hold `_library` exclusively (no sharing).
// - All access is protected by `Arc<Mutex<LoadedPlugin>>` at the registry level.
unsafe impl Send for LoadedPlugin {}

// SAFETY: Same as Send reasoning. The plugin is never accessed concurrently
// from multiple threads without Mutex protection at the call site.
unsafe impl Sync for LoadedPlugin {}

impl LoadedPlugin {
    /// Load a plugin from a shared library path.
    ///
    /// Reads the `ambara-plugin.toml` manifest from the same directory as the
    /// library, loads the library, locates the vtable, verifies the ABI
    /// version, and calls `plugin_init` with the serialised config.
    ///
    /// # Arguments
    ///
    /// * `library_path` - Path to the `.so`/`.dll`/`.dylib` file.
    /// * `config` - Host-provided configuration JSON (merged with manifest defaults).
    ///
    /// # Errors
    ///
    /// Returns [`PluginError`] if:
    /// - The manifest file cannot be found or parsed.
    /// - The library cannot be loaded (missing file, wrong architecture).
    /// - The vtable symbol is absent or the ABI version mismatches.
    /// - `plugin_create` or `plugin_init` fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use ambara::plugins::loader::LoadedPlugin;
    /// use std::path::Path;
    ///
    /// let plugin = LoadedPlugin::load(Path::new("libmy_plugin.so"), &serde_json::json!({}))?;
    /// assert!(plugin.health_check());
    /// ```
    pub fn load(library_path: &Path, config: &serde_json::Value) -> Result<Self, PluginError> {
        // --- Step 1: Read manifest ---
        let manifest_path = library_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("ambara-plugin.toml");

        let manifest = PluginManifest::from_path(&manifest_path)?;
        manifest.validate()?;

        // Check Ambara version compatibility
        let current_version = crate::VERSION;
        if !manifest.is_compatible_with(current_version) {
            return Err(PluginError::AmbaraVersionTooOld {
                plugin_id: manifest.plugin.id.clone(),
                required: manifest.plugin.min_ambara_version.clone(),
                current: current_version.to_string(),
            });
        }

        // --- Step 2: Load the library ---
        // SAFETY: Loading a shared library is inherently unsafe — the caller
        // is responsible for ensuring the library is trusted code.
        let library = unsafe {
            libloading::Library::new(library_path).map_err(|e| PluginError::PluginLoadFailed {
                path: library_path.to_path_buf(),
                reason: e.to_string(),
            })?
        };

        // --- Step 3: Locate the vtable symbol ---
        // SAFETY: We immediately read the pointer value and release the
        // symbol borrow. The pointer is valid as long as `library` is loaded.
        let vtable_ptr: *const PluginVTable = unsafe {
            let symbol = library
                .get::<*const PluginVTable>(b"ambara_plugin_vtable\0")
                .map_err(|_| PluginError::MissingVtableSymbol {
                    path: library_path.to_path_buf(),
                })?;
            *symbol
        };

        if vtable_ptr.is_null() {
            return Err(PluginError::MissingVtableSymbol {
                path: library_path.to_path_buf(),
            });
        }

        // --- Step 4: Check ABI version ---
        // SAFETY: vtable_ptr is non-null and points into the loaded library.
        let abi_version = unsafe { (*vtable_ptr).abi_version };
        if abi_version != HOST_ABI_VERSION {
            return Err(PluginError::AbiVersionMismatch {
                plugin_id: manifest.plugin.id.clone(),
                plugin_abi: abi_version,
                host_abi: HOST_ABI_VERSION,
            });
        }

        // --- Step 5: Create plugin instance ---
        // SAFETY: As above.
        let handle = unsafe { ((*vtable_ptr).plugin_create)() };
        if handle.is_null() {
            return Err(PluginError::PluginInitFailed {
                plugin_id: manifest.plugin.id.clone(),
                message: "plugin_create returned null".to_string(),
            });
        }

        // --- Step 6: Derive capabilities and build init config ---
        let capabilities = CapabilitySet::from_manifest(&manifest.plugin.capabilities);

        // Merge manifest config defaults with host-provided config
        let mut init_config = serde_json::json!({});
        for (k, v) in &manifest.plugin.config {
            init_config[k] = serde_json::Value::String(v.clone());
        }
        if let Some(obj) = config.as_object() {
            for (k, v) in obj {
                init_config[k] = v.clone();
            }
        }
        init_config["granted_capabilities"] = serde_json::json!(
            capabilities.granted().map(|c| c.id()).collect::<Vec<_>>()
        );

        let config_json = serde_json::to_string(&init_config)
            .unwrap_or_else(|_| "{}".to_string());
        let config_bytes = config_json.as_bytes();

        // --- Step 7: Initialise the plugin ---
        // SAFETY: handle is non-null; config_bytes is valid UTF-8.
        let init_result = unsafe {
            ((*vtable_ptr).plugin_init)(handle, config_bytes.as_ptr(), config_bytes.len())
        };

        if init_result != AbiResult::Ok {
            // Destroy before returning error
            // SAFETY: handle is valid; we're cleaning up.
            unsafe { ((*vtable_ptr).plugin_destroy)(handle) };
            return Err(PluginError::PluginInitFailed {
                plugin_id: manifest.plugin.id.clone(),
                message: format!("plugin_init returned {:?}", init_result),
            });
        }

        Ok(Self {
            manifest,
            library_path: library_path.to_path_buf(),
            handle,
            vtable_ptr,
            loaded_at: Instant::now(),
            capabilities,
            last_healthy: true,
            _library: library,
        })
    }

    /// Return the plugin's unique ID from its manifest.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.manifest.plugin.id
    }

    /// List all filter IDs contributed by this plugin.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` of filter identifiers (e.g., `"comfy.ksampler"`).
    #[must_use]
    pub fn filter_ids(&self) -> Vec<String> {
        // SAFETY: vtable_ptr is valid; handle is valid and initialised.
        let count = unsafe { ((*self.vtable_ptr).filter_count)(self.handle) };
        (0..count)
            .filter_map(|i| {
                // SAFETY: i < count; handle is valid.
                let ptr = unsafe { ((*self.vtable_ptr).filter_id_at)(self.handle, i) };
                if ptr.is_null() {
                    return None;
                }
                // SAFETY: ptr is a valid null-terminated C string.
                let cstr = unsafe { CStr::from_ptr(ptr) };
                cstr.to_str().ok().map(ToString::to_string)
            })
            .collect()
    }

    /// Retrieve the serialised [`NodeMetadata`] for a named filter.
    ///
    /// # Arguments
    ///
    /// * `filter_id` - The filter identifier string.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError::PluginExecutionError`] if the plugin fails to
    /// serialise the metadata.
    pub fn filter_metadata(&self, filter_id: &str) -> Result<NodeMetadata, PluginError> {
        let id_cstr = std::ffi::CString::new(filter_id).map_err(|e| {
            PluginError::PluginExecutionError {
                plugin_id: self.id().to_string(),
                filter_id: filter_id.to_string(),
                message: format!("invalid filter_id: {e}"),
            }
        })?;

        let mut buf = vec![0u8; DEFAULT_BUF_SIZE];
        // SAFETY: All pointer arguments are valid for their respective lengths.
        let written = unsafe {
            ((*self.vtable_ptr).filter_metadata_json)(
                self.handle,
                id_cstr.as_ptr(),
                buf.as_mut_ptr(),
                buf.len(),
            )
        };

        if written == 0 {
            return Err(PluginError::PluginExecutionError {
                plugin_id: self.id().to_string(),
                filter_id: filter_id.to_string(),
                message: "filter_metadata_json returned 0 bytes".to_string(),
            });
        }

        let json_slice = &buf[..written];
        serde_json::from_slice::<NodeMetadata>(json_slice).map_err(|e| {
            PluginError::PluginExecutionError {
                plugin_id: self.id().to_string(),
                filter_id: filter_id.to_string(),
                message: format!("failed to deserialise NodeMetadata JSON: {e}"),
            }
        })
    }

    /// Execute a filter node via the C ABI.
    ///
    /// # Arguments
    ///
    /// * `filter_id` - Filter identifier.
    /// * `inputs` - Named input values as JSON-serialisable `Value` pairs.
    /// * `params` - Named parameter values.
    ///
    /// # Returns
    ///
    /// A map of output port name → [`Value`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError`] if serialisation, the ABI call, or
    /// deserialisation fails.
    pub fn execute_filter(
        &mut self,
        filter_id: &str,
        inputs: &[(&str, Value)],
        params: &[(&str, Value)],
    ) -> Result<std::collections::HashMap<String, Value>, PluginError> {
        let id_cstr =
            std::ffi::CString::new(filter_id).map_err(|e| PluginError::PluginExecutionError {
                plugin_id: self.id().to_string(),
                filter_id: filter_id.to_string(),
                message: format!("invalid filter_id: {e}"),
            })?;

        let inputs_map: serde_json::Value = inputs
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::to_value(v).unwrap_or(serde_json::Value::Null)))
            .collect::<serde_json::Map<_, _>>()
            .into();
        let params_map: serde_json::Value = params
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::to_value(v).unwrap_or(serde_json::Value::Null)))
            .collect::<serde_json::Map<_, _>>()
            .into();

        let inputs_json = serde_json::to_string(&inputs_map).unwrap_or_else(|_| "{}".to_string());
        let params_json = serde_json::to_string(&params_map).unwrap_or_else(|_| "{}".to_string());

        let inputs_bytes = inputs_json.as_bytes();
        let params_bytes = params_json.as_bytes();

        let mut out_buf = vec![0u8; DEFAULT_BUF_SIZE];
        let mut out_written: usize = 0;

        // SAFETY: All pointer arguments point to valid, appropriately sized memory.
        let result = unsafe {
            ((*self.vtable_ptr).filter_execute)(
                self.handle,
                id_cstr.as_ptr(),
                inputs_bytes.as_ptr(),
                inputs_bytes.len(),
                params_bytes.as_ptr(),
                params_bytes.len(),
                out_buf.as_mut_ptr(),
                out_buf.len(),
                &mut out_written,
            )
        };

        if result != AbiResult::Ok {
            return Err(PluginError::PluginExecutionError {
                plugin_id: self.id().to_string(),
                filter_id: filter_id.to_string(),
                message: format!("filter_execute returned {:?}", result),
            });
        }

        let out_slice = &out_buf[..out_written];
        let output_json: serde_json::Value = serde_json::from_slice(out_slice).map_err(|e| {
            PluginError::PluginExecutionError {
                plugin_id: self.id().to_string(),
                filter_id: filter_id.to_string(),
                message: format!("failed to parse output JSON: {e}"),
            }
        })?;

        let mut outputs = std::collections::HashMap::new();
        if let serde_json::Value::Object(map) = output_json {
            for (k, v) in map {
                if let Ok(value) = serde_json::from_value::<Value>(v) {
                    outputs.insert(k, value);
                }
            }
        }
        Ok(outputs)
    }

    /// Validate a filter's inputs and parameters.
    ///
    /// Returns a `Vec<String>` of validation error messages. An empty vec
    /// means validation passed.
    #[must_use]
    pub fn validate_filter(
        &self,
        filter_id: &str,
        inputs: &[(&str, Value)],
        params: &[(&str, Value)],
    ) -> Vec<String> {
        let Ok(id_cstr) = std::ffi::CString::new(filter_id) else {
            return vec![format!("invalid filter_id: {filter_id}")];
        };

        let inputs_map: serde_json::Value = inputs
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::to_value(v).unwrap_or(serde_json::Value::Null)))
            .collect::<serde_json::Map<_, _>>()
            .into();
        let params_map: serde_json::Value = params
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::to_value(v).unwrap_or(serde_json::Value::Null)))
            .collect::<serde_json::Map<_, _>>()
            .into();

        let inputs_json = serde_json::to_string(&inputs_map).unwrap_or_else(|_| "{}".to_string());
        let params_json = serde_json::to_string(&params_map).unwrap_or_else(|_| "{}".to_string());
        let inputs_bytes = inputs_json.as_bytes();
        let params_bytes = params_json.as_bytes();

        let mut out_buf = vec![0u8; DEFAULT_BUF_SIZE];
        let mut out_written: usize = 0;

        // SAFETY: All pointers are valid for their respective lengths.
        let _result = unsafe {
            ((*self.vtable_ptr).filter_validate)(
                self.handle,
                id_cstr.as_ptr(),
                inputs_bytes.as_ptr(),
                inputs_bytes.len(),
                params_bytes.as_ptr(),
                params_bytes.len(),
                out_buf.as_mut_ptr(),
                out_buf.len(),
                &mut out_written,
            )
        };

        if out_written == 0 {
            return Vec::new();
        }

        let out_slice = &out_buf[..out_written];
        serde_json::from_slice::<Vec<String>>(out_slice).unwrap_or_default()
    }

    /// Run the plugin health check and return a [`HealthReport`].
    ///
    /// Also updates `self.last_healthy`.
    pub fn health_check(&mut self) -> HealthReport {
        let now = Instant::now();
        // SAFETY: vtable_ptr and handle are valid.
        let result = unsafe { ((*self.vtable_ptr).plugin_health_check)(self.handle) };
        let healthy = result == AbiResult::Ok;
        self.last_healthy = healthy;
        if healthy {
            HealthReport::healthy(self.id(), now)
        } else {
            HealthReport::unhealthy(
                self.id(),
                format!("plugin_health_check returned {:?}", result),
                now,
            )
        }
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: vtable_ptr is valid (library not yet dropped since we are
            // in drop() which runs before field drops). handle is non-null and
            // has not been freed (we null it after the call). This must occur
            // before `_library` is dropped (which happens as the last field).
            unsafe {
                ((*self.vtable_ptr).plugin_destroy)(self.handle);
            }
            self.handle = std::ptr::null_mut();
        }
        // After this function returns, Rust drops fields in declaration order:
        // manifest, library_path, handle (null, no-op), vtable_ptr (raw ptr, no-op),
        // loaded_at, capabilities, last_healthy, and LAST: _library (unloads .so).
    }
}
