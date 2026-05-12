//! Runtime support for Matter native executables
//!
//! Provides built-in functions and runtime services.

pub mod builtins;

use std::ptr;

/// Runtime context for native execution
pub struct Runtime {
    // Future: heap allocator, GC, etc.
}

impl Runtime {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime context used by JIT/native calls.
pub struct NativeRuntime {
    pub vm_ptr: *mut std::ffi::c_void,
    pub lookup_fn: extern "C" fn(*mut std::ffi::c_void, *const i8) -> *const u8,
    pub get_global_fn: extern "C" fn(*mut std::ffi::c_void, *const i8) -> i64,
    pub set_global_fn: extern "C" fn(*mut std::ffi::c_void, *const i8, i64),
}

impl NativeRuntime {
    pub fn new() -> Self {
        Self {
            vm_ptr: ptr::null_mut(),
            lookup_fn: default_lookup_fn,
            get_global_fn: default_get_global_fn,
            set_global_fn: default_set_global_fn,
        }
    }

    pub extern "C" fn print_int(value: i64) {
        builtins::matter_print_int(value);
    }
}

extern "C" fn default_lookup_fn(_vm_ptr: *mut std::ffi::c_void, _name: *const i8) -> *const u8 {
    ptr::null()
}

extern "C" fn default_get_global_fn(_vm_ptr: *mut std::ffi::c_void, _name: *const i8) -> i64 {
    0
}

extern "C" fn default_set_global_fn(_vm_ptr: *mut std::ffi::c_void, _name: *const i8, _value: i64) {
}

impl Default for NativeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct ExecutableMemory {
    code: Vec<u8>,
}

impl ExecutableMemory {
    pub fn new(code: &[u8]) -> Result<Self, String> {
        Ok(Self {
            code: code.to_vec(),
        })
    }

    /// Returns a pointer to the underlying code bytes.
    /// Useful for cache lookup interoperability in C ABI hooks.
    pub fn code_ptr(&self) -> *const u8 {
        self.code.as_ptr()
    }

    /// # Safety
    ///
    /// This function is marked unsafe because future backends may execute raw
    /// machine code that must obey ABI and memory-safety contracts.
    pub unsafe fn execute_i64(&self, _runtime: &mut NativeRuntime) -> i64 {
        // Minimal safe fallback until the native executable backend owns platform memory mapping.
        let _ = self.code.len();
        0
    }
}
