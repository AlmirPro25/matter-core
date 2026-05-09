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

        "package-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            package_json(manifest);
        }

        "project-deps-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            project_deps_json(manifest);
        }

        "project-check-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            project_check_json(manifest);
        }

        "project-verify-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            project_verify_json(manifest);
        }

        "project-run-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            project_run_json(manifest);
        }

        "project-imports-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            project_imports_json(manifest);
        }
        "project-lock-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            project_lock_json(manifest);
        }

        "project-fingerprint-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            project_fingerprint_json(manifest);
        }

        "project-source-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            project_source_json(manifest);
        }

        "project-compile-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            let output = if args.len() >= 5 && args[3] == "-o" {
                &args[4]
            } else {
                "output.mbc"
            };
            project_compile_json(manifest, output);
        }

        "project-build-json" => {
            let manifest = if args.len() >= 3 {
                &args[2]
            } else {
                "matter.toml"
            };
            let output = if args.len() >= 5 && args[3] == "-o" {
                Some(args[4].as_str())
            } else {
                None
            };
            project_build_json(manifest, output);
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
    println!("  matter-cli package-json [matter.toml]       Inspect Matter package manifest as JSON");
    println!("  matter-cli project-deps-json [matter.toml]  Inspect resolved package dependencies as JSON");
    println!("  matter-cli project-check-json [matter.toml] Validate package entrypoint as JSON");
    println!("  matter-cli project-verify-json [matter.toml] Verify dependencies, imports, and compile checks as JSON");
    println!("  matter-cli project-run-json [matter.toml]   Run package entrypoint as JSON");
    println!("  matter-cli project-imports-json [matter.toml] Inspect package import graph as JSON");
    println!("  matter-cli project-lock-json [matter.toml]  Print reproducible package lock JSON");
    println!("  matter-cli project-fingerprint-json [matter.toml] Print project cache fingerprint JSON");
    println!("  matter-cli project-source-json [matter.toml] Print resolved package source as JSON");
    println!("  matter-cli project-compile-json [matter.toml] [-o out] Compile package entrypoint as JSON");
    println!("  matter-cli project-build-json [matter.toml] [-o out] Verify and build cacheable bytecode JSON");
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
            "\"package-json\",",
            "\"project-deps-json\",",
            "\"project-check-json\",",
            "\"project-verify-json\",",
            "\"project-run-json\",",
            "\"project-imports-json\",",
            "\"project-lock-json\",",
            "\"project-fingerprint-json\",",
            "\"project-source-json\",",
            "\"project-compile-json\",",
            "\"project-build-json\",",
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
            "\"network\",",
            "\"concurrency\",",
            "\"packages\"",
            "]",
            "}}"
        ),
        env!("CARGO_PKG_VERSION")
    );
}

#[derive(Debug, Default)]
struct PackageManifest {
    name: String,
    version: String,
    entry: String,
    stdlib: String,
    store: String,
    dependencies: Vec<ManifestDependency>,
}

#[derive(Debug)]
struct ManifestDependency {
    name: String,
    path: String,
}

struct ProjectContext {
    manifest_path: String,
    base_dir: PathBuf,
    manifest: PackageManifest,
}

struct EnvSnapshot {
    key: &'static str,
    previous: Option<String>,
}

struct ProjectFileLock {
    kind: String,
    path: String,
    bytes: usize,
    fingerprint: String,
}

fn package_json(path: &str) {
    let source = fs::read_to_string(path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(path),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let manifest = parse_package_manifest(&source).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"manifest\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(path),
            json_escape(&error)
        );
        process::exit(1);
    });

    println!(
        concat!(
            "{{",
            "\"ok\":true,",
            "\"input\":\"{}\",",
            "\"package\":{{\"name\":\"{}\",\"version\":\"{}\",\"entry\":\"{}\"}},",
            "\"paths\":{{\"stdlib\":\"{}\",\"store\":\"{}\"}},",
            "\"dependencies\":[{}]",
            "}}"
        ),
        json_escape(path),
        json_escape(&manifest.name),
        json_escape(&manifest.version),
        json_escape(&manifest.entry),
        json_escape(&manifest.stdlib),
        json_escape(&manifest.store),
        manifest_dependencies_json(&manifest.dependencies)
    );
}

