//! Matter Standard Library
//! Biblioteca padrão com módulos math, string, list, time, random, json, etc

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

            _ => Err(format!("Unknown audio method: {}", method)),
        }
    }
}
