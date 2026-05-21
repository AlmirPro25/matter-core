use matter_backend::Value;
use matter_error::{ErrorType, MatterError};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

type Result<T> = std::result::Result<T, MatterError>;

fn runtime_error(message: impl Into<String>) -> MatterError {
    MatterError::new(ErrorType::Runtime, message)
}

fn lock_unpoison<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

type GoFunc = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type GoFree = unsafe extern "C" fn(*mut c_char);

#[derive(Clone)]
pub struct GoBridge {
    libraries: Arc<Mutex<HashMap<String, &'static libloading::Library>>>,
    functions: Arc<Mutex<HashMap<String, GoFunc>>>,
    free_functions: Arc<Mutex<HashMap<String, GoFree>>>,
}

impl GoBridge {
    pub fn new() -> Self {
        Self {
            libraries: Arc::new(Mutex::new(HashMap::new())),
            functions: Arc::new(Mutex::new(HashMap::new())),
            free_functions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn load_package(&self, package: &str) -> Result<()> {
        let library_path = resolve_go_library(package);
        let library = unsafe {
            libloading::Library::new(&library_path).map_err(|error| {
                runtime_error(format!(
                    "Failed to load Go library '{}': {}",
                    library_path.display(),
                    error
                ))
            })?
        };

        // Go c-shared libraries host a Go runtime and are not safe to unload
        // with FreeLibrary/dlclose while the process continues. Keep them
        // loaded for the process lifetime.
        let library = Box::leak(Box::new(library));
        lock_unpoison(&self.libraries).insert(package.to_string(), library);
        Ok(())
    }

    pub fn call_function(&self, package: &str, function: &str, args: Vec<Value>) -> Result<Value> {
        let func_key = format!("{}::{}", package, function);
        let func = {
            let mut functions = lock_unpoison(&self.functions);
            if let Some(func) = functions.get(&func_key) {
                *func
            } else {
                let libraries = lock_unpoison(&self.libraries);
                let library = libraries
                    .get(package)
                    .ok_or_else(|| runtime_error(format!("Go package not loaded: {}", package)))?;
                let symbol = unsafe {
                    library
                        .get::<GoFunc>(function.as_bytes())
                        .map_err(|error| {
                            runtime_error(format!("Function not found '{}': {}", function, error))
                        })?
                };
                let func = *symbol;
                functions.insert(func_key, func);
                func
            }
        };

        let json_args = encode_args_json(&args)?;
        let c_args = CString::new(json_args)
            .map_err(|error| runtime_error(format!("Invalid Go FFI args string: {}", error)))?;

        let result_ptr = unsafe { func(c_args.as_ptr()) };
        if result_ptr.is_null() {
            return Err(runtime_error("Go function returned null"));
        }

        let result = unsafe {
            CStr::from_ptr(result_ptr)
                .to_str()
                .map_err(|error| runtime_error(format!("Invalid UTF-8 from Go: {}", error)))?
                .to_string()
        };

        self.free_go_string(package, result_ptr);

        decode_value_json(&result).map_err(runtime_error)
    }

    pub fn import(&self, package: &str, alias: Option<&str>) -> Result<GoPackage> {
        self.load_package(package)?;
        Ok(GoPackage {
            bridge: self.clone(),
            package: package.to_string(),
            alias: alias.map(str::to_string),
        })
    }

    fn free_go_string(&self, package: &str, ptr: *mut c_char) {
        let free = {
            let mut free_functions = lock_unpoison(&self.free_functions);
            if let Some(free) = free_functions.get(package) {
                Some(*free)
            } else {
                let libraries = lock_unpoison(&self.libraries);
                let free = libraries.get(package).and_then(|library| unsafe {
                    library
                        .get::<GoFree>(b"matter_free_string")
                        .ok()
                        .map(|symbol| *symbol)
                });
                if let Some(free) = free {
                    free_functions.insert(package.to_string(), free);
                }
                free
            }
        };

        unsafe {
            if let Some(free) = free {
                free(ptr);
            } else {
                libc::free(ptr as *mut c_void);
            }
        }
    }
}

impl Default for GoBridge {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GoPackage {
    bridge: GoBridge,
    package: String,
    alias: Option<String>,
}

impl GoPackage {
    pub fn call(&self, function: &str, args: Vec<Value>) -> Result<Value> {
        self.bridge.call_function(&self.package, function, args)
    }

    pub fn name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.package)
    }
}

pub fn value_to_go_json(value: &Value) -> Result<String> {
    Ok(encode_value(value).to_string())
}

pub fn go_json_to_value(json: &str) -> Result<Value> {
    decode_value_json(json).map_err(runtime_error)
}

fn resolve_go_library(package: &str) -> PathBuf {
    let path = Path::new(package);
    if path.exists() || path.extension().is_some() || package.contains(std::path::MAIN_SEPARATOR) {
        return path.to_path_buf();
    }

    let file_name = if cfg!(target_os = "windows") {
        format!("{}.dll", package.replace(['/', '-'], "_"))
    } else if cfg!(target_os = "macos") {
        format!("lib{}.dylib", package.replace(['/', '-'], "_"))
    } else {
        format!("lib{}.so", package.replace(['/', '-'], "_"))
    };
    PathBuf::from(file_name)
}

fn encode_args_json(args: &[Value]) -> Result<String> {
    serde_json::to_string(&args.iter().map(encode_value).collect::<Vec<_>>())
        .map_err(|error| runtime_error(format!("Failed to serialize Go args: {}", error)))
}

fn encode_value(value: &Value) -> JsonValue {
    let mut object = serde_json::Map::new();
    match value {
        Value::Int(value) => {
            object.insert("type".to_string(), JsonValue::String("int".to_string()));
            object.insert("value".to_string(), JsonValue::Number((*value).into()));
        }
        Value::Float(value) => {
            object.insert("type".to_string(), JsonValue::String("float".to_string()));
            object.insert(
                "value".to_string(),
                serde_json::Number::from_f64(*value)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null),
            );
        }
        Value::Bool(value) => {
            object.insert("type".to_string(), JsonValue::String("bool".to_string()));
            object.insert("value".to_string(), JsonValue::Bool(*value));
        }
        Value::String(value) => {
            object.insert("type".to_string(), JsonValue::String("string".to_string()));
            object.insert("value".to_string(), JsonValue::String((**value).clone()));
        }
        Value::Unit => {
            object.insert("type".to_string(), JsonValue::String("unit".to_string()));
        }
        Value::Null => {
            object.insert("type".to_string(), JsonValue::String("null".to_string()));
        }
        Value::List(values) => {
            object.insert("type".to_string(), JsonValue::String("list".to_string()));
            object.insert(
                "value".to_string(),
                JsonValue::Array(values.iter().map(encode_value).collect()),
            );
        }
        Value::Map(values) => {
            object.insert("type".to_string(), JsonValue::String("map".to_string()));
            object.insert(
                "value".to_string(),
                JsonValue::Object(
                    values
                        .iter()
                        .map(|(key, value)| (key.clone(), encode_value(value)))
                        .collect(),
                ),
            );
        }
        Value::Struct { type_name, fields } => {
            object.insert("type".to_string(), JsonValue::String("struct".to_string()));
            object.insert("name".to_string(), JsonValue::String((**type_name).clone()));
            object.insert(
                "value".to_string(),
                JsonValue::Object(
                    fields
                        .iter()
                        .map(|(key, value)| (key.clone(), encode_value(value)))
                        .collect(),
                ),
            );
        }
        Value::Function(name) => {
            object.insert(
                "type".to_string(),
                JsonValue::String("function".to_string()),
            );
            object.insert("value".to_string(), JsonValue::String((**name).clone()));
        }
    }
    JsonValue::Object(object)
}

