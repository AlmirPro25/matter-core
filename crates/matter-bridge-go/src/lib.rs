// Matter Bridge: Go (via cgo)
// Permite importar e usar packages Go em Matter

use matter_backend::Value;
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug)]
pub enum BridgeError {
    RuntimeError(String),
    ImportError(String),
    ConversionError(String),
}

pub type BridgeResult<T> = Result<T, BridgeError>;

pub trait Bridge {
    fn name(&self) -> &str;
    fn load_module(&mut self, module_path: &str) -> BridgeResult<()>;
    fn call(&self, module: &str, function: &str, args: Vec<Value>) -> BridgeResult<Value>;
    fn get_attribute(&self, module: &str, name: &str) -> BridgeResult<Value>;
}

pub struct GoBridge {
    /// Packages Go carregados
    packages: HashMap<String, GoPackage>,
}

struct GoPackage {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    functions: HashMap<String, GoFunction>,
}

struct GoFunction {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    package: String,
}

impl GoBridge {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    /// Carrega um package Go
    pub fn load_package(&mut self, package_name: &str) -> BridgeResult<()> {
        // Verifica se Go está instalado
        let go_check = Command::new("go").arg("version").output();

        if go_check.is_err() {
            return Err(BridgeError::RuntimeError(
                "Go not installed. Install from https://go.dev/dl/".to_string(),
            ));
        }

        // Verifica se package existe
        let check = Command::new("go").args(["list", package_name]).output();

        match check {
            Ok(output) if output.status.success() => {
                // Package existe, carrega
                let pkg = GoPackage {
                    name: package_name.to_string(),
                    functions: HashMap::new(),
                };
                self.packages.insert(package_name.to_string(), pkg);
                Ok(())
            }
            _ => {
                // Tenta instalar package
                println!("Installing Go package: {}", package_name);
                let install = Command::new("go").args(["get", package_name]).output();

                match install {
                    Ok(output) if output.status.success() => {
                        let pkg = GoPackage {
                            name: package_name.to_string(),
                            functions: HashMap::new(),
                        };
                        self.packages.insert(package_name.to_string(), pkg);
                        Ok(())
                    }
                    _ => Err(BridgeError::ImportError(format!(
                        "Failed to install Go package: {}",
                        package_name
                    ))),
                }
            }
        }
    }

    /// Chama uma função Go
    pub fn call_function(
        &self,
        package: &str,
        function: &str,
        args: Vec<Value>,
    ) -> BridgeResult<Value> {
        // Verifica se package está carregado
        if !self.packages.contains_key(package) {
            return Err(BridgeError::RuntimeError(format!(
                "Package not loaded: {}",
                package
            )));
        }

        // Gera código Go temporário para chamar a função
        let go_code = self.generate_go_wrapper(package, function, &args)?;

        // Executa código Go
        self.execute_go_code(&go_code)
    }

    /// Gera código Go wrapper
    fn generate_go_wrapper(
        &self,
        package: &str,
        function: &str,
        args: &[Value],
    ) -> BridgeResult<String> {
        let mut code = String::new();

        // Package declaration
        code.push_str("package main\n\n");

        // Imports
        code.push_str("import (\n");
        code.push_str("    \"fmt\"\n");
        code.push_str("    \"encoding/json\"\n");
        code.push_str(&format!("    pkg \"{}\"\n", package));
        code.push_str(")\n\n");

        // Main function
        code.push_str("func main() {\n");

        // Convert args
        for (i, arg) in args.iter().enumerate() {
            let go_value = self.value_to_go(arg)?;
            code.push_str(&format!("    arg{} := {}\n", i, go_value));
        }

        // Call function
        code.push_str(&format!("    result := pkg.{}(", function));
        for i in 0..args.len() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&format!("arg{}", i));
        }
        code.push_str(")\n");

        // Print result as JSON
        code.push_str("    jsonResult, _ := json.Marshal(result)\n");
        code.push_str("    fmt.Println(string(jsonResult))\n");
        code.push_str("}\n");

        Ok(code)
    }

    /// Converte Value para código Go
    fn value_to_go(&self, value: &Value) -> BridgeResult<String> {
        match value {
            Value::Int(n) => Ok(n.to_string()),
            Value::Float(f) => Ok(format!("{:?}", f)),
            Value::String(s) => serde_json::to_string(s.as_str()).map_err(|e| {
                BridgeError::ConversionError(format!("Failed to encode Go string literal: {}", e))
            }),
            Value::Bool(b) => Ok(b.to_string()),
            Value::List(items) => {
                let go_items: Result<Vec<_>, _> =
                    items.iter().map(|v| self.value_to_go(v)).collect();
                Ok(format!("[]interface{{}}{{{}}}", go_items?.join(", ")))
            }
            Value::Map(map) => {
                let mut pairs = Vec::new();
                for (k, v) in map.iter() {
                    let go_key = serde_json::to_string(k).map_err(|e| {
                        BridgeError::ConversionError(format!("Failed to encode Go map key: {}", e))
                    })?;
                    let go_val = self.value_to_go(v)?;
                    pairs.push(format!("{}: {}", go_key, go_val));
                }
                Ok(format!("map[string]interface{{}}{{{}}}", pairs.join(", ")))
            }
            Value::Struct { fields, .. } => {
                let mut pairs = Vec::new();
                for (k, v) in fields.iter() {
                    let go_key = serde_json::to_string(k).map_err(|e| {
                        BridgeError::ConversionError(format!(
                            "Failed to encode Go struct key: {}",
                            e
                        ))
                    })?;
                    let go_val = self.value_to_go(v)?;
                    pairs.push(format!("{}: {}", go_key, go_val));
                }
                Ok(format!("map[string]interface{{}}{{{}}}", pairs.join(", ")))
            }
            Value::Unit => Ok("nil".to_string()),
            Value::Null => Ok("nil".to_string()),
            Value::Function(_) => Err(BridgeError::ConversionError(
                "Cannot convert Matter function to Go".to_string(),
            )),
            Value::Closure(_) => Err(BridgeError::ConversionError(
                "Cannot convert Matter closure to Go".to_string(),
            )),
        }
    }

    /// Executa código Go
    fn execute_go_code(&self, code: &str) -> BridgeResult<Value> {
        // Cria arquivo temporário
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("matter_go_bridge.go");

        std::fs::write(&temp_file, code)
            .map_err(|e| BridgeError::RuntimeError(format!("Failed to write temp file: {}", e)))?;

        // Executa com go run
        let output = Command::new("go")
            .args(["run", temp_file.to_str().unwrap()])
            .output()
            .map_err(|e| BridgeError::RuntimeError(format!("Failed to execute Go: {}", e)))?;

        // Remove arquivo temporário
        let _ = std::fs::remove_file(&temp_file);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BridgeError::RuntimeError(format!(
                "Go execution failed: {}",
                stderr
            )));
        }

        // Parse resultado JSON
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json_value: serde_json::Value = serde_json::from_str(stdout.trim())
            .map_err(|e| BridgeError::ConversionError(format!("Failed to parse JSON: {}", e)))?;

        // Converte para Value
        self.json_to_value(&json_value)
    }

    /// Converte JSON para Value
    fn json_to_value(&self, json: &serde_json::Value) -> BridgeResult<Value> {
        match json {
            serde_json::Value::Null => Ok(Value::Unit),
            serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::Int(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Value::Float(f))
                } else {
                    Err(BridgeError::ConversionError("Invalid number".to_string()))
                }
            }
            serde_json::Value::String(s) => Ok(Value::new_string(s.clone())),
            serde_json::Value::Array(arr) => {
                let values: Result<Vec<_>, _> = arr.iter().map(|v| self.json_to_value(v)).collect();
                Ok(Value::new_list(values?))
            }
            serde_json::Value::Object(obj) => {
                let mut map = HashMap::new();
                for (k, v) in obj {
                    map.insert(k.clone(), self.json_to_value(v)?);
                }
                Ok(Value::new_map(map))
            }
        }
    }
}

