/// Matter CLI
/// Interface de linha de comando para Matter

use matter_ast::Program;
use matter_bytecode::{Bytecode, BytecodeBuilder, SemanticError};
use matter_lexer::{Lexer, Token};
use matter_parser::{ParseError, Parser};
use matter_runtime::Runtime;
use std::env;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }
    
    let command = &args[1];
    
    match command.as_str() {
        "capabilities-json" => {
            print_capabilities_json();
        }

        "run" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli run <file.matter|->");
                process::exit(1);
            }
            run_file(&args[2]);
        }

        "eval" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli eval <source>");
                process::exit(1);
            }
            eval_source(&args[2]);
        }

        "eval-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli eval-json <source>");
                process::exit(1);
            }
            eval_json(&args[2]);
        }

        "run-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli run-json <file.matter|->");
                process::exit(1);
            }
            run_json(&args[2]);
        }
        
        "emit" => {
            if args.len() < 4 {
                eprintln!("Usage: matter-cli emit <file.matter|-> <event>");
                process::exit(1);
            }
            emit_event(&args[2], &args[3]);
        }

        "emit-json" => {
            if args.len() < 4 {
                eprintln!("Usage: matter-cli emit-json <file.matter|-> <event>");
                process::exit(1);
            }
            emit_json(&args[2], &args[3]);
        }

        "check" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli check <file.matter|->");
                process::exit(1);
            }
            check_file(&args[2]);
        }

        "tokens-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli tokens-json <file.matter|->");
                process::exit(1);
            }
            tokens_json(&args[2]);
        }

        "imports-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli imports-json <file.matter|->");
                process::exit(1);
            }
            imports_json(&args[2]);
        }

        "check-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli check-json <file.matter|->");
                process::exit(1);
            }
            check_json(&args[2]);
        }
        
        "compile" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli compile <file.matter|-> [-o output.mbc]");
                process::exit(1);
            }
            
            let output = if args.len() >= 5 && args[3] == "-o" {
                &args[4]
            } else {
                "output.mbc"
            };
            
            compile_file(&args[2], output);
        }

        "compile-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli compile-json <file.matter|-> [-o output.mbc]");
                process::exit(1);
            }

            let output = if args.len() >= 5 && args[3] == "-o" {
                &args[4]
            } else {
                "output.mbc"
            };

            compile_json(&args[2], output);
        }
        
        "run-bytecode" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli run-bytecode <file.mbc>");
                process::exit(1);
            }
            run_bytecode(&args[2]);
        }

        "run-bytecode-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli run-bytecode-json <file.mbc>");
                process::exit(1);
            }
            run_bytecode_json(&args[2]);
        }

        "emit-bytecode" => {
            if args.len() < 4 {
                eprintln!("Usage: matter-cli emit-bytecode <file.mbc> <event>");
                process::exit(1);
            }
            emit_bytecode(&args[2], &args[3]);
        }

        "emit-bytecode-json" => {
            if args.len() < 4 {
                eprintln!("Usage: matter-cli emit-bytecode-json <file.mbc> <event>");
                process::exit(1);
            }
            emit_bytecode_json(&args[2], &args[3]);
        }

        "inspect" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli inspect <file.mbc>");
                process::exit(1);
            }
            inspect_bytecode(&args[2]);
        }

        "inspect-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli inspect-json <file.mbc>");
                process::exit(1);
            }
            inspect_json(&args[2]);
        }
        
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Matter CLI - Matter Core Language Runtime");
    println!();
    println!("Usage:");
    println!("  matter-cli capabilities-json                Print machine-readable capabilities");
    println!("  matter-cli run <file.matter|->              Run Matter source file or stdin");
    println!("  matter-cli eval <source>                    Run Matter source passed as text");
    println!("  matter-cli eval-json <source>               Run source text and print JSON result");
    println!("  matter-cli run-json <file.matter|->         Run source and print JSON result");
    println!("  matter-cli emit <file.matter|-> <event>     Emit event in Matter program");
    println!("  matter-cli emit-json <file.matter|-> <event> Emit event and print JSON result");
    println!("  matter-cli check <file.matter|->            Parse and compile without running");
    println!("  matter-cli tokens-json <file.matter|->      Tokenize source and print JSON");
    println!("  matter-cli imports-json <file.matter|->     Inspect local imports as JSON");
    println!("  matter-cli check-json <file.matter|->       Validate source and print JSON");
    println!("  matter-cli compile <file.matter|-> [-o out] Compile to bytecode (.mbc)");
    println!("  matter-cli compile-json <file.matter|-> [-o out] Compile and print JSON");
    println!("  matter-cli run-bytecode <file.mbc>          Run bytecode file");
    println!("  matter-cli run-bytecode-json <file.mbc>     Run bytecode and print JSON result");
    println!("  matter-cli emit-bytecode <file.mbc> <event> Emit event from bytecode");
    println!("  matter-cli emit-bytecode-json <file.mbc> <event> Emit bytecode event as JSON");
    println!("  matter-cli inspect <file.mbc>               Inspect bytecode file");
    println!("  matter-cli inspect-json <file.mbc>          Inspect bytecode and print JSON");
    println!();
    println!("Use '-' as the input path to read Matter source from stdin.");
}