fn project_deps_json(manifest_path: &str) {
    let project = load_project_or_json_exit(manifest_path);
    let mut items = Vec::new();

    for dependency in &project.manifest.dependencies {
        let resolved_path = project_path(&project.base_dir, &dependency.path);
        let canonical = resolved_path.canonicalize().unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"dependency\",\"package\":\"{}\",\"manifest\":\"{}\",\"dependency\":\"{}\",\"path\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&dependency.name),
                json_escape(&dependency.path),
                json_escape(&error.to_string())
            );
            process::exit(1);
        });

        let bytes = fs::read(&canonical).unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"dependency\",\"package\":\"{}\",\"manifest\":\"{}\",\"dependency\":\"{}\",\"path\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&dependency.name),
                json_escape(&canonical.display().to_string()),
                json_escape(&error.to_string())
            );
            process::exit(1);
        });

        items.push(format!(
            "{{\"name\":\"{}\",\"path\":\"{}\",\"resolved\":\"{}\",\"bytes\":{},\"fingerprint\":\"{}\"}}",
            json_escape(&dependency.name),
            json_escape(&dependency.path),
            json_escape(&canonical.display().to_string()),
            bytes.len(),
            json_escape(&fnv1a64_hex(&bytes))
        ));
    }

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"count\":{},\"dependencies\":[{}]}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        items.len(),
        items.join(",")
    );
}
fn project_check_json(manifest_path: &str) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(&source, &entry_label, &[("package", &project.manifest.name)]);

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"summary\":{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        bytecode_summary_json(&bytecode)
    );
}

fn project_verify_json(manifest_path: &str) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let entry_path = project_path(&project.base_dir, &project.manifest.entry);
    let entry_label = entry_path.display().to_string();
    let source = fs::read_to_string(&entry_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let base_dir = entry_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let mut imports = Vec::new();
    let mut stack = vec![entry_path
        .canonicalize()
        .unwrap_or_else(|_| entry_path.clone())];

    if let Err(error) = collect_imports_with_dependencies(
        &source,
        &base_dir,
        &entry_label,
        &project.base_dir,
        &project.manifest.dependencies,
        &mut stack,
        &mut imports,
    ) {
        println!(
            "{{\"ok\":false,\"stage\":\"import\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error)
        );
        process::exit(1);
    }

    let resolved_source = resolve_imports_with_dependencies(
        &source,
        &base_dir,
        &project.base_dir,
        &project.manifest.dependencies,
        &mut HashSet::new(),
    ).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"import\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error)
        );
        process::exit(1);
    });

    let bytecode = build_json_or_exit(
        &resolved_source,
        &entry_label,
        &[("package", &project.manifest.name)],
    );

    let mut files = Vec::new();
    let mut seen_files = HashSet::new();
    push_lock_file(&mut files, &mut seen_files, "manifest", Path::new(&project.manifest_path));
    push_lock_file(&mut files, &mut seen_files, "entry", &entry_path);
    for import in &imports {
        push_lock_file(&mut files, &mut seen_files, &import.source, Path::new(&import.resolved));
    }
    let lock_fingerprint = project_lock_fingerprint(&files, &imports, &project.manifest.dependencies);

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"lock_fingerprint\":\"{}\",\"dependencies_count\":{},\"imports_count\":{},\"files_count\":{},\"source_bytes\":{},\"summary\":{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(&lock_fingerprint),
        project.manifest.dependencies.len(),
        imports.len(),
        files.len(),
        resolved_source.as_bytes().len(),
        bytecode_summary_json(&bytecode)
    );
}
fn project_run_json(manifest_path: &str) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(&source, &entry_label, &[("package", &project.manifest.name)]);

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.run() {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error),
            json_string_array(&runtime.take_output())
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output\":{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_string_array(&runtime.take_output())
    );
}

