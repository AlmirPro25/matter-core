// Matter Bridge: Go Native (cgo FFI)
// Direct FFI to Go libraries using cgo
// Performance: 100-1000x faster than subprocess

use matter_ast::Value;
use matter_error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::{Arc, Mutex};

fn lock_unpoison<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Go function pointer type
type GoFunc = unsafe extern "C" fn(*const c_char) -> *mut c_char;

/// Go bridge with direct FFI via cgo
pub struct GoBridge {
    /// Loaded Go shared libraries
    libraries: Arc<Mutex<HashMap<String, libloading::Library>>>,
    /// Cached function pointers
    functions: Arc<Mutex<HashMap<String, GoFunc>>>,
}

impl GoBridge {
    /// Create a new Go bridge
    pub fn new() -> Self {
        Self {
            libraries: Arc::new(Mutex::new(HashMap::new())),
            functions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Load a Go package (compiled as shared library)
    pub fn load_package(&self, package: &str) -> Result<()> {
        let lib_name = format!("lib{}.so", package.replace("/", "_"));

        let lib = unsafe {
            libloading::Library::new(&lib_name).map_err(|e| {
                Error::Runtime(format!("Failed to load Go library {}: {}", lib_name, e))
            })?
        };

        lock_unpoison(&self.libraries).insert(package.to_string(), lib);
        Ok(())
    }

    /// Call a Go function with direct FFI
    pub fn call_function(&self, package: &str, function: &str, args: Vec<Value>) -> Result<Value> {
        // Get or load function pointer
        let func_key = format!("{}::{}", package, function);

        let func = {
            let mut functions = lock_unpoison(&self.functions);

            if let Some(func) = functions.get(&func_key) {
                *func
            } else {
                // Load function from library
                let libraries = lock_unpoison(&self.libraries);
                let lib = libraries
                    .get(package)
                    .ok_or_else(|| Error::Runtime(format!("Go package not loaded: {}", package)))?;

                let func: GoFunc = unsafe {
                    let symbol = lib.get(function.as_bytes()).map_err(|e| {
                        Error::Runtime(format!("Function not found: {}: {}", function, e))
                    })?;
                    *symbol
                };

                functions.insert(func_key.clone(), func);
                func
            }
        };

        // Convert Matter values to JSON for Go
        let json_args = serde_json::to_string(&args)
            .map_err(|e| Error::Runtime(format!("Failed to serialize args: {}", e)))?;

        let c_args = CString::new(json_args)
            .map_err(|e| Error::Runtime(format!("Invalid string: {}", e)))?;

        // Call Go function via FFI
        let result_ptr = unsafe { func(c_args.as_ptr()) };

        if result_ptr.is_null() {
            return Err(Error::Runtime("Go function returned null".to_string()));
        }

        // Convert result back to Matter value
        let result_str = unsafe {
            let c_str = CStr::from_ptr(result_ptr);
            let str_slice = c_str
                .to_str()
                .map_err(|e| Error::Runtime(format!("Invalid UTF-8 from Go: {}", e)))?;
            str_slice.to_string()
        };

        // Free Go-allocated memory
        unsafe {
            libc::free(result_ptr as *mut c_void);
        }

        // Parse JSON result
        let value: Value = serde_json::from_str(&result_str)
            .map_err(|e| Error::Runtime(format!("Failed to parse Go result: {}", e)))?;

        Ok(value)
    }

    /// Import a Go package
    pub fn import(&self, package: &str, alias: Option<&str>) -> Result<GoPackage> {
        self.load_package(package)?;

        Ok(GoPackage {
            bridge: self.clone(),
            package: package.to_string(),
            alias: alias.map(|s| s.to_string()),
        })
    }
}

impl Clone for GoBridge {
    fn clone(&self) -> Self {
        Self {
            libraries: Arc::clone(&self.libraries),
            functions: Arc::clone(&self.functions),
        }
    }
}

impl Default for GoBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents an imported Go package
pub struct GoPackage {
    bridge: GoBridge,
    package: String,
    alias: Option<String>,
}

impl GoPackage {
    /// Call a function from this package
    pub fn call(&self, function: &str, args: Vec<Value>) -> Result<Value> {
        self.bridge.call_function(&self.package, function, args)
    }

    /// Get package name
    pub fn name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.package)
    }
}

/// Convert Matter value to Go-compatible JSON
pub fn value_to_go_json(value: &Value) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|e| Error::Runtime(format!("Failed to convert to Go JSON: {}", e)))
}

/// Convert Go JSON to Matter value
pub fn go_json_to_value(json: &str) -> Result<Value> {
    serde_json::from_str(json)
        .map_err(|e| Error::Runtime(format!("Failed to parse Go JSON: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_bridge_creation() {
        let bridge = GoBridge::new();
        assert!(bridge.libraries.lock().unwrap().is_empty());
    }

    #[test]
    fn test_value_conversion() {
        let value = Value::Number(42.0);
        let json = value_to_go_json(&value).unwrap();
        assert_eq!(json, "42.0");

        let back = go_json_to_value(&json).unwrap();
        assert_eq!(back, value);
    }
}
