//! Matter Core Documentation Generator
//!
//! Generates documentation from Matter source code.

use std::collections::HashMap;

/// Documentation comment
#[derive(Debug, Clone)]
pub struct DocComment {
    pub description: String,
    pub params: Vec<(String, String)>,
    pub returns: Option<String>,
    pub examples: Vec<String>,
}

impl DocComment {
    pub fn new() -> Self {
        Self {
            description: String::new(),
            params: Vec::new(),
            returns: None,
            examples: Vec::new(),
        }
    }

    /// Parse doc comments from lines
    pub fn parse(lines: &[String]) -> Self {
        let mut doc = Self::new();
        let mut current_section = Section::Description;
        let mut current_example = String::new();

        for line in lines {
            let content = line.trim_start_matches("##").trim();

            if content.is_empty() {
                continue;
            }

            if content.starts_with("Parâmetros:") || content.starts_with("Parameters:") {
                current_section = Section::Params;
                continue;
            } else if content.starts_with("Retorna:") || content.starts_with("Returns:") {
                current_section = Section::Returns;
                continue;
            } else if content.starts_with("Exemplo:") || content.starts_with("Example:") {
                current_section = Section::Example;
                continue;
            }

            match current_section {
                Section::Description => {
                    if !doc.description.is_empty() {
                        doc.description.push('\n');
                    }
                    doc.description.push_str(content);
                }
                Section::Params => {
                    if let Some((name, desc)) = parse_param(content) {
                        doc.params.push((name, desc));
                    }
                }
                Section::Returns => {
                    if let Some(returns) = &mut doc.returns {
                        returns.push('\n');
                        returns.push_str(content);
                    } else {
                        doc.returns = Some(content.to_string());
                    }
                }
                Section::Example => {
                    if !current_example.is_empty() {
                        current_example.push('\n');
                    }
                    current_example.push_str(content);
                }
            }
        }

        if !current_example.is_empty() {
            doc.examples.push(current_example);
        }

        doc
    }

    /// Generate Markdown
    pub fn to_markdown(&self, name: &str, signature: &str) -> String {
        let mut md = String::new();

        md.push_str(&format!("# {}\n\n", name));
        md.push_str(&format!("{}\n\n", self.description));

        md.push_str("## Assinatura\n\n");
        md.push_str("```matter\n");
        md.push_str(signature);
        md.push_str("\n```\n\n");

        if !self.params.is_empty() {
            md.push_str("## Parâmetros\n\n");
            for (name, desc) in &self.params {
                md.push_str(&format!("- `{}` - {}\n", name, desc));
            }
            md.push('\n');
        }

        if let Some(returns) = &self.returns {
            md.push_str("## Retorna\n\n");
            md.push_str(returns);
            md.push_str("\n\n");
        }

        if !self.examples.is_empty() {
            md.push_str("## Exemplos\n\n");
            for example in &self.examples {
                md.push_str("```matter\n");
                md.push_str(example);
                md.push_str("\n```\n\n");
            }
        }

        md
    }

    /// Generate HTML
    pub fn to_html(&self, name: &str, signature: &str) -> String {
        let mut html = String::new();

        html.push_str(&format!("<h1>{}</h1>\n", escape_html(name)));
        html.push_str(&format!("<p>{}</p>\n", escape_html(&self.description)));

        html.push_str("<h2>Assinatura</h2>\n");
        html.push_str("<pre><code class=\"language-matter\">");
        html.push_str(&escape_html(signature));
        html.push_str("</code></pre>\n");

        if !self.params.is_empty() {
            html.push_str("<h2>Parâmetros</h2>\n<ul>\n");
            for (name, desc) in &self.params {
                html.push_str(&format!(
                    "<li><code>{}</code> - {}</li>\n",
                    escape_html(name),
                    escape_html(desc)
                ));
            }
            html.push_str("</ul>\n");
        }

        if let Some(returns) = &self.returns {
            html.push_str("<h2>Retorna</h2>\n");
            html.push_str(&format!("<p>{}</p>\n", escape_html(returns)));
        }

        if !self.examples.is_empty() {
            html.push_str("<h2>Exemplos</h2>\n");
            for example in &self.examples {
                html.push_str("<pre><code class=\"language-matter\">");
                html.push_str(&escape_html(example));
                html.push_str("</code></pre>\n");
            }
        }

        html
    }
}

