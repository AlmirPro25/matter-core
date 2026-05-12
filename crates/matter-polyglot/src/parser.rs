//! Parser para imports externos (import X from python)

use crate::{ExternalImport, LanguageTarget};
use regex::Regex;

pub struct PolyglotParser {
    import_regex: Regex,
}

impl PolyglotParser {
    pub fn new() -> Self {
        // Regex para: import "package" from language [as alias]
        let import_regex =
            Regex::new(r#"import\s+"([^"]+)"\s+from\s+(\w+)(?:\s+as\s+(\w+))?"#).unwrap();

        Self { import_regex }
    }

    /// Parse uma linha de import externo
    pub fn parse_import(&self, line: &str) -> Option<ExternalImport> {
        if let Some(captures) = self.import_regex.captures(line) {
            let package = captures.get(1)?.as_str().to_string();
            let language_str = captures.get(2)?.as_str();
            let language = LanguageTarget::from_str(language_str)?;
            let alias = captures.get(3).map(|m| m.as_str().to_string());

            let mut import = ExternalImport::new(package, language);
            if let Some(alias) = alias {
                import = import.with_alias(alias);
            }

            Some(import)
        } else {
            None
        }
    }

    /// Parse todos os imports de um arquivo Matter
    pub fn parse_file(&self, source: &str) -> Vec<ExternalImport> {
        let mut imports = Vec::new();

        for line in source.lines() {
            let line = line.trim();
            if line.starts_with("import") && line.contains("from") {
                if let Some(import) = self.parse_import(line) {
                    imports.push(import);
                }
            }
        }

        imports
    }
}

impl Default for PolyglotParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_import() {
        let parser = PolyglotParser::new();
        let import = parser
            .parse_import(r#"import "numpy" from python"#)
            .unwrap();

        assert_eq!(import.package, "numpy");
        assert_eq!(import.language, LanguageTarget::Python);
        assert_eq!(import.alias, None);
    }

    #[test]
    fn test_parse_import_with_alias() {
        let parser = PolyglotParser::new();
        let import = parser
            .parse_import(r#"import "numpy" from python as np"#)
            .unwrap();

        assert_eq!(import.package, "numpy");
        assert_eq!(import.language, LanguageTarget::Python);
        assert_eq!(import.alias, Some("np".to_string()));
        assert_eq!(import.binding_name(), "np");
    }

    #[test]
    fn test_parse_nodejs_import() {
        let parser = PolyglotParser::new();
        let import = parser
            .parse_import(r#"import "express" from nodejs"#)
            .unwrap();

        assert_eq!(import.package, "express");
        assert_eq!(import.language, LanguageTarget::NodeJS);
    }

    #[test]
    fn test_parse_file() {
        let parser = PolyglotParser::new();
        let source = r#"
# Comentário
import "numpy" from python as np
import "pandas" from python
import "express" from nodejs

fn main() {
    print "Hello"
}
        "#;

        let imports = parser.parse_file(source);
        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].package, "numpy");
        assert_eq!(imports[1].package, "pandas");
        assert_eq!(imports[2].package, "express");
    }
}