fn decode_value_json(json: &str) -> std::result::Result<Value, String> {
    let parsed: JsonValue =
        serde_json::from_str(json).map_err(|error| format!("invalid Go FFI JSON: {}", error))?;
    decode_value(&parsed)
}

fn decode_value(value: &JsonValue) -> std::result::Result<Value, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "Go FFI value must be an object".to_string())?;
    let value_type = object
        .get("type")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| "Go FFI value is missing type".to_string())?;

    match value_type {
        "int" => Ok(Value::Int(
            object
                .get("value")
                .and_then(JsonValue::as_i64)
                .ok_or_else(|| "Go FFI int is invalid".to_string())?,
        )),
        "float" => Ok(Value::Float(
            object
                .get("value")
                .and_then(JsonValue::as_f64)
                .ok_or_else(|| "Go FFI float is invalid".to_string())?,
        )),
        "bool" => Ok(Value::Bool(
            object
                .get("value")
                .and_then(JsonValue::as_bool)
                .ok_or_else(|| "Go FFI bool is invalid".to_string())?,
        )),
        "string" => Ok(Value::new_string(
            object
                .get("value")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| "Go FFI string is invalid".to_string())?
                .to_string(),
        )),
        "unit" => Ok(Value::Unit),
        "null" => Ok(Value::Null),
        "list" => object
            .get("value")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| "Go FFI list is invalid".to_string())?
            .iter()
            .map(decode_value)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map(Value::new_list),
        "map" => {
            let values = object
                .get("value")
                .and_then(JsonValue::as_object)
                .ok_or_else(|| "Go FFI map is invalid".to_string())?;
            let mut map = HashMap::new();
            for (key, value) in values {
                map.insert(key.clone(), decode_value(value)?);
            }
            Ok(Value::new_map(map))
        }
        other => Err(format!("unsupported Go FFI value type '{}'", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn test_go_bridge_creation() {
        let bridge = GoBridge::new();
        assert!(bridge.libraries.lock().unwrap().is_empty());
    }

    #[test]
    fn test_value_conversion() {
        let value = Value::Float(42.0);
        let json = value_to_go_json(&value).unwrap();
        assert_eq!(json, r#"{"type":"float","value":42.0}"#);
        let back = go_json_to_value(&json).unwrap();
        assert_eq!(back, value);
    }

    #[test]
    fn calls_real_go_shared_library() {
        if Command::new("go").arg("version").output().is_err() {
            eprintln!("skipping Go native smoke: go is not available");
            return;
        }

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "matter_go_native_smoke_{}_{}",
            std::process::id(),
            unique
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let source = manifest_dir
            .parent()
            .and_then(std::path::Path::parent)
            .unwrap()
            .join("examples")
            .join("go_native_plugin")
            .join("plugin.go");
        assert!(
            source.exists(),
            "Go native example source missing at {}",
            source.display()
        );
        eprintln!("building Go native example: {}", source.display());

        let library = dir.join(if cfg!(target_os = "windows") {
            "matter_go_native_smoke.dll"
        } else if cfg!(target_os = "macos") {
            "libmatter_go_native_smoke.dylib"
        } else {
            "libmatter_go_native_smoke.so"
        });

        let status = Command::new("go")
            .current_dir(&dir)
            .args(["build", "-buildmode=c-shared", "-o"])
            .arg(&library)
            .arg(&source)
            .status()
            .unwrap();
        assert!(status.success(), "go build -buildmode=c-shared failed");

        let bridge = GoBridge::new();
        let package = library.display().to_string();
        bridge.load_package(&package).unwrap();

        let added = bridge
            .call_function(&package, "add", vec![Value::Int(40), Value::Int(2)])
            .unwrap();
        assert_eq!(added, Value::Int(42));

        let described = bridge.call_function(&package, "describe", vec![]).unwrap();
        assert_eq!(
            described,
            Value::new_string("hello from Go native".to_string())
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
