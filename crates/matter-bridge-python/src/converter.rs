//! Conversão de tipos entre Matter e Python

use matter_backend::Value;
use pyo3::prelude::*;
use pyo3::types::*;
use std::collections::HashMap;

pub struct PythonTypeConverter;

impl PythonTypeConverter {
    /// Converte valor Python para Matter
    pub fn to_matter(&self, py: Python, py_obj: &PyAny) -> Result<Value, String> {
        // None
        if py_obj.is_none() {
            return Ok(Value::Unit);
        }

        // Bool (deve vir antes de Int, pois bool é subclasse de int em Python)
        if let Ok(b) = py_obj.extract::<bool>() {
            return Ok(Value::Bool(b));
        }

        // Int
        if let Ok(i) = py_obj.extract::<i64>() {
            return Ok(Value::Int(i));
        }

        // Float
        if let Ok(f) = py_obj.extract::<f64>() {
            return Ok(Value::Float(f));
        }

        // String
        if let Ok(s) = py_obj.extract::<String>() {
            return Ok(Value::new_string(s));
        }

        // List
        if let Ok(list) = py_obj.downcast::<PyList>() {
            let items: Result<Vec<Value>, String> =
                list.iter().map(|item| self.to_matter(py, item)).collect();
            return Ok(Value::new_list(items?));
        }

        // Tuple (converte para List)
        if let Ok(tuple) = py_obj.downcast::<PyTuple>() {
            let items: Result<Vec<Value>, String> =
                tuple.iter().map(|item| self.to_matter(py, item)).collect();
            return Ok(Value::new_list(items?));
        }

        // Dict
        if let Ok(dict) = py_obj.downcast::<PyDict>() {
            let mut map = HashMap::new();
            for (key, value) in dict.iter() {
                let key_str = key
                    .extract::<String>()
                    .map_err(|_| "Dict keys must be strings".to_string())?;
                let value_matter = self.to_matter(py, value)?;
                map.insert(key_str, value_matter);
            }
            return Ok(Value::new_map(map));
        }

        // NumPy array (se disponível)
        if py_obj.hasattr("__array__").unwrap_or(false) {
            // Converte para lista Python primeiro
            let to_list = py_obj
                .call_method0("tolist")
                .map_err(|e| format!("Failed to convert numpy array: {}", e))?;
            return self.to_matter(py, to_list);
        }

        Err(format!(
            "Unsupported Python type: {}",
            py_obj.get_type().name().unwrap_or("unknown")
        ))
    }

    /// Converte valor Matter para Python
    pub fn from_matter(&self, py: Python, value: &Value) -> PyResult<PyObject> {
        match value {
            Value::Unit => Ok(py.None()),
            Value::Bool(b) => Ok(b.to_object(py)),
            Value::Int(i) => Ok(i.to_object(py)),
            Value::Float(f) => Ok(f.to_object(py)),
            Value::String(s) => Ok(s.to_object(py)),
            Value::List(items) => {
                let py_items: PyResult<Vec<PyObject>> = items
                    .iter()
                    .map(|item| self.from_matter(py, item))
                    .collect();
                Ok(PyList::new(py, py_items?).to_object(py))
            }
            Value::Map(map) => {
                let py_dict = PyDict::new(py);
                for (key, value) in map.iter() {
                    let py_value = self.from_matter(py, value)?;
                    py_dict.set_item(key, py_value)?;
                }
                Ok(py_dict.to_object(py))
            }
            Value::Struct { fields, .. } => {
                // Converte struct para dict
                let py_dict = PyDict::new(py);
                for (key, value) in fields.iter() {
                    let py_value = self.from_matter(py, value)?;
                    py_dict.set_item(key, py_value)?;
                }
                Ok(py_dict.to_object(py))
            }
            Value::Function(_) => Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Cannot convert Matter function to Python",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_int() {
        Python::with_gil(|py| {
            let converter = PythonTypeConverter;

            // Matter → Python
            let matter_val = Value::Int(42);
            let py_obj = converter.from_matter(py, &matter_val).unwrap();
            assert_eq!(py_obj.extract::<i64>(py).unwrap(), 42);

            // Python → Matter
            let py_int = 42.to_object(py);
            let matter_result = converter.to_matter(py, py_int.as_ref(py)).unwrap();
            assert_eq!(matter_result, Value::Int(42));
        });
    }

    #[test]
    fn test_convert_string() {
        Python::with_gil(|py| {
            let converter = PythonTypeConverter;

            let matter_val = Value::new_string("hello".to_string());
            let py_obj = converter.from_matter(py, &matter_val).unwrap();
            assert_eq!(py_obj.extract::<String>(py).unwrap(), "hello");
        });
    }

    #[test]
    fn test_convert_list() {
        Python::with_gil(|py| {
            let converter = PythonTypeConverter;

            let matter_val = Value::new_list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);

            let py_obj = converter.from_matter(py, &matter_val).unwrap();
            let py_list = py_obj.downcast::<PyList>(py).unwrap();
            assert_eq!(py_list.len(), 3);
        });
    }

    #[test]
    fn test_convert_dict() {
        Python::with_gil(|py| {
            let converter = PythonTypeConverter;

            let mut map = HashMap::new();
            map.insert("name".to_string(), Value::new_string("Matter".to_string()));
            map.insert("version".to_string(), Value::Int(1));

            let matter_val = Value::new_map(map);
            let py_obj = converter.from_matter(py, &matter_val).unwrap();
            let py_dict = py_obj.downcast::<PyDict>(py).unwrap();

            assert_eq!(py_dict.len(), 2);
            assert_eq!(
                py_dict
                    .get_item("name")
                    .unwrap()
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "Matter"
            );
        });
    }
}
