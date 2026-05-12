// Matter Bridge: Java (via JNI)
// Permite importar e usar classes Java em Matter

use matter_backend::Value;
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug)]
pub enum BridgeError {
    RuntimeError(String),
    ConversionError(String),
}

pub type BridgeResult<T> = Result<T, BridgeError>;

pub trait Bridge {
    fn name(&self) -> &str;
    fn load_module(&mut self, module_path: &str) -> BridgeResult<()>;
    fn call(&self, module: &str, function: &str, args: Vec<Value>) -> BridgeResult<Value>;
    fn get_attribute(&self, module: &str, name: &str) -> BridgeResult<Value>;
}

pub struct JavaBridge {
    /// Classes Java carregadas
    classes: HashMap<String, JavaClass>,
    /// Classpath
    classpath: Vec<String>,
}

struct JavaClass {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    methods: HashMap<String, JavaMethod>,
}

struct JavaMethod {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    class: String,
}

impl JavaBridge {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            classpath: vec![".".to_string()],
        }
    }

    /// Adiciona ao classpath
    pub fn add_classpath(&mut self, path: String) {
        self.classpath.push(path);
    }

    /// Carrega uma classe Java
    pub fn load_class(&mut self, class_name: &str) -> BridgeResult<()> {
        // Verifica se Java está instalado
        let java_check = Command::new("java").arg("-version").output();

        if java_check.is_err() {
            return Err(BridgeError::RuntimeError(
                "Java not installed. Install JDK from https://adoptium.net/".to_string(),
            ));
        }

        // Carrega classe
        let class = JavaClass {
            name: class_name.to_string(),
            methods: HashMap::new(),
        };
        self.classes.insert(class_name.to_string(), class);
        Ok(())
    }

    /// Chama um método Java
    pub fn call_method(&self, class: &str, method: &str, args: Vec<Value>) -> BridgeResult<Value> {
        // Verifica se classe está carregada
        if !self.classes.contains_key(class) {
            return Err(BridgeError::RuntimeError(format!(
                "Class not loaded: {}",
                class
            )));
        }

        // Gera código Java temporário para chamar o método
        let java_code = self.generate_java_wrapper(class, method, &args)?;

        // Executa código Java
        self.execute_java_code(&java_code)
    }

    /// Gera código Java wrapper
    fn generate_java_wrapper(
        &self,
        class: &str,
        method: &str,
        args: &[Value],
    ) -> BridgeResult<String> {
        let mut code = String::new();

        // Imports
        code.push_str("import com.google.gson.Gson;\n");
        code.push_str(&format!("import {};\n\n", class));

        // Class declaration
        code.push_str("public class MatterBridge {\n");
        code.push_str("    public static void main(String[] args) {\n");

        // Create Gson instance
        code.push_str("        Gson gson = new Gson();\n");

        // Convert args
        for (i, arg) in args.iter().enumerate() {
            let java_value = self.value_to_java(arg)?;
            code.push_str(&format!("        Object arg{} = {};\n", i, java_value));
        }

        // Call method
        let class_simple = class.split('.').last().unwrap_or(class);
        code.push_str(&format!(
            "        {} obj = new {}();\n",
            class_simple, class_simple
        ));
        code.push_str(&format!("        Object result = obj.{}(", method));
        for i in 0..args.len() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&format!("arg{}", i));
        }
        code.push_str(");\n");

        // Print result as JSON
        code.push_str("        String jsonResult = gson.toJson(result);\n");
        code.push_str("        System.out.println(jsonResult);\n");
        code.push_str("    }\n");
        code.push_str("}\n");

        Ok(code)
    }

    /// Converte Value para código Java
    fn value_to_java(&self, value: &Value) -> BridgeResult<String> {
        match value {
            Value::Int(n) => Ok(n.to_string()),
            Value::Float(f) => Ok(format!("{}d", f)),
            Value::String(s) => Ok(format!("\"{}\"", s)),
            Value::Bool(b) => Ok(b.to_string()),
            Value::List(items) => {
                let java_items: Result<Vec<_>, _> =
                    items.iter().map(|v| self.value_to_java(v)).collect();
                Ok(format!("new Object[]{{{}}}", java_items?.join(", ")))
            }
            Value::Map(map) => {
                // Usa HashMap
                let mut pairs = Vec::new();
                for (k, v) in map.iter() {
                    let java_val = self.value_to_java(v)?;
                    pairs.push(format!("map.put(\"{}\", {})", k, java_val));
                }
                let mut code = "new HashMap<String, Object>() {{ ".to_string();
                code.push_str(&pairs.join("; "));
                code.push_str(" }}");
                Ok(code)
            }
            Value::Struct { fields, .. } => {
                let mut pairs = Vec::new();
                for (k, v) in fields.iter() {
                    let java_val = self.value_to_java(v)?;
                    pairs.push(format!("map.put(\"{}\", {})", k, java_val));
                }
                let mut code = "new HashMap<String, Object>() {{ ".to_string();
                code.push_str(&pairs.join("; "));
                code.push_str(" }}");
                Ok(code)
            }
            Value::Unit => Ok("null".to_string()),
            Value::Function(_) => Err(BridgeError::ConversionError(
                "Cannot convert Matter function to Java".to_string(),
            )),
        }
    }

    /// Executa código Java
    fn execute_java_code(&self, code: &str) -> BridgeResult<Value> {
        // Cria arquivo temporário
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("MatterBridge.java");

        std::fs::write(&temp_file, code)
            .map_err(|e| BridgeError::RuntimeError(format!("Failed to write temp file: {}", e)))?;

        // Compila
        let classpath = self.classpath.join(":");
        let compile = Command::new("javac")
            .args(&["-cp", &classpath, temp_file.to_str().unwrap()])
            .output()
            .map_err(|e| BridgeError::RuntimeError(format!("Failed to compile Java: {}", e)))?;

        if !compile.status.success() {
            let stderr = String::from_utf8_lossy(&compile.stderr);
            return Err(BridgeError::RuntimeError(format!(
                "Java compilation failed: {}",
                stderr
            )));
        }

        // Executa
        let output = Command::new("java")
            .args(&[
                "-cp",
                &format!("{}:{}", temp_dir.to_str().unwrap(), classpath),
                "MatterBridge",
            ])
            .output()
            .map_err(|e| BridgeError::RuntimeError(format!("Failed to execute Java: {}", e)))?;

        // Remove arquivos temporários
        let _ = std::fs::remove_file(&temp_file);
        let _ = std::fs::remove_file(temp_dir.join("MatterBridge.class"));

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BridgeError::RuntimeError(format!(
                "Java execution failed: {}",
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

impl Bridge for JavaBridge {
    fn name(&self) -> &str {
        "java"
    }

    fn load_module(&mut self, module_path: &str) -> BridgeResult<()> {
        self.load_class(module_path)
    }

    fn call(&self, module: &str, function: &str, args: Vec<Value>) -> BridgeResult<Value> {
        self.call_method(module, function, args)
    }

    fn get_attribute(&self, module: &str, name: &str) -> BridgeResult<Value> {
        // Java não tem atributos de classe como Python
        // Retorna método como callable
        Ok(Value::new_string(format!("{}.{}", module, name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_bridge_creation() {
        let bridge = JavaBridge::new();
        assert_eq!(bridge.name(), "java");
    }

    #[test]
    fn test_value_to_java() {
        let bridge = JavaBridge::new();

        assert_eq!(bridge.value_to_java(&Value::Int(42)).unwrap(), "42");
        assert_eq!(bridge.value_to_java(&Value::Float(3.14)).unwrap(), "3.14d");
        assert_eq!(
            bridge
                .value_to_java(&Value::new_string("hello".to_string()))
                .unwrap(),
            "\"hello\""
        );
        assert_eq!(bridge.value_to_java(&Value::Bool(true)).unwrap(), "true");
        assert_eq!(bridge.value_to_java(&Value::Unit).unwrap(), "null");
    }

    #[test]
    fn test_json_conversion() {
        let bridge = JavaBridge::new();

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

    #[test]
    fn test_classpath() {
        let mut bridge = JavaBridge::new();
        bridge.add_classpath("/path/to/lib.jar".to_string());
        assert_eq!(bridge.classpath.len(), 2);
    }
}
