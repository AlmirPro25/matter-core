//! Vec Backend - Sprint 80
//! Dynamic array with O(1) push/pop, capacity management

use matter_backend::{Backend, Value};

use matter_memory::rc::Rc;

pub struct VecBackend;

impl VecBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VecBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for VecBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "new" => {
                // Vec.new() → empty Vec
                Ok(Value::new_list(Vec::new()))
            }

            "with_capacity" => {
                // Vec.with_capacity(10) → Vec with reserved capacity
                if args.len() != 1 {
                    return Err(format!(
                        "Vec.with_capacity expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let capacity = args[0]
                    .as_int()
                    .map_err(|e| format!("Vec.with_capacity: {}", e))?;
                let vec = Vec::with_capacity(capacity as usize);
                Ok(Value::new_list(vec))
            }

            "push" => {
                // Vec.push(vec, value) → new vec with value appended
                if args.len() != 2 {
                    return Err(format!("Vec.push expects 2 arguments, got {}", args.len()));
                }
                if let Value::List(items) = args[0].clone() {
                    let mut new_items = (*items).to_vec();
                    new_items.push(args[1].clone());
                    Ok(Value::new_list(new_items))
                } else {
                    Err("Vec.push: first argument must be a Vec".to_string())
                }
            }

            "pop" => {
                // Vec.pop(vec) → {vec: new_vec, value: popped_value}
                if args.len() != 1 {
                    return Err(format!("Vec.pop expects 1 argument, got {}", args.len()));
                }
                if let Value::List(items) = args[0].clone() {
                    let mut new_items = (*items).to_vec();
                    if let Some(value) = new_items.pop() {
                        let mut map = std::collections::HashMap::new();
                        map.insert("vec".to_string(), Value::List(Rc::new(new_items)));
                        map.insert("value".to_string(), value);
                        Ok(Value::new_map(map))
                    } else {
                        Err("Vec.pop: empty Vec".to_string())
                    }
                } else {
                    Err("Vec.pop: argument must be a Vec".to_string())
                }
            }

            "get" => {
                // Vec.get(vec, index) → value
                if args.len() != 2 {
                    return Err(format!("Vec.get expects 2 arguments, got {}", args.len()));
                }
                if let Value::List(items) = &args[0] {
                    let idx = args[1].as_int().map_err(|e| format!("Vec.get: {}", e))?;
                    if idx < 0 || idx >= items.len() as i64 {
                        return Err(format!(
                            "Vec.get: index {} out of bounds (len={})",
                            idx,
                            items.len()
                        ));
                    }
                    Ok(items[idx as usize].clone())
                } else {
                    Err("Vec.get: first argument must be a Vec".to_string())
                }
            }

            "set" => {
                // Vec.set(vec, index, value) → new vec with updated value
                if args.len() != 3 {
                    return Err(format!("Vec.set expects 3 arguments, got {}", args.len()));
                }
                if let Value::List(items) = args[0].clone() {
                    let idx = args[1].as_int().map_err(|e| format!("Vec.set: {}", e))?;
                    let mut new_items = (*items).to_vec();
                    if idx < 0 || idx >= new_items.len() as i64 {
                        return Err(format!(
                            "Vec.set: index {} out of bounds (len={})",
                            idx,
                            new_items.len()
                        ));
                    }
                    new_items[idx as usize] = args[2].clone();
                    Ok(Value::new_list(new_items))
                } else {
                    Err("Vec.set: first argument must be a Vec".to_string())
                }
            }

            "len" => {
                // Vec.len(vec) → Int
                if args.len() != 1 {
                    return Err(format!("Vec.len expects 1 argument, got {}", args.len()));
                }
                if let Value::List(items) = &args[0] {
                    Ok(Value::Int(items.len() as i64))
                } else {
                    Err("Vec.len: argument must be a Vec".to_string())
                }
            }

            "is_empty" => {
                // Vec.is_empty(vec) → Bool
                if args.len() != 1 {
                    return Err(format!(
                        "Vec.is_empty expects 1 argument, got {}",
                        args.len()
                    ));
                }
                if let Value::List(items) = &args[0] {
                    Ok(Value::Bool(items.is_empty()))
                } else {
                    Err("Vec.is_empty: argument must be a Vec".to_string())
                }
            }

            "clear" => {
                // Vec.clear(vec) → empty vec
                if args.len() != 1 {
                    return Err(format!("Vec.clear expects 1 argument, got {}", args.len()));
                }
                Ok(Value::new_list(Vec::new()))
            }

            "contains" => {
                // Vec.contains(vec, value) → Bool
                if args.len() != 2 {
                    return Err(format!(
                        "Vec.contains expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                if let Value::List(items) = &args[0] {
                    Ok(Value::Bool(items.contains(&args[1])))
                } else {
                    Err("Vec.contains: first argument must be a Vec".to_string())
                }
            }

            "index_of" => {
                // Vec.index_of(vec, value) → Int (or -1 if not found)
                if args.len() != 2 {
                    return Err(format!(
                        "Vec.index_of expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                if let Value::List(items) = &args[0] {
                    for (i, item) in items.iter().enumerate() {
                        if item == &args[1] {
                            return Ok(Value::Int(i as i64));
                        }
                    }
                    Ok(Value::Int(-1))
                } else {
                    Err("Vec.index_of: first argument must be a Vec".to_string())
                }
            }

            "insert" => {
                // Vec.insert(vec, index, value) → new vec with value inserted
                if args.len() != 3 {
                    return Err(format!(
                        "Vec.insert expects 3 arguments, got {}",
                        args.len()
                    ));
                }
                if let Value::List(items) = args[0].clone() {
                    let idx = args[1].as_int().map_err(|e| format!("Vec.insert: {}", e))?;
                    let mut new_items = (*items).to_vec();
                    if idx < 0 || idx > new_items.len() as i64 {
                        return Err(format!(
                            "Vec.insert: index {} out of bounds (len={})",
                            idx,
                            new_items.len()
                        ));
                    }
                    new_items.insert(idx as usize, args[2].clone());
                    Ok(Value::new_list(new_items))
                } else {
                    Err("Vec.insert: first argument must be a Vec".to_string())
                }
            }

            "remove" => {
                // Vec.remove(vec, index) → {vec: new_vec, value: removed_value}
                if args.len() != 2 {
                    return Err(format!(
                        "Vec.remove expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                if let Value::List(items) = args[0].clone() {
                    let idx = args[1].as_int().map_err(|e| format!("Vec.remove: {}", e))?;
                    let mut new_items = (*items).to_vec();
                    if idx < 0 || idx >= new_items.len() as i64 {
                        return Err(format!(
                            "Vec.remove: index {} out of bounds (len={})",
                            idx,
                            new_items.len()
                        ));
                    }
                    let value = new_items.remove(idx as usize);
                    let mut map = std::collections::HashMap::new();
                    map.insert("vec".to_string(), Value::List(Rc::new(new_items)));
                    map.insert("value".to_string(), value);
                    Ok(Value::new_map(map))
                } else {
                    Err("Vec.remove: first argument must be a Vec".to_string())
                }
            }

            "extend" => {
                // Vec.extend(vec1, vec2) → concatenated vec
                if args.len() != 2 {
                    return Err(format!(
                        "Vec.extend expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                if let (Value::List(a), Value::List(b)) = (&args[0], &args[1]) {
                    let mut new_items = (*a).to_vec();
                    new_items.extend((*b).to_vec());
                    Ok(Value::new_list(new_items))
                } else {
                    Err("Vec.extend: both arguments must be Vecs".to_string())
                }
            }

            "slice" => {
                // Vec.slice(vec, start, end) → new vec with elements [start..end)
                if args.len() != 3 {
                    return Err(format!("Vec.slice expects 3 arguments, got {}", args.len()));
                }
                if let Value::List(items) = &args[0] {
                    let start = args[1].as_int().map_err(|e| format!("Vec.slice: {}", e))? as usize;
                    let end = args[2].as_int().map_err(|e| format!("Vec.slice: {}", e))? as usize;
                    if start > items.len() || end > items.len() || start > end {
                        return Err(format!(
                            "Vec.slice: index out of bounds (start={}, end={}, len={})",
                            start,
                            end,
                            items.len()
                        ));
                    }
                    Ok(Value::new_list(items[start..end].to_vec()))
                } else {
                    Err("Vec.slice: first argument must be a Vec".to_string())
                }
            }

            "reverse" => {
                // Vec.reverse(vec) → reversed vec
                if args.len() != 1 {
                    return Err(format!(
                        "Vec.reverse expects 1 argument, got {}",
                        args.len()
                    ));
                }
                if let Value::List(items) = args[0].clone() {
                    let mut new_items = (*items).to_vec();
                    new_items.reverse();
                    Ok(Value::new_list(new_items))
                } else {
                    Err("Vec.reverse: argument must be a Vec".to_string())
                }
            }

            "sort" => {
                // Vec.sort(vec) → sorted vec (integers only for now)
                if args.len() != 1 {
                    return Err(format!("Vec.sort expects 1 argument, got {}", args.len()));
                }
                if let Value::List(items) = args[0].clone() {
                    let mut int_items: Vec<i64> = Vec::new();
                    for item in items.iter() {
                        int_items.push(item.as_int().map_err(|e| format!("Vec.sort: {}", e))?);
                    }
                    int_items.sort();
                    let sorted: Vec<Value> = int_items.into_iter().map(Value::Int).collect();
                    Ok(Value::new_list(sorted))
                } else {
                    Err("Vec.sort: argument must be a Vec".to_string())
                }
            }

            "filter" => {
                // Vec.filter(vec, predicate_fn_name) → filtered vec
                // Note: This would need function reference support
                Err("Vec.filter: not yet implemented (needs function references)".to_string())
            }

            "map" => {
                // Vec.map(vec, transform_fn_name) → transformed vec
                // Note: This would need function reference support
                Err("Vec.map: not yet implemented (needs function references)".to_string())
            }

            "first" => {
                // Vec.first(vec) → first element or error if empty
                if args.len() != 1 {
                    return Err(format!("Vec.first expects 1 argument, got {}", args.len()));
                }
                if let Value::List(items) = &args[0] {
                    items
                        .first()
                        .cloned()
                        .ok_or_else(|| "Vec.first: empty Vec".to_string())
                } else {
                    Err("Vec.first: argument must be a Vec".to_string())
                }
            }

            "last" => {
                // Vec.last(vec) → last element or error if empty
                if args.len() != 1 {
                    return Err(format!("Vec.last expects 1 argument, got {}", args.len()));
                }
                if let Value::List(items) = &args[0] {
                    items
                        .last()
                        .cloned()
                        .ok_or_else(|| "Vec.last: empty Vec".to_string())
                } else {
                    Err("Vec.last: argument must be a Vec".to_string())
                }
            }

            _ => Err(format!("Unknown Vec method: {}", method)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_push() {
        let mut backend = VecBackend::new();

        // Create new Vec
        let vec = backend.call("new", vec![]).unwrap();

        // Push elements
        let vec = backend
            .call("push", vec![vec.clone(), Value::Int(10)])
            .unwrap();
        let vec = backend
            .call("push", vec![vec.clone(), Value::Int(20)])
            .unwrap();
        let vec = backend
            .call("push", vec![vec.clone(), Value::Int(30)])
            .unwrap();

        // Check length
        let len = backend.call("len", vec![vec.clone()]).unwrap();
        assert_eq!(len, Value::Int(3));

        // Get elements
        let val = backend
            .call("get", vec![vec.clone(), Value::Int(0)])
            .unwrap();
        assert_eq!(val, Value::Int(10));

        let val = backend
            .call("get", vec![vec.clone(), Value::Int(2)])
            .unwrap();
        assert_eq!(val, Value::Int(30));
    }

    #[test]
    fn test_pop() {
        let mut backend = VecBackend::new();

        let vec = Value::new_list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        let result = backend.call("pop", vec![vec]).unwrap();

        if let Value::Map(map) = result {
            let popped = map.get("value").unwrap();
            assert_eq!(*popped, Value::Int(3));

            let new_vec = map.get("vec").unwrap();
            if let Value::List(items) = new_vec {
                assert_eq!(items.len(), 2);
            }
        } else {
            panic!("Expected map");
        }
    }

    #[test]
    fn test_contains_and_index_of() {
        let mut backend = VecBackend::new();

        let vec = Value::new_list(vec![Value::Int(10), Value::Int(20), Value::Int(30)]);

        // Contains
        let result = backend
            .call("contains", vec![vec.clone(), Value::Int(20)])
            .unwrap();
        assert_eq!(result, Value::Bool(true));

        let result = backend
            .call("contains", vec![vec.clone(), Value::Int(99)])
            .unwrap();
        assert_eq!(result, Value::Bool(false));

        // Index of
        let result = backend
            .call("index_of", vec![vec.clone(), Value::Int(30)])
            .unwrap();
        assert_eq!(result, Value::Int(2));

        let result = backend
            .call("index_of", vec![vec.clone(), Value::Int(99)])
            .unwrap();
        assert_eq!(result, Value::Int(-1));
    }

    #[test]
    fn test_insert_and_remove() {
        let mut backend = VecBackend::new();

        let vec = Value::new_list(vec![Value::Int(10), Value::Int(30)]);

        // Insert at index 1
        let vec = backend
            .call("insert", vec![vec, Value::Int(1), Value::Int(20)])
            .unwrap();

        // Should be [10, 20, 30]
        let val = backend
            .call("get", vec![vec.clone(), Value::Int(1)])
            .unwrap();
        assert_eq!(val, Value::Int(20));

        // Remove at index 1
        let result = backend.call("remove", vec![vec, Value::Int(1)]).unwrap();

        if let Value::Map(map) = result {
            let removed = map.get("value").unwrap();
            assert_eq!(*removed, Value::Int(20));

            // Should be [10, 30] again
            let new_vec = map.get("vec").unwrap();
            let len = backend.call("len", vec![new_vec.clone()]).unwrap();
            assert_eq!(len, Value::Int(2));
        }
    }

    #[test]
    fn test_slice() {
        let mut backend = VecBackend::new();

        let vec = Value::new_list(vec![
            Value::Int(0),
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]);

        // Slice [1..4)
        let result = backend
            .call("slice", vec![vec, Value::Int(1), Value::Int(4)])
            .unwrap();

        if let Value::List(items) = result {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::Int(1));
            assert_eq!(items[2], Value::Int(3));
        }
    }

    #[test]
    fn test_reverse_and_sort() {
        let mut backend = VecBackend::new();

        let vec = Value::new_list(vec![Value::Int(3), Value::Int(1), Value::Int(2)]);

        // Reverse
        let rev = backend.call("reverse", vec![vec.clone()]).unwrap();
        if let Value::List(items) = rev {
            assert_eq!(items[0], Value::Int(2));
            assert_eq!(items[2], Value::Int(3));
        }

        // Sort
        let sorted = backend.call("sort", vec![vec]).unwrap();
        if let Value::List(items) = sorted {
            assert_eq!(items[0], Value::Int(1));
            assert_eq!(items[1], Value::Int(2));
            assert_eq!(items[2], Value::Int(3));
        }
    }
}