fn project_imports_json(manifest_path: &str) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let entry_path = project_path(&project.base_dir, &project.manifest.entry);
    let entry_label = entry_path.display().to_string();
    let source = fs::read_to_string(&entry_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let base_dir = entry_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let mut imports = Vec::new();
    let mut stack = vec![entry_path
        .canonicalize()
        .unwrap_or_else(|_| entry_path.clone())];

    if let Err(error) = collect_imports_with_dependencies(
        &source,
        &base_dir,
        &entry_label,
        &project.base_dir,
        &project.manifest.dependencies,
        &mut stack,
        &mut imports,
    ) {
        println!(
            "{{\"ok\":false,\"stage\":\"import\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error)
        );
        process::exit(1);
    }

    let items: Vec<String> = imports.iter().map(import_info_json).collect();

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"count\":{},\"imports\":[{}]}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        imports.len(),
        items.join(",")
    );
}
fn project_lock_json(manifest_path: &str) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let entry_path = project_path(&project.base_dir, &project.manifest.entry);
    let entry_label = entry_path.display().to_string();
    let source = fs::read_to_string(&entry_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let base_dir = entry_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let mut imports = Vec::new();
    let mut stack = vec![entry_path
        .canonicalize()
        .unwrap_or_else(|_| entry_path.clone())];

    if let Err(error) = collect_imports_with_dependencies(
        &source,
        &base_dir,
        &entry_label,
        &project.base_dir,
        &project.manifest.dependencies,
        &mut stack,
        &mut imports,
    ) {
        println!(
            "{{\"ok\":false,\"stage\":\"import\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error)
        );
        process::exit(1);
    }

    let mut files = Vec::new();
    let mut seen_files = HashSet::new();
    push_lock_file(&mut files, &mut seen_files, "manifest", Path::new(&project.manifest_path));
    push_lock_file(&mut files, &mut seen_files, "entry", &entry_path);
    for import in &imports {
        push_lock_file(&mut files, &mut seen_files, &import.source, Path::new(&import.resolved));
    }

    let file_items: Vec<String> = files.iter().map(project_file_lock_json).collect();
    let import_items: Vec<String> = imports.iter().map(import_info_json).collect();
    let lock_fingerprint = project_lock_fingerprint(&files, &imports, &project.manifest.dependencies);

    println!(
        "{{\"ok\":true,\"package\":{{\"name\":\"{}\",\"version\":\"{}\"}},\"manifest\":\"{}\",\"entry\":\"{}\",\"lock_fingerprint\":\"{}\",\"files_count\":{},\"files\":[{}],\"dependencies\":[{}],\"imports_count\":{},\"imports\":[{}]}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest.version),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(&lock_fingerprint),
        files.len(),
        file_items.join(","),
        manifest_dependencies_json(&project.manifest.dependencies),
        imports.len(),
        import_items.join(",")
    );
}
fn project_fingerprint_json(manifest_path: &str) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let entry_path = project_path(&project.base_dir, &project.manifest.entry);
    let entry_label = entry_path.display().to_string();
    let source = fs::read_to_string(&entry_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let base_dir = entry_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let mut imports = Vec::new();
    let mut stack = vec![entry_path
        .canonicalize()
        .unwrap_or_else(|_| entry_path.clone())];

    if let Err(error) = collect_imports_with_dependencies(
        &source,
        &base_dir,
        &entry_label,
        &project.base_dir,
        &project.manifest.dependencies,
        &mut stack,
        &mut imports,
    ) {
        println!(
            "{{\"ok\":false,\"stage\":\"import\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error)
        );
        process::exit(1);
    }

    let mut files = Vec::new();
    let mut seen_files = HashSet::new();
    push_lock_file(&mut files, &mut seen_files, "manifest", Path::new(&project.manifest_path));
    push_lock_file(&mut files, &mut seen_files, "entry", &entry_path);
    for import in &imports {
        push_lock_file(&mut files, &mut seen_files, &import.source, Path::new(&import.resolved));
    }

    let fingerprint = project_lock_fingerprint(&files, &imports, &project.manifest.dependencies);

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"entry\":\"{}\",\"lock_fingerprint\":\"{}\",\"files_count\":{},\"imports_count\":{},\"dependencies_count\":{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(&fingerprint),
        files.len(),
        imports.len(),
        project.manifest.dependencies.len()
    );
}
fn project_source_json(manifest_path: &str) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let fingerprint = fnv1a64_hex(source.as_bytes());

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"bytes\":{},\"fingerprint\":\"{}\",\"source\":\"{}\"}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        source.as_bytes().len(),
        json_escape(&fingerprint),
        json_escape(&source)
    );
}
fn project_compile_json(manifest_path: &str, output: &str) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(
        &source,
        &entry_label,
        &[("package", &project.manifest.name), ("output", output)],
    );

    if let Err(error) = bytecode.save_to_file(output) {
        println!(
            "{{\"ok\":false,\"stage\":\"write\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(output),
            json_escape(&error.to_string())
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output\":\"{}\",\"summary\":{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(output),
        bytecode_summary_json(&bytecode)
    );
}

