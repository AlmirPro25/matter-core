//! Conversão de tipos entre Matter e JavaScript/Node.js

use matter_backend::Value;
use serde_json;
use std::collections::HashMap;

pub struct JavaScriptTypeConverter;

impl JavaScriptTypeConverter {
    /// Converte argumentos Matter para JSON (JavaScript)
    pub fn args_to_json(&self, args: &[Value]) -> Result<String, String> {
        let json_args: Vec<serde_json::Value> = args
            .iter()
            .map(|arg| self.matter_to_json(arg))
            .collect::<Result<_, _>>()?;

        serde_json::to_string(&json_args)
            .map_err(|e| format!("Failed to serialize arguments: {}", e))
    }

    /// Converte valor Matter para JSON
    pub fn matter_to_json(&self, value: &Value) -> Result<serde_json::Value, String> {
        match value {
            Value::Unit => Ok(serde_json::Value::Null),
            Value::Null => Ok(serde_json::Value::Null),
            Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
            Value::Int(i) => Ok(serde_json::Value::Number((*i).into())),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .ok_or_else(|| "Invalid float value".to_string()),
            Value::String(s) => Ok(serde_json::Value::String(s.to_string())),
            Value::List(items) => {
                let json_items: Vec<serde_json::Value> = items
                    .iter()
                    .map(|item| self.matter_to_json(item))
                    .collect::<Result<_, _>>()?;
                Ok(serde_json::Value::Array(json_items))
            }
            Value::Map(map) => {
                let mut json_map = serde_json::Map::new();
                for (key, value) in map.iter() {
                    json_map.insert(key.clone(), self.matter_to_json(value)?);
                }
                Ok(serde_json::Value::Object(json_map))
            }
            Value::Struct { fields, .. } => {
                let mut json_map = serde_json::Map::new();
                for (key, value) in fields.iter() {
                    json_map.insert(key.clone(), self.matter_to_json(value)?);
                }
                Ok(serde_json::Value::Object(json_map))
            }
            Value::Function(_) => Err("Cannot convert Matter function to JavaScript".to_string()),
        }
    }

    /// Converte JSON para valor Matter
    pub fn json_to_matter(&self, json_str: &str) -> Result<Value, String> {
        let json_value: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        self.json_value_to_matter(&json_value)
    }

    /// Converte serde_json::Value para Matter Value
    fn json_value_to_matter(&self, json: &serde_json::Value) -> Result<Value, String> {
        match json {
            serde_json::Value::Null => Ok(Value::Unit),
            serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::Int(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Value::Float(f))
                } else {
                    Err("Invalid number".to_string())
                }
            }
            serde_json::Value::String(s) => Ok(Value::new_string(s.clone())),
            serde_json::Value::Array(arr) => {
                let items: Vec<Value> = arr
                    .iter()
                    .map(|item| self.json_value_to_matter(item))
                    .collect::<Result<_, _>>()?;
                Ok(Value::new_list(items))
            }
            serde_json::Value::Object(obj) => {
                let mut map = HashMap::new();
                for (key, value) in obj {
                    map.insert(key.clone(), self.json_value_to_matter(value)?);
                }
                Ok(Value::new_map(map))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matter_to_json() {
        let converter = JavaScriptTypeConverter;

        // Int
        let json = converter.matter_to_json(&Value::Int(42)).unwrap();
        assert_eq!(json, serde_json::json!(42));

        // String
        let json = converter
            .matter_to_json(&Value::new_string("hello".to_string()))
            .unwrap();
        assert_eq!(json, serde_json::json!("hello"));

        // List
        let json = converter
            .matter_to_json(&Value::new_list(vec![Value::Int(1), Value::Int(2)]))
            .unwrap();
        assert_eq!(json, serde_json::json!([1, 2]));

        // Null
        let json = converter.matter_to_json(&Value::Null).unwrap();
        assert_eq!(json, serde_json::Value::Null);
    }

    #[test]
    fn test_json_to_matter() {
        let converter = JavaScriptTypeConverter;

        // Number
        let matter = converter.json_to_matter("42").unwrap();
        assert_eq!(matter, Value::Int(42));

        // String
        let matter = converter.json_to_matter(r#""hello""#).unwrap();
        assert_eq!(matter, Value::new_string("hello".to_string()));

        // Array
        let matter = converter.json_to_matter("[1, 2, 3]").unwrap();
        assert_eq!(
            matter,
            Value::new_list(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }
}
