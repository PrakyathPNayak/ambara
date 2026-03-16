//! ComfyUI bridge plugin (scaffold).
//!
//! This crate is intentionally a minimal placeholder so the workspace can
//! compile while full ComfyUI bridge functionality is implemented.

use ambara::plugins::api::{AbiResult, PluginHandle, PluginVTable, HOST_ABI_VERSION};
use std::ffi::c_char;
use std::ptr;

#[repr(C)]
struct StubPlugin;

unsafe extern "C" fn plugin_create() -> *mut PluginHandle {
    let boxed = Box::new(StubPlugin);
    Box::into_raw(boxed) as *mut PluginHandle
}

unsafe extern "C" fn plugin_destroy(handle: *mut PluginHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut StubPlugin);
    }
}

unsafe extern "C" fn plugin_init(
    _handle: *mut PluginHandle,
    _config_json: *const u8,
    _config_len: usize,
) -> AbiResult {
    AbiResult::Ok
}

unsafe extern "C" fn filter_count(_handle: *const PluginHandle) -> usize {
    0
}

unsafe extern "C" fn filter_id_at(_handle: *const PluginHandle, _index: usize) -> *const c_char {
    ptr::null()
}

unsafe extern "C" fn filter_metadata_json(
    _handle: *const PluginHandle,
    _filter_id: *const c_char,
    _output_buf: *mut u8,
    _output_capacity: usize,
) -> usize {
    0
}

unsafe extern "C" fn filter_execute(
    _handle: *mut PluginHandle,
    _filter_id: *const c_char,
    _inputs_json: *const u8,
    _inputs_len: usize,
    _params_json: *const u8,
    _params_len: usize,
    _output_buf: *mut u8,
    _output_capacity: usize,
    _output_len: *mut usize,
) -> AbiResult {
    AbiResult::ErrNotSupported
}

unsafe extern "C" fn filter_validate(
    _handle: *const PluginHandle,
    _filter_id: *const c_char,
    _inputs_json: *const u8,
    _inputs_len: usize,
    _params_json: *const u8,
    _params_len: usize,
    _output_buf: *mut u8,
    _output_capacity: usize,
    _output_len: *mut usize,
) -> AbiResult {
    AbiResult::Ok
}

unsafe extern "C" fn plugin_health_check(
    _handle: *const PluginHandle,
) -> AbiResult {
    AbiResult::Ok
}

#[no_mangle]
pub static ambara_plugin_vtable: PluginVTable = PluginVTable {
    abi_version: HOST_ABI_VERSION,
    plugin_create,
    plugin_destroy,
    plugin_init,
    filter_count,
    filter_id_at,
    filter_metadata_json,
    filter_execute,
    filter_validate,
    plugin_health_check,
};
