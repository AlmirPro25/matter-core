//! Matter Standard Library
//! Biblioteca padrão com módulos math, string, list, time, random, json, etc

// Sprint 80: Extended stdlib modules
pub mod file_io;
pub mod vec;
pub mod hashmap;

// Tensor backend: algebra linear nativa em Rust (matmul, softmax, etc.)
pub mod tensor;

// Re-exports
pub use file_io::FileBackend as FileIOBackend;
pub use vec::VecBackend;
pub use hashmap::HashMapBackend;
pub use tensor::TensorBackend;

use matter_backend::{Backend, Value};
use std::collections::HashMap;
use std::io::Write;
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
                let base = args[0].as_float().map_err(|e| format!("math.pow: {}", e))?;
                let exp = args[1].as_float().map_err(|e| format!("math.pow: {}", e))?;
                Ok(Value::Float(base.powf(exp)))
            }

            "sqrt" => {
                if args.len() != 1 {
                    return Err(format!("math.sqrt expects 1 argument, got {}", args.len()));
                }
                let n = args[0]
                    .as_float()
                    .map_err(|e| format!("math.sqrt: {}", e))?;
                if n < 0.0 {
                    return Err("math.sqrt: negative number not supported".to_string());
                }
                Ok(Value::Float(n.sqrt()))
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

            "pi" => Ok(Value::Float(std::f64::consts::PI)),
            "e" => Ok(Value::Float(std::f64::consts::E)),

            "sin" => {
                if args.len() != 1 {
                    return Err(format!("math.sin expects 1 argument, got {}", args.len()));
                }
                let x = args[0].as_float().map_err(|e| format!("math.sin: {}", e))?;
                Ok(Value::Float(x.sin()))
            }
            "cos" => {
                if args.len() != 1 {
                    return Err(format!("math.cos expects 1 argument, got {}", args.len()));
                }
                let x = args[0].as_float().map_err(|e| format!("math.cos: {}", e))?;
                Ok(Value::Float(x.cos()))
            }
            "tan" => {
                if args.len() != 1 {
                    return Err(format!("math.tan expects 1 argument, got {}", args.len()));
                }
                let x = args[0].as_float().map_err(|e| format!("math.tan: {}", e))?;
                Ok(Value::Float(x.tan()))
            }
            "log" => {
                if args.len() != 1 {
                    return Err(format!("math.log expects 1 argument, got {}", args.len()));
                }
                let x = args[0].as_float().map_err(|e| format!("math.log: {}", e))?;
                if x <= 0.0 {
                    return Err("math.log: argument must be positive".to_string());
                }
                Ok(Value::Float(x.log10()))
            }
            "ln" => {
                if args.len() != 1 {
                    return Err(format!("math.ln expects 1 argument, got {}", args.len()));
                }
                let x = args[0].as_float().map_err(|e| format!("math.ln: {}", e))?;
                if x <= 0.0 {
                    return Err("math.ln: argument must be positive".to_string());
                }
                Ok(Value::Float(x.ln()))
            }
            "floor" => {
                if args.len() != 1 {
                    return Err(format!("math.floor expects 1 argument, got {}", args.len()));
                }
                let x = args[0]
                    .as_float()
                    .map_err(|e| format!("math.floor: {}", e))?;
                Ok(Value::Int(x.floor() as i64))
            }
            "ceil" => {
                if args.len() != 1 {
                    return Err(format!("math.ceil expects 1 argument, got {}", args.len()));
                }
                let x = args[0]
                    .as_float()
                    .map_err(|e| format!("math.ceil: {}", e))?;
                Ok(Value::Int(x.ceil() as i64))
            }
            "round" => {
                if args.len() != 1 {
                    return Err(format!("math.round expects 1 argument, got {}", args.len()));
                }
                let x = args[0]
                    .as_float()
                    .map_err(|e| format!("math.round: {}", e))?;
                Ok(Value::Int(x.round() as i64))
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

            "starts_with" => {
                if args.len() != 2 {
                    return Err(format!(
                        "string.starts_with expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.starts_with: {}", e))?;
                let prefix = args[1]
                    .as_string()
                    .map_err(|e| format!("string.starts_with: {}", e))?;
                Ok(Value::Bool(s.starts_with(&prefix)))
            }
            "ends_with" => {
                if args.len() != 2 {
                    return Err(format!(
                        "string.ends_with expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.ends_with: {}", e))?;
                let suffix = args[1]
                    .as_string()
                    .map_err(|e| format!("string.ends_with: {}", e))?;
                Ok(Value::Bool(s.ends_with(&suffix)))
            }
            "substring" => {
                if args.len() != 3 {
                    return Err(format!(
                        "string.substring expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.substring: {}", e))?;
                let start = args[1]
                    .as_int()
                    .map_err(|e| format!("string.substring: {}", e))?
                    as usize;
                let end = args[2]
                    .as_int()
                    .map_err(|e| format!("string.substring: {}", e))?
                    as usize;
                let len = s.len();
                if start > len || end > len || start > end {
                    return Err(format!(
                        "string.substring: index out of bounds (start={}, end={}, len={})",
                        start, end, len
                    ));
                }
                Ok(Value::new_string(s[start..end].to_string()))
            }
            "repeat" => {
                if args.len() != 2 {
                    return Err(format!(
                        "string.repeat expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.repeat: {}", e))?;
                let n = args[1]
                    .as_int()
                    .map_err(|e| format!("string.repeat: {}", e))?;
                if n < 0 {
                    return Err("string.repeat: count must be non-negative".to_string());
                }
                Ok(Value::new_string(s.repeat(n as usize)))
            }
            "index_of" => {
                if args.len() != 2 {
                    return Err(format!(
                        "string.index_of expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.index_of: {}", e))?;
                let needle = args[1]
                    .as_string()
                    .map_err(|e| format!("string.index_of: {}", e))?;
                match s.find(&needle) {
                    Some(pos) => Ok(Value::Int(pos as i64)),
                    None => Ok(Value::Int(-1)),
                }
            }
            "char_at" => {
                if args.len() != 2 {
                    return Err(format!(
                        "string.char_at expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.char_at: {}", e))?;
                let idx = args[1]
                    .as_int()
                    .map_err(|e| format!("string.char_at: {}", e))?
                    as usize;
                if idx >= s.len() {
                    return Err(format!(
                        "string.char_at: index {} out of bounds (len={})",
                        idx,
                        s.len()
                    ));
                }
                Ok(Value::new_string(s.chars().nth(idx).unwrap().to_string()))
            }

            "format" => {
                // string.format("Hello, {0}! You have {1} messages.", ["Almir", 5])
                // OR string.format("Hi {name}!", {name: "Almir"})
                if args.len() < 2 {
                    return Err(format!(
                        "string.format expects at least 2 arguments, got {}",
                        args.len()
                    ));
                }
                let template = args[0]
                    .as_string()
                    .map_err(|e| format!("string.format: {}", e))?;
                let mut result = template.clone();

                match &args[1] {
                    Value::List(items) => {
                        for (i, item) in items.iter().enumerate() {
                            let placeholder = format!("{{{}}}", i);
                            result = result.replace(&placeholder, &item.to_display_string());
                        }
                    }
                    Value::Map(m) => {
                        for (key, val) in m.iter() {
                            let placeholder = format!("{{{}}}", key);
                            result = result.replace(&placeholder, &val.to_display_string());
                        }
                    }
                    other => {
                        // Single value: replace {0}
                        result = result.replace("{0}", &other.to_display_string());
                    }
                }
                Ok(Value::new_string(result))
            }

            "pad_left" => {
                if args.len() != 3 {
                    return Err(format!(
                        "string.pad_left expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.pad_left: {}", e))?;
                let width = args[1]
                    .as_int()
                    .map_err(|e| format!("string.pad_left: {}", e))?
                    as usize;
                let pad_char = args[2]
                    .as_string()
                    .map_err(|e| format!("string.pad_left: {}", e))?;
                let pad = pad_char.chars().next().unwrap_or(' ');
                if s.len() >= width {
                    Ok(Value::new_string(s))
                } else {
                    let padding = std::iter::repeat(pad)
                        .take(width - s.len())
                        .collect::<String>();
                    Ok(Value::new_string(format!("{}{}", padding, s)))
                }
            }

            "pad_right" => {
                if args.len() != 3 {
                    return Err(format!(
                        "string.pad_right expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                let s = args[0]
                    .as_string()
                    .map_err(|e| format!("string.pad_right: {}", e))?;
                let width = args[1]
                    .as_int()
                    .map_err(|e| format!("string.pad_right: {}", e))?
                    as usize;
                let pad_char = args[2]
                    .as_string()
                    .map_err(|e| format!("string.pad_right: {}", e))?;
                let pad = pad_char.chars().next().unwrap_or(' ');
                if s.len() >= width {
                    Ok(Value::new_string(s))
                } else {
                    let padding = std::iter::repeat(pad)
                        .take(width - s.len())
                        .collect::<String>();
                    Ok(Value::new_string(format!("{}{}", s, padding)))
                }
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

            "push" => {
                if args.len() != 2 {
                    return Err(format!("list.push expects 2 arguments, got {}", args.len()));
                }
                if let Value::List(items) = args[0].clone() {
                    let mut new_items = (*items).to_vec();
                    new_items.push(args[1].clone());
                    Ok(Value::new_list(new_items))
                } else {
                    Err("list.push: first argument must be a list".to_string())
                }
            }

            "pop" => {
                if args.len() != 1 {
                    return Err(format!("list.pop expects 1 argument, got {}", args.len()));
                }
                if let Value::List(items) = args[0].clone() {
                    let mut new_items = (*items).to_vec();
                    new_items.pop();
                    Ok(Value::new_list(new_items))
                } else {
                    Err("list.pop: argument must be a list".to_string())
                }
            }

            "slice" => {
                if args.len() != 3 {
                    return Err(format!(
                        "list.slice expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                if let Value::List(items) = &args[0] {
                    let start =
                        args[1].as_int().map_err(|e| format!("list.slice: {}", e))? as usize;
                    let end = args[2].as_int().map_err(|e| format!("list.slice: {}", e))? as usize;
                    if start > items.len() || end > items.len() || start > end {
                        return Err(format!(
                            "list.slice: index out of bounds (start={}, end={}, len={})",
                            start,
                            end,
                            items.len()
                        ));
                    }
                    Ok(Value::new_list(items[start..end].to_vec()))
                } else {
                    Err("list.slice: first argument must be a list".to_string())
                }
            }

            "range" => {
                if args.len() == 1 {
                    let end = args[0].as_int().map_err(|e| format!("list.range: {}", e))?;
                    let items: Vec<Value> = (0..end).map(Value::Int).collect();
                    Ok(Value::new_list(items))
                } else if args.len() == 2 {
                    let start = args[0].as_int().map_err(|e| format!("list.range: {}", e))?;
                    let end = args[1].as_int().map_err(|e| format!("list.range: {}", e))?;
                    let items: Vec<Value> = (start..end).map(Value::Int).collect();
                    Ok(Value::new_list(items))
                } else {
                    Err(format!(
                        "list.range expects 1 or 2 arguments, got {}",
                        args.len()
                    ))
                }
            }

            "concat" => {
                if args.len() != 2 {
                    return Err(format!(
                        "list.concat expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                if let (Value::List(a), Value::List(b)) = (&args[0], &args[1]) {
                    let mut new_items = (*a).to_vec();
                    new_items.extend((*b).to_vec());
                    Ok(Value::new_list(new_items))
                } else {
                    Err("list.concat: both arguments must be lists".to_string())
                }
            }

            "contains" => {
                if args.len() != 2 {
                    return Err(format!(
                        "list.contains expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                if let Value::List(items) = &args[0] {
                    Ok(Value::Bool(items.contains(&args[1])))
                } else {
                    Err("list.contains: first argument must be a list".to_string())
                }
            }

            "len" => {
                if args.len() != 1 {
                    return Err(format!("list.len expects 1 argument, got {}", args.len()));
                }
                if let Value::List(items) = &args[0] {
                    Ok(Value::Int(items.len() as i64))
                } else {
                    Err("list.len: argument must be a list".to_string())
                }
            }

            "get" => {
                if args.len() != 2 {
                    return Err(format!("list.get expects 2 arguments, got {}", args.len()));
                }
                if let Value::List(items) = &args[0] {
                    let idx = args[1].as_int().map_err(|e| format!("list.get: {}", e))?;
                    if idx < 0 || idx >= items.len() as i64 {
                        return Err(format!(
                            "list.get: index {} out of bounds (len={})",
                            idx,
                            items.len()
                        ));
                    }
                    Ok(items[idx as usize].clone())
                } else {
                    Err("list.get: first argument must be a list".to_string())
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
        assert_eq!(result, Value::Float(8.0));
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
            panic!("expected list result");
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
            panic!("expected list result");
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
            panic!("expected int timestamp");
        }
    }

    #[test]
    fn test_random_int() {
        let mut random = RandomBackend::new();
        let result = random.call("int", vec![Value::Int(10)]).unwrap();
        if let Value::Int(n) = result {
            assert!((0..10).contains(&n));
        } else {
            panic!("expected int result");
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

        // Test complex JSON with quotes, commas and escaping
        let result = json
            .call("parse", vec![Value::new_string("{\"name\":\"Almir, Dev\",\"skills\":[\"Rust\",\"Matter Core\"],\"active\":true,\"empty_list\":[]}".to_string())])
            .unwrap();
        if let Value::Map(map) = result {
            assert_eq!(
                map.get("name").unwrap(),
                &Value::new_string("Almir, Dev".to_string())
            );
            assert_eq!(map.get("active").unwrap(), &Value::Bool(true));
            if let Value::List(skills) = map.get("skills").unwrap() {
                assert_eq!(skills[0], Value::new_string("Rust".to_string()));
                assert_eq!(skills[1], Value::new_string("Matter Core".to_string()));
            } else {
                panic!("Expected list for skills");
            }
        } else {
            panic!("Expected map result");
        }
    }

    #[test]
    fn test_list_get() {
        let mut list_backend = ListBackend::new();
        let list = Value::new_list(vec![Value::Int(10), Value::Int(20), Value::Int(30)]);
        let result = list_backend.call("get", vec![list, Value::Int(1)]).unwrap();
        assert_eq!(result, Value::Int(20));
    }

    #[test]
    fn test_file_lines_and_write_lines() {
        let mut file_backend = FileBackend::new();
        let temp_path = std::env::temp_dir().join("matter_test_lines.txt");
        let path_str = Value::new_string(temp_path.to_string_lossy().to_string());

        let lines_to_write = Value::new_list(vec![
            Value::new_string("linha 1".to_string()),
            Value::new_string("linha 2".to_string()),
            Value::new_string("linha 3".to_string()),
        ]);

        let write_res = file_backend
            .call("write_lines", vec![path_str.clone(), lines_to_write])
            .unwrap();
        assert_eq!(write_res, Value::Bool(true));

        let read_res = file_backend.call("lines", vec![path_str.clone()]).unwrap();
        if let Value::List(lines) = read_res {
            assert_eq!(lines.len(), 3);
            assert_eq!(lines[0].as_string().unwrap(), "linha 1");
            assert_eq!(lines[1].as_string().unwrap(), "linha 2");
            assert_eq!(lines[2].as_string().unwrap(), "linha 3");
        } else {
            panic!("Expected list of lines");
        }

        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_audio_melody_and_chord() {
        let mut audio_backend = AudioBackend::new();

        let freqs = Value::new_list(vec![Value::Int(440), Value::Int(880)]);
        let durations = Value::new_list(vec![Value::Int(5), Value::Int(5)]); // short duration for test

        let melody_res = audio_backend
            .call("melody", vec![freqs.clone(), durations])
            .unwrap();
        assert_eq!(melody_res, Value::Bool(true));

        let chord_res = audio_backend
            .call("chord", vec![freqs, Value::Int(5)])
            .unwrap();
        assert_eq!(chord_res, Value::Bool(true));
    }

    #[test]
    fn world_backend_partitions_cells_and_limits_interest() {
        let mut world = WorldBackend::new();
        world
            .call(
                "configure",
                vec![
                    Value::Float(100.0),
                    Value::Float(150.0),
                    Value::Int(2),
                    Value::Int(2),
                ],
            )
            .unwrap();

        for (id, x, y) in [
            ("p1", 10.0, 10.0),
            ("p2", 15.0, 15.0),
            ("p3", 20.0, 20.0),
            ("p4", 120.0, 10.0),
        ] {
            world
                .call(
                    "place",
                    vec![
                        Value::new_string(id.to_string()),
                        Value::Float(x),
                        Value::Float(y),
                    ],
                )
                .unwrap();
        }

        let nearby = world
            .call("nearby", vec![Value::new_string("p1".to_string())])
            .unwrap();
        let Value::Map(nearby_map) = nearby else {
            panic!("world.nearby should return a map");
        };
        assert_eq!(nearby_map.get("visible_count"), Some(&Value::Int(2)));
        assert_eq!(nearby_map.get("hidden_count"), Some(&Value::Int(1)));
        assert_eq!(nearby_map.get("degraded"), Some(&Value::Bool(true)));

        let plan = world.call("plan", vec![]).unwrap();
        let Value::Map(plan_map) = plan else {
            panic!("world.plan should return a map");
        };
        assert_eq!(plan_map.get("entities"), Some(&Value::Int(4)));
        assert_eq!(plan_map.get("cell_count"), Some(&Value::Int(2)));
        assert_eq!(plan_map.get("hot_cells"), Some(&Value::Int(1)));
        assert_eq!(plan_map.get("degraded"), Some(&Value::Bool(true)));
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
        Value::Null => Ok("null".to_string()),
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
        Value::Closure(_) => Ok("\"<closure>\"".to_string()),
    }
}

// Helper: Convert JSON string to Matter Value (robust parser)
struct JsonParser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn parse(&mut self) -> Result<Value, String> {
        self.skip_whitespace();
        match self.chars.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string(),
            Some(&c) if c == '-' || c == '+' || c.is_ascii_digit() => self.parse_number(),
            Some('t') | Some('f') => self.parse_boolean(),
            Some('n') => self.parse_null(),
            Some(c) => Err(format!("Unexpected character: '{}'", c)),
            None => Err("Unexpected end of input".to_string()),
        }
    }

    fn parse_object(&mut self) -> Result<Value, String> {
        self.chars.next(); // consume '{'
        let mut map = HashMap::new();
        loop {
            self.skip_whitespace();
            if self.chars.peek() == Some(&'}') {
                self.chars.next();
                break;
            }
            let key = match self.parse()? {
                Value::String(s) => (*s).clone(),
                other => return Err(format!("Expected string key in object, got {:?}", other)),
            };
            self.skip_whitespace();
            if self.chars.next() != Some(':') {
                return Err("Expected ':' after key in object".to_string());
            }
            let val = self.parse()?;
            map.insert(key, val);
            self.skip_whitespace();
            match self.chars.peek() {
                Some(',') => {
                    self.chars.next();
                }
                Some('}') => {
                    self.chars.next();
                    break;
                }
                Some(c) => return Err(format!("Expected ',' or '}}' in object, got '{}'", c)),
                None => return Err("Unexpected end of input inside object".to_string()),
            }
        }
        Ok(Value::new_map(map))
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.chars.next(); // consume '['
        let mut list = Vec::new();
        loop {
            self.skip_whitespace();
            if self.chars.peek() == Some(&']') {
                self.chars.next();
                break;
            }
            let val = self.parse()?;
            list.push(val);
            self.skip_whitespace();
            match self.chars.peek() {
                Some(',') => {
                    self.chars.next();
                }
                Some(']') => {
                    self.chars.next();
                    break;
                }
                Some(c) => return Err(format!("Expected ',' or ']' in array, got '{}'", c)),
                None => return Err("Unexpected end of input inside array".to_string()),
            }
        }
        Ok(Value::new_list(list))
    }

    fn parse_string(&mut self) -> Result<Value, String> {
        self.chars.next(); // consume '"'
        let mut s = String::new();
        while let Some(c) = self.chars.next() {
            if c == '"' {
                return Ok(Value::new_string(s));
            } else if c == '\\' {
                match self.chars.next() {
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some('/') => s.push('/'),
                    Some('b') => s.push('\x08'),
                    Some('f') => s.push('\x0C'),
                    Some('n') => s.push('\n'),
                    Some('r') => s.push('\r'),
                    Some('t') => s.push('\t'),
                    Some('u') => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            if let Some(hc) = self.chars.next() {
                                hex.push(hc);
                            } else {
                                return Err("Invalid unicode escape".to_string());
                            }
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = std::char::from_u32(code) {
                                s.push(ch);
                            } else {
                                s.push_str(&format!("\\u{}", hex));
                            }
                        } else {
                            s.push_str(&format!("\\u{}", hex));
                        }
                    }
                    Some(escaped) => {
                        s.push('\\');
                        s.push(escaped);
                    }
                    None => return Err("Unexpected end of input inside string escape".to_string()),
                }
            } else {
                s.push(c);
            }
        }
        Err("Unterminated string".to_string())
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let mut s = String::new();
        let mut is_float = false;
        while let Some(&c) = self.chars.peek() {
            if c == '-' || c == '+' || c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' {
                if c == '.' || c == 'e' || c == 'E' {
                    is_float = true;
                }
                s.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }
        if is_float {
            if let Ok(f) = s.parse::<f64>() {
                Ok(Value::Float(f))
            } else {
                Err(format!("Invalid float: {}", s))
            }
        } else {
            if let Ok(i) = s.parse::<i64>() {
                Ok(Value::Int(i))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(Value::Float(f))
            } else {
                Err(format!("Invalid integer: {}", s))
            }
        }
    }

    fn parse_boolean(&mut self) -> Result<Value, String> {
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphabetic() {
                s.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }
        if s == "true" {
            Ok(Value::Bool(true))
        } else if s == "false" {
            Ok(Value::Bool(false))
        } else {
            Err(format!("Expected boolean, got '{}'", s))
        }
    }

    fn parse_null(&mut self) -> Result<Value, String> {
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphabetic() {
                s.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }
        if s == "null" {
            Ok(Value::Unit)
        } else {
            Err(format!("Expected null, got '{}'", s))
        }
    }
}

fn json_to_value(json: &str) -> Result<Value, String> {
    let mut parser = JsonParser::new(json);
    let val = parser.parse()?;
    parser.skip_whitespace();
    if parser.chars.peek().is_some() {
        return Err("Unexpected trailing characters in JSON".to_string());
    }
    Ok(val)
}

/// Map backend - operações com dicionários
pub struct MapBackend;

impl MapBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MapBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for MapBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "new" => Ok(Value::new_map(HashMap::new())),

            "get" => {
                if args.len() != 2 {
                    return Err(format!("map.get expects 2 arguments, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    let key = args[1].as_string().map_err(|e| format!("map.get: {}", e))?;
                    Ok(m.get(&key).cloned().unwrap_or(Value::Unit))
                } else {
                    Err("map.get: first argument must be a map".to_string())
                }
            }

            "set" => {
                if args.len() != 3 {
                    return Err(format!("map.set expects 3 arguments, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    let key = args[1].as_string().map_err(|e| format!("map.set: {}", e))?;
                    let mut new_map = (**m).clone();
                    new_map.insert(key, args[2].clone());
                    Ok(Value::new_map(new_map))
                } else {
                    Err("map.set: first argument must be a map".to_string())
                }
            }

            "remove" => {
                if args.len() != 2 {
                    return Err(format!(
                        "map.remove expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                if let Value::Map(m) = &args[0] {
                    let key = args[1]
                        .as_string()
                        .map_err(|e| format!("map.remove: {}", e))?;
                    let mut new_map = (**m).clone();
                    new_map.remove(&key);
                    Ok(Value::new_map(new_map))
                } else {
                    Err("map.remove: first argument must be a map".to_string())
                }
            }

            "has" => {
                if args.len() != 2 {
                    return Err(format!("map.has expects 2 arguments, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    let key = args[1].as_string().map_err(|e| format!("map.has: {}", e))?;
                    Ok(Value::Bool(m.contains_key(&key)))
                } else {
                    Err("map.has: first argument must be a map".to_string())
                }
            }

            "keys" => {
                if args.len() != 1 {
                    return Err(format!("map.keys expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    let keys: Vec<Value> = m.keys().map(|k| Value::new_string(k.clone())).collect();
                    Ok(Value::new_list(keys))
                } else {
                    Err("map.keys: argument must be a map".to_string())
                }
            }

            "values" => {
                if args.len() != 1 {
                    return Err(format!("map.values expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    let values: Vec<Value> = m.values().cloned().collect();
                    Ok(Value::new_list(values))
                } else {
                    Err("map.values: argument must be a map".to_string())
                }
            }

            "size" => {
                if args.len() != 1 {
                    return Err(format!("map.size expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    Ok(Value::Int(m.len() as i64))
                } else {
                    Err("map.size: argument must be a map".to_string())
                }
            }

            "merge" => {
                if args.len() != 2 {
                    return Err(format!("map.merge expects 2 arguments, got {}", args.len()));
                }
                if let (Value::Map(a), Value::Map(b)) = (&args[0], &args[1]) {
                    let mut merged = (**a).clone();
                    for (k, v) in b.iter() {
                        merged.insert(k.clone(), v.clone());
                    }
                    Ok(Value::new_map(merged))
                } else {
                    Err("map.merge: both arguments must be maps".to_string())
                }
            }

            _ => Err(format!("Unknown map method: {}", method)),
        }
    }
}

/// Type backend - inspeção e conversão de tipos
pub struct TypeBackend;

impl TypeBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for TypeBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "of" => {
                if args.len() != 1 {
                    return Err(format!("type.of expects 1 argument, got {}", args.len()));
                }
                let t = match &args[0] {
                    Value::Int(_) => "int",
                    Value::Float(_) => "float",
                    Value::Bool(_) => "bool",
                    Value::String(_) => "string",
                    Value::List(_) => "list",
                    Value::Map(_) => "map",
                    Value::Struct { .. } => "struct",
                    Value::Function(_) => "function",
                    Value::Closure(_) => "closure",
                    Value::Unit => "unit",
                    Value::Null => "null",
                };
                Ok(Value::new_string(t.to_string()))
            }

            "to_int" => {
                if args.len() != 1 {
                    return Err(format!(
                        "type.to_int expects 1 argument, got {}",
                        args.len()
                    ));
                }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(*n)),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                    Value::String(s) => s
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| format!("type.to_int: cannot convert '{}' to int", s)),
                    _ => Err(format!("type.to_int: cannot convert {:?} to int", args[0])),
                }
            }

            "to_float" => {
                if args.len() != 1 {
                    return Err(format!(
                        "type.to_float expects 1 argument, got {}",
                        args.len()
                    ));
                }
                match &args[0] {
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::Int(n) => Ok(Value::Float(*n as f64)),
                    Value::String(s) => s
                        .parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| format!("type.to_float: cannot convert '{}' to float", s)),
                    _ => Err(format!(
                        "type.to_float: cannot convert {:?} to float",
                        args[0]
                    )),
                }
            }

            "to_string" => {
                if args.len() != 1 {
                    return Err(format!(
                        "type.to_string expects 1 argument, got {}",
                        args.len()
                    ));
                }
                Ok(Value::new_string(args[0].to_display_string()))
            }

            "is_int" => {
                if args.len() != 1 {
                    return Err(format!(
                        "type.is_int expects 1 argument, got {}",
                        args.len()
                    ));
                }
                Ok(Value::Bool(matches!(&args[0], Value::Int(_))))
            }
            "is_float" => {
                if args.len() != 1 {
                    return Err(format!(
                        "type.is_float expects 1 argument, got {}",
                        args.len()
                    ));
                }
                Ok(Value::Bool(matches!(&args[0], Value::Float(_))))
            }
            "is_string" => {
                if args.len() != 1 {
                    return Err(format!(
                        "type.is_string expects 1 argument, got {}",
                        args.len()
                    ));
                }
                Ok(Value::Bool(matches!(&args[0], Value::String(_))))
            }
            "is_list" => {
                if args.len() != 1 {
                    return Err(format!(
                        "type.is_list expects 1 argument, got {}",
                        args.len()
                    ));
                }
                Ok(Value::Bool(matches!(&args[0], Value::List(_))))
            }
            "is_map" => {
                if args.len() != 1 {
                    return Err(format!(
                        "type.is_map expects 1 argument, got {}",
                        args.len()
                    ));
                }
                Ok(Value::Bool(matches!(&args[0], Value::Map(_))))
            }
            "is_bool" => {
                if args.len() != 1 {
                    return Err(format!(
                        "type.is_bool expects 1 argument, got {}",
                        args.len()
                    ));
                }
                Ok(Value::Bool(matches!(&args[0], Value::Bool(_))))
            }

            _ => Err(format!("Unknown type method: {}", method)),
        }
    }
}

/// Console backend - I/O interativo
pub struct ConsoleBackend;

impl ConsoleBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConsoleBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for ConsoleBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "read_line" | "read" => {
                if args.len() > 1 {
                    return Err(format!(
                        "console.read expects 0 or 1 arguments, got {}",
                        args.len()
                    ));
                }
                // Optional prompt
                if args.len() == 1 {
                    let prompt = args[0]
                        .as_string()
                        .map_err(|e| format!("console.read: {}", e))?;
                    print!("{}", prompt);
                    std::io::stdout()
                        .flush()
                        .map_err(|e| format!("console.read: {}", e))?;
                }
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| format!("console.read: {}", e))?;
                Ok(Value::new_string(
                    input
                        .trim_end_matches('\n')
                        .trim_end_matches('\r')
                        .to_string(),
                ))
            }

            "write" => {
                if args.len() != 1 {
                    return Err(format!(
                        "console.write expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let text = args[0].to_display_string();
                print!("{}", text);
                std::io::stdout()
                    .flush()
                    .map_err(|e| format!("console.write: {}", e))?;
                Ok(Value::Unit)
            }

            _ => Err(format!("Unknown console method: {}", method)),
        }
    }
}

/// File backend - leitura e escrita de arquivos
pub struct FileBackend;

impl FileBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for FileBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "read" => {
                if args.len() != 1 {
                    return Err(format!("file.read expects 1 argument, got {}", args.len()));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("file.read: {}", e))?;
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("file.read: failed to read '{}': {}", path, e))?;
                Ok(Value::new_string(content))
            }

            "write" => {
                if args.len() != 2 {
                    return Err(format!(
                        "file.write expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("file.write: {}", e))?;
                let content = args[1]
                    .as_string()
                    .map_err(|e| format!("file.write: {}", e))?;
                std::fs::write(&path, &content)
                    .map_err(|e| format!("file.write: failed to write '{}': {}", path, e))?;
                Ok(Value::Bool(true))
            }

            "exists" => {
                if args.len() != 1 {
                    return Err(format!(
                        "file.exists expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("file.exists: {}", e))?;
                Ok(Value::Bool(std::path::Path::new(&path).exists()))
            }

            "delete" => {
                if args.len() != 1 {
                    return Err(format!(
                        "file.delete expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("file.delete: {}", e))?;
                std::fs::remove_file(&path)
                    .map_err(|e| format!("file.delete: failed to delete '{}': {}", path, e))?;
                Ok(Value::Bool(true))
            }

            "append" => {
                if args.len() != 2 {
                    return Err(format!(
                        "file.append expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("file.append: {}", e))?;
                let content = args[1]
                    .as_string()
                    .map_err(|e| format!("file.append: {}", e))?;
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&path)
                    .map_err(|e| format!("file.append: failed to open '{}': {}", path, e))?;
                file.write_all(content.as_bytes())
                    .map_err(|e| format!("file.append: failed to append to '{}': {}", path, e))?;
                Ok(Value::Bool(true))
            }

            "lines" => {
                if args.len() != 1 {
                    return Err(format!("file.lines expects 1 argument, got {}", args.len()));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("file.lines: {}", e))?;
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("file.lines: failed to read '{}': {}", path, e))?;
                let lines: Vec<Value> = content
                    .lines()
                    .map(|l| Value::new_string(l.to_string()))
                    .collect();
                Ok(Value::new_list(lines))
            }

            "write_lines" => {
                if args.len() != 2 {
                    return Err(format!(
                        "file.write_lines expects 2 arguments (path, list_of_strings), got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("file.write_lines: {}", e))?;
                if let Value::List(lines) = &args[1] {
                    let mut file = std::fs::File::create(&path).map_err(|e| {
                        format!("file.write_lines: failed to create '{}': {}", path, e)
                    })?;
                    for line_val in lines.iter() {
                        let line_str = line_val
                            .as_string()
                            .map_err(|e| format!("file.write_lines: {}", e))?;
                        writeln!(file, "{}", line_str).map_err(|e| {
                            format!("file.write_lines: failed to write to '{}': {}", path, e)
                        })?;
                    }
                    Ok(Value::Bool(true))
                } else {
                    Err("file.write_lines: second argument must be a list of strings".to_string())
                }
            }

            _ => Err(format!("Unknown file method: {}", method)),
        }
    }
}

/// Audio backend - síntese de áudio nativa retrô
pub struct AudioBackend;

impl AudioBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
extern "system" {
    fn Beep(dwFreq: u32, dwDuration: u32) -> i32;
}

impl Backend for AudioBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "beep" => {
                if args.len() != 2 {
                    return Err(format!(
                        "audio.beep expects 2 arguments (freq, duration_ms), got {}",
                        args.len()
                    ));
                }
                let freq = args[0].as_int().map_err(|e| format!("audio.beep: {}", e))? as u32;
                let duration = args[1].as_int().map_err(|e| format!("audio.beep: {}", e))? as u32;

                #[cfg(target_os = "windows")]
                unsafe {
                    Beep(freq, duration);
                }
                #[cfg(not(target_os = "windows"))]
                {
                    print!("\x07");
                    let _ = std::io::stdout().flush();
                }
                Ok(Value::Bool(true))
            }

            "laser" => {
                #[cfg(target_os = "windows")]
                unsafe {
                    for f in (200..=800).rev().step_by(50) {
                        Beep(f, 15);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    print!("\x07");
                    let _ = std::io::stdout().flush();
                }
                Ok(Value::Bool(true))
            }

            "jump" => {
                #[cfg(target_os = "windows")]
                unsafe {
                    for f in (150..=600).step_by(40) {
                        Beep(f, 12);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    print!("\x07");
                    let _ = std::io::stdout().flush();
                }
                Ok(Value::Bool(true))
            }

            "melody" => {
                if args.len() != 2 {
                    return Err(format!(
                        "audio.melody expects 2 arguments (frequencies, durations), got {}",
                        args.len()
                    ));
                }
                if let (Value::List(freqs), Value::List(durations)) = (&args[0], &args[1]) {
                    if freqs.len() != durations.len() {
                        return Err(format!(
                            "audio.melody: lists must be of the same length (freqs={}, durations={})",
                            freqs.len(),
                            durations.len()
                        ));
                    }
                    for i in 0..freqs.len() {
                        let freq = freqs[i]
                            .as_int()
                            .map_err(|e| format!("audio.melody: {}", e))?
                            as u32;
                        let duration = durations[i]
                            .as_int()
                            .map_err(|e| format!("audio.melody: {}", e))?
                            as u32;
                        #[cfg(target_os = "windows")]
                        unsafe {
                            Beep(freq, duration);
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            print!("\x07");
                            let _ = std::io::stdout().flush();
                            std::thread::sleep(std::time::Duration::from_millis(duration as u64));
                        }
                    }
                    Ok(Value::Bool(true))
                } else {
                    Err("audio.melody: both arguments must be lists".to_string())
                }
            }

            "chord" => {
                if args.len() != 2 {
                    return Err(format!(
                        "audio.chord expects 2 arguments (frequencies, duration_ms), got {}",
                        args.len()
                    ));
                }
                if let Value::List(freqs) = &args[0] {
                    let duration = args[1]
                        .as_int()
                        .map_err(|e| format!("audio.chord: {}", e))?
                        as u32;
                    let mut handles = Vec::new();
                    for freq_val in freqs.iter() {
                        let freq = freq_val
                            .as_int()
                            .map_err(|e| format!("audio.chord: {}", e))?
                            as u32;
                        #[cfg(target_os = "windows")]
                        {
                            let handle = std::thread::spawn(move || unsafe {
                                Beep(freq, duration);
                            });
                            handles.push(handle);
                        }
                    }
                    #[cfg(target_os = "windows")]
                    for handle in handles {
                        let _ = handle.join();
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        print!("\x07");
                        let _ = std::io::stdout().flush();
                        std::thread::sleep(std::time::Duration::from_millis(duration as u64));
                    }
                    Ok(Value::Bool(true))
                } else {
                    Err("audio.chord: first argument must be a list of frequencies".to_string())
                }
            }

            _ => Err(format!("Unknown audio method: {}", method)),
        }
    }
}

#[derive(Debug, Clone)]
struct WorldEntity {
    id: String,
    x: f64,
    y: f64,
    cell_x: i64,
    cell_y: i64,
    layer: usize,
    version: u64,
}

#[derive(Debug, Clone)]
struct WorldConfig {
    cell_size: f64,
    interest_radius: f64,
    cell_capacity: usize,
    max_visible: usize,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            cell_size: 256.0,
            interest_radius: 128.0,
            cell_capacity: 64,
            max_visible: 48,
        }
    }
}

#[derive(Debug, Clone)]
struct WorldVisibleEntity {
    id: String,
    distance: f64,
    cell_x: i64,
    cell_y: i64,
    layer: usize,
}

/// World backend - spatial partitioning, interest management, and layered cells.
pub struct WorldBackend {
    config: WorldConfig,
    entities: HashMap<String, WorldEntity>,
    version: u64,
}

impl WorldBackend {
    pub fn new() -> Self {
        Self {
            config: WorldConfig::default(),
            entities: HashMap::new(),
            version: 0,
        }
    }

    fn configure(&mut self, args: Vec<Value>) -> Result<Value, String> {
        if args.len() != 4 {
            return Err(format!(
                "world.configure expects 4 arguments (cell_size, interest_radius, cell_capacity, max_visible), got {}",
                args.len()
            ));
        }

        let cell_size = args[0]
            .as_float()
            .map_err(|e| format!("world.configure: {}", e))?;
        let interest_radius = args[1]
            .as_float()
            .map_err(|e| format!("world.configure: {}", e))?;
        let cell_capacity = args[2]
            .as_int()
            .map_err(|e| format!("world.configure: {}", e))?;
        let max_visible = args[3]
            .as_int()
            .map_err(|e| format!("world.configure: {}", e))?;

        if cell_size <= 0.0 {
            return Err("world.configure: cell_size must be > 0".to_string());
        }
        if interest_radius <= 0.0 {
            return Err("world.configure: interest_radius must be > 0".to_string());
        }
        if cell_capacity <= 0 {
            return Err("world.configure: cell_capacity must be > 0".to_string());
        }
        if max_visible <= 0 {
            return Err("world.configure: max_visible must be > 0".to_string());
        }

        self.config = WorldConfig {
            cell_size,
            interest_radius,
            cell_capacity: cell_capacity as usize,
            max_visible: max_visible as usize,
        };
        self.recompute_layers();
        Ok(self.status_payload())
    }

    fn place_entity(&mut self, args: Vec<Value>, require_new: bool) -> Result<Value, String> {
        if args.len() != 3 {
            return Err(format!(
                "world.place expects 3 arguments (id, x, y), got {}",
                args.len()
            ));
        }

        let id = args[0]
            .as_string()
            .map_err(|e| format!("world.place: {}", e))?;
        if id.trim().is_empty() {
            return Err("world.place: id must not be empty".to_string());
        }
        if require_new && self.entities.contains_key(&id) {
            return Err(format!("world.spawn: entity '{}' already exists", id));
        }

        let x = args[1]
            .as_float()
            .map_err(|e| format!("world.place: {}", e))?;
        let y = args[2]
            .as_float()
            .map_err(|e| format!("world.place: {}", e))?;
        let (cell_x, cell_y) = self.cell_for(x, y);
        self.version = self.version.saturating_add(1);
        self.entities.insert(
            id.clone(),
            WorldEntity {
                id: id.clone(),
                x,
                y,
                cell_x,
                cell_y,
                layer: 0,
                version: self.version,
            },
        );
        self.recompute_layers();
        Ok(self
            .entity_payload(&id)
            .unwrap_or_else(|| world_error_payload("missing entity after place")))
    }

    fn move_entity(&mut self, args: Vec<Value>) -> Result<Value, String> {
        if args.len() != 3 {
            return Err(format!(
                "world.move expects 3 arguments (id, x, y), got {}",
                args.len()
            ));
        }

        let id = args[0]
            .as_string()
            .map_err(|e| format!("world.move: {}", e))?;
        if !self.entities.contains_key(&id) {
            return Err(format!("world.move: unknown entity '{}'", id));
        }
        self.place_entity(
            vec![args[0].clone(), args[1].clone(), args[2].clone()],
            false,
        )
    }

    fn nearby_payload(&self, args: Vec<Value>) -> Result<Value, String> {
        if args.len() != 1 {
            return Err(format!(
                "world.nearby expects 1 argument (id), got {}",
                args.len()
            ));
        }
        let id = args[0]
            .as_string()
            .map_err(|e| format!("world.nearby: {}", e))?;
        let source = self
            .entity_payload(&id)
            .ok_or_else(|| format!("world.nearby: unknown entity '{}'", id))?;
        let (visible, hidden_count) = self.visible_for(&id)?;
        let visible_values = visible
            .iter()
            .map(|entity| {
                world_map(vec![
                    ("id", Value::new_string(entity.id.clone())),
                    ("distance", Value::Float(entity.distance)),
                    ("cell_x", Value::Int(entity.cell_x)),
                    ("cell_y", Value::Int(entity.cell_y)),
                    ("layer", Value::Int(entity.layer as i64)),
                ])
            })
            .collect::<Vec<_>>();

        Ok(world_map(vec![
            ("entity", source),
            ("visible", Value::new_list(visible_values)),
            ("visible_count", Value::Int(visible.len() as i64)),
            ("hidden_count", Value::Int(hidden_count as i64)),
            ("interest_radius", Value::Float(self.config.interest_radius)),
            ("max_visible", Value::Int(self.config.max_visible as i64)),
            ("degraded", Value::Bool(hidden_count > 0)),
        ]))
    }

    fn plan_payload(&self) -> Value {
        let cells = self.cell_payloads();
        let hot_cells = cells
            .iter()
            .filter(|cell| match cell {
                Value::Map(map) => matches!(map.get("overloaded"), Some(Value::Bool(true))),
                _ => false,
            })
            .count();
        let worst_cell_population = cells
            .iter()
            .filter_map(|cell| match cell {
                Value::Map(map) => map.get("population").and_then(|value| value.as_int().ok()),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let total_layers = cells
            .iter()
            .filter_map(|cell| match cell {
                Value::Map(map) => map.get("layers").and_then(|value| value.as_int().ok()),
                _ => None,
            })
            .sum::<i64>();
        let interest_edges = self
            .entities
            .keys()
            .filter_map(|id| self.visible_for(id).ok())
            .map(|(visible, _)| visible.len() as i64)
            .sum::<i64>();

        world_map(vec![
            (
                "mode",
                Value::new_string("logical_world_partition".to_string()),
            ),
            ("entities", Value::Int(self.entities.len() as i64)),
            ("cells", Value::new_list(cells)),
            ("cell_count", Value::Int(self.cell_count() as i64)),
            ("hot_cells", Value::Int(hot_cells as i64)),
            ("total_layers", Value::Int(total_layers)),
            ("interest_edges", Value::Int(interest_edges)),
            ("worst_cell_population", Value::Int(worst_cell_population)),
            ("degraded", Value::Bool(hot_cells > 0)),
            (
                "policy",
                Value::new_string(
                    "same-layer radius interest with overflow hidden as aggregate".to_string(),
                ),
            ),
        ])
    }

    fn status_payload(&self) -> Value {
        let plan = self.plan_payload();
        world_map(vec![
            ("backend", Value::new_string("world".to_string())),
            (
                "model",
                Value::new_string("spatial_cell_interest_runtime".to_string()),
            ),
            ("mode", Value::new_string("simulated_runtime".to_string())),
            ("stub", Value::Bool(false)),
            ("hardware", Value::Bool(false)),
            ("simulated", Value::Bool(true)),
            ("entities", Value::Int(self.entities.len() as i64)),
            ("cell_size", Value::Float(self.config.cell_size)),
            ("interest_radius", Value::Float(self.config.interest_radius)),
            (
                "cell_capacity",
                Value::Int(self.config.cell_capacity as i64),
            ),
            ("max_visible", Value::Int(self.config.max_visible as i64)),
            ("version", Value::Int(self.version as i64)),
            (
                "capabilities",
                Value::new_list(vec![
                    Value::new_string("configure".to_string()),
                    Value::new_string("place".to_string()),
                    Value::new_string("spawn".to_string()),
                    Value::new_string("move".to_string()),
                    Value::new_string("nearby".to_string()),
                    Value::new_string("plan".to_string()),
                    Value::new_string("status".to_string()),
                ]),
            ),
            ("plan", plan),
        ])
    }

    fn visible_for(&self, id: &str) -> Result<(Vec<WorldVisibleEntity>, usize), String> {
        let source = self
            .entities
            .get(id)
            .ok_or_else(|| format!("world.nearby: unknown entity '{}'", id))?;
        let mut visible = Vec::new();
        let mut hidden_count = 0usize;

        for entity in self.entities.values() {
            if entity.id == source.id {
                continue;
            }
            let distance = ((source.x - entity.x).powi(2) + (source.y - entity.y).powi(2)).sqrt();
            if distance > self.config.interest_radius {
                continue;
            }
            if entity.layer != source.layer {
                hidden_count += 1;
                continue;
            }
            visible.push(WorldVisibleEntity {
                id: entity.id.clone(),
                distance,
                cell_x: entity.cell_x,
                cell_y: entity.cell_y,
                layer: entity.layer,
            });
        }

        visible.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        if visible.len() > self.config.max_visible {
            hidden_count += visible.len() - self.config.max_visible;
            visible.truncate(self.config.max_visible);
        }

        Ok((visible, hidden_count))
    }

    fn recompute_layers(&mut self) {
        let mut cells: HashMap<(i64, i64), Vec<String>> = HashMap::new();
        for (id, entity) in &self.entities {
            cells
                .entry((entity.cell_x, entity.cell_y))
                .or_default()
                .push(id.clone());
        }

        for members in cells.values_mut() {
            members.sort();
            for (index, id) in members.iter().enumerate() {
                if let Some(entity) = self.entities.get_mut(id) {
                    entity.layer = index / self.config.cell_capacity;
                }
            }
        }
    }

    fn cell_for(&self, x: f64, y: f64) -> (i64, i64) {
        (
            (x / self.config.cell_size).floor() as i64,
            (y / self.config.cell_size).floor() as i64,
        )
    }

    fn entity_payload(&self, id: &str) -> Option<Value> {
        let entity = self.entities.get(id)?;
        Some(world_map(vec![
            ("id", Value::new_string(entity.id.clone())),
            ("x", Value::Float(entity.x)),
            ("y", Value::Float(entity.y)),
            ("cell_x", Value::Int(entity.cell_x)),
            ("cell_y", Value::Int(entity.cell_y)),
            (
                "cell",
                Value::new_string(world_cell_key(entity.cell_x, entity.cell_y)),
            ),
            ("layer", Value::Int(entity.layer as i64)),
            ("version", Value::Int(entity.version as i64)),
        ]))
    }

    fn cell_payloads(&self) -> Vec<Value> {
        let mut cells: HashMap<(i64, i64), Vec<&WorldEntity>> = HashMap::new();
        for entity in self.entities.values() {
            cells
                .entry((entity.cell_x, entity.cell_y))
                .or_default()
                .push(entity);
        }

        let mut keys = cells.keys().copied().collect::<Vec<_>>();
        keys.sort();
        keys.into_iter()
            .map(|(cell_x, cell_y)| {
                let members = cells.get(&(cell_x, cell_y)).cloned().unwrap_or_default();
                let population = members.len();
                let layers = members
                    .iter()
                    .map(|entity| entity.layer)
                    .max()
                    .map(|layer| layer + 1)
                    .unwrap_or(0);
                world_map(vec![
                    ("key", Value::new_string(world_cell_key(cell_x, cell_y))),
                    ("x", Value::Int(cell_x)),
                    ("y", Value::Int(cell_y)),
                    ("population", Value::Int(population as i64)),
                    ("layers", Value::Int(layers as i64)),
                    (
                        "overloaded",
                        Value::Bool(population > self.config.cell_capacity),
                    ),
                ])
            })
            .collect()
    }

    fn cell_count(&self) -> usize {
        let mut cells = Vec::new();
        for entity in self.entities.values() {
            let key = (entity.cell_x, entity.cell_y);
            if !cells.contains(&key) {
                cells.push(key);
            }
        }
        cells.len()
    }
}

impl Default for WorldBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for WorldBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "configure" => self.configure(args),
            "reset" => {
                if !args.is_empty() {
                    return Err(format!(
                        "world.reset expects 0 arguments, got {}",
                        args.len()
                    ));
                }
                self.entities.clear();
                self.version = 0;
                Ok(self.status_payload())
            }
            "spawn" => self.place_entity(args, true),
            "place" | "upsert" => self.place_entity(args, false),
            "move" => self.move_entity(args),
            "nearby" | "interest" => self.nearby_payload(args),
            "plan" => {
                if !args.is_empty() {
                    return Err(format!(
                        "world.plan expects 0 arguments, got {}",
                        args.len()
                    ));
                }
                Ok(self.plan_payload())
            }
            "status" => {
                if !args.is_empty() {
                    return Err(format!(
                        "world.status expects 0 arguments, got {}",
                        args.len()
                    ));
                }
                Ok(self.status_payload())
            }
            _ => Err(format!("Unknown world method: {}", method)),
        }
    }
}

fn world_map(entries: Vec<(&str, Value)>) -> Value {
    let mut map = HashMap::new();
    for (key, value) in entries {
        map.insert(key.to_string(), value);
    }
    Value::new_map(map)
}

fn world_cell_key(x: i64, y: i64) -> String {
    format!("{},{}", x, y)
}

fn world_error_payload(message: &str) -> Value {
    world_map(vec![("error", Value::new_string(message.to_string()))])
}

// ── Result Backend ──

pub struct ResultBackend;

impl ResultBackend {
    pub fn new() -> Self { Self }
}

impl Default for ResultBackend {
    fn default() -> Self { Self::new() }
}

impl Backend for ResultBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "ok" => {
                if args.len() != 1 {
                    return Err(format!("result.ok expects 1 argument, got {}", args.len()));
                }
                let mut map = HashMap::new();
                map.insert("tag".to_string(), Value::new_string("Ok".to_string()));
                map.insert("value".to_string(), args[0].clone());
                Ok(Value::new_map(map))
            }
            "err" => {
                if args.len() != 1 {
                    return Err(format!("result.err expects 1 argument, got {}", args.len()));
                }
                let mut map = HashMap::new();
                map.insert("tag".to_string(), Value::new_string("Err".to_string()));
                map.insert("error".to_string(), args[0].clone());
                Ok(Value::new_map(map))
            }
            "is_ok" => {
                if args.len() != 1 {
                    return Err(format!("result.is_ok expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    Ok(Value::Bool(m.get("tag").map_or(false, |v| v.to_display_string() == "Ok")))
                } else {
                    Ok(Value::Bool(false))
                }
            }
            "is_err" => {
                if args.len() != 1 {
                    return Err(format!("result.is_err expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    Ok(Value::Bool(m.get("tag").map_or(false, |v| v.to_display_string() == "Err")))
                } else {
                    Ok(Value::Bool(false))
                }
            }
            "unwrap" => {
                if args.len() != 1 {
                    return Err(format!("result.unwrap expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    match m.get("tag").map(|v| v.to_display_string()).as_deref() {
                        Some("Ok") => Ok(m.get("value").cloned().unwrap_or(Value::Null)),
                        Some("Err") => {
                            let err = m.get("error").map_or("unknown error".to_string(), |v| v.to_display_string());
                            Err(format!("result.unwrap: called on Err({})", err))
                        }
                        _ => Err("result.unwrap: not a Result".to_string()),
                    }
                } else {
                    Err("result.unwrap: not a Result".to_string())
                }
            }
            "unwrap_or" => {
                if args.len() != 2 {
                    return Err(format!("result.unwrap_or expects 2 arguments, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    match m.get("tag").map(|v| v.to_display_string()).as_deref() {
                        Some("Ok") => Ok(m.get("value").cloned().unwrap_or(Value::Null)),
                        _ => Ok(args[1].clone()),
                    }
                } else {
                    Ok(args[1].clone())
                }
            }
            "try_unwrap" => {
                if args.len() != 1 {
                    return Err(format!("result.try_unwrap expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    match m.get("tag").map(|v| v.to_display_string()).as_deref() {
                        Some("Ok") => Ok(m.get("value").cloned().unwrap_or(Value::Null)),
                        Some("Err") => {
                            let err = m.get("error").cloned().unwrap_or(Value::Null);
                            let mut err_map = HashMap::new();
                            err_map.insert("tag".to_string(), Value::new_string("Err".to_string()));
                            err_map.insert("error".to_string(), err);
                            Ok(Value::new_map(err_map))
                        }
                        _ => Ok(args[0].clone()),
                    }
                } else {
                    Ok(args[0].clone())
                }
            }
            "map" => {
                if args.len() != 2 {
                    return Err(format!("result.map expects 2 arguments (result, fn), got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    match m.get("tag").map(|v| v.to_display_string()).as_deref() {
                        Some("Ok") => {
                            let val = m.get("value").cloned().unwrap_or(Value::Null);
                            // Apply the function
                            match &args[1] {
                                Value::Function(_name) => {
                                    let mut map_result = HashMap::new();
                                    map_result.insert("tag".to_string(), Value::new_string("Ok".to_string()));
                                    map_result.insert("value".to_string(), val);
                                    // Caller should handle the actual function call
                                    Ok(Value::new_map(map_result))
                                }
                                _ => {
                                    let mut map_result = HashMap::new();
                                    map_result.insert("tag".to_string(), Value::new_string("Ok".to_string()));
                                    map_result.insert("value".to_string(), val);
                                    Ok(Value::new_map(map_result))
                                }
                            }
                        }
                        _ => Ok(args[0].clone()),
                    }
                } else {
                    Ok(args[0].clone())
                }
            }
            _ => Err(format!("Unknown result method: {}", method)),
        }
    }
}

// ── Option Backend ──

pub struct OptionBackend;

impl OptionBackend {
    pub fn new() -> Self { Self }
}

impl Default for OptionBackend {
    fn default() -> Self { Self::new() }
}

impl Backend for OptionBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "some" => {
                if args.len() != 1 {
                    return Err(format!("option.some expects 1 argument, got {}", args.len()));
                }
                let mut map = HashMap::new();
                map.insert("tag".to_string(), Value::new_string("Some".to_string()));
                map.insert("value".to_string(), args[0].clone());
                Ok(Value::new_map(map))
            }
            "none" => {
                let mut map = HashMap::new();
                map.insert("tag".to_string(), Value::new_string("None".to_string()));
                Ok(Value::new_map(map))
            }
            "is_some" => {
                if args.len() != 1 {
                    return Err(format!("option.is_some expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    Ok(Value::Bool(m.get("tag").map_or(false, |v| v.to_display_string() == "Some")))
                } else {
                    Ok(Value::Bool(false))
                }
            }
            "is_none" => {
                if args.len() != 1 {
                    return Err(format!("option.is_none expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    Ok(Value::Bool(m.get("tag").map_or(false, |v| v.to_display_string() == "None")))
                } else {
                    Ok(Value::Bool(true))
                }
            }
            "unwrap" => {
                if args.len() != 1 {
                    return Err(format!("option.unwrap expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    match m.get("tag").map(|v| v.to_display_string()).as_deref() {
                        Some("Some") => Ok(m.get("value").cloned().unwrap_or(Value::Null)),
                        _ => Err("option.unwrap: called on None".to_string()),
                    }
                } else {
                    Err("option.unwrap: not an Option".to_string())
                }
            }
            "unwrap_or" => {
                if args.len() != 2 {
                    return Err(format!("option.unwrap_or expects 2 arguments, got {}", args.len()));
                }
                if let Value::Map(m) = &args[0] {
                    match m.get("tag").map(|v| v.to_display_string()).as_deref() {
                        Some("Some") => Ok(m.get("value").cloned().unwrap_or(Value::Null)),
                        _ => Ok(args[1].clone()),
                    }
                } else {
                    Ok(args[1].clone())
                }
            }
            _ => Err(format!("Unknown option method: {}", method)),
        }
    }
}
