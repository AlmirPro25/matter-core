//! Matter Rust Bridge - dynamic Rust FFI bridge.
//!
//! Dynamic modules use a small JSON ABI:
//! `extern "C" fn symbol(args_json: *const c_char) -> *mut c_char`.
//! `args_json` is a JSON array of typed Matter values and the return value is
//! one typed Matter value. A module may export `matter_free_string` to free the
//! returned allocation.

use matter_backend::Value;
use matter_polyglot::{bridge::LanguageBridge, LanguageTarget};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

type MatterRustFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type MatterRustFree = unsafe extern "C" fn(*mut c_char);

pub struct RustBridge {
    loaded_crates: HashMap<String, LoadedRustModule>,
    initialized: bool,
}

struct LoadedRustModule {
    #[allow(dead_code)]
    name: String,
    library: Option<libloading::Library>,
}

impl RustBridge {
    pub fn new() -> Self {
        Self {
            loaded_crates: HashMap::new(),
            initialized: false,
        }
    }

    fn load_rust_module(&self, module: &str) -> Result<LoadedRustModule, String> {
        let path = Path::new(module);
        if path.exists() {
            let library = unsafe { libloading::Library::new(path) }
                .map_err(|e| format!("Failed to load Rust dynamic library '{}': {}", module, e))?;
            return Ok(LoadedRustModule {
                name: module.to_string(),
                library: Some(library),
            });
        }

        println!(
            "Rust crate '{}' registered for compile-time linking",
            module
        );
        Ok(LoadedRustModule {
            name: module.to_string(),
            library: None,
        })
    }

    fn call_dynamic_function(
        &self,
        module: &str,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let loaded = self
            .loaded_crates
            .get(module)
            .ok_or_else(|| format!("Crate '{}' not imported", module))?;
        let library = loaded.library.as_ref().ok_or_else(|| {
            format!(
                "Rust crate '{}' is registered only; import a dynamic library path to call functions",
                module
            )
        })?;

        let args_json = serde_json::to_string(
            &args
                .iter()
                .map(encode_value_json)
                .collect::<Vec<serde_json::Value>>(),
        )
        .map_err(|e| format!("Failed to encode Rust FFI arguments: {}", e))?;
        let args_c = CString::new(args_json)
            .map_err(|_| "Rust FFI arguments contain an interior NUL byte".to_string())?;

        unsafe {
            let symbol: libloading::Symbol<MatterRustFn> = library
                .get(function.as_bytes())
                .map_err(|e| format!("Failed to load Rust symbol '{}': {}", function, e))?;
            let result_ptr = symbol(args_c.as_ptr());
            if result_ptr.is_null() {
                return Err(format!(
                    "Rust FFI function '{}::{}' returned null",
                    module, function
                ));
            }

            let result_bytes = CStr::from_ptr(result_ptr).to_bytes().to_vec();

            if let Ok(free_symbol) = library.get::<MatterRustFree>(b"matter_free_string") {
                free_symbol(result_ptr);
            }

            let result_json = String::from_utf8(result_bytes)
                .map_err(|e| format!("Rust FFI returned invalid UTF-8: {}", e))?;
            let json: serde_json::Value = serde_json::from_str(&result_json)
                .map_err(|e| format!("Rust FFI returned invalid JSON: {}", e))?;
            decode_value_json(&json)
        }
    }
}

impl Default for RustBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageBridge for RustBridge {
    fn language(&self) -> LanguageTarget {
        LanguageTarget::Rust
    }

    fn initialize(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        let output = std::process::Command::new("cargo")
            .arg("--version")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                self.initialized = true;
                Ok(())
            }
            _ => Err("Cargo not found. Please install Rust toolchain.".to_string()),
        }
    }

    fn import_module(&mut self, module: &str) -> Result<(), String> {
        let loaded = self.load_rust_module(module)?;
        self.loaded_crates.insert(module.to_string(), loaded);
        Ok(())
    }

    fn call_function(
        &mut self,
        module: &str,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        self.call_dynamic_function(module, function, args)
    }

    fn get_attribute(&mut self, module: &str, attribute: &str) -> Result<Value, String> {
        if !self.loaded_crates.contains_key(module) {
            return Err(format!("Crate '{}' not imported", module));
        }

        Err(format!(
            "Rust attribute access not implemented for dynamic symbol '{}::{}'",
            module, attribute
        ))
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.loaded_crates.clear();
        self.initialized = false;
        Ok(())
    }
}

