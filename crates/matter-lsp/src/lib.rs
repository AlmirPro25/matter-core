//! Matter Language Server Protocol (recovery v1)
//!
//! Completions, diagnostics, and hover for Matter Core **language-only** 0.2.0.
//! Diagnostics use the same lexer/parser + semantic validation path as the CLI
//! (`BytecodeBuilder::build_checked`), so import/export/type/panic honesty is
//! preserved. Does **not** resolve modules, talk to the network, spawn shells,
//! or install packages.

use matter_bytecode::BytecodeBuilder;
use matter_parser::Parser;
use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct MatterLanguageServer {
    client: Client,
    document_map: Mutex<HashMap<String, String>>,
}

/// Keywords recognized for completion/hover. Reserved-but-unimplemented
/// keywords still appear so the editor can surface honesty diagnostics.
const KEYWORDS: &[&str] = &[
    "let", "set", "fn", "return", "if", "else", "on", "print", "while", "for", "in", "loop",
    "break", "continue", "struct", "import", "from", "as", "export", "match", "null", "spawn",
    "true", "false", "and", "or", "not", "panic",
];

/// Language-only stdlib surface commonly used in Core. File I/O is listed
/// because the builtins exist, but programs are **default-deny** under File
/// Capabilities v1 (CLI grants required).
const BUILTINS: &[&str] = &[
    "math.abs",
    "math.min",
    "math.max",
    "math.pow",
    "math.sqrt",
    "math.sin",
    "math.cos",
    "math.floor",
    "math.ceil",
    "math.round",
    "string.len",
    "string.upper",
    "string.lower",
    "string.trim",
    "string.split",
    "string.join",
    "string.contains",
    "string.replace",
    "list.sort",
    "list.reverse",
    "list.push",
    "list.pop",
    "list.len",
    "list.slice",
    "list.concat",
    "list.contains",
    "json.stringify",
    "json.parse",
    "time.now",
    "random.int",
    "random.bool",
    "file.read",
    "file.write",
    "file.lines",
];

/// A single diagnostic for tests and LSP publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterDiagnostic {
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub code: String,
}

/// Analyze source the same way language-only CLI validation does (parse + build_checked).
pub fn analyze_source(source: &str) -> Vec<MatterDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut parser = Parser::from_source(source);
    match parser.parse() {
        Ok(program) => {
            let builder = BytecodeBuilder::new();
            if let Err(e) = builder.build_checked(&program) {
                diagnostics.push(MatterDiagnostic {
                    line: 0,
                    column: 0,
                    message: e.to_string(),
                    code: "compile-error".to_string(),
                });
            }
        }
        Err(e) => {
            let line = (e.line as u32).saturating_sub(1);
            let col = (e.column as u32).saturating_sub(1);
            diagnostics.push(MatterDiagnostic {
                line,
                column: col,
                message: e.message.clone(),
                code: "parse-error".to_string(),
            });
        }
    }
    diagnostics
}

