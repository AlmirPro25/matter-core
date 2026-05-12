// Matter Bridge: Java Native (JNI FFI)
// Direct FFI to Java libraries using JNI
// Performance: 100-1000x faster than subprocess

use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jdouble, jint, jlong, jstring};
use jni::{JNIEnv, JavaVM};
use matter_ast::Value;
use matter_error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn lock_unpoison<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Java bridge with direct FFI via JNI
pub struct JavaBridge {
    /// JVM instance
    jvm: Arc<Mutex<Option<JavaVM>>>,
    /// Loaded Java classes
    classes: Arc<Mutex<HashMap<String, String>>>,
}

impl JavaBridge {
    /// Create a new Java bridge
    pub fn new() -> Result<Self> {
        Ok(Self {
            jvm: Arc::new(Mutex::new(None)),
            classes: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Initialize JVM if not already initialized
    fn ensure_jvm(&self) -> Result<()> {
        let mut jvm_lock = lock_unpoison(&self.jvm);

        if jvm_lock.is_none() {
            // Build JVM with default options
            let jvm_args = jni::InitArgsBuilder::new()
                .version(jni::JNIVersion::V8)
                .option("-Xms64m")
                .option("-Xmx512m")
                .build()
                .map_err(|e| Error::Runtime(format!("Failed to build JVM args: {}", e)))?;

            let jvm = JavaVM::new(jvm_args)
                .map_err(|e| Error::Runtime(format!("Failed to create JVM: {}", e)))?;

            *jvm_lock = Some(jvm);
        }

        Ok(())
    }

    /// Get JNI environment
    fn get_env(&self) -> Result<JNIEnv> {
        self.ensure_jvm()?;

        let jvm_lock = lock_unpoison(&self.jvm);
        let jvm = jvm_lock
            .as_ref()
            .ok_or_else(|| Error::Runtime("JVM not initialized".to_string()))?;

        jvm.attach_current_thread()
            .map_err(|e| Error::Runtime(format!("Failed to attach to JVM: {}", e)))
    }

    /// Load a Java class
    pub fn load_class(&self, class_name: &str) -> Result<()> {
        let env = self.get_env()?;

        // Try to find the class
        let class = env.find_class(class_name).map_err(|e| {
            Error::Runtime(format!("Failed to load Java class {}: {}", class_name, e))
        })?;

        lock_unpoison(&self.classes).insert(class_name.to_string(), class_name.to_string());
        Ok(())
    }

    /// Call a static Java method
    pub fn call_static_method(
        &self,
        class_name: &str,
        method_name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        let env = self.get_env()?;

        // Load class if not already loaded
        if !lock_unpoison(&self.classes).contains_key(class_name) {
            self.load_class(class_name)?;
        }

        // Find the class
        let class = env
            .find_class(class_name)
            .map_err(|e| Error::Runtime(format!("Class not found: {}: {}", class_name, e)))?;

        // Convert Matter values to JValues
        let jvalues: Vec<JValue> = args
            .iter()
            .map(|v| self.value_to_jvalue(&env, v))
            .collect::<Result<Vec<_>>>()?;

        // Build method signature
        let signature = self.build_signature(&args);

        // Call the static method
        let result = env
            .call_static_method(class, method_name, &signature, &jvalues)
            .map_err(|e| Error::Runtime(format!("Failed to call method {}: {}", method_name, e)))?;

        // Convert result back to Matter value
        self.jvalue_to_value(&env, result)
    }

    /// Call an instance Java method
    pub fn call_method(
        &self,
        object: &JObject,
        method_name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        let env = self.get_env()?;

        // Convert Matter values to JValues
        let jvalues: Vec<JValue> = args
            .iter()
            .map(|v| self.value_to_jvalue(&env, v))
            .collect::<Result<Vec<_>>>()?;

        // Build method signature
        let signature = self.build_signature(&args);

        // Call the method
        let result = env
            .call_method(object, method_name, &signature, &jvalues)
            .map_err(|e| Error::Runtime(format!("Failed to call method {}: {}", method_name, e)))?;

        // Convert result back to Matter value
        self.jvalue_to_value(&env, result)
    }

    /// Convert Matter value to JValue
    fn value_to_jvalue(&self, env: &JNIEnv, value: &Value) -> Result<JValue> {
        match value {
            Value::Null => Ok(JValue::Object(JObject::null())),
            Value::Bool(b) => Ok(JValue::Bool(*b as jboolean)),
            Value::Number(n) => Ok(JValue::Double(*n as jdouble)),
            Value::String(s) => {
                let jstring = env
                    .new_string(s)
                    .map_err(|e| Error::Runtime(format!("Failed to create Java string: {}", e)))?;
                Ok(JValue::Object(jstring.into()))
            }
            Value::Array(arr) => {
                // Convert to JSON string for simplicity
                let json = serde_json::to_string(arr)
                    .map_err(|e| Error::Runtime(format!("Failed to serialize array: {}", e)))?;
                let jstring = env
                    .new_string(&json)
                    .map_err(|e| Error::Runtime(format!("Failed to create Java string: {}", e)))?;
                Ok(JValue::Object(jstring.into()))
            }
            Value::Object(_) => {
                // Convert to JSON string for simplicity
                let json = serde_json::to_string(value)
                    .map_err(|e| Error::Runtime(format!("Failed to serialize object: {}", e)))?;
                let jstring = env
                    .new_string(&json)
                    .map_err(|e| Error::Runtime(format!("Failed to create Java string: {}", e)))?;
                Ok(JValue::Object(jstring.into()))
            }
            _ => Err(Error::Runtime(format!(
                "Unsupported value type for Java: {:?}",
                value
            ))),
        }
    }

    /// Convert JValue to Matter value
    fn jvalue_to_value(&self, env: &JNIEnv, jvalue: JValue) -> Result<Value> {
        match jvalue {
            JValue::Bool(b) => Ok(Value::Bool(b != 0)),
            JValue::Byte(b) => Ok(Value::Number(b as f64)),
            JValue::Char(c) => Ok(Value::String(format!("{}", c as u8 as char))),
            JValue::Short(s) => Ok(Value::Number(s as f64)),
            JValue::Int(i) => Ok(Value::Number(i as f64)),
            JValue::Long(l) => Ok(Value::Number(l as f64)),
            JValue::Float(f) => Ok(Value::Number(f as f64)),
            JValue::Double(d) => Ok(Value::Number(d)),
            JValue::Object(obj) => {
                if obj.is_null() {
                    return Ok(Value::Null);
                }

                // Try to convert to string
                let jstring = JString::from(obj);
                let rust_string = env
                    .get_string(jstring)
                    .map_err(|e| Error::Runtime(format!("Failed to get Java string: {}", e)))?;

                Ok(Value::String(rust_string.into()))
            }
            JValue::Void => Ok(Value::Null),
        }
    }

    /// Build JNI method signature from arguments
    fn build_signature(&self, args: &[Value]) -> String {
        let mut sig = String::from("(");

        for arg in args {
            sig.push_str(match arg {
                Value::Bool(_) => "Z",
                Value::Number(_) => "D",
                Value::String(_) => "Ljava/lang/String;",
                _ => "Ljava/lang/String;", // Use string for complex types
            });
        }

        sig.push_str(")Ljava/lang/Object;");
        sig
    }

    /// Import a Java class
    pub fn import(&self, class_name: &str, alias: Option<&str>) -> Result<JavaClass> {
        self.load_class(class_name)?;

        Ok(JavaClass {
            bridge: self.clone(),
            class_name: class_name.to_string(),
            alias: alias.map(|s| s.to_string()),
        })
    }
}

impl Clone for JavaBridge {
    fn clone(&self) -> Self {
        Self {
            jvm: Arc::clone(&self.jvm),
            classes: Arc::clone(&self.classes),
        }
    }
}

impl Default for JavaBridge {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            jvm: Arc::new(Mutex::new(None)),
            classes: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

/// Represents an imported Java class
pub struct JavaClass {
    bridge: JavaBridge,
    class_name: String,
    alias: Option<String>,
}

impl JavaClass {
    /// Call a static method from this class
    pub fn call_static(&self, method: &str, args: Vec<Value>) -> Result<Value> {
        self.bridge
            .call_static_method(&self.class_name, method, args)
    }

    /// Get class name
    pub fn name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.class_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_bridge_creation() {
        let bridge = JavaBridge::new();
        assert!(bridge.is_ok());
    }

    #[test]
    fn test_signature_building() {
        let bridge = JavaBridge::new().unwrap();
        let args = vec![
            Value::Bool(true),
            Value::Number(42.0),
            Value::String("test".to_string()),
        ];
        let sig = bridge.build_signature(&args);
        assert_eq!(sig, "(ZDLjava/lang/String;)Ljava/lang/Object;");
    }
}
