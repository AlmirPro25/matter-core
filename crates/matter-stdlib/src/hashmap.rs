//! HashMap Backend - Sprint 80
//! Key-value store with O(1) average lookup, insert, delete

use matter_backend::{Backend, Value};
use matter_memory::rc::Rc;
use std::collections::HashMap as StdHashMap;

pub struct HashMapBackend;

impl HashMapBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HashMapBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for HashMapBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "new" => {
                // HashMap.new() → empty map
                Ok(Value::new_map(StdHashMap::new()))
            }
            
            "insert" => {
                // HashMap.insert(map, key, value) → new map with key-value inserted
                if args.len() != 3 {
                    return Err(format!("HashMap.insert expects 3 arguments, got {}", args.len()));
                }
                if let Value::Map(map) = args[0].clone() {
                    let key = args[1].as_string().map_err(|e| format!("HashMap.insert: key must be string: {}", e))?;
                    let mut new_map = (*map).clone();
                    new_map.insert(key, args[2].clone());
                    Ok(Value::new_map(new_map))
                } else {
                    Err("HashMap.insert: first argument must be a HashMap".to_string())
                }
            }
            
            "get" => {
                // HashMap.get(map, key) → value or error if key doesn't exist
                if args.len() != 2 {
                    return Err(format!("HashMap.get expects 2 arguments, got {}", args.len()));
                }
                if let Value::Map(map) = &args[0] {
                    let key = args[1].as_string().map_err(|e| format!("HashMap.get: key must be string: {}", e))?;
                    map.get(&key)
                        .cloned()
                        .ok_or_else(|| format!("HashMap.get: key '{}' not found", key))
                } else {
                    Err("HashMap.get: first argument must be a HashMap".to_string())
                }
            }
            
            "get_or_default" => {
                // HashMap.get_or_default(map, key, default) → value or default
                if args.len() != 3 {
                    return Err(format!("HashMap.get_or_default expects 3 arguments, got {}", args.len()));
                }
                if let Value::Map(map) = &args[0] {
                    let key = args[1].as_string().map_err(|e| format!("HashMap.get_or_default: {}", e))?;
                    Ok(map.get(&key).cloned().unwrap_or_else(|| args[2].clone()))
                } else {
                    Err("HashMap.get_or_default: first argument must be a HashMap".to_string())
                }
            }
            
            "contains_key" => {
                // HashMap.contains_key(map, key) → Bool
                if args.len() != 2 {
                    return Err(format!("HashMap.contains_key expects 2 arguments, got {}", args.len()));
                }
                if let Value::Map(map) = &args[0] {
                    let key = args[1].as_string().map_err(|e| format!("HashMap.contains_key: {}", e))?;
                    Ok(Value::Bool(map.contains_key(&key)))
                } else {
                    Err("HashMap.contains_key: first argument must be a HashMap".to_string())
                }
            }
            
            "remove" => {
                // HashMap.remove(map, key) → {map: new_map, value: removed_value}
                if args.len() != 2 {
                    return Err(format!("HashMap.remove expects 2 arguments, got {}", args.len()));
                }
                if let Value::Map(map) = args[0].clone() {
                    let key = args[1].as_string().map_err(|e| format!("HashMap.remove: {}", e))?;
                    let mut new_map = (*map).clone();
                    if let Some(value) = new_map.remove(&key) {
                        let mut result_map = StdHashMap::new();
                        result_map.insert("map".to_string(), Value::Map(Rc::new(new_map)));
                        result_map.insert("value".to_string(), value);
                        Ok(Value::Map(Rc::new(result_map)))
                    } else {
                        Err(format!("HashMap.remove: key '{}' not found", key))
                    }
                } else {
                    Err("HashMap.remove: first argument must be a HashMap".to_string())
                }
            }
            
            "keys" => {
                // HashMap.keys(map) → List<String>
                if args.len() != 1 {
                    return Err(format!("HashMap.keys expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(map) = &args[0] {
                    let keys: Vec<Value> = map.keys()
                        .map(|k| Value::new_string(k.clone()))
                        .collect();
                    Ok(Value::new_list(keys))
                } else {
                    Err("HashMap.keys: argument must be a HashMap".to_string())
                }
            }
            
            "values" => {
                // HashMap.values(map) → List<Value>
                if args.len() != 1 {
                    return Err(format!("HashMap.values expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(map) = &args[0] {
                    let values: Vec<Value> = map.values().cloned().collect();
                    Ok(Value::new_list(values))
                } else {
                    Err("HashMap.values: argument must be a HashMap".to_string())
                }
            }
            
            "len" => {
                // HashMap.len(map) → Int
                if args.len() != 1 {
                    return Err(format!("HashMap.len expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(map) = &args[0] {
                    Ok(Value::Int(map.len() as i64))
                } else {
                    Err("HashMap.len: argument must be a HashMap".to_string())
                }
            }
            
            "is_empty" => {
                // HashMap.is_empty(map) → Bool
                if args.len() != 1 {
                    return Err(format!("HashMap.is_empty expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(map) = &args[0] {
                    Ok(Value::Bool(map.is_empty()))
                } else {
                    Err("HashMap.is_empty: argument must be a HashMap".to_string())
                }
            }
            
            "clear" => {
                // HashMap.clear(map) → empty map
                if args.len() != 1 {
                    return Err(format!("HashMap.clear expects 1 argument, got {}", args.len()));
                }
                Ok(Value::new_map(StdHashMap::new()))
            }
            
            "merge" => {
                // HashMap.merge(map1, map2) → merged map (map2 overwrites map1 on conflicts)
                if args.len() != 2 {
                    return Err(format!("HashMap.merge expects 2 arguments, got {}", args.len()));
                }
                if let (Value::Map(map1), Value::Map(map2)) = (&args[0], &args[1]) {
                    let mut result = (**map1).clone();  // Clone the HashMap inside the Rc
                    for (k, v) in map2.iter() {
                        result.insert(k.clone(), v.clone());
                    }
                    Ok(Value::new_map(result))
                } else {
                    Err("HashMap.merge: both arguments must be HashMaps".to_string())
                }
            }
            
            "from_pairs" => {
                // HashMap.from_pairs([["key1", val1], ["key2", val2]]) → HashMap
                if args.len() != 1 {
                    return Err(format!("HashMap.from_pairs expects 1 argument, got {}", args.len()));
                }
                if let Value::List(pairs) = &args[0] {
                    let mut map = StdHashMap::new();
                    for pair in pairs.iter() {
                        if let Value::List(kv) = pair {
                            if kv.len() != 2 {
                                return Err("HashMap.from_pairs: each pair must have exactly 2 elements".to_string());
                            }
                            let key = kv[0].as_string().map_err(|e| format!("HashMap.from_pairs: {}", e))?;
                            map.insert(key, kv[1].clone());
                        } else {
                            return Err("HashMap.from_pairs: each element must be a pair (list of 2)".to_string());
                        }
                    }
                    Ok(Value::new_map(map))
                } else {
                    Err("HashMap.from_pairs: argument must be a list".to_string())
                }
            }
            
            "to_pairs" => {
                // HashMap.to_pairs(map) → [["key1", val1], ["key2", val2], ...]
                if args.len() != 1 {
                    return Err(format!("HashMap.to_pairs expects 1 argument, got {}", args.len()));
                }
                if let Value::Map(map) = &args[0] {
                    let pairs: Vec<Value> = map.iter()
                        .map(|(k, v)| {
                            Value::new_list(vec![
                                Value::new_string(k.clone()),
                                v.clone(),
                            ])
                        })
                        .collect();
                    Ok(Value::new_list(pairs))
                } else {
                    Err("HashMap.to_pairs: argument must be a HashMap".to_string())
                }
            }
            
            "filter" => {
                // HashMap.filter(map, predicate_fn) → filtered map
                // Note: Needs function references
                Err("HashMap.filter: not yet implemented (needs function references)".to_string())
            }
            
            "map_values" => {
                // HashMap.map_values(map, transform_fn) → map with transformed values
                // Note: Needs function references
                Err("HashMap.map_values: not yet implemented (needs function references)".to_string())
            }
            
            _ => Err(format!("Unknown HashMap method: {}", method)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_and_insert() {
        let mut backend = HashMapBackend::new();
        
        // Create new HashMap
        let map = backend.call("new", vec![]).unwrap();
        
        // Insert key-value pairs
        let map = backend.call("insert", vec![
            map.clone(),
            Value::new_string("name".to_string()),
            Value::new_string("Matter".to_string()),
        ]).unwrap();
        
        let map = backend.call("insert", vec![
            map.clone(),
            Value::new_string("version".to_string()),
            Value::Int(1),
        ]).unwrap();
        
        // Check length
        let len = backend.call("len", vec![map.clone()]).unwrap();
        assert_eq!(len, Value::Int(2));
        
        // Get values
        let name = backend.call("get", vec![
            map.clone(),
            Value::new_string("name".to_string()),
        ]).unwrap();
        assert_eq!(name.as_string().unwrap(), "Matter");
        
        let version = backend.call("get", vec![
            map.clone(),
            Value::new_string("version".to_string()),
        ]).unwrap();
        assert_eq!(version, Value::Int(1));
    }
    
    #[test]
    fn test_contains_key() {
        let mut backend = HashMapBackend::new();
        
        let mut map_data = StdHashMap::new();
        map_data.insert("key1".to_string(), Value::Int(100));
        let map = Value::new_map(map_data);
        
        // Contains
        let result = backend.call("contains_key", vec![
            map.clone(),
            Value::new_string("key1".to_string()),
        ]).unwrap();
        assert_eq!(result, Value::Bool(true));
        
        let result = backend.call("contains_key", vec![
            map.clone(),
            Value::new_string("key2".to_string()),
        ]).unwrap();
        assert_eq!(result, Value::Bool(false));
    }
    
    #[test]
    fn test_remove() {
        let mut backend = HashMapBackend::new();
        
        let mut map_data = StdHashMap::new();
        map_data.insert("a".to_string(), Value::Int(1));
        map_data.insert("b".to_string(), Value::Int(2));
        let map = Value::new_map(map_data);
        
        // Remove key "a"
        let result = backend.call("remove", vec![
            map,
            Value::new_string("a".to_string()),
        ]).unwrap();
        
        if let Value::Map(result_map) = result {
            let removed = result_map.get("value").unwrap();
            assert_eq!(*removed, Value::Int(1));
            
            let new_map = result_map.get("map").unwrap();
            let len = backend.call("len", vec![new_map.clone()]).unwrap();
            assert_eq!(len, Value::Int(1));
        } else {
            panic!("Expected map");
        }
    }
    
    #[test]
    fn test_keys_and_values() {
        let mut backend = HashMapBackend::new();
        
        let mut map_data = StdHashMap::new();
        map_data.insert("x".to_string(), Value::Int(10));
        map_data.insert("y".to_string(), Value::Int(20));
        let map = Value::new_map(map_data);
        
        // Keys
        let keys = backend.call("keys", vec![map.clone()]).unwrap();
        if let Value::List(key_list) = keys {
            assert_eq!(key_list.len(), 2);
            // Order may vary
            assert!(key_list.contains(&Value::new_string("x".to_string())));
            assert!(key_list.contains(&Value::new_string("y".to_string())));
        }
        
        // Values
        let values = backend.call("values", vec![map.clone()]).unwrap();
        if let Value::List(value_list) = values {
            assert_eq!(value_list.len(), 2);
            assert!(value_list.contains(&Value::Int(10)));
            assert!(value_list.contains(&Value::Int(20)));
        }
    }
    
    #[test]
    fn test_merge() {
        let mut backend = HashMapBackend::new();
        
        let mut map1_data = StdHashMap::new();
        map1_data.insert("a".to_string(), Value::Int(1));
        map1_data.insert("b".to_string(), Value::Int(2));
        let map1 = Value::new_map(map1_data);
        
        let mut map2_data = StdHashMap::new();
        map2_data.insert("b".to_string(), Value::Int(200)); // Overwrites
        map2_data.insert("c".to_string(), Value::Int(3));
        let map2 = Value::new_map(map2_data);
        
        // Merge
        let merged = backend.call("merge", vec![map1, map2]).unwrap();
        
        let len = backend.call("len", vec![merged.clone()]).unwrap();
        assert_eq!(len, Value::Int(3)); // a, b, c
        
        // b should be overwritten
        let b_val = backend.call("get", vec![
            merged.clone(),
            Value::new_string("b".to_string()),
        ]).unwrap();
        assert_eq!(b_val, Value::Int(200));
    }
    
    #[test]
    fn test_from_pairs_to_pairs() {
        let mut backend = HashMapBackend::new();
        
        let pairs = Value::new_list(vec![
            Value::new_list(vec![
                Value::new_string("k1".to_string()),
                Value::Int(10),
            ]),
            Value::new_list(vec![
                Value::new_string("k2".to_string()),
                Value::Int(20),
            ]),
        ]);
        
        // from_pairs
        let map = backend.call("from_pairs", vec![pairs]).unwrap();
        let len = backend.call("len", vec![map.clone()]).unwrap();
        assert_eq!(len, Value::Int(2));
        
        // to_pairs
        let pairs_back = backend.call("to_pairs", vec![map]).unwrap();
        if let Value::List(pair_list) = pairs_back {
            assert_eq!(pair_list.len(), 2);
        }
    }
}