fn project_build_json(manifest_path: &str, output: Option<&str>) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let entry_path = project_path(&project.base_dir, &project.manifest.entry);
    let entry_label = entry_path.display().to_string();
    let source = fs::read_to_string(&entry_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let base_dir = entry_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let mut imports = Vec::new();
    let mut stack = vec![entry_path
        .canonicalize()
        .unwrap_or_else(|_| entry_path.clone())];

    if let Err(error) = collect_imports_with_dependencies(
        &source,
        &base_dir,
        &entry_label,
        &project.base_dir,
        &project.manifest.dependencies,
        &mut stack,
        &mut imports,
    ) {
        println!(
            "{{\"ok\":false,\"stage\":\"import\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error)
        );
        process::exit(1);
    }

    let resolved_source = resolve_imports_with_dependencies(
        &source,
        &base_dir,
        &project.base_dir,
        &project.manifest.dependencies,
        &mut HashSet::new(),
    ).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"import\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error)
        );
        process::exit(1);
    });

    let bytecode = build_json_or_exit(
        &resolved_source,
        &entry_label,
        &[("package", &project.manifest.name)],
    );

    let mut files = Vec::new();
    let mut seen_files = HashSet::new();
    push_lock_file(&mut files, &mut seen_files, "manifest", Path::new(&project.manifest_path));
    push_lock_file(&mut files, &mut seen_files, "entry", &entry_path);
    for import in &imports {
        push_lock_file(&mut files, &mut seen_files, &import.source, Path::new(&import.resolved));
    }
    let lock_fingerprint = project_lock_fingerprint(&files, &imports, &project.manifest.dependencies);

    let output_path = output
        .map(|path| path.to_string())
        .unwrap_or_else(|| project_artifact_path(&project.manifest.name, &lock_fingerprint));

    if let Some(parent) = Path::new(&output_path).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = fs::create_dir_all(parent) {
                println!(
                    "{{\"ok\":false,\"stage\":\"write\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                    json_escape(&project.manifest.name),
                    json_escape(&project.manifest_path),
                    json_escape(&entry_label),
                    json_escape(&output_path),
                    json_escape(&error.to_string())
                );
                process::exit(1);
            }
        }
    }

    if let Err(error) = bytecode.save_to_file(&output_path) {
        println!(
            "{{\"ok\":false,\"stage\":\"write\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&output_path),
            json_escape(&error.to_string())
        );
        process::exit(1);
    }

    let bytecode_bytes = fs::read(&output_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&output_path),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output\":\"{}\",\"lock_fingerprint\":\"{}\",\"bytecode_fingerprint\":\"{}\",\"bytecode_bytes\":{},\"files_count\":{},\"imports_count\":{},\"dependencies_count\":{},\"summary\":{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(&output_path),
        json_escape(&lock_fingerprint),
        json_escape(&fnv1a64_hex(&bytecode_bytes)),
        bytecode_bytes.len(),
        files.len(),
        imports.len(),
        project.manifest.dependencies.len(),
        bytecode_summary_json(&bytecode)
    );
}

fn load_project_or_json_exit(manifest_path: &str) -> ProjectContext {
    let source = fs::read_to_string(manifest_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"manifest\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(manifest_path),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let manifest = parse_package_manifest(&source).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"manifest\",\"manifest\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(manifest_path),
            json_escape(&error)
        );
        process::exit(1);
    });

    let manifest_file = Path::new(manifest_path);
    let base_dir = manifest_file
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();

    ProjectContext {
        manifest_path: manifest_path.to_string(),
        base_dir,
        manifest,
    }
}

fn read_project_entry_or_json_exit(project: &ProjectContext) -> (String, String) {
    let entry_path = project_path(&project.base_dir, &project.manifest.entry);
    let entry_label = entry_path.display().to_string();
    let source = fs::read_to_string(&entry_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let base_dir = entry_path.parent().unwrap_or(Path::new("."));
    let resolved = resolve_imports_with_dependencies(
        &source,
        base_dir,
        &project.base_dir,
        &project.manifest.dependencies,
        &mut HashSet::new(),
    ).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"import\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&error)
        );
        process::exit(1);
    });

    (resolved, entry_label)
}

fn apply_project_env(project: &ProjectContext) -> Vec<EnvSnapshot> {
    let mut snapshots = Vec::new();

    if !project.manifest.stdlib.is_empty() {
        snapshots.push(set_env_snapshot(
            "MATTER_STDLIB_PATH",
            project_path(&project.base_dir, &project.manifest.stdlib).display().to_string(),
        ));
    }

    if !project.manifest.store.is_empty() {
        snapshots.push(set_env_snapshot(
            "MATTER_STORE_PATH",
            project_path(&project.base_dir, &project.manifest.store).display().to_string(),
        ));
    }

    snapshots
}