fn print_capabilities_json() {
    println!(
        concat!(
            "{{",
            "\"ok\":true,",
            "\"name\":\"matter-cli\",",
            "\"version\":\"{}\",",
            "\"bytecode\":\"MBC1\",",
            "\"stdin\":true,",
            "\"json_commands\":[",
            "\"capabilities-json\",",
            "\"eval-json\",",
            "\"tokens-json\",",
            "\"imports-json\",",
            "\"check-json\",",
            "\"run-json\",",
            "\"emit-json\",",
            "\"compile-json\",",
            "\"inspect-json\",",
            "\"run-bytecode-json\",",
            "\"emit-bytecode-json\"",
            "],",
            "\"source_commands\":[",
            "\"run\",",
            "\"eval\",",
            "\"emit\",",
            "\"check\",",
            "\"compile\"",
            "],",
            "\"bytecode_commands\":[",
            "\"run-bytecode\",",
            "\"emit-bytecode\",",
            "\"inspect\"",
            "],",
            "\"language_features\":[",
            "\"variables\",",
            "\"functions\",",
            "\"recursion\",",
            "\"if\",",
            "\"while\",",
            "\"loop\",",
            "\"for\",",
            "\"break\",",
            "\"continue\",",
            "\"events\",",
            "\"lists\",",
            "\"maps\",",
            "\"structs\",",
            "\"backend_calls\",",
            "\"imports\",",
            "\"stdlib\",",
            "\"persistence\",",
            "\"network\"",
            "]",
            "}}"
        ),
        env!("CARGO_PKG_VERSION")
    );
}

fn run_file(path: &str) {
    let source = read_source_or_exit(path);
    run_source(&source);
}

fn eval_source(source: &str) {
    run_source(source);
}

fn eval_json(source: &str) {
    run_source_json(source, "<eval>");
}

fn run_source(source: &str) {
    let mut parser = Parser::from_source(&source);
    let program = parser.parse().unwrap_or_else(|e| {
        print_parse_error(&source, &e);
        process::exit(1);
    });
    
    let builder = BytecodeBuilder::new();
    let bytecode = build_checked_or_exit(builder, &program);
    
    let mut runtime = Runtime::new(bytecode);
    
    if let Err(e) = runtime.run() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

fn run_json(path: &str) {
    let source = read_source_or_exit(path);
    run_source_json(&source, source_label(path));
}

fn run_source_json(source: &str, input: &str) {
    let bytecode = build_json_or_exit(source, input, &[]);

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.run() {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
            json_escape(input),
            json_escape(&error),
            json_string_array(&runtime.take_output())
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"output\":{}}}",
        json_escape(input),
        json_string_array(&runtime.take_output())
    );
}

fn emit_event(path: &str, event: &str) {
    let source = read_source_or_exit(path);
    let mut parser = Parser::from_source(&source);
    let program = parser.parse().unwrap_or_else(|e| {
        print_parse_error(&source, &e);
        process::exit(1);
    });
    
    let builder = BytecodeBuilder::new();
    let bytecode = build_checked_or_exit(builder, &program);
    
    let mut runtime = Runtime::new(bytecode);
    
    if let Err(e) = runtime.emit_event(event) {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

fn emit_json(path: &str, event: &str) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let bytecode = build_json_or_exit(&source, input, &[("event", event)]);

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.emit_event(event) {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"input\":\"{}\",\"event\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
            json_escape(input),
            json_escape(event),
            json_escape(&error),
            json_string_array(&runtime.take_output())
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"event\":\"{}\",\"output\":{}}}",
        json_escape(input),
        json_escape(event),
        json_string_array(&runtime.take_output())
    );
}

fn check_file(path: &str) {
    let source = read_source_or_exit(path);

    let mut parser = Parser::from_source(&source);
    let program = parser.parse().unwrap_or_else(|e| {
        print_parse_error(&source, &e);
        process::exit(1);
    });

    let builder = BytecodeBuilder::new();
    let bytecode = build_checked_or_exit(builder, &program);

    println!("✓ Check passed");
    println!("  Input:          {}", source_label(path));
    println!("  Constants:      {}", bytecode.constants.len());
    println!("  Functions:      {}", bytecode.functions.len());
    println!("  Event handlers: {}", bytecode.event_handlers.len());
    println!("  Instructions:   {}", bytecode.main_instructions.len());
}

fn tokens_json(path: &str) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_spanned();
    let items: Vec<String> = tokens
        .iter()
        .enumerate()
        .map(|(index, spanned)| token_json(index, &spanned.token, spanned.span.line, spanned.span.column))
        .collect();

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"tokens\":[{}]}}",
        json_escape(input),
        items.join(",")
    );
}