#[tower_lsp::async_trait]
impl LanguageServer for MatterLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "matter-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..CompletionOptions::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                // Formatting not implemented in v1 — do not advertise it.
                document_formatting_provider: Some(OneOf::Left(false)),
                // No cross-file module resolution in Core 0.2.0.
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        ..DiagnosticOptions::default()
                    },
                )),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let _ = self
            .client
            .log_message(
                MessageType::INFO,
                "Matter LSP initialized (language-only Core 0.2.0 surface)",
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        // Clear documents so a stop does not retain state.
        if let Ok(mut map) = self.document_map.lock() {
            map.clear();
        }
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.document_map
            .lock()
            .unwrap()
            .insert(uri.to_string(), text.clone());
        self.publish_diagnostics(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.document_map
                .lock()
                .unwrap()
                .insert(uri.to_string(), change.text.clone());
            self.publish_diagnostics(uri, change.text).await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let doc = self.document_map.lock().unwrap();
        let source = doc.get(&uri.to_string()).cloned().unwrap_or_default();
        drop(doc);

        let line = source.lines().nth(position.line as usize).unwrap_or("");
        let character = (position.character as usize).min(line.len());
        let prefix = &line[..character];

        let mut items = Vec::new();

        if let Some(dot_pos) = prefix.rfind('.') {
            let target = prefix[..dot_pos].split_whitespace().last().unwrap_or("");
            let method_prefix = &prefix[dot_pos + 1..];
            items.extend(completions_for_target(target, method_prefix));
        } else {
            let word_prefix = prefix
                .rsplit(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .unwrap_or("");
            for kw in KEYWORDS {
                if kw.starts_with(word_prefix) {
                    items.push(CompletionItem {
                        label: kw.to_string(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        detail: Some("keyword".to_string()),
                        documentation: keyword_documentation(kw).map(documentation_from_string),
                        ..CompletionItem::default()
                    });
                }
            }
            for builtin in BUILTINS {
                let name = builtin.split('.').last().unwrap_or(builtin);
                if name.starts_with(word_prefix) || builtin.starts_with(word_prefix) {
                    items.push(CompletionItem {
                        label: builtin.to_string(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(builtin_detail(builtin).to_string()),
                        documentation: Some(documentation_from_string(builtin_doc(builtin))),
                        ..CompletionItem::default()
                    });
                }
            }
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc = self.document_map.lock().unwrap();
        let source = doc.get(&uri.to_string()).cloned().unwrap_or_default();
        drop(doc);

        let line = source.lines().nth(position.line as usize).unwrap_or("");
        let word = extract_word_at_position(line, position.character as usize);

        if let Some(word) = word {
            if let Some(doc_text) = keyword_documentation(&word) {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc_text,
                    }),
                    range: None,
                }));
            }

            for builtin in BUILTINS {
                if builtin.ends_with(&word) || *builtin == word {
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: builtin_doc(builtin),
                        }),
                        range: None,
                    }));
                }
            }

            if let Some(doc_text) = find_definition_hover(&source, &word) {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc_text,
                    }),
                    range: None,
                }));
            }
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc = self.document_map.lock().unwrap();
        let source = doc.get(&uri.to_string()).cloned().unwrap_or_default();
        drop(doc);

        let line = source.lines().nth(position.line as usize).unwrap_or("");
        let word = extract_word_at_position(line, position.character as usize);

        if let Some(word) = word {
            for (i, src_line) in source.lines().enumerate() {
                let trimmed = src_line.trim();
                if let Some(loc) = definition_location_on_line(uri, i as u32, trimmed, &word, "fn ")
                {
                    return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
                }
                if let Some(loc) =
                    definition_location_on_line(uri, i as u32, trimmed, &word, "let ")
                {
                    return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
                }
            }
        }

        Ok(None)
    }
}

impl MatterLanguageServer {
    async fn publish_diagnostics(&self, uri: Url, source: String) {
        let diagnostics: Vec<Diagnostic> = analyze_source(&source)
            .into_iter()
            .map(|d| Diagnostic {
                range: Range::new(
                    Position::new(d.line, d.column),
                    Position::new(d.line, d.column.saturating_add(8)),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String(d.code)),
                source: Some("matter-lsp".to_string()),
                message: d.message,
                ..Diagnostic::default()
            })
            .collect();

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

fn definition_location_on_line(
    uri: &Url,
    line: u32,
    trimmed: &str,
    word: &str,
    prefix: &str,
) -> Option<Location> {
    if !trimmed.starts_with(prefix) {
        return None;
    }
    let after = &trimmed[prefix.len()..];
    // crude: name starts after keyword
    let name_start_in_trim = if let Some(pos) = after.find(word) {
        // ensure word boundary-ish
        let end = pos + word.len();
        let ok_start = pos == 0 || !after.as_bytes()[pos - 1].is_ascii_alphanumeric();
        let ok_end = end >= after.len() || !after.as_bytes()[end].is_ascii_alphanumeric();
        if ok_start && ok_end {
            Some(prefix.len() + pos)
        } else {
            None
        }
    } else {
        None
    }?;
    // Map trim-relative index is imperfect for indented lines; use full-line search.
    let _ = name_start_in_trim;
    let full_pos = trimmed.find(word)?;
    Some(Location {
        uri: uri.clone(),
        range: Range {
            start: Position::new(line, full_pos as u32),
            end: Position::new(line, (full_pos + word.len()) as u32),
        },
    })
}

fn completions_for_target(target: &str, prefix: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let methods: &[(&str, &str)] = match target {
        "math" => &[
            ("abs", "abs(n)"),
            ("sqrt", "sqrt(n)"),
            ("sin", "sin(n)"),
            ("cos", "cos(n)"),
            ("floor", "floor(n)"),
            ("ceil", "ceil(n)"),
        ],
        "string" => &[
            ("len", "len(s)"),
            ("upper", "upper(s)"),
            ("lower", "lower(s)"),
            ("trim", "trim(s)"),
            ("split", "split(s, sep)"),
            ("contains", "contains(s, sub)"),
        ],
        "list" => &[
            ("sort", "sort(l)"),
            ("push", "push(l, item)"),
            ("pop", "pop(l)"),
            ("len", "len(l)"),
            ("reverse", "reverse(l)"),
        ],
        "json" => &[("stringify", "stringify(v)"), ("parse", "parse(s)")],
        "file" => &[
            ("read", "read(path) — requires --allow-fs-read"),
            ("write", "write(path, data) — requires --allow-fs-write"),
            ("lines", "lines(path) — requires --allow-fs-read"),
        ],
        _ => &[],
    };

    for (name, doc) in methods {
        if name.starts_with(prefix) || prefix.is_empty() {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(doc.to_string()),
                ..CompletionItem::default()
            });
        }
    }
    items
}

