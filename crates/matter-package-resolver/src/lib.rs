//! Import resolver for Matter language
//! Resolves import paths to file paths and inlines source code

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for import resolution
pub struct ResolverConfig {
    /// Root directory of the project (where matter.toml lives)
    pub project_root: PathBuf,
    /// Declared dependencies: name -> entry file path
    pub dependencies: HashMap<String, PathBuf>,
    /// Maximum recursion depth for nested imports
    pub max_depth: usize,
}

impl ResolverConfig {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            dependencies: HashMap::new(),
            max_depth: 32,
        }
    }

    pub fn with_dependencies(mut self, deps: HashMap<String, PathBuf>) -> Self {
        self.dependencies = deps;
        self
    }
}

/// Information about a resolved import
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    /// The original import path as written in source
    pub original_path: String,
    /// The resolved absolute file path
    pub resolved_path: PathBuf,
    /// The source code content of the imported file
    pub source: String,
    /// Imports within this import (nested)
    pub nested: Vec<ResolvedImport>,
}

/// Errors during import resolution
#[derive(Debug, Clone)]
pub enum ResolveError {
    /// The import path could not be found
    NotFound(String),
    /// Circular import detected
    CircularImport(Vec<String>),
    /// Maximum recursion depth exceeded
    MaxDepthExceeded(String),
    /// IO error reading file
    IoError(String),
    /// Invalid path
    InvalidPath(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::NotFound(path) => write!(f, "Import not found: {}", path),
            ResolveError::CircularImport(chain) => {
                write!(f, "Circular import: {}", chain.join(" -> "))
            }
            ResolveError::MaxDepthExceeded(path) => {
                write!(f, "Max import depth exceeded at: {}", path)
            }
            ResolveError::IoError(msg) => write!(f, "IO error: {}", msg),
            ResolveError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
        }
    }
}

impl std::error::Error for ResolveError {}

/// The main import resolver
pub struct ImportResolver {
    config: ResolverConfig,
    /// Cache of resolved sources: absolute path -> source code
    cache: HashMap<PathBuf, String>,
    /// Stack of currently resolving imports (for circular detection)
    resolving: Vec<PathBuf>,
}

impl ImportResolver {
    pub fn new(config: ResolverConfig) -> Self {
        Self {
            config,
            cache: HashMap::new(),
            resolving: Vec::new(),
        }
    }

    /// Resolve an import path to an absolute file path
    pub fn resolve_path(
        &self,
        import_path: &str,
        from_file: Option<&Path>,
    ) -> Result<PathBuf, ResolveError> {
        // 1. Check if it's a declared dependency
        if let Some(dep_path) = self.config.dependencies.get(import_path) {
            return Ok(dep_path.clone());
        }

        // 2. Check for std/ prefix -> resolve against stdlib/
        if let Some(std_path) = import_path.strip_prefix("std/") {
            let full = self.config.project_root.join("stdlib").join(format!("{}.matter", std_path));
            if full.exists() {
                return Ok(full);
            }
            // Try without .matter extension appended (if already has it)
            let full2 = self.config.project_root.join("stdlib").join(std_path);
            if full2.exists() {
                return Ok(full2);
            }
            return Ok(full);
        }

        // 3. Resolve relative to the importing file
        if let Some(from) = from_file {
            if let Some(parent) = from.parent() {
                // Try direct relative path
                let candidate = parent.join(import_path);
                if candidate.exists() {
                    return Ok(candidate);
                }
                // Try with .matter extension
                let with_ext = parent.join(format!("{}.matter", import_path));
                if with_ext.exists() {
                    return Ok(with_ext);
                }
            }
        }

        // 4. Resolve relative to project root
        let candidate = self.config.project_root.join(import_path);
        if candidate.exists() {
            return Ok(candidate);
        }
        let with_ext = self.config.project_root.join(format!("{}.matter", import_path));
        if with_ext.exists() {
            return Ok(with_ext);
        }

        // 5. Return best guess (will error when trying to read)
        if let Some(from) = from_file {
            if let Some(parent) = from.parent() {
                return Ok(parent.join(import_path));
            }
        }
        Ok(self.config.project_root.join(import_path))
    }