fn imports_json(path: &str) {
    let (source, base_dir, mut stack) = if path == "-" {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source).unwrap_or_else(|e| {
            println!(
                "{{\"ok\":false,\"stage\":\"read\",\"input\":\"<stdin>\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&e.to_string())
            );
            process::exit(1);
        });
        (source, PathBuf::from("."), Vec::new())
    } else {
        let source = fs::read_to_string(path).unwrap_or_else(|e| {
            println!(
                "{{\"ok\":false,\"stage\":\"read\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&e.to_string())
            );
            process::exit(1);
        });
        let root = Path::new(path).canonicalize().unwrap_or_else(|_| PathBuf::from(path));
        let base = Path::new(path)
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        (source, base, vec![root])
    };

    let mut imports = Vec::new();
    if let Err(error) = collect_imports(&source, &base_dir, source_label(path), &mut stack, &mut imports) {
        println!(
            "{{\"ok\":false,\"stage\":\"import\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(source_label(path)),
            json_escape(&error)
        );
        process::exit(1);
    }

    let items: Vec<String> = imports
        .iter()
        .map(|import| {
            format!(
                "{{\"from\":\"{}\",\"path\":\"{}\",\"resolved\":\"{}\"}}",
                json_escape(&import.from),
                json_escape(&import.path),
                json_escape(&import.resolved)
            )
        })
        .collect();

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"count\":{},\"imports\":[{}]}}",
        json_escape(source_label(path)),
        imports.len(),
        items.join(",")
    );
}

fn check_json(path: &str) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let bytecode = build_json_or_exit(&source, input, &[]);

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"summary\":{}}}",
        json_escape(input),
        bytecode_summary_json(&bytecode)
    );
}

fn compile_file(input: &str, output: &str) {
    let source = read_source_or_exit(input);
    let mut parser = Parser::from_source(&source);
    let program = parser.parse().unwrap_or_else(|e| {
        print_parse_error(&source, &e);
        process::exit(1);
    });
    
    let builder = BytecodeBuilder::new();
    let bytecode = build_checked_or_exit(builder, &program);
    
    // Save to file
    if let Err(e) = bytecode.save_to_file(output) {
        eprintln!("Error writing bytecode to '{}': {}", output, e);
        process::exit(1);
    }
    
    println!("✓ Compiled successfully!");
    println!("  Input:  {}", source_label(input));
    println!("  Output: {}", output);
    println!();
    println!("  Constants:      {}", bytecode.constants.len());
    println!("  Functions:      {}", bytecode.functions.len());
    println!("  Event handlers: {}", bytecode.event_handlers.len());
    println!("  Instructions:   {}", bytecode.main_instructions.len());
}

fn compile_json(input: &str, output: &str) {
    let source = read_source_or_exit(input);
    let input_label = source_label(input);
    let bytecode = build_json_or_exit(&source, input_label, &[("output", output)]);

    if let Err(error) = bytecode.save_to_file(output) {
        println!(
            "{{\"ok\":false,\"stage\":\"write\",\"input\":\"{}\",\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(input_label),
            json_escape(output),
            json_escape(&error.to_string())
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"output\":\"{}\",\"summary\":{}}}",
        json_escape(input_label),
        json_escape(output),
        bytecode_summary_json(&bytecode)
    );
}

fn read_source_or_exit(path: &str) -> String {
    if path == "-" {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source).unwrap_or_else(|e| {
            eprintln!("Error reading Matter source from stdin: {}", e);
            process::exit(1);
        });
        resolve_imports_or_exit(&source, Path::new("."))
    } else {
        let source = fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Error reading file '{}': {}", path, e);
            process::exit(1);
        });
        let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
        resolve_imports_or_exit(&source, base_dir)
    }
}

