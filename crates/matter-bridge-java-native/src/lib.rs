// Matter Bridge: Java Native (JNI FFI)
//
// The real JNI implementation is gated behind `jni-native` because it requires
// a local JDK/JVM setup. The default crate still exposes a stable API surface so
// the whole workspace can compile on machines without JAVA_HOME.

#![allow(clippy::result_large_err, dead_code)]

use matter_backend::Value;
#[cfg(not(feature = "jni-native"))]
use matter_error::{ErrorType, MatterError};
#[cfg(not(feature = "jni-native"))]
use std::sync::Arc;

#[cfg(not(feature = "jni-native"))]
type Result<T> = std::result::Result<T, MatterError>;

#[cfg(not(feature = "jni-native"))]
fn runtime_error(message: impl Into<String>) -> MatterError {
    MatterError::new(ErrorType::Runtime, message)
}

#[cfg(feature = "jni-native")]
mod real;

#[cfg(feature = "jni-native")]
pub use real::*;

#[cfg(not(feature = "jni-native"))]
#[derive(Clone, Default)]
pub struct JavaBridge {
    loaded_classes: Arc<std::sync::Mutex<Vec<String>>>,
}

#[cfg(not(feature = "jni-native"))]
impl JavaBridge {
    pub fn new() -> Result<Self> {
        Ok(Self::default())
    }

    pub fn load_class(&self, class_name: &str) -> Result<()> {
        let mut classes = self
            .loaded_classes
            .lock()
            .map_err(|_| runtime_error("Java bridge class registry lock poisoned"))?;
        if !classes.iter().any(|name| name == class_name) {
            classes.push(class_name.to_string());
        }
        Ok(())
    }

    pub fn call_static_method(
        &self,
        _class_name: &str,
        _method_name: &str,
        _args: Vec<Value>,
    ) -> Result<Value> {
        Err(runtime_error(
            "Java native bridge unavailable: rebuild with feature `jni-native` and a JDK installed",
        ))
    }

    pub fn import(&self, class_name: &str, alias: Option<&str>) -> Result<JavaClass> {
        self.load_class(class_name)?;
        Ok(JavaClass {
            bridge: self.clone(),
            class_name: class_name.to_string(),
            alias: alias.map(|value| value.to_string()),
        })
    }

    fn build_signature(&self, args: &[Value]) -> String {
        let mut sig = String::from("(");
        for arg in args {
            sig.push_str(match arg {
                Value::Bool(_) => "Z",
                Value::Int(_) | Value::Float(_) => "D",
                Value::String(_) => "Ljava/lang/String;",
                _ => "Ljava/lang/String;",
            });
        }
        sig.push_str(")Ljava/lang/Object;");
        sig
    }
}

#[cfg(not(feature = "jni-native"))]
pub struct JavaClass {
    bridge: JavaBridge,
    class_name: String,
    alias: Option<String>,
}

#[cfg(not(feature = "jni-native"))]
impl JavaClass {
    pub fn call_static(&self, method: &str, args: Vec<Value>) -> Result<Value> {
        self.bridge
            .call_static_method(&self.class_name, method, args)
    }

    pub fn name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.class_name)
    }
}

#[cfg(all(test, not(feature = "jni-native")))]
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
            Value::Float(42.0),
            Value::new_string("test".to_string()),
        ];
        let sig = bridge.build_signature(&args);
        assert_eq!(sig, "(ZDLjava/lang/String;)Ljava/lang/Object;");
    }
}
