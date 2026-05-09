/// Matter Standard Library
/// Biblioteca padrão com módulos math, string, list, etc

use matter_backend::{Backend, Value};

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
                    return Err("math.pow: negative exponent not supported for integers".to_string());
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
                let s = args[0].as_string().map_err(|e| format!("string.len: {}", e))?;
                Ok(Value::Int(s.len() as i64))
            }
            
            "upper" => {
                if args.len() != 1 {
                    return Err(format!("string.upper expects 1 argument, got {}", args.len()));
                }
                let s = args[0].as_string().map_err(|e| format!("string.upper: {}", e))?;
                Ok(Value::String(s.to_uppercase()))
            }
            
            "lower" => {
                if args.len() != 1 {
                    return Err(format!("string.lower expects 1 argument, got {}", args.len()));
                }
                let s = args[0].as_string().map_err(|e| format!("string.lower: {}", e))?;
                Ok(Value::String(s.to_lowercase()))
            }
            
            "trim" => {
                if args.len() != 1 {
                    return Err(format!("string.trim expects 1 argument, got {}", args.len()));
                }
                let s = args[0].as_string().map_err(|e| format!("string.trim: {}", e))?;
                Ok(Value::String(s.trim().to_string()))
            }
            
            "split" => {
                if args.len() != 2 {
                    return Err(format!("string.split expects 2 arguments, got {}", args.len()));
                }
                let s = args[0].as_string().map_err(|e| format!("string.split: {}", e))?;
                let sep = args[1].as_string().map_err(|e| format!("string.split: {}", e))?;
                
                let parts: Vec<Value> = s
                    .split(&sep)
                    .map(|part| Value::String(part.to_string()))
                    .collect();
                
                Ok(Value::List(parts))
            }
            
            "join" => {
                if args.len() != 2 {
                    return Err(format!("string.join expects 2 arguments, got {}", args.len()));
                }
                let sep = args[0].as_string().map_err(|e| format!("string.join: {}", e))?;
                
                if let Value::List(items) = &args[1] {
                    let strings: Result<Vec<String>, String> = items
                        .iter()
                        .map(|v| v.as_string())
                        .collect();
                    
                    let strings = strings.map_err(|e| format!("string.join: {}", e))?;
                    Ok(Value::String(strings.join(&sep)))
                } else {
                    Err("string.join: second argument must be a list".to_string())
                }
            }
            
            "contains" => {
                if args.len() != 2 {
                    return Err(format!("string.contains expects 2 arguments, got {}", args.len()));
                }
                let s = args[0].as_string().map_err(|e| format!("string.contains: {}", e))?;
                let needle = args[1].as_string().map_err(|e| format!("string.contains: {}", e))?;
                Ok(Value::Bool(s.contains(&needle)))
            }
            
            "replace" => {
                if args.len() != 3 {
                    return Err(format!("string.replace expects 3 arguments, got {}", args.len()));
                }
                let s = args[0].as_string().map_err(|e| format!("string.replace: {}", e))?;
                let from = args[1].as_string().map_err(|e| format!("string.replace: {}", e))?;
                let to = args[2].as_string().map_err(|e| format!("string.replace: {}", e))?;
                Ok(Value::String(s.replace(&from, &to)))
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
                    for item in &items {
                        int_items.push(item.as_int().map_err(|e| format!("list.sort: {}", e))?);
                    }
                    int_items.sort();
                    
                    let sorted: Vec<Value> = int_items.into_iter().map(Value::Int).collect();
                    Ok(Value::List(sorted))
                } else {
                    Err("list.sort: argument must be a list".to_string())
                }
            }
            
            "reverse" => {
                if args.len() != 1 {
                    return Err(format!("list.reverse expects 1 argument, got {}", args.len()));
                }
                
                if let Value::List(mut items) = args[0].clone() {
                    items.reverse();
                    Ok(Value::List(items))
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
                    for item in items {
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
        
        let min = math.call("min", vec![Value::Int(10), Value::Int(20)]).unwrap();
        assert_eq!(min, Value::Int(10));
        
        let max = math.call("max", vec![Value::Int(10), Value::Int(20)]).unwrap();
        assert_eq!(max, Value::Int(20));
    }

    #[test]
    fn test_math_pow() {
        let mut math = MathBackend::new();
        let result = math.call("pow", vec![Value::Int(2), Value::Int(3)]).unwrap();
        assert_eq!(result, Value::Int(8));
    }

    #[test]
    fn test_string_upper_lower() {
        let mut string = StringBackend::new();
        
        let upper = string.call("upper", vec![Value::String("hello".to_string())]).unwrap();
        assert_eq!(upper, Value::String("HELLO".to_string()));
        
        let lower = string.call("lower", vec![Value::String("WORLD".to_string())]).unwrap();
        assert_eq!(lower, Value::String("world".to_string()));
    }

    #[test]
    fn test_string_split_join() {
        let mut string = StringBackend::new();
        
        let split = string.call("split", vec![
            Value::String("a,b,c".to_string()),
            Value::String(",".to_string())
        ]).unwrap();
        
        if let Value::List(parts) = split {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], Value::String("a".to_string()));
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_list_sort() {
        let mut list = ListBackend::new();
        let result = list.call("sort", vec![
            Value::List(vec![Value::Int(3), Value::Int(1), Value::Int(2)])
        ]).unwrap();
        
        if let Value::List(sorted) = result {
            assert_eq!(sorted, vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_list_sum() {
        let mut list = ListBackend::new();
        let result = list.call("sum", vec![
            Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        ]).unwrap();
        
        assert_eq!(result, Value::Int(6));
    }
}