fn resolve_imports_or_exit(source: &str, base_dir: &Path) -> String {
    let mut seen = HashSet::new();
    resolve_imports(source, base_dir, &mut seen).unwrap_or_else(|e| {
        eprintln!("Import error: {}", e);
        process::exit(1);
    })
}

fn resolve_imports(
    source: &str,
    base_dir: &Path,
    seen: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let mut resolved = String::new();

    for line in source.lines() {
        if let Some(import_path) = parse_import_line(line) {
            let canonical = resolve_import_path(&import_path, base_dir)?;

            if !seen.insert(canonical.clone()) {
                return Err(format!("circular import detected for '{}'", canonical.display()));
            }

            let imported_source = fs::read_to_string(&canonical).map_err(|e| {
                format!("could not read import '{}': {}", canonical.display(), e)
            })?;
            let imported_base = canonical.parent().unwrap_or(Path::new("."));
            resolved.push_str(&resolve_imports(&imported_source, imported_base, seen)?);
            resolved.push('\n');
            seen.remove(&canonical);
        } else {
            resolved.push_str(line);
            resolved.push('\n');
        }
    }

    Ok(resolved)
}

fn parse_import_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("import ")?;
    let rest = rest.trim();

    if !rest.starts_with('"') {
        return None;
    }

    let end = rest[1..].find('"')? + 1;
    let path = &rest[1..end];
    let trailing = rest[end + 1..].trim();

    if trailing.is_empty() || trailing.starts_with('#') {
        Some(path.to_string())
    } else {
        None
    }
}

fn resolve_import_path(import_path: &str, base_dir: &Path) -> Result<PathBuf, String> {
    let full_path = if is_std_import(import_path) {
        stdlib_root().join(strip_std_prefix(import_path))
    } else {
        base_dir.join(import_path)
    };

    full_path
        .canonicalize()
        .map_err(|e| format!("could not resolve import '{}': {}", full_path.display(), e))
}

fn is_std_import(import_path: &str) -> bool {
    import_path.starts_with("std/") || import_path.starts_with("std\\")
}

fn strip_std_prefix(import_path: &str) -> &str {
    import_path
        .strip_prefix("std/")
        .or_else(|| import_path.strip_prefix("std\\"))
        .unwrap_or(import_path)
}

fn stdlib_root() -> PathBuf {
    if let Ok(path) = env::var("MATTER_STDLIB_PATH") {
        return PathBuf::from(path);
    }

    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("stdlib")
}

fn source_label(path: &str) -> &str {
    if path == "-" {
        "<stdin>"
    } else {
        path
    }
}

struct ImportInfo {
    from: String,
    path: String,
    resolved: String,
}

fn collect_imports(
    source: &str,
    base_dir: &Path,
    from_label: &str,
    stack: &mut Vec<PathBuf>,
    imports: &mut Vec<ImportInfo>,
) -> Result<(), String> {
    for line in source.lines() {
        if let Some(import_path) = parse_import_line(line) {
            let canonical = resolve_import_path(&import_path, base_dir)?;

            if stack.iter().any(|path| path == &canonical) {
                return Err(format!("circular import detected for '{}'", canonical.display()));
            }

            let resolved = canonical.display().to_string();
            imports.push(ImportInfo {
                from: from_label.to_string(),
                path: import_path.clone(),
                resolved: resolved.clone(),
            });

            let imported_source = fs::read_to_string(&canonical).map_err(|e| {
                format!("could not read import '{}': {}", canonical.display(), e)
            })?;
            let imported_base = canonical
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf();

            stack.push(canonical);
            collect_imports(
                &imported_source,
                &imported_base,
                &resolved,
                stack,
                imports,
            )?;
            stack.pop();
        }
    }

    Ok(())
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn json_string_array(values: &[String]) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect();
    format!("[{}]", items.join(","))
}

