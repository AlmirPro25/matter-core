//! File I/O Backend - Sprint 80
//! Read, write, append, exists, delete operations
//!
//! All program-initiated paths are gated by [`FsCapabilityPolicy`] (File Capabilities v1).

use crate::fs_capability::{FsCapabilityPolicy, FsPermission};
use matter_backend::{Backend, Value};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct FileBackend {
    policy: FsCapabilityPolicy,
}

impl FileBackend {
    /// Default: deny all program filesystem access.
    pub fn new() -> Self {
        Self {
            policy: FsCapabilityPolicy::deny_all(),
        }
    }

    pub fn with_policy(policy: FsCapabilityPolicy) -> Self {
        Self { policy }
    }

    fn auth(&self, method: &str, path: &str) -> Result<PathBuf, String> {
        let perm =
            FsCapabilityPolicy::permission_for_method("fileio", method).ok_or_else(|| {
                format!(
                    "capability_denied: multi-path or unknown fileio method '{}'",
                    method
                )
            })?;
        self.policy.check_path(path, perm)
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
                    return Err(format!(
                        "fileio.read expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.read: {}", e))?;
                let path = self.auth("read", &path)?;

                let mut file = fs::File::open(&path)
                    .map_err(|e| format!("fileio.read: cannot open: {}", e))?;

                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .map_err(|e| format!("fileio.read: cannot read: {}", e))?;

                Ok(Value::new_string(contents))
            }

