//! Core execution commands: run, eval, run-json, eval-json

use matter_bytecode::BytecodeBuilder;
use matter_parser::Parser;
use matter_runtime::Runtime;
use std::fs;
use std::process;

/// Run a Matter source file
pub fn run_file(path: &str) {
    let source = read_source_or_exit(path);
    let mut parser = match Parser::from_source_checked(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    let builder = BytecodeBuilder::new();
    let bytecode = match builder.build_checked(&program) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Compilation error: {:?}", e);
            process::exit(1);
        }
    };

    let mut runtime = Runtime::new(bytecode);
    if let Err(e) = runtime.run() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

/// Run a Matter source file and print JSON result
pub fn run_file_json(path: &str, with_energy: bool) {
    let source = read_source_or_exit(path);
    run_source_json(&source, path, with_energy);
}

/// Evaluate Matter source text
pub fn eval_source(source: &str) {
    let mut parser = match Parser::from_source_checked(source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    let builder = BytecodeBuilder::new();
    let bytecode = match builder.build_checked(&program) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Compilation error: {:?}", e);
            process::exit(1);
        }
    };

    let mut runtime = Runtime::new(bytecode);
    if let Err(e) = runtime.run() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

/// Evaluate Matter source text and print JSON result
pub fn eval_source_json(source: &str) {
    run_source_json(source, "<eval>", false);
}

/// Internal: run source and print JSON
pub fn run_source_json(source: &str, label: &str, with_energy: bool) {
    let _ = with_energy;
    let mut parser = match Parser::from_source_checked(source) {
        Ok(p) => p,
        Err(e) => {
            let err = serde_json::json!({
                "ok": false,
                "error": format!("{}", e),
                "phase": "parse",
            });
            println!("{}", serde_json::to_string_pretty(&err).unwrap_or_else(|_| "{\"ok\":false}".into()));
            process::exit(1);
        }
    };
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            let err = serde_json::json!({
                "ok": false,
                "error": format!("{}", e),
                "phase": "parse",
            });
            println!("{}", serde_json::to_string_pretty(&err).unwrap_or_else(|_| "{\"ok\":false}".into()));
            process::exit(1);
        }
    };

    let builder = BytecodeBuilder::new();
    let bytecode = match builder.build_checked(&program) {
        Ok(b) => b,
        Err(e) => {
            let err = serde_json::json!({
                "ok": false,
                "error": format!("{:?}", e),
                "phase": "compile",
            });
            println!("{}", serde_json::to_string_pretty(&err).unwrap_or_else(|_| "{\"ok\":false}".into()));
            process::exit(1);
        }
    };

    let mut runtime = Runtime::new(bytecode);
    let start = std::time::Instant::now();
    let run_result = runtime.run();
    let elapsed = start.elapsed();

    match run_result {
        Ok(()) => {
            let output = runtime.take_output();
            let result = serde_json::json!({
                "ok": true,
                "input": label,
                "output": output,
                "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
            });
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        Err(e) => {
            let err = serde_json::json!({
                "ok": false,
                "error": format!("{}", e),
                "phase": "runtime",
            });
            println!("{}", serde_json::to_string_pretty(&err).unwrap());
            process::exit(1);
        }
    }
}

/// Read source from file or stdin
pub fn read_source_or_exit(path: &str) -> String {
    if path == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf).unwrap_or_else(|e| {
            eprintln!("Failed to read stdin: {}", e);
            process::exit(1);
        });
        buf
    } else {
        fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Failed to read '{}': {}", path, e);
            process::exit(1);
        })
    }
}