fn token_json(index: usize, token: &Token, line: usize, column: usize) -> String {
    let (kind, value) = match token {
        Token::Let => ("let", None),
        Token::Set => ("set", None),
        Token::Fn => ("fn", None),
        Token::Return => ("return", None),
        Token::If => ("if", None),
        Token::Else => ("else", None),
        Token::On => ("on", None),
        Token::Print => ("print", None),
        Token::While => ("while", None),
        Token::For => ("for", None),
        Token::In => ("in", None),
        Token::Loop => ("loop", None),
        Token::Break => ("break", None),
        Token::Continue => ("continue", None),
        Token::Struct => ("struct", None),
        Token::Import => ("import", None),
        Token::Int(value) => ("int", Some(value.to_string())),
        Token::String(value) => ("string", Some(value.clone())),
        Token::Bool(value) => ("bool", Some(value.to_string())),
        Token::Ident(value) => ("ident", Some(value.clone())),
        Token::Plus => ("plus", None),
        Token::Minus => ("minus", None),
        Token::Star => ("star", None),
        Token::Slash => ("slash", None),
        Token::Eq => ("eq", None),
        Token::EqEq => ("eq_eq", None),
        Token::NotEq => ("not_eq", None),
        Token::Lt => ("lt", None),
        Token::Gt => ("gt", None),
        Token::LtEq => ("lt_eq", None),
        Token::GtEq => ("gt_eq", None),
        Token::LParen => ("lparen", None),
        Token::RParen => ("rparen", None),
        Token::LBrace => ("lbrace", None),
        Token::RBrace => ("rbrace", None),
        Token::LBracket => ("lbracket", None),
        Token::RBracket => ("rbracket", None),
        Token::Comma => ("comma", None),
        Token::Dot => ("dot", None),
        Token::Colon => ("colon", None),
        Token::Newline => ("newline", None),
        Token::Eof => ("eof", None),
    };

    let value_field = match value {
        Some(value) => format!(",\"value\":\"{}\"", json_escape(&value)),
        None => String::new(),
    };

    format!(
        "{{\"index\":{},\"kind\":\"{}\",\"line\":{},\"column\":{}{}}}",
        index,
        json_escape(kind),
        line,
        column,
        value_field
    )
}

fn json_field(name: &str, value: &str) -> String {
    format!("\"{}\":\"{}\"", json_escape(name), json_escape(value))
}

fn json_context(input: &str, extras: &[(&str, &str)]) -> String {
    let mut fields = vec![json_field("input", input)];
    for (name, value) in extras {
        fields.push(json_field(name, value));
    }
    fields.join(",")
}

fn bytecode_summary_json(bytecode: &Bytecode) -> String {
    format!(
        "{{\"constants\":{},\"functions\":{},\"event_handlers\":{},\"instructions\":{}}}",
        bytecode.constants.len(),
        bytecode.functions.len(),
        bytecode.event_handlers.len(),
        bytecode.main_instructions.len()
    )
}

fn print_parse_error_json(input: &str, extras: &[(&str, &str)], error: &ParseError) {
    println!(
        "{{\"ok\":false,\"stage\":\"parse\",{},\"error\":{{\"message\":\"{}\",\"line\":{},\"column\":{}}}}}",
        json_context(input, extras),
        json_escape(&error.to_string()),
        error.line,
        error.column
    );
}

fn print_semantic_error_json(input: &str, extras: &[(&str, &str)], error: &SemanticError) {
    println!(
        "{{\"ok\":false,\"stage\":\"semantic\",{},\"error\":{{\"message\":\"{}\"}}}}",
        json_context(input, extras),
        json_escape(&error.to_string())
    );
}

fn build_json_or_exit(source: &str, input: &str, extras: &[(&str, &str)]) -> Bytecode {
    let mut parser = Parser::from_source(source);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(error) => {
            print_parse_error_json(input, extras, &error);
            process::exit(1);
        }
    };

    let builder = BytecodeBuilder::new();
    match builder.build_checked(&program) {
        Ok(bytecode) => bytecode,
        Err(error) => {
            print_semantic_error_json(input, extras, &error);
            process::exit(1);
        }
    }
}

fn run_bytecode(path: &str) {
    use matter_bytecode::Bytecode;
    
    let bytecode = Bytecode::load_from_file(path).unwrap_or_else(|e| {
        eprintln!("Error loading bytecode from '{}': {}", path, e);
        process::exit(1);
    });
    
    let mut runtime = Runtime::new(bytecode);
    
    if let Err(e) = runtime.run() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

fn run_bytecode_json(path: &str) {
    use matter_bytecode::Bytecode;

    let bytecode = match Bytecode::load_from_file(path) {
        Ok(bytecode) => bytecode,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"load\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            );
            process::exit(1);
        }
    };

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.run() {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
            json_escape(path),
            json_escape(&error),
            json_string_array(&runtime.take_output())
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"output\":{}}}",
        json_escape(path),
        json_string_array(&runtime.take_output())
    );
}