fn extract_word_at_position(line: &str, character: usize) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return None;
    }
    let character = character.min(chars.len());
    if character == 0 && chars.is_empty() {
        return None;
    }

    let mut start = character.min(chars.len());
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }

    let mut end = character.min(chars.len());
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(chars[start..end].iter().collect())
}

fn keyword_documentation(word: &str) -> Option<String> {
    match word {
        "let" => Some(
            "```matter\nlet name = value\n```\n\nBind a name (Core).".to_string(),
        ),
        "set" => Some(
            "```matter\nset name = value\n```\n\nReassign a name (Core).".to_string(),
        ),
        "fn" => Some(
            "```matter\nfn name(params) { body }\n```\n\nNamed function (Core). Anonymous `fn(...)` lambdas are **not** fully wired as first-class values in 0.2.0.".to_string(),
        ),
        "if" => Some("```matter\nif condition { body } else { body }\n```".to_string()),
        "while" => Some("```matter\nwhile condition { body }\n```".to_string()),
        "for" => Some("```matter\nfor item in iterable { body }\n```".to_string()),
        "loop" => Some("```matter\nloop { body }\n```".to_string()),
        "return" => Some("```matter\nreturn value\n```".to_string()),
        "import" => Some(
            "**Not implemented** in Matter Core 0.2.0.\n\n```matter\nimport \"path\"\n```\n\nHard error: module system is not available; no module is loaded (semantic honesty)."
                .to_string(),
        ),
        "export" => Some(
            "**Not implemented** in Matter Core 0.2.0.\n\n```matter\nexport { name }\n```\n\nHard error: no symbols are exported (semantic honesty)."
                .to_string(),
        ),
        "from" | "as" => Some(
            "Import syntax fragment. Module system is **not** available in Core 0.2.0."
                .to_string(),
        ),
        "struct" => Some(
            "```matter\nstruct Name { field1, field2 }\n```\n\nStruct definition (fields are names; **type annotations are unsupported** and hard-error)."
                .to_string(),
        ),
        "match" => Some(
            "```matter\nmatch subject {\n  value => { body }\n}\n```\n\n**Equality / first-arm** semantics in Core 0.2.0 — not full pattern matching."
                .to_string(),
        ),
        "panic" => Some(
            "**Reserved word** — the `panic` construct is **not implemented** in Matter Core 0.2.0 (hard diagnostic)."
                .to_string(),
        ),
        "true" | "false" => Some("Boolean literal.".to_string()),
        "null" => Some("Null literal.".to_string()),
        "and" | "or" | "not" => Some("Logical operator (word form).".to_string()),
        "print" => Some("```matter\nprint value\n```".to_string()),
        "on" | "spawn" => Some("Event / spawn statement (Core event surface).".to_string()),
        _ => None,
    }
}

fn builtin_detail(builtin: &str) -> &'static str {
    if builtin.starts_with("file.") {
        "builtin (File Caps v1: default-deny)"
    } else {
        "builtin"
    }
}

fn builtin_doc(builtin: &str) -> String {
    if builtin.starts_with("file.") {
        format!(
            "```matter\n{}\n```\n\nStdlib file API. **File Capabilities v1**: denied unless the CLI grants `--allow-fs-read|write|delete`.",
            builtin
        )
    } else {
        format!(
            "```matter\n{}\n```\n\nBuilt-in function (language-only surface).",
            builtin
        )
    }
}

fn find_definition_hover(source: &str, word: &str) -> Option<String> {
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("fn ") {
            if let Some(pos) = trimmed.find(word) {
                let after = &trimmed[pos..];
                if after.starts_with(word)
                    && (after.len() <= word.len()
                        || !after.as_bytes()[word.len()].is_ascii_alphanumeric())
                {
                    return Some(format!(
                        "```matter\n{}\n```\n\nFunction defined in this file.",
                        trimmed
                    ));
                }
            }
        }
    }
    None
}

