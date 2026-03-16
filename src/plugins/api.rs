//! # Stable Plugin C ABI
//!
//! This module defines the **stable binary interface** between the Ambara host
//! process and external plugin shared libraries. Every public type here is
//! `#[repr(C)]` and must remain layout-compatible across Ambara releases.
//!
//! ## Versioning Contract
//!
//! [`HOST_ABI_VERSION`] is incremented **only on breaking changes**. A plugin
//! compiled against an older version with the same major ABI version _may_ still
//! work if no breaking fields were accessed. The host refuses to load any plugin
//! whose `PluginVTable::abi_version` differs from `HOST_ABI_VERSION`.
//!
//! ## ABI Rules (MANDATORY for plugin authors)
//!
//! 1. Export the vtable as a `static PluginVTable` named `ambara_plugin_vtable`.
//! 2. All function pointers must be `unsafe extern "C"`.
//! 3. String data (pointers + lengths) must be valid UTF-8.
//! 4. Every `extern "C"` function body **must** wrap its logic in
//!    `std::panic::catch_unwind` to prevent unwinding across the FFI boundary.
//! 5. The plugin must **never** call back into Ambara library functions.
//! 6. New fields in `PluginVTable` go at the **end** only — never in the middle.
//!
//! ## Examples
//!
//! ```rust,ignore
//! // In your plugin crate:
//! use ambara_plugin_abi::*;
//!
//! #[no_mangle]
//! pub static ambara_plugin_vtable: PluginVTable = PluginVTable {
//!     abi_version: HOST_ABI_VERSION,
//!     plugin_create: my_plugin_create,
//!     plugin_destroy: my_plugin_destroy,
//!     // ...
//! };
//! ```

use std::ffi::c_char;

/// The ABI version this build of Ambara supports.
///
/// Plugins must export a [`PluginVTable`] whose `abi_version` field equals
/// this constant. Increment this constant when the vtable layout changes.
pub const HOST_ABI_VERSION: u32 = 1;

/// Opaque plugin instance handle.
///
/// Allocated and owned by the plugin's [`PluginVTable::plugin_create`] function.
/// The host holds this pointer and passes it back on every subsequent call.
/// The host never dereferences or frees this pointer directly.
#[repr(C)]
pub struct PluginHandle {
    _opaque: [u8; 0],
}

/// A non-owning UTF-8 string slice passed across the FFI boundary.
///
/// The lifetime of the data pointed to by `ptr` must be clearly documented
/// for each function that uses this type. Generally, data is valid until the
/// next call into the plugin.
#[repr(C)]
pub struct AbiStr {
    /// Pointer to UTF-8 encoded bytes. May not be null.
    pub ptr: *const u8,
    /// Number of bytes (not characters).
    pub len: usize,
}

/// Result codes returned by plugin functions.
///
/// `AbiResult::Ok` (0) indicates success. All other values indicate failure,
/// and the host will propagate an appropriate [`crate::core::error::PluginError`].
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiResult {
    /// Operation completed successfully.
    Ok = 0,
    /// Unclassified failure; treat as fatal.
    ErrUnknown = 1,
    /// Plugin initialisation failed.
    ErrInit = 2,
    /// An input value was invalid or missing.
    ErrInvalidInput = 3,
    /// Filter execution failed.
    ErrExecution = 4,
    /// The requested operation is not supported by this plugin.
    ErrNotSupported = 5,
}

/// The vtable exported by every Ambara plugin.
///
/// The symbol name in the shared library **must** be `ambara_plugin_vtable`.
/// Fields are laid out in C order and must not be reordered. New fields may
/// only be appended at the end.
///
/// # Safety
///
/// All function pointers are `unsafe extern "C"`. Callers must uphold the
/// invariants documented on each field.
#[repr(C)]
pub struct PluginVTable {
    /// ABI version the plugin was compiled against.
    ///
    /// Must equal [`HOST_ABI_VERSION`]. The host refuses to load the plugin
    /// if this field differs.
    pub abi_version: u32,

    /// Allocate a new plugin instance on the heap.
    ///
    /// Called exactly once at load time. Returns a non-null pointer to a
    /// plugin-owned [`PluginHandle`] on success, or null on failure.
    ///
    /// # Safety
    ///
    /// The returned pointer must remain valid until [`plugin_destroy`] is called.
    pub plugin_create: unsafe extern "C" fn() -> *mut PluginHandle,

    /// Deallocate the plugin instance.
    ///
    /// Called exactly once at unload time. The plugin must release all
    /// resources and invalidate the handle. The host will not use the handle
    /// after this call.
    ///
    /// # Safety
    ///
    /// `handle` must be the pointer returned by [`plugin_create`] and must
    /// not have been freed already.
    pub plugin_destroy: unsafe extern "C" fn(handle: *mut PluginHandle),