fn set_env_snapshot(key: &'static str, value: String) -> EnvSnapshot {
    let previous = env::var(key).ok();
    env::set_var(key, value);
    EnvSnapshot { key, previous }
}

impl Drop for EnvSnapshot {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            env::set_var(self.key, value);
        } else {
            env::remove_var(self.key);
        }
    }
}

fn project_path(base_dir: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn project_artifact_path(package_name: &str, fingerprint: &str) -> String {
    let safe_name: String = package_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    format!("target/{}-{}.mbc", safe_name, fingerprint)
}

fn push_lock_file(
    files: &mut Vec<ProjectFileLock>,
    seen: &mut HashSet<String>,
    kind: &str,
    path: &Path,
) {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let key = canonical.display().to_string();
    if !seen.insert(key.clone()) {
        return;
    }

    let bytes = fs::read(&canonical).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"lock\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&key),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    files.push(ProjectFileLock {
        kind: kind.to_string(),
        path: key,
        bytes: bytes.len(),
        fingerprint: fnv1a64_hex(&bytes),
    });
}

fn project_file_lock_json(file: &ProjectFileLock) -> String {
    format!(
        "{{\"kind\":\"{}\",\"path\":\"{}\",\"bytes\":{},\"fingerprint\":\"{}\"}}",
        json_escape(&file.kind),
        json_escape(&file.path),
        file.bytes,
        json_escape(&file.fingerprint)
    )
}

fn project_lock_fingerprint(
    files: &[ProjectFileLock],
    imports: &[ImportInfo],
    dependencies: &[ManifestDependency],
) -> String {
    let mut material = String::new();

    for file in files {
        material.push_str("file\t");
        material.push_str(&file.kind);
        material.push('\t');
        material.push_str(&file.path);
        material.push('\t');
        material.push_str(&file.bytes.to_string());
        material.push('\t');
        material.push_str(&file.fingerprint);
        material.push('\n');
    }

    for dependency in dependencies {
        material.push_str("dependency\t");
        material.push_str(&dependency.name);
        material.push('\t');
        material.push_str(&dependency.path);
        material.push('\n');
    }

    for import in imports {
        material.push_str("import\t");
        material.push_str(&import.from);
        material.push('\t');
        material.push_str(&import.path);
        material.push('\t');
        material.push_str(&import.resolved);
        material.push('\t');
        material.push_str(&import.source);
        material.push('\n');
    }

    fnv1a64_hex(material.as_bytes())
}
fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}
fn parse_package_manifest(source: &str) -> Result<PackageManifest, String> {
    let mut manifest = PackageManifest::default();
    let mut section = String::new();

    for (line_index, raw_line) in source.lines().enumerate() {
        let line = raw_line
            .trim_start_matches('\u{feff}')
            .split('#')
            .next()
            .unwrap_or("")
            .trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }

        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {}: expected key = \"value\"", line_index + 1))?;
        let key = key.trim();
        let value = parse_manifest_string(value.trim())
            .ok_or_else(|| format!("line {}: expected quoted string value", line_index + 1))?;

        match section.as_str() {
            "package" => match key {
                "name" => manifest.name = value,
                "version" => manifest.version = value,
                "entry" => manifest.entry = value,
                _ => return Err(format!("unknown package key '{}'", key)),
            },
            "paths" => match key {
                "stdlib" => manifest.stdlib = value,
                "store" => manifest.store = value,
                _ => return Err(format!("unknown paths key '{}'", key)),
            },
            "dependencies" => manifest.dependencies.push(ManifestDependency {
                name: key.to_string(),
                path: value,
            }),
            "" => return Err(format!("line {}: key outside of section", line_index + 1)),
            _ => return Err(format!("unknown section '{}'", section)),
        }
    }

    if manifest.name.is_empty() {
        return Err("package.name is required".to_string());
    }
    if manifest.version.is_empty() {
        return Err("package.version is required".to_string());
    }
    if manifest.entry.is_empty() {
        return Err("package.entry is required".to_string());
    }

    Ok(manifest)
}