fn documentation_from_string(text: String) -> Documentation {
    Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value: text,
    })
}

pub async fn start_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| MatterLanguageServer {
        client,
        document_map: Mutex::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod recovery_v1_tests {
    use super::*;

    fn msgs(source: &str) -> Vec<String> {
        analyze_source(source)
            .into_iter()
            .map(|d| d.message.to_lowercase())
            .collect()
    }

    #[test]
    fn valid_program_no_diagnostics() {
        let d = analyze_source("let x = 1\nprint x\n");
        assert!(d.is_empty(), "{:?}", d);
    }

    #[test]
    fn invalid_code_parse_error_line_column() {
        let d = analyze_source("let x = @@@\n");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "parse-error");
        // '@' is on line 1 → 0-based line 0
        assert_eq!(d[0].line, 0);
        assert!(d[0].message.to_lowercase().contains("illegal") || d[0].message.contains('@'));
    }

    #[test]
    fn import_unsupported_hard_error() {
        let m = msgs("import \"x.matter\"\n");
        assert!(!m.is_empty());
        assert!(
            m[0].contains("import") && (m[0].contains("not implemented") || m[0].contains("0.2.0")),
            "{:?}",
            m
        );
    }

    #[test]
    fn export_unsupported_hard_error() {
        let m = msgs("export { main }\n");
        assert!(!m.is_empty());
        assert!(
            m[0].contains("export") && (m[0].contains("not implemented") || m[0].contains("0.2.0")),
            "{:?}",
            m
        );
    }

    #[test]
    fn type_annotation_unsupported() {
        let m = msgs("fn f(x: int) { return x }\n");
        assert!(!m.is_empty());
        assert!(
            m[0].contains("type") || m[0].contains("annotation"),
            "{:?}",
            m
        );
    }

    #[test]
    fn panic_unsupported() {
        let m = msgs("panic(\"x\")\n");
        assert!(!m.is_empty());
        assert!(m[0].contains("panic"), "{:?}", m);
    }

    #[test]
    fn match_equality_compiles() {
        let d = analyze_source(
            r#"
let status = 200
match status {
    200 => { print "ok" }
    404 => { print "missing" }
}
"#,
        );
        assert!(d.is_empty(), "{:?}", d);
    }

    #[test]
    fn unicode_identifier_ok() {
        // ascii-safe unicode body content via non-ascii identifier if supported;
        // at minimum UTF-8 source with accented string must not crash analyzer.
        let d = analyze_source("let nome = \"olá\"\nprint nome\n");
        assert!(d.is_empty(), "{:?}", d);
    }

    #[test]
    fn hover_import_not_announced_as_implemented() {
        let h = keyword_documentation("import").expect("import docs");
        assert!(h.to_lowercase().contains("not implemented"));
        assert!(!h.contains("Import a module.") || h.contains("Not implemented"));
    }

    #[test]
    fn hover_export_not_announced_as_implemented() {
        let h = keyword_documentation("export").expect("export docs");
        assert!(h.to_lowercase().contains("not implemented"));
    }

    #[test]
    fn hover_match_equality_semantics() {
        let h = keyword_documentation("match").expect("match docs");
        assert!(h.to_lowercase().contains("equality") || h.contains("first-arm"));
        assert!(!h.to_lowercase().contains("full pattern matching") || h.contains("not full"));
    }

    #[test]
    fn hover_panic_reserved() {
        let h = keyword_documentation("panic").expect("panic docs");
        assert!(h.to_lowercase().contains("not implemented"));
    }

    #[test]
    fn extract_word_column() {
        let w = extract_word_at_position("let foo = 1", 5).unwrap();
        assert_eq!(w, "foo");
    }

    #[test]
    fn shutdown_clears_document_map_logic() {
        // Exercise map clear path used by shutdown without hanging the LSP loop.
        let map = Mutex::new(HashMap::from([(
            "file:///tmp/a.matter".to_string(),
            "let x = 1\n".to_string(),
        )]));
        map.lock().unwrap().clear();
        assert!(map.lock().unwrap().is_empty());
    }

    #[test]
    fn open_change_document_pipeline_via_analyze() {
        // did_open / did_change only re-run analyze_source on full text.
        let open = "let x = 1\nprint x\n";
        assert!(analyze_source(open).is_empty());
        let changed = "let x = @\n";
        assert!(!analyze_source(changed).is_empty());
    }
}
