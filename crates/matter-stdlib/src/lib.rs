//! Matter Standard Library
//! Biblioteca padrão com módulos math, string, list, time, random, json, etc

use matter_backend::{Backend, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Math backend - operações matemáticas
pub struct MathBackend;

impl MathBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MathBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for MathBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "abs" => {
                if args.len() != 1 {
                    return Err(format!("math.abs expects 1 argument, got {}", args.len()));
                }
                let n = args[0].as_int().map_err(|e| format!("math.abs: {}", e))?;
                Ok(Value::Int(n.abs()))
            }

            "min" => {
                if args.len() != 2 {
                    return Err(format!("math.min expects 2 arguments, got {}", args.len()));
                }
                let a = args[0].as_int().map_err(|e| format!("math.min: {}", e))?;
                let b = args[1].as_int().map_err(|e| format!("math.min: {}", e))?;
                Ok(Value::Int(a.min(b)))
            }

            "max" => {
                if args.len() != 2 {
                    return Err(format!("math.max expects 2 arguments, got {}", args.len()));
                }
                let a = args[0].as_int().map_err(|e| format!("math.max: {}", e))?;
                let b = args[1].as_int().map_err(|e| format!("math.max: {}", e))?;
                Ok(Value::Int(a.max(b)))
            }

            "pow" => {
                if args.len() != 2 {
                    return Err(format!("math.pow expects 2 arguments, got {}", args.len()));
                }
                let base = args[0].as_int().map_err(|e| format!("math.pow: {}", e))?;
                let exp = args[1].as_int().map_err(|e| format!("math.pow: {}", e))?;
                if exp < 0 {
                    return Err(
                        "math.pow: negative exponent not supported for integers".to_string()
                    );
                }
                Ok(Value::Int(base.pow(exp as u32)))
            }

            "sqrt" => {
                if args.len() != 1 {
                    return Err(format!("math.sqrt expects 1 argument, got {}", args.len()));
                }
                let n = args[0].as_int().map_err(|e| format!("math.sqrt: {}", e))?;
                if n < 0 {
                    return Err("math.sqrt: negative number not supported".to_string());
                }
                let result = (n as f64).sqrt() as i64;
                Ok(Value::Int(result))
            }

            "mod" => {
                if args.len() != 2 {
                    return Err(format!("math.mod expects 2 arguments, got {}", args.len()));
                }
                let a = args[0].as_int().map_err(|e| format!("math.mod: {}", e))?;
                let b = args[1].as_int().map_err(|e| format!("math.mod: {}", e))?;
                if b == 0 {
                    return Err("math.mod: division by zero".to_string());
                }
                Ok(Value::Int(a % b))
            }

            "clamp" => {
                if args.len() != 3 {
                    return Err(format!(
                        "math.clamp expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                let value = args[0].as_int().map_err(|e| format!("math.clamp: {}", e))?;
                let min = args[1].as_int().map_err(|e| format!("math.clamp: {}", e))?;
                let max = args[2].as_int().map_err(|e| format!("math.clamp: {}", e))?;
                Ok(Value::Int(value.clamp(min, max)))
            }

            _ => Err(format!("Unknown math method: {}", method)),
        }
    }
}

/// String backend - operações com strings
pub struct StringBackend;

impl StringBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StringBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for StringBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "len" => {
                if args.len() != 1 {
                    return Err(format!("string.len expects 1 argument, got {}", args.len()));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.len: {}", e))?;
                Ok(Value::Int(s.len() as i64))
            }

            "upper" => {
                if args.len() != 1 {
                    return Err(format!(
                        "string.upper expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.upper: {}", e))?;
                Ok(Value::new_string(s.to_uppercase()))
            }

            "lower" => {
                if args.len() != 1 {
                    return Err(format!(
                        "string.lower expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.lower: {}", e))?;
                Ok(Value::new_string(s.to_lowercase()))
            }

            "trim" => {
                if args.len() != 1 {
                    return Err(format!(
                        "string.trim expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.trim: {}", e))?;
                Ok(Value::new_string(s.trim().to_string()))
            }

            "split" => {
                if args.len() != 2 {
                    return Err(format!(
                        "string.split expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.split: {}", e))?;
                let sep = args[1]
                    .as_string()
                    .map_err(|e| format!("string.split: {}", e))?;

                let parts: Vec<Value> = s
                    .split(&sep)
                    .map(|part| Value::new_string(part.to_string()))
                    .collect();

                Ok(Value::new_list(parts))
            }

            "join" => {
                if args.len() != 2 {
                    return Err(format!(
                        "string.join expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let sep = args[0]
                    .as_string()
                    .map_err(|e| format!("string.join: {}", e))?;

                if let Value::List(items) = &args[1] {
                    let strings: Result<Vec<String>, String> =
                        items.iter().map(|v| v.as_string()).collect();

                    let strings = strings.map_err(|e| format!("string.join: {}", e))?;
                    Ok(Value::new_string(strings.join(&sep)))
                } else {
                    Err("string.join: second argument must be a list".to_string())
                }
            }

            "contains" => {
                if args.len() != 2 {
                    return Err(format!(
                        "string.contains expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.contains: {}", e))?;
                let needle = args[1]
                    .as_string()
                    .map_err(|e| format!("string.contains: {}", e))?;
                Ok(Value::Bool(s.contains(&needle)))
            }

            "replace" => {
                if args.len() != 3 {
                    return Err(format!(
                        "string.replace expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.replace: {}", e))?;
                let from = args[1]
                    .as_string()
                    .map_err(|e| format!("string.replace: {}", e))?;
                let to = args[2]
                    .as_string()
                    .map_err(|e| format!("string.replace: {}", e))?;
                Ok(Value::new_string(s.replace(&from, &to)))
            }

            _ => Err(format!("Unknown string method: {}", method)),
        }
    }
}

/// List backend - operações com listas
pub struct ListBackend;

impl ListBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for ListBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "sort" => {
                if args.len() != 1 {
                    return Err(format!("list.sort expects 1 argument, got {}", args.len()));
                }

                if let Value::List(items) = args[0].clone() {
                    // Sort integers
                    let mut int_items: Vec<i64> = Vec::new();
                    for item in items.iter() {
                        int_items.push(item.as_int().map_err(|e| format!("list.sort: {}", e))?);
                    }
                    int_items.sort();

                    let sorted: Vec<Value> = int_items.into_iter().map(Value::Int).collect();
                    Ok(Value::new_list(sorted))
                } else {
                    Err("list.sort: argument must be a list".to_string())
                }
            }

            "reverse" => {
                if args.len() != 1 {
                    return Err(format!(
                        "list.reverse expects 1 argument, got {}",
                        args.len()
                    ));
                }

                if let Value::List(items) = args[0].clone() {
                    let mut new_items = (*items).to_vec();
                    new_items.reverse();
                    Ok(Value::new_list(new_items))
                } else {
                    Err("list.reverse: argument must be a list".to_string())
                }
            }

            "sum" => {
                if args.len() != 1 {
                    return Err(format!("list.sum expects 1 argument, got {}", args.len()));
                }

                if let Value::List(items) = &args[0] {
                    let mut sum = 0i64;
                    for item in items.iter() {
                        sum += item.as_int().map_err(|e| format!("list.sum: {}", e))?;
                    }
                    Ok(Value::Int(sum))
                } else {
                    Err("list.sum: argument must be a list".to_string())
                }
            }

            "min" => {
                if args.len() != 1 {
                    return Err(format!("list.min expects 1 argument, got {}", args.len()));
                }

                if let Value::List(items) = &args[0] {
                    if items.is_empty() {
                        return Err("list.min: empty list".to_string());
                    }

                    let mut min = items[0].as_int().map_err(|e| format!("list.min: {}", e))?;
                    for item in items.iter().skip(1) {
                        let val = item.as_int().map_err(|e| format!("list.min: {}", e))?;
                        if val < min {
                            min = val;
                        }
                    }
                    Ok(Value::Int(min))
                } else {
                    Err("list.min: argument must be a list".to_string())
                }
            }

            "max" => {
                if args.len() != 1 {
                    return Err(format!("list.max expects 1 argument, got {}", args.len()));
                }

                if let Value::List(items) = &args[0] {
                    if items.is_empty() {
                        return Err("list.max: empty list".to_string());
                    }

                    let mut max = items[0].as_int().map_err(|e| format!("list.max: {}", e))?;
                    for item in items.iter().skip(1) {
                        let val = item.as_int().map_err(|e| format!("list.max: {}", e))?;
                        if val > max {
                            max = val;
                        }
                    }
                    Ok(Value::Int(max))
                } else {
                    Err("list.max: argument must be a list".to_string())
                }
            }

            _ => Err(format!("Unknown list method: {}", method)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn test_math_abs() {
        let mut math = MathBackend::new();
        let result = math.call("abs", vec![Value::Int(-5)]).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_math_min_max() {
        let mut math = MathBackend::new();

        let min = math
            .call("min", vec![Value::Int(10), Value::Int(20)])
            .unwrap();
        assert_eq!(min, Value::Int(10));

        let max = math
            .call("max", vec![Value::Int(10), Value::Int(20)])
            .unwrap();
        assert_eq!(max, Value::Int(20));
    }

    #[test]
    fn test_math_pow() {
        let mut math = MathBackend::new();
        let result = math
            .call("pow", vec![Value::Int(2), Value::Int(3)])
            .unwrap();
        assert_eq!(result, Value::Int(8));
    }

    #[test]
    fn test_string_upper_lower() {
        let mut string = StringBackend::new();

        let upper = string
            .call("upper", vec![Value::new_string("hello".to_string())])
            .unwrap();
        assert_eq!(upper, Value::new_string("HELLO".to_string()));

        let lower = string
            .call("lower", vec![Value::new_string("WORLD".to_string())])
            .unwrap();
        assert_eq!(lower, Value::new_string("world".to_string()));
    }

    #[test]
    fn test_string_split_join() {
        let mut string = StringBackend::new();

        let split = string
            .call(
                "split",
                vec![
                    Value::new_string("a,b,c".to_string()),
                    Value::new_string(",".to_string()),
                ],
            )
            .unwrap();

        if let Value::List(parts) = split {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], Value::new_string("a".to_string()));
        } else {
            assert!(false, "expected list result");
        }
    }

    #[test]
    fn test_list_sort() {
        let mut list = ListBackend::new();
        let result = list
            .call(
                "sort",
                vec![Value::new_list(vec![
                    Value::Int(3),
                    Value::Int(1),
                    Value::Int(2),
                ])],
            )
            .unwrap();

        if let Value::List(sorted) = result {
            assert_eq!(*sorted, vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        } else {
            assert!(false, "expected list result");
        }
    }

    #[test]
    fn test_list_sum() {
        let mut list = ListBackend::new();
        let result = list
            .call(
                "sum",
                vec![Value::new_list(vec![
                    Value::Int(1),
                    Value::Int(2),
                    Value::Int(3),
                ])],
            )
            .unwrap();

        assert_eq!(result, Value::Int(6));
    }

    #[test]
    fn test_math_mod() {
        let mut math = MathBackend::new();
        let result = math
            .call("mod", vec![Value::Int(10), Value::Int(3)])
            .unwrap();
        assert_eq!(result, Value::Int(1));
    }

    #[test]
    fn test_math_clamp() {
        let mut math = MathBackend::new();
        let result = math
            .call("clamp", vec![Value::Int(15), Value::Int(0), Value::Int(10)])
            .unwrap();
        assert_eq!(result, Value::Int(10));
    }

    #[test]
    fn test_time_now() {
        let mut time = TimeBackend::new();
        let result = time.call("now", vec![]).unwrap();
        if let Value::Int(timestamp) = result {
            assert!(timestamp > 0);
        } else {
            assert!(false, "expected int timestamp");
        }
    }

    #[test]
    fn test_random_int() {
        let mut random = RandomBackend::new();
        let result = random.call("int", vec![Value::Int(10)]).unwrap();
        if let Value::Int(n) = result {
            assert!((0..10).contains(&n));
        } else {
            assert!(false, "expected int result");
        }
    }

    #[test]
    fn test_random_bool() {
        let mut random = RandomBackend::new();
        let result = random.call("bool", vec![]).unwrap();
        assert!(matches!(result, Value::Bool(_)));
    }

    #[test]
    fn test_random_choice() {
        let mut random = RandomBackend::new();
        let list = Value::new_list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        let result = random.call("choice", vec![list]).unwrap();
        assert!(matches!(
            result,
            Value::Int(1) | Value::Int(2) | Value::Int(3)
        ));
    }

    #[test]
    fn test_json_stringify() {
        let mut json = JsonBackend::new();

        // Test int
        let result = json.call("stringify", vec![Value::Int(42)]).unwrap();
        assert_eq!(result, Value::new_string("42".to_string()));

        // Test string
        let result = json
            .call("stringify", vec![Value::new_string("hello".to_string())])
            .unwrap();
        assert_eq!(result, Value::new_string("\"hello\"".to_string()));

        // Test list
        let list = Value::new_list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        let result = json.call("stringify", vec![list]).unwrap();
        assert_eq!(result, Value::new_string("[1,2,3]".to_string()));
    }

    #[test]
    fn test_json_parse() {
        let mut json = JsonBackend::new();

        // Test int
        let result = json
            .call("parse", vec![Value::new_string("42".to_string())])
            .unwrap();
        assert_eq!(result, Value::Int(42));

        // Test bool
        let result = json
            .call("parse", vec![Value::new_string("true".to_string())])
            .unwrap();
        assert_eq!(result, Value::Bool(true));

        // Test string
        let result = json
            .call("parse", vec![Value::new_string("\"hello\"".to_string())])
            .unwrap();
        assert_eq!(result, Value::new_string("hello".to_string()));
    }
}

/// Time backend - operações com tempo
pub struct TimeBackend;

impl TimeBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TimeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for TimeBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "now" => {
                if !args.is_empty() {
                    return Err(format!("time.now expects 0 arguments, got {}", args.len()));
                }
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| format!("time.now: {}", e))?;
                Ok(Value::Int(now.as_millis() as i64))
            }

            "sleep" => {
                if args.len() != 1 {
                    return Err(format!("time.sleep expects 1 argument, got {}", args.len()));
                }
                let ms = args[0].as_int().map_err(|e| format!("time.sleep: {}", e))?;
                if ms < 0 {
                    return Err("time.sleep: negative duration not allowed".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                Ok(Value::Unit)
            }

            _ => Err(format!("Unknown time method: {}", method)),
        }
    }
}

/// Random backend - números aleatórios
pub struct RandomBackend {
    seed: u64,
}

impl RandomBackend {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0);
        Self { seed }
    }

    // Simple LCG (Linear Congruential Generator)
    fn next(&mut self) -> u64 {
        self.seed = self
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.seed
    }
}

impl Default for RandomBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for RandomBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "int" => {
                if args.is_empty() {
                    // Random int (full range)
                    Ok(Value::Int(self.next() as i64))
                } else if args.len() == 1 {
                    // Random int [0, max)
                    let max = args[0].as_int().map_err(|e| format!("random.int: {}", e))?;
                    if max <= 0 {
                        return Err("random.int: max must be positive".to_string());
                    }
                    let result = (self.next() % max as u64) as i64;
                    Ok(Value::Int(result))
                } else if args.len() == 2 {
                    // Random int [min, max)
                    let min = args[0].as_int().map_err(|e| format!("random.int: {}", e))?;
                    let max = args[1].as_int().map_err(|e| format!("random.int: {}", e))?;
                    if min >= max {
                        return Err("random.int: min must be less than max".to_string());
                    }
                    let range = (max - min) as u64;
                    let result = min + (self.next() % range) as i64;
                    Ok(Value::Int(result))
                } else {
                    Err(format!(
                        "random.int expects 0, 1, or 2 arguments, got {}",
                        args.len()
                    ))
                }
            }

            "bool" => {
                if !args.is_empty() {
                    return Err(format!(
                        "random.bool expects 0 arguments, got {}",
                        args.len()
                    ));
                }
                Ok(Value::Bool(self.next().is_multiple_of(2)))
            }

            "choice" => {
                if args.len() != 1 {
                    return Err(format!(
                        "random.choice expects 1 argument, got {}",
                        args.len()
                    ));
                }
                if let Value::List(items) = &args[0] {
                    if items.is_empty() {
                        return Err("random.choice: empty list".to_string());
                    }
                    let index = (self.next() % items.len() as u64) as usize;
                    Ok(items[index].clone())
                } else {
                    Err("random.choice: argument must be a list".to_string())
                }
            }

            _ => Err(format!("Unknown random method: {}", method)),
        }
    }
}

/// JSON backend - parse e stringify JSON
pub struct JsonBackend;

impl JsonBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for JsonBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "stringify" => {
                if args.len() != 1 {
                    return Err(format!(
                        "json.stringify expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let json = value_to_json(&args[0])?;
                Ok(Value::new_string(json))
            }

            "parse" => {
                if args.len() != 1 {
                    return Err(format!("json.parse expects 1 argument, got {}", args.len()));
                }
                let json_str = args[0]
                    .as_string()
                    .map_err(|e| format!("json.parse: {}", e))?;
                let value = json_to_value(&json_str)?;
                Ok(value)
            }

            _ => Err(format!("Unknown json method: {}", method)),
        }
    }
}

// Helper: Convert Matter Value to JSON string
fn value_to_json(value: &Value) -> Result<String, String> {
    match value {
        Value::Int(n) => Ok(n.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::String(s) => Ok(format!("\"{}\"", (**s).replace('\"', "\\\""))),
        Value::Unit => Ok("null".to_string()),
        Value::List(items) => {
            let json_items: Result<Vec<String>, String> = items.iter().map(value_to_json).collect();
            let json_items = json_items?;
            Ok(format!("[{}]", json_items.join(",")))
        }
        Value::Map(entries) => {
            let mut json_entries = Vec::new();
            for (key, value) in entries.iter() {
                let json_value = value_to_json(value)?;
                json_entries.push(format!("\"{}\":{}", key, json_value));
            }
            Ok(format!("{{{}}}", json_entries.join(",")))
        }
        Value::Struct { type_name, fields } => {
            let mut json_entries = Vec::new();
            json_entries.push(format!("\"__type\":\"{}\"", **type_name));
            for (key, value) in fields.iter() {
                let json_value = value_to_json(value)?;
                json_entries.push(format!("\"{}\":{}", key, json_value));
            }
            Ok(format!("{{{}}}", json_entries.join(",")))
        }
        Value::Function(name) => Ok(format!("\"<function {}>\"", **name)),
    }
}

// Helper: Convert JSON string to Matter Value (simplified parser)
fn json_to_value(json: &str) -> Result<Value, String> {
    let json = json.trim();

    // null
    if json == "null" {
        return Ok(Value::Unit);
    }

    // boolean
    if json == "true" {
        return Ok(Value::Bool(true));
    }
    if json == "false" {
        return Ok(Value::Bool(false));
    }

    // number
    if let Ok(n) = json.parse::<i64>() {
        return Ok(Value::Int(n));
    }

    // string
    if json.starts_with('"') && json.ends_with('"') {
        let s = &json[1..json.len() - 1];
        return Ok(Value::new_string(s.replace("\\\"", "\"")));
    }

    // array
    if json.starts_with('[') && json.ends_with(']') {
        let inner = &json[1..json.len() - 1].trim();
        if inner.is_empty() {
            return Ok(Value::new_list(Vec::new()));
        }

        // Simple split by comma (doesn't handle nested structures perfectly)
        let items: Result<Vec<Value>, String> = inner
            .split(',')
            .map(|item| json_to_value(item.trim()))
            .collect();
        return Ok(Value::new_list(items?));
    }

    // object
    if json.starts_with('{') && json.ends_with('}') {
        let inner = &json[1..json.len() - 1].trim();
        if inner.is_empty() {
            return Ok(Value::new_map(HashMap::new()));
        }

        let mut map = HashMap::new();
        // Simple split by comma (doesn't handle nested structures perfectly)
        for pair in inner.split(',') {
            if let Some((key, value)) = pair.split_once(':') {
                let key = key.trim();
                if key.starts_with('"') && key.ends_with('"') {
                    let key = &key[1..key.len() - 1];
                    let value = json_to_value(value.trim())?;
                    map.insert(key.to_string(), value);
                }
            }
        }
        return Ok(Value::new_map(map));
    }

    Err(format!("json.parse: invalid JSON: {}", json))
}