pub fn encode_value_json(value: &Value) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    match value {
        Value::Int(n) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("int".to_string()),
            );
            object.insert("value".to_string(), serde_json::Value::Number((*n).into()));
        }
        Value::Float(f) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("float".to_string()),
            );
            object.insert(
                "value".to_string(),
                serde_json::Number::from_f64(*f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null),
            );
        }
        Value::Bool(b) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("bool".to_string()),
            );
            object.insert("value".to_string(), serde_json::Value::Bool(*b));
        }
        Value::String(s) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("string".to_string()),
            );
            object.insert(
                "value".to_string(),
                serde_json::Value::String(s.as_str().to_string()),
            );
        }
        Value::Unit => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("unit".to_string()),
            );
        }
        Value::List(items) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("list".to_string()),
            );
            object.insert(
                "value".to_string(),
                serde_json::Value::Array(items.iter().map(encode_value_json).collect()),
            );
        }
        Value::Map(map) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("map".to_string()),
            );
            let mut values = serde_json::Map::new();
            for (key, value) in map.iter() {
                values.insert(key.clone(), encode_value_json(value));
            }
            object.insert("value".to_string(), serde_json::Value::Object(values));
        }
        Value::Struct { type_name, fields } => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("struct".to_string()),
            );
            object.insert(
                "name".to_string(),
                serde_json::Value::String(type_name.as_str().to_string()),
            );
            let mut values = serde_json::Map::new();
            for (key, value) in fields.iter() {
                values.insert(key.clone(), encode_value_json(value));
            }
            object.insert("value".to_string(), serde_json::Value::Object(values));
        }
        Value::Function(name) => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("function".to_string()),
            );
            object.insert(
                "value".to_string(),
                serde_json::Value::String(name.as_str().to_string()),
            );
        }
        Value::Null => {
            object.insert(
                "type".to_string(),
                serde_json::Value::String("null".to_string()),
            );
        }
    }
    serde_json::Value::Object(object)
}

pub fn decode_value_json(json: &serde_json::Value) -> Result<Value, String> {
    let object = json
        .as_object()
        .ok_or_else(|| "Rust FFI value must be an object".to_string())?;
    let value_type = object
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Rust FFI value is missing type".to_string())?;

    match value_type {
        "error" => Err(object
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Rust FFI function returned an error")
            .to_string()),
        "int" => Ok(Value::Int(
            object
                .get("value")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "Rust FFI int is invalid".to_string())?,
        )),
        "float" => Ok(Value::Float(
            object
                .get("value")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| "Rust FFI float is invalid".to_string())?,
        )),
        "bool" => Ok(Value::Bool(
            object
                .get("value")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| "Rust FFI bool is invalid".to_string())?,
        )),
        "string" => Ok(Value::new_string(
            object
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Rust FFI string is invalid".to_string())?
                .to_string(),
        )),
        "unit" => Ok(Value::Unit),
        "null" => Ok(Value::Null),
        "list" => {
            let values = object
                .get("value")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "Rust FFI list is invalid".to_string())?;
            values
                .iter()
                .map(decode_value_json)
                .collect::<Result<Vec<_>, _>>()
                .map(Value::new_list)
        }
        "map" => {
            let values = object
                .get("value")
                .and_then(|v| v.as_object())
                .ok_or_else(|| "Rust FFI map is invalid".to_string())?;
            let mut map = HashMap::new();
            for (key, value) in values {
                map.insert(key.clone(), decode_value_json(value)?);
            }
            Ok(Value::new_map(map))
        }
        "struct" => {
            let type_name = object
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Rust FFI struct is missing name".to_string())?
                .to_string();
            let values = object
                .get("value")
                .and_then(|v| v.as_object())
                .ok_or_else(|| "Rust FFI struct fields are invalid".to_string())?;
            let mut fields = HashMap::new();
            for (key, value) in values {
                fields.insert(key.clone(), decode_value_json(value)?);
            }
            Ok(Value::new_struct(type_name, fields))
        }
        "function" => Ok(Value::new_function(
            object
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Rust FFI function is invalid".to_string())?
                .to_string(),
        )),
        _ => Err(format!("unknown Rust FFI value type '{}'", value_type)),
    }
}

