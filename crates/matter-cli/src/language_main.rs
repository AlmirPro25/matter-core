//! Matter CLI — language-only entrypoint (Phase 1 default binary).
//!
//! Includes: parse/compile MBC1, load/run bytecode, VM execution, core diagnostics.
//! Excludes by default: agent/shell/network, polyglot bridges, visual/device stacks.
//!
//! Full experimental surface: build `matter-cli-experimental` with
//! `--features experimental-full`.

mod capability_policy;
mod commands;

use capability_policy::is_language_only_denied_command;
use matter_bytecode::{Bytecode, BytecodeBuilder};
use matter_parser::Parser;
use matter_runtime::Runtime;
use serde_json::json;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        process::exit(0);
    }

    let command = args[1].as_str();
    match command {
        "help" | "--help" | "-h" => print_usage(),
        "version" | "--version" | "-V" => {
            println!("matter-cli {} (language-only)", env!("CARGO_PKG_VERSION"));
            println!("edition=language-only features=default");
        }
        "capabilities-json" => print_capabilities_json(),
        "core-status-json" => print_core_status_json(),
        "run" => {
            require_arg(&args, 2, "Usage: matter-cli run <file.matter|->");
            commands::run::run_file(&args[2]);
        }
        "run-json" => {
            require_arg(&args, 2, "Usage: matter-cli run-json <file.matter|->");
            commands::run::run_file_json(&args[2], false);
        }
        "eval" => {
            require_arg(&args, 2, "Usage: matter-cli eval <source>");
            commands::run::eval_source(&args[2]);
        }
        "eval-json" => {
            require_arg(&args, 2, "Usage: matter-cli eval-json <source>");
            commands::run::eval_source_json(&args[2]);
        }
        "check" => {
            require_arg(&args, 2, "Usage: matter-cli check <file.matter|->");
            check_file(&args[2]);
        }
        "check-json" => {
            require_arg(&args, 2, "Usage: matter-cli check-json <file.matter|->");
            check_json(&args[2]);
        }
        "compile" => {
            require_arg(&args, 2, "Usage: matter-cli compile <file.matter|-> [-o out.mbc]");
            let output = option_value(&args[2..], "-o").unwrap_or_else(|| "output.mbc".to_string());
            compile_file(&args[2], &output);
        }
        "compile-json" => {
            require_arg(
                &args,
                2,
                "Usage: matter-cli compile-json <file.matter|-> [-o out.mbc]",
            );
            let output = option_value(&args[2..], "-o").unwrap_or_else(|| "output.mbc".to_string());
            compile_json(&args[2], &output);
        }
        "run-bytecode" => {
            require_arg(&args, 2, "Usage: matter-cli run-bytecode <file.mbc>");
            run_bytecode(&args[2]);
        }
        "run-bytecode-json" => {
            require_arg(&args, 2, "Usage: matter-cli run-bytecode-json <file.mbc>");
            run_bytecode_json(&args[2]);
        }
        "inspect" => {
            require_arg(&args, 2, "Usage: matter-cli inspect <file.mbc>");
            inspect_bytecode(&args[2]);
        }
        "inspect-json" => {
            require_arg(&args, 2, "Usage: matter-cli inspect-json <file.mbc>");
            commands::inspect::reflect_json(&args[2]);
        }
        other => {
            // Phase 3: deny dangerous / experimental capability names unambiguously.
            if is_language_only_denied_command(other) {
                eprintln!(
                    "error: command '{}' is not available in the language-only build",
                    other
                );
                eprintln!(
                    "error: the action was NOT executed (no shell, network, agent, package, or bridge)"
                );
                eprintln!(
                    "hint: experimental edition (NOT a sandbox): cargo build -p matter-cli --release --features experimental-full --bin matter-cli-experimental"
                );
                process::exit(2);
            }
            eprintln!("Unknown command: {}", other);
            eprintln!("error: the action was NOT executed");
            print_usage();
            process::exit(1);
        }
    }
}

fn require_arg(args: &[String], idx: usize, usage: &str) {
    if args.len() <= idx {
        eprintln!("{}", usage);
        process::exit(1);
    }
}