fn emit_bytecode(path: &str, event: &str) {
    use matter_bytecode::Bytecode;

    let bytecode = Bytecode::load_from_file(path).unwrap_or_else(|e| {
        eprintln!("Error loading bytecode from '{}': {}", path, e);
        process::exit(1);
    });

    let mut runtime = Runtime::new(bytecode);

    if let Err(e) = runtime.emit_event(event) {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

fn emit_bytecode_json(path: &str, event: &str) {
    use matter_bytecode::Bytecode;

    let bytecode = match Bytecode::load_from_file(path) {
        Ok(bytecode) => bytecode,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"load\",\"input\":\"{}\",\"event\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(event),
                json_escape(&error.to_string())
            );
            process::exit(1);
        }
    };

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.emit_event(event) {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"input\":\"{}\",\"event\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
            json_escape(path),
            json_escape(event),
            json_escape(&error),
            json_string_array(&runtime.take_output())
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"event\":\"{}\",\"output\":{}}}",
        json_escape(path),
        json_escape(event),
        json_string_array(&runtime.take_output())
    );
}

fn inspect_json(path: &str) {
    use matter_bytecode::{Bytecode, Constant};

    let bytecode = match Bytecode::load_from_file(path) {
        Ok(bytecode) => bytecode,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"load\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            );
            process::exit(1);
        }
    };

    let mut functions: Vec<String> = bytecode
        .functions
        .iter()
        .map(|(name, function)| {
            format!(
                "{{\"name\":\"{}\",\"params\":{},\"instructions\":{}}}",
                json_escape(name),
                function.param_count,
                function.instructions.len()
            )
        })
        .collect();
    functions.sort();

    let mut events: Vec<String> = bytecode
        .event_handlers
        .iter()
        .map(|(name, handler)| {
            format!(
                "{{\"event\":\"{}\",\"instructions\":{}}}",
                json_escape(name),
                handler.instructions.len()
            )
        })
        .collect();
    events.sort();

    let constants: Vec<String> = bytecode
        .constants
        .iter()
        .enumerate()
        .map(|(index, constant)| match constant {
            Constant::Int(value) => format!(
                "{{\"index\":{},\"type\":\"int\",\"value\":{}}}",
                index, value
            ),
            Constant::Bool(value) => format!(
                "{{\"index\":{},\"type\":\"bool\",\"value\":{}}}",
                index, value
            ),
            Constant::String(value) => format!(
                "{{\"index\":{},\"type\":\"string\",\"value\":\"{}\"}}",
                index,
                json_escape(value)
            ),
            Constant::Unit => format!(
                "{{\"index\":{},\"type\":\"unit\",\"value\":null}}",
                index
            ),
        })
        .collect();

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"magic\":\"{}\",\"summary\":{{\"constants\":{},\"functions\":{},\"event_handlers\":{},\"instructions\":{}}},\"functions\":[{}],\"event_handlers\":[{}],\"constants\":[{}]}}",
        json_escape(path),
        json_escape(&String::from_utf8_lossy(&bytecode.magic)),
        bytecode.constants.len(),
        bytecode.functions.len(),
        bytecode.event_handlers.len(),
        bytecode.main_instructions.len(),
        functions.join(","),
        events.join(","),
        constants.join(",")
    );
}

