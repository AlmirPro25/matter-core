//! Inspection commands: reflect-json, tokens-json, check-json, imports-json

use matter_ast::Statement;
use matter_bytecode::BytecodeBuilder;
use matter_lexer::Lexer;
use matter_parser::Parser;
use std::collections::{BTreeMap, HashSet};
use std::process;

/// Inspect source as AST and bytecode JSON
pub fn reflect_json(path: &str) {
    let source = super::run::read_source_or_exit(path);
    let mut parser = Parser::from_source(&source);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            let err = serde_json::json!({ "ok": false, "error": format!("{}", e) });
            println!("{}", serde_json::to_string_pretty(&err).unwrap());
            process::exit(1);
        }
    };

    let builder = BytecodeBuilder::new();
    let bytecode = match builder.build_checked(&program) {
        Ok(b) => b,
        Err(e) => {
            let err = serde_json::json!({ "ok": false, "error": format!("{:?}", e) });
            println!("{}", serde_json::to_string_pretty(&err).unwrap());
            process::exit(1);
        }
    };

    // Collect statement kinds
    let mut statement_kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut calls: BTreeMap<String, usize> = BTreeMap::new();
    let mut backend_calls: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_stmts = 0;

    for stmt in &program.statements {
        let kind = statement_kind_name(stmt);
        *statement_kinds.entry(kind.to_string()).or_insert(0) += 1;
        total_stmts += 1;
        collect_calls_stmt(stmt, &mut calls, &mut backend_calls);
    }

    let summary = serde_json::json!({
        "constants": bytecode.constants.len(),
        "functions": bytecode.functions.len(),
        "event_handlers": bytecode.event_handlers.len(),
        "instructions": bytecode.functions.iter().map(|(_name, f)| f.instructions.len()).sum::<usize>(),
    });

    let result = serde_json::json!({
        "ok": true,
        "input": path,
        "reflection_version": 1,
        "ast": {
            "top_level_statements": program.statements.len(),
            "total_statements": total_stmts,
            "statement_kinds": statement_kinds,
            "calls": calls,
            "backend_calls": backend_calls,
        },
        "bytecode": {
            "summary": summary,
        },
    });

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}

fn statement_kind_name(stmt: &Statement) -> &'static str {
    match stmt {
        Statement::Let { .. } => "Let",
        Statement::Set { .. } => "Set",
        Statement::SetIndex { .. } => "SetIndex",
        Statement::SetField { .. } => "SetField",
        Statement::Print(_) => "Print",
        Statement::FunctionDef { .. } => "FunctionDef",
        Statement::StructDef { .. } => "StructDef",
        Statement::Import { .. } => "Import",
        Statement::ImportFrom { .. } => "ImportFrom",
        Statement::ImportAs { .. } => "ImportAs",
        Statement::Export { .. } => "Export",
        Statement::OnEvent { .. } => "OnEvent",
        Statement::Spawn { .. } => "Spawn",
        Statement::If { .. } => "If",
        Statement::While { .. } => "While",
        Statement::For { .. } => "For",
        Statement::Loop { .. } => "Loop",
        Statement::Break => "Break",
        Statement::Continue => "Continue",
        Statement::Return(_) => "Return",
        Statement::Match { .. } => "Match",
        Statement::Expression(_) => "Expression",
    }
}

fn collect_calls_stmt(
    stmt: &Statement,
    calls: &mut BTreeMap<String, usize>,
    backend_calls: &mut BTreeMap<String, usize>,
) {
    match stmt {
        Statement::Print(e) | Statement::Return(e) | Statement::Expression(e) => {
            collect_calls_expr(e, calls, backend_calls);
        }
        Statement::Let { value, .. } | Statement::Set { value, .. } => {
            collect_calls_expr(value, calls, backend_calls);
        }
        Statement::FunctionDef { body, .. } | Statement::OnEvent { body, .. } => {
            for s in body {
                collect_calls_stmt(s, calls, backend_calls);
            }
        }
        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            collect_calls_expr(condition, calls, backend_calls);
            for s in then_body {
                collect_calls_stmt(s, calls, backend_calls);
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    collect_calls_stmt(s, calls, backend_calls);
                }
            }
        }
        _ => {}
    }
}

fn collect_calls_expr(
    expr: &matter_ast::Expression,
    calls: &mut BTreeMap<String, usize>,
    backend_calls: &mut BTreeMap<String, usize>,
) {
    match expr {
        matter_ast::Expression::Call { callee, args } => {
            if let matter_ast::Expression::Identifier(name) = callee.as_ref() {
                *calls.entry(name.clone()).or_insert(0) += 1;
            }
            for arg in args {
                collect_calls_expr(arg, calls, backend_calls);
            }
        }
        matter_ast::Expression::BackendCall {
            backend,
            method,
            args,
        } => {
            *backend_calls
                .entry(format!("{}.{}", backend, method))
                .or_insert(0) += 1;
            for arg in args {
                collect_calls_expr(arg, calls, backend_calls);
            }
        }
        matter_ast::Expression::Binary { left, right, .. } => {
            collect_calls_expr(left, calls, backend_calls);
            collect_calls_expr(right, calls, backend_calls);
        }
        matter_ast::Expression::Unary { operand, .. } => {
            collect_calls_expr(operand, calls, backend_calls);
        }
        matter_ast::Expression::MethodCall { target, args, .. } => {
            collect_calls_expr(target, calls, backend_calls);
            for arg in args {
                collect_calls_expr(arg, calls, backend_calls);
            }
        }
        matter_ast::Expression::List(elements) => {
            for e in elements {
                collect_calls_expr(e, calls, backend_calls);
            }
        }
        matter_ast::Expression::Map(entries) => {
            for (_, v) in entries {
                collect_calls_expr(v, calls, backend_calls);
            }
        }
        _ => {}
    }
}

/// Tokenize source and print JSON
pub fn tokens_json(path: &str) {
    let source = super::run::read_source_or_exit(path);
    let mut lexer = Lexer::new(&source);
    let tokens: Vec<String> = lexer
        .tokenize()
        .iter()
        .map(|t| format!("{:?}", t))
        .collect();

    let result = serde_json::json!({
        "ok": true,
        "input": path,
        "token_count": tokens.len(),
        "tokens": tokens,
    });

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}

/// Validate source without running
pub fn check_json(path: &str) {
    let source = super::run::read_source_or_exit(path);
    let mut parser = Parser::from_source(&source);
    let result = match parser.parse() {
        Ok(program) => {
            let builder = BytecodeBuilder::new();
            match builder.build_checked(&program) {
                Ok(_bytecode) => serde_json::json!({
                    "ok": true,
                    "input": path,
                    "statements": program.statements.len(),
                }),
                Err(e) => serde_json::json!({
                    "ok": false,
                    "error": format!("{:?}", e),
                    "phase": "compile",
                }),
            }
        }
        Err(e) => serde_json::json!({
            "ok": false,
            "error": format!("{}", e),
            "phase": "parse",
        }),
    };

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
