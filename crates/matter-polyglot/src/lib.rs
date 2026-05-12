//! Matter Polyglot - Universal Language Bridge System
//!
//! Permite importar e usar bibliotecas de Python, Node.js, Rust, Go e Java
//! diretamente em código Matter.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod bridge;
pub mod parser;
pub mod resolver;
pub mod types;

/// Linguagens suportadas pelo sistema Polyglot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageTarget {
    Python,
    NodeJS,
    Rust,
    Go,
    Java,
}

impl LanguageTarget {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "python" | "py" => Some(Self::Python),
            "nodejs" | "node" | "javascript" | "js" => Some(Self::NodeJS),
            "rust" | "rs" => Some(Self::Rust),
            "go" | "golang" => Some(Self::Go),
            "java" => Some(Self::Java),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::NodeJS => "nodejs",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Java => "java",
        }
    }

    pub fn package_manager(&self) -> &'static str {
        match self {
            Self::Python => "pip",
            Self::NodeJS => "npm",
            Self::Rust => "cargo",
            Self::Go => "go",
            Self::Java => "maven",
        }
    }
}

/// Representa um import externo de outra linguagem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalImport {
    /// Nome do package/módulo
    pub package: String,
    /// Linguagem de origem
    pub language: LanguageTarget,
    /// Alias opcional (import X as Y)
    pub alias: Option<String>,
    /// Itens específicos importados (import {a, b} from X)
    pub items: Vec<String>,
}

impl ExternalImport {
    pub fn new(package: String, language: LanguageTarget) -> Self {
        Self {
            package,
            language,
            alias: None,
            items: Vec::new(),
        }
    }

    pub fn with_alias(mut self, alias: String) -> Self {
        self.alias = Some(alias);
        self
    }

    pub fn with_items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }

    /// Retorna o nome usado no código Matter
    pub fn binding_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.package)
    }
}

/// Configuração de dependências polyglot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolyglotConfig {
    pub python: HashMap<String, String>,
    pub nodejs: HashMap<String, String>,
    pub rust: HashMap<String, String>,
    pub go: HashMap<String, String>,
    pub java: HashMap<String, String>,
}

impl Default for PolyglotConfig {
    fn default() -> Self {
        Self {
            python: HashMap::new(),
            nodejs: HashMap::new(),
            rust: HashMap::new(),
            go: HashMap::new(),
            java: HashMap::new(),
        }
    }
}

impl PolyglotConfig {
    pub fn from_toml(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let value: toml::Value = toml::from_str(content)?;

        let mut config = Self::default();

        if let Some(deps) = value.get("dependencies") {
            if let Some(python) = deps.get("python").and_then(|v| v.as_table()) {
                for (k, v) in python {
                    if let Some(version) = v.as_str() {
                        config.python.insert(k.clone(), version.to_string());
                    }
                }
            }

            if let Some(nodejs) = deps.get("nodejs").and_then(|v| v.as_table()) {
                for (k, v) in nodejs {
                    if let Some(version) = v.as_str() {
                        config.nodejs.insert(k.clone(), version.to_string());
                    }
                }
            }

            if let Some(rust) = deps.get("rust").and_then(|v| v.as_table()) {
                for (k, v) in rust {
                    if let Some(version) = v.as_str() {
                        config.rust.insert(k.clone(), version.to_string());
                    }
                }
            }

            if let Some(go) = deps.get("go").and_then(|v| v.as_table()) {
                for (k, v) in go {
                    if let Some(version) = v.as_str() {
                        config.go.insert(k.clone(), version.to_string());
                    }
                }
            }

            if let Some(java) = deps.get("java").and_then(|v| v.as_table()) {
                for (k, v) in java {
                    if let Some(version) = v.as_str() {
                        config.java.insert(k.clone(), version.to_string());
                    }
                }
            }
        }

        Ok(config)
    }

    pub fn get_dependencies(&self, language: LanguageTarget) -> &HashMap<String, String> {
        match language {
            LanguageTarget::Python => &self.python,
            LanguageTarget::NodeJS => &self.nodejs,
            LanguageTarget::Rust => &self.rust,
            LanguageTarget::Go => &self.go,
            LanguageTarget::Java => &self.java,
        }
    }
}