    /// Initialise the plugin with host configuration.
    ///
    /// Called after [`plugin_create`] and before any filter calls.
    ///
    /// # Arguments
    ///
    /// * `handle` - Plugin instance.
    /// * `config_json` - UTF-8 JSON configuration object (`config_len` bytes).
    ///
    /// # Safety
    ///
    /// `config_json` must point to `config_len` valid UTF-8 bytes.
    pub plugin_init: unsafe extern "C" fn(
        handle: *mut PluginHandle,
        config_json: *const u8,
        config_len: usize,
    ) -> AbiResult,

    /// Return the number of filter types this plugin provides.
    ///
    /// Valid indices are `0..filter_count(handle)`.
    ///
    /// # Safety
    ///
    /// `handle` must be a valid, initialised plugin instance.
    pub filter_count: unsafe extern "C" fn(handle: *const PluginHandle) -> usize,

    /// Return a null-terminated C string ID for the filter at `index`.
    ///
    /// Returns null if `index` is out of range. The pointer is valid until
    /// [`plugin_destroy`] is called or the plugin is re-initialised.
    ///
    /// # Safety
    ///
    /// `handle` must be a valid, initialised plugin instance. `index` must be
    /// less than `filter_count(handle)`.
    pub filter_id_at: unsafe extern "C" fn(
        handle: *const PluginHandle,
        index: usize,
    ) -> *const c_char,

    /// Serialise the [`crate::core::node::NodeMetadata`] for a filter to JSON.
    ///
    /// Writes UTF-8 JSON into `out_buf[0..out_buf_len]`. Returns the number of
    /// bytes written, or 0 on error. The JSON is **not** null-terminated.
    ///
    /// # Safety
    ///
    /// `handle` and `filter_id` must be valid. `out_buf` must point to a
    /// writable buffer of at least `out_buf_len` bytes.
    pub filter_metadata_json: unsafe extern "C" fn(
        handle: *const PluginHandle,
        filter_id: *const c_char,
        out_buf: *mut u8,
        out_buf_len: usize,
    ) -> usize,

    /// Execute a filter node.
    ///
    /// All complex data (inputs, params, outputs) cross the boundary as
    /// UTF-8 JSON. Images are base64-encoded PNG inside the JSON value.
    ///
    /// # Arguments
    ///
    /// * `handle` - Plugin instance.
    /// * `filter_id` - Null-terminated filter ID string.
    /// * `inputs_json` / `inputs_len` - Serialised input `Value` map.
    /// * `params_json` / `params_len` - Serialised parameter `Value` map.
    /// * `out_buf` / `out_buf_len` - Caller-allocated output buffer.
    /// * `out_written` - Set to the number of bytes written to `out_buf`.
    ///
    /// # Returns
    ///
    /// [`AbiResult::Ok`] on success, error code otherwise.
    ///
    /// # Safety
    ///
    /// All pointers must be valid for their respective lengths.
    pub filter_execute: unsafe extern "C" fn(
        handle: *mut PluginHandle,
        filter_id: *const c_char,
        inputs_json: *const u8,
        inputs_len: usize,
        params_json: *const u8,
        params_len: usize,
        out_buf: *mut u8,
        out_buf_len: usize,
        out_written: *mut usize,
    ) -> AbiResult,

    /// Validate a filter node's inputs and parameters.
    ///
    /// Writes a JSON array of validation error strings into `out_buf`.
    /// An empty array `[]` means validation passed.
    ///
    /// # Safety
    ///
    /// Same requirements as [`filter_execute`].
    pub filter_validate: unsafe extern "C" fn(
        handle: *const PluginHandle,
        filter_id: *const c_char,
        inputs_json: *const u8,
        inputs_len: usize,
        params_json: *const u8,
        params_len: usize,
        out_buf: *mut u8,
        out_buf_len: usize,
        out_written: *mut usize,
    ) -> AbiResult,

    /// Periodic health check.
    ///
    /// Returns [`AbiResult::Ok`] if the plugin is functioning normally. If
    /// this returns an error, the host will mark the plugin as unhealthy and
    /// stop routing new work to it.
    ///
    /// # Safety
    ///
    /// `handle` must be a valid, initialised plugin instance.
    pub plugin_health_check:
        unsafe extern "C" fn(handle: *const PluginHandle) -> AbiResult,
}

// SAFETY: PluginVTable contains only function pointers which are inherently
// Send and Sync (they are just addresses).
unsafe impl Send for PluginVTable {}
unsafe impl Sync for PluginVTable {}

// SAFETY: Same reasoning as above.
unsafe impl Send for PluginHandle {}
unsafe impl Sync for PluginHandle {}
