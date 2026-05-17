use jni::objects::{JObject, JString, JValueOwned};
use jni::{JNIEnv, JavaVM};
use matter_backend::Value;
use matter_error::{ErrorType, MatterError};
use std::collections::HashMap;
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

#[derive(Clone)]
pub struct JavaBridge {
    jvm: Arc<Mutex<Option<&'static JavaVM>>>,
    classes: Arc<Mutex<HashMap<String, String>>>,
}

impl JavaBridge {
    pub fn new() -> Result<Self> {
        Ok(Self {
            jvm: Arc::new(Mutex::new(None)),
            classes: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn ensure_jvm(&self) -> Result<&'static JavaVM> {
        let mut jvm_lock = lock_unpoison(&self.jvm);
        if let Some(jvm) = jvm_lock.as_ref() {
            return Ok(*jvm);
        }

        let jvm_args = jni::InitArgsBuilder::new()
            .version(jni::JNIVersion::V8)
            .option("-Xms64m")
            .option("-Xmx512m")
            .build()
            .map_err(|error| runtime_error(format!("Failed to build JVM args: {}", error)))?;

        let jvm = Box::leak(Box::new(JavaVM::new(jvm_args).map_err(|error| {
            runtime_error(format!("Failed to create JVM: {}", error))
        })?));
        *jvm_lock = Some(jvm);
        Ok(jvm)
    }

    fn get_env(&self) -> Result<JNIEnv<'static>> {
        let jvm = self.ensure_jvm()?;
        jvm.attach_current_thread_permanently()
            .map_err(|error| runtime_error(format!("Failed to attach to JVM: {}", error)))
    }

    pub fn load_class(&self, class_name: &str) -> Result<()> {
        let mut env = self.get_env()?;
        env.find_class(class_name).map_err(|error| {
            runtime_error(format!(
                "Failed to load Java class {}: {}",
                class_name, error
            ))
        })?;
        lock_unpoison(&self.classes).insert(class_name.to_string(), class_name.to_string());
        Ok(())
    }

    pub fn call_static_method(
        &self,
        class_name: &str,
        method_name: &str,
        args: Vec<Value>,
    ) -> Result<Value> {
        if !args.is_empty() {
            return Err(runtime_error(
                "Java native bridge currently validates JVM class loading and no-arg static calls only",
            ));
        }

        let mut env = self.get_env()?;
        let class = env
            .find_class(class_name)
            .map_err(|error| runtime_error(format!("Class not found {}: {}", class_name, error)))?;

        let result = env
            .call_static_method(class, method_name, "()Ljava/lang/String;", &[])
            .map_err(|error| {
                runtime_error(format!(
                    "Failed to call Java static method {}.{}: {}",
                    class_name, method_name, error
                ))
            })?;

        self.jvalue_to_value(&mut env, result)
    }

    fn jvalue_to_value(&self, env: &mut JNIEnv<'_>, value: JValueOwned<'_>) -> Result<Value> {
        match value {
            JValueOwned::Bool(value) => Ok(Value::Bool(value != 0)),
            JValueOwned::Byte(value) => Ok(Value::Int(value as i64)),
            JValueOwned::Char(value) => Ok(Value::Int(value as i64)),
            JValueOwned::Short(value) => Ok(Value::Int(value as i64)),
            JValueOwned::Int(value) => Ok(Value::Int(value as i64)),
            JValueOwned::Long(value) => Ok(Value::Int(value)),
            JValueOwned::Float(value) => Ok(Value::Float(value as f64)),
            JValueOwned::Double(value) => Ok(Value::Float(value)),
            JValueOwned::Object(object) => self.object_to_value(env, object),
            JValueOwned::Void => Ok(Value::Unit),
        }
    }

    fn object_to_value(&self, env: &mut JNIEnv<'_>, object: JObject<'_>) -> Result<Value> {
        if object.is_null() {
            return Ok(Value::Unit);
        }
        let string = JString::from(object);
        let rust_string = env
            .get_string(&string)
            .map_err(|error| runtime_error(format!("Failed to read Java string: {}", error)))?;
        Ok(Value::new_string(rust_string.into()))
    }

    #[cfg(test)]
    fn build_signature(&self, args: &[Value]) -> String {
        let mut sig = String::from("(");
        for arg in args {
            sig.push_str(match arg {
                Value::Bool(_) => "Z",
                Value::Int(_) => "J",
                Value::Float(_) => "D",
                Value::String(_) => "Ljava/lang/String;",
                _ => "Ljava/lang/String;",
            });
        }
        sig.push_str(")Ljava/lang/Object;");
        sig
    }

    pub fn import(&self, class_name: &str, alias: Option<&str>) -> Result<JavaClass> {
        self.load_class(class_name)?;
        Ok(JavaClass {
            bridge: self.clone(),
            class_name: class_name.to_string(),
            alias: alias.map(str::to_string),
        })
    }
}

impl Default for JavaBridge {
    fn default() -> Self {
        Self::new().expect("JavaBridge::new should not initialize JVM")
    }
}

pub struct JavaClass {
    bridge: JavaBridge,
    class_name: String,
    alias: Option<String>,
}

impl JavaClass {
    pub fn call_static(&self, method: &str, args: Vec<Value>) -> Result<Value> {
        self.bridge
            .call_static_method(&self.class_name, method, args)
    }

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
            Value::Float(42.0),
            Value::new_string("test".to_string()),
        ];
        let sig = bridge.build_signature(&args);
        assert_eq!(sig, "(ZDLjava/lang/String;)Ljava/lang/Object;");
    }

    #[test]
    #[ignore = "requires java/javac/JNI runtime on PATH"]
    fn calls_real_jvm_static_string() {
        let bridge = JavaBridge::new().unwrap();
        let value = bridge
            .call_static_method("java/lang/System", "lineSeparator", vec![])
            .unwrap();

        let Value::String(line_separator) = value else {
            panic!("expected Java String result");
        };
        assert!(!line_separator.is_empty());
    }
}