fn parse_manifest_string(value: &str) -> Option<String> {
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return None;
    }

    Some(value[1..value.len() - 1].replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn manifest_dependencies_json(dependencies: &[ManifestDependency]) -> String {
    let items: Vec<String> = dependencies
        .iter()
        .map(|dependency| {
            format!(
                "{{\"name\":\"{}\",\"path\":\"{}\"}}",
                json_escape(&dependency.name),
                json_escape(&dependency.path)
            )
        })
        .collect();
    items.join(",")
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
    resolve_imports_with_dependencies(source, base_dir, Path::new("."), &[], seen)
}

fn resolve_imports_with_dependencies(
    source: &str,
    base_dir: &Path,
    project_base_dir: &Path,
    dependencies: &[ManifestDependency],
    seen: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let mut resolved = String::new();

    for line in source.lines() {
        if let Some(import_path) = parse_import_line(line) {
            let canonical = resolve_import_path_with_dependencies(
                &import_path,
                base_dir,
                project_base_dir,
                dependencies,
            )?;

            if !seen.insert(canonical.clone()) {
                return Err(format!("circular import detected for '{}'", canonical.display()));
            }

            let imported_source = fs::read_to_string(&canonical).map_err(|e| {
                format!("could not read import '{}': {}", canonical.display(), e)
            })?;
            let imported_base = canonical.parent().unwrap_or(Path::new("."));
            resolved.push_str(&resolve_imports_with_dependencies(
                &imported_source,
                imported_base,
                project_base_dir,
                dependencies,
                seen,
            )?);
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
    let trimmed = line.trim_start_matches('\u{feff}').trim();
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
    resolve_import_path_with_dependencies(import_path, base_dir, Path::new("."), &[])
}

fn resolve_import_path_with_dependencies(
    import_path: &str,
    base_dir: &Path,
    project_base_dir: &Path,
    dependencies: &[ManifestDependency],
) -> Result<PathBuf, String> {
    let full_path = if is_std_import(import_path) {
        stdlib_root().join(strip_std_prefix(import_path))
    } else if let Some(dependency) = dependencies.iter().find(|dependency| dependency.name == import_path) {
        project_path(project_base_dir, &dependency.path)
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
    source: String,
}

fn import_info_json(import: &ImportInfo) -> String {
    format!(
        "{{\"from\":\"{}\",\"path\":\"{}\",\"resolved\":\"{}\",\"source\":\"{}\"}}",
        json_escape(&import.from),
        json_escape(&import.path),
        json_escape(&import.resolved),
        json_escape(&import.source)
    )
}

fn collect_imports(
    source: &str,
    base_dir: &Path,
    from_label: &str,
    stack: &mut Vec<PathBuf>,
    imports: &mut Vec<ImportInfo>,
) -> Result<(), String> {
    collect_imports_with_dependencies(
        source,
        base_dir,
        from_label,
        Path::new("."),
        &[],
        stack,
        imports,
    )
}

fn collect_imports_with_dependencies(
    source: &str,
    base_dir: &Path,
    from_label: &str,
    project_base_dir: &Path,
    dependencies: &[ManifestDependency],
    stack: &mut Vec<PathBuf>,
    imports: &mut Vec<ImportInfo>,
) -> Result<(), String> {
    for line in source.lines() {
        if let Some(import_path) = parse_import_line(line) {
            let canonical = resolve_import_path_with_dependencies(
                &import_path,
                base_dir,
                project_base_dir,
                dependencies,
            )?;

            if stack.iter().any(|path| path == &canonical) {
                return Err(format!("circular import detected for '{}'", canonical.display()));
            }

            let resolved = canonical.display().to_string();
            let source = import_source_kind(&import_path, dependencies);
            imports.push(ImportInfo {
                from: from_label.to_string(),
                path: import_path.clone(),
                resolved: resolved.clone(),
                source,
            });

            let imported_source = fs::read_to_string(&canonical).map_err(|e| {
                format!("could not read import '{}': {}", canonical.display(), e)
            })?;
            let imported_base = canonical
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf();

            stack.push(canonical);
            collect_imports_with_dependencies(
                &imported_source,
                &imported_base,
                &resolved,
                project_base_dir,
                dependencies,
                stack,
                imports,
            )?;
            stack.pop();
        }
    }

    Ok(())
}

fn import_source_kind(import_path: &str, dependencies: &[ManifestDependency]) -> String {
    if is_std_import(import_path) {
        "stdlib".to_string()
    } else if dependencies.iter().any(|dependency| dependency.name == import_path) {
        "dependency".to_string()
    } else {
        "local".to_string()
    }
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
        Token::Spawn => ("spawn", None),
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
        Instruction::SpawnEvent(event) => {
            println!("{:<20} ; enqueue event {}", format!("SpawnEvent(\"{}\")", event), event);
        }
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