fn option_value(args: &[String], flag: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == flag {
            return args.get(i + 1).cloned();
        }
        i += 1;
    }
    None
}

fn print_usage() {
    println!("Matter CLI {} — language-only edition", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage:");
    println!("  matter-cli --help");
    println!("  matter-cli --version");
    println!("  matter-cli capabilities-json");
    println!("  matter-cli core-status-json");
    println!("  matter-cli run <file.matter|->");
    println!("  matter-cli run-json <file.matter|->");
    println!("  matter-cli eval <source>");
    println!("  matter-cli check <file.matter|->");
    println!("  matter-cli check-json <file.matter|->");
    println!("  matter-cli compile <file.matter|-> [-o out.mbc]");
    println!("  matter-cli compile-json <file.matter|-> [-o out.mbc]");
    println!("  matter-cli run-bytecode <file.mbc>");
    println!("  matter-cli run-bytecode-json <file.mbc>");
    println!("  matter-cli inspect <file.mbc>");
    println!();
    println!("Language-only build excludes agent/shell/network, polyglot bridges, visual/device.");
    println!("Experimental full binary: matter-cli-experimental --features experimental-full");
}

fn print_capabilities_json() {
    let payload = json!({
        "ok": true,
        "name": "matter-cli",
        "version": env!("CARGO_PKG_VERSION"),
        "edition": "language-only",
        "bytecode": "MBC1",
        "security": {
            "is_sandbox": false,
            "shell_spawn": false,
            "powershell": false,
            "network": false,
            "polyglot_bridges": false,
            "agent_ui": false,
            "package_install": false,
            "local_command_capture": false,
            "policy": "dangerous capabilities absent from language-only binary / denied by command gate"
        },
        "features": {
            "polyglot": cfg!(feature = "polyglot"),
            "visual": cfg!(feature = "visual"),
            "frontier": cfg!(feature = "frontier"),
            "device": cfg!(feature = "device"),
            "agent": cfg!(feature = "agent"),
            "net": cfg!(feature = "net"),
            "experimental_full": cfg!(feature = "experimental-full")
        },
        "commands": [
            "capabilities-json",
            "core-status-json",
            "run",
            "run-json",
            "eval",
            "eval-json",
            "check",
            "check-json",
            "compile",
            "compile-json",
            "run-bytecode",
            "run-bytecode-json",
            "inspect",
            "inspect-json",
            "help",
            "version"
        ],
        "disabled_by_default": capability_policy::LANGUAGE_ONLY_DENIED_COMMANDS
    });
    println!("{}", serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into()));
}

