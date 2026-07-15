//! Matter Language Server Protocol
//! Provides completion, diagnostics, and hover information

use matter_lexer::Lexer;
use matter_parser::Parser;
use matter_bytecode::BytecodeBuilder;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use std::collections::HashMap;
use std::sync::Mutex;

struct MatterLanguageServer {
    client: Client,
    document_map: Mutex<HashMap<String, String>>,
}

const KEYWORDS: &[&str] = &[
    "let", "set", "fn", "return", "if", "else", "on", "print",
    "while", "for", "in", "loop", "break", "continue", "struct",
    "import", "from", "as", "export", "match", "null", "spawn",
    "ok", "err", "some", "none", "panic", "true", "false",
    "and", "or", "not",
];

const BUILTINS: &[&str] = &[
    "math.abs", "math.min", "math.max", "math.pow", "math.sqrt",
    "math.sin", "math.cos", "math.tan", "math.pi", "math.e",
    "math.floor", "math.ceil", "math.round", "math.log",
    "string.len", "string.upper", "string.lower", "string.trim",
    "string.split", "string.join", "string.contains", "string.replace",
    "string.startswith", "string.endswith", "string.substring",
    "list.sort", "list.reverse", "list.sum", "list.push", "list.pop",
    "list.len", "list.slice", "list.concat", "list.contains",
    "json.stringify", "json.parse",
    "time.now", "time.sleep",
    "random.int", "random.bool", "random.choice",
    "file.read", "file.write", "file.lines",
    "result.ok", "result.err", "result.is_ok", "result.is_err",
    "result.unwrap", "result.unwrap_or", "result.map",
    "option.some", "option.none", "option.is_some", "option.is_none",
    "option.unwrap", "option.unwrap_or",
];

#[tower_lsp::async_trait]
impl LanguageServer for MatterLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "matter-lsp".to_string(),
                version: Some("0.5.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), '"'.to_string()]),
                    ..CompletionOptions::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        inter_file_dependencies: true,
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
            .log_message(MessageType::INFO, "Matter LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
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
        let prefix = &line[..position.character as usize];

        let mut items = Vec::new();

        // If after a dot, suggest methods
        if let Some(dot_pos) = prefix.rfind('.') {
            let target = &prefix[..dot_pos];
            let method_prefix = &prefix[dot_pos + 1..];
            items.extend(self.completions_for_target(target, method_prefix));
        } else {
            // Suggest keywords
            let word_prefix = prefix.split_whitespace().last().unwrap_or("");
            for kw in KEYWORDS {
                if kw.starts_with(word_prefix) {
                    items.push(CompletionItem::new_simple(
                        kw.to_string(),
                        "keyword".to_string(),
                    ));
                }
            }
            // Suggest builtins
            for builtin in BUILTINS {
                let name = builtin.split('.').last().unwrap_or(builtin);
                if name.starts_with(word_prefix) || builtin.starts_with(word_prefix) {
                    items.push(CompletionItem {
                        label: builtin.to_string(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some("builtin".to_string()),
                        documentation: Some(documentation_from_string(
                            format!("Built-in function: {}", builtin),
                        )),
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
            // Check if it's a keyword
            if let Some(doc_text) = keyword_documentation(&word) {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc_text,
                    }),
                    range: None,
                }));
            }

            // Check if it's a builtin
            for builtin in BUILTINS {
                if builtin.ends_with(&word) || *builtin == word {
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!(
                                "```matter\n{}\n```\n\nBuilt-in function",
                                builtin
                            ),
                        }),
                        range: None,
                    }));
                }
            }

            // Try to find function definition in source
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
            // Search for function/variable definition
            for (i, src_line) in source.lines().enumerate() {
                let trimmed = src_line.trim();
                if trimmed.starts_with("fn ") && trimmed.contains(&word) {
                    let start = trimmed.find(&word).unwrap_or(0);
                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position::new(i as u32, start as u32),
                            end: Position::new(i as u32, (start + word.len()) as u32),
                        },
                    })));
                }
                if trimmed.starts_with("let ") && trimmed.contains(&word) {
                    let start = trimmed.find(&word).unwrap_or(0);
                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position::new(i as u32, start as u32),
                            end: Position::new(i as u32, (start + word.len()) as u32),
                        },
                    })));
                }
            }
        }

        Ok(None)
    }
}

