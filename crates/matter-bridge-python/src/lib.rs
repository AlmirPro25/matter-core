//! Matter Python Bridge - FFI para Python via PyO3
//!
//! Permite importar e usar bibliotecas Python diretamente em Matter

use matter_backend::Value;
use matter_polyglot::{bridge::LanguageBridge, LanguageTarget};
use pyo3::prelude::*;
use pyo3::types::*;
use std::collections::HashMap;

pub mod converter;

use converter::PythonTypeConverter;

/// Bridge para Python usando PyO3
pub struct PythonBridge {
    modules: HashMap<String, Py<PyModule>>,
    converter: PythonTypeConverter,
    initialized: bool,
}

impl PythonBridge {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            converter: PythonTypeConverter,
            initialized: false,
        }
    }

    /// Converte argumentos Matter para Python
    fn convert_args(&self, py: Python, args: Vec<Value>) -> PyResult<Py<PyTuple>> {
        let py_args: Vec<PyObject> = args
            .into_iter()
            .map(|arg| self.converter.from_matter(py, &arg))
            .collect::<PyResult<_>>()?;

        Ok(PyTuple::new(py, py_args).into())
    }

    /// Converte resultado Python para Matter
    fn convert_result(&self, py: Python, result: &PyAny) -> Result<Value, String> {
        self.converter.to_matter(py, result)
    }
}

impl Default for PythonBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageBridge for PythonBridge {
    fn language(&self) -> LanguageTarget {
        LanguageTarget::Python
    }

    fn initialize(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        // PyO3 inicializa automaticamente com feature "auto-initialize"
        Python::with_gil(|py| {
            // Verifica se Python está funcionando
            py.run("import sys; print(f'Python {sys.version}')", None, None)
                .map_err(|e| format!("Python initialization failed: {}", e))?;

            self.initialized = true;
            Ok(())
        })
    }

    fn import_module(&mut self, module: &str) -> Result<(), String> {
        Python::with_gil(|py| {
            let py_module = PyModule::import(py, module)
                .map_err(|e| format!("Failed to import Python module '{}': {}", module, e))?;

            self.modules.insert(module.to_string(), py_module.into());
            Ok(())
        })
    }

    fn call_function(
        &mut self,
        module: &str,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        Python::with_gil(|py| {
            // Obtém o módulo
            let py_module = self
                .modules
                .get(module)
                .ok_or_else(|| format!("Module '{}' not imported", module))?
                .as_ref(py);

            // Obtém a função
            let py_func = py_module.getattr(function).map_err(|e| {
                format!(
                    "Function '{}' not found in module '{}': {}",
                    function, module, e
                )
            })?;

            // Converte argumentos
            let py_args = self
                .convert_args(py, args)
                .map_err(|e| format!("Failed to convert arguments: {}", e))?;

            // Chama a função
            let result = py_func
                .call1(py_args.as_ref(py))
                .map_err(|e| format!("Python function call failed: {}", e))?;

            // Converte resultado
            self.convert_result(py, result)
        })
    }

    fn get_attribute(&mut self, module: &str, attribute: &str) -> Result<Value, String> {
        Python::with_gil(|py| {
            // Obtém o módulo
            let py_module = self
                .modules
                .get(module)
                .ok_or_else(|| format!("Module '{}' not imported", module))?
                .as_ref(py);

            // Obtém o atributo
            let py_attr = py_module.getattr(attribute).map_err(|e| {
                format!(
                    "Attribute '{}' not found in module '{}': {}",
                    attribute, module, e
                )
            })?;

            // Converte para Matter
            self.convert_result(py, py_attr)
        })
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.modules.clear();
        self.initialized = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_bridge_initialization() {
        let mut bridge = PythonBridge::new();
        assert!(bridge.initialize().is_ok());
        assert!(bridge.initialized);
    }

    #[test]
    fn test_import_builtin_module() {
        let mut bridge = PythonBridge::new();
        bridge.initialize().unwrap();

        // Importa módulo built-in
        assert!(bridge.import_module("sys").is_ok());
        assert!(bridge.modules.contains_key("sys"));
    }

    #[test]
    fn test_call_python_function() {
        let mut bridge = PythonBridge::new();
        bridge.initialize().unwrap();
        bridge.import_module("math").unwrap();

        // Chama math.sqrt(16)
        let result = bridge
            .call_function("math", "sqrt", vec![Value::Int(16)])
            .unwrap();

        match result {
            Value::Float(f) => assert_eq!(f, 4.0),
            _ => panic!("Expected float result"),
        }
    }

    #[test]
    fn test_get_python_attribute() {
        let mut bridge = PythonBridge::new();
        bridge.initialize().unwrap();
        bridge.import_module("math").unwrap();

        // Obtém math.pi
        let result = bridge.get_attribute("math", "pi").unwrap();

        match result {
            Value::Float(f) => assert!((f - 3.14159).abs() < 0.001),
            _ => panic!("Expected float result"),
        }
    }
}