fn print_core_status_json() {
    // Minimal embedded sample (same claim semantics as experimental core-status:
    // experimental runtime, core loop validated on this host when sample runs).
    let sample = r#"
fn fib(n) {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}
print "Matter Core"
print fib(8)
on boot { print "event: boot" }
spawn boot
"#;
    let mut checks = Vec::new();
    let mut parser = match Parser::from_source_checked(sample) {
        Ok(p) => p,
        Err(e) => {
            println!("{}", json!({
                "ok": false,
                "kind": "core_status",
                "schema_version": 1,
                "summary": {
                    "claim": "experimental_language_runtime",
                    "production_ready": false,
                    "core_loop_validated": false,
                    "edition": "language-only"
                },
                "checks": [{"name":"parse","passed":false,"severity":"fail","detail":e.to_string()}]
            }));
            return;
        }
    };
    let program = match parser.parse() {
        Ok(p) => {
            checks.push(json!({"name":"parse","passed":true,"severity":"pass","detail":format!("{} top-level statements", p.statements.len())}));
            p
        }
        Err(e) => {
            println!("{}", json!({
                "ok": false,
                "kind": "core_status",
                "schema_version": 1,
                "summary": {
                    "claim": "experimental_language_runtime",
                    "production_ready": false,
                    "core_loop_validated": false,
                    "edition": "language-only"
                },
                "checks": [{"name":"parse","passed":false,"severity":"fail","detail":e.to_string()}]
            }));
            return;
        }
    };

    let builder = BytecodeBuilder::new();
    let bytecode = match builder.build_checked(&program) {
        Ok(b) => {
            checks.push(json!({
                "name":"compile",
                "passed":true,
                "severity":"pass",
                "detail": format!("{} main instructions, {} functions, {} event handlers",
                    b.main_instructions.len(), b.functions.len(), b.event_handlers.len())
            }));
            b
        }
        Err(e) => {
            println!("{}", json!({
                "ok": false,
                "kind": "core_status",
                "schema_version": 1,
                "summary": {
                    "claim": "experimental_language_runtime",
                    "production_ready": false,
                    "core_loop_validated": false,
                    "edition": "language-only"
                },
                "checks": checks
            }));
            let _ = e;
            return;
        }
    };

    let mut runtime = Runtime::new_silent(bytecode.clone());
    runtime.set_stdout_enabled(false);
    match runtime.run() {
        Ok(()) => {
            let out = runtime.take_output();
            checks.push(json!({"name":"run","passed":true,"severity":"pass","detail":format!("{} output lines captured", out.len())}));
            println!("{}", json!({
                "ok": true,
                "kind": "core_status",
                "schema_version": 1,
                "$schema": "schemas/core-status.schema.json",
                "summary": {
                    "claim": "experimental_language_runtime",
                    "production_ready": false,
                    "core_loop_validated": true,
                    "execution_controlled": true,
                    "pipeline": "source_to_bytecode_to_vm_to_runtime",
                    "bytecode": "MBC1",
                    "edition": "language-only",
                    "guard_status": "pass"
                },
                "checks": checks,
                "evidence": {
                    "output": out,
                    "sample": "embedded:core-status-language-only",
                    "bytecode": {
                        "summary": {
                            "constants": bytecode.constants.len(),
                            "functions": bytecode.functions.len(),
                            "event_handlers": bytecode.event_handlers.len(),
                            "instructions": bytecode.main_instructions.len()
                        }
                    }
                }
            }));
        }
        Err(e) => {
            checks.push(json!({"name":"run","passed":false,"severity":"fail","detail":e}));
            println!("{}", json!({
                "ok": false,
                "kind": "core_status",
                "schema_version": 1,
                "summary": {
                    "claim": "experimental_language_runtime",
                    "production_ready": false,
                    "core_loop_validated": false,
                    "edition": "language-only"
                },
                "checks": checks
            }));
        }
    }
}

fn read_source(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("stdin read failed: {}", e))?;
        return Ok(buf);
    }
    fs::read_to_string(path).map_err(|e| format!("read '{}': {}", path, e))
}

fn parse_program_or_exit(source: &str) -> matter_ast::Program {
    let mut parser = Parser::from_source_checked(source).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        process::exit(1);
    });
    parser.parse().unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        process::exit(1);
    })
}

fn check_file(path: &str) {
    let source = read_source(path).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });
    let program = parse_program_or_exit(&source);
    let builder = BytecodeBuilder::new();
    match builder.build_checked(&program) {
        Ok(_) => println!("OK {}", path),
        Err(e) => {
            eprintln!("Semantic/compile error: {:?}", e);
            process::exit(1);
        }
    }
}

fn check_json(path: &str) {
    let source = match read_source(path) {
        Ok(s) => s,
        Err(e) => {
            println!("{}", json!({"ok":false,"error":e,"phase":"read"}));
            process::exit(1);
        }
    };
    let mut parser = match Parser::from_source_checked(&source) {
        Ok(p) => p,
        Err(e) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "stage": "parse",
                    "input": path,
                    "error": {"message": e.to_string(), "line": e.line, "column": e.column}
                })
            );
            process::exit(1);
        }
    };
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "stage": "parse",
                    "input": path,
                    "error": {"message": e.to_string(), "line": e.line, "column": e.column}
                })
            );
            process::exit(1);
        }
    };
    let builder = BytecodeBuilder::new();
    match builder.build_checked(&program) {
        Ok(b) => {
            println!(
                "{}",
                json!({
                    "ok": true,
                    "input": path,
                    "summary": {
                        "constants": b.constants.len(),
                        "functions": b.functions.len(),
                        "event_handlers": b.event_handlers.len(),
                        "instructions": b.main_instructions.len()
                    }
                })
            );
        }
        Err(e) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "stage": "semantic",
                    "input": path,
                    "error": {"message": format!("{:?}", e)}
                })
            );
            process::exit(1);
        }
    }
}