impl MatterLanguageServer {
    fn completions_for_target(&self, target: &str, prefix: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let methods: &[(&str, &str)] = match target {
            "math" => &[
                ("abs", "abs(n) -> absolute value"),
                ("sqrt", "sqrt(n) -> square root"),
                ("sin", "sin(n) -> sine"),
                ("cos", "cos(n) -> cosine"),
                ("pi", "pi -> constant"),
            ],
            "string" => &[
                ("len", "len(s) -> length"),
                ("upper", "upper(s) -> uppercase"),
                ("lower", "lower(s) -> lowercase"),
                ("trim", "trim(s) -> trimmed"),
                ("split", "split(s, sep) -> list"),
                ("contains", "contains(s, sub) -> bool"),
            ],
            "list" => &[
                ("sort", "sort(l) -> sorted list"),
                ("push", "push(l, item) -> append"),
                ("pop", "pop(l) -> last item"),
                ("len", "len(l) -> length"),
                ("reverse", "reverse(l) -> reversed"),
            ],
            "json" => &[
                ("stringify", "stringify(v) -> JSON string"),
                ("parse", "parse(s) -> parsed value"),
            ],
            "result" => &[
                ("ok", "ok(v) -> Ok result"),
                ("err", "err(e) -> Err result"),
                ("is_ok", "is_ok(r) -> bool"),
                ("unwrap", "unwrap(r) -> value"),
            ],
            "option" => &[
                ("some", "some(v) -> Some value"),
                ("none", "none() -> None"),
                ("is_some", "is_some(o) -> bool"),
                ("unwrap", "unwrap(o) -> value"),
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

    async fn publish_diagnostics(&self, uri: Url, source: String) {
        let mut diagnostics = Vec::new();

        // Parse and collect errors
        let mut parser = Parser::from_source(&source);
        match parser.parse() {
            Ok(program) => {
                // Try to compile
                let builder = BytecodeBuilder::new();
                if let Err(e) = builder.build_checked(&program) {
                    diagnostics.push(Diagnostic {
                        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("compile-error".to_string())),
                        source: Some("matter-lsp".to_string()),
                        message: format!("{:?}", e),
                        ..Diagnostic::default()
                    });
                }
            }
            Err(e) => {
                let line = (e.line as u32).saturating_sub(1);
                let col = (e.column as u32).saturating_sub(1);
                diagnostics.push(Diagnostic {
                    range: Range::new(
                        Position::new(line, col),
                        Position::new(line, col + 10),
                    ),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("parse-error".to_string())),
                    source: Some("matter-lsp".to_string()),
                    message: e.message.clone(),
                    ..Diagnostic::default()
                });
            }
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

fn extract_word_at_position(line: &str, character: usize) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    if character >= chars.len() {
        return None;
    }

    let mut start = character;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }

    let mut end = character;
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
        "let" => Some("```matter\nlet name = value\n```\n\nDeclare an immutable variable.".to_string()),
        "set" => Some("```matter\nset name = value\n```\n\nReassign a mutable variable.".to_string()),
        "fn" => Some("```matter\nfn name(params) { body }\n```\n\nDefine a function.".to_string()),
        "if" => Some("```matter\nif condition { body } else { body }\n```\n\nConditional branch.".to_string()),
        "while" => Some("```matter\nwhile condition { body }\n```\n\nLoop while condition is true.".to_string()),
        "for" => Some("```matter\nfor item in iterable { body }\n```\n\nIterate over a list.".to_string()),
        "loop" => Some("```matter\nloop { body }\n```\n\nInfinite loop (use break to exit).".to_string()),
        "return" => Some("```matter\nreturn value\n```\n\nReturn from current function.".to_string()),
        "import" => Some("```matter\nimport \"path\"\nimport { name } from \"path\"\nimport \"path\" as alias\n```\n\nImport a module.".to_string()),
        "export" => Some("```matter\nexport { name1, name2 }\n```\n\nExport names from a module.".to_string()),
        "struct" => Some("```matter\nstruct Name { field: type }\n```\n\nDefine a struct type.".to_string()),
        "match" => Some("```matter\nmatch value { pattern => { body } }\n```\n\nPattern matching.".to_string()),
        "ok" => Some("```matter\nok(value)\n```\n\nCreate an Ok Result value.".to_string()),
        "err" => Some("```matter\nerr(error)\n```\n\nCreate an Err Result value.".to_string()),
        "some" => Some("```matter\nsome(value)\n```\n\nCreate a Some Option value.".to_string()),
        "none" => Some("```matter\nnone\n```\n\nThe None Option value.".to_string()),
        "true" | "false" => Some("Boolean literal.".to_string()),
        "null" => Some("Null literal.".to_string()),
        "and" | "or" | "not" => Some("Logical operator (word form).".to_string()),
        _ => None,
    }
}

fn find_definition_hover(source: &str, word: &str) -> Option<String> {
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("fn ") {
            if let Some(pos) = trimmed.find(word) {
                let after = &trimmed[pos..];
                if after.starts_with(word) && (after.len() <= word.len() || !after.as_bytes()[word.len()].is_ascii_alphanumeric()) {
                    return Some(format!("```matter\n{}\n```\n\nFunction defined in this file.", trimmed));
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