pub fn decode_args_json(value: &serde_json::Value) -> Result<Vec<Value>, String> {
    let array = value
        .as_array()
        .ok_or_else(|| "Rust FFI args_json must be an array".to_string())?;
    array.iter().map(decode_value_json).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn test_rust_bridge_initialization() {
        let mut bridge = RustBridge::new();
        let _ = bridge.initialize();
    }

    #[test]
    fn test_import_crate() {
        let mut bridge = RustBridge::new();
        bridge.initialize().ok();

        let result = bridge.import_module("serde");
        assert!(result.is_ok());
        assert!(bridge.loaded_crates.contains_key("serde"));
    }

    #[test]
    fn test_registered_crate_call_reports_not_dynamic() {
        let mut bridge = RustBridge::new();
        bridge.import_module("serde").unwrap();

        let err = bridge
            .call_function("serde", "anything", vec![])
            .expect_err("registered-only crates cannot be called dynamically");

        assert!(err.contains("registered only"));
    }

    #[test]
    fn test_bridge_value_codec_roundtrips_scalars_and_composites() {
        let mut map = HashMap::new();
        map.insert("answer".to_string(), Value::Int(42));
        map.insert("name".to_string(), Value::new_string("matter".to_string()));

        let mut fields = HashMap::new();
        fields.insert("enabled".to_string(), Value::Bool(true));

        let values = vec![
            Value::Int(7),
            Value::Float(2.5),
            Value::Bool(false),
            Value::new_string("hello".to_string()),
            Value::Unit,
            Value::Null,
            Value::new_list(vec![Value::Int(1), Value::new_string("two".to_string())]),
            Value::new_map(map),
            Value::new_struct("Config".to_string(), fields),
            Value::new_function("run".to_string()),
        ];

        for value in values {
            let encoded = encode_value_json(&value);
            let decoded = decode_value_json(&encoded).unwrap();
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn test_bridge_value_codec_reports_formal_errors() {
        let err = decode_value_json(&serde_json::json!({
            "type": "error",
            "message": "plugin failed"
        }))
        .unwrap_err();
        assert_eq!(err, "plugin failed");

        let err = decode_value_json(&serde_json::json!({
            "type": "error"
        }))
        .unwrap_err();
        assert_eq!(err, "Rust FFI function returned an error");
    }

    #[test]
    fn test_bridge_value_codec_rejects_invalid_payloads() {
        assert!(decode_value_json(&serde_json::json!(null))
            .unwrap_err()
            .contains("must be an object"));

        assert!(decode_value_json(&serde_json::json!({ "value": 1 }))
            .unwrap_err()
            .contains("missing type"));

        assert!(decode_value_json(&serde_json::json!({ "type": "int" }))
            .unwrap_err()
            .contains("int is invalid"));
    }

    #[test]
    fn test_dynamic_library_call() {
        if Command::new("rustc").arg("--version").output().is_err() {
            return;
        }

        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("matter_rust_bridge_test.rs");
        let library = temp
            .path()
            .join(dynamic_library_name("matter_rust_bridge_test"));
        fs::write(
            &source,
            r#"
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn add_one(args: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(args) }.to_string_lossy();
    let value = if input.contains("\"value\":41") { 42 } else { 0 };
    CString::new(format!("{{\"type\":\"int\",\"value\":{}}}", value)).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn describe(args: *const c_char) -> *mut c_char {
    let input = unsafe { CStr::from_ptr(args) }.to_string_lossy();
    let value = if input.contains("matter") { "hello matter" } else { "unknown" };
    CString::new(format!("{{\"type\":\"string\",\"value\":\"{}\"}}", value)).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn pair(_args: *const c_char) -> *mut c_char {
    CString::new("{\"type\":\"list\",\"value\":[{\"type\":\"int\",\"value\":1},{\"type\":\"string\",\"value\":\"two\"}]}").unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn fail(_args: *const c_char) -> *mut c_char {
    CString::new("{\"type\":\"error\",\"message\":\"plugin failed\"}").unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn matter_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)); }
    }
}
"#,
        )
        .unwrap();

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("cdylib")
            .arg(&source)
            .arg("-o")
            .arg(&library)
            .output()
            .unwrap();

        if !output.status.success() {
            panic!("rustc failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let module = library.to_string_lossy().to_string();
        let mut bridge = RustBridge::new();
        bridge.import_module(&module).unwrap();

        let result = bridge
            .call_function(&module, "add_one", vec![Value::Int(41)])
            .unwrap();

        assert_eq!(result, Value::Int(42));

        let result = bridge
            .call_function(
                &module,
                "describe",
                vec![Value::new_string("matter".to_string())],
            )
            .unwrap();
        assert_eq!(result, Value::new_string("hello matter".to_string()));

        let result = bridge.call_function(&module, "pair", vec![]).unwrap();
        assert_eq!(
            result,
            Value::new_list(vec![Value::Int(1), Value::new_string("two".to_string())])
        );

        let err = bridge
            .call_function(&module, "fail", vec![])
            .expect_err("error values must become bridge errors");
        assert_eq!(err, "plugin failed");
    }

    fn dynamic_library_name(stem: &str) -> String {
        if cfg!(windows) {
            format!("{}.dll", stem)
        } else if cfg!(target_os = "macos") {
            format!("lib{}.dylib", stem)
        } else {
            format!("lib{}.so", stem)
        }
    }
}