    /// Parse import lines from source code and return (import_path, line_range)
    pub fn parse_import_lines(source: &str) -> Vec<(String, usize)> {
        let mut imports = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") {
                let rest = &trimmed[7..];
                // Handle: import "path"
                if rest.starts_with('"') {
                    if let Some(end) = rest[1..].find('"') {
                        imports.push((rest[1..end + 1].to_string(), i));
                    }
                }
                // Handle: import { a, b } from "path" - extract path
                else if rest.starts_with('{') {
                    if let Some(from_pos) = rest.find(" from ") {
                        let after_from = &rest[from_pos + 6..];
                        let after_from = after_from.trim();
                        if after_from.starts_with('"') {
                            if let Some(end) = after_from[1..].find('"') {
                                imports.push((after_from[1..end + 1].to_string(), i));
                            }
                        }
                    }
                }
                // Handle: import path
                else if rest.starts_with(|c: char| c.is_alphanumeric()) {
                    let path: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
                    imports.push((path, i));
                }
            }
        }
        imports
    }

    /// Resolve all imports in a source file and return the inlined source
    pub fn resolve_and_inline(
        &mut self,
        source: &str,
        file_path: &Path,
    ) -> Result<String, ResolveError> {
        let imports = Self::parse_import_lines(source);
        if imports.is_empty() {
            return Ok(source.to_string());
        }

        let mut resolved_sources: HashMap<usize, String> = HashMap::new();

        for (import_path, line_idx) in &imports {
            let resolved = self.resolve_single_import(import_path, Some(file_path), 0)?;
            resolved_sources.insert(*line_idx, resolved.source);
        }

        // Build the inlined source: replace import lines with resolved source
        let mut result = String::new();
        for (i, line) in source.lines().enumerate() {
            if let Some(imported_source) = resolved_sources.get(&i) {
                result.push_str(imported_source);
                result.push('\n');
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        Ok(result)
    }

    /// Resolve a single import recursively
    fn resolve_single_import(
        &mut self,
        import_path: &str,
        from_file: Option<&Path>,
        depth: usize,
    ) -> Result<ResolvedImport, ResolveError> {
        if depth > self.config.max_depth {
            return Err(ResolveError::MaxDepthExceeded(import_path.to_string()));
        }

        let resolved_path = self.resolve_path(import_path, from_file)?;

        // Check for circular imports
        if self.resolving.contains(&resolved_path) {
            let mut chain: Vec<String> = self
                .resolving
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            chain.push(resolved_path.display().to_string());
            return Err(ResolveError::CircularImport(chain));
        }

        // Read source (use cache if available)
        let source = if let Some(cached) = self.cache.get(&resolved_path) {
            cached.clone()
        } else {
            let src = fs::read_to_string(&resolved_path)
                .map_err(|e| ResolveError::IoError(format!("{}: {}", resolved_path.display(), e)))?;
            self.cache.insert(resolved_path.clone(), src.clone());
            src
        };

        // Resolve nested imports
        self.resolving.push(resolved_path.clone());
        let nested_imports = Self::parse_import_lines(&source);
        let mut nested = Vec::new();

        for (nested_path, _) in &nested_imports {
            let resolved = self.resolve_single_import(nested_path, Some(&resolved_path), depth + 1)?;
            nested.push(resolved);
        }
        self.resolving.pop();

        Ok(ResolvedImport {
            original_path: import_path.to_string(),
            resolved_path,
            source,
            nested,
        })
    }

    /// Get a summary of all imports in a file (for inspection)
    pub fn collect_imports(
        &mut self,
        source: &str,
        file_path: &Path,
    ) -> Vec<ResolvedImport> {
        let imports = Self::parse_import_lines(source);
        let mut result = Vec::new();

        for (import_path, _) in &imports {
            match self.resolve_single_import(import_path, Some(file_path), 0) {
                Ok(resolved) => result.push(resolved),
                Err(_) => {} // Skip unresolvable imports in inspection mode
            }
        }

        result
    }

    /// Clear the resolution cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_import_lines() {
        let source = r#"import "math.matter"
import "utils"
let x = 10
import { foo, bar } from "lib.matter""#;

        let imports = ImportResolver::parse_import_lines(source);
        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].0, "math.matter");
        assert_eq!(imports[1].0, "utils");
        assert_eq!(imports[2].0, "lib.matter");
    }

    #[test]
    fn test_parse_import_with_alias() {
        let source = r#"import { foo as f, bar } from "lib.matter""#;
        let imports = ImportResolver::parse_import_lines(source);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].0, "lib.matter");
    }

    #[test]
    fn test_resolve_std_prefix() {
        let config = ResolverConfig::new(PathBuf::from("/project"));
        let resolver = ImportResolver::new(config);
        let resolved = resolver.resolve_path("std/math", None).unwrap();
        assert!(resolved.to_string_lossy().contains("stdlib"));
    }

    #[test]
    fn test_resolve_relative_to_file() {
        let config = ResolverConfig::new(PathBuf::from("/project"));
        let resolver = ImportResolver::new(config);
        let from = PathBuf::from("/project/src/main.matter");
        let resolved = resolver.resolve_path("helper.matter", Some(&from)).unwrap();
        assert_eq!(resolved, PathBuf::from("/project/src/helper.matter"));
    }
}