            "write" => {
                if args.len() != 2 {
                    return Err(format!(
                        "fileio.write expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.write: {}", e))?;
                let content = args[1]
                    .as_string()
                    .map_err(|e| format!("fileio.write: {}", e))?;
                let path = self.auth("write", &path)?;

                fs::write(&path, content)
                    .map_err(|e| format!("fileio.write: cannot write: {}", e))?;

                Ok(Value::Bool(true))
            }

            "append" => {
                if args.len() != 2 {
                    return Err(format!(
                        "fileio.append expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.append: {}", e))?;
                let content = args[1]
                    .as_string()
                    .map_err(|e| format!("fileio.append: {}", e))?;
                let path = self.auth("append", &path)?;

                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .map_err(|e| format!("fileio.append: cannot open: {}", e))?;

                file.write_all(content.as_bytes())
                    .map_err(|e| format!("fileio.append: cannot write: {}", e))?;

                Ok(Value::Bool(true))
            }

            "exists" => {
                if args.len() != 1 {
                    return Err(format!(
                        "fileio.exists expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.exists: {}", e))?;
                let path = self.auth("exists", &path)?;

                Ok(Value::Bool(path.exists()))
            }

            "delete" => {
                if args.len() != 1 {
                    return Err(format!(
                        "fileio.delete expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.delete: {}", e))?;
                let path = self.auth("delete", &path)?;

                fs::remove_file(&path)
                    .map_err(|e| format!("fileio.delete: cannot delete: {}", e))?;

                Ok(Value::Bool(true))
            }

            "read_lines" => {
                if args.len() != 1 {
                    return Err(format!(
                        "fileio.read_lines expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.read_lines: {}", e))?;
                let path = self.auth("read_lines", &path)?;

                let contents = fs::read_to_string(&path)
                    .map_err(|e| format!("fileio.read_lines: cannot read: {}", e))?;

                let lines: Vec<Value> = contents
                    .lines()
                    .map(|line| Value::new_string(line.to_string()))
                    .collect();

                Ok(Value::new_list(lines))
            }

            "write_lines" => {
                if args.len() != 2 {
                    return Err(format!(
                        "fileio.write_lines expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.write_lines: {}", e))?;
                let path = self.auth("write_lines", &path)?;

                let lines = match &args[1] {
                    Value::List(items) => {
                        let mut result = Vec::new();
                        for item in items.iter() {
                            result.push(
                                item.as_string()
                                    .map_err(|e| format!("fileio.write_lines: {}", e))?,
                            );
                        }
                        result
                    }
                    _ => {
                        return Err("fileio.write_lines: second argument must be a list".to_string())
                    }
                };

                let content = lines.join("\n");
                fs::write(&path, content)
                    .map_err(|e| format!("fileio.write_lines: cannot write: {}", e))?;

                Ok(Value::Bool(true))
            }

            "copy" => {
                if args.len() != 2 {
                    return Err(format!(
                        "fileio.copy expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let src = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.copy: {}", e))?;
                let dst = args[1]
                    .as_string()
                    .map_err(|e| format!("fileio.copy: {}", e))?;
                let src = self.policy.check_path(&src, FsPermission::Read)?;
                let dst = self.policy.check_path(&dst, FsPermission::Write)?;

                fs::copy(&src, &dst).map_err(|e| format!("fileio.copy: cannot copy: {}", e))?;

                Ok(Value::Bool(true))
            }

            "rename" => {
                if args.len() != 2 {
                    return Err(format!(
                        "fileio.rename expects 2 arguments, got {}",
                        args.len()
                    ));
                }
                let old = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.rename: {}", e))?;
                let new = args[1]
                    .as_string()
                    .map_err(|e| format!("fileio.rename: {}", e))?;
                // Rename needs read+write+delete on the source side of the move.
                let old = self.policy.check_path(&old, FsPermission::Read)?;
                let _ = self
                    .policy
                    .check_path(old.to_str().unwrap_or(""), FsPermission::Delete)?;
                let new = self.policy.check_path(&new, FsPermission::Write)?;

                fs::rename(&old, &new)
                    .map_err(|e| format!("fileio.rename: cannot rename: {}", e))?;

                Ok(Value::Bool(true))
            }

            "size" => {
                if args.len() != 1 {
                    return Err(format!(
                        "fileio.size expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.size: {}", e))?;
                let path = self.auth("size", &path)?;

                let metadata = fs::metadata(&path)
                    .map_err(|e| format!("fileio.size: cannot read metadata: {}", e))?;

                Ok(Value::Int(metadata.len() as i64))
            }

            "is_file" => {
                if args.len() != 1 {
                    return Err(format!(
                        "fileio.is_file expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.is_file: {}", e))?;
                let path = self.auth("is_file", &path)?;

                Ok(Value::Bool(path.is_file()))
            }

            "is_dir" => {
                if args.len() != 1 {
                    return Err(format!(
                        "fileio.is_dir expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.is_dir: {}", e))?;
                let path = self.auth("is_dir", &path)?;

                Ok(Value::Bool(path.is_dir()))
            }

            "create_dir" => {
                if args.len() != 1 {
                    return Err(format!(
                        "fileio.create_dir expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.create_dir: {}", e))?;
                let path = self.auth("create_dir", &path)?;

                fs::create_dir_all(&path)
                    .map_err(|e| format!("fileio.create_dir: cannot create directory: {}", e))?;

                Ok(Value::Bool(true))
            }

            "list_dir" => {
                if args.len() != 1 {
                    return Err(format!(
                        "fileio.list_dir expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.list_dir: {}", e))?;
                let path = self.auth("list_dir", &path)?;

                let entries = fs::read_dir(&path)
                    .map_err(|e| format!("fileio.list_dir: cannot read directory: {}", e))?;

                let mut items = Vec::new();
                for entry in entries {
                    let entry = entry.map_err(|e| format!("fileio.list_dir: {}", e))?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    items.push(Value::new_string(name));
                }

                Ok(Value::new_list(items))
            }

            "remove_dir" => {
                if args.len() != 1 {
                    return Err(format!(
                        "fileio.remove_dir expects 1 argument, got {}",
                        args.len()
                    ));
                }
                let path = args[0]
                    .as_string()
                    .map_err(|e| format!("fileio.remove_dir: {}", e))?;
                let path = self.auth("remove_dir", &path)?;

                fs::remove_dir_all(&path)
                    .map_err(|e| format!("fileio.remove_dir: cannot remove directory: {}", e))?;

                Ok(Value::Bool(true))
            }

            _ => Err(format!("Unknown fileio method: {}", method)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("matter_fileio_{}", name));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn backend_for(root: &std::path::Path) -> FileBackend {
        let mut policy = FsCapabilityPolicy::deny_all();
        policy.allow_read_root(root).unwrap();
        policy.allow_write_root(root).unwrap();
        policy.allow_delete_root(root).unwrap();
        FileBackend::with_policy(policy)
    }

    fn temp_path(root: &std::path::Path, name: &str) -> PathBuf {
        root.join(name)
    }

    #[test]
    fn default_denies_without_roots() {
        let mut backend = FileBackend::new();
        let err = backend
            .call(
                "write",
                vec![
                    Value::new_string("x.txt".into()),
                    Value::new_string("y".into()),
                ],
            )
            .unwrap_err();
        assert!(err.contains("capability_denied"));
    }

    #[test]
    fn test_write_and_read() {
        let root = temp_root("write_read");
        let mut backend = backend_for(&root);
        let path = temp_path(&root, "write_read.txt");
        let path_str = path.to_string_lossy().to_string();

        let result = backend.call(
            "write",
            vec![
                Value::new_string(path_str.clone()),
                Value::new_string("Hello, Matter!".to_string()),
            ],
        );
        assert!(result.is_ok(), "{:?}", result);

        let result = backend.call("read", vec![Value::new_string(path_str.clone())]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_string().unwrap(), "Hello, Matter!");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_exists() {
        let root = temp_root("exists");
        let mut backend = backend_for(&root);
        let path = temp_path(&root, "exists.txt");
        let path_str = path.to_string_lossy().to_string();

        let result = backend.call("exists", vec![Value::new_string(path_str.clone())]);
        assert_eq!(result.unwrap(), Value::Bool(false));

        fs::write(&path, "test").unwrap();

        let result = backend.call("exists", vec![Value::new_string(path_str.clone())]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_append() {
        let root = temp_root("append");
        let mut backend = backend_for(&root);
        let path = temp_path(&root, "append.txt");
        let path_str = path.to_string_lossy().to_string();

        backend
            .call(
                "write",
                vec![
                    Value::new_string(path_str.clone()),
                    Value::new_string("Line 1\n".to_string()),
                ],
            )
            .unwrap();

        backend
            .call(
                "append",
                vec![
                    Value::new_string(path_str.clone()),
                    Value::new_string("Line 2\n".to_string()),
                ],
            )
            .unwrap();

        let result = backend.call("read", vec![Value::new_string(path_str.clone())]);
        assert_eq!(result.unwrap().as_string().unwrap(), "Line 1\nLine 2\n");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_read_write_lines() {
        let root = temp_root("lines");
        let mut backend = backend_for(&root);
        let path = temp_path(&root, "lines.txt");
        let path_str = path.to_string_lossy().to_string();

        let lines = vec![
            Value::new_string("First line".to_string()),
            Value::new_string("Second line".to_string()),
            Value::new_string("Third line".to_string()),
        ];

        backend
            .call(
                "write_lines",
                vec![
                    Value::new_string(path_str.clone()),
                    Value::new_list(lines.clone()),
                ],
            )
            .unwrap();

        let result = backend.call("read_lines", vec![Value::new_string(path_str.clone())]);
        if let Value::List(read_lines) = result.unwrap() {
            assert_eq!(read_lines.len(), 3);
            assert_eq!(read_lines[0].as_string().unwrap(), "First line");
        } else {
            panic!("Expected list");
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_directory_operations() {
        let root = temp_root("test_dir_parent");
        let mut backend = backend_for(&root);
        let dir_path = temp_path(&root, "test_dir");
        let dir_str = dir_path.to_string_lossy().to_string();

        // Create directory
        backend
            .call("create_dir", vec![Value::new_string(dir_str.clone())])
            .unwrap();

        // Check is_dir
        let result = backend.call("is_dir", vec![Value::new_string(dir_str.clone())]);
        assert_eq!(result.unwrap(), Value::Bool(true));

        // Create a file inside
        let file_path = dir_path.join("test.txt");
        let file_str = file_path.to_string_lossy().to_string();
        backend
            .call(
                "write",
                vec![
                    Value::new_string(file_str.clone()),
                    Value::new_string("test".to_string()),
                ],
            )
            .unwrap();

        // List directory
        let result = backend.call("list_dir", vec![Value::new_string(dir_str.clone())]);
        if let Value::List(items) = result.unwrap() {
            assert_eq!(items.len(), 1);
        }

        // Cleanup
        let _ = fs::remove_dir_all(root);
    }
}