impl Default for DocComment {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
enum Section {
    Description,
    Params,
    Returns,
    Example,
}

fn parse_param(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, '-').collect();
    if parts.len() == 2 {
        Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
    } else {
        None
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Documentation generator
pub struct DocGenerator {
    docs: HashMap<String, DocComment>,
}

impl DocGenerator {
    pub fn new() -> Self {
        Self {
            docs: HashMap::new(),
        }
    }

    /// Add documentation for a function
    pub fn add_function(&mut self, name: String, doc: DocComment) {
        self.docs.insert(name, doc);
    }

    /// Generate index page
    pub fn generate_index(&self) -> String {
        let mut md = String::from("# Matter Core API Documentation\n\n");
        md.push_str("## Functions\n\n");

        let mut names: Vec<_> = self.docs.keys().collect();
        names.sort();

        for name in names {
            if let Some(doc) = self.docs.get(name) {
                let desc = doc.description.lines().next().unwrap_or("");
                md.push_str(&format!("- [{}]({}.md) - {}\n", name, name, desc));
            }
        }

        md
    }

    /// Generate all documentation files
    pub fn generate_all(&self) -> HashMap<String, String> {
        let mut files = HashMap::new();

        // Index
        files.insert("index.md".to_string(), self.generate_index());

        // Individual function docs
        for (name, doc) in &self.docs {
            let signature = format!("fn {}(...)", name);
            let content = doc.to_markdown(name, &signature);
            files.insert(format!("{}.md", name), content);
        }

        files
    }
}

impl Default for DocGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_comment_parse() {
        let lines = vec![
            "## Soma dois números".to_string(),
            "##".to_string(),
            "## Parâmetros:".to_string(),
            "##   a - Primeiro número".to_string(),
            "##   b - Segundo número".to_string(),
            "##".to_string(),
            "## Retorna:".to_string(),
            "##   Soma de a e b".to_string(),
        ];

        let doc = DocComment::parse(&lines);

        assert_eq!(doc.description, "Soma dois números");
        assert_eq!(doc.params.len(), 2);
        assert_eq!(doc.params[0].0, "a");
        assert_eq!(doc.params[0].1, "Primeiro número");
        assert!(doc.returns.is_some());
    }

    #[test]
    fn test_doc_comment_to_markdown() {
        let mut doc = DocComment::new();
        doc.description = "Test function".to_string();
        doc.params
            .push(("x".to_string(), "Input value".to_string()));
        doc.returns = Some("Output value".to_string());

        let md = doc.to_markdown("test", "fn test(x)");

        assert!(md.contains("# test"));
        assert!(md.contains("Test function"));
        assert!(md.contains("## Parâmetros"));
        assert!(md.contains("`x`"));
    }

    #[test]
    fn test_doc_generator() {
        let mut gen = DocGenerator::new();

        let mut doc = DocComment::new();
        doc.description = "Test function".to_string();

        gen.add_function("test".to_string(), doc);

        let index = gen.generate_index();
        assert!(index.contains("test"));
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<div>"), "&lt;div&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
    }

    #[test]
    fn test_generate_all() {
        let mut gen = DocGenerator::new();

        let mut doc = DocComment::new();
        doc.description = "Test".to_string();
        gen.add_function("test".to_string(), doc);

        let files = gen.generate_all();
        assert!(files.contains_key("index.md"));
        assert!(files.contains_key("test.md"));
    }
}
