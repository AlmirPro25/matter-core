//! Runtime support for Matter native executables
//!
//! Provides built-in functions and runtime services.

pub mod builtins;

use std::ptr;
#[cfg(all(feature = "jit-exec", target_os = "windows"))]
use windows_sys::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, VirtualProtect, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
    PAGE_EXECUTE_READ, PAGE_READWRITE,
};

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

pub struct ExecutableMemory {
    #[cfg(any(not(feature = "jit-exec"), not(target_os = "windows")))]
    code: Vec<u8>,
    #[cfg(all(feature = "jit-exec", target_os = "windows"))]
    ptr: *mut u8,
    #[cfg(all(feature = "jit-exec", target_os = "windows"))]
    len: usize,
}

impl ExecutableMemory {
    pub fn new(code: &[u8]) -> Result<Self, String> {
        #[cfg(all(feature = "jit-exec", target_os = "windows"))]
        {
            if code.is_empty() {
                return Err("empty machine code buffer".to_string());
            }
            unsafe {
                let mem = VirtualAlloc(
                    ptr::null_mut(),
                    code.len(),
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_READWRITE,
                ) as *mut u8;
                if mem.is_null() {
                    return Err("VirtualAlloc failed".to_string());
                }
                std::ptr::copy_nonoverlapping(code.as_ptr(), mem, code.len());
                let mut old_protect = 0u32;
                let ok = VirtualProtect(
                    mem as *mut _,
                    code.len(),
                    PAGE_EXECUTE_READ,
                    &mut old_protect as *mut u32,
                );
                if ok == 0 {
                    let _ = VirtualFree(mem as *mut _, 0, MEM_RELEASE);
                    return Err("VirtualProtect(PAGE_EXECUTE_READ) failed".to_string());
                }
                return Ok(Self {
                    ptr: mem,
                    len: code.len(),
                });
            }
        }

        #[cfg(any(not(feature = "jit-exec"), not(target_os = "windows")))]
        Ok(Self {
            code: code.to_vec(),
        })
    }

    /// Returns a pointer to the underlying code bytes.
    /// Useful for cache lookup interoperability in C ABI hooks.
    pub fn code_ptr(&self) -> *const u8 {
        #[cfg(all(feature = "jit-exec", target_os = "windows"))]
        {
            return self.ptr as *const u8;
        }
        #[cfg(any(not(feature = "jit-exec"), not(target_os = "windows")))]
        self.code.as_ptr()
    }

    /// # Safety
    ///
    /// This function is marked unsafe because future backends may execute raw
    /// machine code that must obey ABI and memory-safety contracts.
    pub unsafe fn execute_i64(&self, _runtime: &mut NativeRuntime) -> i64 {
        #[cfg(all(feature = "jit-exec", target_os = "windows"))]
        {
            let _ = self.len;
            type JitEntry = unsafe extern "C" fn(*mut NativeRuntime) -> i64;
            let entry: JitEntry = std::mem::transmute(self.ptr);
            return entry(_runtime as *mut NativeRuntime);
        }

        #[cfg(any(not(feature = "jit-exec"), not(target_os = "windows")))]
        {
            // Explicit failure path: avoid silently returning an incorrect value.
            let _ = self.code.len();
            panic!(
                "native JIT execution is unavailable: build with feature 'jit-exec' on Windows, or use the interpreter path"
            )
        }
    }
}

#[cfg(all(feature = "jit-exec", target_os = "windows"))]
impl Drop for ExecutableMemory {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                let _ = VirtualFree(self.ptr as *mut _, 0, MEM_RELEASE);
            }
        }
    }
}