fn inspect_bytecode(path: &str) {
    use matter_bytecode::{Bytecode, Constant};
    
    let bytecode = Bytecode::load_from_file(path).unwrap_or_else(|e| {
        eprintln!("Error loading bytecode from '{}': {}", path, e);
        process::exit(1);
    });
    
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║              MBC1 Bytecode Inspector                           ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("File: {}", path);
    println!("Magic: {}", String::from_utf8_lossy(&bytecode.magic));
    println!();
    
    println!("┌─ Summary ──────────────────────────────────────────────────────┐");
    println!("│ Constants:         {:>6}                                      │", bytecode.constants.len());
    println!("│ Functions:         {:>6}                                      │", bytecode.functions.len());
    println!("│ Event Handlers:    {:>6}                                      │", bytecode.event_handlers.len());
    println!("│ Main Instructions: {:>6}                                      │", bytecode.main_instructions.len());
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();
    
    if !bytecode.constants.is_empty() {
        println!("┌─ Constants Pool ───────────────────────────────────────────────┐");
        for (i, constant) in bytecode.constants.iter().enumerate() {
            print!("│ {:>4}: ", i);
            match constant {
                Constant::Int(n) => println!("{:<54} │", format!("Int({})", n)),
                Constant::Bool(b) => println!("{:<54} │", format!("Bool({})", b)),
                Constant::String(s) => {
                    let display = if s.len() > 45 {
                        format!("String(\"{}...\")", &s[..42])
                    } else {
                        format!("String(\"{}\")", s)
                    };
                    println!("{:<54} │", display)
                },
                Constant::Unit => println!("{:<54} │", "Unit"),
            }
        }
        println!("└────────────────────────────────────────────────────────────────┘");
        println!();
    }
    
    if !bytecode.functions.is_empty() {
        println!("┌─ Functions ────────────────────────────────────────────────────┐");
        for (name, func) in &bytecode.functions {
            println!("│ {} ({} params, {} instructions)", name, func.param_count, func.instructions.len());
            println!("│");
            for (i, instr) in func.instructions.iter().enumerate() {
                print_instruction(i, instr, &bytecode.constants, "│   ");
            }
            println!("│");
        }
        println!("└────────────────────────────────────────────────────────────────┘");
        println!();
    }
    
    if !bytecode.event_handlers.is_empty() {
        println!("┌─ Event Handlers ───────────────────────────────────────────────┐");
        for (event, handler) in &bytecode.event_handlers {
            println!("│ on {} ({} instructions)", event, handler.instructions.len());
            println!("│");
            for (i, instr) in handler.instructions.iter().enumerate() {
                print_instruction(i, instr, &bytecode.constants, "│   ");
            }
            println!("│");
        }
        println!("└────────────────────────────────────────────────────────────────┘");
        println!();
    }
    
    if !bytecode.main_instructions.is_empty() {
        println!("┌─ Main Instructions ────────────────────────────────────────────┐");
        for (i, instr) in bytecode.main_instructions.iter().enumerate() {
            print_instruction(i, instr, &bytecode.constants, "│ ");
        }
        println!("└────────────────────────────────────────────────────────────────┘");
    }
}