impl Default for GoBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Bridge for GoBridge {
    fn name(&self) -> &str {
        "go"
    }

    fn load_module(&mut self, module_path: &str) -> BridgeResult<()> {
        self.load_package(module_path)
    }

    fn call(&self, module: &str, function: &str, args: Vec<Value>) -> BridgeResult<Value> {
        self.call_function(module, function, args)
    }

    fn get_attribute(&self, module: &str, name: &str) -> BridgeResult<Value> {
        // Go não tem atributos de módulo como Python
        // Retorna função como callable
        Ok(Value::new_string(format!("{}.{}", module, name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_bridge_creation() {
        let bridge = GoBridge::new();
        assert_eq!(bridge.name(), "go");
    }

    #[test]
    fn test_value_to_go() {
        let bridge = GoBridge::new();

        assert_eq!(bridge.value_to_go(&Value::Int(42)).unwrap(), "42");
        assert_eq!(bridge.value_to_go(&Value::Float(2.5)).unwrap(), "2.5");
        assert_eq!(
            bridge
                .value_to_go(&Value::new_string("hello".to_string()))
                .unwrap(),
            "\"hello\""
        );
        assert_eq!(bridge.value_to_go(&Value::Bool(true)).unwrap(), "true");
        assert_eq!(bridge.value_to_go(&Value::Unit).unwrap(), "nil");
        assert_eq!(bridge.value_to_go(&Value::Null).unwrap(), "nil");
    }

    #[test]
    fn test_value_to_go_escapes_strings_and_composites() {
        let bridge = GoBridge::new();

        assert_eq!(
            bridge
                .value_to_go(&Value::new_string("hello \"go\"".to_string()))
                .unwrap(),
            r#""hello \"go\"""#
        );

        assert_eq!(
            bridge
                .value_to_go(&Value::new_list(vec![
                    Value::Int(1),
                    Value::new_string("two".to_string())
                ]))
                .unwrap(),
            r#"[]interface{}{1, "two"}"#
        );

        let mut map = HashMap::new();
        map.insert("name".to_string(), Value::new_string("test".to_string()));
        map.insert("value".to_string(), Value::Int(42));
        let code = bridge.value_to_go(&Value::new_map(map)).unwrap();
        assert!(code.starts_with("map[string]interface{}{"));
        assert!(code.contains(r#""name": "test""#));
        assert!(code.contains(r#""value": 42"#));
    }

    #[test]
    fn test_load_and_call_standard_library_function() {
        let mut bridge = GoBridge::new();
        if bridge.load_package("math").is_err() {
            return;
        }

        let result = bridge
            .call_function("math", "Sqrt", vec![Value::Float(2.25)])
            .unwrap();

        assert_eq!(result, Value::Float(1.5));
    }

    #[test]
    fn test_json_conversion() {
        let bridge = GoBridge::new();

        let json = serde_json::json!({
            "name": "test",
            "value": 42,
            "active": true
        });

        let value = bridge.json_to_value(&json).unwrap();

        if let Value::Map(map) = value {
            assert!(map.contains_key("name"));
            assert!(map.contains_key("value"));
            assert!(map.contains_key("active"));
        } else {
            panic!("Expected Map");
        }
    }
}