fn compile_file(input: &str, output: &str) {
    let source = read_source(input).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });
    let program = parse_program_or_exit(&source);
    let builder = BytecodeBuilder::new();
    let bytecode = builder.build_checked(&program).unwrap_or_else(|e| {
        eprintln!("Compilation error: {:?}", e);
        process::exit(1);
    });
    if let Err(e) = bytecode.save_to_file(output) {
        eprintln!("Error writing bytecode to '{}': {}", output, e);
        process::exit(1);
    }
    println!("Compiled successfully -> {}", output);
    println!(
        "  constants={} functions={} events={} main_instructions={}",
        bytecode.constants.len(),
        bytecode.functions.len(),
        bytecode.event_handlers.len(),
        bytecode.main_instructions.len()
    );
}

fn compile_json(input: &str, output: &str) {
    let source = match read_source(input) {
        Ok(s) => s,
        Err(e) => {
            println!("{}", json!({"ok":false,"phase":"read","error":e,"ok_claim":false}));
            process::exit(1);
        }
    };
    let mut parser = match Parser::from_source_checked(&source) {
        Ok(p) => p,
        Err(e) => {
            println!("{}", json!({"ok":false,"phase":"parse","error":e.to_string()}));
            process::exit(1);
        }
    };
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            println!("{}", json!({"ok":false,"phase":"parse","error":e.to_string()}));
            process::exit(1);
        }
    };
    let builder = BytecodeBuilder::new();
    let bytecode = match builder.build_checked(&program) {
        Ok(b) => b,
        Err(e) => {
            println!("{}", json!({"ok":false,"phase":"compile","error":format!("{:?}", e)}));
            process::exit(1);
        }
    };
    if let Err(e) = bytecode.save_to_file(output) {
        println!("{}", json!({"ok":false,"phase":"write","error":e.to_string()}));
        process::exit(1);
    }
    println!(
        "{}",
        json!({
            "ok": true,
            "input": input,
            "output": output,
            "summary": {
                "constants": bytecode.constants.len(),
                "functions": bytecode.functions.len(),
                "event_handlers": bytecode.event_handlers.len(),
                "instructions": bytecode.main_instructions.len()
            }
        })
    );
}

fn load_bytecode_or_exit(path: &str) -> Bytecode {
    // load_from_file deserializes AND validates structure before return.
    match Bytecode::load_from_file(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error loading bytecode '{}': {}", path, e);
            process::exit(1);
        }
    }
}

fn run_bytecode(path: &str) {
    let bytecode = load_bytecode_or_exit(path);
    let mut runtime = Runtime::new(bytecode);
    if let Err(e) = runtime.run() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }
}

fn run_bytecode_json(path: &str) {
    let bytecode = match Bytecode::load_from_file(path) {
        Ok(b) => b,
        Err(e) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "input": path,
                    "phase": "load_validate",
                    "error": e.to_string()
                })
            );
            process::exit(1);
        }
    };
    let mut runtime = Runtime::new(bytecode);
    let start = std::time::Instant::now();
    match runtime.run() {
        Ok(()) => {
            let output = runtime.take_output();
            println!(
                "{}",
                json!({
                    "ok": true,
                    "input": path,
                    "output": output,
                    "elapsed_ms": start.elapsed().as_secs_f64() * 1000.0
                })
            );
        }
        Err(e) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "input": path,
                    "phase": "runtime",
                    "error": e
                })
            );
            process::exit(1);
        }
    }
}

fn inspect_bytecode(path: &str) {
    let bytecode = load_bytecode_or_exit(path);
    println!("MBC1 {}", path);
    println!("  constants:      {}", bytecode.constants.len());
    println!("  functions:      {}", bytecode.functions.len());
    println!("  event_handlers: {}", bytecode.event_handlers.len());
    println!("  main_instructions: {}", bytecode.main_instructions.len());
    let _ = Path::new(path);
}
