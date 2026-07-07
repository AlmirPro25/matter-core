//! File I/O Backend - Sprint 80
//! Read, write, append, exists, delete operations

use matter_backend::{Backend, Value};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

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
                // file.read("path/to/file.txt") → String
                if args.len() != 1 {
                    return Err(format!("file.read expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.read: {}", e))?;
                
                let mut file = fs::File::open(&path)
                    .map_err(|e| format!("file.read: cannot open '{}': {}", path, e))?;
                
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .map_err(|e| format!("file.read: cannot read '{}': {}", path, e))?;
                
                Ok(Value::new_string(contents))
            }
            
            "write" => {
                // file.write("path/to/file.txt", "content") → Bool (success)
                if args.len() != 2 {
                    return Err(format!("file.write expects 2 arguments, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.write: {}", e))?;
                let content = args[1].as_string().map_err(|e| format!("file.write: {}", e))?;
                
                fs::write(&path, content)
                    .map_err(|e| format!("file.write: cannot write '{}': {}", path, e))?;
                
                Ok(Value::Bool(true))
            }
            
            "append" => {
                // file.append("path/to/file.txt", "more content") → Bool
                if args.len() != 2 {
                    return Err(format!("file.append expects 2 arguments, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.append: {}", e))?;
                let content = args[1].as_string().map_err(|e| format!("file.append: {}", e))?;
                
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .map_err(|e| format!("file.append: cannot open '{}': {}", path, e))?;
                
                file.write_all(content.as_bytes())
                    .map_err(|e| format!("file.append: cannot write '{}': {}", path, e))?;
                
                Ok(Value::Bool(true))
            }
            
            "exists" => {
                // file.exists("path/to/file.txt") → Bool
                if args.len() != 1 {
                    return Err(format!("file.exists expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.exists: {}", e))?;
                
                Ok(Value::Bool(Path::new(&path).exists()))
            }
            
            "delete" => {
                // file.delete("path/to/file.txt") → Bool
                if args.len() != 1 {
                    return Err(format!("file.delete expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.delete: {}", e))?;
                
                fs::remove_file(&path)
                    .map_err(|e| format!("file.delete: cannot delete '{}': {}", path, e))?;
                
                Ok(Value::Bool(true))
            }
            
            "read_lines" => {
                // file.read_lines("path/to/file.txt") → List<String>
                if args.len() != 1 {
                    return Err(format!("file.read_lines expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.read_lines: {}", e))?;
                
                let contents = fs::read_to_string(&path)
                    .map_err(|e| format!("file.read_lines: cannot read '{}': {}", path, e))?;
                
                let lines: Vec<Value> = contents
                    .lines()
                    .map(|line| Value::new_string(line.to_string()))
                    .collect();
                
                Ok(Value::new_list(lines))
            }
            
            "write_lines" => {
                // file.write_lines("path/to/file.txt", ["line1", "line2"]) → Bool
                if args.len() != 2 {
                    return Err(format!("file.write_lines expects 2 arguments, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.write_lines: {}", e))?;
                
                let lines = match &args[1] {
                    Value::List(items) => {
                        let mut result = Vec::new();
                        for item in items.iter() {
                            result.push(item.as_string().map_err(|e| format!("file.write_lines: {}", e))?);
                        }
                        result
                    }
                    _ => return Err("file.write_lines: second argument must be a list".to_string()),
                };
                
                let content = lines.join("\n");
                fs::write(&path, content)
                    .map_err(|e| format!("file.write_lines: cannot write '{}': {}", path, e))?;
                
                Ok(Value::Bool(true))
            }
            
            "copy" => {
                // file.copy("source.txt", "destination.txt") → Bool
                if args.len() != 2 {
                    return Err(format!("file.copy expects 2 arguments, got {}", args.len()));
                }
                let src = args[0].as_string().map_err(|e| format!("file.copy: {}", e))?;
                let dst = args[1].as_string().map_err(|e| format!("file.copy: {}", e))?;
                
                fs::copy(&src, &dst)
                    .map_err(|e| format!("file.copy: cannot copy '{}' to '{}': {}", src, dst, e))?;
                
                Ok(Value::Bool(true))
            }
            
            "rename" => {
                // file.rename("old.txt", "new.txt") → Bool
                if args.len() != 2 {
                    return Err(format!("file.rename expects 2 arguments, got {}", args.len()));
                }
                let old = args[0].as_string().map_err(|e| format!("file.rename: {}", e))?;
                let new = args[1].as_string().map_err(|e| format!("file.rename: {}", e))?;
                
                fs::rename(&old, &new)
                    .map_err(|e| format!("file.rename: cannot rename '{}' to '{}': {}", old, new, e))?;
                
                Ok(Value::Bool(true))
            }
            
            "size" => {
                // file.size("path/to/file.txt") → Int (bytes)
                if args.len() != 1 {
                    return Err(format!("file.size expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.size: {}", e))?;
                
                let metadata = fs::metadata(&path)
                    .map_err(|e| format!("file.size: cannot read metadata '{}': {}", path, e))?;
                
                Ok(Value::Int(metadata.len() as i64))
            }
            
            "is_file" => {
                // file.is_file("path") → Bool
                if args.len() != 1 {
                    return Err(format!("file.is_file expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.is_file: {}", e))?;
                
                Ok(Value::Bool(Path::new(&path).is_file()))
            }
            
            "is_dir" => {
                // file.is_dir("path") → Bool
                if args.len() != 1 {
                    return Err(format!("file.is_dir expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.is_dir: {}", e))?;
                
                Ok(Value::Bool(Path::new(&path).is_dir()))
            }
            
            "create_dir" => {
                // file.create_dir("path/to/dir") → Bool
                if args.len() != 1 {
                    return Err(format!("file.create_dir expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.create_dir: {}", e))?;
                
                fs::create_dir_all(&path)
                    .map_err(|e| format!("file.create_dir: cannot create directory '{}': {}", path, e))?;
                
                Ok(Value::Bool(true))
            }
            
            "list_dir" => {
                // file.list_dir("path/to/dir") → List<String>
                if args.len() != 1 {
                    return Err(format!("file.list_dir expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.list_dir: {}", e))?;
                
                let entries = fs::read_dir(&path)
                    .map_err(|e| format!("file.list_dir: cannot read directory '{}': {}", path, e))?;
                
                let mut items = Vec::new();
                for entry in entries {
                    let entry = entry.map_err(|e| format!("file.list_dir: {}", e))?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    items.push(Value::new_string(name));
                }
                
                Ok(Value::new_list(items))
            }
            
            "remove_dir" => {
                // file.remove_dir("path/to/dir") → Bool
                if args.len() != 1 {
                    return Err(format!("file.remove_dir expects 1 argument, got {}", args.len()));
                }
                let path = args[0].as_string().map_err(|e| format!("file.remove_dir: {}", e))?;
                
                fs::remove_dir_all(&path)
                    .map_err(|e| format!("file.remove_dir: cannot remove directory '{}': {}", path, e))?;
                
                Ok(Value::Bool(true))
            }
            
            _ => Err(format!("Unknown file method: {}", method)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    
    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("matter_test_{}", name))
    }
    
    #[test]
    fn test_write_and_read() {
        let mut backend = FileBackend::new();
        let path = temp_path("write_read.txt");
        let path_str = path.to_string_lossy().to_string();
        
        // Write
        let result = backend.call("write", vec![
            Value::new_string(path_str.clone()),
            Value::new_string("Hello, Matter!".to_string()),
        ]);
        assert!(result.is_ok());
        
        // Read
        let result = backend.call("read", vec![Value::new_string(path_str.clone())]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_string().unwrap(), "Hello, Matter!");
        
        // Cleanup
        let _ = fs::remove_file(path);
    }
    
    #[test]
    fn test_exists() {
        let mut backend = FileBackend::new();
        let path = temp_path("exists.txt");
        let path_str = path.to_string_lossy().to_string();
        
        // Initially doesn't exist
        let result = backend.call("exists", vec![Value::new_string(path_str.clone())]);
        assert_eq!(result.unwrap(), Value::Bool(false));
        
        // Create file
        fs::write(&path, "test").unwrap();
        
        // Now exists
        let result = backend.call("exists", vec![Value::new_string(path_str.clone())]);
        assert_eq!(result.unwrap(), Value::Bool(true));
        
        // Cleanup
        let _ = fs::remove_file(path);
    }
    
    #[test]
    fn test_append() {
        let mut backend = FileBackend::new();
        let path = temp_path("append.txt");
        let path_str = path.to_string_lossy().to_string();
        
        // Write initial content
        backend.call("write", vec![
            Value::new_string(path_str.clone()),
            Value::new_string("Line 1\n".to_string()),
        ]).unwrap();
        
        // Append more content
        backend.call("append", vec![
            Value::new_string(path_str.clone()),
            Value::new_string("Line 2\n".to_string()),
        ]).unwrap();
        
        // Read and verify
        let result = backend.call("read", vec![Value::new_string(path_str.clone())]);
        assert_eq!(result.unwrap().as_string().unwrap(), "Line 1\nLine 2\n");
        
        // Cleanup
        let _ = fs::remove_file(path);
    }
    
    #[test]
    fn test_read_write_lines() {
        let mut backend = FileBackend::new();
        let path = temp_path("lines.txt");
        let path_str = path.to_string_lossy().to_string();
        
        let lines = vec![
            Value::new_string("First line".to_string()),
            Value::new_string("Second line".to_string()),
            Value::new_string("Third line".to_string()),
        ];
        
        // Write lines
        backend.call("write_lines", vec![
            Value::new_string(path_str.clone()),
            Value::new_list(lines.clone()),
        ]).unwrap();
        
        // Read lines
        let result = backend.call("read_lines", vec![Value::new_string(path_str.clone())]);
        if let Value::List(read_lines) = result.unwrap() {
            assert_eq!(read_lines.len(), 3);
            assert_eq!(read_lines[0].as_string().unwrap(), "First line");
        } else {
            panic!("Expected list");
        }
        
        // Cleanup
        let _ = fs::remove_file(path);
    }
    
    #[test]
    fn test_directory_operations() {
        let mut backend = FileBackend::new();
        let dir_path = temp_path("test_dir");
        let dir_str = dir_path.to_string_lossy().to_string();
        
        // Create directory
        backend.call("create_dir", vec![Value::new_string(dir_str.clone())]).unwrap();
        
        // Check is_dir
        let result = backend.call("is_dir", vec![Value::new_string(dir_str.clone())]);
        assert_eq!(result.unwrap(), Value::Bool(true));
        
        // Create a file inside
        let file_path = dir_path.join("test.txt");
        let file_str = file_path.to_string_lossy().to_string();
        backend.call("write", vec![
            Value::new_string(file_str.clone()),
            Value::new_string("test".to_string()),
        ]).unwrap();
        
        // List directory
        let result = backend.call("list_dir", vec![Value::new_string(dir_str.clone())]);
        if let Value::List(items) = result.unwrap() {
            assert_eq!(items.len(), 1);
        }
        
        // Cleanup
        let _ = fs::remove_dir_all(dir_path);
    }
}