fn print_instruction(index: usize, instr: &matter_bytecode::Instruction, constants: &[matter_bytecode::Constant], prefix: &str) {
    use matter_bytecode::{Instruction, Constant};
    
    print!("{}{:>4}: ", prefix, index);
    
    match instr {
        Instruction::LoadConst(id) => {
            let const_val = match &constants[*id] {
                Constant::Int(n) => format!("{}", n),
                Constant::Bool(b) => format!("{}", b),
                Constant::String(s) => {
                    if s.len() > 20 {
                        format!("\"{}...\"", &s[..17])
                    } else {
                        format!("\"{}\"", s)
                    }
                },
                Constant::Unit => "()".to_string(),
            };
            println!("{:<20} ; const[{}] = {}", format!("LoadConst({})", id), id, const_val);
        }
        Instruction::Jump(target) => {
            println!("{:<20} ; -> {}", format!("Jump({})", target), target);
        }
        Instruction::JumpIfFalse(target) => {
            println!("{:<20} ; -> {} if false", format!("JumpIfFalse({})", target), target);
        }
        Instruction::LoadGlobal(name) => {
            println!("{:<20} ; load {}", format!("LoadGlobal(\"{}\")", name), name);
        }
        Instruction::StoreGlobal(name) => {
            println!("{:<20} ; store {}", format!("StoreGlobal(\"{}\")", name), name);
        }
        Instruction::LoadLocal(name) => {
            println!("{:<20} ; load local {}", format!("LoadLocal(\"{}\")", name), name);
        }
        Instruction::StoreLocal(name) => {
            println!("{:<20} ; store local {}", format!("StoreLocal(\"{}\")", name), name);
        }
        Instruction::StoreExisting(name) => {
            println!("{:<20} ; update existing {}", format!("StoreExisting(\"{}\")", name), name);
        }
        Instruction::Add => println!("{:<20} ; pop b, pop a, push a+b", "Add"),
        Instruction::Sub => println!("{:<20} ; pop b, pop a, push a-b", "Sub"),
        Instruction::Mul => println!("{:<20} ; pop b, pop a, push a*b", "Mul"),
        Instruction::Div => println!("{:<20} ; pop b, pop a, push a/b", "Div"),
        Instruction::Lt => println!("{:<20} ; pop b, pop a, push a<b", "Lt"),
        Instruction::Gt => println!("{:<20} ; pop b, pop a, push a>b", "Gt"),
        Instruction::LtEq => println!("{:<20} ; pop b, pop a, push a<=b", "LtEq"),
        Instruction::GtEq => println!("{:<20} ; pop b, pop a, push a>=b", "GtEq"),
        Instruction::Eq => println!("{:<20} ; pop b, pop a, push a==b", "Eq"),
        Instruction::NotEq => println!("{:<20} ; pop b, pop a, push a!=b", "NotEq"),
        Instruction::Print => println!("{:<20} ; pop and print", "Print"),
        Instruction::Pop => println!("{:<20} ; pop and discard", "Pop"),
        Instruction::PushScope => println!("{:<20} ; enter new scope", "PushScope"),
        Instruction::PopScope => println!("{:<20} ; exit scope", "PopScope"),
        Instruction::Call(n) => println!("{:<20} ; call with {} args", format!("Call({})", n), n),
        Instruction::Return => println!("{:<20} ; return from function", "Return"),
        Instruction::Halt => println!("{:<20} ; stop execution", "Halt"),
        Instruction::BackendCall { backend, method, arg_count } => {
            println!("{:<20} ; {}.{}({})", 
                format!("BackendCall"), backend, method, arg_count);
        }
        Instruction::NewList(size) => {
            println!("{:<20} ; pop {} values, push list", format!("NewList({})", size), size);
        }
        Instruction::LoadIndex => {
            println!("{:<20} ; pop index, pop collection, push value", "LoadIndex");
        }
        Instruction::StoreIndex => {
            println!("{:<20} ; pop value, pop index, pop collection, store value", "StoreIndex");
        }
        Instruction::StoreIndexVar(name) => {
            println!("{:<20} ; mutate {}[index]", format!("StoreIndexVar(\"{}\")", name), name);
        }
        Instruction::ListPush => {
            println!("{:<20} ; pop value, pop list, push updated list", "ListPush");
        }
        Instruction::ListPop => {
            println!("{:<20} ; pop list, push value and updated list", "ListPop");
        }
        Instruction::ListLen => {
            println!("{:<20} ; pop list, push length", "ListLen");
        }
        Instruction::ListPushVar(name) => {
            println!("{:<20} ; mutate {}.push(value)", format!("ListPushVar(\"{}\")", name), name);
        }
        Instruction::ListPopVar(name) => {
            println!("{:<20} ; mutate {}.pop(), push value", format!("ListPopVar(\"{}\")", name), name);
        }
        Instruction::NewMap(size) => {
            println!("{:<20} ; pop {} key/value pairs, push map", format!("NewMap({})", size), size);
        }
        Instruction::MapHas => {
            println!("{:<20} ; pop key, pop map, push bool", "MapHas");
        }
        Instruction::MapKeys => {
            println!("{:<20} ; pop map, push sorted key list", "MapKeys");
        }
        Instruction::MapValues => {
            println!("{:<20} ; pop map, push values by sorted key", "MapValues");
        }
        Instruction::NewStruct(type_name, size) => {
            println!(
                "{:<20} ; pop {} field/value pairs, push {}",
                format!("NewStruct(\"{}\", {})", type_name, size),
                size,
                type_name
            );
        }
        Instruction::LoadField(field) => {
            println!("{:<20} ; pop struct/map, push field", format!("LoadField(\"{}\")", field));
        }
        Instruction::StoreFieldVar { target, field } => {
            println!(
                "{:<20} ; mutate {}.{}",
                format!("StoreFieldVar"),
                target,
                field
            );
        }
    }
}

fn print_parse_error(source: &str, error: &ParseError) {
    eprintln!("Parse error: {}", error);
    let lines: Vec<&str> = source.lines().collect();
    let requested_line = error.line.saturating_sub(1);
    let display_line = requested_line.min(lines.len().saturating_sub(1));

    if let Some(line) = lines.get(display_line) {
        let line_number = display_line + 1;
        let caret_column = if display_line == requested_line {
            error.column.saturating_sub(1)
        } else {
            line.len()
        };

        eprintln!("{:>4} | {}", line_number, line);
        eprintln!("     | {}^", " ".repeat(caret_column));
    }
}

fn build_checked_or_exit(builder: BytecodeBuilder, program: &Program) -> Bytecode {
    builder.build_checked(program).unwrap_or_else(|e| {
        print_semantic_error(&e);
        process::exit(1);
    })
}

fn print_semantic_error(error: &SemanticError) {
    eprintln!("Semantic error: {}", error);
}
