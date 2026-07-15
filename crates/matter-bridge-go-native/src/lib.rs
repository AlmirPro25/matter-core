// Matter Bridge: Go Native (cgo FFI)
//
// The real cgo/libloading implementation is behind `cgo-native`. The default
// build keeps workspace validation independent from a local Go shared-library
// toolchain.

#![allow(clippy::result_large_err)]

use matter_backend::Value;
#[cfg(not(feature = "cgo-native"))]
use matter_error::{ErrorType, MatterError};
#[cfg(not(feature = "cgo-native"))]
use serde_json::Value as JsonValue;
#[cfg(not(feature = "cgo-native"))]
use std::sync::{Arc, Mutex};

#[cfg(not(feature = "cgo-native"))]
type Result<T> = std::result::Result<T, MatterError>;

#[cfg(not(feature = "cgo-native"))]
fn runtime_error(message: impl Into<String>) -> MatterError {
    MatterError::new(ErrorType::Runtime, message)
}

#[cfg(feature = "cgo-native")]
mod real;

#[cfg(feature = "cgo-native")]
pub use real::*;

#[cfg(not(feature = "cgo-native"))]
#[derive(Clone, Default)]
pub struct GoBridge {
    packages: Arc<Mutex<Vec<String>>>,
}

#[cfg(not(feature = "cgo-native"))]
impl GoBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_package(&self, package: &str) -> Result<()> {
        let mut packages = self
            .packages
            .lock()
            .map_err(|_| runtime_error("Go bridge package registry lock poisoned"))?;
        if !packages.iter().any(|name| name == package) {
            packages.push(package.to_string());
        }
        Ok(())
    }

    pub fn call_function(
        &self,
        _package: &str,
        _function: &str,
        _args: Vec<Value>,
    ) -> Result<Value> {
        Err(runtime_error(
            "Go native bridge unavailable: rebuild with feature `cgo-native` and Go shared libraries",
        ))
    }

    pub fn import(&self, package: &str, alias: Option<&str>) -> Result<GoPackage> {
        self.load_package(package)?;
        Ok(GoPackage {
            bridge: self.clone(),
            package: package.to_string(),
            alias: alias.map(|value| value.to_string()),
        })
    }
}

#[cfg(not(feature = "cgo-native"))]
pub struct GoPackage {
    bridge: GoBridge,
    package: String,
    alias: Option<String>,
}

#[cfg(not(feature = "cgo-native"))]
impl GoPackage {
    pub fn call(&self, function: &str, args: Vec<Value>) -> Result<Value> {
        self.bridge.call_function(&self.package, function, args)
    }

    pub fn name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.package)
    }
}

#[cfg(not(feature = "cgo-native"))]
pub fn value_to_go_json(value: &Value) -> Result<String> {
    Ok(value_to_json(value).to_string())
}

#[cfg(not(feature = "cgo-native"))]
pub fn go_json_to_value(json: &str) -> Result<Value> {
    let parsed: JsonValue = serde_json::from_str(json)
        .map_err(|e| runtime_error(format!("Failed to parse Go JSON: {}", e)))?;
    json_to_value(&parsed)
}

#[cfg(not(feature = "cgo-native"))]
fn value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Unit => JsonValue::Null,
        Value::Null => JsonValue::Null,
        Value::Bool(value) => JsonValue::Bool(*value),
        Value::Int(value) => JsonValue::Number((*value).into()),
        Value::Float(value) => serde_json::Number::from_f64(*value)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        Value::String(value) | Value::Function(value) => JsonValue::String(value.to_string()),
        Value::Closure(data) => JsonValue::String(data.func_name.clone()),
        Value::List(items) => JsonValue::Array(items.iter().map(value_to_json).collect()),
        Value::Map(entries) => JsonValue::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), value_to_json(value)))
                .collect(),
        ),
        Value::Struct { type_name, fields } => {
            let mut object = serde_json::Map::new();
            object.insert("type".to_string(), JsonValue::String(type_name.to_string()));
            object.insert(
                "fields".to_string(),
                JsonValue::Object(
                    fields
                        .iter()
                        .map(|(key, value)| (key.clone(), value_to_json(value)))
                        .collect(),
                ),
            );
            JsonValue::Object(object)
        }
    }
}

#[cfg(not(feature = "cgo-native"))]
fn json_to_value(value: &JsonValue) -> Result<Value> {
    match value {
        JsonValue::Null => Ok(Value::Unit),
        JsonValue::Bool(value) => Ok(Value::Bool(*value)),
        JsonValue::Number(value) => {
            if let Some(integer) = value.as_i64() {
                Ok(Value::Int(integer))
            } else if let Some(float) = value.as_f64() {
                Ok(Value::Float(float))
            } else {
                Err(runtime_error("Unsupported JSON number"))
            }
        }
        JsonValue::String(value) => Ok(Value::new_string(value.clone())),
        JsonValue::Array(values) => values
            .iter()
            .map(json_to_value)
            .collect::<Result<Vec<_>>>()
            .map(Value::new_list),
        JsonValue::Object(entries) => entries
            .iter()
            .map(|(key, value)| Ok((key.clone(), json_to_value(value)?)))
            .collect::<Result<std::collections::HashMap<_, _>>>()
            .map(Value::new_map),
    }
}

#[cfg(all(test, not(feature = "cgo-native")))]
mod tests {
    use super::*;

    #[test]
    fn test_go_bridge_creation() {
        let bridge = GoBridge::new();
        assert!(bridge.packages.lock().unwrap().is_empty());
    }

    #[test]
    fn test_value_conversion() {
        let value = Value::Float(42.0);
        let json = value_to_go_json(&value).unwrap();
        assert_eq!(json, "42.0");

        let back = go_json_to_value(&json).unwrap();
        assert_eq!(back, value);
    }
}