/// Sistema principal de gerenciamento Polyglot
pub struct PolyglotSystem {
    imports: Vec<ExternalImport>,
    config: PolyglotConfig,
    #[allow(dead_code)]
    project_root: PathBuf,
}

impl PolyglotSystem {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            imports: Vec::new(),
            config: PolyglotConfig::default(),
            project_root,
        }
    }

    pub fn with_config(mut self, config: PolyglotConfig) -> Self {
        self.config = config;
        self
    }

    pub fn add_import(&mut self, import: ExternalImport) {
        self.imports.push(import);
    }

    pub fn get_imports(&self) -> &[ExternalImport] {
        &self.imports
    }

    pub fn get_imports_by_language(&self, language: LanguageTarget) -> Vec<&ExternalImport> {
        self.imports
            .iter()
            .filter(|imp| imp.language == language)
            .collect()
    }

    /// Valida se todos os imports têm dependências declaradas
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for import in &self.imports {
            let deps = self.config.get_dependencies(import.language);
            if !deps.contains_key(&import.package) {
                errors.push(format!(
                    "Package '{}' from {} not declared in matter.toml",
                    import.package,
                    import.language.as_str()
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Retorna estatísticas sobre os imports
    pub fn stats(&self) -> PolyglotStats {
        let mut stats = PolyglotStats::default();

        for import in &self.imports {
            match import.language {
                LanguageTarget::Python => stats.python_imports += 1,
                LanguageTarget::NodeJS => stats.nodejs_imports += 1,
                LanguageTarget::Rust => stats.rust_imports += 1,
                LanguageTarget::Go => stats.go_imports += 1,
                LanguageTarget::Java => stats.java_imports += 1,
            }
        }

        stats.total_imports = self.imports.len();
        stats
    }
}

#[derive(Debug, Default, Serialize)]
pub struct PolyglotStats {
    pub total_imports: usize,
    pub python_imports: usize,
    pub nodejs_imports: usize,
    pub rust_imports: usize,
    pub go_imports: usize,
    pub java_imports: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_target_from_str() {
        assert_eq!(
            LanguageTarget::from_str("python"),
            Some(LanguageTarget::Python)
        );
        assert_eq!(
            LanguageTarget::from_str("nodejs"),
            Some(LanguageTarget::NodeJS)
        );
        assert_eq!(LanguageTarget::from_str("rust"), Some(LanguageTarget::Rust));
        assert_eq!(LanguageTarget::from_str("go"), Some(LanguageTarget::Go));
        assert_eq!(LanguageTarget::from_str("java"), Some(LanguageTarget::Java));
        assert_eq!(LanguageTarget::from_str("invalid"), None);
    }

    #[test]
    fn test_external_import() {
        let import = ExternalImport::new("numpy".to_string(), LanguageTarget::Python)
            .with_alias("np".to_string());

        assert_eq!(import.package, "numpy");
        assert_eq!(import.language, LanguageTarget::Python);
        assert_eq!(import.binding_name(), "np");
    }

    #[test]
    fn test_polyglot_system() {
        let mut system = PolyglotSystem::new(PathBuf::from("."));

        system.add_import(ExternalImport::new(
            "numpy".to_string(),
            LanguageTarget::Python,
        ));
        system.add_import(ExternalImport::new(
            "express".to_string(),
            LanguageTarget::NodeJS,
        ));

        assert_eq!(system.get_imports().len(), 2);
        assert_eq!(
            system.get_imports_by_language(LanguageTarget::Python).len(),
            1
        );

        let stats = system.stats();
        assert_eq!(stats.total_imports, 2);
        assert_eq!(stats.python_imports, 1);
        assert_eq!(stats.nodejs_imports, 1);
    }

    #[test]
    fn test_polyglot_config_from_toml() {
        let toml_content = r#"
[dependencies.python]
numpy = "1.24.0"
pandas = "2.0.0"

[dependencies.nodejs]
express = "^4.18.0"
        "#;

        let config = PolyglotConfig::from_toml(toml_content).unwrap();

        assert_eq!(config.python.get("numpy"), Some(&"1.24.0".to_string()));
        assert_eq!(config.python.get("pandas"), Some(&"2.0.0".to_string()));
        assert_eq!(config.nodejs.get("express"), Some(&"^4.18.0".to_string()));
    }
}
