//! Matter CLI
//! Interface de linha de comando para Matter

use emnr_matter_bridge::{MatterBridgeSession, MatterPhase, MatterRuntimeEvent};
use matter_ast::{Expression, Program, Statement};
use matter_backend::Value;
use matter_bridge_rust::{decode_args_json, encode_value_json, RustBridge};
use matter_bytecode::{Bytecode, BytecodeBuilder, Instruction, SemanticError};
use matter_lexer::{Lexer, Token};
use matter_parser::{ParseError, Parser};
use matter_polyglot::bridge::LanguageBridge;
use matter_runtime::Runtime;
use matter_sentinel_abi::{self, PvmOpcodeTag};
use serde_json::{json, Value as JsonValue};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static WEB_EVENT_QUEUE: OnceLock<Mutex<Vec<(u64, String)>>> = OnceLock::new();
static WEB_SERVER_STARTED_AT: OnceLock<Instant> = OnceLock::new();
static WEB_EVENT_NEXT_ID: AtomicU64 = AtomicU64::new(1);
static WEB_EVENT_ACK_CURSOR: AtomicU64 = AtomicU64::new(0);
static VM_LIVE_STATE: OnceLock<Mutex<VmLiveState>> = OnceLock::new();

#[derive(Clone)]
struct VmLiveState {
    processed_events: u64,
    click_counter: u64,
    key_counter: u64,
    input_counter: u64,
    tick_counter: u64,
    action_counter: u64,
    last_key: String,
    last_input: String,
    last_action: String,
    last_event: String,
    last_step_result: String,
    visual_color: String,
}

impl Default for VmLiveState {
    fn default() -> Self {
        Self {
            processed_events: 0,
            click_counter: 0,
            key_counter: 0,
            input_counter: 0,
            tick_counter: 0,
            action_counter: 0,
            last_key: "".to_string(),
            last_input: "".to_string(),
            last_action: "".to_string(),
            last_event: "{}".to_string(),
            last_step_result: "{}".to_string(),
            visual_color: "#2563eb".to_string(),
        }
    }
}

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
        "tool-ci-catalog-json" => {
            print_tool_ci_catalog_json();
        }
        "tool-ci-verify-json" => {
            if args.len() < 4 {
                eprintln!("Usage: matter-cli tool-ci-verify-json <reason> <code>");
                process::exit(1);
            }
            tool_ci_verify_json(&args[2], &args[3]);
        }
        "tool-ci-contract-json" => {
            print_tool_ci_contract_json();
        }
        "tool-pipeline-validate-contract-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli tool-pipeline-validate-contract-json <file.json>");
                process::exit(1);
            }
            tool_pipeline_validate_contract_json(&args[2]);
        }
        "tool-pipeline-normalize-contract-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli tool-pipeline-normalize-contract-json <in.json> [out.json]"
                );
                process::exit(1);
            }
            let out = if args.len() >= 4 {
                Some(args[3].as_str())
            } else {
                None
            };
            tool_pipeline_normalize_contract_json(&args[2], out);
        }
        "tool-pipeline-contract-example-json" => {
            print_tool_pipeline_contract_example_json();
        }
        "tool-pipeline-contract-selftest-json" => {
            print_tool_pipeline_contract_selftest_json();
        }
        "tool-pipeline-contract-ci-gate-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli tool-pipeline-contract-ci-gate-json <file.json> [--warn-as-fail]");
                process::exit(1);
            }
            let warn_as_fail = args.iter().skip(3).any(|a| a == "--warn-as-fail");
            tool_pipeline_contract_ci_gate_json(&args[2], warn_as_fail);
        }
        "tool-pipeline-contract-diff-json" => {
            if args.len() < 4 {
                eprintln!(
                    "Usage: matter-cli tool-pipeline-contract-diff-json <baseline.json> <candidate.json>"
                );
                process::exit(1);
            }
            tool_pipeline_contract_diff_json(&args[2], &args[3]);
        }
        "tool-pipeline-contract-upgrade-advice-json" => {
            if args.len() < 4 {
                eprintln!(
                    "Usage: matter-cli tool-pipeline-contract-upgrade-advice-json <baseline.json> <candidate.json>"
                );
                process::exit(1);
            }
            tool_pipeline_contract_upgrade_advice_json(&args[2], &args[3]);
        }
        "tool-pipeline-contract-bundle-json" => {
            if args.len() < 4 {
                eprintln!(
                    "Usage: matter-cli tool-pipeline-contract-bundle-json <baseline.json> <candidate.json>"
                );
                process::exit(1);
            }
            tool_pipeline_contract_bundle_json(&args[2], &args[3]);
        }
        "tool-pipeline-contract-bundle-example-json" => {
            print_tool_pipeline_contract_bundle_example_json();
        }
        "tool-pipeline-apply-next-cycle-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli tool-pipeline-apply-next-cycle-json <next_cycle_config.json>");
                process::exit(1);
            }
            tool_pipeline_apply_next_cycle_json(&args[2]);
        }

        "init" => {
            let options = parse_init_options(&args[2..], false);
            init_project(&options, false);
        }

        "init-json" => {
            let options = parse_init_options(&args[2..], true);
            init_project(&options, true);
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
            let mut manifest = "matter.toml";
            let mut with_energy = false;
            for arg in args.iter().skip(2) {
                if arg == "--with-energy" {
                    with_energy = true;
                } else {
                    manifest = arg;
                }
            }
            project_run_json(manifest, with_energy);
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

        "project-run-build-json" => {
            let mut manifest = "matter.toml";
            let mut output: Option<&str> = None;
            let mut with_energy = false;
            let mut i = 2usize;
            while i < args.len() {
                if args[i] == "--with-energy" {
                    with_energy = true;
                    i += 1;
                } else if args[i] == "-o" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli project-run-build-json [matter.toml] [-o out] [--with-energy]");
                        process::exit(1);
                    }
                    output = Some(args[i + 1].as_str());
                    i += 2;
                } else {
                    manifest = &args[i];
                    i += 1;
                }
            }
            project_run_build_json(manifest, output, with_energy);
        }

        "project-emit-build-json" => {
            if args.len() < 4 {
                eprintln!(
                    "Usage: matter-cli project-emit-build-json [matter.toml] <event> [-o out] [--with-energy]"
                );
                process::exit(1);
            }
            let manifest = &args[2];
            let event = &args[3];
            let mut output: Option<&str> = None;
            let mut with_energy = false;
            let mut i = 4usize;
            while i < args.len() {
                if args[i] == "--with-energy" {
                    with_energy = true;
                    i += 1;
                } else if args[i] == "-o" {
                    if i + 1 >= args.len() {
                        eprintln!(
                            "Usage: matter-cli project-emit-build-json [matter.toml] <event> [-o out] [--with-energy]"
                        );
                        process::exit(1);
                    }
                    output = Some(args[i + 1].as_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            project_emit_build_json(manifest, event, output, with_energy);
        }

        "project-visual-step-build-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli project-visual-step-build-json [matter.toml] <events.json> <delta_ms> [-o out] [--with-energy]");
                process::exit(1);
            }
            let manifest = &args[2];
            let events_path = &args[3];
            let delta_ms = &args[4];
            let mut output: Option<&str> = None;
            let mut with_energy = false;
            let mut i = 5usize;
            while i < args.len() {
                if args[i] == "--with-energy" {
                    with_energy = true;
                    i += 1;
                } else if args[i] == "-o" && i + 1 < args.len() {
                    output = Some(args[i + 1].as_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            project_visual_step_build_json(manifest, events_path, delta_ms, output, with_energy);
        }

        "project-visual-run-build-json" => {
            if args.len() < 6 {
                eprintln!("Usage: matter-cli project-visual-run-build-json [matter.toml] <events.json> <frames> <delta_ms> [-o out] [--with-energy]");
                process::exit(1);
            }
            let manifest = &args[2];
            let events_path = &args[3];
            let frames = &args[4];
            let delta_ms = &args[5];
            let mut output: Option<&str> = None;
            let mut with_energy = false;
            let mut i = 6usize;
            while i < args.len() {
                if args[i] == "--with-energy" {
                    with_energy = true;
                    i += 1;
                } else if args[i] == "-o" && i + 1 < args.len() {
                    output = Some(args[i + 1].as_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            project_visual_run_build_json(
                manifest,
                events_path,
                frames,
                delta_ms,
                output,
                with_energy,
            );
        }

        "project-web-build-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli project-web-build-json [matter.toml] <output_dir> <app_name>");
                process::exit(1);
            }
            let manifest = &args[2];
            let output_dir = &args[3];
            let app_name = &args[4];
            project_web_build_json(manifest, output_dir, app_name);
        }

        "web-serve-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli web-serve-json <dir> [port] [--once]");
                process::exit(1);
            }
            let dir = &args[2];
            let mut port: Option<&str> = None;
            let mut once = false;
            for arg in args.iter().skip(3) {
                if arg == "--once" {
                    once = true;
                } else if port.is_none() {
                    port = Some(arg.as_str());
                }
            }
            web_serve_json(dir, port, once);
        }

        "project-web-serve-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli project-web-serve-json [matter.toml] <output_dir> <app_name> [port] [--once]");
                process::exit(1);
            }
            let manifest = &args[2];
            let output_dir = &args[3];
            let app_name = &args[4];
            let mut port: Option<&str> = None;
            let mut once = false;
            for arg in args.iter().skip(5) {
                if arg == "--once" {
                    once = true;
                } else if port.is_none() {
                    port = Some(arg.as_str());
                }
            }
            project_web_serve_json(manifest, output_dir, app_name, port, once);
        }

        "project-web-smoke-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli project-web-smoke-json [matter.toml] <output_dir> <app_name> [port]");
                process::exit(1);
            }
            let manifest = &args[2];
            let output_dir = &args[3];
            let app_name = &args[4];
            let port = if args.len() >= 6 {
                Some(args[5].as_str())
            } else {
                None
            };
            project_web_smoke_json(manifest, output_dir, app_name, port);
        }

        "project-web-ci-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli project-web-ci-json [matter.toml] <output_dir> <app_name> [port]");
                process::exit(1);
            }
            let manifest = &args[2];
            let output_dir = &args[3];
            let app_name = &args[4];
            let port = if args.len() >= 6 {
                Some(args[5].as_str())
            } else {
                None
            };
            project_web_ci_json(manifest, output_dir, app_name, port);
        }

        "web-events-save-json" => {
            if args.len() < 4 {
                eprintln!(
                    "Usage: matter-cli web-events-save-json <port> <output_events.json> [--clear]"
                );
                process::exit(1);
            }
            let port = &args[2];
            let output_events = &args[3];
            let clear = args.iter().skip(4).any(|arg| arg == "--clear");
            web_events_save_json(port, output_events, clear);
        }

        "web-state-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli web-state-json <port>");
                process::exit(1);
            }
            web_state_json(&args[2]);
        }

        "web-events-tail-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli web-events-tail-json <port> [cursor] [limit] [--ack]");
                process::exit(1);
            }
            let port = &args[2];
            let mut cursor: Option<&str> = None;
            let mut limit: Option<&str> = None;
            let mut ack = false;
            for arg in args.iter().skip(3) {
                if arg == "--ack" {
                    ack = true;
                } else if cursor.is_none() {
                    cursor = Some(arg.as_str());
                } else if limit.is_none() {
                    limit = Some(arg.as_str());
                }
            }
            web_events_tail_json(port, cursor, limit, ack);
        }

        "web-action-json" => {
            if args.len() < 4 {
                eprintln!("Usage: matter-cli web-action-json <port> <action>");
                process::exit(1);
            }
            web_action_json(&args[2], &args[3]);
        }

        "web-actions-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli web-actions-json <port>");
                process::exit(1);
            }
            web_actions_json(&args[2]);
        }

        "web-live-demo-check-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli web-live-demo-check-json <port> [timeout_ms]");
                process::exit(1);
            }
            let timeout_ms = if args.len() >= 4 {
                Some(args[3].as_str())
            } else {
                None
            };
            web_live_demo_check_json(&args[2], timeout_ms);
        }

        "project-web-step-live-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli project-web-step-live-json [matter.toml] <port> <delta_ms> [--clear]");
                process::exit(1);
            }
            let manifest = &args[2];
            let port = &args[3];
            let delta_ms = &args[4];
            let clear = args.iter().skip(5).any(|arg| arg == "--clear");
            project_web_step_live_json(manifest, port, delta_ms, clear);
        }

        "project-web-loop-live-json" => {
            if args.len() < 6 {
                eprintln!("Usage: matter-cli project-web-loop-live-json [matter.toml] <port> <delta_ms> <ticks|forever> [--interval-ms N] [--no-clear] [--telemetry-ms N]");
                process::exit(1);
            }
            let manifest = &args[2];
            let port = &args[3];
            let delta_ms = &args[4];
            let ticks = &args[5];
            let mut interval_ms: i64 = 120;
            let mut telemetry_ms: i64 = 1000;
            let mut no_clear = false;
            let mut i = 6usize;
            while i < args.len() {
                if args[i] == "--interval-ms" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage error: --interval-ms requires an integer value");
                        process::exit(1);
                    }
                    interval_ms = args[i + 1].parse::<i64>().unwrap_or_else(|_| {
                        eprintln!("Usage error: --interval-ms must be an integer");
                        process::exit(1);
                    });
                    i += 2;
                } else if args[i] == "--telemetry-ms" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage error: --telemetry-ms requires an integer value");
                        process::exit(1);
                    }
                    telemetry_ms = args[i + 1].parse::<i64>().unwrap_or_else(|_| {
                        eprintln!("Usage error: --telemetry-ms must be an integer");
                        process::exit(1);
                    });
                    i += 2;
                } else if args[i] == "--no-clear" {
                    no_clear = true;
                    i += 1;
                } else {
                    eprintln!("Usage error: unknown flag '{}'", args[i]);
                    process::exit(1);
                }
            }
            project_web_loop_live_json(
                manifest,
                port,
                delta_ms,
                ticks,
                interval_ms,
                !no_clear,
                telemetry_ms,
            );
        }

        "start-live-demo-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli start-live-demo-json [matter.toml] <output_dir> <app_name> <port> [--delta-ms N] [--interval-ms N] [--telemetry-ms N]");
                process::exit(1);
            }
            let manifest = &args[2];
            let output_dir = &args[3];
            let app_name = &args[4];
            let port = if args.len() >= 6 { &args[5] } else { "8099" };
            let mut delta_ms: i64 = 16;
            let mut interval_ms: i64 = 120;
            let mut telemetry_ms: i64 = 1000;
            let mut i = 6usize;
            while i < args.len() {
                if args[i] == "--delta-ms" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage error: --delta-ms requires an integer value");
                        process::exit(1);
                    }
                    delta_ms = args[i + 1].parse::<i64>().unwrap_or_else(|_| {
                        eprintln!("Usage error: --delta-ms must be an integer");
                        process::exit(1);
                    });
                    i += 2;
                } else if args[i] == "--interval-ms" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage error: --interval-ms requires an integer value");
                        process::exit(1);
                    }
                    interval_ms = args[i + 1].parse::<i64>().unwrap_or_else(|_| {
                        eprintln!("Usage error: --interval-ms must be an integer");
                        process::exit(1);
                    });
                    i += 2;
                } else if args[i] == "--telemetry-ms" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage error: --telemetry-ms requires an integer value");
                        process::exit(1);
                    }
                    telemetry_ms = args[i + 1].parse::<i64>().unwrap_or_else(|_| {
                        eprintln!("Usage error: --telemetry-ms must be an integer");
                        process::exit(1);
                    });
                    i += 2;
                } else {
                    eprintln!("Usage error: unknown flag '{}'", args[i]);
                    process::exit(1);
                }
            }
            start_live_demo_json(
                manifest,
                output_dir,
                app_name,
                port,
                delta_ms,
                interval_ms,
                telemetry_ms,
            );
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
                eprintln!("Usage: matter-cli run-json <file.matter|-> [--with-energy]");
                process::exit(1);
            }
            let with_energy = args.iter().skip(3).any(|arg| arg == "--with-energy");
            run_json(&args[2], with_energy);
        }
        "emnr-run-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli emnr-run-json <file.matter|->");
                process::exit(1);
            }
            emnr_run_json(&args[2]);
        }
        "emnr-run-stream-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-run-stream-json <file1.matter> [file2.matter ...]"
                );
                process::exit(1);
            }
            emnr_run_stream_json(&args[2..]);
        }
        "emnr-gate-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-gate-json <file1.matter> [file2.matter ...] [--policy file.json] [--out report.json] [--state-in state.json] [--state-out state.json] [--warn-as-fail] [--risk-threshold N] [--confidence-threshold N] [--energy-threshold N] [--critical-energy N]"
                );
                process::exit(1);
            }
            let warn_as_fail = args.iter().skip(2).any(|arg| arg == "--warn-as-fail");
            let options = parse_emnr_gate_args(&args[2..], warn_as_fail);
            if options.paths.is_empty() {
                eprintln!("Usage: matter-cli emnr-gate-json <file1.matter> [file2.matter ...] [--policy file.json] [--out report.json] [--state-in state.json] [--state-out state.json] [--warn-as-fail] [--risk-threshold N] [--confidence-threshold N] [--energy-threshold N] [--critical-energy N]");
                process::exit(1);
            }
            emnr_gate_json(&options);
        }
        "emnr-train-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-train-json <file1.matter> [file2.matter ...] [--epochs N] [--state-in state.json] [--state-out state.json] [--out report.json]"
                );
                process::exit(1);
            }
            let options = parse_emnr_train_args(&args[2..]);
            if options.paths.is_empty() {
                eprintln!("Usage: matter-cli emnr-train-json <file1.matter> [file2.matter ...] [--epochs N] [--state-in state.json] [--state-out state.json] [--out report.json]");
                process::exit(1);
            }
            emnr_train_json(&options);
        }
        "emnr-promote-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-promote-json <file1.matter> [file2.matter ...] --baseline-state state.json --candidate-state state.json [--out report.json] [--max-risk-regression N] [--max-error-regression N] [--max-confidence-drop N] [--min-energy N]"
                );
                process::exit(1);
            }
            let options = parse_emnr_promote_args(&args[2..]);
            if options.paths.is_empty() {
                eprintln!("Usage: matter-cli emnr-promote-json <file1.matter> [file2.matter ...] --baseline-state state.json --candidate-state state.json [--out report.json] [--max-risk-regression N] [--max-error-regression N] [--max-confidence-drop N] [--min-energy N]");
                process::exit(1);
            }
            emnr_promote_json(&options);
        }
        "emnr-audit-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-audit-json <report1.json> [report2.json ...] [--out audit.json] [--fail-on-warn]"
                );
                process::exit(1);
            }
            let options = parse_emnr_audit_args(&args[2..]);
            if options.paths.is_empty() {
                eprintln!("Usage: matter-cli emnr-audit-json <report1.json> [report2.json ...] [--out audit.json] [--fail-on-warn]");
                process::exit(1);
            }
            emnr_audit_json(&options);
        }
        "emnr-pipeline-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-pipeline-json <file1.matter> [file2.matter ...] [--epochs N] [--out-dir dir] [--policy file.json] [--fail-on-warn]"
                );
                process::exit(1);
            }
            let options = parse_emnr_pipeline_args(&args[2..]);
            if options.paths.is_empty() {
                eprintln!("Usage: matter-cli emnr-pipeline-json <file1.matter> [file2.matter ...] [--epochs N] [--out-dir dir] [--policy file.json] [--fail-on-warn]");
                process::exit(1);
            }
            emnr_pipeline_json(&options);
        }
        "emnr-ci-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-ci-json <file1.matter> [file2.matter ...] [--epochs N] [--out-dir dir] [--policy file.json] [--baseline-manifest manifest.json|--baseline-index index.json] [--require-non-regression] [--require-pass] [--fail-on-warn] [--out report.json]"
                );
                process::exit(1);
            }
            let options = parse_emnr_ci_args(&args[2..]);
            if options.pipeline.paths.is_empty() {
                eprintln!("Usage: matter-cli emnr-ci-json <file1.matter> [file2.matter ...] [--epochs N] [--out-dir dir] [--policy file.json] [--baseline-manifest manifest.json|--baseline-index index.json] [--require-non-regression] [--require-pass] [--fail-on-warn] [--out report.json]");
                process::exit(1);
            }
            emnr_ci_json(&options);
        }
        "emnr-ci-scorecard-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-ci-scorecard-json <ci-report.json> [--out scorecard.json] [--require-pass]"
                );
                process::exit(1);
            }
            let output = option_value(&args[3..], "--out");
            let require_pass = args.iter().skip(3).any(|arg| arg == "--require-pass");
            emnr_ci_scorecard_json(&args[2], output, require_pass);
        }
        "emnr-ci-history-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-ci-history-json <scorecard1.json> [scorecard2.json ...] [--out history.json] [--require-stable]"
                );
                process::exit(1);
            }
            let output = option_value(&args[3..], "--out");
            let require_stable = args.iter().skip(3).any(|arg| arg == "--require-stable");
            let paths = collect_positional_args(&args[2..], &["--out"]);
            if paths.is_empty() {
                eprintln!("Usage: matter-cli emnr-ci-history-json <scorecard1.json> [scorecard2.json ...] [--out history.json] [--require-stable]");
                process::exit(1);
            }
            emnr_ci_history_json(&paths, output, require_stable);
        }
        "emnr-ci-bundle-json" => {
            if args.len() < 5 {
                eprintln!(
                    "Usage: matter-cli emnr-ci-bundle-json <ci-report.json> --out-dir dir [--require-pass] [--require-stable]"
                );
                process::exit(1);
            }
            let out_dir = option_value(&args[3..], "--out-dir").unwrap_or_else(|| {
                eprintln!("Usage error: --out-dir is required");
                process::exit(1);
            });
            let require_pass = args.iter().skip(3).any(|arg| arg == "--require-pass");
            let require_stable = args.iter().skip(3).any(|arg| arg == "--require-stable");
            emnr_ci_bundle_json(&args[2], out_dir, require_pass, require_stable);
        }
        "emnr-verify-ci-bundle-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-verify-ci-bundle-json <bundle.json> [--require-pass]"
                );
                process::exit(1);
            }
            let require_pass = args.iter().skip(3).any(|arg| arg == "--require-pass");
            emnr_verify_ci_bundle_json(&args[2], require_pass);
        }
        "emnr-ci-bundle-summary-md" => {
            if args.len() < 5 {
                eprintln!(
                    "Usage: matter-cli emnr-ci-bundle-summary-md <bundle.json> --out summary.md"
                );
                process::exit(1);
            }
            let output = option_value(&args[3..], "--out").unwrap_or_else(|| {
                eprintln!("Usage error: --out is required");
                process::exit(1);
            });
            emnr_ci_bundle_summary_md(&args[2], output);
        }
        "emnr-promote-ci-bundle-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli emnr-promote-ci-bundle-json <bundle.json> --index-out index.json [--require-pass] [--out report.json]");
                process::exit(1);
            }
            let index_out = option_value(&args[3..], "--index-out").unwrap_or_else(|| {
                eprintln!("Usage error: --index-out is required");
                process::exit(1);
            });
            let output = option_value(&args[3..], "--out");
            let require_pass = args.iter().skip(3).any(|arg| arg == "--require-pass");
            emnr_promote_ci_bundle_json(&args[2], index_out, output, require_pass);
        }
        "emnr-ci-bundle-index-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli emnr-ci-bundle-index-json <index.json> [--latest N] [--require-current-approved]");
                process::exit(1);
            }
            let latest = option_value(&args[3..], "--latest")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(5);
            let require_current_approved = args
                .iter()
                .skip(3)
                .any(|arg| arg == "--require-current-approved");
            emnr_ci_bundle_index_json(&args[2], latest, require_current_approved);
        }
        "emnr-export-ci-bundle-index-json" => {
            if args.len() < 5 {
                eprintln!(
                    "Usage: matter-cli emnr-export-ci-bundle-index-json <index.json> --out-dir dir"
                );
                process::exit(1);
            }
            let out_dir = option_value(&args[3..], "--out-dir").unwrap_or_else(|| {
                eprintln!("Usage error: --out-dir is required");
                process::exit(1);
            });
            emnr_export_ci_bundle_index_json(&args[2], out_dir);
        }
        "emnr-verify-ci-bundle-package-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-verify-ci-bundle-package-json <package.json> [--require-pass]"
                );
                process::exit(1);
            }
            let require_pass = args.iter().skip(3).any(|arg| arg == "--require-pass");
            emnr_verify_ci_bundle_package_json(&args[2], require_pass);
        }
        "emnr-import-ci-bundle-package-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli emnr-import-ci-bundle-package-json <package.json> --out-dir dir [--require-pass]");
                process::exit(1);
            }
            let out_dir = option_value(&args[3..], "--out-dir").unwrap_or_else(|| {
                eprintln!("Usage error: --out-dir is required");
                process::exit(1);
            });
            let require_pass = args.iter().skip(3).any(|arg| arg == "--require-pass");
            emnr_import_ci_bundle_package_json(&args[2], out_dir, require_pass);
        }
        "emnr-init-json" => {
            let force = args.iter().skip(2).any(|arg| arg == "--force");
            emnr_init_json(force);
        }
        "emnr-sync-json" => {
            let force = args.iter().skip(2).any(|arg| arg == "--force");
            let require_pass = args.iter().skip(2).any(|arg| arg == "--require-pass");
            emnr_sync_json(force, require_pass);
        }
        "emnr-doctor-json" => {
            let require_pass = args.iter().skip(2).any(|arg| arg == "--require-pass");
            emnr_doctor_json(require_pass);
        }
        "emnr-release-gate-json" => {
            let policy_path = option_value(&args[2..], "--policy").unwrap_or(".emnr/policy.json");
            let index_path =
                option_value(&args[2..], "--bundle-index").unwrap_or(".emnr/bundle-index.json");
            let require_pass = args.iter().skip(2).any(|arg| arg == "--require-pass");
            emnr_release_gate_json(policy_path, index_path, require_pass);
        }
        "emnr-agent-advice-json" => {
            let policy_path = option_value(&args[2..], "--policy").unwrap_or(".emnr/policy.json");
            let index_path =
                option_value(&args[2..], "--bundle-index").unwrap_or(".emnr/bundle-index.json");
            let require_pass = args.iter().skip(2).any(|arg| arg == "--require-pass");
            emnr_agent_advice_json(policy_path, index_path, require_pass);
        }
        "emnr-status-json" => {
            let policy_path = option_value(&args[2..], "--policy").unwrap_or(".emnr/policy.json");
            let index_path =
                option_value(&args[2..], "--bundle-index").unwrap_or(".emnr/bundle-index.json");
            let require_pass = args.iter().skip(2).any(|arg| arg == "--require-pass");
            emnr_status_json(policy_path, index_path, require_pass);
        }
        "emnr-status-md" => {
            let policy_path = option_value(&args[2..], "--policy").unwrap_or(".emnr/policy.json");
            let index_path =
                option_value(&args[2..], "--bundle-index").unwrap_or(".emnr/bundle-index.json");
            let output = option_value(&args[2..], "--out").unwrap_or(".emnr/status.md");
            let require_pass = args.iter().skip(2).any(|arg| arg == "--require-pass");
            emnr_status_md(policy_path, index_path, output, require_pass);
        }
        "emnr-verify-manifest-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-verify-manifest-json <manifest.json> [--require-pass]"
                );
                process::exit(1);
            }
            let require_pass = args.iter().skip(3).any(|arg| arg == "--require-pass");
            emnr_verify_manifest_json(&args[2], require_pass);
        }
        "emnr-compare-manifests-json" => {
            if args.len() < 4 {
                eprintln!("Usage: matter-cli emnr-compare-manifests-json <baseline-manifest.json> <candidate-manifest.json> [--require-non-regression]");
                process::exit(1);
            }
            let require_non_regression = args
                .iter()
                .skip(4)
                .any(|arg| arg == "--require-non-regression");
            emnr_compare_manifests_json(&args[2], &args[3], require_non_regression);
        }
        "emnr-promote-baseline-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli emnr-promote-baseline-json <manifest.json> --baseline-out baseline.json [--index-out index.json] [--allow-warn] [--out report.json]");
                process::exit(1);
            }
            let options = parse_emnr_baseline_promote_args(&args[2..]);
            emnr_promote_baseline_json(&options);
        }
        "emnr-baseline-index-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli emnr-baseline-index-json <index.json> [--latest N]");
                process::exit(1);
            }
            let latest = option_value(&args[3..], "--latest")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(5);
            emnr_baseline_index_json(&args[2], latest);
        }
        "emnr-export-baseline-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli emnr-export-baseline-json <index.json> --out-dir dir");
                process::exit(1);
            }
            let out_dir = option_value(&args[3..], "--out-dir").unwrap_or_else(|| {
                eprintln!("Usage error: --out-dir is required");
                process::exit(1);
            });
            emnr_export_baseline_json(&args[2], out_dir);
        }
        "emnr-verify-baseline-package-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli emnr-verify-baseline-package-json <package.json> [--require-pass]");
                process::exit(1);
            }
            let require_pass = args.iter().skip(3).any(|arg| arg == "--require-pass");
            emnr_verify_baseline_package_json(&args[2], require_pass);
        }
        "emnr-import-baseline-package-json" => {
            if args.len() < 5 {
                eprintln!("Usage: matter-cli emnr-import-baseline-package-json <package.json> --out-dir dir [--require-pass]");
                process::exit(1);
            }
            let out_dir = option_value(&args[3..], "--out-dir").unwrap_or_else(|| {
                eprintln!("Usage error: --out-dir is required");
                process::exit(1);
            });
            let require_pass = args.iter().skip(3).any(|arg| arg == "--require-pass");
            emnr_import_baseline_package_json(&args[2], out_dir, require_pass);
        }
        "emnr-baseline-doctor-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli emnr-baseline-doctor-json <baseline-dir> [--require-pass]"
                );
                process::exit(1);
            }
            let require_pass = args.iter().skip(3).any(|arg| arg == "--require-pass");
            emnr_baseline_doctor_json(&args[2], require_pass);
        }
        "emnr-state-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli emnr-state-json <state.json> [--full]");
                process::exit(1);
            }
            let full = args.iter().skip(3).any(|arg| arg == "--full");
            emnr_state_json(&args[2], full);
        }
        "run-energy" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli run-energy <file.matter|->");
                process::exit(1);
            }
            run_energy(&args[2]);
        }
        "run-energy-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli run-energy-json <file.matter|->");
                process::exit(1);
            }
            run_energy_json(&args[2]);
        }

        "doctor" => {
            print_doctor();
        }

        "doctor-json" => {
            print_doctor_json();
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
                eprintln!("Usage: matter-cli emit-json <file.matter|-> <event> [--with-energy]");
                process::exit(1);
            }
            let with_energy = args.iter().skip(4).any(|arg| arg == "--with-energy");
            emit_json(&args[2], &args[3], with_energy);
        }

        "tool-wire-validate-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli tool-wire-validate-json <wire_file>");
                process::exit(1);
            }
            tool_wire_validate_json(&args[2]);
        }

        "tool-wire-merge-json" => {
            if args.len() < 4 {
                eprintln!("Usage: matter-cli tool-wire-merge-json <left_wire_file> <right_wire_file> [strategy]");
                process::exit(1);
            }
            let strategy = if args.len() >= 5 {
                Some(args[4].as_str())
            } else {
                None
            };
            tool_wire_merge_json(&args[2], &args[3], strategy);
        }

        "tool-frame-invoke-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli tool-frame-invoke-json <frame_payload.json> [-o wire.out]"
                );
                process::exit(1);
            }
            let mut output: Option<&str> = None;
            let mut i = 3usize;
            while i < args.len() {
                if args[i] == "-o" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-frame-invoke-json <frame_payload.json> [-o wire.out]");
                        process::exit(1);
                    }
                    output = Some(args[i + 1].as_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            tool_frame_invoke_json(&args[2], output);
        }

        "tool-frame-template-json" => {
            if args.len() >= 4 && args[2] == "-o" {
                tool_frame_template_json(Some(&args[3]));
            } else if args.len() == 2 {
                tool_frame_template_json(None);
            } else {
                eprintln!("Usage: matter-cli tool-frame-template-json [-o frame_payload.json]");
                process::exit(1);
            }
        }

        "tool-wire-extract-json" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter-cli tool-wire-extract-json <invoke_result.json> [-o wire.out]"
                );
                process::exit(1);
            }
            let mut output: Option<&str> = None;
            let mut i = 3usize;
            while i < args.len() {
                if args[i] == "-o" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-wire-extract-json <invoke_result.json> [-o wire.out]");
                        process::exit(1);
                    }
                    output = Some(args[i + 1].as_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            tool_wire_extract_json(&args[2], output);
        }

        "tool-pipeline-demo-json" => {
            let mut out_dir = "tool_pipeline_demo";
            let mut strict = false;
            let mut strategy = "prefer_blocked";
            let mut compare_strategies = false;
            let mut artifact_manifest_json = false;
            let mut emit_contract_bundle = false;
            let mut apply_recommended_energy_mode = false;
            let mut next_cycle_apply_now = false;
            let mut emit_summary_md = false;
            let mut emit_github_step_summary = false;
            let mut summary_format = "none".to_string();
            let mut fail_on_status: Option<String> = None;
            let mut ci_exit_codes = false;
            let mut dry_run = false;
            let mut artifact_prefix = String::new();
            let mut scoring = PipelineScoring::balanced();
            let mut energy_mode: Option<PipelineEnergyMode> = None;
            let mut confidence_threshold_low = 5.0f64;
            let mut confidence_threshold_high = 12.0f64;
            let mut confidence_profile = "balanced".to_string();
            let mut confidence_manual_override = false;
            let mut require_catalog_hash: Option<String> = None;
            let mut contract_bundle_baseline: Option<String> = None;
            let mut next_cycle_config_out: Option<String> = None;
            let mut next_cycle_max_hops: u64 = 3;
            let mut i = 2usize;
            while i < args.len() {
                if args[i] == "--strict" {
                    strict = true;
                    i += 1;
                } else if args[i] == "--compare-strategies" {
                    compare_strategies = true;
                    i += 1;
                } else if args[i] == "--artifact-manifest-json" {
                    artifact_manifest_json = true;
                    i += 1;
                } else if args[i] == "--emit-contract-bundle" {
                    emit_contract_bundle = true;
                    i += 1;
                } else if args[i] == "--apply-recommended-energy-mode" {
                    apply_recommended_energy_mode = true;
                    i += 1;
                } else if args[i] == "--next-cycle-apply-now" {
                    next_cycle_apply_now = true;
                    i += 1;
                } else if args[i] == "--emit-summary-md" {
                    emit_summary_md = true;
                    summary_format = "md".to_string();
                    i += 1;
                } else if args[i] == "--emit-github-step-summary" {
                    emit_github_step_summary = true;
                    summary_format = "md".to_string();
                    i += 1;
                } else if args[i] == "--summary-format" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--strict] [--dry-run] [--compare-strategies] [--emit-summary-md] [--emit-github-step-summary] [--summary-format json|md|both] [--artifact-prefix name] [--fail-on-status status] [--strategy prefer_latest|prefer_blocked|prefer_terminal] [--score-preset conservative|balanced|aggressive]");
                        process::exit(1);
                    }
                    let normalized = args[i + 1].to_ascii_lowercase();
                    if normalized != "json" && normalized != "md" && normalized != "both" {
                        eprintln!(
                            "Usage error: invalid --summary-format '{}'. Use json|md|both",
                            args[i + 1]
                        );
                        process::exit(1);
                    }
                    summary_format = normalized;
                    i += 2;
                } else if args[i] == "--fail-on-status" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--strict] [--dry-run] [--compare-strategies] [--emit-summary-md] [--emit-github-step-summary] [--artifact-prefix name] [--fail-on-status status] [--strategy prefer_latest|prefer_blocked|prefer_terminal] [--score-preset conservative|balanced|aggressive]");
                        process::exit(1);
                    }
                    fail_on_status = Some(args[i + 1].to_ascii_lowercase());
                    i += 2;
                } else if args[i] == "--artifact-prefix" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--strict] [--dry-run] [--compare-strategies] [--emit-summary-md] [--emit-github-step-summary] [--artifact-prefix name] [--summary-format json|md|both] [--fail-on-status status] [--strategy prefer_latest|prefer_blocked|prefer_terminal] [--score-preset conservative|balanced|aggressive]");
                        process::exit(1);
                    }
                    artifact_prefix = args[i + 1].to_string();
                    i += 2;
                } else if args[i] == "--ci-exit-codes" {
                    ci_exit_codes = true;
                    i += 1;
                } else if args[i] == "--dry-run" {
                    dry_run = true;
                    i += 1;
                } else if args[i] == "--score-preset" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--strict] [--compare-strategies] [--strategy auto|prefer_latest|prefer_blocked|prefer_terminal] [--score-preset conservative|balanced|aggressive]");
                        process::exit(1);
                    }
                    scoring = match PipelineScoring::from_preset(&args[i + 1]) {
                        Some(preset) => preset,
                        None => {
                            eprintln!(
                                "Usage error: unknown --score-preset '{}'. Use conservative|balanced|aggressive",
                                args[i + 1]
                            );
                            process::exit(1);
                        }
                    };
                    i += 2;
                } else if args[i] == "--energy-mode" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--energy-mode eco|balanced|performance|adaptive|critical]");
                        process::exit(1);
                    }
                    energy_mode = match normalize_energy_mode(&args[i + 1]) {
                        Some(mode) => Some(mode),
                        None => {
                            eprintln!(
                                "Usage error: unknown --energy-mode '{}'. Use eco|balanced|performance|adaptive|critical",
                                args[i + 1]
                            );
                            process::exit(1);
                        }
                    };
                    i += 2;
                } else if args[i] == "--strategy" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--strict] [--compare-strategies] [--strategy auto|prefer_latest|prefer_blocked|prefer_terminal]");
                        process::exit(1);
                    }
                    strategy = match normalize_strategy(&args[i + 1]) {
                        Some(value) => value,
                        None => {
                            eprintln!(
                                "Usage error: unknown strategy '{}'. Use auto|prefer_latest|prefer_blocked|prefer_terminal",
                                args[i + 1]
                            );
                            process::exit(1);
                        }
                    };
                    i += 2;
                } else if args[i] == "--score-status-ok" && i + 1 < args.len() {
                    scoring.status_ok = args[i + 1].parse::<f64>().unwrap_or(scoring.status_ok);
                    i += 2;
                } else if args[i] == "--score-status-degraded" && i + 1 < args.len() {
                    scoring.status_degraded = args[i + 1]
                        .parse::<f64>()
                        .unwrap_or(scoring.status_degraded);
                    i += 2;
                } else if args[i] == "--score-status-unknown" && i + 1 < args.len() {
                    scoring.status_unknown =
                        args[i + 1].parse::<f64>().unwrap_or(scoring.status_unknown);
                    i += 2;
                } else if args[i] == "--score-action-execute" && i + 1 < args.len() {
                    scoring.action_execute =
                        args[i + 1].parse::<f64>().unwrap_or(scoring.action_execute);
                    i += 2;
                } else if args[i] == "--score-action-resolve-blockers" && i + 1 < args.len() {
                    scoring.action_resolve_blockers = args[i + 1]
                        .parse::<f64>()
                        .unwrap_or(scoring.action_resolve_blockers);
                    i += 2;
                } else if args[i] == "--score-action-none" && i + 1 < args.len() {
                    scoring.action_none = args[i + 1].parse::<f64>().unwrap_or(scoring.action_none);
                    i += 2;
                } else if args[i] == "--score-action-other" && i + 1 < args.len() {
                    scoring.action_other =
                        args[i + 1].parse::<f64>().unwrap_or(scoring.action_other);
                    i += 2;
                } else if args[i] == "--score-penalty-latest" && i + 1 < args.len() {
                    scoring.penalty_latest =
                        args[i + 1].parse::<f64>().unwrap_or(scoring.penalty_latest);
                    i += 2;
                } else if args[i] == "--score-penalty-blocked" && i + 1 < args.len() {
                    scoring.penalty_blocked = args[i + 1]
                        .parse::<f64>()
                        .unwrap_or(scoring.penalty_blocked);
                    i += 2;
                } else if args[i] == "--score-penalty-terminal" && i + 1 < args.len() {
                    scoring.penalty_terminal = args[i + 1]
                        .parse::<f64>()
                        .unwrap_or(scoring.penalty_terminal);
                    i += 2;
                } else if args[i] == "--confidence-threshold-low" && i + 1 < args.len() {
                    confidence_threshold_low = args[i + 1].parse::<f64>().unwrap_or(5.0);
                    confidence_manual_override = true;
                    confidence_profile = "custom".to_string();
                    i += 2;
                } else if args[i] == "--confidence-threshold-high" && i + 1 < args.len() {
                    confidence_threshold_high = args[i + 1].parse::<f64>().unwrap_or(12.0);
                    confidence_manual_override = true;
                    confidence_profile = "custom".to_string();
                    i += 2;
                } else if args[i] == "--confidence-profile" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--confidence-profile strict|balanced|relaxed]");
                        process::exit(1);
                    }
                    match confidence_profile_thresholds(&args[i + 1]) {
                        Some((low, high)) => {
                            confidence_threshold_low = low;
                            confidence_threshold_high = high;
                            confidence_profile = args[i + 1].to_ascii_lowercase();
                            confidence_manual_override = false;
                        }
                        None => {
                            eprintln!(
                                "Usage error: unknown --confidence-profile '{}'. Use strict|balanced|relaxed",
                                args[i + 1]
                            );
                            process::exit(1);
                        }
                    }
                    i += 2;
                } else if args[i] == "--require-catalog-hash" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--require-catalog-hash hash]");
                        process::exit(1);
                    }
                    require_catalog_hash = Some(args[i + 1].to_string());
                    i += 2;
                } else if args[i] == "--contract-bundle-baseline" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--contract-bundle-baseline baseline.json]");
                        process::exit(1);
                    }
                    contract_bundle_baseline = Some(args[i + 1].to_string());
                    i += 2;
                } else if args[i] == "--next-cycle-config-out" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--next-cycle-config-out file.json]");
                        process::exit(1);
                    }
                    next_cycle_config_out = Some(args[i + 1].to_string());
                    i += 2;
                } else if args[i] == "--next-cycle-max-hops" {
                    if i + 1 >= args.len() {
                        eprintln!("Usage: matter-cli tool-pipeline-demo-json [out_dir] [--next-cycle-max-hops N]");
                        process::exit(1);
                    }
                    next_cycle_max_hops = args[i + 1].parse::<u64>().unwrap_or(3);
                    i += 2;
                } else if args[i].starts_with("--") {
                    eprintln!(
                        "Usage error: unknown flag '{}'. Run 'matter-cli help' for valid options.",
                        args[i]
                    );
                    process::exit(1);
                } else {
                    out_dir = &args[i];
                    i += 1;
                }
            }
            if confidence_threshold_low < 0.0
                || confidence_threshold_high < 0.0
                || confidence_threshold_low > confidence_threshold_high
            {
                eprintln!("Usage error: confidence thresholds must satisfy 0 <= low <= high");
                process::exit(1);
            }
            tool_pipeline_demo_json(
                out_dir,
                strict,
                strategy,
                compare_strategies,
                artifact_manifest_json,
                emit_contract_bundle,
                apply_recommended_energy_mode,
                next_cycle_apply_now,
                emit_summary_md,
                emit_github_step_summary,
                &summary_format,
                fail_on_status.as_deref(),
                ci_exit_codes,
                dry_run,
                &artifact_prefix,
                &scoring,
                energy_mode,
                confidence_threshold_low,
                confidence_threshold_high,
                if confidence_manual_override {
                    "custom"
                } else {
                    &confidence_profile
                },
                require_catalog_hash.as_deref(),
                contract_bundle_baseline.as_deref(),
                next_cycle_config_out.as_deref(),
                next_cycle_max_hops,
            );
        }

        "visual-step-json" => {
            if args.len() < 5 {
                eprintln!(
                    "Usage: matter-cli visual-step-json <file.matter|-> <events.json> <delta_ms> [--with-energy]"
                );
                process::exit(1);
            }
            let with_energy = args.iter().skip(5).any(|arg| arg == "--with-energy");
            visual_step_json(&args[2], &args[3], &args[4], with_energy);
        }

        "visual-run-json" => {
            if args.len() < 6 {
                eprintln!("Usage: matter-cli visual-run-json <file.matter|-> <events.json> <frames> <delta_ms> [--with-energy]");
                process::exit(1);
            }
            let with_energy = args.iter().skip(6).any(|arg| arg == "--with-energy");
            visual_run_json(&args[2], &args[3], &args[4], &args[5], with_energy);
        }

        "studio-native" => {
            let input = args
                .iter()
                .skip(2)
                .find(|arg| !arg.starts_with("--"))
                .map(String::as_str)
                .unwrap_or("examples/matter_studio_ui.matter");
            let clear = !args.iter().skip(2).any(|arg| arg == "--no-clear");
            let interactive = args.iter().skip(2).any(|arg| arg == "--interactive");
            studio_native(input, clear, interactive);
        }

        "studio-native-json" => {
            let input = args
                .iter()
                .skip(2)
                .find(|arg| !arg.starts_with("--"))
                .map(String::as_str)
                .unwrap_or("examples/matter_studio_ui.matter");
            studio_native_json(input);
        }

        "sentinel-pvmbc" => {
            let input = first_positional_arg(&args[2..], &["-o", "--output", "--name"])
                .unwrap_or("examples/matter_studio_ui.matter");
            let output = option_value(&args[2..], "-o")
                .or_else(|| option_value(&args[2..], "--output"))
                .unwrap_or("target/matter-studio.pvmbc");
            let name = option_value(&args[2..], "--name").unwrap_or("matter-studio");
            sentinel_pvmbc(input, output, name);
        }

        "sentinel-pvmbc-rust-array" => {
            let input = first_positional_arg(&args[2..], &["--const", "--name"])
                .unwrap_or("examples/matter_studio_ui.matter");
            let const_name = option_value(&args[2..], "--const").unwrap_or("MATTER_STUDIO_PVMBC");
            let name = option_value(&args[2..], "--name").unwrap_or("matter-studio");
            sentinel_pvmbc_rust_array(input, const_name, name);
        }

        "sentinel-mbc1-rust-array" => {
            let input = first_positional_arg(&args[2..], &["--const"])
                .unwrap_or("examples/first_run.matter");
            let const_name = option_value(&args[2..], "--const").unwrap_or("MATTER_BOOT_MBC1");
            sentinel_mbc1_rust_array(input, const_name);
        }

        "sentinel-mbc1-kernel-check-json" => {
            let input = first_positional_arg(&args[2..], &["--budget"])
                .unwrap_or("examples/sentinel_boot.matter");
            let budget = option_value(&args[2..], "--budget")
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(10_000);
            sentinel_mbc1_kernel_check_json(input, budget);
        }

        "rust-ffi-call-json" => {
            if args.len() < 4 {
                eprintln!(
                    "Usage: matter-cli rust-ffi-call-json <library_path> <symbol> [args_json|@args.json]"
                );
                process::exit(1);
            }
            let args_json = args.get(4).map(String::as_str).unwrap_or("[]");
            rust_ffi_call_json(&args[2], &args[3], args_json);
        }

        "rust-ffi-validate-args-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli rust-ffi-validate-args-json <args_json|@args.json>");
                process::exit(1);
            }
            rust_ffi_validate_args_json(&args[2]);
        }

        "sentinel-pvmbc-inspect-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli sentinel-pvmbc-inspect-json <file.pvmbc>");
                process::exit(1);
            }
            sentinel_pvmbc_inspect_json(&args[2]);
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

        "reflect-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli reflect-json <file.matter|->");
                process::exit(1);
            }
            reflect_json(&args[2]);
        }

        "reflexive-guard-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli reflexive-guard-json <file.matter|-> [--max-statements N] [--max-functions N] [--allow-backends]");
                process::exit(1);
            }
            let options = parse_reflexive_guard_options(&args[3..]);
            reflexive_guard_json(&args[2], &options);
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
                eprintln!("Usage: matter-cli run-bytecode-json <file.mbc> [--with-energy]");
                process::exit(1);
            }
            let with_energy = args.iter().skip(3).any(|arg| arg == "--with-energy");
            run_bytecode_json(&args[2], with_energy);
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

        "show-ir" => {
            if args.len() < 3 {
                eprintln!("Usage: matter show-ir <file.matter>");
                process::exit(1);
            }
            show_llvm_ir(&args[2]);
        }

        "compile-native" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: matter compile-native <file.matter> [-o output] [-O0|-O1|-O2|-O3]"
                );
                process::exit(1);
            }

            let mut output = "output";
            let mut opt_level = None;
            let mut i = 3;

            while i < args.len() {
                match args[i].as_str() {
                    "-o" => {
                        if i + 1 < args.len() {
                            output = &args[i + 1];
                            i += 2;
                        } else {
                            eprintln!("Error: -o requires an output filename");
                            process::exit(1);
                        }
                    }
                    s if s.starts_with("-O") => {
                        opt_level = Some(s.to_string());
                        i += 1;
                    }
                    _ => {
                        eprintln!("Unknown option: {}", args[i]);
                        process::exit(1);
                    }
                }
            }

            compile_to_native(&args[2], output, opt_level.as_deref());
        }

        "run-native" => {
            if args.len() < 3 {
                eprintln!("Usage: matter run-native <file.matter> [-O0|-O1|-O2|-O3]");
                process::exit(1);
            }

            let opt_level = if args.len() >= 4 && args[3].starts_with("-O") {
                Some(args[3].as_str())
            } else {
                None
            };

            run_native(&args[2], opt_level);
        }

        "benchmark" => {
            if args.len() < 3 {
                eprintln!("Usage: matter benchmark <file.matter> [--iterations N] [--json]");
                process::exit(1);
            }

            let (iterations, json_output) = parse_benchmark_options(&args[3..]);

            benchmark_program(&args[2], iterations, json_output);
        }

        "benchmark-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter benchmark-json <file.matter> [--iterations N]");
                process::exit(1);
            }

            let (iterations, _) = parse_benchmark_options(&args[3..]);
            benchmark_program(&args[2], iterations, true);
        }

        "perf-diagnose-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter perf-diagnose-json <file.matter|->");
                process::exit(1);
            }

            perf_diagnose_json(&args[2]);
        }

        "benchmark-gate-json" => {
            if args.len() < 3 {
                eprintln!("Usage: matter benchmark-gate-json <benchmark.json> [--max-average-ns N] [--max-median-ns N] [--max-p95-ns N] [--ci-exit-codes]");
                process::exit(1);
            }

            let options = parse_benchmark_gate_options(&args[2], &args[3..]);
            let report_source = fs::read_to_string(&options.report_path).unwrap_or_else(|e| {
                eprintln!("Error reading benchmark report: {}", e);
                process::exit(1);
            });
            let (passed, payload) = evaluate_benchmark_gate(&report_source, &options);
            println!("{}", payload);
            if options.ci_exit_codes && !passed {
                process::exit(1);
            }
        }

        "help" => {
            if args.len() >= 3 {
                print_command_help(&args[2]);
            } else {
                print_help();
            }
        }

        "version" => {
            print_version();
        }

        "backends" => {
            print_backends();
        }

        "examples" => {
            if args.len() >= 3 {
                run_example(&args[2]);
            } else {
                list_examples();
            }
        }

        "repl" => {
            run_repl();
        }
        "agent-chat" => {
            run_agent_chat(&args[2..]);
        }

        "lsp" => {
            run_lsp();
        }

        "debug" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli debug <file.matter>");
                process::exit(1);
            }
            run_debug(&args[2]);
        }

        "format" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli format <file.matter> [--write]");
                process::exit(1);
            }
            let write = args.len() >= 4 && args[3] == "--write";
            run_format(&args[2], write);
        }

        "lint" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli lint <file.matter>");
                process::exit(1);
            }
            run_lint(&args[2]);
        }

        // Sprint 24 Phase 4: GC Commands
        "gc-stats" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli gc-stats <file.matter>");
                process::exit(1);
            }
            gc_stats(&args[2]);
        }

        "gc-collect" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli gc-collect <file.matter>");
                process::exit(1);
            }
            gc_collect(&args[2]);
        }

        "gc-profile" => {
            if args.len() < 3 {
                eprintln!("Usage: matter-cli gc-profile <file.matter>");
                process::exit(1);
            }
            gc_profile(&args[2]);
        }

        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!();
            suggest_command(command);
            eprintln!();
            eprintln!("Run 'matter-cli help' for usage information.");
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Matter CLI - Matter Core Language Runtime");
    println!();
    println!("Usage:");
    println!("  matter-cli capabilities-json                Print machine-readable capabilities");
    println!("  matter-cli tool-ci-catalog-json             Print CI decision reason/code catalog");
    println!("  matter-cli tool-ci-verify-json <reason> <code> Verify CI reason/code mapping");
    println!("  matter-cli tool-ci-contract-json            Print CI contract bundle");
    println!("  matter-cli tool-pipeline-validate-contract-json <file.json> Validate pipeline output contract");
    println!("  matter-cli tool-pipeline-normalize-contract-json <in.json> [out.json] Normalize legacy pipeline JSON to contract");
    println!("  matter-cli tool-pipeline-contract-example-json Print canonical pipeline contract examples");
    println!("  matter-cli tool-pipeline-contract-selftest-json Run pipeline contract self-tests");
    println!("  matter-cli tool-pipeline-contract-ci-gate-json <file.json> [--warn-as-fail] Evaluate CI gate from contract output");
    println!("  matter-cli tool-pipeline-contract-diff-json <baseline.json> <candidate.json> Compare two contract payloads for compatibility");
    println!("  matter-cli tool-pipeline-contract-upgrade-advice-json <baseline.json> <candidate.json> Generate migration advice from contract diff");
    println!("  matter-cli tool-pipeline-contract-bundle-json <baseline.json> <candidate.json> Emit combined diff+advice contract bundle");
    println!("  matter-cli tool-pipeline-apply-next-cycle-json <next_cycle_config.json> Apply next-cycle config");
    println!(
        "  matter-cli package-json [matter.toml]       Inspect Matter package manifest as JSON"
    );
    println!("  matter-cli project-deps-json [matter.toml]  Inspect resolved package dependencies as JSON");
    println!("  matter-cli project-check-json [matter.toml] Validate package entrypoint as JSON");
    println!("  matter-cli project-verify-json [matter.toml] Verify dependencies, imports, and compile checks as JSON");
    println!(
        "  matter-cli project-run-json [matter.toml] [--with-energy]  Run package entrypoint as JSON"
    );
    println!(
        "  matter-cli project-imports-json [matter.toml] Inspect package import graph as JSON"
    );
    println!("  matter-cli project-lock-json [matter.toml]  Print reproducible package lock JSON");
    println!(
        "  matter-cli project-fingerprint-json [matter.toml] Print project cache fingerprint JSON"
    );
    println!(
        "  matter-cli project-source-json [matter.toml] Print resolved package source as JSON"
    );
    println!("  matter-cli project-compile-json [matter.toml] [-o out] Compile package entrypoint as JSON");
    println!("  matter-cli project-build-json [matter.toml] [-o out] Verify and build cacheable bytecode JSON");
    println!(
        "  matter-cli project-run-build-json [matter.toml] [-o out] [--with-energy] Build bytecode and run it as JSON"
    );
    println!(
        "  matter-cli project-emit-build-json [matter.toml] <event> [-o out] [--with-energy] Build bytecode and emit event as JSON"
    );
    println!("  matter-cli project-visual-step-build-json [matter.toml] <events.json> <delta_ms> [-o out] [--with-energy] Run one visual frame with VM event bridge");
    println!("  matter-cli project-visual-run-build-json [matter.toml] <events.json> <frames> <delta_ms> [-o out] [--with-energy] Run visual loop with VM event bridge");
    println!("  matter-cli project-web-build-json [matter.toml] <output_dir> <app_name> Build/export web runtime and verify package lock");
    println!(
        "  matter-cli web-serve-json <dir> [port] [--once] Serve exported web runtime over HTTP"
    );
    println!("  matter-cli project-web-serve-json [matter.toml] <output_dir> <app_name> [port] [--once] Build/export and serve web runtime");
    println!("  matter-cli project-web-smoke-json [matter.toml] <output_dir> <app_name> [port] Build/export + health check smoke test");
    println!("  matter-cli project-web-ci-json [matter.toml] <output_dir> <app_name> [port] Build/export + health/meta/index CI checks");
    println!("  matter-cli web-events-save-json <port> <output_events.json> [--clear] Save live web event queue as PXL_EVENT_QUEUE");
    println!("  matter-cli web-state-json <port>            Read live web server state snapshot");
    println!("  matter-cli web-events-tail-json <port> [cursor] [limit] [--ack] Read incremental web events with cursor");
    println!("  matter-cli web-action-json <port> <action>  Trigger a live declarative VM action");
    println!("  matter-cli web-actions-json <port>          List live declarative VM actions");
    println!("  matter-cli web-live-demo-check-json <port> [timeout_ms] Push click and verify Web->VM->State");
    println!("  matter-cli project-web-step-live-json [matter.toml] <port> <delta_ms> [--clear] Pull live web events and run VM visual step");
    println!("  matter-cli project-web-loop-live-json [matter.toml] <port> <delta_ms> <ticks|forever> [--interval-ms N] [--no-clear] [--telemetry-ms N] Run continuous live web->VM visual loop");
    println!("  matter-cli start-live-demo-json [matter.toml] <output_dir> <app_name> <port> [--delta-ms N] [--interval-ms N] [--telemetry-ms N] Start serve+loop demo and print PIDs");
    println!("  matter-cli run <file.matter|->              Run Matter source file or stdin");
    println!("  matter-cli eval <source>                    Run Matter source passed as text");
    println!("  matter-cli eval-json <source>               Run source text and print JSON result");
    println!(
        "  matter-cli run-json <file.matter|-> [--with-energy]  Run source and print JSON result"
    );
    println!("  matter-cli emnr-run-json <file.matter|-> Run source with EMNR supervision");
    println!(
        "  matter-cli emnr-run-stream-json <files...> Run multiple sources with one EMNR session"
    );
    println!(
        "  matter-cli emnr-gate-json <files...> [--policy file.json] [--out report.json] [--state-in state.json] [--state-out state.json] [--warn-as-fail] [--risk-threshold N] [--confidence-threshold N] [--energy-threshold N] [--critical-energy N] Gate files with EMNR decisions"
    );
    println!(
        "  matter-cli emnr-train-json <files...> [--epochs N] [--state-in state.json] [--state-out state.json] [--out report.json] Calibrate EMNR on Matter runs"
    );
    println!(
        "  matter-cli emnr-promote-json <files...> --baseline-state state.json --candidate-state state.json [--out report.json] Compare EMNR states before promotion"
    );
    println!(
        "  matter-cli emnr-audit-json <reports...> [--out audit.json] [--fail-on-warn] Summarize EMNR reports"
    );
    println!(
        "  matter-cli emnr-pipeline-json <files...> [--epochs N] [--out-dir dir] [--policy file.json] [--fail-on-warn] Run gate/train/promote/audit"
    );
    println!(
        "  matter-cli emnr-ci-json <files...> [--epochs N] [--out-dir dir] [--policy file.json] [--baseline-manifest manifest.json|--baseline-index index.json] [--require-non-regression] [--require-pass] [--fail-on-warn] [--out report.json] Run EMNR CI gate"
    );
    println!(
        "  matter-cli emnr-ci-scorecard-json <ci-report.json> [--out scorecard.json] [--require-pass] Summarize EMNR CI decision"
    );
    println!(
        "  matter-cli emnr-ci-history-json <scorecards...> [--out history.json] [--require-stable] Summarize EMNR CI scorecard history"
    );
    println!(
        "  matter-cli emnr-ci-bundle-json <ci-report.json> --out-dir dir [--require-pass] [--require-stable] Package EMNR CI report, scorecard, and history"
    );
    println!(
        "  matter-cli emnr-verify-ci-bundle-json <bundle.json> [--require-pass] Verify packaged EMNR CI bundle"
    );
    println!(
        "  matter-cli emnr-ci-bundle-summary-md <bundle.json> --out summary.md Emit human-readable EMNR CI bundle summary"
    );
    println!(
        "  matter-cli emnr-promote-ci-bundle-json <bundle.json> --index-out index.json [--require-pass] [--out report.json] Promote verified EMNR CI bundle into release index"
    );
    println!(
        "  matter-cli emnr-ci-bundle-index-json <index.json> [--latest N] [--require-current-approved] Inspect EMNR CI bundle release index"
    );
    println!(
        "  matter-cli emnr-export-ci-bundle-index-json <index.json> --out-dir dir Export EMNR CI bundle release package"
    );
    println!(
        "  matter-cli emnr-verify-ci-bundle-package-json <package.json> [--require-pass] Verify exported EMNR CI bundle release package"
    );
    println!(
        "  matter-cli emnr-import-ci-bundle-package-json <package.json> --out-dir dir [--require-pass] Import EMNR CI bundle release package"
    );
    println!("  matter-cli emnr-init-json [--force] Initialize .emnr project memory");
    println!(
        "  matter-cli emnr-sync-json [--force] [--require-pass] Sync .emnr memory from imported artifacts"
    );
    println!(
        "  matter-cli emnr-doctor-json [--require-pass] Diagnose EMNR/Matter operational health"
    );
    println!(
        "  matter-cli emnr-release-gate-json [--policy file] [--bundle-index file] [--require-pass] Decide if EMNR state allows release"
    );
    println!(
        "  matter-cli emnr-agent-advice-json [--policy file] [--bundle-index file] [--require-pass] Explain EMNR release decision for agents"
    );
    println!(
        "  matter-cli emnr-status-json [--policy file] [--bundle-index file] [--require-pass] Print compact EMNR operational status"
    );
    println!(
        "  matter-cli emnr-status-md [--policy file] [--bundle-index file] [--out status.md] [--require-pass] Emit human-readable EMNR status"
    );
    println!(
        "  matter-cli emnr-verify-manifest-json <manifest.json> [--require-pass] Verify EMNR pipeline manifest"
    );
    println!(
        "  matter-cli emnr-compare-manifests-json <baseline.json> <candidate.json> [--require-non-regression] Compare EMNR pipeline manifests"
    );
    println!(
        "  matter-cli emnr-promote-baseline-json <manifest.json> --baseline-out baseline.json [--index-out index.json] [--allow-warn] [--out report.json] Promote verified EMNR manifest as baseline"
    );
    println!(
        "  matter-cli emnr-baseline-index-json <index.json> [--latest N] Inspect EMNR baseline promotion index"
    );
    println!(
        "  matter-cli emnr-export-baseline-json <index.json> --out-dir dir Export EMNR baseline package"
    );
    println!(
        "  matter-cli emnr-verify-baseline-package-json <package.json> [--require-pass] Verify exported EMNR baseline package"
    );
    println!(
        "  matter-cli emnr-import-baseline-package-json <package.json> --out-dir dir [--require-pass] Import EMNR baseline package"
    );
    println!(
        "  matter-cli emnr-baseline-doctor-json <baseline-dir> [--require-pass] Diagnose imported EMNR baseline directory"
    );
    println!("  matter-cli emnr-state-json <state.json> [--full] Inspect persisted EMNR state");
    println!("  matter-cli run-energy <file.matter|->       Run source and print energy report");
    println!(
        "  matter-cli run-energy-json <file.matter|->  Run source and print JSON energy report"
    );
    println!("  matter-cli benchmark <file.matter|-> [--iterations N] [--json] Benchmark source");
    println!(
        "  matter-cli benchmark-json <file.matter|-> [--iterations N] Benchmark source as JSON"
    );
    println!(
        "  matter-cli perf-diagnose-json <file.matter|-> Diagnose hot-path/performance pressure as JSON"
    );
    println!("  matter-cli benchmark-gate-json <benchmark.json> [--max-median-ns N] [--max-p95-ns N] [--ci-exit-codes] Check benchmark budgets");
    println!("  matter-cli doctor                           Check local Matter workspace health");
    println!(
        "  matter-cli doctor-json                      Check local Matter workspace health as JSON"
    );
    println!("  matter-cli emit <file.matter|-> <event>     Emit event in Matter program");
    println!(
        "  matter-cli emit-json <file.matter|-> <event> [--with-energy] Emit event and print JSON result"
    );
    println!("  matter-cli tool-wire-validate-json <wire_file> Validate Matter tool handoff wire");
    println!("  matter-cli tool-wire-merge-json <left_wire_file> <right_wire_file> [strategy] Merge two handoff wires");
    println!("  matter-cli tool-frame-invoke-json <frame_payload.json> [-o wire.out] Invoke tool frame payload and produce wire");
    println!("  matter-cli tool-frame-template-json [-o frame_payload.json] Generate a starter tool frame payload");
    println!("  matter-cli tool-wire-extract-json <invoke_result.json> [-o wire.out] Extract wire from invoke result JSON");
    println!("  matter-cli tool-pipeline-contract-bundle-example-json Print baseline/candidate/bundle example for CI orchestration");
    println!("  matter-cli tool-pipeline-demo-json [out_dir] [--strict] [--dry-run] [--ci-exit-codes] [--compare-strategies] [--artifact-manifest-json] [--emit-contract-bundle] [--contract-bundle-baseline baseline.json] [--next-cycle-config-out file.json] [--next-cycle-max-hops N] [--apply-recommended-energy-mode] [--next-cycle-apply-now] [--emit-summary-md] [--emit-github-step-summary] [--summary-format json|md|both] [--artifact-prefix name] [--fail-on-status status] [--strategy auto|prefer_latest|prefer_blocked|prefer_terminal] [--energy-mode eco|balanced|performance|adaptive|critical] [--confidence-profile strict|balanced|relaxed] [--confidence-threshold-low N] [--confidence-threshold-high N] [--require-catalog-hash hash] [--score-preset conservative|balanced|aggressive] [--score-status-ok N] [--score-status-degraded N] [--score-status-unknown N] [--score-action-execute N] [--score-action-resolve-blockers N] [--score-action-none N] [--score-action-other N] [--score-penalty-latest N] [--score-penalty-blocked N] [--score-penalty-terminal N] Run template->invoke->validate->merge demo and emit artifacts");
    println!("  matter-cli init [dir] [--name name] [--template basic|event] Create a new Matter project");
    println!("  matter-cli init-json [dir] [--name name] [--template basic|event] Create a new Matter project as JSON");
    println!("  matter-cli visual-step-json <file.matter|-> <events.json> <delta_ms> [--with-energy] Run one visual frame + bridge events to VM");
    println!("  matter-cli visual-run-json <file.matter|-> <events.json> <frames> <delta_ms> [--with-energy] Run multi-frame visual loop + bridge events");
    println!(
        "  matter-cli studio-native [file.matter] [--interactive] [--no-clear] Render native Rust terminal studio"
    );
    println!("  matter-cli studio-native-json [file.matter] Render native studio model as JSON");
    println!("  matter-cli sentinel-pvmbc [file.matter] [-o out.pvmbc] [--name app] Export visual model to Sentinel PVM2 bytecode");
    println!("  matter-cli sentinel-pvmbc-rust-array [file.matter] [--const NAME] [--name app] Print PVM2 bytecode as a Rust &[u8] constant");
    println!("  matter-cli sentinel-mbc1-rust-array [file.matter] [--const NAME] Print MBC1 bytecode as a Rust &[u8] constant");
    println!("  matter-cli sentinel-mbc1-kernel-check-json [file.matter] [--budget N] Compile and run MBC1 in the no_std kernel VM subset");
    println!("  matter-cli rust-ffi-call-json <library_path> <symbol> [args_json|@args.json] Call Rust dynamic library JSON ABI");
    println!("  matter-cli rust-ffi-validate-args-json <args_json|@args.json> Validate Rust FFI ABI args");
    println!("  matter-cli sentinel-pvmbc-inspect-json <file.pvmbc> Inspect Sentinel PVM bytecode as JSON");
    println!("  matter-cli check <file.matter|->            Parse and compile without running");
    println!("  matter-cli tokens-json <file.matter|->      Tokenize source and print JSON");
    println!("  matter-cli imports-json <file.matter|->     Inspect local imports as JSON");
    println!("  matter-cli check-json <file.matter|->       Validate source and print JSON");
    println!(
        "  matter-cli reflect-json <file.matter|->     Inspect source as AST and bytecode JSON"
    );
    println!("  matter-cli reflexive-guard-json <file.matter|-> [--max-statements N] [--max-functions N] [--allow-backends] Evaluate reflexive safety gates");
    println!("  matter-cli compile <file.matter|-> [-o out] Compile to bytecode (.mbc)");
    println!("  matter-cli compile-json <file.matter|-> [-o out] Compile and print JSON");
    println!("  matter-cli run-bytecode <file.mbc>          Run bytecode file");
    println!(
        "  matter-cli run-bytecode-json <file.mbc> [--with-energy] Run bytecode and print JSON result"
    );
    println!("  matter-cli emit-bytecode <file.mbc> <event> Emit event from bytecode");
    println!("  matter-cli emit-bytecode-json <file.mbc> <event> Emit bytecode event as JSON");
    println!("  matter-cli inspect <file.mbc>               Inspect bytecode file");
    println!("  matter-cli inspect-json <file.mbc>          Inspect bytecode and print JSON");
    println!();
    println!("Memory Management:");
    println!("  matter-cli gc-stats <file.matter>           Show GC and memory statistics");
    println!("  matter-cli gc-collect <file.matter>         Force garbage collection");
    println!("  matter-cli gc-profile <file.matter>         Profile memory usage");
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
            "\"tool-ci-catalog-json\",",
            "\"tool-ci-verify-json\",",
            "\"tool-ci-contract-json\",",
            "\"tool-pipeline-validate-contract-json\",",
            "\"tool-pipeline-normalize-contract-json\",",
            "\"tool-pipeline-contract-example-json\",",
            "\"tool-pipeline-contract-selftest-json\",",
            "\"tool-pipeline-contract-ci-gate-json\",",
            "\"tool-pipeline-contract-diff-json\",",
            "\"tool-pipeline-contract-upgrade-advice-json\",",
            "\"tool-pipeline-contract-bundle-json\",",
            "\"tool-pipeline-contract-bundle-example-json\",",
            "\"tool-pipeline-apply-next-cycle-json\",",
            "\"init-json\",",
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
            "\"project-run-build-json\",",
            "\"project-emit-build-json\",",
            "\"project-visual-step-build-json\",",
            "\"project-visual-run-build-json\",",
            "\"project-web-build-json\",",
            "\"web-serve-json\",",
            "\"project-web-serve-json\",",
            "\"project-web-smoke-json\",",
            "\"project-web-ci-json\",",
            "\"web-events-save-json\",",
            "\"web-state-json\",",
            "\"web-events-tail-json\",",
            "\"web-action-json\",",
            "\"web-actions-json\",",
            "\"web-live-demo-check-json\",",
            "\"project-web-step-live-json\",",
            "\"project-web-loop-live-json\",",
            "\"start-live-demo-json\",",
            "\"eval-json\",",
            "\"tokens-json\",",
            "\"imports-json\",",
            "\"check-json\",",
            "\"reflect-json\",",
            "\"reflexive-guard-json\",",
            "\"run-json\",",
            "\"run-energy-json\",",
            "\"benchmark-json\",",
            "\"perf-diagnose-json\",",
            "\"benchmark-gate-json\",",
            "\"doctor-json\",",
            "\"emit-json\",",
            "\"tool-wire-validate-json\",",
            "\"tool-wire-merge-json\",",
            "\"tool-frame-invoke-json\",",
            "\"tool-frame-template-json\",",
            "\"tool-wire-extract-json\",",
            "\"tool-pipeline-demo-json\",",
            "\"visual-step-json\",",
            "\"visual-run-json\",",
            "\"studio-native-json\",",
            "\"sentinel-pvmbc\",",
            "\"sentinel-pvmbc-rust-array\",",
            "\"sentinel-mbc1-rust-array\",",
            "\"sentinel-mbc1-kernel-check-json\",",
            "\"rust-ffi-call-json\",",
            "\"rust-ffi-validate-args-json\",",
            "\"sentinel-pvmbc-inspect-json\",",
            "\"compile-json\",",
            "\"inspect-json\",",
            "\"run-bytecode-json\",",
            "\"emit-bytecode-json\"",
            "],",
            "\"source_commands\":[",
            "\"run\",",
            "\"run-energy\",",
            "\"eval\",",
            "\"emit\",",
            "\"check\",",
            "\"studio-native\",",
            "\"sentinel-pvmbc\",",
            "\"sentinel-pvmbc-rust-array\",",
            "\"sentinel-mbc1-rust-array\",",
            "\"sentinel-mbc1-kernel-check-json\",",
            "\"rust-ffi-call-json\",",
            "\"rust-ffi-validate-args-json\",",
            "\"init\",",
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
            "],",
            "\"ci_reason_catalog_version\":\"1\",",
            "\"ci_reason_catalog\":{{",
            "\"healthy\":0,",
            "\"low_confidence\":10,",
            "\"catalog_hash_mismatch\":20,",
            "\"mkdir_failed\":100,",
            "\"write_frames_failed\":110,",
            "\"invoke_a_failed\":120,",
            "\"invoke_b_failed\":121,",
            "\"extract_wire_failed\":130,",
            "\"matched_fail_status\":140,",
            "\"strict_degraded\":150,",
            "\"unknown\":999",
            "}},",
            "\"ci_reason_metadata\":{{",
            "\"healthy\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}},",
            "\"low_confidence\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}},",
            "\"catalog_hash_mismatch\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}},",
            "\"mkdir_failed\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}},",
            "\"write_frames_failed\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}},",
            "\"invoke_a_failed\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}},",
            "\"invoke_b_failed\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}},",
            "\"extract_wire_failed\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}},",
            "\"matched_fail_status\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}},",
            "\"strict_degraded\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}},",
            "\"unknown\":{{\"deprecation\":\"\",\"replacement_reason\":\"\",\"since_version\":\"1\",\"last_updated\":\"2026-05-11\"}}",
            "}}",
            "}}"
        ),
        env!("CARGO_PKG_VERSION")
    );
}

fn print_tool_ci_catalog_json() {
    println!("{}", tool_ci_catalog_json_string());
}

fn fnv1a64_text_hex(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn tool_ci_catalog_json_string() -> String {
    let items = json!({
        "healthy": 0,
        "low_confidence": 10,
        "catalog_hash_mismatch": 20,
        "mkdir_failed": 100,
        "write_frames_failed": 110,
        "invoke_a_failed": 120,
        "invoke_b_failed": 121,
        "extract_wire_failed": 130,
        "matched_fail_status": 140,
        "strict_degraded": 150,
        "unknown": 999
    });
    let reason_metadata = json!({
        "healthy": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"},
        "low_confidence": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"},
        "catalog_hash_mismatch": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"},
        "mkdir_failed": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"},
        "write_frames_failed": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"},
        "invoke_a_failed": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"},
        "invoke_b_failed": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"},
        "extract_wire_failed": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"},
        "matched_fail_status": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"},
        "strict_degraded": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"},
        "unknown": {"deprecation":"", "replacement_reason":"", "since_version":"1", "last_updated":"2026-05-11"}
    });
    let hash_input = format!("{}|{}", items, reason_metadata);
    let catalog_hash = format!("fnv1a64:{}", fnv1a64_text_hex(&hash_input));
    json!({
        "ok": true,
        "name": "matter-cli",
        "catalog": "ci_reason",
        "version": "1",
        "catalog_hash": catalog_hash,
        "items": items,
        "reason_metadata": reason_metadata,
        "examples": [
            "ciDecision=pass ciDecisionReason=healthy ciDecisionCode=0",
            "ciDecision=warn ciDecisionReason=low_confidence ciDecisionCode=10",
            "ciDecision=fail ciDecisionReason=strict_degraded ciDecisionCode=150"
        ]
    })
    .to_string()
}

fn current_ci_catalog_hash() -> String {
    let payload = tool_ci_catalog_json_string();
    serde_json::from_str::<JsonValue>(&payload)
        .ok()
        .and_then(|doc| {
            doc.get("catalog_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
}

fn print_tool_ci_contract_json() {
    println!("{}", tool_ci_contract_json_string());
}

fn tool_ci_contract_json_string() -> String {
    let catalog = serde_json::from_str::<JsonValue>(&tool_ci_catalog_json_string())
        .unwrap_or(JsonValue::Null);
    let catalog_hash = catalog
        .get("catalog_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    json!({
        "ok": true,
        "name": "matter-cli",
        "contract": "ci_reason_contract",
        "version": "1",
        "catalog_hash": catalog_hash,
        "catalog": catalog,
        "compatibility": {
            "same_major": "compatible",
            "major_upgrade": "breaking_review_required",
            "new_reason_code": "non_breaking_if_existing_codes_stable",
            "changed_reason_code": "breaking"
        },
        "guidance": {
            "pin_fields": [
                "ciDecision",
                "ciDecisionReason",
                "ciDecisionCode"
            ],
            "recommendation": "Gate production on ciDecision and alert on ciDecisionReason changes."
        }
    })
    .to_string()
}

fn tool_ci_verify_json(reason: &str, code_raw: &str) {
    let parsed = code_raw.parse::<i32>();
    let Ok(code) = parsed else {
        println!(
            "{{\"ok\":false,\"reason\":\"{}\",\"inputCode\":\"{}\",\"error\":{{\"message\":\"code must be an integer\"}}}}",
            json_escape(reason),
            json_escape(code_raw)
        );
        return;
    };
    let (expected, matches) = tool_ci_verify(reason, code);
    println!(
        "{{\"ok\":{},\"reason\":\"{}\",\"inputCode\":{},\"expectedCode\":{},\"match\":{}}}",
        if matches { "true" } else { "false" },
        json_escape(reason),
        code,
        expected,
        if matches { "true" } else { "false" }
    );
}

fn tool_ci_verify(reason: &str, code: i32) -> (i32, bool) {
    let expected = ci_reason_code(reason);
    (expected, expected == code)
}

fn validate_pipeline_contract_doc(doc: &JsonValue) -> Result<(), String> {
    let obj = doc
        .as_object()
        .ok_or_else(|| "payload must be a JSON object".to_string())?;
    let required = [
        "ok",
        "contractVersion",
        "catalogHash",
        "ciDecision",
        "ciDecisionReason",
        "ciDecisionCode",
    ];
    for key in required {
        if !obj.contains_key(key) {
            return Err(format!("missing required field '{}'", key));
        }
    }
    if !obj.get("ok").map(|v| v.is_boolean()).unwrap_or(false) {
        return Err("field 'ok' must be boolean".to_string());
    }
    if !obj
        .get("ciDecisionCode")
        .map(|v| v.is_i64() || v.is_u64())
        .unwrap_or(false)
    {
        return Err("field 'ciDecisionCode' must be integer".to_string());
    }
    Ok(())
}

fn normalize_pipeline_contract_doc(doc: &JsonValue) -> JsonValue {
    let mut out = match doc {
        JsonValue::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };

    if !out.contains_key("ok") {
        out.insert("ok".to_string(), JsonValue::Bool(false));
    }

    if !out.contains_key("contractVersion") {
        let value = out
            .get("contract_version")
            .and_then(|v| v.as_str())
            .map(|s| JsonValue::String(s.to_string()))
            .unwrap_or_else(|| JsonValue::String("1".to_string()));
        out.insert("contractVersion".to_string(), value);
    }

    if !out.contains_key("catalogHash") {
        let value = out
            .get("catalog_hash")
            .and_then(|v| v.as_str())
            .map(|s| JsonValue::String(s.to_string()))
            .unwrap_or_else(|| JsonValue::String(current_ci_catalog_hash()));
        out.insert("catalogHash".to_string(), value);
    }

    if !out.contains_key("ciDecision") {
        let value = out
            .get("ci_decision")
            .and_then(|v| v.as_str())
            .map(|s| JsonValue::String(s.to_string()))
            .unwrap_or_else(|| JsonValue::String("pass".to_string()));
        out.insert("ciDecision".to_string(), value);
    }

    if !out.contains_key("ciDecisionReason") {
        let value = out
            .get("ci_decision_reason")
            .and_then(|v| v.as_str())
            .map(|s| JsonValue::String(s.to_string()))
            .unwrap_or_else(|| JsonValue::String("healthy".to_string()));
        out.insert("ciDecisionReason".to_string(), value);
    }

    if !out.contains_key("ciDecisionCode") {
        let value = if let Some(code) = out
            .get("ci_decision_code")
            .and_then(|v| v.as_i64())
            .map(JsonValue::from)
        {
            code
        } else {
            let reason = out
                .get("ciDecisionReason")
                .and_then(|v| v.as_str())
                .unwrap_or("healthy");
            JsonValue::from(ci_reason_code(reason))
        };
        out.insert("ciDecisionCode".to_string(), value);
    }

    JsonValue::Object(out)
}

fn tool_pipeline_validate_contract_json(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"file\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let parsed = match serde_json::from_str::<JsonValue>(&source) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"file\":\"{}\",\"error\":{{\"message\":\"invalid JSON: {}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    match validate_pipeline_contract_doc(&parsed) {
        Ok(()) => println!(
            "{{\"ok\":true,\"file\":\"{}\",\"valid\":true}}",
            json_escape(path)
        ),
        Err(message) => println!(
            "{{\"ok\":false,\"file\":\"{}\",\"valid\":false,\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(path),
            json_escape(&message)
        ),
    }
}

fn tool_pipeline_normalize_contract_json(input_path: &str, output_path: Option<&str>) {
    let source = match fs::read_to_string(input_path) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(input_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let parsed = match serde_json::from_str::<JsonValue>(&source) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"input\":\"{}\",\"error\":{{\"message\":\"invalid JSON: {}\"}}}}",
                json_escape(input_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    let normalized = normalize_pipeline_contract_doc(&parsed);
    let normalized_text = normalized.to_string();

    let mut wrote = false;
    if let Some(path) = output_path {
        if fs::write(path, &normalized_text).is_ok() {
            wrote = true;
        }
    }

    let valid = validate_pipeline_contract_doc(&normalized).is_ok();
    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"outputWritten\":{},\"outputPath\":{},\"valid\":{},\"normalized\":{}}}",
        json_escape(input_path),
        if wrote { "true" } else { "false" },
        output_path
            .map(|p| format!("\"{}\"", json_escape(p)))
            .unwrap_or_else(|| "null".to_string()),
        if valid { "true" } else { "false" },
        normalized_text
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

struct InitOptions {
    dir: PathBuf,
    name: Option<String>,
    template: InitTemplate,
}

#[derive(Debug)]
struct InitResult {
    name: String,
    template: InitTemplate,
    dir: PathBuf,
    manifest_path: PathBuf,
    entry_path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InitTemplate {
    Basic,
    Event,
}

impl InitTemplate {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "basic" => Some(Self::Basic),
            "event" | "events" => Some(Self::Event),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Event => "event",
        }
    }
}

struct EnvSnapshot {
    key: &'static str,
    previous: Option<String>,
}

fn parse_init_options(args: &[String], json_output: bool) -> InitOptions {
    let mut dir: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut template = InitTemplate::Basic;
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "--name" => {
                if i + 1 >= args.len() {
                    init_usage_error("--name requires a value", json_output);
                }
                name = Some(args[i + 1].clone());
                i += 2;
            }
            "--template" => {
                if i + 1 >= args.len() {
                    init_usage_error("--template requires a value", json_output);
                }
                template = InitTemplate::parse(&args[i + 1]).unwrap_or_else(|| {
                    init_usage_error("unknown template; use basic or event", json_output)
                });
                i += 2;
            }
            "--help" | "-h" => {
                print_init_help();
                process::exit(0);
            }
            flag if flag.starts_with("--") => {
                init_usage_error(&format!("unknown option '{}'", flag), json_output);
            }
            value => {
                if dir.is_some() {
                    init_usage_error("init accepts at most one directory", json_output);
                }
                dir = Some(PathBuf::from(value));
                i += 1;
            }
        }
    }

    InitOptions {
        dir: dir.unwrap_or_else(|| PathBuf::from(".")),
        name,
        template,
    }
}

fn init_usage_error(message: &str, json_output: bool) -> ! {
    if json_output {
        println!(
            "{{\"ok\":false,\"stage\":\"usage\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(message)
        );
    } else {
        eprintln!("Usage error: {}", message);
        eprintln!("Usage: matter-cli init [dir] [--name name] [--template basic|event]");
    }
    process::exit(1);
}

fn print_init_help() {
    println!("matter-cli init - Create a new Matter project");
    println!();
    println!("USAGE:");
    println!("    matter-cli init [dir] [--name name] [--template basic|event]");
    println!("    matter-cli init-json [dir] [--name name] [--template basic|event]");
    println!();
    println!("DESCRIPTION:");
    println!("    Creates matter.toml and src/main.matter without overwriting existing files.");
    println!("    Templates: basic, event");
}

#[cfg(test)]
fn init_template_run_output(name: &str, template: InitTemplate) -> Vec<String> {
    match template {
        InitTemplate::Basic => vec!["Matter project ready".to_string(), name.to_string()],
        InitTemplate::Event => vec!["Matter event project ready".to_string(), name.to_string()],
    }
}

fn init_project(options: &InitOptions, json_output: bool) {
    match scaffold_project(options) {
        Ok(result) => {
            if json_output {
                println!(
                    "{{\"ok\":true,\"name\":\"{}\",\"template\":\"{}\",\"dir\":\"{}\",\"manifest\":\"{}\",\"entry\":\"{}\"}}",
                    json_escape(&result.name),
                    json_escape(result.template.name()),
                    json_escape(&result.dir.display().to_string()),
                    json_escape(&result.manifest_path.display().to_string()),
                    json_escape(&result.entry_path.display().to_string())
                );
            } else {
                println!(
                    "Created Matter project '{}' using '{}' template",
                    result.name,
                    result.template.name()
                );
                println!("  Manifest: {}", result.manifest_path.display());
                println!("  Entry:    {}", result.entry_path.display());
                println!();
                println!("Next:");
                println!(
                    "  matter-cli project-check-json {}",
                    result.manifest_path.display()
                );
                println!(
                    "  matter-cli project-run-json {}",
                    result.manifest_path.display()
                );
            }
        }
        Err(error) => {
            if json_output {
                println!(
                    "{{\"ok\":false,\"stage\":\"init\",\"dir\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                    json_escape(&options.dir.display().to_string()),
                    json_escape(&error)
                );
            } else {
                eprintln!("Init failed: {}", error);
            }
            process::exit(1);
        }
    }
}

fn scaffold_project(options: &InitOptions) -> Result<InitResult, String> {
    let dir = options.dir.clone();
    let project_name = options
        .name
        .clone()
        .unwrap_or_else(|| infer_project_name(&dir));
    let project_name = sanitize_project_name(&project_name)?;
    let src_dir = dir.join("src");
    let manifest_path = dir.join("matter.toml");
    let entry_path = src_dir.join("main.matter");

    if manifest_path.exists() {
        return Err(format!("{} already exists", manifest_path.display()));
    }
    if entry_path.exists() {
        return Err(format!("{} already exists", entry_path.display()));
    }

    fs::create_dir_all(&src_dir)
        .map_err(|error| format!("could not create {}: {}", src_dir.display(), error))?;

    fs::write(&manifest_path, render_init_manifest(&project_name))
        .map_err(|error| format!("could not write {}: {}", manifest_path.display(), error))?;
    fs::write(
        &entry_path,
        render_init_entry(&project_name, options.template),
    )
    .map_err(|error| format!("could not write {}: {}", entry_path.display(), error))?;

    Ok(InitResult {
        name: project_name,
        template: options.template,
        dir,
        manifest_path,
        entry_path,
    })
}

fn infer_project_name(dir: &Path) -> String {
    dir.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty() && *name != ".")
        .unwrap_or("matter-app")
        .to_string()
}

fn sanitize_project_name(raw: &str) -> Result<String, String> {
    let mut name = String::new();
    let mut previous_dash = false;

    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if matches!(ch, '-' | '_' | ' ' | '.') && !previous_dash && !name.is_empty() {
            name.push('-');
            previous_dash = true;
        }
    }

    while name.ends_with('-') {
        name.pop();
    }

    if name.is_empty() {
        Err("project name must contain at least one ASCII letter or digit".to_string())
    } else {
        Ok(name)
    }
}

fn render_init_manifest(name: &str) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nentry = \"src/main.matter\"\n\n[paths]\nstdlib = \"stdlib\"\nstore = \".matter_store.json\"\n\n[dependencies]\n",
        name
    )
}

fn render_init_entry(name: &str, template: InitTemplate) -> String {
    match template {
        InitTemplate::Basic => format!(
            "let app = \"{}\"\nprint \"Matter project ready\"\nprint app\n",
            name
        ),
        InitTemplate::Event => format!(
            "let app = \"{}\"\nprint \"Matter event project ready\"\nprint app\n\non boot {{\n    print \"boot event received\"\n}}\n",
            name
        ),
    }
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
    let bytecode = build_json_or_exit(
        &source,
        &entry_label,
        &[("package", &project.manifest.name)],
    );

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

    let base_dir = entry_path.parent().unwrap_or(Path::new(".")).to_path_buf();
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
    push_lock_file(
        &mut files,
        &mut seen_files,
        "manifest",
        Path::new(&project.manifest_path),
    );
    push_lock_file(&mut files, &mut seen_files, "entry", &entry_path);
    for import in &imports {
        push_lock_file(
            &mut files,
            &mut seen_files,
            &import.source,
            Path::new(&import.resolved),
        );
    }
    let lock_fingerprint =
        project_lock_fingerprint(&files, &imports, &project.manifest.dependencies);

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"lock_fingerprint\":\"{}\",\"dependencies_count\":{},\"imports_count\":{},\"files_count\":{},\"source_bytes\":{},\"summary\":{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(&lock_fingerprint),
        project.manifest.dependencies.len(),
        imports.len(),
        files.len(),
        resolved_source.len(),
        bytecode_summary_json(&bytecode)
    );
}

fn project_run_json(manifest_path: &str, with_energy: bool) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(
        &source,
        &entry_label,
        &[("package", &project.manifest.name)],
    );

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.run() {
        if with_energy {
            println!(
                "{{\"ok\":false,\"stage\":\"runtime\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{},\"energy\":{{\"instruction_cost\":{:.2},\"backend_cost\":{:.2}}}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(&error),
                json_string_array(&runtime.take_output()),
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            );
        } else {
            println!(
                "{{\"ok\":false,\"stage\":\"runtime\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(&error),
                json_string_array(&runtime.take_output())
            );
        }
        process::exit(1);
    }

    if with_energy {
        println!(
            "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output\":{},\"energy\":{{\"instruction_cost\":{:.2},\"backend_cost\":{:.2}}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_string_array(&runtime.take_output()),
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        );
    } else {
        println!(
            "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output\":{}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_string_array(&runtime.take_output())
        );
    }
}

fn tool_pipeline_contract_example_json_string() -> String {
    let contract_version = "1";
    let catalog_hash = current_ci_catalog_hash();
    json!({
        "ok": true,
        "contractVersion": contract_version,
        "catalogHash": catalog_hash,
        "examples": {
            "success": {
                "ok": true,
                "outDir": "tool_pipeline_demo",
                "contractVersion": contract_version,
                "catalogHash": catalog_hash,
                "ciDecision": "pass",
                "ciDecisionReason": "healthy",
                "ciDecisionCode": ci_reason_code("healthy"),
                "decisionConfidence": "high",
                "decisionGap": 14.25,
                "caution": false
            },
            "warning": {
                "ok": true,
                "outDir": "tool_pipeline_demo",
                "contractVersion": contract_version,
                "catalogHash": catalog_hash,
                "ciDecision": "warn",
                "ciDecisionReason": "low_confidence",
                "ciDecisionCode": ci_reason_code("low_confidence"),
                "decisionConfidence": "low",
                "decisionGap": 2.50,
                "caution": true
            },
            "failure": {
                "ok": false,
                "stage": "strict-check",
                "outDir": "tool_pipeline_demo",
                "contractVersion": contract_version,
                "catalogHash": catalog_hash,
                "ciDecision": "fail",
                "ciDecisionReason": "strict_degraded",
                "ciDecisionCode": ci_reason_code("strict_degraded"),
                "error": {
                    "message": "merge returned degraded status in strict mode"
                }
            }
        },
        "diffExamples": {
            "compatible": {
                "baseline": {
                    "ok": true,
                    "contractVersion": contract_version,
                    "catalogHash": "fnv1a64:baseline",
                    "ciDecision": "pass",
                    "ciDecisionReason": "healthy",
                    "ciDecisionCode": 0
                },
                "candidate": {
                    "ok": true,
                    "contractVersion": contract_version,
                    "catalogHash": "fnv1a64:candidate",
                    "ciDecision": "pass",
                    "ciDecisionReason": "healthy",
                    "ciDecisionCode": 0
                }
            },
            "breaking": {
                "baseline": {
                    "ok": true,
                    "contractVersion": contract_version,
                    "catalogHash": catalog_hash,
                    "ciDecision": "pass",
                    "ciDecisionReason": "healthy",
                    "ciDecisionCode": 0
                },
                "candidate": {
                    "ok": true,
                    "contractVersion": contract_version,
                    "catalogHash": catalog_hash,
                    "ciDecision": "fail",
                    "ciDecisionReason": "strict_degraded",
                    "ciDecisionCode": 150
                }
            }
        }
    })
    .to_string()
}

fn print_tool_pipeline_contract_example_json() {
    println!("{}", tool_pipeline_contract_example_json_string());
}

fn tool_pipeline_contract_selftest_doc() -> JsonValue {
    let mut checks: Vec<JsonValue> = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;

    let examples = serde_json::from_str::<JsonValue>(&tool_pipeline_contract_example_json_string())
        .unwrap_or(JsonValue::Null);
    let examples_obj = examples
        .get("examples")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    for (name, sample) in examples_obj {
        let result = validate_pipeline_contract_doc(&sample);
        let ok = result.is_ok();
        if ok {
            passed += 1;
        } else {
            failed += 1;
        }
        checks.push(json!({
            "name": format!("example_validate_{}", name),
            "ok": ok,
            "error": result.err()
        }));
    }

    let legacy = json!({
        "ok": true,
        "ci_decision": "warn",
        "ci_decision_reason": "low_confidence",
        "ci_decision_code": 10
    });
    let normalized = normalize_pipeline_contract_doc(&legacy);
    let normalize_validate = validate_pipeline_contract_doc(&normalized);
    let ok = normalize_validate.is_ok();
    if ok {
        passed += 1;
    } else {
        failed += 1;
    }
    checks.push(json!({
        "name": "normalize_then_validate_legacy",
        "ok": ok,
        "error": normalize_validate.err()
    }));

    let baseline = json!({
        "ok": true,
        "contractVersion": "1",
        "catalogHash": "fnv1a64:base",
        "ciDecision": "pass",
        "ciDecisionReason": "healthy",
        "ciDecisionCode": 0
    });
    let candidate_compatible = json!({
        "ok": true,
        "contractVersion": "1",
        "catalogHash": "fnv1a64:new",
        "ciDecision": "pass",
        "ciDecisionReason": "healthy",
        "ciDecisionCode": 0
    });
    let candidate_breaking = json!({
        "ok": true,
        "contractVersion": "1",
        "catalogHash": "fnv1a64:new",
        "ciDecision": "fail",
        "ciDecisionReason": "strict_degraded",
        "ciDecisionCode": 150
    });
    let (compat_a, _, _) =
        classify_pipeline_contract_compatibility(&baseline, &candidate_compatible);
    let ok = compat_a == "compatible";
    if ok {
        passed += 1;
    } else {
        failed += 1;
    }
    checks.push(json!({
        "name": "diff_classifies_compatible",
        "ok": ok,
        "error": if ok { None } else { Some("expected compatible") }
    }));
    let (compat_b, breaking_b, non_breaking_b) =
        classify_pipeline_contract_compatibility(&baseline, &candidate_breaking);
    let ok = compat_b == "breaking";
    if ok {
        passed += 1;
    } else {
        failed += 1;
    }
    checks.push(json!({
        "name": "diff_classifies_breaking",
        "ok": ok,
        "error": if ok { None } else { Some("expected breaking") }
    }));
    let advice = pipeline_contract_upgrade_advice(&compat_b, &breaking_b, &non_breaking_b);
    let ok = !advice.is_empty();
    if ok {
        passed += 1;
    } else {
        failed += 1;
    }
    checks.push(json!({
        "name": "upgrade_advice_present_for_breaking",
        "ok": ok,
        "error": if ok { None } else { Some("expected advice entries") }
    }));

    json!({
        "ok": failed == 0,
        "contractVersion": "1",
        "catalogHash": current_ci_catalog_hash(),
        "ciDecision": if failed == 0 { "pass" } else { "fail" },
        "ciDecisionReason": if failed == 0 { "healthy" } else { "strict_degraded" },
        "ciDecisionCode": if failed == 0 {
            ci_reason_code("healthy")
        } else {
            ci_reason_code("strict_degraded")
        },
        "checks": checks,
        "summary": {
            "passed": passed,
            "failed": failed,
            "total": passed + failed
        }
    })
}

fn print_tool_pipeline_contract_selftest_json() {
    println!("{}", tool_pipeline_contract_selftest_doc());
}

fn derive_ci_gate(decision: &str, warn_as_fail: bool) -> &'static str {
    let normalized = decision.to_ascii_lowercase();
    if normalized == "pass" {
        "pass"
    } else if normalized == "warn" {
        if warn_as_fail {
            "fail"
        } else {
            "warn"
        }
    } else {
        "fail"
    }
}

fn tool_pipeline_contract_ci_gate_json(path: &str, warn_as_fail: bool) {
    let source = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"input\":\"{}\",\"gate\":\"fail\",\"reason\":\"read_error\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let parsed = match serde_json::from_str::<JsonValue>(&source) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"input\":\"{}\",\"gate\":\"fail\",\"reason\":\"invalid_json\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    if let Err(message) = validate_pipeline_contract_doc(&parsed) {
        println!(
            "{{\"ok\":false,\"input\":\"{}\",\"gate\":\"fail\",\"reason\":\"invalid_contract\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(path),
            json_escape(&message)
        );
        return;
    }

    let decision = parsed
        .get("ciDecision")
        .and_then(|v| v.as_str())
        .unwrap_or("fail")
        .to_ascii_lowercase();
    let reason = parsed
        .get("ciDecisionReason")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let code = parsed
        .get("ciDecisionCode")
        .and_then(|v| v.as_i64())
        .unwrap_or(ci_reason_code("unknown") as i64);

    let gate = derive_ci_gate(&decision, warn_as_fail);

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"gate\":\"{}\",\"warnAsFail\":{},\"ciDecision\":\"{}\",\"ciDecisionReason\":\"{}\",\"ciDecisionCode\":{}}}",
        json_escape(path),
        gate,
        if warn_as_fail { "true" } else { "false" },
        json_escape(&decision),
        json_escape(&reason),
        code
    );
}

fn classify_pipeline_contract_compatibility(
    baseline: &JsonValue,
    candidate: &JsonValue,
) -> (String, Vec<String>, Vec<String>) {
    let mut breaking: Vec<String> = Vec::new();
    let mut non_breaking: Vec<String> = Vec::new();

    let baseline_version = baseline
        .get("contractVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let candidate_version = candidate
        .get("contractVersion")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if baseline_version != candidate_version {
        breaking.push(format!(
            "contractVersion changed: '{}' -> '{}'",
            baseline_version, candidate_version
        ));
    }

    let baseline_catalog = baseline
        .get("catalogHash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let candidate_catalog = candidate
        .get("catalogHash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if baseline_catalog != candidate_catalog {
        non_breaking.push(format!(
            "catalogHash changed: '{}' -> '{}'",
            baseline_catalog, candidate_catalog
        ));
    }

    let baseline_code = baseline
        .get("ciDecisionCode")
        .and_then(|v| v.as_i64())
        .unwrap_or(ci_reason_code("unknown") as i64);
    let candidate_code = candidate
        .get("ciDecisionCode")
        .and_then(|v| v.as_i64())
        .unwrap_or(ci_reason_code("unknown") as i64);
    if baseline_code != candidate_code {
        breaking.push(format!(
            "ciDecisionCode changed: {} -> {}",
            baseline_code, candidate_code
        ));
    }

    let baseline_reason = baseline
        .get("ciDecisionReason")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let candidate_reason = candidate
        .get("ciDecisionReason")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    if baseline_reason != candidate_reason {
        non_breaking.push(format!(
            "ciDecisionReason changed: '{}' -> '{}'",
            baseline_reason, candidate_reason
        ));
    }

    let baseline_decision = baseline
        .get("ciDecision")
        .and_then(|v| v.as_str())
        .unwrap_or("fail");
    let candidate_decision = candidate
        .get("ciDecision")
        .and_then(|v| v.as_str())
        .unwrap_or("fail");
    if baseline_decision != candidate_decision {
        non_breaking.push(format!(
            "ciDecision changed: '{}' -> '{}'",
            baseline_decision, candidate_decision
        ));
    }

    let compatibility = if breaking.is_empty() {
        "compatible"
    } else {
        "breaking"
    };
    (compatibility.to_string(), breaking, non_breaking)
}

fn pipeline_contract_upgrade_advice(
    compatibility: &str,
    breaking: &[String],
    non_breaking: &[String],
) -> Vec<String> {
    let mut steps = Vec::new();
    if compatibility == "breaking" {
        steps.push(
            "Pin candidate rollout behind a feature flag until all consumers update.".to_string(),
        );
        steps.push(
            "Regenerate consumer mappings for ciDecisionCode and release together with producer."
                .to_string(),
        );
        if breaking
            .iter()
            .any(|entry| entry.contains("contractVersion changed"))
        {
            steps.push(
                "Treat this as major contract change: bump consumer contract parser compatibility matrix."
                    .to_string(),
            );
        }
        if breaking
            .iter()
            .any(|entry| entry.contains("ciDecisionCode changed"))
        {
            steps.push(
                "Update CI policy tables to the new ciDecisionCode mapping before enforcing strict gates."
                    .to_string(),
            );
        }
    } else {
        steps.push("Safe to rollout progressively; no breaking fields detected.".to_string());
        if !non_breaking.is_empty() {
            steps.push(
                "Notify observability owners about non-breaking reason/hash drift for dashboards."
                    .to_string(),
            );
        }
    }
    steps
}

fn tool_pipeline_contract_diff_json(baseline_path: &str, candidate_path: &str) {
    let baseline_source = match fs::read_to_string(baseline_path) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"read_error\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let candidate_source = match fs::read_to_string(candidate_path) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"read_error\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    let baseline_parsed = match serde_json::from_str::<JsonValue>(&baseline_source) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_json\",\"error\":{{\"message\":\"baseline: {}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let candidate_parsed = match serde_json::from_str::<JsonValue>(&candidate_source) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_json\",\"error\":{{\"message\":\"candidate: {}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    if let Err(message) = validate_pipeline_contract_doc(&baseline_parsed) {
        println!(
            "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_contract\",\"error\":{{\"message\":\"baseline: {}\"}}}}",
            json_escape(baseline_path),
            json_escape(candidate_path),
            json_escape(&message)
        );
        return;
    }
    if let Err(message) = validate_pipeline_contract_doc(&candidate_parsed) {
        println!(
            "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_contract\",\"error\":{{\"message\":\"candidate: {}\"}}}}",
            json_escape(baseline_path),
            json_escape(candidate_path),
            json_escape(&message)
        );
        return;
    }

    let (compatibility, breaking, non_breaking) =
        classify_pipeline_contract_compatibility(&baseline_parsed, &candidate_parsed);
    let gate = if compatibility == "compatible" {
        "pass"
    } else {
        "fail"
    };

    println!(
        "{{\"ok\":true,\"baseline\":\"{}\",\"candidate\":\"{}\",\"compatibility\":\"{}\",\"gate\":\"{}\",\"breakingChanges\":{},\"nonBreakingChanges\":{},\"summary\":{{\"breakingCount\":{},\"nonBreakingCount\":{}}}}}",
        json_escape(baseline_path),
        json_escape(candidate_path),
        compatibility,
        gate,
        json_string_array(&breaking),
        json_string_array(&non_breaking),
        breaking.len(),
        non_breaking.len()
    );
}

fn tool_pipeline_contract_upgrade_advice_json(baseline_path: &str, candidate_path: &str) {
    let baseline_source = match fs::read_to_string(baseline_path) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"read_error\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let candidate_source = match fs::read_to_string(candidate_path) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"read_error\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let baseline_parsed = match serde_json::from_str::<JsonValue>(&baseline_source) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_json\",\"error\":{{\"message\":\"baseline: {}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let candidate_parsed = match serde_json::from_str::<JsonValue>(&candidate_source) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_json\",\"error\":{{\"message\":\"candidate: {}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    if let Err(message) = validate_pipeline_contract_doc(&baseline_parsed) {
        println!(
            "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_contract\",\"error\":{{\"message\":\"baseline: {}\"}}}}",
            json_escape(baseline_path),
            json_escape(candidate_path),
            json_escape(&message)
        );
        return;
    }
    if let Err(message) = validate_pipeline_contract_doc(&candidate_parsed) {
        println!(
            "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_contract\",\"error\":{{\"message\":\"candidate: {}\"}}}}",
            json_escape(baseline_path),
            json_escape(candidate_path),
            json_escape(&message)
        );
        return;
    }

    let (compatibility, breaking, non_breaking) =
        classify_pipeline_contract_compatibility(&baseline_parsed, &candidate_parsed);
    let advice = pipeline_contract_upgrade_advice(&compatibility, &breaking, &non_breaking);
    let rollout = if compatibility == "breaking" {
        "controlled_migration"
    } else {
        "progressive_rollout"
    };

    println!(
        "{{\"ok\":true,\"baseline\":\"{}\",\"candidate\":\"{}\",\"compatibility\":\"{}\",\"rollout\":\"{}\",\"breakingChanges\":{},\"nonBreakingChanges\":{},\"advice\":{},\"summary\":{{\"breakingCount\":{},\"nonBreakingCount\":{},\"adviceCount\":{}}}}}",
        json_escape(baseline_path),
        json_escape(candidate_path),
        compatibility,
        rollout,
        json_string_array(&breaking),
        json_string_array(&non_breaking),
        json_string_array(&advice),
        breaking.len(),
        non_breaking.len(),
        advice.len()
    );
}

fn tool_pipeline_contract_bundle_json(baseline_path: &str, candidate_path: &str) {
    let baseline_source = match fs::read_to_string(baseline_path) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"read_error\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let candidate_source = match fs::read_to_string(candidate_path) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"read_error\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let baseline_parsed = match serde_json::from_str::<JsonValue>(&baseline_source) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_json\",\"error\":{{\"message\":\"baseline: {}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let candidate_parsed = match serde_json::from_str::<JsonValue>(&candidate_source) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_json\",\"error\":{{\"message\":\"candidate: {}\"}}}}",
                json_escape(baseline_path),
                json_escape(candidate_path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    if let Err(message) = validate_pipeline_contract_doc(&baseline_parsed) {
        println!(
            "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_contract\",\"error\":{{\"message\":\"baseline: {}\"}}}}",
            json_escape(baseline_path),
            json_escape(candidate_path),
            json_escape(&message)
        );
        return;
    }
    if let Err(message) = validate_pipeline_contract_doc(&candidate_parsed) {
        println!(
            "{{\"ok\":false,\"compatibility\":\"breaking\",\"baseline\":\"{}\",\"candidate\":\"{}\",\"reason\":\"invalid_contract\",\"error\":{{\"message\":\"candidate: {}\"}}}}",
            json_escape(baseline_path),
            json_escape(candidate_path),
            json_escape(&message)
        );
        return;
    }

    let (compatibility, breaking, non_breaking) =
        classify_pipeline_contract_compatibility(&baseline_parsed, &candidate_parsed);
    let advice = pipeline_contract_upgrade_advice(&compatibility, &breaking, &non_breaking);
    let gate = if compatibility == "compatible" {
        "pass"
    } else {
        "fail"
    };
    let rollout = if compatibility == "breaking" {
        "controlled_migration"
    } else {
        "progressive_rollout"
    };

    println!(
        "{{\"ok\":true,\"contractVersion\":\"1\",\"catalogHash\":\"{}\",\"baselinePath\":\"{}\",\"candidatePath\":\"{}\",\"baseline\":{},\"candidate\":{},\"diff\":{{\"compatibility\":\"{}\",\"gate\":\"{}\",\"breakingChanges\":{},\"nonBreakingChanges\":{}}},\"upgrade\":{{\"rollout\":\"{}\",\"advice\":{}}},\"summary\":{{\"breakingCount\":{},\"nonBreakingCount\":{},\"adviceCount\":{}}}}}",
        json_escape(&current_ci_catalog_hash()),
        json_escape(baseline_path),
        json_escape(candidate_path),
        baseline_parsed,
        candidate_parsed,
        compatibility,
        gate,
        json_string_array(&breaking),
        json_string_array(&non_breaking),
        rollout,
        json_string_array(&advice),
        breaking.len(),
        non_breaking.len(),
        advice.len()
    );
}

fn tool_pipeline_contract_bundle_example_json_string() -> String {
    let baseline = json!({
        "ok": true,
        "contractVersion": "1",
        "catalogHash": "fnv1a64:baseline",
        "ciDecision": "pass",
        "ciDecisionReason": "healthy",
        "ciDecisionCode": 0
    });
    let candidate = json!({
        "ok": true,
        "contractVersion": "1",
        "catalogHash": "fnv1a64:candidate",
        "ciDecision": "warn",
        "ciDecisionReason": "low_confidence",
        "ciDecisionCode": 10
    });
    let (compatibility, breaking, non_breaking) =
        classify_pipeline_contract_compatibility(&baseline, &candidate);
    let advice = pipeline_contract_upgrade_advice(&compatibility, &breaking, &non_breaking);
    let gate = if compatibility == "compatible" {
        "pass"
    } else {
        "fail"
    };
    let rollout = if compatibility == "breaking" {
        "controlled_migration"
    } else {
        "progressive_rollout"
    };
    json!({
        "ok": true,
        "contractVersion": "1",
        "usage": {
            "build_bundle": "matter-cli tool-pipeline-contract-bundle-json baseline.json candidate.json",
            "build_advice": "matter-cli tool-pipeline-contract-upgrade-advice-json baseline.json candidate.json",
            "gate_from_contract": "matter-cli tool-pipeline-contract-ci-gate-json candidate.json --warn-as-fail"
        },
        "baseline": baseline,
        "candidate": candidate,
        "bundle": {
            "diff": {
                "compatibility": compatibility,
                "gate": gate,
                "breakingChanges": breaking,
                "nonBreakingChanges": non_breaking
            },
            "upgrade": {
                "rollout": rollout,
                "advice": advice
            }
        }
    })
    .to_string()
}

fn print_tool_pipeline_contract_bundle_example_json() {
    println!("{}", tool_pipeline_contract_bundle_example_json_string());
}

fn tool_pipeline_apply_next_cycle_json(path: &str) {
    let source = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"read\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let parsed = match serde_json::from_str::<JsonValue>(&source) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"parse\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    let out_dir = parsed
        .get("out_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("tool_pipeline_demo");
    let strategy = parsed
        .get("next_cycle")
        .and_then(|v| v.get("strategy"))
        .and_then(|v| v.as_str())
        .and_then(normalize_strategy)
        .unwrap_or("prefer_latest");
    let energy_mode = parsed
        .get("next_cycle")
        .and_then(|v| v.get("energy_mode"))
        .and_then(|v| v.as_str())
        .and_then(normalize_energy_mode);
    let hop = parsed.get("hop").and_then(|v| v.as_u64()).unwrap_or(0);
    let max_hops = parsed.get("max_hops").and_then(|v| v.as_u64()).unwrap_or(3);
    if hop >= max_hops {
        println!(
            "{{\"ok\":false,\"stage\":\"guard\",\"input\":\"{}\",\"reason\":\"max_hops_exceeded\",\"hop\":{},\"max_hops\":{},\"error\":{{\"message\":\"next-cycle chain stopped by hop limit\"}}}}",
            json_escape(path),
            hop,
            max_hops
        );
        return;
    }

    tool_pipeline_demo_json(
        out_dir,
        false,
        strategy,
        false,
        false,
        false,
        false,
        false,
        false,
        false,
        "json",
        None,
        false,
        false,
        "",
        &PipelineScoring::balanced(),
        energy_mode,
        5.0,
        12.0,
        "balanced",
        None,
        None,
        Some(path),
        max_hops,
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

    let base_dir = entry_path.parent().unwrap_or(Path::new(".")).to_path_buf();
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

    let base_dir = entry_path.parent().unwrap_or(Path::new(".")).to_path_buf();
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
    push_lock_file(
        &mut files,
        &mut seen_files,
        "manifest",
        Path::new(&project.manifest_path),
    );
    push_lock_file(&mut files, &mut seen_files, "entry", &entry_path);
    for import in &imports {
        push_lock_file(
            &mut files,
            &mut seen_files,
            &import.source,
            Path::new(&import.resolved),
        );
    }

    let file_items: Vec<String> = files.iter().map(project_file_lock_json).collect();
    let import_items: Vec<String> = imports.iter().map(import_info_json).collect();
    let lock_fingerprint =
        project_lock_fingerprint(&files, &imports, &project.manifest.dependencies);

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

    let base_dir = entry_path.parent().unwrap_or(Path::new(".")).to_path_buf();
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
    push_lock_file(
        &mut files,
        &mut seen_files,
        "manifest",
        Path::new(&project.manifest_path),
    );
    push_lock_file(&mut files, &mut seen_files, "entry", &entry_path);
    for import in &imports {
        push_lock_file(
            &mut files,
            &mut seen_files,
            &import.source,
            Path::new(&import.resolved),
        );
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
        source.len(),
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

    let base_dir = entry_path.parent().unwrap_or(Path::new(".")).to_path_buf();
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
    push_lock_file(
        &mut files,
        &mut seen_files,
        "manifest",
        Path::new(&project.manifest_path),
    );
    push_lock_file(&mut files, &mut seen_files, "entry", &entry_path);
    for import in &imports {
        push_lock_file(
            &mut files,
            &mut seen_files,
            &import.source,
            Path::new(&import.resolved),
        );
    }
    let lock_fingerprint =
        project_lock_fingerprint(&files, &imports, &project.manifest.dependencies);

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

fn project_run_build_json(manifest_path: &str, output: Option<&str>, with_energy: bool) {
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

    let base_dir = entry_path.parent().unwrap_or(Path::new(".")).to_path_buf();
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
    let summary = bytecode_summary_json(&bytecode);

    let mut files = Vec::new();
    let mut seen_files = HashSet::new();
    push_lock_file(
        &mut files,
        &mut seen_files,
        "manifest",
        Path::new(&project.manifest_path),
    );
    push_lock_file(&mut files, &mut seen_files, "entry", &entry_path);
    for import in &imports {
        push_lock_file(
            &mut files,
            &mut seen_files,
            &import.source,
            Path::new(&import.resolved),
        );
    }
    let lock_fingerprint =
        project_lock_fingerprint(&files, &imports, &project.manifest.dependencies);

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

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);
    if let Err(error) = runtime.run() {
        if with_energy {
            println!(
                "{{\"ok\":false,\"stage\":\"runtime\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_file\":\"{}\",\"lock_fingerprint\":\"{}\",\"bytecode_fingerprint\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{},\"energy\":{{\"instruction_cost\":{:.2},\"backend_cost\":{:.2}}}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(&output_path),
                json_escape(&lock_fingerprint),
                json_escape(&fnv1a64_hex(&bytecode_bytes)),
                json_escape(&error),
                json_string_array(&runtime.take_output()),
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            );
        } else {
            println!(
                "{{\"ok\":false,\"stage\":\"runtime\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_file\":\"{}\",\"lock_fingerprint\":\"{}\",\"bytecode_fingerprint\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(&output_path),
                json_escape(&lock_fingerprint),
                json_escape(&fnv1a64_hex(&bytecode_bytes)),
                json_escape(&error),
                json_string_array(&runtime.take_output())
            );
        }
        process::exit(1);
    }

    if with_energy {
        println!(
            "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_file\":\"{}\",\"lock_fingerprint\":\"{}\",\"bytecode_fingerprint\":\"{}\",\"bytecode_bytes\":{},\"files_count\":{},\"imports_count\":{},\"dependencies_count\":{},\"output\":{},\"summary\":{},\"energy\":{{\"instruction_cost\":{:.2},\"backend_cost\":{:.2}}}}}",
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
            json_string_array(&runtime.take_output()),
            summary,
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        );
    } else {
        println!(
            "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_file\":\"{}\",\"lock_fingerprint\":\"{}\",\"bytecode_fingerprint\":\"{}\",\"bytecode_bytes\":{},\"files_count\":{},\"imports_count\":{},\"dependencies_count\":{},\"output\":{},\"summary\":{}}}",
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
            json_string_array(&runtime.take_output()),
            summary
        );
    }
}

fn project_emit_build_json(
    manifest_path: &str,
    event: &str,
    output: Option<&str>,
    with_energy: bool,
) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let entry_path = project_path(&project.base_dir, &project.manifest.entry);
    let entry_label = entry_path.display().to_string();
    let source = fs::read_to_string(&entry_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"event\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(event),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let base_dir = entry_path.parent().unwrap_or(Path::new(".")).to_path_buf();
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
            "{{\"ok\":false,\"stage\":\"import\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"event\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(event),
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
            "{{\"ok\":false,\"stage\":\"import\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"event\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(event),
            json_escape(&error)
        );
        process::exit(1);
    });

    let bytecode = build_json_or_exit(
        &resolved_source,
        &entry_label,
        &[("package", &project.manifest.name), ("event", event)],
    );
    let summary = bytecode_summary_json(&bytecode);

    let mut files = Vec::new();
    let mut seen_files = HashSet::new();
    push_lock_file(
        &mut files,
        &mut seen_files,
        "manifest",
        Path::new(&project.manifest_path),
    );
    push_lock_file(&mut files, &mut seen_files, "entry", &entry_path);
    for import in &imports {
        push_lock_file(
            &mut files,
            &mut seen_files,
            &import.source,
            Path::new(&import.resolved),
        );
    }
    let lock_fingerprint =
        project_lock_fingerprint(&files, &imports, &project.manifest.dependencies);

    let output_path = output
        .map(|path| path.to_string())
        .unwrap_or_else(|| project_artifact_path(&project.manifest.name, &lock_fingerprint));

    if let Some(parent) = Path::new(&output_path).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = fs::create_dir_all(parent) {
                println!(
                    "{{\"ok\":false,\"stage\":\"write\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"event\":\"{}\",\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                    json_escape(&project.manifest.name),
                    json_escape(&project.manifest_path),
                    json_escape(&entry_label),
                    json_escape(event),
                    json_escape(&output_path),
                    json_escape(&error.to_string())
                );
                process::exit(1);
            }
        }
    }

    if let Err(error) = bytecode.save_to_file(&output_path) {
        println!(
            "{{\"ok\":false,\"stage\":\"write\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"event\":\"{}\",\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(event),
            json_escape(&output_path),
            json_escape(&error.to_string())
        );
        process::exit(1);
    }

    let bytecode_bytes = fs::read(&output_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"read\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"event\":\"{}\",\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(event),
            json_escape(&output_path),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);
    if let Err(error) = runtime.emit_event(event) {
        if with_energy {
            println!(
                "{{\"ok\":false,\"stage\":\"runtime\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"event\":\"{}\",\"output_file\":\"{}\",\"lock_fingerprint\":\"{}\",\"bytecode_fingerprint\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{},\"energy\":{{\"instruction_cost\":{:.2},\"backend_cost\":{:.2}}}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(event),
                json_escape(&output_path),
                json_escape(&lock_fingerprint),
                json_escape(&fnv1a64_hex(&bytecode_bytes)),
                json_escape(&error),
                json_string_array(&runtime.take_output()),
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            );
        } else {
            println!(
                "{{\"ok\":false,\"stage\":\"runtime\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"event\":\"{}\",\"output_file\":\"{}\",\"lock_fingerprint\":\"{}\",\"bytecode_fingerprint\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(event),
                json_escape(&output_path),
                json_escape(&lock_fingerprint),
                json_escape(&fnv1a64_hex(&bytecode_bytes)),
                json_escape(&error),
                json_string_array(&runtime.take_output())
            );
        }
        process::exit(1);
    }

    if with_energy {
        println!(
            "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"event\":\"{}\",\"output_file\":\"{}\",\"lock_fingerprint\":\"{}\",\"bytecode_fingerprint\":\"{}\",\"bytecode_bytes\":{},\"files_count\":{},\"imports_count\":{},\"dependencies_count\":{},\"output\":{},\"summary\":{},\"energy\":{{\"instruction_cost\":{:.2},\"backend_cost\":{:.2}}}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(event),
            json_escape(&output_path),
            json_escape(&lock_fingerprint),
            json_escape(&fnv1a64_hex(&bytecode_bytes)),
            bytecode_bytes.len(),
            files.len(),
            imports.len(),
            project.manifest.dependencies.len(),
            json_string_array(&runtime.take_output()),
            summary,
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        );
    } else {
        println!(
            "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"event\":\"{}\",\"output_file\":\"{}\",\"lock_fingerprint\":\"{}\",\"bytecode_fingerprint\":\"{}\",\"bytecode_bytes\":{},\"files_count\":{},\"imports_count\":{},\"dependencies_count\":{},\"output\":{},\"summary\":{}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(event),
            json_escape(&output_path),
            json_escape(&lock_fingerprint),
            json_escape(&fnv1a64_hex(&bytecode_bytes)),
            bytecode_bytes.len(),
            files.len(),
            imports.len(),
            project.manifest.dependencies.len(),
            json_string_array(&runtime.take_output()),
            summary
        );
    }
}

fn project_visual_step_build_json(
    manifest_path: &str,
    events_path: &str,
    delta_ms_raw: &str,
    _output: Option<&str>,
    with_energy: bool,
) {
    let delta_ms = delta_ms_raw.parse::<i64>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"delta_ms must be an integer\"}}}}",
            json_escape(manifest_path),
            json_escape(events_path)
        );
        process::exit(1);
    });

    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(
        &source,
        &entry_label,
        &[("package", &project.manifest.name), ("events", events_path)],
    );

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);
    if let Err(error) = runtime.run() {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(events_path),
            json_escape(&error),
            json_string_array(&runtime.take_output()),
            energy_json_fragment(
                with_energy,
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            )
        );
        process::exit(1);
    }

    let result = runtime
        .visual_app_step(events_path, delta_ms)
        .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"visual\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"events\":\"{}\",\"deltaMs\":{},\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(events_path),
                delta_ms,
                json_escape(&error),
                json_string_array(&runtime.take_output()),
                energy_json_fragment(
                    with_energy,
                    runtime.vm().estimated_instruction_cost(),
                    runtime.vm().estimated_backend_cost()
                )
            );
            process::exit(1);
        });

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"events\":\"{}\",\"deltaMs\":{},\"result\":{},\"output\":{}{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(events_path),
        delta_ms,
        value_to_json(&result),
        json_string_array(&runtime.take_output()),
        energy_json_fragment(
            with_energy,
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        )
    );
}

fn project_visual_run_build_json(
    manifest_path: &str,
    events_path: &str,
    frames_raw: &str,
    delta_ms_raw: &str,
    _output: Option<&str>,
    with_energy: bool,
) {
    let frames = frames_raw.parse::<i64>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"frames must be an integer\"}}}}",
            json_escape(manifest_path),
            json_escape(events_path)
        );
        process::exit(1);
    });
    let delta_ms = delta_ms_raw.parse::<i64>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"delta_ms must be an integer\"}}}}",
            json_escape(manifest_path),
            json_escape(events_path)
        );
        process::exit(1);
    });

    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(
        &source,
        &entry_label,
        &[("package", &project.manifest.name), ("events", events_path)],
    );

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);
    if let Err(error) = runtime.run() {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(events_path),
            json_escape(&error),
            json_string_array(&runtime.take_output()),
            energy_json_fragment(
                with_energy,
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            )
        );
        process::exit(1);
    }

    let result = runtime
        .visual_app_run(events_path, frames, delta_ms)
        .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"visual\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"events\":\"{}\",\"frames\":{},\"deltaMs\":{},\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(events_path),
                frames,
                delta_ms,
                json_escape(&error),
                json_string_array(&runtime.take_output()),
                energy_json_fragment(
                    with_energy,
                    runtime.vm().estimated_instruction_cost(),
                    runtime.vm().estimated_backend_cost()
                )
            );
            process::exit(1);
        });

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"events\":\"{}\",\"frames\":{},\"deltaMs\":{},\"result\":{},\"output\":{}{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(events_path),
        frames,
        delta_ms,
        value_to_json(&result),
        json_string_array(&runtime.take_output()),
        energy_json_fragment(
            with_energy,
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        )
    );
}

fn project_web_build_json(manifest_path: &str, output_dir: &str, app_name: &str) {
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(
        &source,
        &entry_label,
        &[("package", &project.manifest.name), ("target", "web")],
    );

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

    if let Err(error) = runtime.visual_export_web_runtime(output_dir, app_name) {
        println!(
            "{{\"ok\":false,\"stage\":\"web_export\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(output_dir),
            json_escape(app_name),
            json_escape(&error),
            json_string_array(&runtime.take_output())
        );
        process::exit(1);
    }

    let verification = runtime
        .visual_verify_web_runtime(output_dir)
        .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"web_verify\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(output_dir),
                json_escape(app_name),
                json_escape(&error),
                json_string_array(&runtime.take_output())
            );
            process::exit(1);
        });

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"app\":\"{}\",\"output_dir\":\"{}\",\"verification\":{},\"output\":{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(app_name),
        json_escape(output_dir),
        value_to_json(&verification),
        json_string_array(&runtime.take_output())
    );
}

fn project_web_serve_json(
    manifest_path: &str,
    output_dir: &str,
    app_name: &str,
    port_raw: Option<&str>,
    once: bool,
) {
    let port = port_raw
        .unwrap_or("8080")
        .parse::<u16>()
        .unwrap_or_else(|_| {
            println!(
                "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
                json_escape(manifest_path),
                json_escape(output_dir),
                json_escape(app_name)
            );
            process::exit(1);
        });

    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(
        &source,
        &entry_label,
        &[("package", &project.manifest.name), ("target", "web")],
    );

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

    if let Err(error) = runtime.visual_export_web_runtime(output_dir, app_name) {
        println!(
            "{{\"ok\":false,\"stage\":\"web_export\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(output_dir),
            json_escape(app_name),
            json_escape(&error),
            json_string_array(&runtime.take_output())
        );
        process::exit(1);
    }

    let verification = runtime
        .visual_verify_web_runtime(output_dir)
        .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"web_verify\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(output_dir),
                json_escape(app_name),
                json_escape(&error),
                json_string_array(&runtime.take_output())
            );
            process::exit(1);
        });

    let addr = format!("127.0.0.1:{}", port);
    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"app\":\"{}\",\"output_dir\":\"{}\",\"port\":{},\"url\":\"http://{}/\",\"verification\":{},\"output\":{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(app_name),
        json_escape(output_dir),
        port,
        json_escape(&addr),
        value_to_json(&verification),
        json_string_array(&runtime.take_output())
    );

    serve_web_dir_blocking(output_dir, port, once);
}

fn project_web_smoke_json(
    manifest_path: &str,
    output_dir: &str,
    app_name: &str,
    port_raw: Option<&str>,
) {
    let port = port_raw
        .unwrap_or("8080")
        .parse::<u16>()
        .unwrap_or_else(|_| {
            println!(
                "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
                json_escape(manifest_path),
                json_escape(output_dir),
                json_escape(app_name)
            );
            process::exit(1);
        });

    // Build/export/verify first
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(
        &source,
        &entry_label,
        &[
            ("package", &project.manifest.name),
            ("target", "web"),
            ("smoke", "true"),
        ],
    );
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
    if let Err(error) = runtime.visual_export_web_runtime(output_dir, app_name) {
        println!(
            "{{\"ok\":false,\"stage\":\"web_export\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(output_dir),
            json_escape(app_name),
            json_escape(&error),
            json_string_array(&runtime.take_output())
        );
        process::exit(1);
    }

    let verify = runtime
        .visual_verify_web_runtime(output_dir)
        .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"web_verify\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(output_dir),
                json_escape(app_name),
                json_escape(&error),
                json_string_array(&runtime.take_output())
            );
            process::exit(1);
        });

    // Serve once in background, then hit /health
    let serve_dir = output_dir.to_string();
    let handle = std::thread::spawn(move || {
        serve_web_dir_blocking(&serve_dir, port, true);
    });
    std::thread::sleep(std::time::Duration::from_millis(120));

    let health = http_get_health(port);
    let _ = handle.join();

    match health {
        Ok(body) if body.contains("\"ok\":true") => {
            println!(
                "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"app\":\"{}\",\"output_dir\":\"{}\",\"port\":{},\"url\":\"http://127.0.0.1:{}/\",\"health\":{},\"verification\":{},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(app_name),
                json_escape(output_dir),
                port,
                port,
                body,
                value_to_json(&verify),
                json_string_array(&runtime.take_output())
            );
        }
        Ok(body) => {
            println!(
                "{{\"ok\":false,\"stage\":\"smoke\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"app\":\"{}\",\"output_dir\":\"{}\",\"port\":{},\"error\":{{\"message\":\"health check failed\"}},\"health\":{},\"verification\":{},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(app_name),
                json_escape(output_dir),
                port,
                body,
                value_to_json(&verify),
                json_string_array(&runtime.take_output())
            );
            process::exit(1);
        }
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"smoke\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"app\":\"{}\",\"output_dir\":\"{}\",\"port\":{},\"error\":{{\"message\":\"{}\"}},\"verification\":{},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(app_name),
                json_escape(output_dir),
                port,
                json_escape(&error),
                value_to_json(&verify),
                json_string_array(&runtime.take_output())
            );
            process::exit(1);
        }
    }
}

fn project_web_ci_json(
    manifest_path: &str,
    output_dir: &str,
    app_name: &str,
    port_raw: Option<&str>,
) {
    let port = port_raw
        .unwrap_or("8080")
        .parse::<u16>()
        .unwrap_or_else(|_| {
            println!(
                "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
                json_escape(manifest_path),
                json_escape(output_dir),
                json_escape(app_name)
            );
            process::exit(1);
        });

    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(
        &source,
        &entry_label,
        &[
            ("package", &project.manifest.name),
            ("target", "web"),
            ("ci", "true"),
        ],
    );
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
    if let Err(error) = runtime.visual_export_web_runtime(output_dir, app_name) {
        println!(
            "{{\"ok\":false,\"stage\":\"web_export\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(output_dir),
            json_escape(app_name),
            json_escape(&error),
            json_string_array(&runtime.take_output())
        );
        process::exit(1);
    }
    let verify = runtime
        .visual_verify_web_runtime(output_dir)
        .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"web_verify\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_dir\":\"{}\",\"app\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(output_dir),
                json_escape(app_name),
                json_escape(&error),
                json_string_array(&runtime.take_output())
            );
            process::exit(1);
        });

    let check = |path: &'static str| -> Result<String, String> {
        let serve_dir = output_dir.to_string();
        let handle = std::thread::spawn(move || {
            serve_web_dir_blocking(&serve_dir, port, true);
        });
        std::thread::sleep(std::time::Duration::from_millis(120));
        let body = http_get_path(port, path);
        let _ = handle.join();
        body
    };

    let health = check("/health");
    let meta = check("/__meta");
    let state = check("/state");
    let vm_state = check("/state/vm");
    let index = check("/");
    let events_push =
        check("/events?e=%7B%22type%22%3A%22ci_probe%22%2C%22target%22%3A%22matter%22%7D");
    let events_read = check("/events");
    let actions_list = check("/actions");
    let action_push = check("/actions?name=theme.apply");

    let health_ok = health
        .as_ref()
        .map(|body| body.contains("\"ok\":true"))
        .unwrap_or(false);
    let meta_ok = meta
        .as_ref()
        .map(|body| body.contains("\"port\":") && body.contains("\"root\":"))
        .unwrap_or(false);
    let index_ok = index
        .as_ref()
        .map(|body| body.contains("PXL Canvas Engine") || body.contains("<canvas"))
        .unwrap_or(false);
    let state_ok = state
        .as_ref()
        .map(|body| {
            body.contains("\"ok\":true")
                && body.contains("\"uptimeMs\":")
                && body.contains("\"events\":")
        })
        .unwrap_or(false);
    let vm_state_ok = vm_state
        .as_ref()
        .map(|body| {
            body.contains("\"ok\":true")
                && body.contains("\"processedEvents\":")
                && body.contains("\"visual\":")
        })
        .unwrap_or(false);
    let events_push_ok = events_push
        .as_ref()
        .map(|body| body.contains("\"accepted\":true"))
        .unwrap_or(false);
    let events_read_ok = events_read
        .as_ref()
        .map(|body| body.contains("\"ok\":true") && body.contains("ci_probe"))
        .unwrap_or(false);
    let action_push_ok = action_push
        .as_ref()
        .map(|body| {
            body.contains("\"accepted\":true") && body.contains("\"action\":\"theme.apply\"")
        })
        .unwrap_or(false);
    let actions_list_ok = actions_list
        .as_ref()
        .map(|body| {
            body.contains("\"ok\":true")
                && body.contains("\"theme.apply\"")
                && body.contains("\"counter.reset\"")
        })
        .unwrap_or(false);
    let ok = health_ok
        && meta_ok
        && state_ok
        && vm_state_ok
        && index_ok
        && events_push_ok
        && events_read_ok
        && actions_list_ok
        && action_push_ok;

    println!(
        "{{\"ok\":{},\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"app\":\"{}\",\"output_dir\":\"{}\",\"port\":{},\"url\":\"http://127.0.0.1:{}/\",\"checks\":{{\"health\":{},\"meta\":{},\"state\":{},\"vmState\":{},\"index\":{},\"eventsPush\":{},\"eventsRead\":{},\"actionsList\":{},\"actionPush\":{}}},\"responses\":{{\"health\":{},\"meta\":{},\"state\":{},\"vmState\":{},\"index\":{},\"eventsPush\":{},\"eventsRead\":{},\"actionsList\":{},\"actionPush\":{}}},\"verification\":{},\"output\":{}}}",
        ok,
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(app_name),
        json_escape(output_dir),
        port,
        port,
        health_ok,
        meta_ok,
        state_ok,
        vm_state_ok,
        index_ok,
        events_push_ok,
        events_read_ok,
        actions_list_ok,
        action_push_ok,
        result_json_string(&health),
        result_json_string(&meta),
        result_json_string(&state),
        result_json_string(&vm_state),
        result_json_string(&index),
        result_json_string(&events_push),
        result_json_string(&events_read),
        result_json_string(&actions_list),
        result_json_string(&action_push),
        value_to_json(&verify),
        json_string_array(&runtime.take_output())
    );

    if !ok {
        process::exit(1);
    }
}

fn web_events_save_json(port_raw: &str, output_events: &str, clear: bool) {
    let port = port_raw.parse::<u16>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"port\":\"{}\",\"output\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
            json_escape(port_raw),
            json_escape(output_events)
        );
        process::exit(1);
    });
    let path = if clear {
        "/events/queue?clear=1"
    } else {
        "/events/queue"
    };
    let body = http_get_path(port, path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"fetch\",\"port\":{},\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            port,
            json_escape(output_events),
            json_escape(&error)
        );
        process::exit(1);
    });
    if let Err(error) = fs::write(output_events, &body) {
        println!(
            "{{\"ok\":false,\"stage\":\"write\",\"port\":{},\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            port,
            json_escape(output_events),
            json_escape(&error.to_string())
        );
        process::exit(1);
    }
    println!(
        "{{\"ok\":true,\"port\":{},\"path\":\"{}\",\"clear\":{},\"bytes\":{}}}",
        port,
        json_escape(output_events),
        clear,
        body.len()
    );
}

fn web_state_json(port_raw: &str) {
    let port = port_raw.parse::<u16>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"port\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
            json_escape(port_raw)
        );
        process::exit(1);
    });
    let body = http_get_path(port, "/state").unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"fetch\",\"port\":{},\"error\":{{\"message\":\"{}\"}}}}",
            port,
            json_escape(&error)
        );
        process::exit(1);
    });
    println!("{}", body);
}

fn web_events_tail_json(
    port_raw: &str,
    cursor_raw: Option<&str>,
    limit_raw: Option<&str>,
    ack: bool,
) {
    let port = port_raw.parse::<u16>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"port\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
            json_escape(port_raw)
        );
        process::exit(1);
    });
    let cursor = cursor_raw.unwrap_or("0");
    let limit = limit_raw.unwrap_or("50");
    let path = if ack {
        format!("/state/events?cursor={}&limit={}&ack=1", cursor, limit)
    } else {
        format!("/state/events?cursor={}&limit={}", cursor, limit)
    };
    let body = http_get_path(port, &path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"fetch\",\"port\":{},\"error\":{{\"message\":\"{}\"}}}}",
            port,
            json_escape(&error)
        );
        process::exit(1);
    });
    println!("{}", body);
}

fn web_action_json(port_raw: &str, action: &str) {
    let port = port_raw.parse::<u16>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"port\":\"{}\",\"action\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
            json_escape(port_raw),
            json_escape(action)
        );
        process::exit(1);
    });
    let path = format!("/actions?name={}", url_encode_component(action));
    let body = http_get_path(port, &path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"fetch\",\"port\":{},\"action\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            port,
            json_escape(action),
            json_escape(&error)
        );
        process::exit(1);
    });
    println!("{}", body);
}

fn web_actions_json(port_raw: &str) {
    let port = port_raw.parse::<u16>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"port\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
            json_escape(port_raw)
        );
        process::exit(1);
    });
    let body = http_get_path(port, "/actions").unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"fetch\",\"port\":{},\"error\":{{\"message\":\"{}\"}}}}",
            port,
            json_escape(&error)
        );
        process::exit(1);
    });
    println!("{}", body);
}

fn web_live_demo_check_json(port_raw: &str, timeout_ms_raw: Option<&str>) {
    let port = port_raw.parse::<u16>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"port\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
            json_escape(port_raw)
        );
        process::exit(1);
    });
    let timeout_ms = timeout_ms_raw
        .unwrap_or("3000")
        .parse::<u64>()
        .unwrap_or_else(|_| {
            println!(
                "{{\"ok\":false,\"stage\":\"args\",\"port\":{},\"timeoutMs\":\"{}\",\"error\":{{\"message\":\"timeout_ms must be an integer\"}}}}",
                port,
                json_escape(timeout_ms_raw.unwrap_or(""))
            );
            process::exit(1);
        });

    let before_body = http_get_path(port, "/state/vm").unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"before\",\"port\":{},\"error\":{{\"message\":\"{}\"}}}}",
            port,
            json_escape(&error)
        );
        process::exit(1);
    });
    let before = parse_vm_state_probe(&before_body);
    let click_event = format!(
        "{{\"type\":\"click\",\"timestamp\":{},\"source\":\"canvas\",\"payload\":{{\"target\":\"web-live-demo-check\",\"x\":1,\"y\":1}}}}",
        current_timestamp_ms()
    );
    let push_path = format!("/events?e={}", url_encode_component(&click_event));
    let push_body = http_get_path(port, &push_path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"push\",\"port\":{},\"error\":{{\"message\":\"{}\"}},\"before\":{}}}",
            port,
            json_escape(&error),
            before_body
        );
        process::exit(1);
    });

    let started = std::time::Instant::now();
    let mut after_body = before_body.clone();
    let mut after = before.clone();
    while started.elapsed().as_millis() as u64 <= timeout_ms {
        std::thread::sleep(std::time::Duration::from_millis(120));
        if let Ok(body) = http_get_path(port, "/state/vm") {
            after = parse_vm_state_probe(&body);
            after_body = body;
            if after.click_counter > before.click_counter
                && after.processed_events > before.processed_events
            {
                break;
            }
        }
    }

    let click_delta = after.click_counter.saturating_sub(before.click_counter);
    let processed_delta = after
        .processed_events
        .saturating_sub(before.processed_events);
    let color_changed = before.color != after.color;
    let ok = click_delta > 0 && processed_delta > 0;
    println!(
        "{{\"ok\":{},\"port\":{},\"timeoutMs\":{},\"event\":{},\"push\":{},\"before\":{},\"after\":{},\"delta\":{{\"clickCounter\":{},\"processedEvents\":{},\"colorChanged\":{}}}}}",
        ok,
        port,
        timeout_ms,
        click_event,
        push_body,
        before_body,
        after_body,
        click_delta,
        processed_delta,
        color_changed
    );
    if !ok {
        process::exit(1);
    }
}

fn project_web_step_live_json(
    manifest_path: &str,
    port_raw: &str,
    delta_ms_raw: &str,
    clear: bool,
) {
    let port = port_raw.parse::<u16>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"port\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
            json_escape(manifest_path),
            json_escape(port_raw)
        );
        process::exit(1);
    });
    let path = if clear {
        "/events/queue?clear=1"
    } else {
        "/events/queue"
    };
    let body = http_get_path(port, path).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"fetch\",\"manifest\":\"{}\",\"port\":{},\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(manifest_path),
            port,
            json_escape(&error)
        );
        process::exit(1);
    });

    let project = load_project_or_json_exit(manifest_path);
    let events_path = project
        .base_dir
        .join(".matter_live_events.json")
        .display()
        .to_string();
    if let Err(error) = fs::write(&events_path, body.as_bytes()) {
        println!(
            "{{\"ok\":false,\"stage\":\"write\",\"manifest\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(manifest_path),
            json_escape(&events_path),
            json_escape(&error.to_string())
        );
        process::exit(1);
    }
    let delta_ms = delta_ms_raw.parse::<i64>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"deltaMs\":\"{}\",\"error\":{{\"message\":\"delta_ms must be an integer\"}}}}",
            json_escape(manifest_path),
            json_escape(delta_ms_raw)
        );
        process::exit(1);
    });
    let project = load_project_or_json_exit(manifest_path);
    let _env = apply_project_env(&project);
    let (source, entry_label) = read_project_entry_or_json_exit(&project);
    let bytecode = build_json_or_exit(
        &source,
        &entry_label,
        &[
            ("package", &project.manifest.name),
            ("events", &events_path),
        ],
    );
    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);
    if let Err(error) = runtime.run() {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
            json_escape(&project.manifest.name),
            json_escape(&project.manifest_path),
            json_escape(&entry_label),
            json_escape(&events_path),
            json_escape(&error),
            json_string_array(&runtime.take_output())
        );
        process::exit(1);
    }
    let result = runtime
        .visual_app_step(&events_path, delta_ms)
        .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"visual\",\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"events\":\"{}\",\"deltaMs\":{},\"error\":{{\"message\":\"{}\"}},\"output\":{}}}",
                json_escape(&project.manifest.name),
                json_escape(&project.manifest_path),
                json_escape(&entry_label),
                json_escape(&events_path),
                delta_ms,
                json_escape(&error),
                json_string_array(&runtime.take_output())
            );
            process::exit(1);
        });

    let mut vm_state = vm_live_state().lock().unwrap_or_else(|e| e.into_inner());
    let events = parse_event_queue_events(&body);
    let stats = reduce_events_into_state(&events, &mut vm_state);
    vm_state.last_step_result = value_to_json(&result);
    let publish = publish_vm_live_state(port, &vm_state);

    println!(
        "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"events\":\"{}\",\"deltaMs\":{},\"processed\":{{\"events\":{},\"click\":{},\"key\":{},\"tick\":{},\"input\":{},\"action\":{}}},\"vm\":{{\"clickCounter\":{},\"keyCounter\":{},\"inputCounter\":{},\"tickCounter\":{},\"actionCounter\":{},\"processedEvents\":{},\"color\":\"{}\"}},\"publish\":{},\"result\":{},\"output\":{}}}",
        json_escape(&project.manifest.name),
        json_escape(&project.manifest_path),
        json_escape(&entry_label),
        json_escape(&events_path),
        delta_ms,
        stats.total,
        stats.click,
        stats.key,
        stats.tick,
        stats.input,
        stats.action,
        vm_state.click_counter,
        vm_state.key_counter,
        vm_state.input_counter,
        vm_state.tick_counter,
        vm_state.action_counter,
        vm_state.processed_events,
        json_escape(&vm_state.visual_color),
        result_json_string(&publish),
        vm_state.last_step_result,
        json_string_array(&runtime.take_output())
    );
}

fn project_web_loop_live_json(
    manifest_path: &str,
    port_raw: &str,
    delta_ms_raw: &str,
    ticks_raw: &str,
    interval_ms: i64,
    clear_each_tick: bool,
    telemetry_ms: i64,
) {
    let port = port_raw.parse::<u16>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"port\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
            json_escape(manifest_path),
            json_escape(port_raw)
        );
        process::exit(1);
    });
    let tick_limit = if ticks_raw.eq_ignore_ascii_case("forever") {
        None
    } else {
        let ticks = ticks_raw.parse::<i64>().unwrap_or_else(|_| {
            println!(
                "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"ticks\":\"{}\",\"error\":{{\"message\":\"ticks must be an integer or 'forever'\"}}}}",
                json_escape(manifest_path),
                json_escape(ticks_raw)
            );
            process::exit(1);
        });
        if ticks <= 0 {
            println!(
                "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"ticks\":{},\"error\":{{\"message\":\"ticks must be > 0\"}}}}",
                json_escape(manifest_path),
                ticks
            );
            process::exit(1);
        }
        Some(ticks)
    };
    let delta_ms = delta_ms_raw.parse::<i64>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"deltaMs\":\"{}\",\"error\":{{\"message\":\"delta_ms must be an integer\"}}}}",
            json_escape(manifest_path),
            json_escape(delta_ms_raw)
        );
        process::exit(1);
    });
    if interval_ms < 0 {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"intervalMs\":{},\"error\":{{\"message\":\"interval_ms must be >= 0\"}}}}",
            json_escape(manifest_path),
            interval_ms
        );
        process::exit(1);
    }
    if telemetry_ms <= 0 {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"telemetryMs\":{},\"error\":{{\"message\":\"telemetry_ms must be > 0\"}}}}",
            json_escape(manifest_path),
            telemetry_ms
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"stage\":\"start\",\"manifest\":\"{}\",\"port\":{},\"ticks\":\"{}\",\"intervalMs\":{},\"clearEachTick\":{},\"deltaMs\":{},\"telemetryMs\":{}}}",
        json_escape(manifest_path),
        port,
        json_escape(ticks_raw),
        interval_ms,
        clear_each_tick,
        delta_ms,
        telemetry_ms
    );
    let started_at = std::time::Instant::now();
    let mut telemetry_window_started = std::time::Instant::now();
    let mut telemetry_window_ticks: i64 = 0;
    let mut tick: i64 = 0;
    loop {
        tick += 1;
        if let Some(limit) = tick_limit {
            if tick > limit {
                break;
            }
        }
        let ack = clear_each_tick;
        let limit = 512u64;
        let (queue_json, next_cursor, _total_count, _click_count, _key_count, _last_event) =
            fetch_incremental_event_queue(port, WEB_EVENT_ACK_CURSOR.load(Ordering::Relaxed), limit, ack)
                .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"fetch\",\"mode\":\"incremental\",\"tick\":{},\"manifest\":\"{}\",\"port\":{},\"error\":{{\"message\":\"{}\"}}}}",
                tick,
                json_escape(manifest_path),
                port,
                json_escape(&error)
            );
            process::exit(1);
        });
        WEB_EVENT_ACK_CURSOR.store(next_cursor, Ordering::Relaxed);

        let project = load_project_or_json_exit(manifest_path);
        let events_path = project
            .base_dir
            .join(".matter_live_events.json")
            .display()
            .to_string();
        if let Err(error) = fs::write(&events_path, queue_json.as_bytes()) {
            println!(
                "{{\"ok\":false,\"stage\":\"write\",\"tick\":{},\"manifest\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                tick,
                json_escape(manifest_path),
                json_escape(&events_path),
                json_escape(&error.to_string())
            );
            process::exit(1);
        }

        project_visual_step_build_json(manifest_path, &events_path, delta_ms_raw, None, false);
        {
            let mut vm_state = vm_live_state().lock().unwrap_or_else(|e| e.into_inner());
            let events = parse_event_queue_events(&queue_json);
            let _stats = reduce_events_into_state(&events, &mut vm_state);
            let _ = publish_vm_live_state(port, &vm_state);
        }
        telemetry_window_ticks += 1;

        let elapsed_telemetry_ms = telemetry_window_started.elapsed().as_millis() as i64;
        if elapsed_telemetry_ms >= telemetry_ms {
            let elapsed_sec = (elapsed_telemetry_ms as f64) / 1000.0;
            let tps = if elapsed_sec > 0.0 {
                telemetry_window_ticks as f64 / elapsed_sec
            } else {
                0.0
            };
            println!(
                "{{\"ok\":true,\"stage\":\"telemetry\",\"tick\":{},\"uptimeMs\":{},\"windowMs\":{},\"windowTicks\":{},\"ticksPerSec\":{:.3}}}",
                tick,
                started_at.elapsed().as_millis(),
                elapsed_telemetry_ms,
                telemetry_window_ticks,
                tps
            );
            telemetry_window_started = std::time::Instant::now();
            telemetry_window_ticks = 0;
        }

        let has_more = tick_limit.map(|limit| tick < limit).unwrap_or(true);
        if has_more && interval_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(interval_ms as u64));
        }
    }
    println!(
        "{{\"ok\":true,\"stage\":\"done\",\"ticks\":{},\"uptimeMs\":{}}}",
        tick_limit.unwrap_or(tick),
        started_at.elapsed().as_millis()
    );
}

#[allow(clippy::zombie_processes)]
fn start_live_demo_json(
    manifest_path: &str,
    output_dir: &str,
    app_name: &str,
    port_raw: &str,
    delta_ms: i64,
    interval_ms: i64,
    telemetry_ms: i64,
) {
    let port = port_raw.parse::<u16>().unwrap_or_else(|_| {
        println!(
            "{{\"ok\":false,\"stage\":\"args\",\"manifest\":\"{}\",\"port\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
            json_escape(manifest_path),
            json_escape(port_raw)
        );
        process::exit(1);
    });
    let exe = env::current_exe().unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"exe\",\"manifest\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(manifest_path),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });
    let cwd = env::current_dir().unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"cwd\",\"manifest\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(manifest_path),
            json_escape(&error.to_string())
        );
        process::exit(1);
    });

    let serve_child = Command::new(&exe)
        .current_dir(&cwd)
        .arg("project-web-serve-json")
        .arg(manifest_path)
        .arg(output_dir)
        .arg(app_name)
        .arg(port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"spawn_serve\",\"manifest\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(manifest_path),
                json_escape(&error.to_string())
            );
            process::exit(1);
        });

    std::thread::sleep(std::time::Duration::from_millis(700));

    let loop_child = Command::new(&exe)
        .current_dir(&cwd)
        .arg("project-web-loop-live-json")
        .arg(manifest_path)
        .arg(port.to_string())
        .arg(delta_ms.to_string())
        .arg("forever")
        .arg("--interval-ms")
        .arg(interval_ms.to_string())
        .arg("--telemetry-ms")
        .arg(telemetry_ms.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"spawn_loop\",\"manifest\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"servePid\":{}}}",
                json_escape(manifest_path),
                json_escape(&error.to_string()),
                serve_child.id()
            );
            process::exit(1);
        });

    std::thread::sleep(std::time::Duration::from_millis(900));
    let health = http_get_health(port);
    let health_ok = health
        .as_ref()
        .map(|body| body.contains("\"ok\":true"))
        .unwrap_or(false);

    println!(
        "{{\"ok\":{},\"manifest\":\"{}\",\"outputDir\":\"{}\",\"app\":\"{}\",\"port\":{},\"servePid\":{},\"loopPid\":{},\"canvasUrl\":\"http://127.0.0.1:{}/\",\"stateUrl\":\"http://127.0.0.1:{}/state/vm\",\"health\":{}}}",
        health_ok,
        json_escape(manifest_path),
        json_escape(output_dir),
        json_escape(app_name),
        port,
        serve_child.id(),
        loop_child.id(),
        port,
        port,
        result_json_string(&health)
    );
}

fn http_get_health(port: u16) -> Result<String, String> {
    http_get_path(port, "/health")
}

fn http_get_path(port: u16, path: &str) -> Result<String, String> {
    let addr = format!("127.0.0.1:{}", port);
    let mut stream = TcpStream::connect(&addr).map_err(|e| e.to_string())?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        path
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw).map_err(|e| e.to_string())?;
    let body = raw
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    Ok(body)
}

fn result_json_string(result: &Result<String, String>) -> String {
    match result {
        Ok(value) => format!("\"{}\"", json_escape(value)),
        Err(error) => format!("\"error:{}\"", json_escape(error)),
    }
}

fn web_serve_json(dir: &str, port_raw: Option<&str>, once: bool) {
    let port = port_raw
        .unwrap_or("8080")
        .parse::<u16>()
        .unwrap_or_else(|_| {
            println!(
                "{{\"ok\":false,\"stage\":\"args\",\"dir\":\"{}\",\"error\":{{\"message\":\"port must be a valid u16\"}}}}",
                json_escape(dir)
            );
            process::exit(1);
        });

    println!(
        "{{\"ok\":true,\"dir\":\"{}\",\"port\":{},\"url\":\"http://127.0.0.1:{}/\"}}",
        json_escape(dir),
        port,
        port
    );
    serve_web_dir_blocking(dir, port, once);
}

fn serve_web_dir_blocking(dir: &str, port: u16, once: bool) {
    let _ = WEB_SERVER_STARTED_AT.get_or_init(Instant::now);
    let root = Path::new(dir);
    if !root.exists() || !root.is_dir() {
        eprintln!("web server error: directory does not exist: {}", dir);
        process::exit(1);
    }
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).unwrap_or_else(|error| {
        eprintln!("web server bind error on {}: {}", addr, error);
        process::exit(1);
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_web_request(stream, root, port) {
                    eprintln!("web request error: {}", error);
                }
                if once {
                    break;
                }
            }
            Err(error) => eprintln!("web connection error: {}", error),
        }
    }
}

fn web_event_queue() -> &'static Mutex<Vec<(u64, String)>> {
    WEB_EVENT_QUEUE.get_or_init(|| Mutex::new(Vec::new()))
}

fn url_decode_component(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = bytes[i + 1] as char;
                let lo = bytes[i + 2] as char;
                let hex = [hi, lo].iter().collect::<String>();
                let value = u8::from_str_radix(&hex, 16).ok()?;
                out.push(value);
                i += 3;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

fn url_encode_component(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{:02X}", byte));
        }
    }
    out
}

fn query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        let k = it.next().unwrap_or("");
        if k == key {
            let raw = it.next().unwrap_or("");
            return url_decode_component(raw);
        }
    }
    None
}

fn query_u64(query: &str, key: &str, default: u64) -> u64 {
    query_param(query, key)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn is_known_action(action: &str) -> bool {
    matches!(action, "theme.apply" | "counter.reset")
}

fn actions_registry_json() -> &'static str {
    "{\"ok\":true,\"schemaVersion\":1,\"actions\":[{\"name\":\"theme.apply\",\"type\":\"style\",\"description\":\"Toggle visual theme phase\"},{\"name\":\"counter.reset\",\"type\":\"state\",\"description\":\"Reset live counters\"}]}"
}

fn vm_live_state() -> &'static Mutex<VmLiveState> {
    VM_LIVE_STATE.get_or_init(|| Mutex::new(VmLiveState::default()))
}

fn vm_live_state_snapshot_json(vm_state: &VmLiveState) -> String {
    format!(
        "{{\"processedEvents\":{},\"clickCounter\":{},\"keyCounter\":{},\"inputCounter\":{},\"tickCounter\":{},\"actionCounter\":{},\"lastKey\":\"{}\",\"lastInput\":\"{}\",\"lastAction\":\"{}\",\"lastEvent\":{},\"lastStepResult\":{},\"visualColor\":\"{}\"}}",
        vm_state.processed_events,
        vm_state.click_counter,
        vm_state.key_counter,
        vm_state.input_counter,
        vm_state.tick_counter,
        vm_state.action_counter,
        json_escape(&vm_state.last_key),
        json_escape(&vm_state.last_input),
        json_escape(&vm_state.last_action),
        vm_state.last_event,
        vm_state.last_step_result,
        json_escape(&vm_state.visual_color)
    )
}

fn vm_live_state_response_json(vm_state: &VmLiveState) -> String {
    format!(
        "{{\"ok\":true,\"public\":{{\"schemaVersion\":2,\"clickCounter\":{},\"keyCounter\":{},\"inputCounter\":{},\"tickCounter\":{},\"actionCounter\":{},\"lastKey\":\"{}\",\"lastInput\":\"{}\",\"lastAction\":\"{}\",\"color\":\"{}\",\"components\":[{{\"name\":\"clickCounter\",\"type\":\"metric\",\"props\":{{\"value\":{},\"label\":\"Clicks\"}},\"actions\":[]}},{{\"name\":\"keyCounter\",\"type\":\"metric\",\"props\":{{\"value\":{},\"label\":\"Keys\"}},\"actions\":[]}},{{\"name\":\"inputCounter\",\"type\":\"metric\",\"props\":{{\"value\":{},\"label\":\"Inputs\"}},\"actions\":[]}},{{\"name\":\"tickCounter\",\"type\":\"metric\",\"props\":{{\"value\":{},\"label\":\"Ticks\"}},\"actions\":[]}},{{\"name\":\"actionCounter\",\"type\":\"metric\",\"props\":{{\"value\":{},\"label\":\"Actions\"}},\"actions\":[]}},{{\"name\":\"visualColor\",\"type\":\"style\",\"props\":{{\"value\":\"{}\",\"label\":\"Color\"}},\"actions\":[\"theme.apply\"]}},{{\"name\":\"counterControls\",\"type\":\"control\",\"props\":{{\"label\":\"Counters\"}},\"actions\":[\"counter.reset\"]}}]}},\"processedEvents\":{},\"lastEvent\":{},\"lastStepResult\":{},\"visual\":{{\"counter\":{},\"color\":\"{}\"}},\"snapshot\":{}}}",
        vm_state.click_counter,
        vm_state.key_counter,
        vm_state.input_counter,
        vm_state.tick_counter,
        vm_state.action_counter,
        json_escape(&vm_state.last_key),
        json_escape(&vm_state.last_input),
        json_escape(&vm_state.last_action),
        json_escape(&vm_state.visual_color),
        vm_state.click_counter,
        vm_state.key_counter,
        vm_state.input_counter,
        vm_state.tick_counter,
        vm_state.action_counter,
        json_escape(&vm_state.visual_color),
        vm_state.processed_events,
        vm_state.last_event,
        vm_state.last_step_result,
        vm_state.click_counter,
        json_escape(&vm_state.visual_color),
        vm_live_state_snapshot_json(vm_state)
    )
}

fn apply_vm_live_state_snapshot(raw: &str, vm_state: &mut VmLiveState) -> Result<(), String> {
    let doc: JsonValue = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    vm_state.processed_events = doc
        .get("processedEvents")
        .and_then(|v| v.as_u64())
        .unwrap_or(vm_state.processed_events);
    vm_state.click_counter = doc
        .get("clickCounter")
        .and_then(|v| v.as_u64())
        .unwrap_or(vm_state.click_counter);
    vm_state.key_counter = doc
        .get("keyCounter")
        .and_then(|v| v.as_u64())
        .unwrap_or(vm_state.key_counter);
    vm_state.input_counter = doc
        .get("inputCounter")
        .and_then(|v| v.as_u64())
        .unwrap_or(vm_state.input_counter);
    vm_state.tick_counter = doc
        .get("tickCounter")
        .and_then(|v| v.as_u64())
        .unwrap_or(vm_state.tick_counter);
    vm_state.action_counter = doc
        .get("actionCounter")
        .and_then(|v| v.as_u64())
        .unwrap_or(vm_state.action_counter);
    vm_state.last_key = doc
        .get("lastKey")
        .and_then(|v| v.as_str())
        .unwrap_or(&vm_state.last_key)
        .to_string();
    vm_state.last_input = doc
        .get("lastInput")
        .and_then(|v| v.as_str())
        .unwrap_or(&vm_state.last_input)
        .to_string();
    vm_state.last_action = doc
        .get("lastAction")
        .and_then(|v| v.as_str())
        .unwrap_or(&vm_state.last_action)
        .to_string();
    if let Some(event) = doc.get("lastEvent") {
        vm_state.last_event = event.to_string();
    }
    if let Some(result) = doc.get("lastStepResult") {
        vm_state.last_step_result = result.to_string();
    }
    vm_state.visual_color = doc
        .get("visualColor")
        .and_then(|v| v.as_str())
        .unwrap_or(&vm_state.visual_color)
        .to_string();
    Ok(())
}

fn publish_vm_live_state(port: u16, vm_state: &VmLiveState) -> Result<String, String> {
    let snapshot = vm_live_state_snapshot_json(vm_state);
    let path = format!("/state/vm/update?s={}", url_encode_component(&snapshot));
    http_get_path(port, &path)
}

#[derive(Clone, Default)]
struct VmStateProbe {
    processed_events: u64,
    click_counter: u64,
    color: String,
}

fn parse_vm_state_probe(body: &str) -> VmStateProbe {
    let parsed = serde_json::from_str::<JsonValue>(body).unwrap_or(JsonValue::Null);
    VmStateProbe {
        processed_events: parsed
            .get("processedEvents")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        click_counter: parsed
            .get("visual")
            .and_then(|v| v.get("counter"))
            .and_then(|v| v.as_u64())
            .or_else(|| {
                parsed
                    .get("public")
                    .and_then(|v| v.get("clickCounter"))
                    .and_then(|v| v.as_u64())
            })
            .unwrap_or(0),
        color: parsed
            .get("visual")
            .and_then(|v| v.get("color"))
            .and_then(|v| v.as_str())
            .or_else(|| {
                parsed
                    .get("public")
                    .and_then(|v| v.get("color"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("")
            .to_string(),
    }
}

fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn detect_event_type(raw: &str) -> &'static str {
    if raw.contains("\"type\":\"click\"") || raw.contains("\"type\":\"pointer\"") {
        "click"
    } else if raw.contains("\"type\":\"key\"") || raw.contains("\"type\":\"keyboard\"") {
        "key"
    } else if raw.contains("\"type\":\"tick\"") {
        "tick"
    } else {
        "input"
    }
}

fn canonical_event_type(event_type: Option<&str>, raw: &str) -> &'static str {
    match event_type.unwrap_or("") {
        "click" | "pointer" => "click",
        "key" | "keyboard" => "key",
        "tick" => "tick",
        "input" => "input",
        _ => detect_event_type(raw),
    }
}

fn normalize_event_json(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Ok(doc) = serde_json::from_str::<JsonValue>(trimmed) {
        let event_type = canonical_event_type(doc.get("type").and_then(|v| v.as_str()), trimmed);
        let timestamp = doc
            .get("timestamp")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(current_timestamp_ms);
        let source = doc
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("canvas");
        if let Some(payload) = doc.get("payload") {
            return json!({
                "type": event_type,
                "timestamp": timestamp,
                "source": source,
                "payload": payload,
            })
            .to_string();
        }
        return json!({
            "type": event_type,
            "timestamp": timestamp,
            "source": source,
            "payload": doc,
        })
        .to_string();
    }
    json!({
        "type": detect_event_type(trimmed),
        "timestamp": current_timestamp_ms(),
        "source": "canvas",
        "payload": { "value": trimmed },
    })
    .to_string()
}

#[derive(Default)]
struct EventBatchStats {
    total: u64,
    click: u64,
    key: u64,
    tick: u64,
    input: u64,
    action: u64,
    last_event: String,
}

fn reduce_events_into_state(events: &[JsonValue], vm_state: &mut VmLiveState) -> EventBatchStats {
    let mut stats = EventBatchStats {
        last_event: "{}".to_string(),
        ..EventBatchStats::default()
    };
    for event in events {
        stats.total += 1;
        stats.last_event = event.to_string();
        let event_type = event
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("input");
        let payload = event.get("payload").cloned().unwrap_or(JsonValue::Null);
        match event_type {
            "click" => {
                stats.click += 1;
                vm_state.click_counter = vm_state.click_counter.saturating_add(1);
            }
            "key" => {
                stats.key += 1;
                vm_state.key_counter = vm_state.key_counter.saturating_add(1);
                vm_state.last_key = payload
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
            }
            "tick" => {
                stats.tick += 1;
                vm_state.tick_counter = vm_state.tick_counter.saturating_add(1);
            }
            _ => {
                stats.input += 1;
                vm_state.input_counter = vm_state.input_counter.saturating_add(1);
                vm_state.last_input = payload.to_string();
            }
        }
        let action = payload
            .get("action")
            .and_then(|v| v.as_str())
            .or_else(|| payload.get("event").and_then(|v| v.as_str()))
            .filter(|name| is_known_action(name));
        if let Some(action) = action {
            stats.action += 1;
            vm_state.action_counter = vm_state.action_counter.saturating_add(1);
            vm_state.last_action = action.to_string();
            if action == "counter.reset" {
                vm_state.click_counter = 0;
                vm_state.key_counter = 0;
                vm_state.input_counter = 0;
                vm_state.tick_counter = 0;
            }
        }
    }
    vm_state.processed_events = vm_state.processed_events.saturating_add(stats.total);
    if stats.total > 0 {
        vm_state.last_event = stats.last_event.clone();
    }
    let color_phase = vm_state
        .click_counter
        .saturating_add(vm_state.action_counter);
    vm_state.visual_color = if color_phase.is_multiple_of(2) {
        "#2563eb".to_string()
    } else {
        "#dc2626".to_string()
    };
    stats
}

fn parse_event_queue_events(queue_json: &str) -> Vec<JsonValue> {
    serde_json::from_str::<JsonValue>(queue_json)
        .ok()
        .and_then(|doc| doc.get("events").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default()
}

fn fetch_incremental_event_queue(
    port: u16,
    cursor: u64,
    limit: u64,
    ack: bool,
) -> Result<(String, u64, u64, u64, u64, String), String> {
    let path = if ack {
        format!("/state/events?cursor={}&limit={}&ack=1", cursor, limit)
    } else {
        format!("/state/events?cursor={}&limit={}", cursor, limit)
    };
    let body = http_get_path(port, &path)?;
    let parsed: JsonValue = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let next_cursor = parsed
        .get("nextCursor")
        .and_then(|v| v.as_u64())
        .unwrap_or(cursor);
    let events = parsed
        .get("events")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut payload_events: Vec<JsonValue> = Vec::new();
    let mut click_count = 0u64;
    let mut key_count = 0u64;
    let mut last_event = "{}".to_string();
    for item in events {
        if let Some(event) = item.get("event") {
            payload_events.push(event.clone());
            if let Some(t) = event.get("type").and_then(|v| v.as_str()) {
                if t == "click" {
                    click_count += 1;
                } else if t == "key" {
                    key_count += 1;
                }
            }
            last_event = event.to_string();
        }
    }
    let queue = json!({
        "format": "PXL_EVENT_QUEUE",
        "version": 1,
        "count": payload_events.len(),
        "events": payload_events,
    });
    let total_count = payload_events.len() as u64;
    Ok((
        queue.to_string(),
        next_cursor,
        total_count,
        click_count,
        key_count,
        last_event,
    ))
}

fn handle_web_request(mut stream: TcpStream, root: &Path, port: u16) -> Result<(), String> {
    let mut first_line = String::new();
    {
        let mut reader = BufReader::new(&mut stream);
        reader
            .read_line(&mut first_line)
            .map_err(|e| e.to_string())?;
    }
    if first_line.trim().is_empty() {
        return Ok(());
    }

    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let request_path = parts.next().unwrap_or("/");
    if method != "GET" {
        write_http_response(
            &mut stream,
            405,
            "text/plain; charset=utf-8",
            b"Method Not Allowed",
        )?;
        return Ok(());
    }

    let mut request_parts = request_path.splitn(2, '?');
    let rel = request_parts.next().unwrap_or("/");
    let query = request_parts.next().unwrap_or("");
    if rel == "/health" {
        let body = b"{\"ok\":true,\"service\":\"matter-web-server\"}";
        write_http_response(&mut stream, 200, "application/json; charset=utf-8", body)?;
        return Ok(());
    }
    if rel == "/__meta" {
        let body = format!(
            "{{\"ok\":true,\"port\":{},\"root\":\"{}\"}}",
            port,
            json_escape(&root.display().to_string())
        );
        write_http_response(
            &mut stream,
            200,
            "application/json; charset=utf-8",
            body.as_bytes(),
        )?;
        return Ok(());
    }
    if rel == "/state" {
        let queue = web_event_queue();
        let events = queue.lock().map_err(|e| e.to_string())?;
        let uptime_ms = WEB_SERVER_STARTED_AT
            .get()
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let first_event_id = events.first().map(|(id, _)| *id).unwrap_or(0);
        let last_event_id = events.last().map(|(id, _)| *id).unwrap_or(0);
        let first_event = events.first().map(|(_, s)| s.as_str()).unwrap_or("");
        let last_event = events.last().map(|(_, s)| s.as_str()).unwrap_or("");
        let ack_cursor = WEB_EVENT_ACK_CURSOR.load(Ordering::Relaxed);
        let body = format!(
            "{{\"ok\":true,\"service\":\"matter-web-server\",\"port\":{},\"root\":\"{}\",\"uptimeMs\":{},\"events\":{{\"count\":{},\"firstId\":{},\"lastId\":{},\"ackCursor\":{},\"first\":\"{}\",\"last\":\"{}\"}}}}",
            port,
            json_escape(&root.display().to_string()),
            uptime_ms,
            events.len(),
            first_event_id,
            last_event_id,
            ack_cursor,
            json_escape(first_event),
            json_escape(last_event)
        );
        write_http_response(
            &mut stream,
            200,
            "application/json; charset=utf-8",
            body.as_bytes(),
        )?;
        return Ok(());
    }
    if rel == "/state/vm" {
        let vm_state = vm_live_state().lock().map_err(|e| e.to_string())?.clone();
        let body = vm_live_state_response_json(&vm_state);
        write_http_response(
            &mut stream,
            200,
            "application/json; charset=utf-8",
            body.as_bytes(),
        )?;
        return Ok(());
    }
    if rel == "/state/vm/update" {
        let Some(state_json) = query_param(query, "s") else {
            write_http_response(
                &mut stream,
                400,
                "application/json; charset=utf-8",
                b"{\"ok\":false,\"error\":{\"message\":\"missing state snapshot\"}}",
            )?;
            return Ok(());
        };
        let mut vm_state = vm_live_state().lock().map_err(|e| e.to_string())?;
        match apply_vm_live_state_snapshot(&state_json, &mut vm_state) {
            Ok(()) => {
                let body = vm_live_state_response_json(&vm_state);
                write_http_response(
                    &mut stream,
                    200,
                    "application/json; charset=utf-8",
                    body.as_bytes(),
                )?;
            }
            Err(error) => {
                let body = format!(
                    "{{\"ok\":false,\"error\":{{\"message\":\"{}\"}}}}",
                    json_escape(&error)
                );
                write_http_response(
                    &mut stream,
                    400,
                    "application/json; charset=utf-8",
                    body.as_bytes(),
                )?;
            }
        }
        return Ok(());
    }
    if rel == "/state/events" {
        let cursor = query_u64(query, "cursor", 0);
        let limit = query_u64(query, "limit", 50).clamp(1, 1000) as usize;
        let ack = query_param(query, "ack")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let queue = web_event_queue();
        let events = queue.lock().map_err(|e| e.to_string())?;
        let mut selected: Vec<(u64, String)> = Vec::new();
        for (id, event) in events.iter() {
            if *id > cursor {
                selected.push((*id, event.clone()));
                if selected.len() >= limit {
                    break;
                }
            }
        }
        let next_cursor = selected.last().map(|(id, _)| *id).unwrap_or(cursor);
        if ack {
            WEB_EVENT_ACK_CURSOR.store(next_cursor, Ordering::Relaxed);
        }
        let ack_cursor = WEB_EVENT_ACK_CURSOR.load(Ordering::Relaxed);
        let events_json = selected
            .iter()
            .map(|(id, event)| format!("{{\"id\":{},\"event\":{}}}", id, event))
            .collect::<Vec<_>>()
            .join(",");
        let body = format!(
            "{{\"ok\":true,\"cursor\":{},\"nextCursor\":{},\"limit\":{},\"count\":{},\"ack\":{},\"ackCursor\":{},\"events\":[{}]}}",
            cursor,
            next_cursor,
            limit,
            selected.len(),
            ack,
            ack_cursor,
            events_json
        );
        write_http_response(
            &mut stream,
            200,
            "application/json; charset=utf-8",
            body.as_bytes(),
        )?;
        return Ok(());
    }
    if rel == "/actions" {
        if let Some(action_name) = query_param(query, "name") {
            if !is_known_action(&action_name) {
                let body = format!(
                    "{{\"ok\":false,\"accepted\":false,\"action\":\"{}\",\"error\":{{\"message\":\"unknown action\"}},\"registry\":{}}}",
                    json_escape(&action_name),
                    actions_registry_json()
                );
                write_http_response(
                    &mut stream,
                    400,
                    "application/json; charset=utf-8",
                    body.as_bytes(),
                )?;
                return Ok(());
            }
            let event_id = WEB_EVENT_NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let normalized = format!(
                "{{\"type\":\"input\",\"timestamp\":{},\"source\":\"canvas\",\"payload\":{{\"action\":\"{}\"}}}}",
                current_timestamp_ms(),
                json_escape(&action_name)
            );
            let queue = web_event_queue();
            let mut events = queue.lock().map_err(|e| e.to_string())?;
            events.push((event_id, normalized));
            let body = format!(
                "{{\"ok\":true,\"accepted\":true,\"action\":\"{}\",\"count\":{},\"eventId\":{}}}",
                json_escape(&action_name),
                events.len(),
                event_id
            );
            write_http_response(
                &mut stream,
                200,
                "application/json; charset=utf-8",
                body.as_bytes(),
            )?;
            return Ok(());
        }
        write_http_response(
            &mut stream,
            200,
            "application/json; charset=utf-8",
            actions_registry_json().as_bytes(),
        )?;
        return Ok(());
    }
    if rel == "/actions/trigger" {
        write_http_response(
            &mut stream,
            400,
            "application/json; charset=utf-8",
            b"{\"ok\":false,\"error\":{\"message\":\"missing action name\"}}",
        )?;
        return Ok(());
    }
    if rel == "/events" || rel == "/events/queue" {
        if let Some(event_json) = query_param(query, "e") {
            let queue = web_event_queue();
            let mut events = queue.lock().map_err(|e| e.to_string())?;
            let event_id = WEB_EVENT_NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let normalized = normalize_event_json(&event_json);
            events.push((event_id, normalized));
            let body = format!(
                "{{\"ok\":true,\"accepted\":true,\"count\":{}}}",
                events.len()
            );
            write_http_response(
                &mut stream,
                200,
                "application/json; charset=utf-8",
                body.as_bytes(),
            )?;
            return Ok(());
        }
        let queue = web_event_queue();
        let mut events = queue.lock().map_err(|e| e.to_string())?;
        let events_payload = events
            .iter()
            .map(|(_, item)| item.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let clear = query_param(query, "clear")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let body = if rel == "/events/queue" {
            format!(
                "{{\"format\":\"PXL_EVENT_QUEUE\",\"version\":1,\"count\":{},\"events\":[{}]}}",
                events.len(),
                events_payload
            )
        } else {
            format!(
                "{{\"ok\":true,\"count\":{},\"events\":[{}]}}",
                events.len(),
                events_payload
            )
        };
        if clear {
            events.clear();
        }
        write_http_response(
            &mut stream,
            200,
            "application/json; charset=utf-8",
            body.as_bytes(),
        )?;
        return Ok(());
    }
    let rel = rel.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let path = root.join(rel);
    let path = if path.is_dir() {
        path.join("index.html")
    } else {
        path
    };
    if !path.exists() {
        write_http_response(&mut stream, 404, "text/plain; charset=utf-8", b"Not Found")?;
        return Ok(());
    }

    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let ctype = content_type_for(path.extension().and_then(|ext| ext.to_str()).unwrap_or(""));
    write_http_response(&mut stream, 200, ctype, &bytes)?;
    Ok(())
}

fn write_http_response(
    stream: &mut TcpStream,
    code: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let status = match code {
        200 => "200 OK",
        400 => "400 Bad Request",
        404 => "404 Not Found",
        405 => "405 Method Not Allowed",
        _ => "500 Internal Server Error",
    };
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        content_type,
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(body).map_err(|e| e.to_string())?;
    Ok(())
}

fn content_type_for(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
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

    let compiled = compile_visual_app_dsl_if_present(&resolved);
    (compiled, entry_label)
}

struct VisualAppDsl {
    app_name: String,
    screen_name: String,
    counters: Vec<String>,
    buttons: Vec<String>,
    canvas_click: bool,
}

fn compile_visual_app_dsl_if_present(source: &str) -> String {
    let trimmed = source.trim_start();
    if !trimmed.starts_with("app ") {
        return source.to_string();
    }
    match parse_visual_app_dsl(source) {
        Some(app) => render_visual_app_dsl_matter(&app),
        None => source.to_string(),
    }
}

fn parse_visual_app_dsl(source: &str) -> Option<VisualAppDsl> {
    let mut app_name = None;
    let mut screen_name = "main".to_string();
    let mut counters = Vec::new();
    let mut buttons = Vec::new();
    let mut canvas_click = false;

    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("app ") {
            app_name = quoted_value(line);
        } else if line.starts_with("screen ") {
            if let Some(value) = quoted_value(line) {
                screen_name = value;
            }
        } else if line.starts_with("counter ") {
            if let Some(value) = quoted_value(line) {
                counters.push(value);
            }
        } else if line.starts_with("button ") {
            if let Some(value) = quoted_value(line) {
                buttons.push(value);
            }
        } else if line.starts_with("canvas on click") {
            canvas_click = true;
        }
    }

    Some(VisualAppDsl {
        app_name: app_name?,
        screen_name,
        counters,
        buttons,
        canvas_click,
    })
}

fn quoted_value(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let rest = &line[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn visual_region_id(kind: &str, name: &str) -> String {
    let mut out = String::new();
    out.push_str(kind);
    out.push('_');
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_end_matches('_').to_string()
}

fn matter_string(value: &str) -> String {
    format!("\"{}\"", json_escape(value))
}

fn render_visual_app_dsl_matter(app: &VisualAppDsl) -> String {
    let mut out = String::new();
    out.push_str("# Generated from Matter visual app DSL\n");
    out.push_str(&format!("visual.run({})\n", matter_string(&app.app_name)));
    out.push_str("visual.surface(\"main\", 720, 520)\n");
    out.push_str(&format!(
        "visual.scene({})\n",
        matter_string(&app.screen_name)
    ));
    out.push_str("visual.editor(false)\n");
    out.push_str("visual.theme(\"page\", \"#eef2f7\")\n");
    out.push_str("visual.theme(\"surface\", \"#f8fafc\")\n");
    out.push_str("visual.theme(\"accent\", \"#2563eb\")\n");
    out.push_str("visual.theme(\"selected\", \"#f59e0b\")\n");
    out.push_str("visual.theme(\"regionFill\", \"rgba(37,99,235,.18)\")\n");
    out.push_str("visual.theme(\"activeFill\", \"rgba(5,150,105,.22)\")\n");

    out.push_str("visual.region(\"app_title\", 24, 22, 672, 52)\n");
    out.push_str(&format!(
        "visual.text(\"app_title\", {})\n",
        matter_string(&app.app_name)
    ));
    out.push_str("visual.set(\"app_title\", \"semantic\", \"app_title\")\n");
    out.push_str("visual.layer(\"app_title\", 1)\n");

    out.push_str("visual.region(\"canvas\", 24, 92, 672, 218)\n");
    out.push_str("visual.text(\"canvas\", \"Clique no canvas\")\n");
    out.push_str("visual.set(\"canvas\", \"semantic\", \"interactive_canvas\")\n");
    out.push_str("visual.set(\"canvas\", \"event\", \"canvas_click\")\n");
    out.push_str("visual.state(\"canvas\", \"active\")\n");
    out.push_str("visual.layer(\"canvas\", 2)\n");
    if app.canvas_click {
        out.push_str("visual.pulse(\"canvas\")\n");
    }

    let mut x = 24;
    for counter in &app.counters {
        let id = visual_region_id("counter", counter);
        out.push_str(&format!(
            "visual.region({}, {}, 330, 210, 58)\n",
            matter_string(&id),
            x
        ));
        out.push_str(&format!(
            "visual.text({}, {})\n",
            matter_string(&id),
            matter_string(&format!("{}: 0", counter))
        ));
        out.push_str(&format!(
            "visual.set({}, \"semantic\", \"counter\")\n",
            matter_string(&id)
        ));
        out.push_str(&format!("visual.layer({}, 3)\n", matter_string(&id)));
        x += 230;
    }

    let mut button_y = 412;
    for button in &app.buttons {
        let id = visual_region_id("button", button);
        out.push_str(&format!(
            "visual.region({}, 24, {}, 320, 48)\n",
            matter_string(&id),
            button_y
        ));
        out.push_str(&format!(
            "visual.text({}, {})\n",
            matter_string(&id),
            matter_string(button)
        ));
        out.push_str(&format!(
            "visual.set({}, \"event\", {})\n",
            matter_string(&id),
            matter_string(button)
        ));
        out.push_str(&format!(
            "visual.state({}, \"active\")\n",
            matter_string(&id)
        ));
        out.push_str(&format!("visual.layer({}, 4)\n", matter_string(&id)));
        button_y += 58;
    }

    out
}

fn apply_project_env(project: &ProjectContext) -> Vec<EnvSnapshot> {
    let mut snapshots = Vec::new();

    if !project.manifest.stdlib.is_empty() {
        snapshots.push(set_env_snapshot(
            "MATTER_STDLIB_PATH",
            project_path(&project.base_dir, &project.manifest.stdlib)
                .display()
                .to_string(),
        ));
    }

    if !project.manifest.store.is_empty() {
        snapshots.push(set_env_snapshot(
            "MATTER_STORE_PATH",
            project_path(&project.base_dir, &project.manifest.store)
                .display()
                .to_string(),
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

    Some(
        value[1..value.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\"),
    )
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
    run_source_json(source, "<eval>", false);
}

fn run_source(source: &str) {
    let mut parser = Parser::from_source(source);
    let program = parser.parse().unwrap_or_else(|e| {
        print_parse_error(source, &e);
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

fn run_json(path: &str, with_energy: bool) {
    let source = read_source_or_exit(path);
    run_source_json(&source, source_label(path), with_energy);
}

fn emnr_run_json(path: &str) {
    let mut session = MatterBridgeSession::new();
    let (payload, ok) = emnr_run_one_json(path, &mut session);
    println!(
        "{}",
        serde_json::to_string(&payload).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            )
        })
    );
    if !ok {
        process::exit(1);
    }
}

struct EmnrStreamReport {
    report: JsonValue,
    failed_count: u64,
    isolate_count: u64,
    throttle_count: u64,
    observe_count: u64,
    mean_risk: f64,
    mean_reward: f64,
    mean_confidence: f64,
    mean_prediction_error: f64,
    final_energy: f32,
    memory_records: usize,
}

#[derive(Clone, Copy)]
struct EmnrGatePolicy {
    warn_as_fail: bool,
    risk_threshold: f64,
    confidence_threshold: f64,
    energy_threshold: f32,
    critical_energy: f32,
}

struct EmnrGateOptions {
    paths: Vec<String>,
    policy: EmnrGatePolicy,
    output: Option<String>,
    state_input: Option<String>,
    state_output: Option<String>,
}

struct EmnrTrainOptions {
    paths: Vec<String>,
    epochs: usize,
    output: Option<String>,
    state_input: Option<String>,
    state_output: Option<String>,
}

struct EmnrPromoteOptions {
    paths: Vec<String>,
    baseline_state: String,
    candidate_state: String,
    output: Option<String>,
    policy: EmnrPromotePolicy,
}

#[derive(Clone, Copy)]
struct EmnrPromotePolicy {
    max_risk_regression: f64,
    max_error_regression: f64,
    max_confidence_drop: f64,
    min_energy: f32,
}

struct EmnrAuditOptions {
    paths: Vec<String>,
    output: Option<String>,
    fail_on_warn: bool,
}

struct EmnrPipelineOptions {
    paths: Vec<String>,
    epochs: usize,
    out_dir: String,
    fail_on_warn: bool,
    gate_policy: EmnrGatePolicy,
    promote_policy: EmnrPromotePolicy,
    policy_path: Option<String>,
}

struct EmnrCiOptions {
    pipeline: EmnrPipelineOptions,
    baseline_manifest: Option<String>,
    baseline_index: Option<String>,
    require_non_regression: bool,
    require_pass: bool,
    output: Option<String>,
}

struct EmnrBaselinePromoteOptions {
    manifest: String,
    baseline_output: String,
    require_pass: bool,
    index_output: Option<String>,
    output: Option<String>,
}

impl Default for EmnrGatePolicy {
    fn default() -> Self {
        Self {
            warn_as_fail: false,
            risk_threshold: 0.40,
            confidence_threshold: 0.55,
            energy_threshold: 25.0,
            critical_energy: 10.0,
        }
    }
}

impl Default for EmnrPromotePolicy {
    fn default() -> Self {
        Self {
            max_risk_regression: 0.02,
            max_error_regression: 0.02,
            max_confidence_drop: 0.02,
            min_energy: 20.0,
        }
    }
}

fn emnr_run_stream_json(paths: &[String]) {
    let stream = collect_emnr_stream(paths);
    println!(
        "{}",
        serde_json::to_string(&stream.report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if stream.failed_count > 0 {
        process::exit(1);
    }
}

fn emnr_gate_json(options: &EmnrGateOptions) {
    let mut session = match options.state_input.as_deref() {
        Some(path) => load_emnr_gate_state(path),
        None => MatterBridgeSession::new(),
    };
    let stream = collect_emnr_stream_with_session(&options.paths, &mut session);
    if let Some(path) = options.state_output.as_deref() {
        save_emnr_gate_state(path, &session);
    }
    let policy = options.policy;
    let status = emnr_gate_status(&stream, &policy);
    let exit_code = if status == "fail" || (status == "warn" && policy.warn_as_fail) {
        1
    } else {
        0
    };
    let reason = emnr_gate_reason(&stream, status, &policy);
    let report = json!({
        "ok": exit_code == 0,
        "status": status,
        "exit_code": exit_code,
        "policy": {
            "warn_as_fail": policy.warn_as_fail,
            "risk_threshold": policy.risk_threshold,
            "confidence_threshold": policy.confidence_threshold,
            "energy_threshold": policy.energy_threshold,
            "critical_energy": policy.critical_energy,
        },
        "reason": reason,
        "emnr_stream": stream.report,
    });

    let report_text = serde_json::to_string(&report).unwrap_or_else(|error| {
        format!(
            "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&error.to_string())
        )
    });
    if let Some(path) = options.output.as_deref() {
        if let Err(error) = fs::write(path, &report_text) {
            eprintln!("Error writing EMNR gate report '{}': {}", path, error);
            process::exit(1);
        }
    }
    println!("{report_text}");
    if exit_code != 0 {
        process::exit(exit_code);
    }
}

fn emnr_train_json(options: &EmnrTrainOptions) {
    let mut session = match options.state_input.as_deref() {
        Some(path) => load_emnr_gate_state(path),
        None => MatterBridgeSession::new(),
    };
    let report = emnr_train_report_for_session(
        &options.paths,
        options.epochs,
        &mut session,
        options.state_input.as_deref(),
        options.state_output.as_deref(),
    );
    if let Some(path) = options.state_output.as_deref() {
        save_emnr_gate_state(path, &session);
    }

    let report_text = serde_json::to_string(&report).unwrap_or_else(|error| {
        format!(
            "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&error.to_string())
        )
    });
    if let Some(path) = options.output.as_deref() {
        if let Err(error) = fs::write(path, &report_text) {
            eprintln!("Error writing EMNR train report '{}': {}", path, error);
            process::exit(1);
        }
    }
    println!("{report_text}");
    if report["failed_count"].as_u64().unwrap_or(1) > 0 {
        process::exit(1);
    }
}

fn emnr_train_report_for_session(
    paths: &[String],
    epochs: usize,
    session: &mut MatterBridgeSession,
    state_input: Option<&str>,
    state_output: Option<&str>,
) -> JsonValue {
    let mut epoch_reports = Vec::new();
    let mut failed_count = 0u64;
    let started_tick = session.runtime.tick;
    let started_energy = session.runtime.energy.current;
    let started_memory_records = session.runtime.memory.records.len();

    for epoch in 0..epochs.max(1) {
        let stream = collect_emnr_stream_with_session(paths, session);
        failed_count += stream.failed_count;
        epoch_reports.push(json!({
            "epoch": epoch + 1,
            "files": paths.len(),
            "failed_count": stream.failed_count,
            "mean_confidence": stream.mean_confidence,
            "mean_risk": stream.mean_risk,
            "mean_reward": stream.mean_reward,
            "mean_prediction_error": stream.mean_prediction_error,
            "final_energy": stream.final_energy,
            "memory_records": stream.memory_records,
            "decision_counts": stream.report["summary"]["decision_counts"].clone(),
        }));
    }

    let first_epoch = epoch_reports.first().cloned().unwrap_or_else(|| json!({}));
    let last_epoch = epoch_reports.last().cloned().unwrap_or_else(|| json!({}));
    json!({
        "ok": failed_count == 0,
        "status": if failed_count == 0 { "trained" } else { "partial" },
        "files": paths.len(),
        "epochs": epoch_reports.len(),
        "failed_count": failed_count,
        "state_input": state_input,
        "state_output": state_output,
        "started": {
            "tick": started_tick,
            "energy": started_energy,
            "memory_records": started_memory_records,
        },
        "finished": {
            "tick": session.runtime.tick,
            "energy": session.runtime.energy.current,
            "memory_records": session.runtime.memory.records.len(),
            "field": session.runtime.field,
        },
        "learning_delta": {
            "confidence": last_epoch["mean_confidence"].as_f64().unwrap_or_default()
                - first_epoch["mean_confidence"].as_f64().unwrap_or_default(),
            "risk": last_epoch["mean_risk"].as_f64().unwrap_or_default()
                - first_epoch["mean_risk"].as_f64().unwrap_or_default(),
            "prediction_error": last_epoch["mean_prediction_error"].as_f64().unwrap_or_default()
                - first_epoch["mean_prediction_error"].as_f64().unwrap_or_default(),
            "energy": session.runtime.energy.current - started_energy,
            "memory_records": session.runtime.memory.records.len() as i64
                - started_memory_records as i64,
        },
        "epoch_reports": epoch_reports,
    })
}

fn emnr_promote_json(options: &EmnrPromoteOptions) {
    let report = emnr_promote_report(options);
    let report_text = serde_json::to_string(&report).unwrap_or_else(|error| {
        format!(
            "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&error.to_string())
        )
    });
    if let Some(path) = options.output.as_deref() {
        if let Err(error) = fs::write(path, &report_text) {
            eprintln!("Error writing EMNR promote report '{}': {}", path, error);
            process::exit(1);
        }
    }
    println!("{report_text}");
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_promote_report(options: &EmnrPromoteOptions) -> JsonValue {
    let mut baseline = load_emnr_gate_state(&options.baseline_state);
    let mut candidate = load_emnr_gate_state(&options.candidate_state);
    let baseline_stream = collect_emnr_stream_with_session(&options.paths, &mut baseline);
    let candidate_stream = collect_emnr_stream_with_session(&options.paths, &mut candidate);

    let risk_delta = candidate_stream.mean_risk - baseline_stream.mean_risk;
    let error_delta =
        candidate_stream.mean_prediction_error - baseline_stream.mean_prediction_error;
    let confidence_delta = candidate_stream.mean_confidence - baseline_stream.mean_confidence;
    let energy_delta = candidate_stream.final_energy - baseline_stream.final_energy;
    let failures = candidate_stream.failed_count + baseline_stream.failed_count;
    let status = emnr_promote_status(
        &baseline_stream,
        &candidate_stream,
        options,
        risk_delta,
        error_delta,
        confidence_delta,
    );
    let exit_code = if status == "promote" { 0 } else { 1 };
    json!({
        "ok": exit_code == 0,
        "status": status,
        "exit_code": exit_code,
        "files": options.paths.len(),
        "baseline_state": options.baseline_state,
        "candidate_state": options.candidate_state,
        "policy": {
            "max_risk_regression": options.policy.max_risk_regression,
            "max_error_regression": options.policy.max_error_regression,
            "max_confidence_drop": options.policy.max_confidence_drop,
            "min_energy": options.policy.min_energy,
        },
        "metrics": {
            "baseline": emnr_stream_metrics_json(&baseline_stream),
            "candidate": emnr_stream_metrics_json(&candidate_stream),
            "delta": {
                "risk": risk_delta,
                "prediction_error": error_delta,
                "confidence": confidence_delta,
                "energy": energy_delta,
                "memory_records": candidate_stream.memory_records as i64
                    - baseline_stream.memory_records as i64,
            }
        },
        "reason": emnr_promote_reason(
            failures,
            risk_delta,
            error_delta,
            confidence_delta,
            candidate_stream.final_energy,
            options,
        ),
    })
}

fn emnr_audit_json(options: &EmnrAuditOptions) {
    let mut reports = Vec::new();
    for path in &options.paths {
        let raw = fs::read_to_string(path).unwrap_or_else(|error| {
            eprintln!("Error reading EMNR report '{}': {}", path, error);
            process::exit(1);
        });
        let value = serde_json::from_str::<JsonValue>(&raw).unwrap_or_else(|error| {
            eprintln!("Error parsing EMNR report '{}': {}", path, error);
            process::exit(1);
        });
        reports.push((path.clone(), value));
    }

    let report = emnr_audit_report(&reports, options.fail_on_warn);
    let report_text = serde_json::to_string(&report).unwrap_or_else(|error| {
        format!(
            "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&error.to_string())
        )
    });
    if let Some(path) = options.output.as_deref() {
        if let Err(error) = fs::write(path, &report_text) {
            eprintln!("Error writing EMNR audit report '{}': {}", path, error);
            process::exit(1);
        }
    }
    println!("{report_text}");
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_pipeline_json(options: &EmnrPipelineOptions) {
    let report = emnr_pipeline_report(options);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_pipeline_report(options: &EmnrPipelineOptions) -> JsonValue {
    let out_dir = PathBuf::from(&options.out_dir);
    if let Err(error) = fs::create_dir_all(&out_dir) {
        eprintln!(
            "Error creating EMNR pipeline output directory '{}': {}",
            options.out_dir, error
        );
        process::exit(1);
    }

    let baseline_state = out_dir.join("baseline-state.json");
    let candidate_state = out_dir.join("candidate-state.json");
    let gate_report_path = out_dir.join("gate-report.json");
    let train_report_path = out_dir.join("train-report.json");
    let promote_report_path = out_dir.join("promote-report.json");
    let audit_report_path = out_dir.join("audit-report.json");
    let manifest_path = out_dir.join("manifest.json");

    let mut baseline_session = MatterBridgeSession::new();
    let gate_stream = collect_emnr_stream_with_session(&options.paths, &mut baseline_session);
    save_emnr_gate_state_path(&baseline_state, &baseline_session);
    let gate_policy = options.gate_policy;
    let gate_status = emnr_gate_status(&gate_stream, &gate_policy);
    let gate_exit_code = if gate_status == "fail" { 1 } else { 0 };
    let gate_report = json!({
        "ok": gate_exit_code == 0,
        "status": gate_status,
        "exit_code": gate_exit_code,
        "policy": {
            "warn_as_fail": gate_policy.warn_as_fail,
            "risk_threshold": gate_policy.risk_threshold,
            "confidence_threshold": gate_policy.confidence_threshold,
            "energy_threshold": gate_policy.energy_threshold,
            "critical_energy": gate_policy.critical_energy,
        },
        "reason": emnr_gate_reason(&gate_stream, gate_status, &gate_policy),
        "emnr_stream": gate_stream.report,
    });
    write_json_report_path(&gate_report_path, &gate_report, "EMNR pipeline gate report");

    let mut candidate_session = baseline_session.clone();
    let train_report = emnr_train_report_for_session(
        &options.paths,
        options.epochs,
        &mut candidate_session,
        Some(&baseline_state.to_string_lossy()),
        Some(&candidate_state.to_string_lossy()),
    );
    save_emnr_gate_state_path(&candidate_state, &candidate_session);
    write_json_report_path(
        &train_report_path,
        &train_report,
        "EMNR pipeline train report",
    );

    let promote_options = EmnrPromoteOptions {
        paths: options.paths.clone(),
        baseline_state: baseline_state.to_string_lossy().to_string(),
        candidate_state: candidate_state.to_string_lossy().to_string(),
        output: None,
        policy: options.promote_policy,
    };
    let promote_report = emnr_promote_report(&promote_options);
    write_json_report_path(
        &promote_report_path,
        &promote_report,
        "EMNR pipeline promote report",
    );

    let audit_inputs = vec![
        (
            gate_report_path.to_string_lossy().to_string(),
            gate_report.clone(),
        ),
        (
            train_report_path.to_string_lossy().to_string(),
            train_report.clone(),
        ),
        (
            promote_report_path.to_string_lossy().to_string(),
            promote_report.clone(),
        ),
    ];
    let audit_report = emnr_audit_report(&audit_inputs, options.fail_on_warn);
    write_json_report_path(
        &audit_report_path,
        &audit_report,
        "EMNR pipeline audit report",
    );

    let exit_code = audit_report["exit_code"].as_i64().unwrap_or(1);
    let artifacts = emnr_pipeline_artifacts_json(
        &baseline_state,
        &candidate_state,
        &gate_report_path,
        &train_report_path,
        &promote_report_path,
        &audit_report_path,
        &manifest_path,
    );
    let policy_json = emnr_pipeline_policy_json(gate_policy, options.promote_policy);
    let manifest = emnr_pipeline_manifest_json(
        options,
        &artifacts,
        &policy_json,
        &gate_report,
        &train_report,
        &promote_report,
        &audit_report,
    );
    write_json_report_path(&manifest_path, &manifest, "EMNR pipeline manifest");

    json!({
        "ok": exit_code == 0,
        "status": audit_report["status"].clone(),
        "exit_code": exit_code,
        "files": options.paths.len(),
        "epochs": options.epochs,
        "out_dir": options.out_dir,
        "policy_path": options.policy_path,
        "policy": policy_json,
        "artifacts": artifacts,
        "manifest": manifest,
        "gate": {
            "status": gate_report["status"].clone(),
            "reason": gate_report["reason"].clone(),
        },
        "train": {
            "status": train_report["status"].clone(),
            "learning_delta": train_report["learning_delta"].clone(),
        },
        "promote": {
            "status": promote_report["status"].clone(),
            "reason": promote_report["reason"].clone(),
        },
        "audit": audit_report,
    })
}

fn emnr_ci_json(options: &EmnrCiOptions) {
    let report = emnr_ci_report(options);
    let report_text = serde_json::to_string(&report).unwrap_or_else(|error| {
        format!(
            "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&error.to_string())
        )
    });
    if let Some(path) = options.output.as_deref() {
        if let Err(error) = fs::write(path, &report_text) {
            eprintln!("Error writing EMNR CI report '{}': {}", path, error);
            process::exit(1);
        }
    }
    println!("{report_text}");
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_ci_report(options: &EmnrCiOptions) -> JsonValue {
    let pipeline = emnr_pipeline_report(&options.pipeline);
    let manifest = pipeline["manifest"].clone();
    let manifest_path = pipeline["artifacts"]["manifest"]
        .as_str()
        .unwrap_or("manifest.json")
        .to_string();
    let verify = emnr_verify_manifest_report(&manifest_path, &manifest, options.require_pass);
    let resolved_baseline_manifest = emnr_ci_resolved_baseline_manifest(options);
    let compare = resolved_baseline_manifest.as_deref().map(|baseline_path| {
        let baseline = load_json_file_or_exit(baseline_path, "EMNR CI baseline manifest");
        emnr_compare_manifests_report(
            baseline_path,
            &manifest_path,
            &baseline,
            &manifest,
            options.require_non_regression,
        )
    });
    let compare_exit = compare
        .as_ref()
        .and_then(|report| report["exit_code"].as_i64())
        .unwrap_or(0);
    let pipeline_exit = pipeline["exit_code"].as_i64().unwrap_or(1);
    let verify_exit = verify["exit_code"].as_i64().unwrap_or(1);
    let exit_code = if pipeline_exit != 0 || verify_exit != 0 || compare_exit != 0 {
        1
    } else {
        0
    };
    json!({
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "files": options.pipeline.paths.len(),
        "epochs": options.pipeline.epochs,
        "out_dir": options.pipeline.out_dir,
        "baseline_manifest": resolved_baseline_manifest,
        "baseline_index": options.baseline_index,
        "require_pass": options.require_pass,
        "require_non_regression": options.require_non_regression,
        "pipeline": {
            "status": pipeline["status"].clone(),
            "manifest": manifest_path,
            "health_score": pipeline["audit"]["health_score"].clone(),
        },
        "verify": verify,
        "compare": compare.unwrap_or(JsonValue::Null),
    })
}

fn emnr_ci_resolved_baseline_manifest(options: &EmnrCiOptions) -> Option<String> {
    if options.baseline_manifest.is_some() {
        options.baseline_manifest.clone()
    } else {
        options.baseline_index.as_deref().and_then(|index_path| {
            let index = load_json_file_or_exit(index_path, "EMNR CI baseline index");
            index["current"]["manifest"].as_str().map(str::to_string)
        })
    }
}

fn emnr_ci_scorecard_json(path: &str, output: Option<&str>, require_pass: bool) {
    let report = load_json_file_or_exit(path, "EMNR CI report");
    let scorecard = emnr_ci_scorecard_report(path, &report, require_pass);
    let scorecard_text = serde_json::to_string(&scorecard).unwrap_or_else(|error| {
        format!(
            "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&error.to_string())
        )
    });
    if let Some(path) = output {
        if let Err(error) = fs::write(path, &scorecard_text) {
            eprintln!("Error writing EMNR CI scorecard '{}': {}", path, error);
            process::exit(1);
        }
    }
    println!("{scorecard_text}");
    if scorecard["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_ci_scorecard_report(path: &str, report: &JsonValue, require_pass: bool) -> JsonValue {
    let ci_status = report["status"].as_str().unwrap_or("unknown");
    let pipeline_status = report["pipeline"]["status"].as_str().unwrap_or("unknown");
    let verify_status = report["verify"]["status"].as_str().unwrap_or("unknown");
    let compare_status = report["compare"]["status"].as_str().unwrap_or("none");
    let compare_verdict = report["compare"]["verdict"].as_str().unwrap_or("none");
    let health_score = report["pipeline"]["health_score"].as_f64().unwrap_or(0.0);
    let verify_fail_count = report["verify"]["fail_count"].as_u64().unwrap_or(0);
    let verify_warn_count = report["verify"]["warn_count"].as_u64().unwrap_or(0);
    let compare_fail_count = report["compare"]["fail_count"].as_u64().unwrap_or(0);
    let ci_exit = report["exit_code"].as_i64().unwrap_or(1);

    let decision = if ci_exit != 0
        || ci_status == "fail"
        || verify_fail_count > 0
        || compare_fail_count > 0
        || compare_verdict == "regressed"
    {
        "block"
    } else if verify_warn_count > 0 || health_score < 90.0 || compare_status == "none" {
        "review"
    } else {
        "approve"
    };
    let exit_code = if require_pass && decision != "approve" {
        1
    } else {
        0
    };
    json!({
        "schema": "matter.emnr.ci.scorecard.v1",
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "input": path,
        "require_pass": require_pass,
        "decision": decision,
        "health_score": health_score,
        "baseline": {
            "manifest": report.get("baseline_manifest").cloned().unwrap_or(JsonValue::Null),
            "index": report.get("baseline_index").cloned().unwrap_or(JsonValue::Null),
        },
        "pipeline": {
            "status": pipeline_status,
            "manifest": report["pipeline"]["manifest"].clone(),
        },
        "verify": {
            "status": verify_status,
            "fail_count": verify_fail_count,
            "warn_count": verify_warn_count,
        },
        "compare": {
            "status": compare_status,
            "verdict": compare_verdict,
            "fail_count": compare_fail_count,
            "deltas": report["compare"]["deltas"].clone(),
        },
        "signals": {
            "learning_delta": report["verify"]["summary"]["learning_delta"].clone(),
            "promotion_delta": report["verify"]["summary"]["promotion_delta"].clone(),
        },
    })
}

fn emnr_ci_history_json(paths: &[String], output: Option<&str>, require_stable: bool) {
    let scorecards = paths
        .iter()
        .map(|path| {
            (
                path.clone(),
                load_json_file_or_exit(path, "EMNR CI scorecard"),
            )
        })
        .collect::<Vec<_>>();
    let history = emnr_ci_history_report(&scorecards, require_stable);
    let history_text = serde_json::to_string(&history).unwrap_or_else(|error| {
        format!(
            "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&error.to_string())
        )
    });
    if let Some(path) = output {
        if let Err(error) = fs::write(path, &history_text) {
            eprintln!("Error writing EMNR CI history '{}': {}", path, error);
            process::exit(1);
        }
    }
    println!("{history_text}");
    if history["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_ci_history_report(scorecards: &[(String, JsonValue)], require_stable: bool) -> JsonValue {
    let mut approve_count = 0u64;
    let mut review_count = 0u64;
    let mut block_count = 0u64;
    let mut invalid_count = 0u64;
    let mut entries = Vec::new();
    let mut health_values = Vec::new();

    for (path, scorecard) in scorecards {
        let schema_ok = scorecard.get("schema").and_then(JsonValue::as_str)
            == Some("matter.emnr.ci.scorecard.v1");
        if !schema_ok {
            invalid_count += 1;
        }
        let decision = scorecard["decision"].as_str().unwrap_or("unknown");
        match decision {
            "approve" => approve_count += 1,
            "review" => review_count += 1,
            "block" => block_count += 1,
            _ => invalid_count += 1,
        }
        let health = scorecard["health_score"].as_f64().unwrap_or(0.0);
        health_values.push(health);
        entries.push(json!({
            "path": path,
            "schema_ok": schema_ok,
            "decision": decision,
            "status": scorecard["status"].clone(),
            "health_score": health,
            "compare_verdict": scorecard["compare"]["verdict"].clone(),
        }));
    }

    let first_health = health_values.first().copied().unwrap_or(0.0);
    let latest_health = health_values.last().copied().unwrap_or(0.0);
    let health_delta = latest_health - first_health;
    let latest_decision = entries
        .last()
        .and_then(|entry| entry["decision"].as_str())
        .unwrap_or("unknown");
    let trend = if block_count > 0 {
        "blocked"
    } else if health_delta < -EMNR_MANIFEST_COMPARE_EPSILON {
        "degrading"
    } else if health_delta > EMNR_MANIFEST_COMPARE_EPSILON {
        "improving"
    } else {
        "stable"
    };
    let stable = invalid_count == 0
        && block_count == 0
        && latest_decision == "approve"
        && trend != "degrading";
    let exit_code = if require_stable && !stable { 1 } else { 0 };

    json!({
        "schema": "matter.emnr.ci.history.v1",
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "require_stable": require_stable,
        "stable": stable,
        "trend": trend,
        "latest_decision": latest_decision,
        "count": scorecards.len(),
        "summary": {
            "approve": approve_count,
            "review": review_count,
            "block": block_count,
            "invalid": invalid_count,
        },
        "health": {
            "first": first_health,
            "latest": latest_health,
            "delta": health_delta,
        },
        "entries": entries,
    })
}

fn emnr_ci_bundle_json(
    ci_report_path: &str,
    out_dir: &str,
    require_pass: bool,
    require_stable: bool,
) {
    let ci_report = load_json_file_or_exit(ci_report_path, "EMNR CI report");
    let bundle = emnr_ci_bundle_report(
        ci_report_path,
        out_dir,
        &ci_report,
        require_pass,
        require_stable,
    );
    println!(
        "{}",
        serde_json::to_string(&bundle).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if bundle["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_ci_bundle_report(
    ci_report_path: &str,
    out_dir: &str,
    ci_report: &JsonValue,
    require_pass: bool,
    require_stable: bool,
) -> JsonValue {
    let out_dir_path = Path::new(out_dir);
    if let Err(error) = fs::create_dir_all(out_dir_path) {
        eprintln!(
            "Error creating EMNR CI bundle directory '{}': {}",
            out_dir, error
        );
        process::exit(1);
    }

    let ci_report_out = out_dir_path.join("ci-report.json");
    let scorecard_path = out_dir_path.join("scorecard.json");
    let history_path = out_dir_path.join("history.json");
    let bundle_path = out_dir_path.join("bundle.json");
    let scorecard = emnr_ci_scorecard_report(ci_report_path, ci_report, require_pass);
    let scorecard_entries = vec![(scorecard_path.display().to_string(), scorecard.clone())];
    let history = emnr_ci_history_report(&scorecard_entries, require_stable);
    let exit_code = if scorecard["exit_code"].as_i64().unwrap_or(1) != 0
        || history["exit_code"].as_i64().unwrap_or(1) != 0
    {
        1
    } else {
        0
    };
    let bundle = json!({
        "schema": "matter.emnr.ci.bundle.v1",
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "input": ci_report_path,
        "out_dir": out_dir,
        "require_pass": require_pass,
        "require_stable": require_stable,
        "decision": scorecard["decision"].clone(),
        "stable": history["stable"].clone(),
        "trend": history["trend"].clone(),
        "artifacts": {
            "ci_report": ci_report_out,
            "scorecard": scorecard_path,
            "history": history_path,
            "bundle": bundle_path,
        },
        "scorecard": scorecard,
        "history": history,
    });

    write_json_report_path(&ci_report_out, ci_report, "EMNR CI bundle report");
    write_json_report_path(
        &scorecard_path,
        &bundle["scorecard"],
        "EMNR CI bundle scorecard",
    );
    write_json_report_path(&history_path, &bundle["history"], "EMNR CI bundle history");
    write_json_report_path(&bundle_path, &bundle, "EMNR CI bundle manifest");
    bundle
}

fn emnr_verify_ci_bundle_json(path: &str, require_pass: bool) {
    let bundle = load_json_file_or_exit(path, "EMNR CI bundle");
    let report = emnr_verify_ci_bundle_report(path, &bundle, require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_verify_ci_bundle_report(path: &str, bundle: &JsonValue, require_pass: bool) -> JsonValue {
    let mut checks = Vec::new();
    let mut fail_count = 0u64;
    let schema_ok =
        bundle.get("schema").and_then(JsonValue::as_str) == Some("matter.emnr.ci.bundle.v1");
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "schema",
        schema_ok,
        "fail",
        "bundle schema must be matter.emnr.ci.bundle.v1",
    );

    let artifact_names = ["ci_report", "scorecard", "history", "bundle"];
    for artifact_name in artifact_names {
        let artifact_path = bundle["artifacts"][artifact_name]
            .as_str()
            .unwrap_or_default();
        push_manifest_check(
            &mut checks,
            &mut fail_count,
            &format!("artifact_{artifact_name}"),
            !artifact_path.is_empty() && Path::new(artifact_path).exists(),
            "fail",
            "referenced EMNR CI bundle artifact must exist",
        );
    }

    let decision = bundle["decision"].as_str().unwrap_or("unknown");
    let scorecard_decision = bundle["scorecard"]["decision"]
        .as_str()
        .unwrap_or("unknown");
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "decision_matches_scorecard",
        decision == scorecard_decision,
        "fail",
        "bundle decision must match embedded scorecard decision",
    );

    let stable = bundle["stable"].as_bool().unwrap_or(false);
    let history_stable = bundle["history"]["stable"].as_bool().unwrap_or(false);
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "stable_matches_history",
        stable == history_stable,
        "fail",
        "bundle stable flag must match embedded history stable flag",
    );

    let trend = bundle["trend"].as_str().unwrap_or("unknown");
    let history_trend = bundle["history"]["trend"].as_str().unwrap_or("unknown");
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "trend_matches_history",
        trend == history_trend,
        "fail",
        "bundle trend must match embedded history trend",
    );

    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "required_pass",
        !require_pass || (decision == "approve" && stable),
        "fail",
        "bundle must be approved and stable when --require-pass is set",
    );

    let exit_code = if fail_count > 0 { 1 } else { 0 };
    json!({
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "input": path,
        "require_pass": require_pass,
        "decision": decision,
        "stable": stable,
        "trend": trend,
        "checks": checks,
        "fail_count": fail_count,
        "artifacts": bundle.get("artifacts").cloned().unwrap_or(JsonValue::Null),
    })
}

fn emnr_ci_bundle_summary_md(bundle_path: &str, output: &str) {
    let bundle = load_json_file_or_exit(bundle_path, "EMNR CI bundle");
    let summary = emnr_ci_bundle_summary_markdown(bundle_path, &bundle);
    if let Err(error) = fs::write(output, &summary) {
        eprintln!(
            "Error writing EMNR CI bundle summary '{}': {}",
            output, error
        );
        process::exit(1);
    }
    println!("{}", summary);
}

fn emnr_ci_bundle_summary_markdown(bundle_path: &str, bundle: &JsonValue) -> String {
    let decision = bundle["decision"].as_str().unwrap_or("unknown");
    let status = bundle["status"].as_str().unwrap_or("unknown");
    let stable = bundle["stable"].as_bool().unwrap_or(false);
    let trend = bundle["trend"].as_str().unwrap_or("unknown");
    let health_score = bundle["scorecard"]["health_score"].as_f64().unwrap_or(0.0);
    let verify_fail_count = bundle["scorecard"]["verify"]["fail_count"]
        .as_u64()
        .unwrap_or(0);
    let verify_warn_count = bundle["scorecard"]["verify"]["warn_count"]
        .as_u64()
        .unwrap_or(0);
    let compare_verdict = bundle["scorecard"]["compare"]["verdict"]
        .as_str()
        .unwrap_or("unknown");
    let history_count = bundle["history"]["count"].as_u64().unwrap_or(0);
    let health_delta = bundle["history"]["health"]["delta"].as_f64().unwrap_or(0.0);
    let artifacts = &bundle["artifacts"];

    let mut out = String::new();
    out.push_str("# EMNR CI Bundle\n\n");
    out.push_str(&format!("- bundle: `{}`\n", bundle_path));
    out.push_str(&format!("- status: `{}`\n", status));
    out.push_str(&format!("- decision: `{}`\n", decision));
    out.push_str(&format!("- stable: `{}`\n", stable));
    out.push_str(&format!("- trend: `{}`\n", trend));
    out.push_str(&format!("- health_score: `{:.2}`\n", health_score));
    out.push_str(&format!("- compare_verdict: `{}`\n", compare_verdict));
    out.push_str(&format!("- verify_fail_count: `{}`\n", verify_fail_count));
    out.push_str(&format!("- verify_warn_count: `{}`\n", verify_warn_count));
    out.push_str(&format!("- history_count: `{}`\n", history_count));
    out.push_str(&format!("- health_delta: `{:.6}`\n\n", health_delta));
    out.push_str("## Artifacts\n\n");
    out.push_str(&format!(
        "- ci_report: `{}`\n",
        artifacts["ci_report"].as_str().unwrap_or("")
    ));
    out.push_str(&format!(
        "- scorecard: `{}`\n",
        artifacts["scorecard"].as_str().unwrap_or("")
    ));
    out.push_str(&format!(
        "- history: `{}`\n",
        artifacts["history"].as_str().unwrap_or("")
    ));
    out.push_str(&format!(
        "- bundle: `{}`\n",
        artifacts["bundle"].as_str().unwrap_or("")
    ));
    out
}

fn emnr_promote_ci_bundle_json(
    bundle_path: &str,
    index_out: &str,
    output: Option<&str>,
    require_pass: bool,
) {
    let bundle = load_json_file_or_exit(bundle_path, "EMNR CI bundle");
    let report = emnr_promote_ci_bundle_report(bundle_path, index_out, &bundle, require_pass);
    let report_text = serde_json::to_string(&report).unwrap_or_else(|error| {
        format!(
            "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&error.to_string())
        )
    });
    if let Some(path) = output {
        if let Err(error) = fs::write(path, &report_text) {
            eprintln!(
                "Error writing EMNR CI bundle promotion report '{}': {}",
                path, error
            );
            process::exit(1);
        }
    }
    println!("{report_text}");
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_promote_ci_bundle_report(
    bundle_path: &str,
    index_out: &str,
    bundle: &JsonValue,
    require_pass: bool,
) -> JsonValue {
    let verify = emnr_verify_ci_bundle_report(bundle_path, bundle, require_pass);
    let verify_exit = verify["exit_code"].as_i64().unwrap_or(1);
    if verify_exit != 0 {
        return json!({
            "ok": false,
            "status": "rejected",
            "exit_code": verify_exit,
            "bundle": bundle_path,
            "index_out": index_out,
            "verify": verify,
        });
    }

    let index = emnr_update_ci_bundle_index(index_out, bundle_path, bundle);
    write_json_report_path(Path::new(index_out), &index, "EMNR CI bundle release index");
    json!({
        "ok": true,
        "status": "promoted",
        "exit_code": 0,
        "bundle": bundle_path,
        "index_out": index_out,
        "current": index["current"].clone(),
        "release_count": index["release_count"].clone(),
        "verify": verify,
    })
}

fn emnr_update_ci_bundle_index(
    index_out: &str,
    bundle_path: &str,
    bundle: &JsonValue,
) -> JsonValue {
    let existing = if Path::new(index_out).exists() {
        load_json_file_or_exit(index_out, "EMNR CI bundle release index")
    } else {
        json!({
            "schema": "matter.emnr.ci.bundle.index.v1",
            "current": JsonValue::Null,
            "release_count": 0,
            "releases": []
        })
    };
    let mut releases = existing
        .get("releases")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    let record = json!({
        "sequence": releases.len() + 1,
        "bundle": bundle_path,
        "status": bundle["status"].clone(),
        "decision": bundle["decision"].clone(),
        "stable": bundle["stable"].clone(),
        "trend": bundle["trend"].clone(),
        "health_score": bundle["scorecard"]["health_score"].clone(),
        "compare_verdict": bundle["scorecard"]["compare"]["verdict"].clone(),
        "artifacts": bundle["artifacts"].clone(),
    });
    releases.push(record.clone());
    json!({
        "schema": "matter.emnr.ci.bundle.index.v1",
        "current": record,
        "release_count": releases.len(),
        "releases": releases,
    })
}

fn emnr_ci_bundle_index_json(path: &str, latest: usize, require_current_approved: bool) {
    let index = load_json_file_or_exit(path, "EMNR CI bundle release index");
    let report = emnr_ci_bundle_index_report(path, &index, latest, require_current_approved);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_ci_bundle_index_report(
    path: &str,
    index: &JsonValue,
    latest: usize,
    require_current_approved: bool,
) -> JsonValue {
    let schema_ok =
        index.get("schema").and_then(JsonValue::as_str) == Some("matter.emnr.ci.bundle.index.v1");
    let releases = index
        .get("releases")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    let latest_records = releases
        .iter()
        .rev()
        .take(latest.max(1))
        .cloned()
        .collect::<Vec<_>>();
    let approve_count = releases
        .iter()
        .filter(|release| release["decision"].as_str() == Some("approve"))
        .count();
    let review_count = releases
        .iter()
        .filter(|release| release["decision"].as_str() == Some("review"))
        .count();
    let block_count = releases
        .iter()
        .filter(|release| release["decision"].as_str() == Some("block"))
        .count();
    let mean_health = if releases.is_empty() {
        JsonValue::Null
    } else {
        let sum = releases
            .iter()
            .filter_map(|release| release["health_score"].as_f64())
            .sum::<f64>();
        json!(sum / releases.len() as f64)
    };
    let current = index.get("current").cloned().unwrap_or(JsonValue::Null);
    let current_approved = current["decision"].as_str() == Some("approve")
        && current["stable"].as_bool() == Some(true);
    let exit_code = if !schema_ok || (require_current_approved && !current_approved) {
        1
    } else {
        0
    };
    json!({
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "input": path,
        "schema": index.get("schema").cloned().unwrap_or(JsonValue::Null),
        "require_current_approved": require_current_approved,
        "current_approved": current_approved,
        "current": current,
        "release_count": releases.len(),
        "summary": {
            "approve": approve_count,
            "review": review_count,
            "block": block_count,
        },
        "mean_health_score": mean_health,
        "latest": latest_records,
    })
}

fn emnr_export_ci_bundle_index_json(index_path: &str, out_dir: &str) {
    let index = load_json_file_or_exit(index_path, "EMNR CI bundle release index");
    let report = emnr_export_ci_bundle_index_report(index_path, out_dir, &index);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_export_ci_bundle_index_report(
    index_path: &str,
    out_dir: &str,
    index: &JsonValue,
) -> JsonValue {
    let schema_ok =
        index.get("schema").and_then(JsonValue::as_str) == Some("matter.emnr.ci.bundle.index.v1");
    let current_bundle = index["current"]["bundle"].as_str().unwrap_or_default();
    let current_exists = !current_bundle.is_empty() && Path::new(current_bundle).exists();
    if !schema_ok || !current_exists {
        return json!({
            "ok": false,
            "status": "fail",
            "exit_code": 1,
            "index": index_path,
            "out_dir": out_dir,
            "error": {
                "schema_ok": schema_ok,
                "current_exists": current_exists,
                "message": "EMNR CI bundle index is invalid or current bundle is missing"
            }
        });
    }

    let out_dir_path = Path::new(out_dir);
    if let Err(error) = fs::create_dir_all(out_dir_path) {
        eprintln!(
            "Error creating EMNR CI bundle package directory '{}': {}",
            out_dir, error
        );
        process::exit(1);
    }
    let package_index = out_dir_path.join("bundle-index.json");
    let package_bundle = out_dir_path.join("current-bundle.json");
    let package_json = out_dir_path.join("package.json");
    let current_bundle_json = load_json_file_or_exit(current_bundle, "EMNR CI current bundle");
    write_json_report_path(&package_index, index, "EMNR CI package bundle index");
    write_json_report_path(
        &package_bundle,
        &current_bundle_json,
        "EMNR CI package current bundle",
    );
    let package = json!({
        "schema": "matter.emnr.ci.bundle.package.v1",
        "source_index": index_path,
        "source_bundle": current_bundle,
        "artifacts": {
            "index": package_index,
            "bundle": package_bundle,
        },
        "current": index["current"].clone(),
        "release_count": index["release_count"].clone(),
    });
    write_json_report_path(&package_json, &package, "EMNR CI bundle package manifest");
    json!({
        "ok": true,
        "status": "exported",
        "exit_code": 0,
        "index": index_path,
        "out_dir": out_dir,
        "package": package_json,
        "artifacts": package["artifacts"].clone(),
        "current": package["current"].clone(),
    })
}

fn emnr_verify_ci_bundle_package_json(path: &str, require_pass: bool) {
    let package = load_json_file_or_exit(path, "EMNR CI bundle package");
    let report = emnr_verify_ci_bundle_package_report(path, &package, require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_verify_ci_bundle_package_report(
    path: &str,
    package: &JsonValue,
    require_pass: bool,
) -> JsonValue {
    let mut checks = Vec::new();
    let mut fail_count = 0u64;
    let schema_ok = package.get("schema").and_then(JsonValue::as_str)
        == Some("matter.emnr.ci.bundle.package.v1");
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "schema",
        schema_ok,
        "fail",
        "package schema must be matter.emnr.ci.bundle.package.v1",
    );

    let index_path = package["artifacts"]["index"].as_str().unwrap_or_default();
    let bundle_path = package["artifacts"]["bundle"].as_str().unwrap_or_default();
    let index_exists = !index_path.is_empty() && Path::new(index_path).exists();
    let bundle_exists = !bundle_path.is_empty() && Path::new(bundle_path).exists();
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "artifact_index",
        index_exists,
        "fail",
        "EMNR CI bundle package index artifact must exist",
    );
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "artifact_bundle",
        bundle_exists,
        "fail",
        "EMNR CI bundle package current bundle artifact must exist",
    );

    let index_report = if index_exists {
        let index = load_json_file_or_exit(index_path, "EMNR CI package bundle index");
        emnr_ci_bundle_index_report(index_path, &index, 5, require_pass)
    } else {
        JsonValue::Null
    };
    let bundle_report = if bundle_exists {
        let bundle = load_json_file_or_exit(bundle_path, "EMNR CI package current bundle");
        emnr_verify_ci_bundle_report(bundle_path, &bundle, require_pass)
    } else {
        JsonValue::Null
    };
    if index_report["exit_code"].as_i64().unwrap_or(1) != 0 {
        fail_count += 1;
    }
    if bundle_report["exit_code"].as_i64().unwrap_or(1) != 0 {
        fail_count += 1;
    }
    let exit_code = if fail_count > 0 { 1 } else { 0 };
    json!({
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "input": path,
        "require_pass": require_pass,
        "checks": checks,
        "fail_count": fail_count,
        "index": index_report,
        "bundle": bundle_report,
        "current": package.get("current").cloned().unwrap_or(JsonValue::Null),
    })
}

fn emnr_import_ci_bundle_package_json(path: &str, out_dir: &str, require_pass: bool) {
    let package = load_json_file_or_exit(path, "EMNR CI bundle package");
    let report = emnr_import_ci_bundle_package_report(path, out_dir, &package, require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_import_ci_bundle_package_report(
    package_path: &str,
    out_dir: &str,
    package: &JsonValue,
    require_pass: bool,
) -> JsonValue {
    let verify = emnr_verify_ci_bundle_package_report(package_path, package, require_pass);
    let verify_exit = verify["exit_code"].as_i64().unwrap_or(1);
    if verify_exit != 0 {
        return json!({
            "ok": false,
            "status": "rejected",
            "exit_code": verify_exit,
            "package": package_path,
            "out_dir": out_dir,
            "verify": verify,
        });
    }

    let out_dir_path = Path::new(out_dir);
    if let Err(error) = fs::create_dir_all(out_dir_path) {
        eprintln!(
            "Error creating EMNR CI bundle import directory '{}': {}",
            out_dir, error
        );
        process::exit(1);
    }
    let source_index = package["artifacts"]["index"].as_str().unwrap_or_default();
    let source_bundle = package["artifacts"]["bundle"].as_str().unwrap_or_default();
    let imported_index_path = out_dir_path.join("bundle-index.json");
    let imported_bundle_path = out_dir_path.join("current-bundle.json");
    let mut imported_index = load_json_file_or_exit(source_index, "EMNR CI package bundle index");
    let mut imported_bundle =
        load_json_file_or_exit(source_bundle, "EMNR CI package current bundle");

    if let Some(current) = imported_index
        .get_mut("current")
        .and_then(JsonValue::as_object_mut)
    {
        current.insert("bundle".to_string(), json!(imported_bundle_path));
        current.insert(
            "artifacts".to_string(),
            json!({
                "bundle": imported_bundle_path,
                "ci_report": imported_bundle["artifacts"]["ci_report"].clone(),
                "history": imported_bundle["artifacts"]["history"].clone(),
                "scorecard": imported_bundle["artifacts"]["scorecard"].clone(),
            }),
        );
    }
    if let Some(artifacts) = imported_bundle
        .get_mut("artifacts")
        .and_then(JsonValue::as_object_mut)
    {
        artifacts.insert("bundle".to_string(), json!(imported_bundle_path));
    }

    write_json_report_path(
        &imported_index_path,
        &imported_index,
        "EMNR imported CI bundle index",
    );
    write_json_report_path(
        &imported_bundle_path,
        &imported_bundle,
        "EMNR imported CI current bundle",
    );
    json!({
        "ok": true,
        "status": "imported",
        "exit_code": 0,
        "package": package_path,
        "out_dir": out_dir,
        "artifacts": {
            "index": imported_index_path,
            "bundle": imported_bundle_path,
        },
        "current": imported_index["current"].clone(),
        "verify": verify,
    })
}

fn emnr_init_json(force: bool) {
    let report = emnr_init_report(force);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_init_report(force: bool) -> JsonValue {
    let root = Path::new(".emnr");
    if let Err(error) = fs::create_dir_all(root) {
        eprintln!("Error creating .emnr directory: {}", error);
        process::exit(1);
    }

    let policy = json!({
        "schema": "matter.emnr.policy.v1",
        "gate": {
            "warn_as_fail": false,
            "risk_threshold": 0.4,
            "confidence_threshold": 0.55,
            "energy_threshold": 25.0,
            "critical_energy": 10.0
        },
        "promotion": {
            "max_risk_regression": 0.02,
            "max_error_regression": 0.02,
            "max_confidence_drop": 0.02,
            "min_energy": 20.0
        },
        "release": {
            "require_approved_bundle": true,
            "require_stable_history": true,
            "min_health_score": 95.0,
            "reject_trends": ["blocked", "degrading"]
        }
    });
    let paths = json!({
        "schema": "matter.emnr.paths.v1",
        "policy": ".emnr/policy.json",
        "state": ".emnr/state.json",
        "baseline_index": ".emnr/baseline-index.json",
        "bundle_index": ".emnr/bundle-index.json",
        "current_bundle": ".emnr/current-bundle.json",
        "artifacts_dir": ".build-artifacts"
    });
    let state = json!({
        "schema": "matter.emnr.project.state.v1",
        "status": "initialized",
        "tick": 0,
        "memory_records": 0,
        "last_decision": JsonValue::Null,
        "notes": "Project-level EMNR memory anchor. Runtime state artifacts remain JSON-compatible."
    });
    let default_baseline_index = json!({
        "schema": "matter.emnr.baseline.index.v1",
        "current": JsonValue::Null,
        "promotion_count": 0,
        "promotions": []
    });
    let default_bundle_index = json!({
        "schema": "matter.emnr.ci.bundle.index.v1",
        "current": JsonValue::Null,
        "release_count": 0,
        "releases": []
    });
    let baseline_index = emnr_init_seed_json(
        ".build-artifacts/emnr-imported-baseline/index.json",
        default_baseline_index,
    );
    let bundle_index = emnr_init_seed_json(
        ".build-artifacts/emnr-imported-ci-bundle-package/bundle-index.json",
        default_bundle_index,
    );

    let writes = vec![
        emnr_init_write_json(&root.join("policy.json"), &policy, force),
        emnr_init_write_json(&root.join("paths.json"), &paths, force),
        emnr_init_write_json(&root.join("state.json"), &state, force),
        emnr_init_write_json(&root.join("baseline-index.json"), &baseline_index, force),
        emnr_init_write_json(&root.join("bundle-index.json"), &bundle_index, force),
    ];
    let imported_bundle =
        Path::new(".build-artifacts/emnr-imported-ci-bundle-package/current-bundle.json");
    let bundle_link = if imported_bundle.exists() {
        let bundle = load_json_file_or_exit(
            &imported_bundle.to_string_lossy(),
            "EMNR imported current bundle",
        );
        emnr_init_write_json(&root.join("current-bundle.json"), &bundle, force)
    } else {
        emnr_init_write_json(
            &root.join("current-bundle.json"),
            &json!({
                "schema": "matter.emnr.ci.bundle.v1",
                "status": "missing",
                "decision": "review",
                "stable": false,
                "trend": "uninitialized",
                "artifacts": {}
            }),
            force,
        )
    };
    let mut all_writes = writes;
    all_writes.push(bundle_link);
    let written = all_writes
        .iter()
        .filter(|item| item["status"].as_str() == Some("written"))
        .count();
    let skipped = all_writes
        .iter()
        .filter(|item| item["status"].as_str() == Some("exists"))
        .count();
    json!({
        "schema": "matter.emnr.init.v1",
        "ok": true,
        "status": "initialized",
        "exit_code": 0,
        "force": force,
        "root": ".emnr",
        "summary": {
            "written": written,
            "skipped": skipped,
            "total": all_writes.len()
        },
        "files": all_writes,
    })
}

fn emnr_init_write_json(path: &Path, value: &JsonValue, force: bool) -> JsonValue {
    if path.exists() && !force {
        return json!({
            "path": path,
            "status": "exists",
            "written": false,
        });
    }
    write_json_report_path(path, value, "EMNR project memory");
    json!({
        "path": path,
        "status": "written",
        "written": true,
    })
}

fn emnr_init_seed_json(seed_path: &str, fallback: JsonValue) -> JsonValue {
    if Path::new(seed_path).exists() {
        load_json_file_or_exit(seed_path, "EMNR init seed")
    } else {
        fallback
    }
}

fn emnr_sync_json(force: bool, require_pass: bool) {
    let report = emnr_sync_report(force, require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_sync_report(force: bool, require_pass: bool) -> JsonValue {
    let root = Path::new(".emnr");
    if let Err(error) = fs::create_dir_all(root) {
        eprintln!("Error creating .emnr directory: {}", error);
        process::exit(1);
    }

    let files = vec![
        emnr_sync_copy_json(
            Path::new(".build-artifacts/emnr-imported-baseline/index.json"),
            &root.join("baseline-index.json"),
            force,
        ),
        emnr_sync_copy_json(
            Path::new(".build-artifacts/emnr-imported-ci-bundle-package/bundle-index.json"),
            &root.join("bundle-index.json"),
            force,
        ),
        emnr_sync_copy_json(
            Path::new(".build-artifacts/emnr-imported-ci-bundle-package/current-bundle.json"),
            &root.join("current-bundle.json"),
            force,
        ),
    ];
    let written = files
        .iter()
        .filter(|item| item["status"].as_str() == Some("written"))
        .count();
    let skipped = files
        .iter()
        .filter(|item| item["status"].as_str() == Some("exists"))
        .count();
    let missing_sources = files
        .iter()
        .filter(|item| item["status"].as_str() == Some("missing_source"))
        .count();

    let policy_path = ".emnr/policy.json";
    let index_path = ".emnr/bundle-index.json";
    let status_path = ".emnr/status.md";
    let status = if Path::new(policy_path).exists() && Path::new(index_path).exists() {
        let policy = load_json_file_or_exit(policy_path, "EMNR release policy");
        let index = load_json_file_or_exit(index_path, "EMNR CI bundle release index");
        let report = emnr_status_report(policy_path, index_path, &policy, &index, require_pass);
        let markdown = emnr_status_markdown(&report);
        if let Err(error) = fs::write(status_path, markdown) {
            eprintln!(
                "Error writing EMNR status markdown '{}': {}",
                status_path, error
            );
            process::exit(1);
        }
        json!({
            "status": "written",
            "path": status_path,
            "report": report,
        })
    } else {
        json!({
            "status": "skipped",
            "reason": "missing_policy_or_bundle_index",
            "path": status_path,
        })
    };
    let status_exit = status["report"]["exit_code"].as_i64().unwrap_or(0);
    let exit_code = if require_pass && status_exit != 0 {
        1
    } else {
        0
    };

    json!({
        "schema": "matter.emnr.sync.v1",
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "synced" } else { "failed" },
        "exit_code": exit_code,
        "force": force,
        "require_pass": require_pass,
        "root": ".emnr",
        "summary": {
            "written": written,
            "skipped": skipped,
            "missing_sources": missing_sources,
            "total": files.len(),
        },
        "files": files,
        "status_markdown": status,
    })
}

fn emnr_sync_copy_json(source: &Path, destination: &Path, force: bool) -> JsonValue {
    if !source.exists() {
        return json!({
            "source": source,
            "destination": destination,
            "status": "missing_source",
            "written": false,
        });
    }
    if destination.exists() && !force {
        return json!({
            "source": source,
            "destination": destination,
            "status": "exists",
            "written": false,
        });
    }
    let value = load_json_file_or_exit(&source.to_string_lossy(), "EMNR sync source");
    write_json_report_path(destination, &value, "EMNR synced project memory");
    json!({
        "source": source,
        "destination": destination,
        "status": "written",
        "written": true,
    })
}

fn emnr_doctor_json(require_pass: bool) {
    let report = emnr_doctor_report(require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_release_gate_json(policy_path: &str, index_path: &str, require_pass: bool) {
    let policy = load_json_file_or_exit(policy_path, "EMNR release policy");
    let index = load_json_file_or_exit(index_path, "EMNR CI bundle release index");
    let report = emnr_release_gate_report(policy_path, index_path, &policy, &index, require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_agent_advice_json(policy_path: &str, index_path: &str, require_pass: bool) {
    let policy = load_json_file_or_exit(policy_path, "EMNR release policy");
    let index = load_json_file_or_exit(index_path, "EMNR CI bundle release index");
    let gate = emnr_release_gate_report(policy_path, index_path, &policy, &index, require_pass);
    let advice = emnr_agent_advice_report(&gate, require_pass);
    println!(
        "{}",
        serde_json::to_string(&advice).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if advice["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_status_json(policy_path: &str, index_path: &str, require_pass: bool) {
    let policy = load_json_file_or_exit(policy_path, "EMNR release policy");
    let index = load_json_file_or_exit(index_path, "EMNR CI bundle release index");
    let report = emnr_status_report(policy_path, index_path, &policy, &index, require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_status_md(policy_path: &str, index_path: &str, output: &str, require_pass: bool) {
    let policy = load_json_file_or_exit(policy_path, "EMNR release policy");
    let index = load_json_file_or_exit(index_path, "EMNR CI bundle release index");
    let report = emnr_status_report(policy_path, index_path, &policy, &index, require_pass);
    let markdown = emnr_status_markdown(&report);
    if let Err(error) = fs::write(output, &markdown) {
        eprintln!("Error writing EMNR status markdown '{}': {}", output, error);
        process::exit(1);
    }
    println!("{}", markdown);
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_status_markdown(status: &JsonValue) -> String {
    let mut out = String::new();
    out.push_str("# EMNR Status\n\n");
    out.push_str(&format!(
        "- status: `{}`\n",
        status["status"].as_str().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- gate: `{}`\n",
        status["decision"]["gate"].as_str().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- advice: `{}`\n",
        status["decision"]["advice"].as_str().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- recommended_action: `{}`\n",
        status["decision"]["recommended_action"]
            .as_str()
            .unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- health_score: `{:.2}`\n",
        status["current"]["health_score"].as_f64().unwrap_or(0.0)
    ));
    out.push_str(&format!(
        "- stable: `{}`\n",
        status["current"]["stable"].as_bool().unwrap_or(false)
    ));
    out.push_str(&format!(
        "- trend: `{}`\n",
        status["current"]["trend"].as_str().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "- doctor: `{}/{}` checks passed\n",
        status["health"]["doctor"]["pass"].as_u64().unwrap_or(0),
        status["health"]["doctor"]["total"].as_u64().unwrap_or(0)
    ));
    out.push_str(&format!(
        "- release_fail_count: `{}`\n",
        status["health"]["release_fail_count"].as_u64().unwrap_or(0)
    ));
    out.push_str(&format!(
        "- release_warn_count: `{}`\n\n",
        status["health"]["release_warn_count"].as_u64().unwrap_or(0)
    ));
    out.push_str("## Artifacts\n\n");
    out.push_str(&format!(
        "- policy: `{}`\n",
        status["artifacts"]["policy"].as_str().unwrap_or("")
    ));
    out.push_str(&format!(
        "- bundle_index: `{}`\n",
        status["artifacts"]["bundle_index"].as_str().unwrap_or("")
    ));
    out.push_str(&format!(
        "- bundle: `{}`\n",
        status["artifacts"]["bundle"].as_str().unwrap_or("")
    ));
    out
}

fn emnr_status_report(
    policy_path: &str,
    index_path: &str,
    policy: &JsonValue,
    index: &JsonValue,
    require_pass: bool,
) -> JsonValue {
    let doctor = emnr_doctor_report(false);
    let gate = emnr_release_gate_report(policy_path, index_path, policy, index, require_pass);
    let advice = emnr_agent_advice_report(&gate, require_pass);
    let doctor_ok = doctor["exit_code"].as_i64().unwrap_or(1) == 0;
    let gate_ok = gate["gate"].as_str() == Some("allow");
    let advice_ok = advice["advice"].as_str() == Some("approve");
    let exit_code = if require_pass && !(doctor_ok && gate_ok && advice_ok) {
        1
    } else {
        0
    };
    json!({
        "schema": "matter.emnr.status.v1",
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "require_pass": require_pass,
        "policy_path": policy_path,
        "bundle_index": index_path,
        "decision": {
            "gate": gate["gate"].clone(),
            "advice": advice["advice"].clone(),
            "recommended_action": advice["recommended_action"].clone(),
            "reason_codes": advice["reason_codes"].clone(),
        },
        "current": gate.get("current").cloned().unwrap_or(JsonValue::Null),
        "health": {
            "doctor_ok": doctor_ok,
            "gate_ok": gate_ok,
            "advice_ok": advice_ok,
            "doctor": doctor["summary"].clone(),
            "release_fail_count": gate["fail_count"].clone(),
            "release_warn_count": gate["warn_count"].clone(),
        },
        "artifacts": {
            "bundle": gate["current"]["bundle"].clone(),
            "bundle_index": index_path,
            "policy": policy_path,
        },
    })
}

fn emnr_agent_advice_report(gate: &JsonValue, require_pass: bool) -> JsonValue {
    let gate_decision = gate["gate"].as_str().unwrap_or("block");
    let failed_checks = gate["checks"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|check| check["status"].as_str() == Some("fail"))
        .collect::<Vec<_>>();
    let warn_checks = gate["checks"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|check| check["status"].as_str() == Some("warn"))
        .collect::<Vec<_>>();
    let reason_codes = if !failed_checks.is_empty() {
        failed_checks
            .iter()
            .filter_map(|check| check["name"].as_str())
            .map(str::to_string)
            .collect::<Vec<_>>()
    } else if !warn_checks.is_empty() {
        warn_checks
            .iter()
            .filter_map(|check| check["name"].as_str())
            .map(str::to_string)
            .collect::<Vec<_>>()
    } else {
        vec!["emnr_release_gate_clean".to_string()]
    };
    let advice = match gate_decision {
        "allow" => "approve",
        "review" => "review",
        _ => "block",
    };
    let recommended_action = match advice {
        "approve" => "promote_or_release",
        "review" => "inspect_scorecard_and_rerun_pipeline",
        _ => "block_release_and_rebuild_emnr_bundle",
    };
    let exit_code = if require_pass && advice != "approve" {
        1
    } else {
        0
    };
    json!({
        "schema": "matter.emnr.agent.advice.v1",
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "require_pass": require_pass,
        "advice": advice,
        "gate": gate_decision,
        "recommended_action": recommended_action,
        "reason_codes": reason_codes,
        "failed_checks": failed_checks,
        "warn_checks": warn_checks,
        "current": gate.get("current").cloned().unwrap_or(JsonValue::Null),
        "release_gate": gate,
    })
}

fn emnr_release_gate_report(
    policy_path: &str,
    index_path: &str,
    policy: &JsonValue,
    index: &JsonValue,
    require_pass: bool,
) -> JsonValue {
    let release = &policy["release"];
    let require_approved_bundle = release["require_approved_bundle"].as_bool().unwrap_or(true);
    let require_stable_history = release["require_stable_history"].as_bool().unwrap_or(true);
    let min_health_score = release["min_health_score"].as_f64().unwrap_or(95.0);
    let reject_trends = release["reject_trends"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| vec![json!("blocked"), json!("degrading")]);
    let current = index.get("current").cloned().unwrap_or(JsonValue::Null);
    let decision = current["decision"].as_str().unwrap_or("unknown");
    let stable = current["stable"].as_bool().unwrap_or(false);
    let trend = current["trend"].as_str().unwrap_or("unknown");
    let health_score = current["health_score"].as_f64().unwrap_or(0.0);
    let index_report = emnr_ci_bundle_index_report(index_path, index, 5, require_approved_bundle);

    let mut checks = Vec::new();
    let mut fail_count = 0u64;
    let mut warn_count = 0u64;
    push_release_check(
        &mut checks,
        &mut fail_count,
        "index_schema",
        index.get("schema").and_then(JsonValue::as_str) == Some("matter.emnr.ci.bundle.index.v1"),
        "fail",
        "bundle index schema must be matter.emnr.ci.bundle.index.v1",
    );
    push_release_check(
        &mut checks,
        &mut fail_count,
        "approved_bundle",
        !require_approved_bundle || decision == "approve",
        "fail",
        "current bundle must be approved",
    );
    push_release_check(
        &mut checks,
        &mut fail_count,
        "stable_history",
        !require_stable_history || stable,
        "fail",
        "current bundle history must be stable",
    );
    push_release_check(
        &mut checks,
        &mut fail_count,
        "health_score",
        health_score >= min_health_score,
        "fail",
        "current bundle health score is below release policy",
    );
    let trend_rejected = reject_trends
        .iter()
        .filter_map(JsonValue::as_str)
        .any(|rejected| rejected == trend);
    push_release_check(
        &mut checks,
        &mut fail_count,
        "trend",
        !trend_rejected,
        "fail",
        "current bundle trend is rejected by release policy",
    );
    if decision == "review" {
        warn_count += 1;
        checks.push(json!({
            "name": "manual_review",
            "status": "warn",
            "message": "current bundle asks for manual review",
        }));
    }

    let gate = if fail_count > 0 {
        "block"
    } else if warn_count > 0 {
        "review"
    } else {
        "allow"
    };
    let exit_code = if require_pass && gate != "allow" {
        1
    } else {
        0
    };
    json!({
        "schema": "matter.emnr.release.gate.v1",
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "gate": gate,
        "require_pass": require_pass,
        "policy_path": policy_path,
        "bundle_index": index_path,
        "policy": {
            "require_approved_bundle": require_approved_bundle,
            "require_stable_history": require_stable_history,
            "min_health_score": min_health_score,
            "reject_trends": reject_trends,
        },
        "current": current,
        "checks": checks,
        "fail_count": fail_count,
        "warn_count": warn_count,
        "index": index_report,
    })
}

fn push_release_check(
    checks: &mut Vec<JsonValue>,
    fail_count: &mut u64,
    name: &str,
    passed: bool,
    severity: &str,
    message: &str,
) {
    if !passed && severity == "fail" {
        *fail_count += 1;
    }
    checks.push(json!({
        "name": name,
        "status": if passed { "pass" } else { severity },
        "message": message,
    }));
}

fn emnr_doctor_report(require_pass: bool) -> JsonValue {
    let checks = vec![
        emnr_doctor_file_check("emnr_workspace", "emnr/Cargo.toml"),
        emnr_doctor_file_check(
            "emnr_policy",
            "emnr/examples/matter_emnr_pipeline_policy.json",
        ),
        emnr_doctor_file_check("matter_cli", "crates/matter-cli/Cargo.toml"),
        emnr_doctor_json_check(
            "ci_bundle",
            ".build-artifacts/emnr-ci-bundle/bundle.json",
            |path, value| emnr_verify_ci_bundle_report(path, value, require_pass),
        ),
        emnr_doctor_json_check(
            "ci_bundle_index",
            ".build-artifacts/emnr-ci-bundle-index.json",
            |path, value| emnr_ci_bundle_index_report(path, value, 5, require_pass),
        ),
        emnr_doctor_json_check(
            "ci_bundle_package",
            ".build-artifacts/emnr-ci-bundle-package/package.json",
            |path, value| emnr_verify_ci_bundle_package_report(path, value, require_pass),
        ),
        emnr_doctor_json_check(
            "imported_ci_bundle_index",
            ".build-artifacts/emnr-imported-ci-bundle-package/bundle-index.json",
            |path, value| emnr_ci_bundle_index_report(path, value, 5, require_pass),
        ),
        emnr_doctor_json_check(
            "imported_ci_bundle",
            ".build-artifacts/emnr-imported-ci-bundle-package/current-bundle.json",
            |path, value| emnr_verify_ci_bundle_report(path, value, require_pass),
        ),
        emnr_doctor_json_check(
            "baseline_package",
            ".build-artifacts/emnr-baseline-package/package.json",
            |path, value| emnr_verify_baseline_package_report(path, value, require_pass),
        ),
        emnr_doctor_json_check(
            "imported_baseline",
            ".build-artifacts/emnr-imported-baseline",
            |path, _value| emnr_baseline_doctor_report(path, require_pass),
        ),
    ];

    let pass_count = checks
        .iter()
        .filter(|check| check["status"].as_str() == Some("pass"))
        .count();
    let missing_count = checks
        .iter()
        .filter(|check| check["status"].as_str() == Some("missing"))
        .count();
    let fail_count = checks
        .iter()
        .filter(|check| check["status"].as_str() == Some("fail"))
        .count();
    let exit_code = if fail_count > 0 || (require_pass && missing_count > 0) {
        1
    } else {
        0
    };
    json!({
        "schema": "matter.emnr.doctor.v1",
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "require_pass": require_pass,
        "summary": {
            "total": checks.len(),
            "pass": pass_count,
            "missing": missing_count,
            "fail": fail_count,
        },
        "checks": checks,
    })
}

fn emnr_doctor_file_check(name: &str, path: &str) -> JsonValue {
    let exists = Path::new(path).exists();
    json!({
        "name": name,
        "path": path,
        "status": if exists { "pass" } else { "missing" },
        "ok": exists,
    })
}

fn emnr_doctor_json_check<F>(name: &str, path: &str, verify: F) -> JsonValue
where
    F: Fn(&str, &JsonValue) -> JsonValue,
{
    if !Path::new(path).exists() {
        return json!({
            "name": name,
            "path": path,
            "status": "missing",
            "ok": false,
        });
    }
    let value = if Path::new(path).is_dir() {
        JsonValue::Null
    } else {
        load_json_file_or_exit(path, "EMNR doctor artifact")
    };
    let report = verify(path, &value);
    let ok = report["exit_code"].as_i64().unwrap_or(1) == 0;
    json!({
        "name": name,
        "path": path,
        "status": if ok { "pass" } else { "fail" },
        "ok": ok,
        "report": report,
    })
}

fn emnr_pipeline_artifacts_json(
    baseline_state: &Path,
    candidate_state: &Path,
    gate_report: &Path,
    train_report: &Path,
    promote_report: &Path,
    audit_report: &Path,
    manifest: &Path,
) -> JsonValue {
    json!({
        "baseline_state": baseline_state,
        "candidate_state": candidate_state,
        "gate_report": gate_report,
        "train_report": train_report,
        "promote_report": promote_report,
        "audit_report": audit_report,
        "manifest": manifest,
    })
}

fn emnr_pipeline_policy_json(
    gate_policy: EmnrGatePolicy,
    promote_policy: EmnrPromotePolicy,
) -> JsonValue {
    json!({
        "gate": {
            "warn_as_fail": gate_policy.warn_as_fail,
            "risk_threshold": gate_policy.risk_threshold,
            "confidence_threshold": gate_policy.confidence_threshold,
            "energy_threshold": gate_policy.energy_threshold,
            "critical_energy": gate_policy.critical_energy,
        },
        "promotion": {
            "max_risk_regression": promote_policy.max_risk_regression,
            "max_error_regression": promote_policy.max_error_regression,
            "max_confidence_drop": promote_policy.max_confidence_drop,
            "min_energy": promote_policy.min_energy,
        }
    })
}

fn emnr_pipeline_manifest_json(
    options: &EmnrPipelineOptions,
    artifacts: &JsonValue,
    policy: &JsonValue,
    gate_report: &JsonValue,
    train_report: &JsonValue,
    promote_report: &JsonValue,
    audit_report: &JsonValue,
) -> JsonValue {
    json!({
        "schema": "matter.emnr.pipeline.manifest.v1",
        "command": "emnr-pipeline-json",
        "inputs": options.paths,
        "epochs": options.epochs,
        "out_dir": options.out_dir,
        "policy_path": options.policy_path,
        "fail_on_warn": options.fail_on_warn,
        "policy": policy,
        "artifacts": artifacts,
        "summary": {
            "status": audit_report["status"].clone(),
            "ok": audit_report["ok"].clone(),
            "health_score": audit_report["health_score"].clone(),
            "gate_status": gate_report["status"].clone(),
            "train_status": train_report["status"].clone(),
            "promote_status": promote_report["status"].clone(),
            "audit_status": audit_report["status"].clone(),
            "learning_delta": train_report["learning_delta"].clone(),
            "promotion_delta": promote_report["metrics"]["delta"].clone(),
        }
    })
}

fn emnr_verify_manifest_json(path: &str, require_pass: bool) {
    let manifest = load_json_file_or_exit(path, "EMNR pipeline manifest");
    let report = emnr_verify_manifest_report(path, &manifest, require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_verify_manifest_report(path: &str, manifest: &JsonValue, require_pass: bool) -> JsonValue {
    let mut checks = Vec::new();
    let mut fail_count = 0u64;
    let mut warn_count = 0u64;

    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "schema",
        manifest.get("schema").and_then(JsonValue::as_str)
            == Some("matter.emnr.pipeline.manifest.v1"),
        "fail",
        "manifest schema must be matter.emnr.pipeline.manifest.v1",
    );
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "command",
        manifest.get("command").and_then(JsonValue::as_str) == Some("emnr-pipeline-json"),
        "fail",
        "manifest command must be emnr-pipeline-json",
    );
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "inputs",
        manifest
            .get("inputs")
            .and_then(JsonValue::as_array)
            .map(|inputs| !inputs.is_empty())
            .unwrap_or(false),
        "fail",
        "manifest must include at least one input",
    );
    let status = manifest["summary"]["status"].as_str().unwrap_or("unknown");
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "summary_status",
        !require_pass || status == "pass",
        "fail",
        "manifest summary status must be pass when --require-pass is set",
    );

    if let Some(artifacts) = manifest.get("artifacts").and_then(JsonValue::as_object) {
        for key in [
            "baseline_state",
            "candidate_state",
            "gate_report",
            "train_report",
            "promote_report",
            "audit_report",
            "manifest",
        ] {
            let exists = artifacts
                .get(key)
                .and_then(JsonValue::as_str)
                .map(Path::new)
                .map(Path::exists)
                .unwrap_or(false);
            push_manifest_check(
                &mut checks,
                &mut fail_count,
                &format!("artifact_{key}"),
                exists,
                "fail",
                "referenced artifact must exist",
            );
        }
    } else {
        push_manifest_check(
            &mut checks,
            &mut fail_count,
            "artifacts",
            false,
            "fail",
            "manifest must include artifacts object",
        );
    }

    let health_score = manifest["summary"]["health_score"].as_f64().unwrap_or(0.0);
    push_manifest_check(
        &mut checks,
        &mut warn_count,
        "health_score",
        health_score >= 50.0,
        "warn",
        "health score below 50 indicates weak EMNR pipeline health",
    );

    let exit_code = if fail_count > 0 { 1 } else { 0 };
    json!({
        "ok": exit_code == 0,
        "status": if fail_count > 0 { "fail" } else if warn_count > 0 { "warn" } else { "pass" },
        "exit_code": exit_code,
        "input": path,
        "require_pass": require_pass,
        "schema": manifest.get("schema").cloned().unwrap_or(JsonValue::Null),
        "summary": manifest.get("summary").cloned().unwrap_or(JsonValue::Null),
        "checks": checks,
        "fail_count": fail_count,
        "warn_count": warn_count,
    })
}

fn emnr_compare_manifests_json(
    baseline_path: &str,
    candidate_path: &str,
    require_non_regression: bool,
) {
    let baseline = load_json_file_or_exit(baseline_path, "EMNR baseline manifest");
    let candidate = load_json_file_or_exit(candidate_path, "EMNR candidate manifest");
    let report = emnr_compare_manifests_report(
        baseline_path,
        candidate_path,
        &baseline,
        &candidate,
        require_non_regression,
    );
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_compare_manifests_report(
    baseline_path: &str,
    candidate_path: &str,
    baseline: &JsonValue,
    candidate: &JsonValue,
    require_non_regression: bool,
) -> JsonValue {
    let baseline_summary = &baseline["summary"];
    let candidate_summary = &candidate["summary"];
    let baseline_status = baseline_summary["status"].as_str().unwrap_or("unknown");
    let candidate_status = candidate_summary["status"].as_str().unwrap_or("unknown");
    let baseline_health = baseline_summary["health_score"].as_f64().unwrap_or(0.0);
    let candidate_health = candidate_summary["health_score"].as_f64().unwrap_or(0.0);
    let baseline_learning = &baseline_summary["learning_delta"];
    let candidate_learning = &candidate_summary["learning_delta"];
    let baseline_promotion = &baseline_summary["promotion_delta"];
    let candidate_promotion = &candidate_summary["promotion_delta"];

    let deltas = json!({
        "health_score": candidate_health - baseline_health,
        "learning": {
            "confidence": json_delta(candidate_learning, baseline_learning, "confidence"),
            "risk": json_delta(candidate_learning, baseline_learning, "risk"),
            "prediction_error": json_delta(candidate_learning, baseline_learning, "prediction_error"),
            "energy": json_delta(candidate_learning, baseline_learning, "energy"),
            "memory_records": json_delta(candidate_learning, baseline_learning, "memory_records"),
        },
        "promotion": {
            "confidence": json_delta(candidate_promotion, baseline_promotion, "confidence"),
            "risk": json_delta(candidate_promotion, baseline_promotion, "risk"),
            "prediction_error": json_delta(candidate_promotion, baseline_promotion, "prediction_error"),
            "energy": json_delta(candidate_promotion, baseline_promotion, "energy"),
            "memory_records": json_delta(candidate_promotion, baseline_promotion, "memory_records"),
        }
    });

    let mut checks = Vec::new();
    let mut fail_count = 0u64;
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "candidate_schema",
        candidate.get("schema").and_then(JsonValue::as_str)
            == Some("matter.emnr.pipeline.manifest.v1"),
        "fail",
        "candidate manifest schema must be matter.emnr.pipeline.manifest.v1",
    );
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "status_non_regression",
        !require_non_regression || status_rank(candidate_status) >= status_rank(baseline_status),
        "fail",
        "candidate status must not regress below baseline",
    );
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "health_non_regression",
        !require_non_regression
            || candidate_health + EMNR_MANIFEST_COMPARE_EPSILON >= baseline_health,
        "fail",
        "candidate health score must not decrease",
    );
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "promotion_confidence_non_regression",
        !require_non_regression
            || deltas["promotion"]["confidence"]
                .as_f64()
                .unwrap_or_default()
                >= -EMNR_MANIFEST_COMPARE_EPSILON,
        "fail",
        "candidate promotion confidence delta must not decrease",
    );
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "promotion_risk_non_regression",
        !require_non_regression
            || deltas["promotion"]["risk"].as_f64().unwrap_or_default()
                <= EMNR_MANIFEST_COMPARE_EPSILON,
        "fail",
        "candidate promotion risk delta must not increase",
    );
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "promotion_error_non_regression",
        !require_non_regression
            || deltas["promotion"]["prediction_error"]
                .as_f64()
                .unwrap_or_default()
                <= EMNR_MANIFEST_COMPARE_EPSILON,
        "fail",
        "candidate promotion prediction error delta must not increase",
    );

    let verdict = if fail_count > 0 {
        "regressed"
    } else if candidate_health > baseline_health + EMNR_MANIFEST_COMPARE_EPSILON
        || deltas["promotion"]["confidence"]
            .as_f64()
            .unwrap_or_default()
            > EMNR_MANIFEST_COMPARE_EPSILON
        || deltas["promotion"]["risk"].as_f64().unwrap_or_default() < -EMNR_MANIFEST_COMPARE_EPSILON
        || deltas["promotion"]["prediction_error"]
            .as_f64()
            .unwrap_or_default()
            < -EMNR_MANIFEST_COMPARE_EPSILON
    {
        "improved"
    } else {
        "equivalent"
    };
    let exit_code = if require_non_regression && fail_count > 0 {
        1
    } else {
        0
    };
    json!({
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "verdict": verdict,
        "require_non_regression": require_non_regression,
        "baseline": {
            "path": baseline_path,
            "status": baseline_status,
            "health_score": baseline_health,
        },
        "candidate": {
            "path": candidate_path,
            "status": candidate_status,
            "health_score": candidate_health,
        },
        "deltas": deltas,
        "checks": checks,
        "fail_count": fail_count,
    })
}

fn emnr_promote_baseline_json(options: &EmnrBaselinePromoteOptions) {
    let report = emnr_promote_baseline_report(options);
    let report_text = serde_json::to_string(&report).unwrap_or_else(|error| {
        format!(
            "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(&error.to_string())
        )
    });
    if let Some(path) = options.output.as_deref() {
        if let Err(error) = fs::write(path, &report_text) {
            eprintln!(
                "Error writing EMNR baseline promotion report '{}': {}",
                path, error
            );
            process::exit(1);
        }
    }
    println!("{report_text}");
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_promote_baseline_report(options: &EmnrBaselinePromoteOptions) -> JsonValue {
    let manifest = load_json_file_or_exit(&options.manifest, "EMNR baseline candidate manifest");
    let verify = emnr_verify_manifest_report(&options.manifest, &manifest, options.require_pass);
    let verify_exit = verify["exit_code"].as_i64().unwrap_or(1);
    if verify_exit == 0 {
        let baseline_path = Path::new(&options.baseline_output);
        if let Some(parent) = baseline_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            if let Err(error) = fs::create_dir_all(parent) {
                eprintln!(
                    "Error creating EMNR baseline directory '{}': {}",
                    parent.display(),
                    error
                );
                process::exit(1);
            }
        }
        write_json_report_path(baseline_path, &manifest, "EMNR promoted baseline manifest");
        if let Some(index_path) = options.index_output.as_deref() {
            let index = emnr_update_baseline_index(index_path, options, &manifest);
            write_json_report_path(
                Path::new(index_path),
                &index,
                "EMNR baseline promotion index",
            );
        }
    }
    json!({
        "ok": verify_exit == 0,
        "status": if verify_exit == 0 { "promoted" } else { "rejected" },
        "exit_code": verify_exit,
        "manifest": options.manifest,
        "baseline_output": options.baseline_output,
        "index_output": options.index_output,
        "require_pass": options.require_pass,
        "verify": verify,
    })
}

fn emnr_update_baseline_index(
    index_path: &str,
    options: &EmnrBaselinePromoteOptions,
    manifest: &JsonValue,
) -> JsonValue {
    let existing = if Path::new(index_path).exists() {
        load_json_file_or_exit(index_path, "EMNR baseline index")
    } else {
        json!({
            "schema": "matter.emnr.baseline.index.v1",
            "promotions": []
        })
    };
    let mut promotions = existing
        .get("promotions")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    promotions.push(json!({
        "sequence": promotions.len() + 1,
        "manifest": options.manifest,
        "baseline_output": options.baseline_output,
        "status": manifest["summary"]["status"].clone(),
        "health_score": manifest["summary"]["health_score"].clone(),
        "inputs": manifest["inputs"].clone(),
        "epochs": manifest["epochs"].clone(),
        "policy_path": manifest["policy_path"].clone(),
        "learning_delta": manifest["summary"]["learning_delta"].clone(),
        "promotion_delta": manifest["summary"]["promotion_delta"].clone(),
    }));
    json!({
        "schema": "matter.emnr.baseline.index.v1",
        "current": {
            "manifest": options.manifest,
            "baseline_output": options.baseline_output,
            "status": manifest["summary"]["status"].clone(),
            "health_score": manifest["summary"]["health_score"].clone(),
        },
        "promotion_count": promotions.len(),
        "promotions": promotions,
    })
}

fn emnr_baseline_index_json(path: &str, latest: usize) {
    let index = load_json_file_or_exit(path, "EMNR baseline index");
    let report = emnr_baseline_index_report(path, &index, latest);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_baseline_index_report(path: &str, index: &JsonValue, latest: usize) -> JsonValue {
    let schema_ok =
        index.get("schema").and_then(JsonValue::as_str) == Some("matter.emnr.baseline.index.v1");
    let promotions = index
        .get("promotions")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    let latest_records = promotions
        .iter()
        .rev()
        .take(latest.max(1))
        .cloned()
        .collect::<Vec<_>>();
    let mean_health = if promotions.is_empty() {
        JsonValue::Null
    } else {
        let sum = promotions
            .iter()
            .filter_map(|promotion| promotion.get("health_score").and_then(JsonValue::as_f64))
            .sum::<f64>();
        json!(sum / promotions.len() as f64)
    };
    json!({
        "ok": schema_ok,
        "status": if schema_ok { "pass" } else { "fail" },
        "exit_code": if schema_ok { 0 } else { 1 },
        "input": path,
        "schema": index.get("schema").cloned().unwrap_or(JsonValue::Null),
        "current": index.get("current").cloned().unwrap_or(JsonValue::Null),
        "promotion_count": promotions.len(),
        "mean_health_score": mean_health,
        "latest": latest_records,
    })
}

fn emnr_export_baseline_json(index_path: &str, out_dir: &str) {
    let index = load_json_file_or_exit(index_path, "EMNR baseline index");
    let report = emnr_export_baseline_report(index_path, out_dir, &index);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_export_baseline_report(index_path: &str, out_dir: &str, index: &JsonValue) -> JsonValue {
    let baseline_source = index["current"]["baseline_output"]
        .as_str()
        .unwrap_or_default();
    let schema_ok =
        index.get("schema").and_then(JsonValue::as_str) == Some("matter.emnr.baseline.index.v1");
    let baseline_exists = !baseline_source.is_empty() && Path::new(baseline_source).exists();
    if !schema_ok || !baseline_exists {
        return json!({
            "ok": false,
            "status": "fail",
            "exit_code": 1,
            "index": index_path,
            "out_dir": out_dir,
            "error": {
                "schema_ok": schema_ok,
                "baseline_exists": baseline_exists,
                "message": "baseline index is invalid or current baseline file is missing"
            }
        });
    }

    let out_dir_path = Path::new(out_dir);
    if let Err(error) = fs::create_dir_all(out_dir_path) {
        eprintln!(
            "Error creating EMNR baseline package directory '{}': {}",
            out_dir, error
        );
        process::exit(1);
    }
    let package_index = out_dir_path.join("baseline-index.json");
    let package_manifest = out_dir_path.join("baseline-manifest.json");
    let package_json = out_dir_path.join("package.json");
    write_json_report_path(&package_index, index, "EMNR package baseline index");
    let baseline_manifest = load_json_file_or_exit(baseline_source, "EMNR current baseline");
    write_json_report_path(
        &package_manifest,
        &baseline_manifest,
        "EMNR package baseline manifest",
    );
    let package = json!({
        "schema": "matter.emnr.baseline.package.v1",
        "source_index": index_path,
        "source_baseline": baseline_source,
        "artifacts": {
            "index": package_index,
            "manifest": package_manifest,
        },
        "current": index["current"].clone(),
        "promotion_count": index["promotion_count"].clone(),
    });
    write_json_report_path(&package_json, &package, "EMNR baseline package manifest");
    json!({
        "ok": true,
        "status": "exported",
        "exit_code": 0,
        "index": index_path,
        "out_dir": out_dir,
        "package": package_json,
        "artifacts": package["artifacts"].clone(),
        "current": package["current"].clone(),
    })
}

fn emnr_verify_baseline_package_json(path: &str, require_pass: bool) {
    let package = load_json_file_or_exit(path, "EMNR baseline package");
    let report = emnr_verify_baseline_package_report(path, &package, require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_verify_baseline_package_report(
    path: &str,
    package: &JsonValue,
    require_pass: bool,
) -> JsonValue {
    let mut checks = Vec::new();
    let mut fail_count = 0u64;
    let schema_ok = package.get("schema").and_then(JsonValue::as_str)
        == Some("matter.emnr.baseline.package.v1");
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "schema",
        schema_ok,
        "fail",
        "package schema must be matter.emnr.baseline.package.v1",
    );

    let index_path = package["artifacts"]["index"].as_str().unwrap_or_default();
    let manifest_path = package["artifacts"]["manifest"]
        .as_str()
        .unwrap_or_default();
    let index_exists = !index_path.is_empty() && Path::new(index_path).exists();
    let manifest_exists = !manifest_path.is_empty() && Path::new(manifest_path).exists();
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "artifact_index",
        index_exists,
        "fail",
        "baseline package index artifact must exist",
    );
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "artifact_manifest",
        manifest_exists,
        "fail",
        "baseline package manifest artifact must exist",
    );

    let index_report = if index_exists {
        let index = load_json_file_or_exit(index_path, "EMNR package baseline index");
        emnr_baseline_index_report(index_path, &index, 5)
    } else {
        JsonValue::Null
    };
    let manifest_report = if manifest_exists {
        let manifest = load_json_file_or_exit(manifest_path, "EMNR package baseline manifest");
        emnr_verify_manifest_report(manifest_path, &manifest, require_pass)
    } else {
        JsonValue::Null
    };
    if index_report["exit_code"].as_i64().unwrap_or(1) != 0 {
        fail_count += 1;
    }
    if manifest_report["exit_code"].as_i64().unwrap_or(1) != 0 {
        fail_count += 1;
    }
    let exit_code = if fail_count > 0 { 1 } else { 0 };
    json!({
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "input": path,
        "require_pass": require_pass,
        "checks": checks,
        "fail_count": fail_count,
        "index": index_report,
        "manifest": manifest_report,
        "current": package.get("current").cloned().unwrap_or(JsonValue::Null),
    })
}

fn emnr_import_baseline_package_json(path: &str, out_dir: &str, require_pass: bool) {
    let package = load_json_file_or_exit(path, "EMNR baseline package");
    let report = emnr_import_baseline_package_report(path, out_dir, &package, require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_import_baseline_package_report(
    package_path: &str,
    out_dir: &str,
    package: &JsonValue,
    require_pass: bool,
) -> JsonValue {
    let verify = emnr_verify_baseline_package_report(package_path, package, require_pass);
    let verify_exit = verify["exit_code"].as_i64().unwrap_or(1);
    if verify_exit != 0 {
        return json!({
            "ok": false,
            "status": "rejected",
            "exit_code": verify_exit,
            "package": package_path,
            "out_dir": out_dir,
            "verify": verify,
        });
    }

    let out_dir_path = Path::new(out_dir);
    if let Err(error) = fs::create_dir_all(out_dir_path) {
        eprintln!(
            "Error creating EMNR baseline import directory '{}': {}",
            out_dir, error
        );
        process::exit(1);
    }
    let source_index = package["artifacts"]["index"].as_str().unwrap_or_default();
    let source_manifest = package["artifacts"]["manifest"]
        .as_str()
        .unwrap_or_default();
    let imported_index_path = out_dir_path.join("index.json");
    let imported_manifest_path = out_dir_path.join("main-manifest.json");
    let mut imported_index = load_json_file_or_exit(source_index, "EMNR package baseline index");
    let imported_manifest =
        load_json_file_or_exit(source_manifest, "EMNR package baseline manifest");
    if let Some(current) = imported_index
        .get_mut("current")
        .and_then(JsonValue::as_object_mut)
    {
        current.insert("manifest".to_string(), json!(imported_manifest_path));
        current.insert("baseline_output".to_string(), json!(imported_manifest_path));
    }
    write_json_report_path(
        &imported_index_path,
        &imported_index,
        "EMNR imported baseline index",
    );
    write_json_report_path(
        &imported_manifest_path,
        &imported_manifest,
        "EMNR imported baseline manifest",
    );
    json!({
        "ok": true,
        "status": "imported",
        "exit_code": 0,
        "package": package_path,
        "out_dir": out_dir,
        "artifacts": {
            "index": imported_index_path,
            "manifest": imported_manifest_path,
        },
        "current": package.get("current").cloned().unwrap_or(JsonValue::Null),
        "verify": verify,
    })
}

fn emnr_baseline_doctor_json(dir: &str, require_pass: bool) {
    let report = emnr_baseline_doctor_report(dir, require_pass);
    println!(
        "{}",
        serde_json::to_string(&report).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(&error.to_string())
            )
        })
    );
    if report["exit_code"].as_i64().unwrap_or(1) != 0 {
        process::exit(1);
    }
}

fn emnr_baseline_doctor_report(dir: &str, require_pass: bool) -> JsonValue {
    let baseline_dir = Path::new(dir);
    let index_path = baseline_dir.join("index.json");
    let manifest_path = baseline_dir.join("main-manifest.json");
    let mut checks = Vec::new();
    let mut fail_count = 0u64;

    let index_exists = index_path.exists();
    let manifest_exists = manifest_path.exists();
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "index_file",
        index_exists,
        "fail",
        "baseline directory must contain index.json",
    );
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "manifest_file",
        manifest_exists,
        "fail",
        "baseline directory must contain main-manifest.json",
    );

    let index_report = if index_exists {
        let index = load_json_file_or_exit(
            &index_path.to_string_lossy(),
            "EMNR imported baseline index",
        );
        emnr_baseline_index_report(&index_path.to_string_lossy(), &index, 5)
    } else {
        JsonValue::Null
    };
    let manifest_report = if manifest_exists {
        let manifest = load_json_file_or_exit(
            &manifest_path.to_string_lossy(),
            "EMNR imported baseline manifest",
        );
        emnr_verify_manifest_report(&manifest_path.to_string_lossy(), &manifest, require_pass)
    } else {
        JsonValue::Null
    };

    let index_ok = index_report["exit_code"].as_i64().unwrap_or(1) == 0;
    let manifest_ok = manifest_report["exit_code"].as_i64().unwrap_or(1) == 0;
    if index_exists && !index_ok {
        fail_count += 1;
    }
    if manifest_exists && !manifest_ok {
        fail_count += 1;
    }

    let current_manifest = index_report["current"]["manifest"]
        .as_str()
        .unwrap_or_default();
    let current_manifest_matches = !current_manifest.is_empty()
        && Path::new(current_manifest)
            .canonicalize()
            .ok()
            .zip(manifest_path.canonicalize().ok())
            .is_some_and(|(left, right)| left == right);
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "current_manifest_pointer",
        !index_exists || !manifest_exists || current_manifest_matches,
        "fail",
        "index current.manifest must point at main-manifest.json for portable CI use",
    );

    let baseline_output = index_report["current"]["baseline_output"]
        .as_str()
        .unwrap_or_default();
    let baseline_output_matches = !baseline_output.is_empty()
        && Path::new(baseline_output)
            .canonicalize()
            .ok()
            .zip(manifest_path.canonicalize().ok())
            .is_some_and(|(left, right)| left == right);
    push_manifest_check(
        &mut checks,
        &mut fail_count,
        "baseline_output_pointer",
        !index_exists || !manifest_exists || baseline_output_matches,
        "fail",
        "index current.baseline_output must point at main-manifest.json for exported package roundtrips",
    );

    let exit_code = if fail_count > 0 { 1 } else { 0 };
    json!({
        "ok": exit_code == 0,
        "status": if exit_code == 0 { "pass" } else { "fail" },
        "exit_code": exit_code,
        "baseline_dir": dir,
        "require_pass": require_pass,
        "ci_ready": exit_code == 0,
        "artifacts": {
            "index": index_path,
            "manifest": manifest_path,
        },
        "checks": checks,
        "fail_count": fail_count,
        "index": index_report,
        "manifest": manifest_report,
    })
}

const EMNR_MANIFEST_COMPARE_EPSILON: f64 = 0.000_01;

fn json_delta(candidate: &JsonValue, baseline: &JsonValue, key: &str) -> f64 {
    candidate[key].as_f64().unwrap_or_default() - baseline[key].as_f64().unwrap_or_default()
}

fn status_rank(status: &str) -> i32 {
    match status {
        "pass" | "promote" | "trained" => 3,
        "warn" => 2,
        "fail" | "reject" | "partial" => 1,
        _ => 0,
    }
}

fn push_manifest_check(
    checks: &mut Vec<JsonValue>,
    counter: &mut u64,
    name: &str,
    passed: bool,
    severity: &str,
    message: &str,
) {
    if !passed {
        *counter += 1;
    }
    checks.push(json!({
        "name": name,
        "status": if passed { "pass" } else { severity },
        "message": message,
    }));
}

fn emnr_gate_status(stream: &EmnrStreamReport, policy: &EmnrGatePolicy) -> &'static str {
    if stream.failed_count > 0
        || stream.isolate_count > 0
        || stream.final_energy < policy.critical_energy
    {
        "fail"
    } else if stream.throttle_count > 0
        || stream.observe_count > 0
        || stream.mean_risk >= policy.risk_threshold
        || stream.mean_confidence < policy.confidence_threshold
        || stream.final_energy < policy.energy_threshold
    {
        "warn"
    } else {
        "pass"
    }
}

fn emnr_promote_status(
    baseline: &EmnrStreamReport,
    candidate: &EmnrStreamReport,
    options: &EmnrPromoteOptions,
    risk_delta: f64,
    error_delta: f64,
    confidence_delta: f64,
) -> &'static str {
    if baseline.failed_count > 0
        || candidate.failed_count > 0
        || candidate.isolate_count > 0
        || candidate.final_energy < options.policy.min_energy
        || risk_delta > options.policy.max_risk_regression
        || error_delta > options.policy.max_error_regression
        || confidence_delta < -options.policy.max_confidence_drop
    {
        "reject"
    } else {
        "promote"
    }
}

fn emnr_stream_metrics_json(stream: &EmnrStreamReport) -> JsonValue {
    json!({
        "failed_count": stream.failed_count,
        "isolate_count": stream.isolate_count,
        "throttle_count": stream.throttle_count,
        "observe_count": stream.observe_count,
        "mean_confidence": stream.mean_confidence,
        "mean_risk": stream.mean_risk,
        "mean_reward": stream.mean_reward,
        "mean_prediction_error": stream.mean_prediction_error,
        "final_energy": stream.final_energy,
        "memory_records": stream.memory_records,
    })
}

fn emnr_promote_reason(
    failures: u64,
    risk_delta: f64,
    error_delta: f64,
    confidence_delta: f64,
    candidate_energy: f32,
    options: &EmnrPromoteOptions,
) -> String {
    if failures > 0 {
        format!("{failures} validation run(s) failed")
    } else if candidate_energy < options.policy.min_energy {
        format!(
            "candidate energy {:.2} is below promotion minimum",
            candidate_energy
        )
    } else if risk_delta > options.policy.max_risk_regression {
        format!("candidate risk regressed by {:.4}", risk_delta)
    } else if error_delta > options.policy.max_error_regression {
        format!("candidate prediction error regressed by {:.4}", error_delta)
    } else if confidence_delta < -options.policy.max_confidence_drop {
        format!("candidate confidence dropped by {:.4}", -confidence_delta)
    } else {
        "candidate EMNR state passed promotion policy".to_string()
    }
}

fn emnr_audit_report(reports: &[(String, JsonValue)], fail_on_warn: bool) -> JsonValue {
    let mut items = Vec::new();
    let mut pass_count = 0u64;
    let mut warn_count = 0u64;
    let mut fail_count = 0u64;
    let mut total_energy = 0.0f64;
    let mut energy_count = 0u64;
    let mut total_confidence = 0.0f64;
    let mut confidence_count = 0u64;
    let mut total_risk = 0.0f64;
    let mut risk_count = 0u64;
    let mut total_prediction_error = 0.0f64;
    let mut prediction_error_count = 0u64;

    for (path, report) in reports {
        let kind = emnr_report_kind(report);
        let raw_status = report.get("status").and_then(JsonValue::as_str).unwrap_or(
            if report.get("ok").and_then(JsonValue::as_bool) == Some(true) {
                "pass"
            } else {
                "fail"
            },
        );
        let normalized_status = emnr_audit_status(raw_status, report);
        match normalized_status {
            "fail" => fail_count += 1,
            "warn" => warn_count += 1,
            _ => pass_count += 1,
        }

        let metrics = emnr_audit_metrics(report);
        if let Some(value) = metrics.get("energy").and_then(JsonValue::as_f64) {
            total_energy += value;
            energy_count += 1;
        }
        if let Some(value) = metrics.get("confidence").and_then(JsonValue::as_f64) {
            total_confidence += value;
            confidence_count += 1;
        }
        if let Some(value) = metrics.get("risk").and_then(JsonValue::as_f64) {
            total_risk += value;
            risk_count += 1;
        }
        if let Some(value) = metrics.get("prediction_error").and_then(JsonValue::as_f64) {
            total_prediction_error += value;
            prediction_error_count += 1;
        }

        items.push(json!({
            "path": path,
            "kind": kind,
            "status": normalized_status,
            "raw_status": raw_status,
            "ok": report.get("ok").and_then(JsonValue::as_bool).unwrap_or(false),
            "metrics": metrics,
            "reason": report.get("reason").cloned().unwrap_or(JsonValue::Null),
        }));
    }

    let overall_status = if fail_count > 0 || (fail_on_warn && warn_count > 0) {
        "fail"
    } else if warn_count > 0 {
        "warn"
    } else {
        "pass"
    };
    let report_count = reports.len() as f64;
    let health_score = if report_count == 0.0 {
        0.0
    } else {
        ((pass_count as f64 + warn_count as f64 * 0.5) / report_count * 100.0).round()
    };
    json!({
        "ok": overall_status != "fail",
        "status": overall_status,
        "exit_code": if overall_status == "fail" { 1 } else { 0 },
        "fail_on_warn": fail_on_warn,
        "reports": reports.len(),
        "health_score": health_score,
        "summary": {
            "pass": pass_count,
            "warn": warn_count,
            "fail": fail_count,
            "mean_energy": mean_or_null(total_energy, energy_count),
            "mean_confidence": mean_or_null(total_confidence, confidence_count),
            "mean_risk": mean_or_null(total_risk, risk_count),
            "mean_prediction_error": mean_or_null(total_prediction_error, prediction_error_count),
        },
        "items": items,
    })
}

fn emnr_report_kind(report: &JsonValue) -> &'static str {
    if report.get("emnr_stream").is_some() {
        "gate"
    } else if report.get("epoch_reports").is_some() {
        "train"
    } else if report
        .get("metrics")
        .and_then(|metrics| metrics.get("delta"))
        .is_some()
    {
        "promote"
    } else if report.get("regions").is_some() && report.get("field").is_some() {
        "state"
    } else {
        "unknown"
    }
}

fn emnr_audit_status<'a>(raw_status: &'a str, report: &JsonValue) -> &'a str {
    match raw_status {
        "fail" | "reject" | "partial" => "fail",
        "warn" => "warn",
        "pass" | "trained" | "promote" => "pass",
        _ if report.get("ok").and_then(JsonValue::as_bool) == Some(true) => "pass",
        _ => "fail",
    }
}

fn emnr_audit_metrics(report: &JsonValue) -> JsonValue {
    let kind = emnr_report_kind(report);
    match kind {
        "gate" => json!({
            "energy": report["emnr_stream"]["summary"]["final_energy"].clone(),
            "confidence": report["emnr_stream"]["summary"]["mean_confidence"].clone(),
            "risk": report["emnr_stream"]["summary"]["mean_risk"].clone(),
            "reward": report["emnr_stream"]["summary"]["mean_reward"].clone(),
            "memory_records": report["emnr_stream"]["summary"]["memory_records"].clone(),
        }),
        "train" => json!({
            "energy": report["finished"]["energy"].clone(),
            "confidence_delta": report["learning_delta"]["confidence"].clone(),
            "risk_delta": report["learning_delta"]["risk"].clone(),
            "prediction_error_delta": report["learning_delta"]["prediction_error"].clone(),
            "memory_records": report["finished"]["memory_records"].clone(),
        }),
        "promote" => json!({
            "energy": report["metrics"]["candidate"]["final_energy"].clone(),
            "confidence": report["metrics"]["candidate"]["mean_confidence"].clone(),
            "risk": report["metrics"]["candidate"]["mean_risk"].clone(),
            "prediction_error": report["metrics"]["candidate"]["mean_prediction_error"].clone(),
            "confidence_delta": report["metrics"]["delta"]["confidence"].clone(),
            "risk_delta": report["metrics"]["delta"]["risk"].clone(),
            "prediction_error_delta": report["metrics"]["delta"]["prediction_error"].clone(),
            "memory_records": report["metrics"]["candidate"]["memory_records"].clone(),
        }),
        "state" => json!({
            "energy": report["energy"]["current"].clone(),
            "memory_records": report["memory"]["records"].clone(),
            "tick": report["tick"].clone(),
        }),
        _ => json!({}),
    }
}

fn mean_or_null(sum: f64, count: u64) -> JsonValue {
    if count == 0 {
        JsonValue::Null
    } else {
        json!(sum / count as f64)
    }
}

fn parse_emnr_gate_args(args: &[String], warn_as_fail: bool) -> EmnrGateOptions {
    let mut paths = Vec::new();
    let mut policy = EmnrGatePolicy {
        warn_as_fail,
        ..EmnrGatePolicy::default()
    };
    let mut output = None;
    let mut state_input = None;
    let mut state_output = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--warn-as-fail" => i += 1,
            "--policy" => {
                policy = load_emnr_gate_policy(args, i, policy);
                i += 2;
            }
            "--out" => {
                output = Some(parse_string_flag(args, i, "--out"));
                i += 2;
            }
            "--state-in" => {
                state_input = Some(parse_string_flag(args, i, "--state-in"));
                i += 2;
            }
            "--state-out" => {
                state_output = Some(parse_string_flag(args, i, "--state-out"));
                i += 2;
            }
            "--risk-threshold" => {
                policy.risk_threshold = parse_f64_flag(args, i, "--risk-threshold").clamp(0.0, 1.0);
                i += 2;
            }
            "--confidence-threshold" => {
                policy.confidence_threshold =
                    parse_f64_flag(args, i, "--confidence-threshold").clamp(0.0, 1.0);
                i += 2;
            }
            "--energy-threshold" => {
                policy.energy_threshold =
                    parse_f32_flag(args, i, "--energy-threshold").clamp(0.0, 100.0);
                i += 2;
            }
            "--critical-energy" => {
                policy.critical_energy =
                    parse_f32_flag(args, i, "--critical-energy").clamp(0.0, 100.0);
                i += 2;
            }
            value if value.starts_with("--") => {
                eprintln!("Usage error: unknown EMNR gate flag '{}'", value);
                process::exit(1);
            }
            value => {
                paths.push(value.to_string());
                i += 1;
            }
        }
    }
    EmnrGateOptions {
        paths,
        policy,
        output,
        state_input,
        state_output,
    }
}

fn parse_emnr_train_args(args: &[String]) -> EmnrTrainOptions {
    let mut paths = Vec::new();
    let mut epochs = 1usize;
    let mut output = None;
    let mut state_input = None;
    let mut state_output = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--epochs" => {
                epochs = parse_usize_flag(args, i, "--epochs").max(1);
                i += 2;
            }
            "--out" => {
                output = Some(parse_string_flag(args, i, "--out"));
                i += 2;
            }
            "--state-in" => {
                state_input = Some(parse_string_flag(args, i, "--state-in"));
                i += 2;
            }
            "--state-out" => {
                state_output = Some(parse_string_flag(args, i, "--state-out"));
                i += 2;
            }
            value if value.starts_with("--") => {
                eprintln!("Usage error: unknown EMNR train flag '{}'", value);
                process::exit(1);
            }
            value => {
                paths.push(value.to_string());
                i += 1;
            }
        }
    }
    EmnrTrainOptions {
        paths,
        epochs,
        output,
        state_input,
        state_output,
    }
}

fn parse_emnr_promote_args(args: &[String]) -> EmnrPromoteOptions {
    let mut paths = Vec::new();
    let mut baseline_state = None;
    let mut candidate_state = None;
    let mut output = None;
    let mut policy = EmnrPromotePolicy::default();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--baseline-state" => {
                baseline_state = Some(parse_string_flag(args, i, "--baseline-state"));
                i += 2;
            }
            "--candidate-state" => {
                candidate_state = Some(parse_string_flag(args, i, "--candidate-state"));
                i += 2;
            }
            "--out" => {
                output = Some(parse_string_flag(args, i, "--out"));
                i += 2;
            }
            "--max-risk-regression" => {
                policy.max_risk_regression =
                    parse_f64_flag(args, i, "--max-risk-regression").clamp(0.0, 1.0);
                i += 2;
            }
            "--max-error-regression" => {
                policy.max_error_regression =
                    parse_f64_flag(args, i, "--max-error-regression").clamp(0.0, 1.0);
                i += 2;
            }
            "--max-confidence-drop" => {
                policy.max_confidence_drop =
                    parse_f64_flag(args, i, "--max-confidence-drop").clamp(0.0, 1.0);
                i += 2;
            }
            "--min-energy" => {
                policy.min_energy = parse_f32_flag(args, i, "--min-energy").clamp(0.0, 100.0);
                i += 2;
            }
            value if value.starts_with("--") => {
                eprintln!("Usage error: unknown EMNR promote flag '{}'", value);
                process::exit(1);
            }
            value => {
                paths.push(value.to_string());
                i += 1;
            }
        }
    }
    EmnrPromoteOptions {
        paths,
        baseline_state: baseline_state.unwrap_or_else(|| {
            eprintln!("Usage error: --baseline-state is required");
            process::exit(1);
        }),
        candidate_state: candidate_state.unwrap_or_else(|| {
            eprintln!("Usage error: --candidate-state is required");
            process::exit(1);
        }),
        output,
        policy,
    }
}

fn parse_emnr_audit_args(args: &[String]) -> EmnrAuditOptions {
    let mut paths = Vec::new();
    let mut output = None;
    let mut fail_on_warn = false;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                output = Some(parse_string_flag(args, i, "--out"));
                i += 2;
            }
            "--fail-on-warn" => {
                fail_on_warn = true;
                i += 1;
            }
            value if value.starts_with("--") => {
                eprintln!("Usage error: unknown EMNR audit flag '{}'", value);
                process::exit(1);
            }
            value => {
                paths.push(value.to_string());
                i += 1;
            }
        }
    }
    EmnrAuditOptions {
        paths,
        output,
        fail_on_warn,
    }
}

fn parse_emnr_pipeline_args(args: &[String]) -> EmnrPipelineOptions {
    let mut paths = Vec::new();
    let mut epochs = 2usize;
    let mut out_dir = ".build-artifacts/emnr-pipeline".to_string();
    let mut fail_on_warn = false;
    let mut gate_policy = EmnrGatePolicy::default();
    let mut promote_policy = EmnrPromotePolicy::default();
    let mut policy_path = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--epochs" => {
                epochs = parse_usize_flag(args, i, "--epochs").max(1);
                i += 2;
            }
            "--out-dir" => {
                out_dir = parse_string_flag(args, i, "--out-dir");
                i += 2;
            }
            "--policy" => {
                let path = parse_string_flag(args, i, "--policy");
                let value = load_json_file_or_exit(&path, "EMNR pipeline policy");
                apply_emnr_pipeline_policy_json(&mut gate_policy, &mut promote_policy, &value);
                policy_path = Some(path);
                i += 2;
            }
            "--fail-on-warn" => {
                fail_on_warn = true;
                i += 1;
            }
            value if value.starts_with("--") => {
                eprintln!("Usage error: unknown EMNR pipeline flag '{}'", value);
                process::exit(1);
            }
            value => {
                paths.push(value.to_string());
                i += 1;
            }
        }
    }
    EmnrPipelineOptions {
        paths,
        epochs,
        out_dir,
        fail_on_warn,
        gate_policy,
        promote_policy,
        policy_path,
    }
}

fn parse_emnr_ci_args(args: &[String]) -> EmnrCiOptions {
    let mut pipeline_args = Vec::new();
    let mut baseline_manifest = None;
    let mut baseline_index = None;
    let mut require_non_regression = false;
    let mut require_pass = false;
    let mut output = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--baseline-manifest" => {
                baseline_manifest = Some(parse_string_flag(args, i, "--baseline-manifest"));
                i += 2;
            }
            "--baseline-index" => {
                baseline_index = Some(parse_string_flag(args, i, "--baseline-index"));
                i += 2;
            }
            "--require-non-regression" => {
                require_non_regression = true;
                i += 1;
            }
            "--require-pass" => {
                require_pass = true;
                i += 1;
            }
            "--out" => {
                output = Some(parse_string_flag(args, i, "--out"));
                i += 2;
            }
            value => {
                pipeline_args.push(value.to_string());
                if matches!(value, "--epochs" | "--out-dir" | "--policy") && i + 1 < args.len() {
                    i += 1;
                    pipeline_args.push(args[i].clone());
                }
                i += 1;
            }
        }
    }
    EmnrCiOptions {
        pipeline: parse_emnr_pipeline_args(&pipeline_args),
        baseline_manifest,
        baseline_index,
        require_non_regression,
        require_pass,
        output,
    }
}

fn parse_emnr_baseline_promote_args(args: &[String]) -> EmnrBaselinePromoteOptions {
    let manifest = args.first().cloned().unwrap_or_else(|| {
        eprintln!("Usage error: manifest path is required");
        process::exit(1);
    });
    let mut baseline_output = None;
    let mut require_pass = true;
    let mut index_output = None;
    let mut output = None;
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--baseline-out" => {
                baseline_output = Some(parse_string_flag(args, i, "--baseline-out"));
                i += 2;
            }
            "--index-out" => {
                index_output = Some(parse_string_flag(args, i, "--index-out"));
                i += 2;
            }
            "--allow-warn" => {
                require_pass = false;
                i += 1;
            }
            "--out" => {
                output = Some(parse_string_flag(args, i, "--out"));
                i += 2;
            }
            value => {
                eprintln!(
                    "Usage error: unknown EMNR baseline promotion flag '{}'",
                    value
                );
                process::exit(1);
            }
        }
    }
    EmnrBaselinePromoteOptions {
        manifest,
        baseline_output: baseline_output.unwrap_or_else(|| {
            eprintln!("Usage error: --baseline-out is required");
            process::exit(1);
        }),
        require_pass,
        index_output,
        output,
    }
}

fn load_emnr_gate_policy(
    args: &[String],
    index: usize,
    mut policy: EmnrGatePolicy,
) -> EmnrGatePolicy {
    let path = parse_string_flag(args, index, "--policy");
    let value = load_json_file_or_exit(&path, "EMNR gate policy");
    apply_emnr_gate_policy_json(&mut policy, &value);
    policy
}

fn load_json_file_or_exit(path: &str, label: &str) -> JsonValue {
    let raw = fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("Error reading {label} '{}': {}", path, error);
        process::exit(1);
    });
    serde_json::from_str::<JsonValue>(&raw).unwrap_or_else(|error| {
        eprintln!("Error parsing {label} '{}': {}", path, error);
        process::exit(1);
    })
}

fn apply_emnr_gate_policy_json(policy: &mut EmnrGatePolicy, value: &JsonValue) {
    let value = value.get("gate").unwrap_or(value);
    if let Some(raw) = value.get("warn_as_fail").and_then(JsonValue::as_bool) {
        policy.warn_as_fail = raw;
    }
    if let Some(raw) = value.get("risk_threshold").and_then(JsonValue::as_f64) {
        policy.risk_threshold = raw.clamp(0.0, 1.0);
    }
    if let Some(raw) = value
        .get("confidence_threshold")
        .and_then(JsonValue::as_f64)
    {
        policy.confidence_threshold = raw.clamp(0.0, 1.0);
    }
    if let Some(raw) = value.get("energy_threshold").and_then(JsonValue::as_f64) {
        policy.energy_threshold = (raw as f32).clamp(0.0, 100.0);
    }
    if let Some(raw) = value.get("critical_energy").and_then(JsonValue::as_f64) {
        policy.critical_energy = (raw as f32).clamp(0.0, 100.0);
    }
}

fn apply_emnr_promote_policy_json(policy: &mut EmnrPromotePolicy, value: &JsonValue) {
    let value = value
        .get("promotion")
        .or_else(|| value.get("promote"))
        .unwrap_or(value);
    if let Some(raw) = value.get("max_risk_regression").and_then(JsonValue::as_f64) {
        policy.max_risk_regression = raw.clamp(0.0, 1.0);
    }
    if let Some(raw) = value
        .get("max_error_regression")
        .and_then(JsonValue::as_f64)
    {
        policy.max_error_regression = raw.clamp(0.0, 1.0);
    }
    if let Some(raw) = value.get("max_confidence_drop").and_then(JsonValue::as_f64) {
        policy.max_confidence_drop = raw.clamp(0.0, 1.0);
    }
    if let Some(raw) = value.get("min_energy").and_then(JsonValue::as_f64) {
        policy.min_energy = (raw as f32).clamp(0.0, 100.0);
    }
}

fn apply_emnr_pipeline_policy_json(
    gate_policy: &mut EmnrGatePolicy,
    promote_policy: &mut EmnrPromotePolicy,
    value: &JsonValue,
) {
    apply_emnr_gate_policy_json(gate_policy, value);
    apply_emnr_promote_policy_json(promote_policy, value);
}

fn parse_string_flag(args: &[String], index: usize, name: &str) -> String {
    if index + 1 >= args.len() {
        eprintln!("Usage error: {name} requires a value");
        process::exit(1);
    }
    args[index + 1].clone()
}

fn parse_f64_flag(args: &[String], index: usize, name: &str) -> f64 {
    if index + 1 >= args.len() {
        eprintln!("Usage error: {name} requires a numeric value");
        process::exit(1);
    }
    args[index + 1].parse::<f64>().unwrap_or_else(|_| {
        eprintln!("Usage error: {name} must be numeric");
        process::exit(1);
    })
}

fn parse_f32_flag(args: &[String], index: usize, name: &str) -> f32 {
    parse_f64_flag(args, index, name) as f32
}

fn parse_usize_flag(args: &[String], index: usize, name: &str) -> usize {
    if index + 1 >= args.len() {
        eprintln!("Usage error: {name} requires a numeric value");
        process::exit(1);
    }
    args[index + 1].parse::<usize>().unwrap_or_else(|_| {
        eprintln!("Usage error: {name} must be a positive integer");
        process::exit(1);
    })
}

fn collect_emnr_stream(paths: &[String]) -> EmnrStreamReport {
    let mut session = MatterBridgeSession::new();
    collect_emnr_stream_with_session(paths, &mut session)
}

fn collect_emnr_stream_with_session(
    paths: &[String],
    session: &mut MatterBridgeSession,
) -> EmnrStreamReport {
    let mut runs = Vec::new();
    let mut decision_counts = BTreeMap::<String, u64>::new();
    let mut ok_count = 0u64;
    let mut failed_count = 0u64;
    let mut confidence_sum = 0.0f64;
    let mut risk_sum = 0.0f64;
    let mut reward_sum = 0.0f64;
    let mut prediction_error_sum = 0.0f64;

    for path in paths {
        let (payload, ok) = emnr_run_one_json(path, session);
        if ok {
            ok_count += 1;
        } else {
            failed_count += 1;
        }

        if let Some(emnr) = payload.get("emnr") {
            if let Some(decision) = emnr.get("decision").and_then(JsonValue::as_str) {
                *decision_counts.entry(decision.to_string()).or_default() += 1;
            }
            confidence_sum += emnr
                .get("confidence")
                .and_then(JsonValue::as_f64)
                .unwrap_or_default();
            risk_sum += emnr
                .get("risk_score")
                .and_then(JsonValue::as_f64)
                .unwrap_or_default();
            reward_sum += emnr
                .get("reward_score")
                .and_then(JsonValue::as_f64)
                .unwrap_or_default();
            prediction_error_sum += emnr
                .get("prediction_error")
                .and_then(JsonValue::as_f64)
                .unwrap_or_default();
        }
        runs.push(payload);
    }

    let total = runs.len() as f64;
    let mean_confidence = if total == 0.0 {
        0.0
    } else {
        confidence_sum / total
    };
    let mean_risk = if total == 0.0 { 0.0 } else { risk_sum / total };
    let mean_reward = if total == 0.0 {
        0.0
    } else {
        reward_sum / total
    };
    let mean_prediction_error = if total == 0.0 {
        0.0
    } else {
        prediction_error_sum / total
    };
    let final_energy = session.runtime.energy.current;
    let memory_records = session.runtime.memory.records.len();
    let isolate_count = *decision_counts.get("IsolateRisk").unwrap_or(&0);
    let throttle_count = *decision_counts.get("ThrottleRuntime").unwrap_or(&0);
    let observe_count = *decision_counts.get("ObserveOnly").unwrap_or(&0);
    let report = json!({
        "ok": failed_count == 0,
        "files": runs.len(),
        "runs": runs,
        "summary": {
            "ok_count": ok_count,
            "failed_count": failed_count,
            "decision_counts": decision_counts,
            "mean_confidence": mean_confidence,
            "mean_risk": mean_risk,
            "mean_reward": mean_reward,
            "mean_prediction_error": mean_prediction_error,
            "final_energy": final_energy,
            "memory_records": memory_records,
            "field": session.runtime.field,
        }
    });

    EmnrStreamReport {
        report,
        failed_count,
        isolate_count,
        throttle_count,
        observe_count,
        mean_risk,
        mean_reward,
        mean_confidence,
        mean_prediction_error,
        final_energy,
        memory_records,
    }
}

fn load_emnr_gate_state(path: &str) -> MatterBridgeSession {
    let raw = fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("Error reading EMNR state '{}': {}", path, error);
        process::exit(1);
    });
    serde_json::from_str::<MatterBridgeSession>(&raw).unwrap_or_else(|error| {
        eprintln!("Error parsing EMNR state '{}': {}", path, error);
        process::exit(1);
    })
}

fn save_emnr_gate_state(path: &str, session: &MatterBridgeSession) {
    let raw = serde_json::to_string_pretty(session).unwrap_or_else(|error| {
        eprintln!("Error serializing EMNR state '{}': {}", path, error);
        process::exit(1);
    });
    if let Err(error) = fs::write(path, raw) {
        eprintln!("Error writing EMNR state '{}': {}", path, error);
        process::exit(1);
    }
}

fn save_emnr_gate_state_path(path: &Path, session: &MatterBridgeSession) {
    let raw = serde_json::to_string_pretty(session).unwrap_or_else(|error| {
        eprintln!(
            "Error serializing EMNR state '{}': {}",
            path.display(),
            error
        );
        process::exit(1);
    });
    if let Err(error) = fs::write(path, raw) {
        eprintln!("Error writing EMNR state '{}': {}", path.display(), error);
        process::exit(1);
    }
}

fn write_json_report_path(path: &Path, report: &JsonValue, label: &str) {
    let raw = serde_json::to_string(report).unwrap_or_else(|error| {
        eprintln!("Error serializing {label}: {error}");
        process::exit(1);
    });
    if let Err(error) = fs::write(path, raw) {
        eprintln!("Error writing {label} '{}': {}", path.display(), error);
        process::exit(1);
    }
}

fn emnr_state_json(path: &str, full: bool) {
    let session = load_emnr_gate_state(path);
    let runtime = &session.runtime;
    let summary = json!({
        "ok": true,
        "input": path,
        "tick": runtime.tick,
        "energy": {
            "current": runtime.energy.current,
            "capacity": runtime.energy.capacity,
            "fatigue": runtime.energy.fatigue,
            "pressure": runtime.energy.pressure(),
        },
        "memory": {
            "records": runtime.memory.records.len(),
            "max_records": runtime.memory.max_records,
            "top_records": runtime.memory.records.iter().rev().take(5).map(|record| {
                json!({
                    "id": record.id,
                    "label": record.label,
                    "emotional_weight": record.emotional_weight,
                    "access_count": record.access_count,
                    "confidence": record.confidence,
                    "created_at": record.created_at,
                    "last_access": record.last_access,
                })
            }).collect::<Vec<_>>(),
        },
        "field": runtime.field,
        "regions": {
            "attention": emnr_region_state_json(&runtime.attention),
            "prediction": emnr_region_state_json(&runtime.prediction),
            "emotion": emnr_region_state_json(&runtime.emotion),
            "memory": emnr_region_state_json(&runtime.memory_region),
            "action": emnr_region_state_json(&runtime.action),
        },
        "has_last_prediction": runtime.last_prediction.is_some(),
        "full_state": if full {
            serde_json::to_value(&session).unwrap_or(JsonValue::Null)
        } else {
            JsonValue::Null
        }
    });

    println!(
        "{}",
        serde_json::to_string(&summary).unwrap_or_else(|error| {
            format!(
                "{{\"ok\":false,\"stage\":\"serialize\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            )
        })
    );
}

fn emnr_region_state_json(region: &emnr_region::BrainRegion) -> JsonValue {
    json!({
        "id": region.id,
        "name": region.name,
        "layers": region.layers.len(),
        "molecules": region.layers.iter().map(|layer| layer.molecules.len()).sum::<usize>(),
        "local_energy": region.local_energy,
        "plasticity": region.plasticity,
        "state_len": region.state.len(),
        "state_energy": region.state.energy,
    })
}

fn emnr_gate_reason(stream: &EmnrStreamReport, status: &str, policy: &EmnrGatePolicy) -> String {
    if stream.failed_count > 0 {
        format!("{} Matter run(s) failed", stream.failed_count)
    } else if stream.isolate_count > 0 {
        format!(
            "{} EMNR decision(s) requested isolation",
            stream.isolate_count
        )
    } else if stream.final_energy < policy.critical_energy {
        format!("final EMNR energy {:.2} is critical", stream.final_energy)
    } else if stream.throttle_count > 0 {
        format!(
            "{} EMNR decision(s) requested runtime throttling",
            stream.throttle_count
        )
    } else if stream.observe_count > 0 {
        format!(
            "{} EMNR decision(s) requested observation before action",
            stream.observe_count
        )
    } else if stream.mean_risk >= policy.risk_threshold {
        format!(
            "mean risk {:.3} is above warning threshold",
            stream.mean_risk
        )
    } else if stream.mean_confidence < policy.confidence_threshold {
        format!(
            "mean confidence {:.3} is below warning threshold",
            stream.mean_confidence
        )
    } else if stream.final_energy < policy.energy_threshold {
        format!("final EMNR energy {:.2} is low", stream.final_energy)
    } else {
        format!("EMNR gate {status}: risk, confidence, and energy are within policy")
    }
}

fn emnr_run_one_json(path: &str, session: &mut MatterBridgeSession) -> (JsonValue, bool) {
    let started = Instant::now();
    let input = source_label(path).to_string();
    let source = match read_source_for_json(path) {
        Ok(source) => source,
        Err(error) => {
            let decision = emnr_decision_for_event(
                session,
                MatterRuntimeEvent {
                    module: input.clone(),
                    phase: MatterPhase::Parse,
                    cpu_cost: 0.0,
                    memory_pressure: 0.0,
                    error_rate: 1.0,
                    latency_ms: started.elapsed().as_secs_f32() * 1_000.0,
                    cache_hit_rate: 0.0,
                    optimization_confidence: 0.0,
                    security_risk: 0.35,
                    novelty_score: 0.4,
                    label: Some("source_read_failed".to_string()),
                },
            );
            return (
                build_emnr_run_payload(false, "read", &input, &[], None, Some(&error), decision),
                false,
            );
        }
    };

    let mut parser = Parser::from_source(&source);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(error) => {
            let decision = emnr_decision_for_event(
                session,
                MatterRuntimeEvent {
                    module: input.clone(),
                    phase: MatterPhase::Parse,
                    cpu_cost: source_complexity_cost(&source),
                    memory_pressure: source_memory_pressure(&source),
                    error_rate: 0.85,
                    latency_ms: started.elapsed().as_secs_f32() * 1_000.0,
                    cache_hit_rate: 0.15,
                    optimization_confidence: 0.0,
                    security_risk: 0.25,
                    novelty_score: 0.65,
                    label: Some("parse_failed".to_string()),
                },
            );
            return (
                build_emnr_run_payload(
                    false,
                    "parse",
                    &input,
                    &[],
                    None,
                    Some(&format!("{} at {}:{}", error, error.line, error.column)),
                    decision,
                ),
                false,
            );
        }
    };

    let builder = BytecodeBuilder::new();
    let bytecode = match builder.build_checked(&program) {
        Ok(bytecode) => bytecode,
        Err(error) => {
            let decision = emnr_decision_for_event(
                session,
                MatterRuntimeEvent {
                    module: input.clone(),
                    phase: MatterPhase::Compile,
                    cpu_cost: source_complexity_cost(&source),
                    memory_pressure: source_memory_pressure(&source),
                    error_rate: 0.75,
                    latency_ms: started.elapsed().as_secs_f32() * 1_000.0,
                    cache_hit_rate: 0.25,
                    optimization_confidence: 0.0,
                    security_risk: 0.20,
                    novelty_score: 0.55,
                    label: Some("semantic_failed".to_string()),
                },
            );
            return (
                build_emnr_run_payload(
                    false,
                    "semantic",
                    &input,
                    &[],
                    None,
                    Some(&error.to_string()),
                    decision,
                ),
                false,
            );
        }
    };

    let bytecode_stats = BytecodeStats::from_bytecode(&bytecode);
    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    let run_result = runtime.run();
    let output = runtime.take_output();
    let instruction_cost = runtime.vm().estimated_instruction_cost();
    let backend_cost = runtime.vm().estimated_backend_cost();
    let latency_ms = started.elapsed().as_secs_f32() * 1_000.0;

    match run_result {
        Ok(()) => {
            let decision = emnr_decision_for_event(
                session,
                MatterRuntimeEvent {
                    module: input.clone(),
                    phase: MatterPhase::Execute,
                    cpu_cost: instruction_cost.clamp(0.0, 250.0) as f32,
                    memory_pressure: bytecode_stats.memory_pressure(),
                    error_rate: 0.0,
                    latency_ms,
                    cache_hit_rate: bytecode_stats.cache_hit_rate(),
                    optimization_confidence: bytecode_stats
                        .optimization_confidence(instruction_cost),
                    security_risk: backend_security_risk(backend_cost),
                    novelty_score: bytecode_stats.novelty_score(),
                    label: Some("matter_execution_ok".to_string()),
                },
            );
            (
                build_emnr_run_payload(
                    true,
                    "runtime",
                    &input,
                    &output,
                    Some((instruction_cost, backend_cost, bytecode_stats)),
                    None,
                    decision,
                ),
                true,
            )
        }
        Err(error) => {
            let decision = emnr_decision_for_event(
                session,
                MatterRuntimeEvent {
                    module: input.clone(),
                    phase: MatterPhase::Execute,
                    cpu_cost: instruction_cost.clamp(0.0, 250.0) as f32,
                    memory_pressure: bytecode_stats.memory_pressure(),
                    error_rate: 1.0,
                    latency_ms,
                    cache_hit_rate: 0.05,
                    optimization_confidence: 0.0,
                    security_risk: 0.45,
                    novelty_score: 0.75,
                    label: Some("matter_execution_failed".to_string()),
                },
            );
            (
                build_emnr_run_payload(
                    false,
                    "runtime",
                    &input,
                    &output,
                    Some((instruction_cost, backend_cost, bytecode_stats)),
                    Some(&error),
                    decision,
                ),
                false,
            )
        }
    }
}

fn run_energy(path: &str) {
    let source = read_source_or_exit(path);
    let mut parser = Parser::from_source(&source);
    let program = parser.parse().unwrap_or_else(|e| {
        print_parse_error(&source, &e);
        process::exit(1);
    });

    let builder = BytecodeBuilder::new();
    let bytecode = build_checked_or_exit(builder, &program);
    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.run() {
        eprintln!("Runtime error: {}", error);
        process::exit(1);
    }

    for line in runtime.take_output() {
        println!("{}", line);
    }

    println!(
        "[energy] instruction_cost={:.2} backend_cost={:.2}",
        runtime.vm().estimated_instruction_cost(),
        runtime.vm().estimated_backend_cost()
    );
}

fn run_energy_json(path: &str) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let bytecode = build_json_or_exit(&source, input, &[]);

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.run() {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{},\"energy\":{{\"instruction_cost\":{:.2},\"backend_cost\":{:.2}}}}}",
            json_escape(input),
            json_escape(&error),
            json_string_array(&runtime.take_output()),
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"output\":{},\"energy\":{{\"instruction_cost\":{:.2},\"backend_cost\":{:.2}}}}}",
        json_escape(input),
        json_string_array(&runtime.take_output()),
        runtime.vm().estimated_instruction_cost(),
        runtime.vm().estimated_backend_cost()
    );
}

fn run_source_json(source: &str, input: &str, with_energy: bool) {
    let bytecode = build_json_or_exit(source, input, &[]);

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.run() {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
            json_escape(input),
            json_escape(&error),
            json_string_array(&runtime.take_output()),
            energy_json_fragment(
                with_energy,
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            )
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"output\":{}{}}}",
        json_escape(input),
        json_string_array(&runtime.take_output()),
        energy_json_fragment(
            with_energy,
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        )
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

fn emit_json(path: &str, event: &str, with_energy: bool) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let bytecode = build_json_or_exit(&source, input, &[("event", event)]);

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.emit_event(event) {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"input\":\"{}\",\"event\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
            json_escape(input),
            json_escape(event),
            json_escape(&error),
            json_string_array(&runtime.take_output()),
            energy_json_fragment(
                with_energy,
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            )
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"event\":\"{}\",\"output\":{}{}}}",
        json_escape(input),
        json_escape(event),
        json_string_array(&runtime.take_output()),
        energy_json_fragment(
            with_energy,
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        )
    );
}

fn tool_wire_validate_json(wire_file: &str) {
    let wire = match fs::read_to_string(wire_file) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"read\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(wire_file),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    let mut runtime = Runtime::new_silent(Bytecode::new());
    match runtime.tool_validate_wire(&wire) {
        Ok(value) => println!(
            "{{\"ok\":true,\"input\":\"{}\",\"result\":{}}}",
            json_escape(wire_file),
            value_to_json(&value)
        ),
        Err(error) => println!(
            "{{\"ok\":false,\"stage\":\"tool.validate_wire\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(wire_file),
            json_escape(&error)
        ),
    }
}

fn tool_wire_merge_json(left_wire_file: &str, right_wire_file: &str, strategy: Option<&str>) {
    let left_wire = match fs::read_to_string(left_wire_file) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"read-left\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(left_wire_file),
                json_escape(&error.to_string())
            );
            return;
        }
    };
    let right_wire = match fs::read_to_string(right_wire_file) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"read-right\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(right_wire_file),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    let mut runtime = Runtime::new_silent(Bytecode::new());
    match runtime.tool_merge_wire(&left_wire, &right_wire, strategy) {
        Ok(value) => println!(
            "{{\"ok\":true,\"left\":\"{}\",\"right\":\"{}\",\"strategy\":\"{}\",\"result\":{}}}",
            json_escape(left_wire_file),
            json_escape(right_wire_file),
            json_escape(strategy.unwrap_or("prefer_latest")),
            value_to_json(&value)
        ),
        Err(error) => println!(
            "{{\"ok\":false,\"stage\":\"tool.merge_wire\",\"left\":\"{}\",\"right\":\"{}\",\"strategy\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(left_wire_file),
            json_escape(right_wire_file),
            json_escape(strategy.unwrap_or("prefer_latest")),
            json_escape(&error)
        ),
    }
}

fn tool_frame_invoke_json(frame_payload_file: &str, wire_output: Option<&str>) {
    let source = match fs::read_to_string(frame_payload_file) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"read\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(frame_payload_file),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    let parsed: JsonValue = match serde_json::from_str(&source) {
        Ok(value) => value,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"parse\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(frame_payload_file),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    let payload = match json_to_backend_value(&parsed) {
        Ok(value) => value,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"convert\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(frame_payload_file),
                json_escape(&error)
            );
            return;
        }
    };

    let mut runtime = Runtime::new_silent(Bytecode::new());
    match runtime.tool_invoke_frame(payload) {
        Ok(value) => {
            let mut saved_wire = false;
            if let Some(path) = wire_output {
                if let Value::Map(entries) = &value {
                    if let Some(wire_value) = entries.get("wire") {
                        if let Ok(wire) = wire_value.as_string() {
                            if fs::write(path, wire).is_ok() {
                                saved_wire = true;
                            }
                        }
                    }
                }
            }
            println!(
                "{{\"ok\":true,\"input\":\"{}\",\"wireOutput\":{},\"result\":{}}}",
                json_escape(frame_payload_file),
                if saved_wire { "true" } else { "false" },
                value_to_json(&value)
            );
        }
        Err(error) => println!(
            "{{\"ok\":false,\"stage\":\"tool.invoke_frame\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(frame_payload_file),
            json_escape(&error)
        ),
    }
}

fn tool_frame_template_json(output_file: Option<&str>) {
    let template = json!({
        "from_name": "planner",
        "from_role": "planner",
        "to_name": "worker",
        "to_role": "worker",
        "task_id": "task-001",
        "state": "proposed",
        "goal": "Describe the task objective",
        "summary": "Provide current context and constraints",
        "next_action": "execute",
        "facts": ["fact-1", "fact-2"],
        "blockers": [],
        "requests": []
    });
    let encoded = template.to_string();

    if let Some(path) = output_file {
        match fs::write(path, &encoded) {
            Ok(_) => println!(
                "{{\"ok\":true,\"output\":\"{}\",\"template\":{}}}",
                json_escape(path),
                encoded
            ),
            Err(error) => println!(
                "{{\"ok\":false,\"stage\":\"write\",\"output\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(path),
                json_escape(&error.to_string())
            ),
        }
    } else {
        println!("{{\"ok\":true,\"template\":{}}}", encoded);
    }
}

fn tool_wire_extract_json(invoke_result_file: &str, wire_output: Option<&str>) {
    let source = match fs::read_to_string(invoke_result_file) {
        Ok(content) => content,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"read\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(invoke_result_file),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    let parsed: JsonValue = match serde_json::from_str(&source) {
        Ok(value) => value,
        Err(error) => {
            println!(
                "{{\"ok\":false,\"stage\":\"parse\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}}}}",
                json_escape(invoke_result_file),
                json_escape(&error.to_string())
            );
            return;
        }
    };

    let wire = parsed
        .get("result")
        .and_then(|result| result.get("wire"))
        .and_then(|wire| wire.as_str())
        .map(|wire| wire.to_string());

    let Some(wire) = wire else {
        println!(
            "{{\"ok\":false,\"stage\":\"extract\",\"input\":\"{}\",\"error\":{{\"message\":\"missing result.wire in invoke result\"}}}}",
            json_escape(invoke_result_file)
        );
        return;
    };

    let mut saved = false;
    if let Some(path) = wire_output {
        if fs::write(path, &wire).is_ok() {
            saved = true;
        }
    }

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"wireOutput\":{},\"wire\":\"{}\"}}",
        json_escape(invoke_result_file),
        if saved { "true" } else { "false" },
        json_escape(&wire)
    );
}

struct PipelineScoring {
    status_ok: f64,
    status_degraded: f64,
    status_unknown: f64,
    action_execute: f64,
    action_resolve_blockers: f64,
    action_none: f64,
    action_other: f64,
    penalty_latest: f64,
    penalty_blocked: f64,
    penalty_terminal: f64,
}

#[derive(Clone, Copy)]
enum PipelineEnergyMode {
    Eco,
    Balanced,
    Performance,
    Adaptive,
    Critical,
}

impl PipelineScoring {
    fn balanced() -> Self {
        Self {
            status_ok: 100.0,
            status_degraded: 55.0,
            status_unknown: 20.0,
            action_execute: 25.0,
            action_resolve_blockers: 8.0,
            action_none: 3.0,
            action_other: 5.0,
            penalty_latest: 6.0,
            penalty_blocked: 12.0,
            penalty_terminal: 8.0,
        }
    }

    fn conservative() -> Self {
        Self {
            status_ok: 100.0,
            status_degraded: 40.0,
            status_unknown: 15.0,
            action_execute: 20.0,
            action_resolve_blockers: 10.0,
            action_none: 2.0,
            action_other: 4.0,
            penalty_latest: 8.0,
            penalty_blocked: 15.0,
            penalty_terminal: 10.0,
        }
    }

    fn aggressive() -> Self {
        Self {
            status_ok: 100.0,
            status_degraded: 70.0,
            status_unknown: 25.0,
            action_execute: 30.0,
            action_resolve_blockers: 6.0,
            action_none: 4.0,
            action_other: 6.0,
            penalty_latest: 4.0,
            penalty_blocked: 8.0,
            penalty_terminal: 6.0,
        }
    }

    fn from_preset(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "balanced" => Some(Self::balanced()),
            "conservative" => Some(Self::conservative()),
            "aggressive" => Some(Self::aggressive()),
            _ => None,
        }
    }
}

fn normalize_energy_mode(input: &str) -> Option<PipelineEnergyMode> {
    match input.to_ascii_lowercase().as_str() {
        "eco" => Some(PipelineEnergyMode::Eco),
        "balanced" => Some(PipelineEnergyMode::Balanced),
        "performance" => Some(PipelineEnergyMode::Performance),
        "adaptive" => Some(PipelineEnergyMode::Adaptive),
        "critical" => Some(PipelineEnergyMode::Critical),
        _ => None,
    }
}

fn energy_mode_name(mode: PipelineEnergyMode) -> &'static str {
    match mode {
        PipelineEnergyMode::Eco => "eco",
        PipelineEnergyMode::Balanced => "balanced",
        PipelineEnergyMode::Performance => "performance",
        PipelineEnergyMode::Adaptive => "adaptive",
        PipelineEnergyMode::Critical => "critical",
    }
}

fn apply_energy_mode_scoring(base: &PipelineScoring, mode: PipelineEnergyMode) -> PipelineScoring {
    let mut scoring = PipelineScoring {
        status_ok: base.status_ok,
        status_degraded: base.status_degraded,
        status_unknown: base.status_unknown,
        action_execute: base.action_execute,
        action_resolve_blockers: base.action_resolve_blockers,
        action_none: base.action_none,
        action_other: base.action_other,
        penalty_latest: base.penalty_latest,
        penalty_blocked: base.penalty_blocked,
        penalty_terminal: base.penalty_terminal,
    };
    match mode {
        PipelineEnergyMode::Eco => {
            scoring.penalty_blocked += 5.0;
            scoring.penalty_terminal += 3.0;
            scoring.action_execute -= 3.0;
        }
        PipelineEnergyMode::Balanced => {}
        PipelineEnergyMode::Performance => {
            scoring.action_execute += 4.0;
            scoring.penalty_latest -= 1.5;
            scoring.penalty_terminal -= 1.0;
        }
        PipelineEnergyMode::Adaptive => {
            scoring.action_resolve_blockers += 2.0;
            scoring.penalty_blocked += 1.0;
        }
        PipelineEnergyMode::Critical => {
            scoring.penalty_blocked += 8.0;
            scoring.penalty_terminal += 6.0;
            scoring.action_execute -= 6.0;
            scoring.status_degraded -= 10.0;
        }
    }
    scoring
}

fn energy_policy_rationale(mode: PipelineEnergyMode) -> &'static str {
    match mode {
        PipelineEnergyMode::Eco => {
            "Eco mode increases penalties for expensive merge paths and reduces execute bias to preserve resources."
        }
        PipelineEnergyMode::Balanced => {
            "Balanced mode keeps neutral tradeoffs between throughput and resource efficiency."
        }
        PipelineEnergyMode::Performance => {
            "Performance mode boosts execute preference and lowers penalties to maximize throughput."
        }
        PipelineEnergyMode::Adaptive => {
            "Adaptive mode favors unblock-first behavior while keeping moderate energy penalties."
        }
        PipelineEnergyMode::Critical => {
            "Critical mode strongly penalizes costly paths and degraded states to minimize risk under constrained conditions."
        }
    }
}

fn energy_policy_factors(scoring: &PipelineScoring) -> JsonValue {
    json!({
        "status_ok": scoring.status_ok,
        "status_degraded": scoring.status_degraded,
        "status_unknown": scoring.status_unknown,
        "action_execute": scoring.action_execute,
        "action_resolve_blockers": scoring.action_resolve_blockers,
        "action_none": scoring.action_none,
        "action_other": scoring.action_other,
        "penalty_latest": scoring.penalty_latest,
        "penalty_blocked": scoring.penalty_blocked,
        "penalty_terminal": scoring.penalty_terminal
    })
}

fn recommended_energy_mode_for_ci_decision(
    decision: &str,
    current: PipelineEnergyMode,
) -> PipelineEnergyMode {
    match decision {
        "fail" => PipelineEnergyMode::Critical,
        "warn" => PipelineEnergyMode::Adaptive,
        _ => current,
    }
}

fn confidence_profile_thresholds(profile: &str) -> Option<(f64, f64)> {
    match profile.to_ascii_lowercase().as_str() {
        "strict" => Some((8.0, 16.0)),
        "balanced" => Some((5.0, 12.0)),
        "relaxed" => Some((3.0, 8.0)),
        _ => None,
    }
}

fn ci_reason_code(reason: &str) -> i32 {
    match reason {
        "healthy" => 0,
        "low_confidence" => 10,
        "catalog_hash_mismatch" => 20,
        "mkdir_failed" => 100,
        "write_frames_failed" => 110,
        "invoke_a_failed" => 120,
        "invoke_b_failed" => 121,
        "extract_wire_failed" => 130,
        "matched_fail_status" => 140,
        "strict_degraded" => 150,
        _ => 999,
    }
}

fn normalize_strategy(input: &str) -> Option<&'static str> {
    match input.to_ascii_lowercase().as_str() {
        "auto" => Some("auto"),
        "prefer_latest" | "latest" => Some("prefer_latest"),
        "prefer_blocked" | "blocked" => Some("prefer_blocked"),
        "prefer_terminal" | "terminal" => Some("prefer_terminal"),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments, clippy::format_in_format_args)]
fn tool_pipeline_demo_json(
    out_dir: &str,
    strict: bool,
    strategy: &str,
    compare_strategies: bool,
    artifact_manifest_json: bool,
    emit_contract_bundle: bool,
    apply_recommended_energy_mode: bool,
    next_cycle_apply_now: bool,
    emit_summary_md: bool,
    emit_github_step_summary: bool,
    summary_format: &str,
    fail_on_status: Option<&str>,
    ci_exit_codes: bool,
    dry_run: bool,
    artifact_prefix: &str,
    scoring: &PipelineScoring,
    energy_mode_override: Option<PipelineEnergyMode>,
    confidence_threshold_low: f64,
    confidence_threshold_high: f64,
    confidence_profile: &str,
    require_catalog_hash: Option<&str>,
    contract_bundle_baseline_path: Option<&str>,
    next_cycle_config_out: Option<&str>,
    next_cycle_max_hops: u64,
) {
    let base = PathBuf::from(out_dir);
    let contract_version = "1";
    let catalog_hash = current_ci_catalog_hash();
    if let Err(error) = fs::create_dir_all(&base) {
        println!(
            "{{\"ok\":false,\"stage\":\"mkdir\",\"outDir\":\"{}\",\"contractVersion\":\"{}\",\"catalogHash\":\"{}\",\"ciDecision\":\"fail\",\"ciDecisionReason\":\"mkdir_failed\",\"ciDecisionCode\":{},\"error\":{{\"message\":\"{}\"}}}}",
            json_escape(out_dir),
            json_escape(contract_version),
            json_escape(&catalog_hash),
            ci_reason_code("mkdir_failed"),
            json_escape(&error.to_string())
        );
        return;
    }

    let name = |suffix: &str| -> String {
        if artifact_prefix.is_empty() {
            suffix.to_string()
        } else {
            format!("{}_{}", artifact_prefix, suffix)
        }
    };
    let frame_a_path = base.join(name("frame_a.json"));
    let frame_b_path = base.join(name("frame_b.json"));
    let invoke_a_path = base.join(name("invoke_a.json"));
    let invoke_b_path = base.join(name("invoke_b.json"));
    let wire_a_path = base.join(name("wire_a.txt"));
    let wire_b_path = base.join(name("wire_b.txt"));
    let merged_path = base.join(name("merged.json"));
    let validate_a_path = base.join(name("validate_a.json"));
    let validate_b_path = base.join(name("validate_b.json"));
    let report_path = base.join(name("report.json"));
    let report_compare_path = base.join(name("report_compare.json"));
    let report_compare_dashboard_path = base.join(name("report_compare_dashboard.json"));
    let contract_baseline_path = base.join(name("contract_baseline.json"));
    let contract_candidate_path = base.join(name("contract_candidate.json"));
    let contract_bundle_path = base.join(name("contract_bundle.json"));
    let next_cycle_config_path = next_cycle_config_out
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join(name("next_cycle_config.json")));
    let next_cycle_apply_result_path = base.join(name("next_cycle_apply_result.json"));
    let artifact_manifest_path = base.join(name("artifact_manifest.json"));
    let summary_md_path = base.join(name("summary.md"));
    let summary_json_path = base.join(name("summary.json"));

    if let Some(required_hash) = require_catalog_hash {
        if required_hash != catalog_hash {
            println!(
                "{{\"ok\":false,\"stage\":\"catalog-hash-check\",\"outDir\":\"{}\",\"ciDecision\":\"fail\",\"ciDecisionReason\":\"catalog_hash_mismatch\",\"ciDecisionCode\":{},\"contractVersion\":\"{}\",\"catalogHash\":\"{}\",\"requiredCatalogHash\":\"{}\",\"error\":{{\"message\":\"catalog hash mismatch\"}}}}",
                json_escape(out_dir),
                ci_reason_code("catalog_hash_mismatch"),
                json_escape(contract_version),
                json_escape(&catalog_hash),
                json_escape(required_hash)
            );
            return;
        }
    }

    let frame_a = json!({
        "from_name": "planner",
        "from_role": "planner",
        "to_name": "worker",
        "to_role": "worker",
        "task_id": "pipeline-demo",
        "state": "proposed",
        "goal": "Execute tool pipeline demo",
        "summary": "First frame in ready state",
        "next_action": "execute",
        "facts": ["demo", "energy-aware"],
        "blockers": [],
        "requests": []
    });
    let frame_b = json!({
        "from_name": "planner",
        "from_role": "planner",
        "to_name": "worker",
        "to_role": "worker",
        "task_id": "pipeline-demo",
        "state": "blocked",
        "goal": "Execute tool pipeline demo",
        "summary": "Second frame blocked for merge strategy demo",
        "next_action": "resolve-blockers",
        "facts": ["demo", "blocked-branch"],
        "blockers": ["missing_api_key"],
        "requests": []
    });

    if fs::write(&frame_a_path, frame_a.to_string()).is_err()
        || fs::write(&frame_b_path, frame_b.to_string()).is_err()
    {
        println!(
            "{{\"ok\":false,\"stage\":\"write-frames\",\"outDir\":\"{}\",\"contractVersion\":\"{}\",\"catalogHash\":\"{}\",\"ciDecision\":\"fail\",\"ciDecisionReason\":\"write_frames_failed\",\"ciDecisionCode\":{}}}",
            json_escape(out_dir),
            json_escape(contract_version),
            json_escape(&catalog_hash),
            ci_reason_code("write_frames_failed")
        );
        return;
    }

    let mut runtime = Runtime::new_silent(Bytecode::new());
    let energy_mode = energy_mode_override
        .or_else(|| {
            runtime
                .energy_mode_backend()
                .ok()
                .as_deref()
                .and_then(normalize_energy_mode)
        })
        .unwrap_or(PipelineEnergyMode::Balanced);
    let (invoke_a, invoke_b) = if dry_run {
        (
            Value::new_map(std::collections::HashMap::from([
                ("ok".to_string(), Value::Bool(true)),
                (
                    "response_kind".to_string(),
                    Value::new_string("needs_context".to_string()),
                ),
                (
                    "wire".to_string(),
                    Value::new_string("status=ok\nnext_action=execute\n".to_string()),
                ),
            ])),
            Value::new_map(std::collections::HashMap::from([
                ("ok".to_string(), Value::Bool(true)),
                (
                    "response_kind".to_string(),
                    Value::new_string("blocked".to_string()),
                ),
                (
                    "wire".to_string(),
                    Value::new_string(
                        "status=degraded\nnext_action=resolve-blockers\n".to_string(),
                    ),
                ),
            ])),
        )
    } else {
        let invoke_a = match runtime
            .tool_invoke_frame(json_to_backend_value(&frame_a).unwrap_or(Value::Unit))
        {
            Ok(value) => value,
            Err(error) => {
                println!(
                    "{{\"ok\":false,\"stage\":\"invoke-a\",\"outDir\":\"{}\",\"contractVersion\":\"{}\",\"catalogHash\":\"{}\",\"ciDecision\":\"fail\",\"ciDecisionReason\":\"invoke_a_failed\",\"ciDecisionCode\":{},\"error\":{{\"message\":\"{}\"}}}}",
                    json_escape(out_dir),
                    json_escape(contract_version),
                    json_escape(&catalog_hash),
                    ci_reason_code("invoke_a_failed"),
                    json_escape(&error)
                );
                return;
            }
        };
        let invoke_b = match runtime
            .tool_invoke_frame(json_to_backend_value(&frame_b).unwrap_or(Value::Unit))
        {
            Ok(value) => value,
            Err(error) => {
                println!(
                    "{{\"ok\":false,\"stage\":\"invoke-b\",\"outDir\":\"{}\",\"contractVersion\":\"{}\",\"catalogHash\":\"{}\",\"ciDecision\":\"fail\",\"ciDecisionReason\":\"invoke_b_failed\",\"ciDecisionCode\":{},\"error\":{{\"message\":\"{}\"}}}}",
                    json_escape(out_dir),
                    json_escape(contract_version),
                    json_escape(&catalog_hash),
                    ci_reason_code("invoke_b_failed"),
                    json_escape(&error)
                );
                return;
            }
        };
        (invoke_a, invoke_b)
    };

    let wire_a = match &invoke_a {
        Value::Map(entries) => entries.get("wire").and_then(|v| v.as_string().ok()),
        _ => None,
    };
    let wire_b = match &invoke_b {
        Value::Map(entries) => entries.get("wire").and_then(|v| v.as_string().ok()),
        _ => None,
    };
    let (Some(wire_a), Some(wire_b)) = (wire_a, wire_b) else {
        println!(
            "{{\"ok\":false,\"stage\":\"extract-wire\",\"outDir\":\"{}\",\"contractVersion\":\"{}\",\"catalogHash\":\"{}\",\"ciDecision\":\"fail\",\"ciDecisionReason\":\"extract_wire_failed\",\"ciDecisionCode\":{}}}",
            json_escape(out_dir),
            json_escape(contract_version),
            json_escape(&catalog_hash),
            ci_reason_code("extract_wire_failed")
        );
        return;
    };

    let _ = fs::write(&invoke_a_path, value_to_json(&invoke_a));
    let _ = fs::write(&invoke_b_path, value_to_json(&invoke_b));
    let _ = fs::write(&wire_a_path, &wire_a);
    let _ = fs::write(&wire_b_path, &wire_b);

    let validate_a = runtime
        .tool_validate_wire(&wire_a)
        .unwrap_or_else(|_| Value::new_map(std::collections::HashMap::new()));
    let validate_b = runtime
        .tool_validate_wire(&wire_b)
        .unwrap_or_else(|_| Value::new_map(std::collections::HashMap::new()));
    let should_compare = compare_strategies || strategy == "auto";
    let scoring = apply_energy_mode_scoring(scoring, energy_mode);
    let selected_strategy = if strategy == "auto" {
        "prefer_latest"
    } else {
        strategy
    };
    let mut merged = runtime
        .tool_merge_wire(&wire_a, &wire_b, Some(selected_strategy))
        .unwrap_or_else(|_| Value::new_map(std::collections::HashMap::new()));

    let _ = fs::write(&validate_a_path, value_to_json(&validate_a));
    let _ = fs::write(&validate_b_path, value_to_json(&validate_b));
    let _ = fs::write(&merged_path, value_to_json(&merged));

    let invoke_a_kind = match &invoke_a {
        Value::Map(entries) => entries
            .get("response_kind")
            .and_then(|v| v.as_string().ok())
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    };
    let invoke_b_kind = match &invoke_b {
        Value::Map(entries) => entries
            .get("response_kind")
            .and_then(|v| v.as_string().ok())
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    };
    let validate_a_ok = matches!(
        &validate_a,
        Value::Map(entries) if matches!(entries.get("ok"), Some(Value::Bool(true)))
    );
    let validate_b_ok = matches!(
        &validate_b,
        Value::Map(entries) if matches!(entries.get("ok"), Some(Value::Bool(true)))
    );
    let merged_status = match &merged {
        Value::Map(entries) => entries
            .get("status")
            .and_then(|v| v.as_string().ok())
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    };

    let mut decision_gap_out = 0.0f64;
    let mut decision_confidence_out = "n/a".to_string();
    let mut caution_out = false;
    let mut ci_decision_out = "pass".to_string();
    let mut ci_decision_reason_out = "healthy".to_string();
    let mut emitted_runtime_events: Vec<String> = Vec::new();
    let compare_result = if should_compare {
        let strategies = ["prefer_latest", "prefer_blocked", "prefer_terminal"];
        let mut rows = Vec::new();
        let mut best_strategy = "prefer_latest".to_string();
        let mut best_score = f64::MIN;
        let mut second_best_score = f64::MIN;
        let mut best_result = Value::new_map(std::collections::HashMap::new());
        for strategy_name in strategies {
            let result = runtime
                .tool_merge_wire(&wire_a, &wire_b, Some(strategy_name))
                .unwrap_or_else(|_| Value::new_map(std::collections::HashMap::new()));
            let (status, next_action) = match &result {
                Value::Map(entries) => (
                    entries
                        .get("status")
                        .and_then(|v| v.as_string().ok())
                        .unwrap_or_else(|| "unknown".to_string()),
                    entries
                        .get("next_action")
                        .and_then(|v| v.as_string().ok())
                        .unwrap_or_else(|| "unknown".to_string()),
                ),
                _ => ("unknown".to_string(), "unknown".to_string()),
            };

            let status_score = match status.as_str() {
                "ok" => scoring.status_ok,
                "degraded" => scoring.status_degraded,
                _ => scoring.status_unknown,
            };
            let action_score = match next_action.as_str() {
                "execute" => scoring.action_execute,
                "resolve-blockers" => scoring.action_resolve_blockers,
                "none" => scoring.action_none,
                _ => scoring.action_other,
            };
            let energy_penalty = match strategy_name {
                "prefer_blocked" => scoring.penalty_blocked,
                "prefer_terminal" => scoring.penalty_terminal,
                _ => scoring.penalty_latest,
            };
            let score = status_score + action_score - energy_penalty;

            if score > best_score {
                second_best_score = best_score;
                best_score = score;
                best_strategy = strategy_name.to_string();
                best_result = result.clone();
            } else if score > second_best_score {
                second_best_score = score;
            }

            rows.push(json!({
                "strategy": strategy_name,
                "status": status,
                "next_action": next_action,
                "score": score,
                "result": serde_json::from_str::<JsonValue>(&value_to_json(&result)).unwrap_or(JsonValue::Null)
            }));
        }
        let decision_gap = if second_best_score == f64::MIN {
            0.0
        } else {
            best_score - second_best_score
        };
        let decision_confidence = if decision_gap >= confidence_threshold_high {
            "high"
        } else if decision_gap >= confidence_threshold_low {
            "medium"
        } else {
            "low"
        };
        decision_gap_out = decision_gap;
        decision_confidence_out = decision_confidence.to_string();
        caution_out = decision_confidence == "low";
        ci_decision_out = if caution_out {
            "warn".to_string()
        } else {
            "pass".to_string()
        };
        ci_decision_reason_out = if caution_out {
            "low_confidence".to_string()
        } else {
            "healthy".to_string()
        };
        let compare_doc = json!({
            "ok": true,
            "contract_version": contract_version,
            "catalog_hash": catalog_hash,
            "selected_strategy": strategy,
            "best_strategy": best_strategy,
            "best_score": best_score,
            "second_best_score": second_best_score,
            "decision_gap": decision_gap,
            "decision_confidence": decision_confidence,
            "confidence_threshold_low": confidence_threshold_low,
            "confidence_threshold_high": confidence_threshold_high,
            "confidence_profile": confidence_profile,
            "scoring_model": "score=status_score+action_score-energy_penalty",
            "scoring_weights": {
                "status_ok": scoring.status_ok,
                "status_degraded": scoring.status_degraded,
                "status_unknown": scoring.status_unknown,
                "action_execute": scoring.action_execute,
                "action_resolve_blockers": scoring.action_resolve_blockers,
                "action_none": scoring.action_none,
                "action_other": scoring.action_other,
                "penalty_latest": scoring.penalty_latest,
                "penalty_blocked": scoring.penalty_blocked,
                "penalty_terminal": scoring.penalty_terminal
            },
            "energy_mode": energy_mode_name(energy_mode),
            "rows": rows
        });
        if strategy == "auto" {
            merged = best_result;
        }
        if decision_confidence == "low" {
            if runtime.emit_event("strategy.uncertain").is_ok() {
                emitted_runtime_events.push("strategy.uncertain".to_string());
            }
            if runtime.emit_event("performance.drop").is_ok() {
                emitted_runtime_events.push("performance.drop".to_string());
            }
        }
        let _ = fs::write(&report_compare_path, compare_doc.to_string());
        let dashboard_doc = json!({
            "ok": true,
            "contract_version": contract_version,
            "catalog_hash": catalog_hash,
            "decision": {
                "requested_strategy": strategy,
                "effective_strategy": if strategy == "auto" {
                    compare_doc.get("best_strategy").and_then(|v| v.as_str()).unwrap_or("prefer_latest")
                } else {
                    selected_strategy
                },
                "model": "score=status_score+action_score-energy_penalty",
                "energy_mode": energy_mode_name(energy_mode),
                "policy_rationale": energy_policy_rationale(energy_mode),
                "policy_factors": energy_policy_factors(&scoring),
                "decision_gap": decision_gap,
                "decision_confidence": decision_confidence,
                "caution": decision_confidence == "low",
                "confidence_threshold_low": confidence_threshold_low,
                "confidence_threshold_high": confidence_threshold_high,
                "confidence_profile": confidence_profile,
                "ci_decision": ci_decision_out,
                "ci_decision_reason": ci_decision_reason_out,
                "ci_decision_code": ci_reason_code(&ci_decision_reason_out),
                "emitted_runtime_events": emitted_runtime_events.clone()
            },
            "timeline": compare_doc.get("rows").cloned().unwrap_or(JsonValue::Array(vec![]))
        });
        let _ = fs::write(&report_compare_dashboard_path, dashboard_doc.to_string());
        Some(compare_doc)
    } else {
        None
    };

    let recommended_energy_mode =
        recommended_energy_mode_for_ci_decision(&ci_decision_out, energy_mode);
    let effective_energy_mode = if apply_recommended_energy_mode {
        recommended_energy_mode
    } else {
        energy_mode
    };

    let previous_chain = fs::read_to_string(&next_cycle_config_path)
        .ok()
        .and_then(|text| serde_json::from_str::<JsonValue>(&text).ok())
        .unwrap_or(JsonValue::Null);
    let previous_hop = previous_chain
        .get("hop")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let previous_chain_id = previous_chain
        .get("chain_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            format!("chain-{}", ns)
        });
    let effective_max_hops = if next_cycle_max_hops == 0 {
        1
    } else {
        next_cycle_max_hops
    };
    let next_cycle_config = json!({
        "ok": true,
        "chain_id": previous_chain_id,
        "hop": previous_hop.saturating_add(1),
        "max_hops": effective_max_hops,
        "parent_config": if previous_chain.is_null() {
            JsonValue::Null
        } else {
            JsonValue::String(next_cycle_config_path.display().to_string())
        },
        "out_dir": out_dir,
        "contract_version": contract_version,
        "catalog_hash": catalog_hash,
        "ci_decision": ci_decision_out,
        "ci_decision_reason": ci_decision_reason_out,
        "ci_decision_code": ci_reason_code(&ci_decision_reason_out),
        "energy_mode_current": energy_mode_name(energy_mode),
        "energy_mode_recommended": energy_mode_name(recommended_energy_mode),
        "energy_mode_effective": energy_mode_name(effective_energy_mode),
        "apply_recommended_energy_mode": apply_recommended_energy_mode,
        "next_cycle": {
            "energy_mode": energy_mode_name(recommended_energy_mode),
            "strategy": if strategy == "auto" {
                compare_result
                    .as_ref()
                    .and_then(|d| d.get("best_strategy"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("prefer_latest")
            } else {
                selected_strategy
            }
        }
    });
    let _ = fs::write(&next_cycle_config_path, next_cycle_config.to_string());
    if next_cycle_apply_now {
        let _ = fs::write(
            &next_cycle_apply_result_path,
            json!({
                "ok": true,
                "stage": "scheduled",
                "next_cycle_config": next_cycle_config_path.display().to_string(),
                "hop": previous_hop.saturating_add(1),
                "max_hops": effective_max_hops,
                "note": "Executing next cycle from generated config"
            })
            .to_string(),
        );
        tool_pipeline_apply_next_cycle_json(&next_cycle_config_path.display().to_string());
        let _ = fs::write(
            &next_cycle_apply_result_path,
            json!({
                "ok": true,
                "stage": "completed",
                "next_cycle_config": next_cycle_config_path.display().to_string()
            })
            .to_string(),
        );
    }

    let report = json!({
        "ok": true,
        "contract_version": contract_version,
        "catalog_hash": catalog_hash,
        "strict": strict,
        "dry_run": dry_run,
        "energy_mode": energy_mode_name(energy_mode),
        "energy_mode_effective": energy_mode_name(effective_energy_mode),
        "energy_mode_recommended": energy_mode_name(recommended_energy_mode),
        "next_cycle_energy_mode": energy_mode_name(recommended_energy_mode),
        "apply_recommended_energy_mode": apply_recommended_energy_mode,
        "compare_strategies": compare_strategies,
        "decision_gap": decision_gap_out,
        "decision_confidence": decision_confidence_out,
        "caution": caution_out,
        "ci_decision": ci_decision_out,
        "ci_decision_reason": ci_decision_reason_out,
        "ci_decision_code": ci_reason_code(&ci_decision_reason_out),
        "confidence_threshold_low": confidence_threshold_low,
        "confidence_threshold_high": confidence_threshold_high,
        "confidence_profile": confidence_profile,
        "emitted_runtime_events": emitted_runtime_events.clone(),
        "strategy_requested": strategy,
        "strategy_effective": if strategy == "auto" {
            compare_result
                .as_ref()
                .and_then(|d| d.get("best_strategy"))
                .and_then(|v| v.as_str())
                .unwrap_or("prefer_latest")
        } else {
            selected_strategy
        },
        "invoke": {
            "a_response_kind": invoke_a_kind,
            "b_response_kind": invoke_b_kind
        },
        "validate": {
            "a_ok": validate_a_ok,
            "b_ok": validate_b_ok
        },
        "merge": {
            "status": merged_status
        }
    });
    let _ = fs::write(&report_path, report.to_string());

    if emit_contract_bundle {
        let mut baseline_fallback_reason: Option<String> = None;
        let baseline = if let Some(path) = contract_bundle_baseline_path {
            match fs::read_to_string(path) {
                Ok(text) => match serde_json::from_str::<JsonValue>(&text) {
                    Ok(doc) => match validate_pipeline_contract_doc(&doc) {
                        Ok(()) => doc,
                        Err(message) => {
                            baseline_fallback_reason =
                                Some(format!("invalid_contract: {}", message));
                            json!({
                                "ok": true,
                                "contractVersion": contract_version,
                                "catalogHash": catalog_hash,
                                "ciDecision": "pass",
                                "ciDecisionReason": "healthy",
                                "ciDecisionCode": ci_reason_code("healthy")
                            })
                        }
                    },
                    Err(error) => {
                        baseline_fallback_reason = Some(format!("invalid_json: {}", error));
                        json!({
                            "ok": true,
                            "contractVersion": contract_version,
                            "catalogHash": catalog_hash,
                            "ciDecision": "pass",
                            "ciDecisionReason": "healthy",
                            "ciDecisionCode": ci_reason_code("healthy")
                        })
                    }
                },
                Err(error) => {
                    baseline_fallback_reason = Some(format!("read_error: {}", error));
                    json!({
                        "ok": true,
                        "contractVersion": contract_version,
                        "catalogHash": catalog_hash,
                        "ciDecision": "pass",
                        "ciDecisionReason": "healthy",
                        "ciDecisionCode": ci_reason_code("healthy")
                    })
                }
            }
        } else {
            baseline_fallback_reason = Some("missing_baseline_flag".to_string());
            json!({
                "ok": true,
                "contractVersion": contract_version,
                "catalogHash": catalog_hash,
                "ciDecision": "pass",
                "ciDecisionReason": "healthy",
                "ciDecisionCode": ci_reason_code("healthy")
            })
        };
        let candidate = json!({
            "ok": true,
            "contractVersion": contract_version,
            "catalogHash": catalog_hash,
            "ciDecision": ci_decision_out,
            "ciDecisionReason": ci_decision_reason_out,
            "ciDecisionCode": ci_reason_code(&ci_decision_reason_out)
        });
        let (compatibility, breaking, non_breaking) =
            classify_pipeline_contract_compatibility(&baseline, &candidate);
        let advice = pipeline_contract_upgrade_advice(&compatibility, &breaking, &non_breaking);
        let gate = if compatibility == "compatible" {
            "pass"
        } else {
            "fail"
        };
        let rollout = if compatibility == "breaking" {
            "controlled_migration"
        } else {
            "progressive_rollout"
        };
        let bundle = json!({
            "ok": true,
            "contractVersion": contract_version,
            "catalogHash": catalog_hash,
            "baselinePath": contract_baseline_path.display().to_string(),
            "candidatePath": contract_candidate_path.display().to_string(),
            "baseline": baseline,
            "candidate": candidate,
            "diff": {
                "compatibility": compatibility,
                "gate": gate,
                "breakingChanges": breaking,
                "nonBreakingChanges": non_breaking
            },
            "upgrade": {
                "rollout": rollout,
                "advice": advice
            },
            "energy": {
                "modeRuntime": energy_mode_name(energy_mode),
                "modeEffective": energy_mode_name(effective_energy_mode),
                "modeRecommended": energy_mode_name(recommended_energy_mode),
                "nextCycleMode": energy_mode_name(recommended_energy_mode)
            },
            "baselineFallbackReason": baseline_fallback_reason
        });
        let _ = fs::write(&contract_baseline_path, baseline.to_string());
        let _ = fs::write(&contract_candidate_path, candidate.to_string());
        let _ = fs::write(&contract_bundle_path, bundle.to_string());
    }

    let effective_summary_format = if emit_summary_md && summary_format == "none" {
        "md"
    } else {
        summary_format
    };

    if effective_summary_format == "md" || effective_summary_format == "both" {
        let mut summary = String::new();
        summary.push_str("# Tool Pipeline Demo\n\n");
        summary.push_str(&format!("- out_dir: `{}`\n", out_dir));
        summary.push_str(&format!("- contract_version: `{}`\n", contract_version));
        summary.push_str(&format!("- catalog_hash: `{}`\n", catalog_hash));
        summary.push_str(&format!("- strategy.requested: `{}`\n", strategy));
        summary.push_str(&format!(
            "- strategy.effective: `{}`\n",
            if strategy == "auto" {
                compare_result
                    .as_ref()
                    .and_then(|d| d.get("best_strategy"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("prefer_latest")
            } else {
                selected_strategy
            }
        ));
        summary.push_str(&format!("- strict: `{}`\n", strict));
        summary.push_str(&format!("- dry_run: `{}`\n", dry_run));
        summary.push_str(&format!(
            "- energy_mode: `{}`\n",
            energy_mode_name(energy_mode)
        ));
        summary.push_str(&format!(
            "- energy_mode_effective: `{}`\n",
            energy_mode_name(effective_energy_mode)
        ));
        summary.push_str(&format!(
            "- apply_recommended_energy_mode: `{}`\n",
            apply_recommended_energy_mode
        ));
        summary.push_str(&format!("- compare_strategies: `{}`\n", compare_strategies));
        summary.push_str(&format!(
            "- decision: `confidence={} gap={:.2} caution={} profile={} thresholds(low={},high={})`\n",
            decision_confidence_out, decision_gap_out, caution_out, confidence_profile, confidence_threshold_low, confidence_threshold_high
        ));
        summary.push_str(&format!("- ci_decision: `{}`\n", ci_decision_out));
        summary.push_str(&format!(
            "- ci_decision_reason: `{}`\n",
            ci_decision_reason_out
        ));
        summary.push_str(&format!("- invoke.a_response_kind: `{}`\n", invoke_a_kind));
        summary.push_str(&format!("- invoke.b_response_kind: `{}`\n", invoke_b_kind));
        summary.push_str(&format!("- validate.a_ok: `{}`\n", validate_a_ok));
        summary.push_str(&format!("- validate.b_ok: `{}`\n", validate_b_ok));
        summary.push_str(&format!("- merge.status: `{}`\n", merged_status));
        if let Some(status) = fail_on_status {
            summary.push_str(&format!("- fail_on_status: `{}`\n", status));
        }
        let _ = fs::write(&summary_md_path, summary);
        if emit_github_step_summary {
            if let Ok(step_summary_path) = env::var("GITHUB_STEP_SUMMARY") {
                let _ = fs::write(
                    step_summary_path,
                    fs::read_to_string(&summary_md_path).unwrap_or_default(),
                );
            }
        }
    }

    if effective_summary_format == "json" || effective_summary_format == "both" {
        let summary_doc = json!({
            "out_dir": out_dir,
            "contract_version": contract_version,
            "catalog_hash": catalog_hash,
            "strategy_requested": strategy,
            "strategy_effective": if strategy == "auto" {
                compare_result
                    .as_ref()
                    .and_then(|d| d.get("best_strategy"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("prefer_latest")
            } else {
                selected_strategy
            },
            "strict": strict,
            "energy_mode": energy_mode_name(energy_mode),
            "energy_mode_effective": energy_mode_name(effective_energy_mode),
            "energy_mode_recommended": energy_mode_name(recommended_energy_mode),
            "apply_recommended_energy_mode": apply_recommended_energy_mode,
            "compare_strategies": compare_strategies,
            "decision_gap": decision_gap_out,
            "decision_confidence": decision_confidence_out,
            "caution": caution_out,
            "ci_decision": ci_decision_out,
            "ci_decision_reason": ci_decision_reason_out,
            "ci_decision_code": ci_reason_code(&ci_decision_reason_out),
            "confidence_threshold_low": confidence_threshold_low,
            "confidence_threshold_high": confidence_threshold_high,
            "confidence_profile": confidence_profile,
            "emitted_runtime_events": emitted_runtime_events.clone(),
            "invoke": {
                "a_response_kind": invoke_a_kind,
                "b_response_kind": invoke_b_kind
            },
            "validate": {
                "a_ok": validate_a_ok,
                "b_ok": validate_b_ok
            },
            "merge": {
                "status": merged_status
            }
        });
        let _ = fs::write(&summary_json_path, summary_doc.to_string());
    }

    if let Some(target_status) = fail_on_status {
        if merged_status == target_status {
            println!(
                "{{\"ok\":false,\"stage\":\"fail-on-status\",\"outDir\":\"{}\",\"contractVersion\":\"{}\",\"catalogHash\":\"{}\",\"ciDecision\":\"fail\",\"ciDecisionReason\":\"matched_fail_status\",\"ciDecisionCode\":{},\"error\":{{\"message\":\"merge status matched fail-on-status\"}},\"mergeStatus\":\"{}\",\"failOnStatus\":\"{}\",\"report\":\"{}\"}}",
                json_escape(out_dir),
                json_escape(contract_version),
                json_escape(&catalog_hash),
                ci_reason_code("matched_fail_status"),
                json_escape(&merged_status),
                json_escape(target_status),
                json_escape(&report_path.display().to_string())
            );
            if ci_exit_codes {
                process::exit(22);
            }
            return;
        }
    }

    if strict
        && match &merged {
            Value::Map(entries) => matches!(
                entries.get("status"),
                Some(Value::String(status)) if status.as_str() == "degraded"
            ),
            _ => false,
        }
    {
        println!(
            "{{\"ok\":false,\"stage\":\"strict-check\",\"outDir\":\"{}\",\"contractVersion\":\"{}\",\"catalogHash\":\"{}\",\"ciDecision\":\"fail\",\"ciDecisionReason\":\"strict_degraded\",\"ciDecisionCode\":{},\"error\":{{\"message\":\"merge returned degraded status in strict mode\"}},\"report\":\"{}\"}}",
            json_escape(out_dir),
            json_escape(contract_version),
            json_escape(&catalog_hash),
            ci_reason_code("strict_degraded"),
            json_escape(&report_path.display().to_string())
        );
        if ci_exit_codes {
            process::exit(21);
        }
        return;
    }

    if artifact_manifest_json {
        let mut artifacts = vec![
            frame_a_path.clone(),
            frame_b_path.clone(),
            invoke_a_path.clone(),
            invoke_b_path.clone(),
            wire_a_path.clone(),
            wire_b_path.clone(),
            validate_a_path.clone(),
            validate_b_path.clone(),
            merged_path.clone(),
            report_path.clone(),
        ];
        if should_compare {
            artifacts.push(report_compare_path.clone());
            artifacts.push(report_compare_dashboard_path.clone());
        }
        if emit_contract_bundle {
            artifacts.push(contract_baseline_path.clone());
            artifacts.push(contract_candidate_path.clone());
            artifacts.push(contract_bundle_path.clone());
        }
        artifacts.push(next_cycle_config_path.clone());
        if next_cycle_apply_now {
            artifacts.push(next_cycle_apply_result_path.clone());
        }
        if effective_summary_format == "md" || effective_summary_format == "both" {
            artifacts.push(summary_md_path.clone());
        }
        if effective_summary_format == "json" || effective_summary_format == "both" {
            artifacts.push(summary_json_path.clone());
        }
        let items: Vec<JsonValue> = artifacts
            .into_iter()
            .map(|path| {
                let exists = path.exists();
                let size_bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                json!({
                    "path": path.display().to_string(),
                    "exists": exists,
                    "size_bytes": size_bytes
                })
            })
            .collect();
        let manifest = json!({
            "ok": true,
            "contract_version": contract_version,
            "catalog_hash": catalog_hash,
            "out_dir": out_dir,
            "artifact_prefix": artifact_prefix,
            "energy_mode": energy_mode_name(energy_mode),
            "energy_mode_effective": energy_mode_name(effective_energy_mode),
            "energy_mode_recommended": energy_mode_name(recommended_energy_mode),
            "policy_rationale": energy_policy_rationale(energy_mode),
            "policy_factors": energy_policy_factors(&scoring),
            "confidence_profile": confidence_profile,
            "confidence_threshold_low": confidence_threshold_low,
            "confidence_threshold_high": confidence_threshold_high,
            "ci_decision": ci_decision_out,
            "ci_decision_reason": ci_decision_reason_out,
            "ci_decision_code": ci_reason_code(&ci_decision_reason_out),
            "count": items.len(),
            "artifacts": items
        });
        let _ = fs::write(&artifact_manifest_path, manifest.to_string());
    }

    println!(
        "{{\"ok\":true,\"outDir\":\"{}\",\"contractVersion\":\"{}\",\"catalogHash\":\"{}\",\"strict\":{},\"dryRun\":{},\"compareStrategies\":{},\"summaryFormat\":\"{}\",\"artifactPrefix\":\"{}\",\"strategyRequested\":\"{}\",\"strategyEffective\":\"{}\",\"confidenceProfile\":\"{}\",\"confidenceThresholdLow\":{:.2},\"confidenceThresholdHigh\":{:.2},\"decisionConfidence\":\"{}\",\"decisionGap\":{:.2},\"caution\":{},\"ciDecision\":\"{}\",\"ciDecisionReason\":\"{}\",\"ciDecisionCode\":{},\"applyRecommendedEnergyMode\":{},\"nextCycleApplyNow\":{},\"energyModeEffective\":\"{}\",\"emittedRuntimeEvents\":{},\"artifacts\":[\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"{}{}{}{}{}{}{}{}]}}",
        json_escape(out_dir),
        json_escape(contract_version),
        json_escape(&catalog_hash),
        if strict { "true" } else { "false" },
        if dry_run { "true" } else { "false" },
        if compare_strategies { "true" } else { "false" },
        json_escape(effective_summary_format),
        json_escape(artifact_prefix),
        json_escape(strategy),
        json_escape(if strategy == "auto" {
            compare_result
                .as_ref()
                .and_then(|d| d.get("best_strategy"))
                .and_then(|v| v.as_str())
                .unwrap_or("prefer_latest")
        } else {
            selected_strategy
        }),
        json_escape(confidence_profile),
        confidence_threshold_low,
        confidence_threshold_high,
        json_escape(&decision_confidence_out),
        decision_gap_out,
        if caution_out { "true" } else { "false" },
        json_escape(&ci_decision_out),
        json_escape(&ci_decision_reason_out),
        ci_reason_code(&ci_decision_reason_out),
        if apply_recommended_energy_mode { "true" } else { "false" },
        if next_cycle_apply_now { "true" } else { "false" },
        json_escape(energy_mode_name(effective_energy_mode)),
        serde_json::to_string(&emitted_runtime_events).unwrap_or_else(|_| "[]".to_string()),
        json_escape(&frame_a_path.display().to_string()),
        json_escape(&frame_b_path.display().to_string()),
        json_escape(&invoke_a_path.display().to_string()),
        json_escape(&invoke_b_path.display().to_string()),
        json_escape(&wire_a_path.display().to_string()),
        json_escape(&wire_b_path.display().to_string()),
        json_escape(&validate_a_path.display().to_string()),
        json_escape(&validate_b_path.display().to_string()),
        json_escape(&merged_path.display().to_string()),
        json_escape(&report_path.display().to_string()),
        if compare_result.is_some() {
            format!(
                ",\"{}\"",
                json_escape(&report_compare_path.display().to_string())
            )
        } else {
            "".to_string()
        },
        if should_compare {
            format!(
                ",\"{}\"",
                json_escape(&report_compare_dashboard_path.display().to_string())
            )
        } else {
            "".to_string()
        },
        if effective_summary_format == "md" || effective_summary_format == "both" {
            format!(",\"{}\"", json_escape(&summary_md_path.display().to_string()))
        } else {
            "".to_string()
        },
        if effective_summary_format == "json" || effective_summary_format == "both" {
            format!(",\"{}\"", json_escape(&summary_json_path.display().to_string()))
        } else {
            "".to_string()
        },
        if artifact_manifest_json {
            format!(
                ",\"{}\"",
                json_escape(&artifact_manifest_path.display().to_string())
            )
        } else {
            "".to_string()
        },
        if emit_contract_bundle {
            format!(
                ",\"{}\",\"{}\",\"{}\"",
                json_escape(&contract_baseline_path.display().to_string()),
                json_escape(&contract_candidate_path.display().to_string()),
                json_escape(&contract_bundle_path.display().to_string())
            )
        } else {
            "".to_string()
        },
        format!(
            ",\"{}\"",
            json_escape(&next_cycle_config_path.display().to_string())
        ),
        if next_cycle_apply_now {
            format!(
                ",\"{}\"",
                json_escape(&next_cycle_apply_result_path.display().to_string())
            )
        } else {
            "".to_string()
        }

    );
}

fn visual_step_json(path: &str, events_path: &str, delta_ms_raw: &str, with_energy: bool) {
    let delta_ms = delta_ms_raw.parse::<i64>().unwrap_or_else(|_| {
        eprintln!("Usage error: delta_ms must be an integer");
        process::exit(1);
    });
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let bytecode = build_json_or_exit(&source, input, &[("events", events_path)]);

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.run() {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"input\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
            json_escape(input),
            json_escape(events_path),
            json_escape(&error),
            json_string_array(&runtime.take_output()),
            energy_json_fragment(
                with_energy,
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            )
        );
        process::exit(1);
    }

    let result = runtime.visual_app_step(events_path, delta_ms).unwrap_or_else(|error| {
        println!(
            "{{\"ok\":false,\"stage\":\"visual\",\"input\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
            json_escape(input),
            json_escape(events_path),
            json_escape(&error),
            json_string_array(&runtime.take_output()),
            energy_json_fragment(
                with_energy,
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            )
        );
        process::exit(1);
    });

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"events\":\"{}\",\"deltaMs\":{},\"result\":{},\"output\":{}{}}}",
        json_escape(input),
        json_escape(events_path),
        delta_ms,
        value_to_json(&result),
        json_string_array(&runtime.take_output()),
        energy_json_fragment(
            with_energy,
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        )
    );
}

fn visual_run_json(
    path: &str,
    events_path: &str,
    frames_raw: &str,
    delta_ms_raw: &str,
    with_energy: bool,
) {
    let frames = frames_raw.parse::<i64>().unwrap_or_else(|_| {
        eprintln!("Usage error: frames must be an integer");
        process::exit(1);
    });
    let delta_ms = delta_ms_raw.parse::<i64>().unwrap_or_else(|_| {
        eprintln!("Usage error: delta_ms must be an integer");
        process::exit(1);
    });
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let bytecode = build_json_or_exit(&source, input, &[("events", events_path)]);

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);

    if let Err(error) = runtime.run() {
        println!(
            "{{\"ok\":false,\"stage\":\"runtime\",\"input\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
            json_escape(input),
            json_escape(events_path),
            json_escape(&error),
            json_string_array(&runtime.take_output()),
            energy_json_fragment(
                with_energy,
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            )
        );
        process::exit(1);
    }

    let result = runtime
        .visual_app_run(events_path, frames, delta_ms)
        .unwrap_or_else(|error| {
            println!(
                "{{\"ok\":false,\"stage\":\"visual\",\"input\":\"{}\",\"events\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
                json_escape(input),
                json_escape(events_path),
                json_escape(&error),
                json_string_array(&runtime.take_output()),
                energy_json_fragment(
                    with_energy,
                    runtime.vm().estimated_instruction_cost(),
                    runtime.vm().estimated_backend_cost()
                )
            );
            process::exit(1);
        });

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"events\":\"{}\",\"frames\":{},\"deltaMs\":{},\"result\":{},\"output\":{}{}}}",
        json_escape(input),
        json_escape(events_path),
        frames,
        delta_ms,
        value_to_json(&result),
        json_string_array(&runtime.take_output()),
        energy_json_fragment(
            with_energy,
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        )
    );
}

#[derive(Debug, Clone)]
struct NativeStudioModel {
    input: String,
    output: Vec<String>,
    status: Vec<String>,
    surface_name: String,
    surface_width: i64,
    surface_height: i64,
    regions: Vec<NativeStudioRegion>,
    instruction_cost: f64,
    backend_cost: f64,
}

#[derive(Debug, Clone)]
struct NativeStudioRegion {
    name: String,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
    text: String,
    semantic: String,
    event: String,
    state: String,
}

fn studio_native(path: &str, clear: bool, interactive: bool) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let mut model = build_native_studio_model(&source, input).unwrap_or_else(|error| {
        eprintln!("studio-native error: {}", error);
        process::exit(1);
    });
    if interactive {
        run_native_studio_loop(&source, input, clear, &mut model);
    } else {
        let frame = render_native_studio(&model, clear);
        print!("{}", frame);
    }
}

fn studio_native_json(path: &str) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    match build_native_studio_model(&source, input) {
        Ok(model) => println!("{}", native_studio_model_json(&model)),
        Err(error) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "input": input,
                    "error": { "message": error }
                })
            );
            process::exit(1);
        }
    }
}

fn sentinel_pvmbc(path: &str, output: &str, name: &str) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let model = build_native_studio_model(&source, input).unwrap_or_else(|error| {
        eprintln!("sentinel-pvmbc error: {}", error);
        process::exit(1);
    });
    let app_name = sanitize_sentinel_app_name(name);
    let bytes = encode_sentinel_pvmbc(&model, &app_name);
    let output_path = Path::new(output);
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(error) = fs::create_dir_all(parent) {
                eprintln!("sentinel-pvmbc mkdir error: {}", error);
                process::exit(1);
            }
        }
    }
    if let Err(error) = fs::write(output_path, &bytes) {
        eprintln!("sentinel-pvmbc write error: {}", error);
        process::exit(1);
    }
    println!(
        "{}",
        json!({
            "ok": true,
            "input": model.input,
            "output": output,
            "format": "PVM2",
            "name": app_name,
            "bytes": bytes.len(),
            "surface": {
                "name": model.surface_name,
                "width": model.surface_width,
                "height": model.surface_height
            },
            "regions": model.regions.len()
        })
    );
}

fn sentinel_pvmbc_rust_array(path: &str, const_name: &str, name: &str) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let model = build_native_studio_model(&source, input).unwrap_or_else(|error| {
        eprintln!("sentinel-pvmbc-rust-array error: {}", error);
        process::exit(1);
    });
    let app_name = sanitize_sentinel_app_name(name);
    let bytes = encode_sentinel_pvmbc(&model, &app_name);
    print!("{}", sentinel_rust_byte_array(const_name, &bytes));
}

fn sentinel_mbc1_rust_array(path: &str, const_name: &str) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let bytecode = compile_source(&source, input).unwrap_or_else(|error| {
        eprintln!("sentinel-mbc1-rust-array error: {}", error);
        process::exit(1);
    });
    let bytes = serialize_bytecode_or_exit(&bytecode, "sentinel-mbc1-rust-array");
    print!("{}", sentinel_rust_byte_array(const_name, &bytes));
}

fn sentinel_mbc1_kernel_check_json(path: &str, instruction_budget: u32) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let (ok, payload) = sentinel_mbc1_kernel_check_payload(&source, input, instruction_budget);
    println!("{}", payload);
    if !ok {
        process::exit(1);
    }
}

fn rust_ffi_call_json(library_path: &str, symbol: &str, args_json: &str) {
    let args_source = match rust_ffi_args_source(args_json) {
        Ok(source) => source,
        Err(error) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "stage": "read_args",
                    "library": library_path,
                    "symbol": symbol,
                    "error": { "message": error }
                })
            );
            process::exit(1);
        }
    };

    let args_value = match serde_json::from_str::<JsonValue>(&args_source) {
        Ok(value) => value,
        Err(error) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "stage": "parse_args",
                    "library": library_path,
                    "symbol": symbol,
                    "error": { "message": error.to_string() }
                })
            );
            process::exit(1);
        }
    };

    let args = match rust_ffi_args_from_json(&args_value) {
        Ok(args) => args,
        Err(error) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "stage": "decode_args",
                    "library": library_path,
                    "symbol": symbol,
                    "error": { "message": error }
                })
            );
            process::exit(1);
        }
    };

    let mut bridge = RustBridge::new();
    let result = bridge
        .import_module(library_path)
        .and_then(|_| bridge.call_function(library_path, symbol, args));

    match result {
        Ok(value) => {
            println!(
                "{}",
                json!({
                    "ok": true,
                    "library": library_path,
                    "symbol": symbol,
                    "result": encode_value_json(&value)
                })
            );
        }
        Err(error) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "stage": "call",
                    "library": library_path,
                    "symbol": symbol,
                    "error": { "message": error }
                })
            );
            process::exit(1);
        }
    }
}

fn rust_ffi_validate_args_json(args_json: &str) {
    match rust_ffi_validate_args_payload(args_json) {
        Ok(payload) => println!("{}", payload),
        Err(payload) => {
            println!("{}", payload);
            process::exit(1);
        }
    }
}

fn rust_ffi_validate_args_payload(args_json: &str) -> Result<JsonValue, JsonValue> {
    let args_source = rust_ffi_args_source(args_json).map_err(|error| {
        json!({
            "ok": false,
            "stage": "read_args",
            "error": { "message": error }
        })
    })?;

    let args_value = serde_json::from_str::<JsonValue>(&args_source).map_err(|error| {
        json!({
            "ok": false,
            "stage": "parse_args",
            "error": { "message": error.to_string() }
        })
    })?;

    let args = rust_ffi_args_from_json(&args_value).map_err(|error| {
        json!({
            "ok": false,
            "stage": "decode_args",
            "error": { "message": error }
        })
    })?;

    Ok(json!({
        "ok": true,
        "args_count": args.len(),
        "args": args.iter().map(encode_value_json).collect::<Vec<_>>()
    }))
}

fn rust_ffi_args_source(args_json: &str) -> Result<String, String> {
    if let Some(path) = args_json.strip_prefix('@') {
        fs::read_to_string(path)
            .map(|source| source.trim_start_matches('\u{feff}').to_string())
            .map_err(|error| format!("failed to read Rust FFI args file '{}': {}", path, error))
    } else {
        Ok(args_json.to_string())
    }
}

fn rust_ffi_args_from_json(value: &JsonValue) -> Result<Vec<Value>, String> {
    decode_args_json(value)
}

#[cfg(test)]
fn rust_ffi_value_from_json(value: &JsonValue) -> Result<Value, String> {
    matter_bridge_rust::decode_value_json(value)
}

fn sentinel_mbc1_kernel_check_payload(
    source: &str,
    input: &str,
    instruction_budget: u32,
) -> (bool, JsonValue) {
    let bytecode = match compile_source(source, input) {
        Ok(bytecode) => bytecode,
        Err(error) => {
            return (
                false,
                json!({
                    "ok": false,
                    "input": input,
                    "stage": "compile",
                    "error": { "message": error }
                }),
            );
        }
    };

    let bytes = match serialize_bytecode(&bytecode) {
        Ok(bytes) => bytes,
        Err(error) => {
            return (
                false,
                json!({
                    "ok": false,
                    "input": input,
                    "stage": "serialize",
                    "error": { "message": error }
                }),
            );
        }
    };

    let info = match matter_kernel_vm::inspect_mbc1(&bytes) {
        Ok(info) => info,
        Err(error) => {
            return (
                false,
                json!({
                    "ok": false,
                    "input": input,
                    "stage": "inspect",
                    "bytes": bytes.len(),
                    "error": { "kind": format!("{:?}", error) }
                }),
            );
        }
    };

    match matter_kernel_vm::run_mbc1_main(&bytes, instruction_budget) {
        Ok(result) => (
            true,
            json!({
                "ok": true,
                "input": input,
                "format": "MBC1",
                "bytes": bytes.len(),
                "instruction_budget": instruction_budget,
                "info": kernel_mbc_info_json(&info),
                "run": kernel_run_result_json(&result)
            }),
        ),
        Err(error) => (
            false,
            json!({
                "ok": false,
                "input": input,
                "stage": "run",
                "format": "MBC1",
                "bytes": bytes.len(),
                "instruction_budget": instruction_budget,
                "info": kernel_mbc_info_json(&info),
                "error": { "kind": format!("{:?}", error) }
            }),
        ),
    }
}

fn serialize_bytecode(bytecode: &Bytecode) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    bytecode
        .serialize(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

fn serialize_bytecode_or_exit(bytecode: &Bytecode, command: &str) -> Vec<u8> {
    match serialize_bytecode(bytecode) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{} serialize error: {}", command, error);
            process::exit(1);
        }
    }
}

fn kernel_mbc_info_json(info: &matter_kernel_vm::MbcInfo) -> JsonValue {
    json!({
        "version": {
            "major": info.version_major,
            "minor": info.version_minor
        },
        "flags": info.flags,
        "declared_total_instructions": info.declared_total_instructions,
        "decoded_total_instructions": info.decoded_total_instructions,
        "constants": info.constants,
        "functions": info.functions,
        "events": info.events,
        "main_instructions": info.main_instructions
    })
}

fn kernel_run_result_json(result: &matter_kernel_vm::KernelRunResult<'_>) -> JsonValue {
    json!({
        "last_value": kernel_value_json(&result.last_value),
        "prints": result.prints.iter().map(kernel_value_json).collect::<Vec<_>>(),
        "syscalls": result.syscalls.iter().map(kernel_syscall_json).collect::<Vec<_>>(),
        "instructions_executed": result.instructions_executed,
        "max_call_depth": result.max_call_depth,
        "halted": result.halted
    })
}

fn kernel_syscall_json(syscall: &matter_kernel_vm::KernelSyscall<'_>) -> JsonValue {
    json!({
        "backend": String::from_utf8_lossy(syscall.backend).to_string(),
        "method": String::from_utf8_lossy(syscall.method).to_string(),
        "args": syscall.args.iter().map(kernel_value_json).collect::<Vec<_>>()
    })
}

fn kernel_value_json(value: &matter_kernel_vm::KernelValue) -> JsonValue {
    match value {
        matter_kernel_vm::KernelValue::Int(value) => json!({
            "type": "int",
            "value": value
        }),
        matter_kernel_vm::KernelValue::Bool(value) => json!({
            "type": "bool",
            "value": value
        }),
        matter_kernel_vm::KernelValue::Unit => json!({
            "type": "unit",
            "value": null
        }),
    }
}

fn sentinel_pvmbc_inspect_json(path: &str) {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "input": path,
                    "error": { "message": error.to_string() }
                })
            );
            process::exit(1);
        }
    };
    match matter_sentinel_abi::inspect_pvmbc(&bytes) {
        Ok(report) => println!(
            "{}",
            json!({
                "ok": true,
                "input": path,
                "bytes": bytes.len(),
                "package": sentinel_pvmbc_report_json(&report)
            })
        ),
        Err(error) => {
            println!(
                "{}",
                json!({
                    "ok": false,
                    "input": path,
                    "bytes": bytes.len(),
                    "error": { "message": format!("{:?}", error) }
                })
            );
            process::exit(1);
        }
    }
}

fn sentinel_rust_byte_array(const_name: &str, bytes: &[u8]) -> String {
    let const_name = sanitize_rust_const_name(const_name);
    let mut output = String::new();
    output.push_str("pub const ");
    output.push_str(&const_name);
    output.push_str(": &[u8] = &[\n");

    for chunk in bytes.chunks(16) {
        output.push_str("    ");
        for byte in chunk {
            output.push_str(&format!("0x{:02X}, ", byte));
        }
        output.push('\n');
    }

    output.push_str("];\n");
    output
}

fn sanitize_rust_const_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
        } else if ch == '_' || ch == '-' || ch.is_whitespace() {
            out.push('_');
        }
    }

    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let out = out.trim_matches('_').to_string();
    let out = if out.is_empty() {
        "MATTER_PVMBC".to_string()
    } else {
        out
    };

    if out
        .as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_digit())
    {
        format!("_{}", out)
    } else {
        out
    }
}

fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].as_str())
}

fn first_positional_arg<'a>(args: &'a [String], value_options: &[&str]) -> Option<&'a str> {
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if value_options.contains(&arg.as_str()) {
            skip_next = true;
            continue;
        }
        if !arg.starts_with('-') {
            return Some(arg.as_str());
        }
    }
    None
}

fn collect_positional_args(args: &[String], value_options: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if value_options.contains(&arg.as_str()) {
            skip_next = true;
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        values.push(arg.clone());
    }
    values
}

fn encode_sentinel_pvmbc(model: &NativeStudioModel, app_name: &str) -> Vec<u8> {
    const PVM2_FORMAT_VERSION: u16 = 2;
    const OP_CLEAR: u8 = 0;
    const OP_FILL_RECT: u8 = 1;
    const OP_PULSE: u8 = 2;
    const OP_SET_BEHAVIOR: u8 = 3;
    const OP_FRAME: u8 = 4;

    let name = sanitize_sentinel_app_name(app_name);
    let name_bytes = name.as_bytes();
    let width = sentinel_dimension(model.surface_width);
    let height = sentinel_dimension(model.surface_height);
    let mut opcodes = Vec::new();

    opcodes.push(OP_CLEAR);
    push_u32(&mut opcodes, 0xff07111e);

    for region in &model.regions {
        opcodes.push(OP_FILL_RECT);
        push_sentinel_region(&mut opcodes, region);
        push_u32(&mut opcodes, sentinel_region_color(region));

        let behavior_id = sentinel_behavior_id(region);
        if behavior_id > 0 {
            opcodes.push(OP_SET_BEHAVIOR);
            push_sentinel_region(&mut opcodes, region);
            push_u16(&mut opcodes, behavior_id);
        }

        if sentinel_region_pulse_energy(region) > 0 {
            opcodes.push(OP_PULSE);
            push_sentinel_region(&mut opcodes, region);
            opcodes.push(sentinel_region_pulse_energy(region));
        }
    }

    opcodes.push(OP_FRAME);

    let opcode_count = sentinel_opcode_count(&opcodes);
    let mut bytes = Vec::with_capacity(40 + name_bytes.len() + opcodes.len());
    bytes.extend_from_slice(b"PVM2");
    push_u16(&mut bytes, PVM2_FORMAT_VERSION);
    push_u16(&mut bytes, name_bytes.len() as u16);
    push_u32(&mut bytes, 1);
    push_u64(&mut bytes, 0);
    push_u32(&mut bytes, 0);
    push_u32(&mut bytes, width);
    push_u32(&mut bytes, height);
    push_u32(&mut bytes, opcode_count);
    bytes.extend_from_slice(name_bytes);
    bytes.extend_from_slice(&opcodes);
    bytes
}

fn sentinel_opcode_count(bytes: &[u8]) -> u32 {
    let mut index = 0;
    let mut count = 0;
    while index < bytes.len() {
        count += 1;
        index += match bytes[index] {
            0 => 1 + 4,
            1 => 1 + 16 + 4,
            2 => 1 + 16 + 1,
            3 => 1 + 16 + 2,
            4 => 1,
            _ => break,
        };
    }
    count
}

fn push_sentinel_region(bytes: &mut Vec<u8>, region: &NativeStudioRegion) {
    push_u32(bytes, sentinel_coord(region.x));
    push_u32(bytes, sentinel_coord(region.y));
    push_u32(bytes, sentinel_dimension(region.w));
    push_u32(bytes, sentinel_dimension(region.h));
}

fn sentinel_region_color(region: &NativeStudioRegion) -> u32 {
    let signal = format!(
        "{} {} {} {}",
        region.name, region.text, region.semantic, region.event
    )
    .to_ascii_lowercase();
    if signal.contains("run") || signal.contains("primary") || region.state == "active" {
        0xffff7a45
    } else if signal.contains("guard") {
        0xffe7b95e
    } else if signal.contains("reflect") {
        0xff77c98d
    } else if signal.contains("editor") || signal.contains("code") || signal.contains("source") {
        0xff151a24
    } else if signal.contains("conversation") || signal.contains("chat") {
        0xff1e2b3d
    } else if signal.contains("input") || signal.contains("prompt") {
        0xff25435f
    } else if signal.contains("sidebar") || signal.contains("nav") {
        0xff202a38
    } else if signal.contains("topbar") || signal.contains("header") {
        0xff113b5f
    } else {
        0xff2b3443
    }
}

fn sentinel_behavior_id(region: &NativeStudioRegion) -> u16 {
    let action = first_non_empty(&[&region.event, &region.semantic, &region.name]);
    match action.as_str() {
        "run_source" | "primary_action" => 101,
        "reflect_source" => 102,
        "guard_source" => 103,
        "send_prompt" | "input" => 104,
        "source_editor" | "code_editor" => 105,
        "conversation" | "chat" => 106,
        _ if region.state == "active" => 1,
        _ => 0,
    }
}

fn sentinel_region_pulse_energy(region: &NativeStudioRegion) -> u8 {
    if region.state == "active" || sentinel_behavior_id(region) >= 101 {
        64
    } else {
        0
    }
}

fn sanitize_sentinel_app_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .take(64)
        .collect();
    if out.is_empty() {
        out.push_str("matter-app");
    }
    out
}

fn sentinel_coord(value: i64) -> u32 {
    value.clamp(0, 8192) as u32
}

fn sentinel_dimension(value: i64) -> u32 {
    value.clamp(1, 8192) as u32
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn sentinel_pvmbc_report_json(report: &matter_sentinel_abi::PvmbcInfo<'_>) -> JsonValue {
    let format = match report.format {
        matter_sentinel_abi::PvmFormat::Pvm1 => "PVM1",
        matter_sentinel_abi::PvmFormat::Pvm2 => "PVM2",
    };
    let name = std::str::from_utf8(report.name).unwrap_or("");
    json!({
        "format": format,
        "format_version": report.format_version,
        "name": name,
        "package_version": report.package_version,
        "permissions": report.permissions,
        "entrypoint": report.entrypoint,
        "width": report.width,
        "height": report.height,
        "declared_opcodes": report.declared_opcodes,
        "decoded_opcodes": report.decoded_opcodes,
        "opcode_counts": {
            "clear": report.opcode_counts[PvmOpcodeTag::Clear as usize],
            "fill_rect": report.opcode_counts[PvmOpcodeTag::FillRect as usize],
            "pulse": report.opcode_counts[PvmOpcodeTag::Pulse as usize],
            "set_behavior": report.opcode_counts[PvmOpcodeTag::SetBehavior as usize],
            "frame": report.opcode_counts[PvmOpcodeTag::Frame as usize]
        },
        "frame_count": report.frame_count
    })
}

fn run_native_studio_loop(source: &str, input: &str, clear: bool, model: &mut NativeStudioModel) {
    loop {
        print!("{}", render_native_studio(model, clear));
        print!("\nCommand [r=run, c=check, v=visual, g=guard, tap <region>, q=quit]> ");
        if let Err(error) = io::stdout().flush() {
            eprintln!("studio-native stdout error: {}", error);
            process::exit(1);
        }

        let mut command = String::new();
        match io::stdin().read_line(&mut command) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                eprintln!("studio-native input error: {}", error);
                process::exit(1);
            }
        }

        let trimmed = command.trim();
        if let Some(target) = trimmed
            .strip_prefix("tap ")
            .or_else(|| trimmed.strip_prefix("click "))
        {
            model.status = native_studio_tap_status(source, model, target);
            continue;
        }

        match trimmed.to_ascii_lowercase().as_str() {
            "q" | "quit" | "exit" => break,
            "r" | "run" => {
                model.status = native_studio_run_status(source);
            }
            "c" | "check" => {
                model.status = native_studio_check_status(source);
            }
            "g" | "guard" => {
                model.status = native_studio_guard_status(source);
            }
            "reflect" | "inspect" => {
                model.status = native_studio_reflect_status(source);
            }
            "v" | "visual" | "" => match build_native_studio_model(source, input) {
                Ok(next) => *model = next,
                Err(error) => model.status = vec![format!("visual refresh failed: {}", error)],
            },
            other => {
                model.status = vec![format!("unknown command: {}", other)];
            }
        }
    }
}

fn native_studio_model_json(model: &NativeStudioModel) -> String {
    let regions: Vec<JsonValue> = model
        .regions
        .iter()
        .map(|region| {
            json!({
                "name": region.name,
                "x": region.x,
                "y": region.y,
                "w": region.w,
                "h": region.h,
                "text": region.text,
                "semantic": region.semantic,
                "event": region.event,
                "state": region.state
            })
        })
        .collect();
    json!({
        "ok": true,
        "input": model.input,
        "surface": {
            "name": model.surface_name,
            "width": model.surface_width,
            "height": model.surface_height
        },
        "regions": regions,
        "output": model.output,
        "status": model.status,
        "energy": {
            "instruction_cost": model.instruction_cost,
            "backend_cost": model.backend_cost
        }
    })
    .to_string()
}

fn build_native_studio_model(source: &str, input: &str) -> Result<NativeStudioModel, String> {
    let (_program, bytecode) = parse_and_build_native_source(source)?;

    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);
    runtime.run().map_err(|error| error.to_string())?;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let events_path = std::env::temp_dir().join(format!(
        "matter_studio_native_events_{}_{}.json",
        process::id(),
        nonce
    ));
    fs::write(
        &events_path,
        "{\"format\":\"PXL_EVENT_QUEUE\",\"version\":1,\"events\":[]}",
    )
    .map_err(|error| error.to_string())?;
    let events_path = events_path
        .to_str()
        .ok_or_else(|| "temporary event path is not UTF-8".to_string())?;

    let result = runtime
        .visual_app_run(events_path, 1, 16)
        .map_err(|error| error.to_string())?;
    let result_json: JsonValue =
        serde_json::from_str(&value_to_json(&result)).map_err(|error| error.to_string())?;
    let snapshot_raw = result_json
        .get("app")
        .and_then(|app| app.get("snapshot"))
        .and_then(JsonValue::as_str)
        .ok_or_else(|| "visual backend did not return a PXL snapshot".to_string())?;
    let snapshot: JsonValue =
        serde_json::from_str(snapshot_raw).map_err(|error| error.to_string())?;

    let surface = snapshot
        .get("surfaces")
        .and_then(JsonValue::as_array)
        .and_then(|items| items.first())
        .ok_or_else(|| "PXL snapshot has no surface".to_string())?;
    let surface_name = json_field_string(surface, "name", "surface");
    let surface_width = json_field_i64(surface, "width", 1);
    let surface_height = json_field_i64(surface, "height", 1);
    let regions = snapshot
        .get("regions")
        .and_then(JsonValue::as_array)
        .map(|items| {
            items
                .iter()
                .map(parse_native_studio_region)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(NativeStudioModel {
        input: input.to_string(),
        output: runtime.take_output(),
        status: vec!["visual model ready".to_string()],
        surface_name,
        surface_width,
        surface_height,
        regions,
        instruction_cost: runtime.vm().estimated_instruction_cost(),
        backend_cost: runtime.vm().estimated_backend_cost(),
    })
}

fn parse_and_build_native_source(source: &str) -> Result<(Program, Bytecode), String> {
    let mut parser = Parser::from_source(source);
    let program = parser.parse().map_err(|error| format!("{:?}", error))?;
    let bytecode = BytecodeBuilder::new()
        .build_checked(&program)
        .map_err(|error| error.to_string())?;
    Ok((program, bytecode))
}

fn native_studio_run_status(source: &str) -> Vec<String> {
    match parse_and_build_native_source(source) {
        Ok((_program, bytecode)) => {
            let mut runtime = Runtime::new_silent(bytecode);
            runtime.set_stdout_enabled(false);
            match runtime.run() {
                Ok(()) => {
                    let mut lines = vec![format!(
                        "run ok | instr={:.2} backend={:.2}",
                        runtime.vm().estimated_instruction_cost(),
                        runtime.vm().estimated_backend_cost()
                    )];
                    lines.extend(
                        runtime
                            .take_output()
                            .into_iter()
                            .map(|line| format!("out: {}", line)),
                    );
                    lines
                }
                Err(error) => vec![format!("run failed: {}", error)],
            }
        }
        Err(error) => vec![format!("run blocked: {}", error)],
    }
}

fn native_studio_check_status(source: &str) -> Vec<String> {
    match parse_and_build_native_source(source) {
        Ok((program, bytecode)) => vec![
            "check ok".to_string(),
            format!("top_level_statements={}", program.statements.len()),
            format!("constants={}", bytecode.constants.len()),
            format!("functions={}", bytecode.functions.len()),
            format!("event_handlers={}", bytecode.event_handlers.len()),
            format!("main_instructions={}", bytecode.main_instructions.len()),
        ],
        Err(error) => vec![format!("check failed: {}", error)],
    }
}

fn native_studio_guard_status(source: &str) -> Vec<String> {
    match parse_and_build_native_source(source) {
        Ok((program, bytecode)) => {
            let report =
                reflexive_guard_report(&program, &bytecode, &ReflexiveGuardOptions::default());
            vec![
                format!(
                    "guard status={}",
                    report["status"].as_str().unwrap_or("unknown")
                ),
                format!(
                    "statements={}",
                    report["metrics"]["total_statements"].as_u64().unwrap_or(0)
                ),
                format!(
                    "functions={}",
                    report["metrics"]["functions"].as_u64().unwrap_or(0)
                ),
                format!(
                    "backend_calls={}",
                    report["metrics"]["backend_calls"].as_u64().unwrap_or(0)
                ),
            ]
        }
        Err(error) => vec![format!("guard failed: {}", error)],
    }
}

fn native_studio_reflect_status(source: &str) -> Vec<String> {
    match parse_and_build_native_source(source) {
        Ok((program, bytecode)) => {
            let ast: JsonValue =
                serde_json::from_str(&ast_reflection_json(&program)).unwrap_or_else(|_| json!({}));
            let bytecode_json: JsonValue =
                serde_json::from_str(&bytecode_reflection_json(&bytecode))
                    .unwrap_or_else(|_| json!({}));
            vec![
                "reflect ok".to_string(),
                format!(
                    "total_statements={}",
                    ast["total_statements"].as_u64().unwrap_or(0)
                ),
                format!(
                    "top_level_statements={}",
                    ast["top_level_statements"].as_u64().unwrap_or(0)
                ),
                format!(
                    "bytecode_functions={}",
                    bytecode_json["summary"]["functions"].as_u64().unwrap_or(0)
                ),
                format!(
                    "bytecode_instructions={}",
                    bytecode_json["summary"]["instructions"]
                        .as_u64()
                        .unwrap_or(0)
                ),
            ]
        }
        Err(error) => vec![format!("reflect failed: {}", error)],
    }
}

fn native_studio_tap_status(source: &str, model: &NativeStudioModel, target: &str) -> Vec<String> {
    let Some(region) = find_native_studio_region(model, target) else {
        return vec![format!("tap target not found: {}", target)];
    };
    let action = first_non_empty(&[&region.event, &region.semantic, &region.name]);
    let mut lines = vec![format!(
        "tap {} -> {}",
        if region.text.is_empty() {
            &region.name
        } else {
            &region.text
        },
        action
    )];
    let mut result = match action.as_str() {
        "run_source" | "primary_action" => native_studio_run_status(source),
        "reflect_source" => native_studio_reflect_status(source),
        "guard_source" => native_studio_guard_status(source),
        "check_source" => native_studio_check_status(source),
        _ => vec![format!("no native action bound for {}", action)],
    };
    lines.append(&mut result);
    lines
}

fn find_native_studio_region<'a>(
    model: &'a NativeStudioModel,
    target: &str,
) -> Option<&'a NativeStudioRegion> {
    let normalized = normalize_native_studio_key(target);
    model.regions.iter().find(|region| {
        [
            region.name.as_str(),
            region.text.as_str(),
            region.event.as_str(),
            region.semantic.as_str(),
        ]
        .iter()
        .any(|candidate| normalize_native_studio_key(candidate) == normalized)
    })
}

fn normalize_native_studio_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn parse_native_studio_region(value: &JsonValue) -> NativeStudioRegion {
    let props = value.get("properties").unwrap_or(&JsonValue::Null);
    NativeStudioRegion {
        name: json_field_string(value, "name", "region"),
        x: json_field_i64(value, "x", 0),
        y: json_field_i64(value, "y", 0),
        w: json_field_i64(value, "w", 1),
        h: json_field_i64(value, "h", 1),
        text: json_field_string(props, "text", ""),
        semantic: json_field_string(props, "semantic", ""),
        event: json_field_string(props, "event", ""),
        state: json_field_string(props, "state", ""),
    }
}

fn json_field_string(value: &JsonValue, key: &str, default: &str) -> String {
    value
        .get(key)
        .and_then(JsonValue::as_str)
        .unwrap_or(default)
        .to_string()
}

fn json_field_i64(value: &JsonValue, key: &str, default: i64) -> i64 {
    value
        .get(key)
        .and_then(JsonValue::as_i64)
        .unwrap_or(default)
}

fn render_native_studio(model: &NativeStudioModel, clear: bool) -> String {
    let mut out = String::new();
    if clear {
        out.push_str("\x1b[2J\x1b[H");
    }
    out.push_str("Matter Studio Native - Rust terminal shell\n");
    out.push_str("============================================================\n");
    out.push_str(&format!("Input: {}\n", model.input));
    out.push_str(&format!(
        "Surface: {} {}x{} | regions={} | energy instr={:.2} backend={:.2}\n",
        model.surface_name,
        model.surface_width,
        model.surface_height,
        model.regions.len(),
        model.instruction_cost,
        model.backend_cost
    ));
    out.push_str("Mode: native Rust CLI, no browser, no HTML, no Node runtime\n\n");
    out.push_str("+----------------------+-------------------------------------+\n");
    out.push_str("| Matter Controls      | Native Visual Surface               |\n");
    out.push_str("+----------------------+-------------------------------------+\n");
    out.push_str("| [R] Run VM           |");
    out.push_str(&format!("{:<37}|\n", format!(" {}", model.surface_name)));
    out.push_str("| [C] Check source     |");
    out.push_str(&format!("{:<37}|\n", " PXL regions rendered below"));
    out.push_str("| [V] Visual preview   |");
    out.push_str(&format!("{:<37}|\n", " generated by visual.* calls"));
    out.push_str("| [G] Guard reflexive  |");
    out.push_str(&format!("{:<37}|\n", " ready for native event loop"));
    out.push_str("+----------------------+-------------------------------------+\n\n");
    out.push_str("Status\n");
    out.push_str("------\n");
    for line in &model.status {
        out.push_str("- ");
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&render_native_region_map(model, 68, 18));
    out.push_str("\nRegions\n");
    out.push_str("-------\n");
    for region in &model.regions {
        let label = if region.text.is_empty() {
            &region.name
        } else {
            &region.text
        };
        let behavior = first_non_empty(&[&region.semantic, &region.event, &region.state]);
        out.push_str(&format!(
            "- {:<18} {:>4},{:<4} {:>4}x{:<4} {}\n",
            truncate(label, 18),
            region.x,
            region.y,
            region.w,
            region.h,
            behavior
        ));
    }
    if !model.output.is_empty() {
        out.push_str("\nVM Output\n");
        out.push_str("---------\n");
        for line in &model.output {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn render_native_region_map(model: &NativeStudioModel, width: usize, height: usize) -> String {
    let mut grid = vec![vec![' '; width]; height];
    let surface_width = model.surface_width.max(1) as f64;
    let surface_height = model.surface_height.max(1) as f64;
    for region in &model.regions {
        let marker = region_marker(region);
        let x0 = ((region.x.max(0) as f64 / surface_width) * width as f64).floor() as usize;
        let y0 = ((region.y.max(0) as f64 / surface_height) * height as f64).floor() as usize;
        let x1 =
            (((region.x + region.w).max(1) as f64 / surface_width) * width as f64).ceil() as usize;
        let y1 = (((region.y + region.h).max(1) as f64 / surface_height) * height as f64).ceil()
            as usize;
        for row in grid.iter_mut().take(y1.min(height)).skip(y0.min(height)) {
            for cell in row.iter_mut().take(x1.min(width)).skip(x0.min(width)) {
                *cell = marker;
            }
        }
    }

    let mut out = String::new();
    out.push('+');
    out.push_str(&"-".repeat(width));
    out.push_str("+\n");
    for row in grid {
        out.push('|');
        for cell in row {
            out.push(cell);
        }
        out.push_str("|\n");
    }
    out.push('+');
    out.push_str(&"-".repeat(width));
    out.push_str("+\n");
    out
}

fn region_marker(region: &NativeStudioRegion) -> char {
    let label = if !region.text.is_empty() {
        &region.text
    } else {
        &region.name
    };
    label
        .chars()
        .find(|ch| ch.is_ascii_alphanumeric())
        .unwrap_or('#')
}

fn first_non_empty(values: &[&str]) -> String {
    values
        .iter()
        .find(|value| !value.is_empty())
        .copied()
        .unwrap_or("")
        .to_string()
}

fn truncate(value: &str, width: usize) -> String {
    let mut text = value.chars().take(width).collect::<String>();
    if value.chars().count() > width && width >= 1 {
        text.pop();
        text.push('~');
    }
    text
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
        .map(|(index, spanned)| {
            token_json(
                index,
                &spanned.token,
                spanned.span.line,
                spanned.span.column,
            )
        })
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
        let root = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(path));
        let base = Path::new(path)
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        (source, base, vec![root])
    };

    let mut imports = Vec::new();
    if let Err(error) = collect_imports(
        &source,
        &base_dir,
        source_label(path),
        &mut stack,
        &mut imports,
    ) {
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

fn reflect_json(path: &str) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let mut parser = Parser::from_source(&source);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(error) => {
            print_parse_error_json(input, &[], &error);
            process::exit(1);
        }
    };

    let bytecode = match BytecodeBuilder::new().build_checked(&program) {
        Ok(bytecode) => bytecode,
        Err(error) => {
            print_semantic_error_json(input, &[], &error);
            process::exit(1);
        }
    };

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"reflection_version\":1,\"ast\":{},\"bytecode\":{}}}",
        json_escape(input),
        ast_reflection_json(&program),
        bytecode_reflection_json(&bytecode)
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReflexiveGuardOptions {
    max_statements: usize,
    max_functions: usize,
    allow_backends: bool,
}

impl Default for ReflexiveGuardOptions {
    fn default() -> Self {
        Self {
            max_statements: 200,
            max_functions: 50,
            allow_backends: false,
        }
    }
}

fn parse_reflexive_guard_options(args: &[String]) -> ReflexiveGuardOptions {
    let mut options = ReflexiveGuardOptions::default();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--max-statements" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --max-statements requires a number");
                    process::exit(1);
                }
                options.max_statements = args[i + 1].parse::<usize>().unwrap_or_else(|_| {
                    eprintln!("Error: --max-statements must be a positive integer");
                    process::exit(1);
                });
                i += 2;
            }
            "--max-functions" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --max-functions requires a number");
                    process::exit(1);
                }
                options.max_functions = args[i + 1].parse::<usize>().unwrap_or_else(|_| {
                    eprintln!("Error: --max-functions must be a positive integer");
                    process::exit(1);
                });
                i += 2;
            }
            "--allow-backends" => {
                options.allow_backends = true;
                i += 1;
            }
            other => {
                eprintln!("Unknown reflexive guard option: {}", other);
                process::exit(1);
            }
        }
    }
    options
}

fn reflexive_guard_json(path: &str, options: &ReflexiveGuardOptions) {
    let source = read_source_or_exit(path);
    let input = source_label(path);
    let mut parser = Parser::from_source(&source);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(error) => {
            print_parse_error_json(input, &[], &error);
            process::exit(1);
        }
    };

    let bytecode = match BytecodeBuilder::new().build_checked(&program) {
        Ok(bytecode) => bytecode,
        Err(error) => {
            print_semantic_error_json(input, &[], &error);
            process::exit(1);
        }
    };

    let report = reflexive_guard_report(&program, &bytecode, options);
    println!(
        "{}",
        json!({
            "ok": report["status"].as_str() != Some("fail"),
            "input": input,
            "guard_version": 1,
            "guard": report,
            "reflection": {
                "ast": serde_json::from_str::<JsonValue>(&ast_reflection_json(&program)).unwrap(),
                "bytecode": serde_json::from_str::<JsonValue>(&bytecode_reflection_json(&bytecode)).unwrap()
            }
        })
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

#[derive(Clone, Copy)]
struct BytecodeStats {
    constants: usize,
    functions: usize,
    event_handlers: usize,
    instructions: usize,
    backend_calls: usize,
}

impl BytecodeStats {
    fn from_bytecode(bytecode: &Bytecode) -> Self {
        let function_instructions = bytecode
            .functions
            .values()
            .map(|function| function.instructions.len())
            .sum::<usize>();
        let event_instructions = bytecode
            .event_handlers
            .values()
            .map(|handler| handler.instructions.len())
            .sum::<usize>();
        let instructions =
            bytecode.main_instructions.len() + function_instructions + event_instructions;
        let backend_calls = bytecode
            .main_instructions
            .iter()
            .chain(
                bytecode
                    .functions
                    .values()
                    .flat_map(|function| function.instructions.iter()),
            )
            .chain(
                bytecode
                    .event_handlers
                    .values()
                    .flat_map(|handler| handler.instructions.iter()),
            )
            .filter(|instruction| matches!(instruction, Instruction::BackendCall { .. }))
            .count();

        Self {
            constants: bytecode.constants.len(),
            functions: bytecode.functions.len(),
            event_handlers: bytecode.event_handlers.len(),
            instructions,
            backend_calls,
        }
    }

    fn memory_pressure(&self) -> f32 {
        (self.constants as f32 * 1.8
            + self.functions as f32 * 4.0
            + self.event_handlers as f32 * 3.0)
            .clamp(0.0, 100.0)
    }

    fn cache_hit_rate(&self) -> f32 {
        (0.92 - self.backend_calls as f32 * 0.04 - self.functions as f32 * 0.01).clamp(0.05, 0.98)
    }

    fn optimization_confidence(&self, instruction_cost: f64) -> f32 {
        let structural_gain = if self.instructions > 12 { 0.72 } else { 0.42 };
        let cost_gain = (instruction_cost as f32 / 120.0).clamp(0.0, 0.25);
        (structural_gain + cost_gain - self.backend_calls as f32 * 0.03).clamp(0.0, 1.0)
    }

    fn novelty_score(&self) -> f32 {
        (self.event_handlers as f32 * 0.12
            + self.functions as f32 * 0.08
            + self.backend_calls as f32 * 0.10
            + self.instructions as f32 / 250.0)
            .clamp(0.05, 1.0)
    }

    fn to_json(self) -> JsonValue {
        json!({
            "constants": self.constants,
            "functions": self.functions,
            "event_handlers": self.event_handlers,
            "instructions": self.instructions,
            "backend_calls": self.backend_calls,
        })
    }
}

fn read_source_for_json(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|error| format!("Error reading Matter source from stdin: {error}"))?;
        let mut seen = HashSet::new();
        resolve_imports(&source, Path::new("."), &mut seen)
    } else {
        let source = fs::read_to_string(path)
            .map_err(|error| format!("Error reading file '{path}': {error}"))?;
        let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
        let mut seen = HashSet::new();
        resolve_imports(&source, base_dir, &mut seen)
    }
}

fn emnr_decision_for_event(
    session: &mut MatterBridgeSession,
    event: MatterRuntimeEvent,
) -> JsonValue {
    serde_json::to_value(session.decide(event)).unwrap_or_else(|error| {
        json!({
            "decision": "ObserveOnly",
            "error": error.to_string(),
        })
    })
}

fn build_emnr_run_payload(
    ok: bool,
    stage: &str,
    input: &str,
    output: &[String],
    energy_and_stats: Option<(f64, f64, BytecodeStats)>,
    error: Option<&str>,
    emnr: JsonValue,
) -> JsonValue {
    let (energy, bytecode) = match energy_and_stats {
        Some((instruction_cost, backend_cost, stats)) => (
            json!({
                "instruction_cost": instruction_cost,
                "backend_cost": backend_cost,
            }),
            stats.to_json(),
        ),
        None => (JsonValue::Null, JsonValue::Null),
    };

    let mut payload = json!({
        "ok": ok,
        "stage": stage,
        "input": input,
        "output": output,
        "emnr": emnr,
    });

    if !energy.is_null() {
        payload["energy"] = energy;
    }
    if !bytecode.is_null() {
        payload["bytecode"] = bytecode;
    }
    if let Some(message) = error {
        payload["error"] = json!({ "message": message });
    }

    payload
}

fn source_complexity_cost(source: &str) -> f32 {
    (source.lines().count() as f32 * 1.5 + source.len() as f32 / 80.0).clamp(0.0, 100.0)
}

fn source_memory_pressure(source: &str) -> f32 {
    (source.len() as f32 / 40.0).clamp(0.0, 100.0)
}

fn backend_security_risk(backend_cost: f64) -> f32 {
    (backend_cost as f32 / 120.0).clamp(0.0, 0.65)
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
                return Err(format!(
                    "circular import detected for '{}'",
                    canonical.display()
                ));
            }

            let imported_source = fs::read_to_string(&canonical)
                .map_err(|e| format!("could not read import '{}': {}", canonical.display(), e))?;
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

fn resolve_import_path_with_dependencies(
    import_path: &str,
    base_dir: &Path,
    project_base_dir: &Path,
    dependencies: &[ManifestDependency],
) -> Result<PathBuf, String> {
    let full_path = if is_std_import(import_path) {
        stdlib_root().join(strip_std_prefix(import_path))
    } else if let Some(dependency) = dependencies
        .iter()
        .find(|dependency| dependency.name == import_path)
    {
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

struct DoctorCheck {
    name: &'static str,
    ok: bool,
    detail: String,
}

fn print_doctor() {
    let checks = collect_doctor_checks();
    let ok = checks.iter().all(|check| check.ok);

    println!("Matter doctor");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!(
        "Workspace: {}",
        env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|error| format!("unknown ({})", error))
    );
    println!();

    for check in &checks {
        let mark = if check.ok { "OK" } else { "FAIL" };
        println!("[{}] {} - {}", mark, check.name, check.detail);
    }

    println!();
    if ok {
        println!("Matter workspace is healthy.");
    } else {
        println!("Matter workspace has problems. See failed checks above.");
        process::exit(1);
    }
}

fn print_doctor_json() {
    let checks = collect_doctor_checks();
    let ok = checks.iter().all(|check| check.ok);
    let workspace = env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|error| format!("unknown ({})", error));
    let checks_json = checks
        .iter()
        .map(|check| {
            format!(
                "{{\"name\":\"{}\",\"ok\":{},\"detail\":\"{}\"}}",
                json_escape(check.name),
                check.ok,
                json_escape(&check.detail)
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    println!(
        "{{\"ok\":{},\"version\":\"{}\",\"workspace\":\"{}\",\"checks\":[{}]}}",
        ok,
        json_escape(env!("CARGO_PKG_VERSION")),
        json_escape(&workspace),
        checks_json
    );

    if !ok {
        process::exit(1);
    }
}

fn collect_doctor_checks() -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    checks.push(DoctorCheck {
        name: "workspace_manifest",
        ok: Path::new("Cargo.toml").exists(),
        detail: if Path::new("Cargo.toml").exists() {
            "Cargo.toml found".to_string()
        } else {
            "Cargo.toml not found in current directory".to_string()
        },
    });

    let cargo_config = Path::new(".cargo").join("config.toml");
    let target_dir_check = fs::read_to_string(&cargo_config)
        .map(|content| content.contains("target-dir") && content.contains("matter_target"))
        .unwrap_or(false);
    checks.push(DoctorCheck {
        name: "safe_target_dir",
        ok: target_dir_check,
        detail: if target_dir_check {
            ".cargo/config.toml points build output outside the spaced workspace path".to_string()
        } else {
            ".cargo/config.toml is missing the expected matter_target build directory".to_string()
        },
    });

    checks.push(DoctorCheck {
        name: "package_manifest",
        ok: Path::new("matter.toml").exists(),
        detail: if Path::new("matter.toml").exists() {
            "matter.toml found".to_string()
        } else {
            "matter.toml not found".to_string()
        },
    });

    checks.push(DoctorCheck {
        name: "examples",
        ok: Path::new("examples").join("hello.matter").exists(),
        detail: if Path::new("examples").join("hello.matter").exists() {
            "examples/hello.matter found".to_string()
        } else {
            "examples/hello.matter not found".to_string()
        },
    });

    match doctor_core_pipeline_check() {
        Ok(output) => checks.push(DoctorCheck {
            name: "core_pipeline",
            ok: true,
            detail: format!("parse -> bytecode -> VM produced {}", output),
        }),
        Err(error) => checks.push(DoctorCheck {
            name: "core_pipeline",
            ok: false,
            detail: error,
        }),
    }

    checks
}

fn doctor_core_pipeline_check() -> Result<String, String> {
    let source = "let x = 40\nlet y = 2\nprint x + y\n";
    let mut parser = Parser::from_source(source);
    let program = parser.parse().map_err(|error| error.to_string())?;
    let bytecode = BytecodeBuilder::new()
        .build_checked(&program)
        .map_err(|error| error.to_string())?;
    let mut runtime = Runtime::new_silent(bytecode);
    runtime.set_stdout_enabled(false);
    runtime.run()?;
    let output = runtime.take_output();

    if output == vec!["42".to_string()] {
        Ok("42".to_string())
    } else {
        Err(format!("expected output 42, got {:?}", output))
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
                return Err(format!(
                    "circular import detected for '{}'",
                    canonical.display()
                ));
            }

            let resolved = canonical.display().to_string();
            let source = import_source_kind(&import_path, dependencies);
            imports.push(ImportInfo {
                from: from_label.to_string(),
                path: import_path.clone(),
                resolved: resolved.clone(),
                source,
            });

            let imported_source = fs::read_to_string(&canonical)
                .map_err(|e| format!("could not read import '{}': {}", canonical.display(), e))?;
            let imported_base = canonical.parent().unwrap_or(Path::new(".")).to_path_buf();

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
    } else if dependencies
        .iter()
        .any(|dependency| dependency.name == import_path)
    {
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

fn energy_json_fragment(with_energy: bool, instruction_cost: f64, backend_cost: f64) -> String {
    if with_energy {
        format!(
            ",\"energy\":{{\"instruction_cost\":{:.2},\"backend_cost\":{:.2}}}",
            instruction_cost, backend_cost
        )
    } else {
        String::new()
    }
}

fn value_to_json(value: &Value) -> String {
    match value {
        Value::Int(n) => n.to_string(),
        Value::Float(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Unit => "null".to_string(),
        Value::String(text) | Value::Function(text) => {
            format!("\"{}\"", json_escape(text.as_ref()))
        }
        Value::List(items) => {
            let encoded: Vec<String> = items.iter().map(value_to_json).collect();
            format!("[{}]", encoded.join(","))
        }
        Value::Map(entries) => {
            let mut pairs: Vec<(String, String)> = entries
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        format!("\"{}\":{}", json_escape(key), value_to_json(value)),
                    )
                })
                .collect();
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            format!(
                "{{{}}}",
                pairs
                    .into_iter()
                    .map(|(_, value)| value)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        Value::Struct { type_name, fields } => {
            let mut pairs: Vec<(String, String)> = fields
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        format!("\"{}\":{}", json_escape(key), value_to_json(value)),
                    )
                })
                .collect();
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            let fields_json = pairs
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"type\":\"{}\",\"fields\":{{{}}}}}",
                json_escape(type_name.as_ref()),
                fields_json
            )
        }
    }
}

fn json_to_backend_value(value: &JsonValue) -> Result<Value, String> {
    match value {
        JsonValue::Null => Ok(Value::Unit),
        JsonValue::Bool(flag) => Ok(Value::Bool(*flag)),
        JsonValue::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Ok(Value::Int(integer))
            } else {
                Err("only integer numbers are supported in tool frame payload".to_string())
            }
        }
        JsonValue::String(text) => Ok(Value::new_string(text.clone())),
        JsonValue::Array(items) => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                values.push(json_to_backend_value(item)?);
            }
            Ok(Value::new_list(values))
        }
        JsonValue::Object(entries) => {
            let mut map = std::collections::HashMap::new();
            for (key, item) in entries {
                map.insert(key.clone(), json_to_backend_value(item)?);
            }
            Ok(Value::new_map(map))
        }
    }
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
        Token::Float(value) => ("float", Some(value.to_string())),
        Token::String(value) => ("string", Some(value.clone())),
        Token::Bool(value) => ("bool", Some(value.to_string())),
        Token::Ident(value) => ("ident", Some(value.clone())),
        Token::Plus => ("plus", None),
        Token::Minus => ("minus", None),
        Token::Star => ("star", None),
        Token::Slash => ("slash", None),
        Token::Percent => ("percent", None),
        Token::Eq => ("eq", None),
        Token::EqEq => ("eq_eq", None),
        Token::NotEq => ("not_eq", None),
        Token::Lt => ("lt", None),
        Token::Gt => ("gt", None),
        Token::LtEq => ("lt_eq", None),
        Token::GtEq => ("gt_eq", None),
        Token::And => ("and", None),
        Token::Or => ("or", None),
        Token::Not => ("not", None),
        Token::Arrow => ("arrow", None),
        Token::LParen => ("lparen", None),
        Token::RParen => ("rparen", None),
        Token::LBrace => ("lbrace", None),
        Token::RBrace => ("rbrace", None),
        Token::LBracket => ("lbracket", None),
        Token::RBracket => ("rbracket", None),
        Token::Comma => ("comma", None),
        Token::Dot => ("dot", None),
        Token::Colon => ("colon", None),
        Token::Semicolon => ("semicolon", None),
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

fn ast_reflection_json(program: &Program) -> String {
    let mut statements = BTreeMap::new();
    let mut calls = BTreeMap::new();
    let mut backend_calls = BTreeMap::new();
    let total_statements = collect_statement_reflection(
        &program.statements,
        &mut statements,
        &mut calls,
        &mut backend_calls,
    );

    format!(
        "{{\"top_level_statements\":{},\"total_statements\":{},\"statement_kinds\":{},\"calls\":{},\"backend_calls\":{}}}",
        program.statements.len(),
        total_statements,
        usize_map_json(&statements),
        usize_map_json(&calls),
        usize_map_json(&backend_calls)
    )
}

fn bytecode_reflection_json(bytecode: &Bytecode) -> String {
    let mut opcodes = BTreeMap::new();
    collect_instruction_histogram(&bytecode.main_instructions, &mut opcodes);
    for function in bytecode.functions.values() {
        collect_instruction_histogram(&function.instructions, &mut opcodes);
    }
    for handler in bytecode.event_handlers.values() {
        collect_instruction_histogram(&handler.instructions, &mut opcodes);
    }

    let mut functions: Vec<String> = bytecode
        .functions
        .values()
        .map(|function| {
            format!(
                "{{\"name\":\"{}\",\"params\":{},\"instructions\":{}}}",
                json_escape(&function.name),
                function.param_count,
                function.instructions.len()
            )
        })
        .collect();
    functions.sort();

    let mut events: Vec<String> = bytecode
        .event_handlers
        .values()
        .map(|handler| {
            format!(
                "{{\"event\":\"{}\",\"instructions\":{}}}",
                json_escape(&handler.event),
                handler.instructions.len()
            )
        })
        .collect();
    events.sort();

    format!(
        "{{\"summary\":{},\"functions\":[{}],\"event_handlers\":[{}],\"opcode_histogram\":{}}}",
        bytecode_summary_json(bytecode),
        functions.join(","),
        events.join(","),
        usize_map_json(&opcodes)
    )
}

fn reflexive_guard_report(
    program: &Program,
    bytecode: &Bytecode,
    options: &ReflexiveGuardOptions,
) -> JsonValue {
    let ast_reflection: JsonValue =
        serde_json::from_str(&ast_reflection_json(program)).expect("valid ast reflection JSON");
    let total_statements = ast_reflection["total_statements"].as_u64().unwrap_or(0) as usize;
    let function_count = bytecode.functions.len();
    let backend_call_count = ast_reflection["backend_calls"]
        .as_object()
        .map(|calls| {
            calls
                .values()
                .filter_map(|value| value.as_u64())
                .sum::<u64>()
        })
        .unwrap_or(0);
    let recursive_functions = direct_recursive_functions(program);

    let mut checks = Vec::new();
    checks.push(reflexive_check_json(
        "statement_budget",
        total_statements <= options.max_statements,
        total_statements,
        options.max_statements,
        "total statement count must stay inside the guard budget",
    ));
    checks.push(reflexive_check_json(
        "function_budget",
        function_count <= options.max_functions,
        function_count,
        options.max_functions,
        "function count must stay inside the guard budget",
    ));

    let backend_passed = options.allow_backends || backend_call_count == 0;
    checks.push(json!({
        "name": "backend_policy",
        "passed": backend_passed,
        "severity": if backend_passed { "pass" } else { "fail" },
        "actual": backend_call_count,
        "limit": if options.allow_backends { JsonValue::Null } else { json!(0) },
        "message": if options.allow_backends {
            "backend calls are explicitly allowed"
        } else {
            "backend calls require --allow-backends in reflexive mode"
        }
    }));

    checks.push(json!({
        "name": "direct_recursion",
        "passed": true,
        "severity": if recursive_functions.is_empty() { "pass" } else { "warn" },
        "functions": recursive_functions.clone(),
        "message": "direct recursion is allowed but should be benchmarked before self-mutation"
    }));

    let has_fail = checks
        .iter()
        .any(|check| check["severity"].as_str() == Some("fail"));
    let has_warn = checks
        .iter()
        .any(|check| check["severity"].as_str() == Some("warn"));
    let status = if has_fail {
        "fail"
    } else if has_warn {
        "warn"
    } else {
        "pass"
    };

    json!({
        "status": status,
        "policy": {
            "max_statements": options.max_statements,
            "max_functions": options.max_functions,
            "allow_backends": options.allow_backends
        },
        "metrics": {
            "total_statements": total_statements,
            "functions": function_count,
            "backend_calls": backend_call_count,
            "direct_recursive_functions": direct_recursive_functions(program)
        },
        "checks": checks
    })
}

fn reflexive_check_json(
    name: &str,
    passed: bool,
    actual: usize,
    limit: usize,
    message: &str,
) -> JsonValue {
    json!({
        "name": name,
        "passed": passed,
        "severity": if passed { "pass" } else { "fail" },
        "actual": actual,
        "limit": limit,
        "message": message
    })
}

fn direct_recursive_functions(program: &Program) -> Vec<String> {
    let mut recursive = Vec::new();
    for statement in &program.statements {
        if let Statement::FunctionDef { name, body, .. } = statement {
            let mut calls = BTreeMap::new();
            collect_calls_in_statements(body, &mut calls);
            if calls.contains_key(name) {
                recursive.push(name.clone());
            }
        }
    }
    recursive.sort();
    recursive
}

fn collect_calls_in_statements(statements: &[Statement], calls: &mut BTreeMap<String, usize>) {
    for statement in statements {
        match statement {
            Statement::Let { value, .. }
            | Statement::Set { value, .. }
            | Statement::Return(value)
            | Statement::Print(value)
            | Statement::Expression(value) => collect_calls_in_expression(value, calls),
            Statement::SetIndex {
                target,
                index,
                value,
            } => {
                collect_calls_in_expression(target, calls);
                collect_calls_in_expression(index, calls);
                collect_calls_in_expression(value, calls);
            }
            Statement::SetField { value, .. } => collect_calls_in_expression(value, calls),
            Statement::FunctionDef { body, .. }
            | Statement::OnEvent { body, .. }
            | Statement::Loop { body } => collect_calls_in_statements(body, calls),
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                collect_calls_in_expression(condition, calls);
                collect_calls_in_statements(then_body, calls);
                if let Some(else_body) = else_body {
                    collect_calls_in_statements(else_body, calls);
                }
            }
            Statement::While { condition, body } => {
                collect_calls_in_expression(condition, calls);
                collect_calls_in_statements(body, calls);
            }
            Statement::For { iterable, body, .. } => {
                collect_calls_in_expression(iterable, calls);
                collect_calls_in_statements(body, calls);
            }
            Statement::StructDef { .. }
            | Statement::Import { .. }
            | Statement::Spawn { .. }
            | Statement::Break
            | Statement::Continue => {}
        }
    }
}

fn collect_calls_in_expression(expression: &Expression, calls: &mut BTreeMap<String, usize>) {
    match expression {
        Expression::Binary { left, right, .. } => {
            collect_calls_in_expression(left, calls);
            collect_calls_in_expression(right, calls);
        }
        Expression::Unary { operand, .. } => collect_calls_in_expression(operand, calls),
        Expression::Call { callee, args } => {
            if let Expression::Identifier(name) = callee.as_ref() {
                bump_count(calls, name);
            }
            collect_calls_in_expression(callee, calls);
            for arg in args {
                collect_calls_in_expression(arg, calls);
            }
        }
        Expression::BackendCall { args, .. } => {
            for arg in args {
                collect_calls_in_expression(arg, calls);
            }
        }
        Expression::List(items) => {
            for item in items {
                collect_calls_in_expression(item, calls);
            }
        }
        Expression::Map(items) | Expression::StructLiteral { fields: items, .. } => {
            for (_, value) in items {
                collect_calls_in_expression(value, calls);
            }
        }
        Expression::Field { target, .. } => collect_calls_in_expression(target, calls),
        Expression::Index { target, index } => {
            collect_calls_in_expression(target, calls);
            collect_calls_in_expression(index, calls);
        }
        Expression::MethodCall { target, args, .. } => {
            collect_calls_in_expression(target, calls);
            for arg in args {
                collect_calls_in_expression(arg, calls);
            }
        }
        Expression::Int(_)
        | Expression::Float(_)
        | Expression::Bool(_)
        | Expression::String(_)
        | Expression::Unit
        | Expression::Identifier(_) => {}
    }
}

fn collect_statement_reflection(
    statements: &[Statement],
    statement_kinds: &mut BTreeMap<String, usize>,
    calls: &mut BTreeMap<String, usize>,
    backend_calls: &mut BTreeMap<String, usize>,
) -> usize {
    let mut total = 0;
    for statement in statements {
        total += 1;
        bump_count(statement_kinds, statement_kind(statement));
        match statement {
            Statement::Let { value, .. }
            | Statement::Set { value, .. }
            | Statement::Return(value)
            | Statement::Print(value)
            | Statement::Expression(value) => {
                collect_expression_reflection(value, calls, backend_calls);
            }
            Statement::SetIndex {
                target,
                index,
                value,
            } => {
                collect_expression_reflection(target, calls, backend_calls);
                collect_expression_reflection(index, calls, backend_calls);
                collect_expression_reflection(value, calls, backend_calls);
            }
            Statement::SetField { value, .. } => {
                collect_expression_reflection(value, calls, backend_calls);
            }
            Statement::FunctionDef { body, .. } | Statement::OnEvent { body, .. } => {
                total += collect_statement_reflection(body, statement_kinds, calls, backend_calls);
            }
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                collect_expression_reflection(condition, calls, backend_calls);
                total +=
                    collect_statement_reflection(then_body, statement_kinds, calls, backend_calls);
                if let Some(else_body) = else_body {
                    total += collect_statement_reflection(
                        else_body,
                        statement_kinds,
                        calls,
                        backend_calls,
                    );
                }
            }
            Statement::While { condition, body } => {
                collect_expression_reflection(condition, calls, backend_calls);
                total += collect_statement_reflection(body, statement_kinds, calls, backend_calls);
            }
            Statement::For { iterable, body, .. } => {
                collect_expression_reflection(iterable, calls, backend_calls);
                total += collect_statement_reflection(body, statement_kinds, calls, backend_calls);
            }
            Statement::Loop { body } => {
                total += collect_statement_reflection(body, statement_kinds, calls, backend_calls);
            }
            Statement::StructDef { .. }
            | Statement::Import { .. }
            | Statement::Spawn { .. }
            | Statement::Break
            | Statement::Continue => {}
        }
    }
    total
}

fn collect_expression_reflection(
    expression: &Expression,
    calls: &mut BTreeMap<String, usize>,
    backend_calls: &mut BTreeMap<String, usize>,
) {
    match expression {
        Expression::Binary { left, right, .. } => {
            collect_expression_reflection(left, calls, backend_calls);
            collect_expression_reflection(right, calls, backend_calls);
        }
        Expression::Unary { operand, .. } => {
            collect_expression_reflection(operand, calls, backend_calls)
        }
        Expression::Call { callee, args } => {
            if let Expression::Identifier(name) = callee.as_ref() {
                bump_count(calls, name);
            }
            collect_expression_reflection(callee, calls, backend_calls);
            for arg in args {
                collect_expression_reflection(arg, calls, backend_calls);
            }
        }
        Expression::BackendCall {
            backend,
            method,
            args,
        } => {
            bump_count(backend_calls, &format!("{}.{}", backend, method));
            for arg in args {
                collect_expression_reflection(arg, calls, backend_calls);
            }
        }
        Expression::List(items) => {
            for item in items {
                collect_expression_reflection(item, calls, backend_calls);
            }
        }
        Expression::Map(items) | Expression::StructLiteral { fields: items, .. } => {
            for (_, value) in items {
                collect_expression_reflection(value, calls, backend_calls);
            }
        }
        Expression::Field { target, .. } => {
            collect_expression_reflection(target, calls, backend_calls)
        }
        Expression::Index { target, index } => {
            collect_expression_reflection(target, calls, backend_calls);
            collect_expression_reflection(index, calls, backend_calls);
        }
        Expression::MethodCall { target, args, .. } => {
            collect_expression_reflection(target, calls, backend_calls);
            for arg in args {
                collect_expression_reflection(arg, calls, backend_calls);
            }
        }
        Expression::Int(_)
        | Expression::Float(_)
        | Expression::Bool(_)
        | Expression::String(_)
        | Expression::Unit
        | Expression::Identifier(_) => {}
    }
}

fn collect_instruction_histogram(
    instructions: &[Instruction],
    opcodes: &mut BTreeMap<String, usize>,
) {
    for instruction in instructions {
        bump_count(opcodes, instruction_kind(instruction));
    }
}

fn bump_count(map: &mut BTreeMap<String, usize>, key: &str) {
    *map.entry(key.to_string()).or_insert(0) += 1;
}

fn usize_map_json(map: &BTreeMap<String, usize>) -> String {
    let fields: Vec<String> = map
        .iter()
        .map(|(key, value)| format!("\"{}\":{}", json_escape(key), value))
        .collect();
    format!("{{{}}}", fields.join(","))
}

fn statement_kind(statement: &Statement) -> &'static str {
    match statement {
        Statement::Let { .. } => "Let",
        Statement::Set { .. } => "Set",
        Statement::SetIndex { .. } => "SetIndex",
        Statement::SetField { .. } => "SetField",
        Statement::Print(_) => "Print",
        Statement::FunctionDef { .. } => "FunctionDef",
        Statement::StructDef { .. } => "StructDef",
        Statement::Import { .. } => "Import",
        Statement::OnEvent { .. } => "OnEvent",
        Statement::Spawn { .. } => "Spawn",
        Statement::If { .. } => "If",
        Statement::While { .. } => "While",
        Statement::For { .. } => "For",
        Statement::Loop { .. } => "Loop",
        Statement::Break => "Break",
        Statement::Continue => "Continue",
        Statement::Return(_) => "Return",
        Statement::Expression(_) => "Expression",
    }
}

fn instruction_kind(instruction: &Instruction) -> &'static str {
    match instruction {
        Instruction::LoadConst(_) => "LoadConst",
        Instruction::LoadGlobal(_) => "LoadGlobal",
        Instruction::StoreGlobal(_) => "StoreGlobal",
        Instruction::LoadLocal(_) => "LoadLocal",
        Instruction::StoreLocal(_) => "StoreLocal",
        Instruction::LoadParam(_) => "LoadParam",
        Instruction::StoreExisting(_) => "StoreExisting",
        Instruction::PushScope => "PushScope",
        Instruction::PopScope => "PopScope",
        Instruction::Add => "Add",
        Instruction::Sub => "Sub",
        Instruction::Mul => "Mul",
        Instruction::Div => "Div",
        Instruction::Mod => "Mod",
        Instruction::Neg => "Neg",
        Instruction::And => "And",
        Instruction::Or => "Or",
        Instruction::Not => "Not",
        Instruction::Eq => "Eq",
        Instruction::NotEq => "NotEq",
        Instruction::Lt => "Lt",
        Instruction::Gt => "Gt",
        Instruction::LtEq => "LtEq",
        Instruction::GtEq => "GtEq",
        Instruction::Jump(_) => "Jump",
        Instruction::JumpIfFalse(_) => "JumpIfFalse",
        Instruction::Call(_) => "Call",
        Instruction::CallNamed { .. } => "CallNamed",
        Instruction::Return => "Return",
        Instruction::SpawnEvent(_) => "SpawnEvent",
        Instruction::Print => "Print",
        Instruction::BackendCall { .. } => "BackendCall",
        Instruction::NewList(_) => "NewList",
        Instruction::LoadIndex => "LoadIndex",
        Instruction::StoreIndex => "StoreIndex",
        Instruction::StoreIndexVar(_) => "StoreIndexVar",
        Instruction::ListPush => "ListPush",
        Instruction::ListPop => "ListPop",
        Instruction::ListLen => "ListLen",
        Instruction::ListPushVar(_) => "ListPushVar",
        Instruction::ListPopVar(_) => "ListPopVar",
        Instruction::NewMap(_) => "NewMap",
        Instruction::MapHas => "MapHas",
        Instruction::MapKeys => "MapKeys",
        Instruction::MapValues => "MapValues",
        Instruction::NewStruct(_, _) => "NewStruct",
        Instruction::LoadField(_) => "LoadField",
        Instruction::StoreFieldVar { .. } => "StoreFieldVar",
        Instruction::Pop => "Pop",
        Instruction::Halt => "Halt",
    }
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

fn run_bytecode_json(path: &str, with_energy: bool) {
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
            "{{\"ok\":false,\"stage\":\"runtime\",\"input\":\"{}\",\"error\":{{\"message\":\"{}\"}},\"output\":{}{}}}",
            json_escape(path),
            json_escape(&error),
            json_string_array(&runtime.take_output()),
            energy_json_fragment(
                with_energy,
                runtime.vm().estimated_instruction_cost(),
                runtime.vm().estimated_backend_cost()
            )
        );
        process::exit(1);
    }

    println!(
        "{{\"ok\":true,\"input\":\"{}\",\"output\":{}{}}}",
        json_escape(path),
        json_string_array(&runtime.take_output()),
        energy_json_fragment(
            with_energy,
            runtime.vm().estimated_instruction_cost(),
            runtime.vm().estimated_backend_cost()
        )
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
            Constant::Float(value) => format!(
                "{{\"index\":{},\"type\":\"float\",\"value\":{}}}",
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
            Constant::Unit => format!("{{\"index\":{},\"type\":\"unit\",\"value\":null}}", index),
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
    println!(
        "│ Constants:         {:>6}                                      │",
        bytecode.constants.len()
    );
    println!(
        "│ Functions:         {:>6}                                      │",
        bytecode.functions.len()
    );
    println!(
        "│ Event Handlers:    {:>6}                                      │",
        bytecode.event_handlers.len()
    );
    println!(
        "│ Main Instructions: {:>6}                                      │",
        bytecode.main_instructions.len()
    );
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    if !bytecode.constants.is_empty() {
        println!("┌─ Constants Pool ───────────────────────────────────────────────┐");
        for (i, constant) in bytecode.constants.iter().enumerate() {
            print!("│ {:>4}: ", i);
            match constant {
                Constant::Int(n) => println!("{:<54} │", format!("Int({})", n)),
                Constant::Float(n) => println!("{:<54} │", format!("Float({})", n)),
                Constant::Bool(b) => println!("{:<54} │", format!("Bool({})", b)),
                Constant::String(s) => {
                    let display = if s.len() > 45 {
                        format!("String(\"{}...\")", &s[..42])
                    } else {
                        format!("String(\"{}\")", s)
                    };
                    println!("{:<54} │", display)
                }
                Constant::Unit => println!("{:<54} │", "Unit"),
            }
        }
        println!("└────────────────────────────────────────────────────────────────┘");
        println!();
    }

    if !bytecode.functions.is_empty() {
        println!("┌─ Functions ────────────────────────────────────────────────────┐");
        for (name, func) in &bytecode.functions {
            println!(
                "│ {} ({} params, {} instructions)",
                name,
                func.param_count,
                func.instructions.len()
            );
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
            println!(
                "│ on {} ({} instructions)",
                event,
                handler.instructions.len()
            );
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

fn print_instruction(
    index: usize,
    instr: &matter_bytecode::Instruction,
    constants: &[matter_bytecode::Constant],
    prefix: &str,
) {
    use matter_bytecode::{Constant, Instruction};

    print!("{}{:>4}: ", prefix, index);

    match instr {
        Instruction::LoadConst(id) => {
            let const_val = match &constants[*id] {
                Constant::Int(n) => format!("{}", n),
                Constant::Float(n) => format!("{}", n),
                Constant::Bool(b) => format!("{}", b),
                Constant::String(s) => {
                    if s.len() > 20 {
                        format!("\"{}...\"", &s[..17])
                    } else {
                        format!("\"{}\"", s)
                    }
                }
                Constant::Unit => "()".to_string(),
            };
            println!(
                "{:<20} ; const[{}] = {}",
                format!("LoadConst({})", id),
                id,
                const_val
            );
        }
        Instruction::Jump(target) => {
            println!("{:<20} ; -> {}", format!("Jump({})", target), target);
        }
        Instruction::JumpIfFalse(target) => {
            println!(
                "{:<20} ; -> {} if false",
                format!("JumpIfFalse({})", target),
                target
            );
        }
        Instruction::LoadGlobal(name) => {
            println!(
                "{:<20} ; load {}",
                format!("LoadGlobal(\"{}\")", name),
                name
            );
        }
        Instruction::StoreGlobal(name) => {
            println!(
                "{:<20} ; store {}",
                format!("StoreGlobal(\"{}\")", name),
                name
            );
        }
        Instruction::LoadLocal(name) => {
            println!(
                "{:<20} ; load local {}",
                format!("LoadLocal(\"{}\")", name),
                name
            );
        }
        Instruction::LoadParam(index) => {
            println!(
                "{:<20} ; load parameter {}",
                format!("LoadParam({})", index),
                index
            );
        }
        Instruction::StoreLocal(name) => {
            println!(
                "{:<20} ; store local {}",
                format!("StoreLocal(\"{}\")", name),
                name
            );
        }
        Instruction::StoreExisting(name) => {
            println!(
                "{:<20} ; update existing {}",
                format!("StoreExisting(\"{}\")", name),
                name
            );
        }
        Instruction::Add => println!("{:<20} ; pop b, pop a, push a+b", "Add"),
        Instruction::Sub => println!("{:<20} ; pop b, pop a, push a-b", "Sub"),
        Instruction::Mul => println!("{:<20} ; pop b, pop a, push a*b", "Mul"),
        Instruction::Div => println!("{:<20} ; pop b, pop a, push a/b", "Div"),
        Instruction::Mod => println!("{:<20} ; pop b, pop a, push a%b", "Mod"),
        Instruction::Neg => println!("{:<20} ; pop value, push -value", "Neg"),
        Instruction::And => println!("{:<20} ; pop b, pop a, push a&&b", "And"),
        Instruction::Or => println!("{:<20} ; pop b, pop a, push a||b", "Or"),
        Instruction::Not => println!("{:<20} ; pop value, push !value", "Not"),
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
        Instruction::CallNamed { name, arg_count } => {
            println!(
                "{:<20} ; call {} with {} args",
                format!("CallNamed(\"{}\", {})", name, arg_count),
                name,
                arg_count
            );
        }
        Instruction::Return => println!("{:<20} ; return from function", "Return"),
        Instruction::SpawnEvent(event) => {
            println!(
                "{:<20} ; enqueue event {}",
                format!("SpawnEvent(\"{}\")", event),
                event
            );
        }
        Instruction::Halt => println!("{:<20} ; stop execution", "Halt"),
        Instruction::BackendCall {
            backend,
            method,
            arg_count,
        } => {
            println!(
                "{:<20} ; {}.{}({})",
                format!("BackendCall"),
                backend,
                method,
                arg_count
            );
        }
        Instruction::NewList(size) => {
            println!(
                "{:<20} ; pop {} values, push list",
                format!("NewList({})", size),
                size
            );
        }
        Instruction::LoadIndex => {
            println!(
                "{:<20} ; pop index, pop collection, push value",
                "LoadIndex"
            );
        }
        Instruction::StoreIndex => {
            println!(
                "{:<20} ; pop value, pop index, pop collection, store value",
                "StoreIndex"
            );
        }
        Instruction::StoreIndexVar(name) => {
            println!(
                "{:<20} ; mutate {}[index]",
                format!("StoreIndexVar(\"{}\")", name),
                name
            );
        }
        Instruction::ListPush => {
            println!(
                "{:<20} ; pop value, pop list, push updated list",
                "ListPush"
            );
        }
        Instruction::ListPop => {
            println!("{:<20} ; pop list, push value and updated list", "ListPop");
        }
        Instruction::ListLen => {
            println!("{:<20} ; pop list, push length", "ListLen");
        }
        Instruction::ListPushVar(name) => {
            println!(
                "{:<20} ; mutate {}.push(value)",
                format!("ListPushVar(\"{}\")", name),
                name
            );
        }
        Instruction::ListPopVar(name) => {
            println!(
                "{:<20} ; mutate {}.pop(), push value",
                format!("ListPopVar(\"{}\")", name),
                name
            );
        }
        Instruction::NewMap(size) => {
            println!(
                "{:<20} ; pop {} key/value pairs, push map",
                format!("NewMap({})", size),
                size
            );
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
            println!(
                "{:<20} ; pop struct/map, push field",
                format!("LoadField(\"{}\")", field)
            );
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

fn compile_source(source: &str, _label: &str) -> Result<Bytecode, String> {
    let mut parser = Parser::from_source(source);
    let program = parser.parse().map_err(|e| e.to_string())?;
    BytecodeBuilder::new()
        .build_checked(&program)
        .map_err(|e| e.to_string())
}

fn print_semantic_error(error: &SemanticError) {
    eprintln!("Semantic error: {}", error);
}

fn print_help() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    Matter CLI - Help                           ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!(
        "Matter Core Language Runtime v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("USAGE:");
    println!("    matter-cli <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!();
    println!("  Source Execution:");
    println!("    run <file>              Run Matter source file");
    println!("    eval <source>           Evaluate Matter source code");
    println!("    emit <file> <event>     Emit event in Matter program");
    println!("    check <file>            Validate source without running");
    println!("    init [dir]              Create a new Matter project");
    println!();
    println!("  Native Compilation:");
    println!("    compile-native <file> [-o out] [-O0|-O1|-O2|-O3]");
    println!("                            Compile to native executable (x86-64)");
    println!("    run-native <file> [-O0|-O1|-O2|-O3]");
    println!("                            Compile and run as native code");
    println!();
    println!("  Bytecode Operations:");
    println!("    compile <file> [-o out] Compile source to bytecode (.mbc)");
    println!("    run-bytecode <file>     Execute bytecode file");
    println!("    emit-bytecode <file> <event>  Emit event from bytecode");
    println!("    inspect <file>          Inspect bytecode structure");
    println!();
    println!("  JSON API (machine-readable output):");
    println!("    capabilities-json       Print CLI capabilities");
    println!("    tool-ci-catalog-json    Print CI reason/code catalog");
    println!("    tool-ci-contract-json   Print CI contract bundle");
    println!(
        "    tool-pipeline-validate-contract-json <file.json> Validate pipeline contract output"
    );
    println!("    tool-pipeline-normalize-contract-json <in.json> [out.json] Normalize pipeline contract output");
    println!("    tool-pipeline-contract-example-json  Print canonical pipeline contract examples");
    println!("    tool-pipeline-contract-selftest-json Run contract self-tests");
    println!("    tool-pipeline-contract-ci-gate-json <file.json> [--warn-as-fail] Evaluate CI gate from contract");
    println!("    tool-pipeline-contract-diff-json <baseline.json> <candidate.json> Compare contract compatibility");
    println!("    tool-pipeline-contract-upgrade-advice-json <baseline.json> <candidate.json> Generate migration advice from contract diff");
    println!("    tool-pipeline-contract-bundle-json <baseline.json> <candidate.json> Emit combined diff+advice bundle");
    println!(
        "    tool-pipeline-contract-bundle-example-json  Print baseline/candidate/bundle example"
    );
    println!("    tool-pipeline-apply-next-cycle-json <next_cycle_config.json> Apply recommended next-cycle config");
    println!("    tool-pipeline-demo-json ... [--next-cycle-apply-now] Execute next cycle immediately after generating config");
    println!("    run-json <file>         Run and output JSON");
    println!("    run-energy <file>       Run and output energy report");
    println!("    run-energy-json <file>  Run and output JSON energy report");
    println!("    benchmark-json <file>   Benchmark and output JSON stats");
    println!("    perf-diagnose-json <file> Diagnose hot-path/performance pressure");
    println!("    benchmark-gate-json <benchmark.json> Check benchmark budgets");
    println!("    init-json [dir]         Create a new Matter project as JSON");
    println!("    doctor-json             Check local workspace health as JSON");
    println!(
        "    visual-step-json <file> <events> <delta_ms> Run one visual frame and bridge events"
    );
    println!(
        "    visual-run-json <file> <events> <frames> <delta_ms> Run visual loop and bridge events"
    );
    println!("    check-json <file>       Validate and output JSON");
    println!("    reflect-json <file>     Inspect source AST and bytecode");
    println!("    reflexive-guard-json <file> Evaluate reflexive safety gates");
    println!("    compile-json <file>     Compile and output JSON");
    println!("    tokens-json <file>      Tokenize and output JSON");
    println!("    imports-json <file>     Inspect imports as JSON");
    println!();
    println!("  Package Management:");
    println!("    package-json [manifest] Inspect package manifest");
    println!("    project-run-json [manifest] [--with-energy] Run package entrypoint");
    println!("    project-check-json [manifest]   Validate package");
    println!("    project-compile-json [manifest] Compile package");
    println!();
    println!("  Utilities:");
    println!("    help [command]          Show help (optionally for specific command)");
    println!("    version                 Show version information");
    println!("    doctor                  Check local workspace health");
    println!("    backends                List available backends");
    println!("    examples [name]         List or run example programs");
    println!("    repl                    Start interactive REPL shell");
    println!("    agent-chat              Start interactive AI chat in CLI");
    println!();
    println!("NOTES:");
    println!("  • Use '-' as filename to read from stdin");
    println!("  • Bytecode format: MBC1 (Matter Bytecode v1)");
    println!("  • Default manifest: matter.toml");
    println!("  • Default output: output.mbc");
    println!();
    println!("EXAMPLES:");
    println!("  matter-cli run hello.matter");
    println!("  matter-cli compile app.matter -o app.mbc");
    println!("  matter-cli run-bytecode app.mbc");
    println!("  matter-cli emit events.matter tap");
    println!("  matter-cli compile-native fibonacci.matter -o fib -O3");
    println!("  matter-cli run-native fibonacci.matter -O3");
    println!("  matter-cli help run");
    println!();
    println!("For more information, visit: https://github.com/matter-lang/matter-core");
}

fn print_command_help(command: &str) {
    match command {
        "run" => {
            println!("matter-cli run - Execute Matter source file");
            println!();
            println!("USAGE:");
            println!("    matter-cli run <file.matter|->");
            println!();
            println!("DESCRIPTION:");
            println!("    Executes a Matter source file through the complete pipeline:");
            println!("    Source → Lexer → Parser → AST → Bytecode → VM → Runtime");
            println!();
            println!("ARGUMENTS:");
            println!("    <file>    Path to .matter source file, or '-' for stdin");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli run hello.matter");
            println!("    echo 'print \"Hello\"' | matter-cli run -");
        }
        "compile" => {
            println!("matter-cli compile - Compile source to bytecode");
            println!();
            println!("USAGE:");
            println!("    matter-cli compile <file.matter|-> [-o output.mbc]");
            println!();
            println!("DESCRIPTION:");
            println!("    Compiles Matter source code to MBC1 bytecode format.");
            println!("    The bytecode can be distributed and executed without source.");
            println!();
            println!("ARGUMENTS:");
            println!("    <file>    Path to .matter source file, or '-' for stdin");
            println!();
            println!("OPTIONS:");
            println!("    -o <output>    Output bytecode file (default: output.mbc)");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli compile app.matter");
            println!("    matter-cli compile app.matter -o app.mbc");
        }
        "run-bytecode" => {
            println!("matter-cli run-bytecode - Execute bytecode file");
            println!();
            println!("USAGE:");
            println!("    matter-cli run-bytecode <file.mbc>");
            println!();
            println!("DESCRIPTION:");
            println!("    Executes a compiled MBC1 bytecode file directly.");
            println!("    Faster than running source, no parsing/compilation needed.");
            println!();
            println!("ARGUMENTS:");
            println!("    <file>    Path to .mbc bytecode file");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli run-bytecode app.mbc");
        }
        "inspect" => {
            println!("matter-cli inspect - Inspect bytecode structure");
            println!();
            println!("USAGE:");
            println!("    matter-cli inspect <file.mbc>");
            println!();
            println!("DESCRIPTION:");
            println!("    Displays detailed information about a bytecode file:");
            println!("    • Constants pool");
            println!("    • Function definitions");
            println!("    • Event handlers");
            println!("    • Instruction listing with annotations");
            println!();
            println!("ARGUMENTS:");
            println!("    <file>    Path to .mbc bytecode file");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli inspect app.mbc");
        }
        "emit" => {
            println!("matter-cli emit - Emit event in Matter program");
            println!();
            println!("USAGE:");
            println!("    matter-cli emit <file.matter|-> <event>");
            println!();
            println!("DESCRIPTION:");
            println!("    Loads a Matter program and triggers a specific event handler.");
            println!("    Useful for testing event-driven behavior.");
            println!();
            println!("ARGUMENTS:");
            println!("    <file>     Path to .matter source file, or '-' for stdin");
            println!("    <event>    Name of event to emit (e.g., 'tap', 'boot')");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli emit events.matter tap");
            println!("    matter-cli emit app.matter boot");
        }
        "check" => {
            println!("matter-cli check - Validate source without running");
            println!();
            println!("USAGE:");
            println!("    matter-cli check <file.matter|->");
            println!();
            println!("DESCRIPTION:");
            println!("    Validates Matter source code through lexing, parsing,");
            println!("    and semantic analysis without executing it.");
            println!("    Reports syntax errors, type errors, and undefined variables.");
            println!();
            println!("ARGUMENTS:");
            println!("    <file>    Path to .matter source file, or '-' for stdin");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli check app.matter");
        }
        "reflect-json" => {
            println!("matter-cli reflect-json - Inspect Matter source as structured data");
            println!();
            println!("USAGE:");
            println!("    matter-cli reflect-json <file.matter|->");
            println!();
            println!("DESCRIPTION:");
            println!("    Parses and compiles Matter source without executing it, then emits");
            println!("    JSON describing AST shape, calls, backend calls, functions, events,");
            println!("    and bytecode opcode distribution.");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli reflect-json examples/reflexive_self.matter");
        }
        "reflexive-guard-json" => {
            println!("matter-cli reflexive-guard-json - Evaluate reflexive safety gates");
            println!();
            println!("USAGE:");
            println!("    matter-cli reflexive-guard-json <file.matter|-> [--max-statements N] [--max-functions N] [--allow-backends]");
            println!();
            println!("DESCRIPTION:");
            println!("    Parses and compiles Matter source without executing it, then emits");
            println!(
                "    JSON with policy checks for guarded self-analysis and future self-mutation."
            );
            println!();
            println!("OPTIONS:");
            println!("    --max-statements N  Maximum allowed total statements (default: 200)");
            println!("    --max-functions N   Maximum allowed functions (default: 50)");
            println!("    --allow-backends    Permit backend calls during reflexive review");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli reflexive-guard-json examples/reflexive_self.matter");
        }
        "init" | "init-json" => {
            print_init_help();
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli init my-app");
            println!("    matter-cli init . --name my-app");
            println!("    matter-cli init-json my-app --template event");
        }
        "backends" => {
            println!("matter-cli backends - List available backends");
            println!();
            println!("USAGE:");
            println!("    matter-cli backends");
            println!();
            println!("DESCRIPTION:");
            println!("    Lists all available backend systems and their methods.");
            println!("    Backends provide external functionality to Matter programs.");
        }
        "examples" => {
            println!("matter-cli examples - List or run example programs");
            println!();
            println!("USAGE:");
            println!("    matter-cli examples [name]");
            println!();
            println!("DESCRIPTION:");
            println!("    Without arguments: lists all available examples");
            println!("    With name: runs the specified example");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli examples");
            println!("    matter-cli examples hello");
        }
        "version" => {
            println!("matter-cli version - Show version information");
            println!();
            println!("USAGE:");
            println!("    matter-cli version");
            println!();
            println!("DESCRIPTION:");
            println!("    Displays detailed version information about Matter Core.");
        }
        "repl" => {
            println!("matter-cli repl - Start interactive REPL shell");
            println!();
            println!("USAGE:");
            println!("    matter-cli repl");
            println!();
            println!("DESCRIPTION:");
            println!("    Starts an interactive Read-Eval-Print Loop (REPL) shell.");
            println!("    Execute Matter code line by line with persistent state.");
            println!();
            println!("SPECIAL COMMANDS:");
            println!("    :help       Show REPL help");
            println!("    :quit       Exit REPL");
            println!("    :clear      Clear screen");
            println!("    :reset      Reset runtime state");
            println!("    :vars       Show all variables");
            println!("    :backends   List available backends");
            println!();
            println!("FEATURES:");
            println!("    • Persistent state between commands ✓");
            println!("    • Multi-line input for blocks");
            println!("    • Command history");
            println!("    • Immediate feedback");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli repl");
        }
        "agent-chat" => {
            println!("matter-cli agent-chat - Interactive AI chat in CLI");
            println!();
            println!("USAGE:");
            println!("    matter-cli agent-chat [--provider PROVIDER] [--profile PROFILE] [--model MODEL] [--system PROMPT] [--api-key KEY] [--list-models] [--specialist]");
            println!();
            println!("ENV:");
            println!("    OPENAI_API_KEY or MATTER_AGENT_API_KEY");
            println!("    MATTER_AGENT_MODEL (optional, overrides profile)");
            println!("    MATTER_AGENT_PROFILE (optional, default coding)");
            println!("    MATTER_AGENT_PROVIDER (optional, default openai)");
            println!("    MATTER_AGENT_SPECIALIST (optional, 1/true enables specialist mode)");
            println!();
            println!("PROVIDERS:");
            println!("    openai");
            println!("    nvidia");
            println!();
            println!("PROFILES:");
            println!("    coding   -> gpt-5.2-codex");
            println!("    balanced -> gpt-5.2");
            println!("    cheap    -> gpt-5-mini");
            println!("    max      -> gpt-5.2-pro");
            println!();
            println!("COMMANDS:");
            println!("    /exit, /quit    Exit chat");
            println!("    /clear          Reset conversation context");
            println!("    /context        Inject env/agent_workspace_context.md into chat");
            println!("    /memory         Show persisted memory notes");
            println!("    /remember TEXT  Save a note to env/agent_memory.json");
            println!("    /forget         Clear persisted memory notes");
            println!("    /status         Show git status --short");
            println!("    /check-cli      Run cargo check -p matter-cli");
            println!("    /clippy-cli     Run strict clippy for matter-cli");
            println!("    /check-workspace Run cargo check --workspace (summarized)");
            println!("    /test-workspace  Run cargo test --workspace -q (summarized)");
            println!("    /fix-plan        Generate fix plan from diagnostics");
            println!("    /autofix-plan    Generate prioritized patch suggestions");
            println!("    /autofix-plan-json Emit autofix plan as JSON");
            println!("    /autofix-apply   Show top-priority assisted patch action");
            println!("    /doctor          Run health checks (git/check/clippy/test)");
            println!("    /doctor-json     Emit health checks as JSON");
            println!("    /snapshot        Emit current agent/session snapshot JSON");
            println!("    /save-session    Persist current chat session JSON");
            println!("    /save-session TAG Persist session with custom tag");
            println!("    /sessions [--json] [--tag T] [--provider P] [--model M] [--contains TXT] [--from TS] [--to TS] [--last 30m|12h|7d] [--sort saved_at|messages] [--limit N]");
            println!("    /load-session    Load saved session (latest or path)");
            println!("    /session-info    Show loaded session metadata");
            println!("    /delete-session  Delete one session (latest or path)");
            println!("    /prune-sessions N Keep only N latest sessions");
            println!("    /session-search T Search text across saved sessions");
            println!("    /session-tail X [N] Show last N messages from session");
            println!("    /session-stats   Show aggregate stats for saved sessions");
            println!("    /session-report [--json|--md|--csv|--html] [--from TS] [--to TS] [--last 30m|12h|7d] [--top N] Show operational report for sessions");
            println!(
                "    /reports [--json] [--latest] [--limit N] [--ext md|csv|html] [--sort modified|name|size] List generated reports"
            );
            println!("    /report-open latest [--ext md|csv|html] Print path to latest report");
            println!("    /report-delete latest [--ext md|csv|html] Delete latest report");
            println!("    /report-prune N [--ext md|csv|html] Keep only N latest reports");
            println!("    /session-export-md X Export session to Markdown");
            println!("    /session-diff [--json] A B Compare two sessions (use latest/latest-N/latest@tag)");
            println!("    /run CMD         Run safe whitelisted dev commands");
        }
        "lsp" => {
            println!("matter-cli lsp - Start Language Server Protocol server");
            println!();
            println!("USAGE:");
            println!("    matter-cli lsp");
            println!();
            println!("DESCRIPTION:");
            println!("    Starts the Matter Language Server Protocol (LSP) server.");
            println!("    Provides IDE features via JSON-RPC over stdin/stdout.");
            println!();
            println!("FEATURES:");
            println!("    • Diagnostics (errors and warnings)");
            println!("    • Autocomplete (variables, functions, backends)");
            println!("    • Go-to-definition");
            println!("    • Hover information");
            println!("    • Find references");
            println!("    • Rename symbol");
            println!("    • Document symbols");
            println!();
            println!("EDITOR INTEGRATION:");
            println!("    VS Code: Install Matter Language extension");
            println!("    Neovim: Configure with lspconfig");
            println!("    Other: Any LSP-compatible editor");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli lsp");
        }
        "debug" => {
            println!("matter-cli debug - Start interactive debugger");
            println!();
            println!("USAGE:");
            println!("    matter-cli debug <file.matter>");
            println!();
            println!("DESCRIPTION:");
            println!("    Starts an interactive debugging session for Matter programs.");
            println!("    Allows setting breakpoints, stepping through code, and");
            println!("    inspecting variables.");
            println!();
            println!("FEATURES:");
            println!("    • Breakpoints (line-based)");
            println!("    • Step execution (into, over, out)");
            println!("    • Variable inspection (locals, globals)");
            println!("    • Call stack visualization");
            println!("    • Continue/pause execution");
            println!();
            println!("DEBUG COMMANDS:");
            println!("    break <line>    Set breakpoint");
            println!("    continue, c     Continue execution");
            println!("    step, s         Step into");
            println!("    next, n         Step over");
            println!("    out, o          Step out");
            println!("    locals          Show local variables");
            println!("    globals         Show global variables");
            println!("    stack           Show call stack");
            println!("    quit, q         Exit debugger");
            println!();
            println!("EXAMPLES:");
            println!("    matter-cli debug app.matter");
        }
        _ => {
            eprintln!("No help available for command: {}", command);
            eprintln!();
            eprintln!("Run 'matter-cli help' to see all available commands.");
            process::exit(1);
        }
    }
}

fn print_version() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    Matter Core                                 ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Version:        {}", env!("CARGO_PKG_VERSION"));
    println!("Bytecode:       MBC1 (Matter Bytecode v1)");
    println!("Build:          {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("FEATURES:");
    println!("  ✓ Variables & Functions");
    println!("  ✓ Recursion & Closures");
    println!("  ✓ Control Flow (if/while/loop/for)");
    println!("  ✓ Data Types (int/bool/string/list/map/struct)");
    println!("  ✓ Event System");
    println!("  ✓ Backend Calls");
    println!("  ✓ Bytecode Compilation");
    println!("  ✓ Package System");
    println!();
    println!("BACKENDS:");
    println!("  • agent    - AI/LLM integration");
    println!("  • visual   - PVM/PXL visual system");
    println!("  • store    - Persistent storage");
    println!("  • net      - Network/HTTP");
    println!("  • math     - Mathematical operations");
    println!("  • string   - String manipulation");
    println!("  • list     - List operations");
    println!("  • time     - Time and delays");
    println!("  • random   - Random number generation");
    println!("  • json     - JSON parsing/serialization");
    println!();
    println!("Repository:     https://github.com/matter-lang/matter-core");
    println!("Documentation:  https://matter-lang.org/docs");
    println!("License:        MIT");
}

fn print_backends() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║              Matter Core - Available Backends                  ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Backends provide external functionality to Matter programs.");
    println!("Call them using: backend.method(args)");
    println!();

    println!("┌─ agent (AI/LLM Integration) ───────────────────────────────────┐");
    println!("│ agent.say(message)          Send message to AI agent           │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("┌─ visual (PVM/PXL Visual System) ───────────────────────────────┐");
    println!("│ visual.run(app_name)        Execute visual application         │");
    println!("│ visual.load(path)           Load PVMBC bytecode                │");
    println!("│ visual.surface(name, w, h)  Create visual surface              │");
    println!("│ visual.region(name, x, y, w, h)  Create visual region          │");
    println!("│ visual.pulse(target)        Animate visual element             │");
    println!("│ visual.set(target, key, value)  Set visual property            │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("┌─ store (Persistent Storage) ───────────────────────────────────┐");
    println!("│ store.set(key, value)       Store persistent value             │");
    println!("│ store.get(key)              Retrieve stored value              │");
    println!("│ store.has(key)              Check if key exists                │");
    println!("│ store.delete(key)           Remove stored value                │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("┌─ net (Network/HTTP) ───────────────────────────────────────────┐");
    println!("│ net.get(url)                HTTP GET request                   │");
    println!("│ net.post(url, body)         HTTP POST request                  │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("┌─ energy (Virtual Energy Engine) ───────────────────────────────┐");
    println!("│ energy.cpu()                Estimated CPU usage                │");
    println!("│ energy.memory()             Estimated memory usage             │");
    println!("│ energy.mode()               Current energy mode                │");
    println!("│ energy.set_mode(mode)       Set mode: eco/balanced/...         │");
    println!("│ energy.profile(config)      Apply profile map                  │");
    println!("│ energy.score(name)          Efficiency score for task/backend  │");
    println!("│ energy.estimate(name)       Estimated cost breakdown           │");
    println!("│ energy.defer(name)          Should defer under current policy  │");
    println!("│ energy.cache(key, value)    Cache value for reuse              │");
    println!("│ energy.reuse(key)           Reuse cached value                 │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("┌─ math (Mathematical Operations) ───────────────────────────────┐");
    println!("│ math.abs(n)                 Absolute value                     │");
    println!("│ math.min(a, b)              Minimum of two numbers             │");
    println!("│ math.max(a, b)              Maximum of two numbers             │");
    println!("│ math.pow(base, exp)         Power/exponentiation               │");
    println!("│ math.sqrt(n)                Square root                        │");
    println!("│ math.mod(a, b)              Modulo operation                   │");
    println!("│ math.clamp(val, min, max)   Clamp value to range               │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("┌─ string (String Manipulation) ─────────────────────────────────┐");
    println!("│ string.len(s)               String length                      │");
    println!("│ string.upper(s)             Convert to uppercase               │");
    println!("│ string.lower(s)             Convert to lowercase               │");
    println!("│ string.trim(s)              Remove whitespace                  │");
    println!("│ string.split(s, delim)      Split into list                    │");
    println!("│ string.join(list, delim)    Join list into string              │");
    println!("│ string.contains(s, substr)  Check if contains substring        │");
    println!("│ string.replace(s, old, new) Replace substring                  │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("┌─ list (List Operations) ───────────────────────────────────────┐");
    println!("│ list.len(list)              List length                        │");
    println!("│ list.push(list, value)      Append value                       │");
    println!("│ list.pop(list)              Remove last value                  │");
    println!("│ list.get(list, index)       Get value at index                 │");
    println!("│ list.set(list, index, val)  Set value at index                 │");
    println!("│ list.contains(list, value)  Check if contains value            │");
    println!("│ list.reverse(list)          Reverse list                       │");
    println!("│ list.sort(list)             Sort list                          │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("┌─ time (Time and Delays) ───────────────────────────────────────┐");
    println!("│ time.now()                  Current timestamp (ms)             │");
    println!("│ time.sleep(ms)              Delay execution                    │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("┌─ random (Random Number Generation) ────────────────────────────┐");
    println!("│ random.int()                Random integer                     │");
    println!("│ random.bool()               Random boolean                     │");
    println!("│ random.choice(list)         Random element from list           │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("┌─ json (JSON Parsing/Serialization) ────────────────────────────┐");
    println!("│ json.stringify(value)       Convert to JSON string             │");
    println!("│ json.parse(json_string)     Parse JSON string                  │");
    println!("└────────────────────────────────────────────────────────────────┘");
    println!();

    println!("EXAMPLES:");
    println!("  agent.say(\"Hello from Matter!\")");
    println!("  let result = math.pow(2, 10)");
    println!("  let upper = string.upper(\"hello\")");
    println!("  store.set(\"counter\", 42)");
    println!("  visual.run(\"pizzaria\")");
}

fn list_examples() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║              Matter Core - Example Programs                    ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    let examples = vec![
        (
            "first_run",
            "First-run demo: VM execution, recursion, lists, and events",
        ),
        ("language_tour", "Executable tour of Matter syntax"),
        ("hello", "Simple hello world program"),
        ("functions", "Function definitions and calls"),
        ("events", "Event system demonstration"),
        ("backend", "Backend calls (agent, store, net)"),
        ("showcase", "Language features showcase"),
        ("visual_basic", "Basic visual backend commands"),
        ("visual_event", "Visual backend with events"),
        ("visual_advanced", "Advanced visual properties"),
        ("visual_load", "Loading PVMBC bytecode"),
        ("stdlib_demo", "Standard library demonstration"),
        ("json_api_demo", "JSON API usage examples"),
    ];

    println!("Available examples:");
    println!();
    for (name, description) in &examples {
        println!("  {:<20} {}", name, description);
    }
    println!();
    println!("USAGE:");
    println!("  matter-cli examples <name>    Run specific example");
    println!();
    println!("EXAMPLES:");
    println!("  matter-cli examples hello");
    println!("  matter-cli examples visual_basic");
}

fn run_example(name: &str) {
    let example_path = format!("examples/{}.matter", name);

    if !Path::new(&example_path).exists() {
        eprintln!("Error: Example '{}' not found", name);
        eprintln!();
        eprintln!("Run 'matter-cli examples' to see available examples.");
        process::exit(1);
    }

    println!("Running example: {}", name);
    println!("─────────────────────────────────────────────────────────────────");
    println!();

    run_file(&example_path);
}

fn suggest_command(input: &str) {
    let commands = vec![
        "run",
        "eval",
        "emit",
        "check",
        "compile",
        "run-bytecode",
        "emit-bytecode",
        "inspect",
        "run-json",
        "run-energy",
        "run-energy-json",
        "doctor",
        "doctor-json",
        "init",
        "init-json",
        "eval-json",
        "emit-json",
        "visual-step-json",
        "visual-run-json",
        "check-json",
        "reflect-json",
        "reflexive-guard-json",
        "compile-json",
        "tokens-json",
        "imports-json",
        "inspect-json",
        "run-bytecode-json",
        "emit-bytecode-json",
        "capabilities-json",
        "tool-ci-catalog-json",
        "tool-pipeline-validate-contract-json",
        "tool-pipeline-normalize-contract-json",
        "tool-pipeline-contract-example-json",
        "tool-pipeline-contract-selftest-json",
        "tool-pipeline-contract-ci-gate-json",
        "tool-pipeline-contract-diff-json",
        "tool-pipeline-contract-upgrade-advice-json",
        "tool-pipeline-contract-bundle-json",
        "tool-pipeline-contract-bundle-example-json",
        "tool-pipeline-apply-next-cycle-json",
        "tool-ci-verify-json",
        "tool-ci-contract-json",
        "package-json",
        "project-check-json",
        "project-run-json",
        "project-compile-json",
        "project-visual-step-build-json",
        "project-visual-run-build-json",
        "project-web-build-json",
        "web-serve-json",
        "project-web-serve-json",
        "project-web-smoke-json",
        "project-web-ci-json",
        "web-events-save-json",
        "web-state-json",
        "web-events-tail-json",
        "web-action-json",
        "web-actions-json",
        "project-web-step-live-json",
        "project-web-loop-live-json",
        "start-live-demo-json",
        "help",
        "version",
        "backends",
        "examples",
    ];

    // Simple Levenshtein distance for suggestions
    let mut suggestions: Vec<(&str, usize)> = commands
        .iter()
        .map(|cmd| (*cmd, levenshtein_distance(input, cmd)))
        .filter(|(_, dist)| *dist <= 3)
        .collect();

    suggestions.sort_by_key(|(_, dist)| *dist);

    if !suggestions.is_empty() {
        eprintln!("Did you mean:");
        for (cmd, _) in suggestions.iter().take(3) {
            eprintln!("    matter-cli {}", cmd);
        }
    }
}

fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
        row[0] = i;
    }
    for (j, cell) in matrix[0].iter_mut().enumerate().take(len2 + 1) {
        *cell = j;
    }

    for (i, c1) in s1.chars().enumerate() {
        for (j, c2) in s2.chars().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = [
                matrix[i][j + 1] + 1,
                matrix[i + 1][j] + 1,
                matrix[i][j] + cost,
            ]
            .iter()
            .min()
            .copied()
            .unwrap_or(0);
        }
    }

    matrix[len1][len2]
}

fn run_repl() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║              Matter REPL - Interactive Shell                   ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Matter Core v{}", env!("CARGO_PKG_VERSION"));
    println!("Type ':help' for help, ':quit' to exit");
    println!();

    // Create persistent runtime with empty bytecode
    let builder = BytecodeBuilder::new();
    let empty_program = Program { statements: vec![] };
    let mut accumulated_bytecode = match builder.build_checked(&empty_program) {
        Ok(bytecode) => bytecode,
        Err(e) => {
            eprintln!("Failed to initialize REPL bytecode: {}", e);
            return;
        }
    };
    let mut runtime = Runtime::new(accumulated_bytecode.clone());

    let mut line_number = 1;
    let mut history: Vec<String> = Vec::new();
    let mut multiline_buffer = String::new();
    let mut in_multiline = false;
    let mut accumulated_source = String::new();

    loop {
        // Print prompt
        let prompt = if in_multiline {
            "... ".to_string()
        } else {
            format!("[{}]> ", line_number)
        };
        print!("{}", prompt);
        if let Err(e) = io::stdout().flush() {
            eprintln!("Error flushing stdout: {}", e);
            break;
        }

        // Read line
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();

        // Handle empty input
        if input.is_empty() {
            if in_multiline {
                // Empty line in multiline mode - execute buffer
                if !multiline_buffer.is_empty() {
                    execute_repl_command_persistent(
                        &multiline_buffer,
                        &mut accumulated_source,
                        &mut accumulated_bytecode,
                        &mut runtime,
                        &mut history,
                    );
                    multiline_buffer.clear();
                    in_multiline = false;
                    line_number += 1;
                }
            }
            continue;
        }

        // Handle special commands (only when not in multiline)
        if !in_multiline && input.starts_with(':') {
            match input {
                ":help" => print_repl_help(),
                ":quit" | ":exit" | ":q" => {
                    println!("Goodbye!");
                    break;
                }
                ":clear" | ":cls" => {
                    print!("\x1B[2J\x1B[1;1H"); // ANSI clear screen
                    println!("Matter REPL v{}", env!("CARGO_PKG_VERSION"));
                    println!();
                }
                ":reset" => {
                    let builder = BytecodeBuilder::new();
                    let empty_program = Program { statements: vec![] };
                    accumulated_bytecode = match builder.build_checked(&empty_program) {
                        Ok(bytecode) => bytecode,
                        Err(e) => {
                            eprintln!("Error resetting runtime state: {}", e);
                            continue;
                        }
                    };
                    runtime = Runtime::new(accumulated_bytecode.clone());
                    accumulated_source.clear();
                    println!("Runtime state reset.");
                    line_number = 1;
                }
                ":vars" => {
                    let globals = runtime.get_globals();
                    if globals.is_empty() {
                        println!("No variables defined.");
                    } else {
                        println!("Variables:");
                        let mut vars: Vec<_> = globals.iter().collect();
                        vars.sort_by_key(|(name, _)| *name);
                        for (name, value) in vars {
                            println!("  {} = {:?}", name, value);
                        }
                    }
                }
                ":backends" => {
                    println!("Available backends:");
                    println!("  agent, visual, store, net, math, string, list, time, random, json");
                    println!("Type 'matter-cli backends' for detailed documentation.");
                }
                ":history" => {
                    println!("Command history:");
                    for (i, cmd) in history.iter().enumerate() {
                        println!("  {}: {}", i + 1, cmd);
                    }
                }
                _ => {
                    println!("Unknown command: {}", input);
                    println!("Type ':help' for available commands.");
                }
            }
            continue;
        }

        // Check for multiline input (lines ending with {, or incomplete blocks)
        if input.ends_with('{')
            || input.starts_with("fn ")
            || input.starts_with("if ")
            || input.starts_with("while ")
            || input.starts_with("loop")
            || input.starts_with("for ")
            || input.starts_with("on ")
        {
            in_multiline = true;
            multiline_buffer.push_str(input);
            multiline_buffer.push('\n');
            continue;
        }

        // If in multiline mode, accumulate
        if in_multiline {
            multiline_buffer.push_str(input);
            multiline_buffer.push('\n');

            // Check if block is complete (simple heuristic: count braces)
            let open_braces = multiline_buffer.matches('{').count();
            let close_braces = multiline_buffer.matches('}').count();

            if close_braces >= open_braces && open_braces > 0 {
                execute_repl_command_persistent(
                    &multiline_buffer,
                    &mut accumulated_source,
                    &mut accumulated_bytecode,
                    &mut runtime,
                    &mut history,
                );
                multiline_buffer.clear();
                in_multiline = false;
                line_number += 1;
            }
            continue;
        }

        // Execute single line
        execute_repl_command_persistent(
            input,
            &mut accumulated_source,
            &mut accumulated_bytecode,
            &mut runtime,
            &mut history,
        );
        line_number += 1;
    }
}

fn execute_repl_command_persistent(
    source: &str,
    accumulated_source: &mut String,
    accumulated_bytecode: &mut Bytecode,
    runtime: &mut Runtime,
    history: &mut Vec<String>,
) {
    // Add to history
    history.push(source.to_string());

    // Accumulate source
    accumulated_source.push_str(source);
    accumulated_source.push('\n');

    // Parse accumulated source
    let mut parser = Parser::from_source(accumulated_source);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            // Remove last command from accumulated source
            let lines: Vec<&str> = accumulated_source.lines().collect();
            *accumulated_source =
                lines[..lines.len().saturating_sub(source.lines().count())].join("\n");
            if !accumulated_source.is_empty() {
                accumulated_source.push('\n');
            }
            return;
        }
    };

    // Build bytecode from accumulated source
    let builder = BytecodeBuilder::new();
    let new_bytecode = match builder.build_checked(&program) {
        Ok(bytecode) => bytecode,
        Err(e) => {
            eprintln!("Semantic error: {}", e);
            // Remove last command from accumulated source
            let lines: Vec<&str> = accumulated_source.lines().collect();
            *accumulated_source =
                lines[..lines.len().saturating_sub(source.lines().count())].join("\n");
            if !accumulated_source.is_empty() {
                accumulated_source.push('\n');
            }
            return;
        }
    };

    // Update accumulated bytecode
    *accumulated_bytecode = new_bytecode.clone();

    // Create new runtime with accumulated bytecode
    let mut new_runtime = Runtime::new(new_bytecode);

    // Execute
    if let Err(e) = new_runtime.run() {
        eprintln!("Runtime error: {}", e);
        // Remove last command from accumulated source
        let lines: Vec<&str> = accumulated_source.lines().collect();
        *accumulated_source =
            lines[..lines.len().saturating_sub(source.lines().count())].join("\n");
        if !accumulated_source.is_empty() {
            accumulated_source.push('\n');
        }
        return;
    }

    // Update runtime reference (transfer state)
    let globals = new_runtime.get_globals();
    runtime.set_globals(globals);
}

fn print_repl_help() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                    Matter REPL - Help                          ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("SPECIAL COMMANDS:");
    println!("  :help           Show this help message");
    println!("  :quit, :exit    Exit REPL");
    println!("  :clear, :cls    Clear screen");
    println!("  :reset          Reset runtime state");
    println!("  :vars           Show all variables");
    println!("  :backends       List available backends");
    println!("  :history        Show command history");
    println!();
    println!("USAGE:");
    println!("  • Type Matter code and press Enter to execute");
    println!("  • Multi-line blocks (functions, if, loops) are supported");
    println!("  • Press Enter on empty line to execute multi-line block");
    println!("  • Use 'print' to see values");
    println!();
    println!("EXAMPLES:");
    println!("  let x = 10");
    println!("  print x");
    println!("  let y = x + 5");
    println!("  print y");
    println!();
    println!("  fn soma(a, b) {{");
    println!("      return a + b");
    println!("  }}");
    println!("  print soma(10, 20)");
    println!();
    println!("TIPS:");
    println!("  • Variables persist between commands");
    println!("  • Use :reset to start fresh");
    println!("  • Backend calls work: agent.say(\"hello\")");
    println!("  • Events can be defined and emitted");
}

fn run_agent_chat(args: &[String]) {
    let mut provider = env::var("MATTER_AGENT_PROVIDER").unwrap_or_else(|_| "openai".to_string());
    let mut profile = env::var("MATTER_AGENT_PROFILE").unwrap_or_else(|_| "coding".to_string());
    let mut model = env::var("MATTER_AGENT_MODEL").unwrap_or_else(|_| "gpt-5.2-codex".to_string());
    let mut system_prompt = "You are Matter Agent, a pragmatic coding assistant inside Matter CLI. Keep answers concise and actionable.".to_string();
    let mut specialist = env::var("MATTER_AGENT_SPECIALIST")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let mut api_key = env::var("OPENAI_API_KEY")
        .ok()
        .or_else(|| env::var("MATTER_AGENT_API_KEY").ok())
        .unwrap_or_default();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--model" if i + 1 < args.len() => {
                model = args[i + 1].clone();
                i += 2;
            }
            "--profile" if i + 1 < args.len() => {
                profile = args[i + 1].clone();
                i += 2;
            }
            "--provider" if i + 1 < args.len() => {
                provider = args[i + 1].clone();
                i += 2;
            }
            "--specialist" => {
                specialist = true;
                i += 1;
            }
            "--system" if i + 1 < args.len() => {
                system_prompt = args[i + 1].clone();
                i += 2;
            }
            "--api-key" if i + 1 < args.len() => {
                api_key = args[i + 1].clone();
                i += 2;
            }
            "--list-models" => {
                print_agent_model_catalog();
                if let Some(key) = resolve_agent_api_key("") {
                    match list_models_via_curl(&provider, &key) {
                        Ok(models) => {
                            if !models.is_empty() {
                                println!();
                                println!("Available from {} /v1/models:", provider);
                                for m in models.iter().take(40) {
                                    println!("  {}", m);
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Could not fetch live model list: {}", err);
                        }
                    }
                }
                return;
            }
            "--help" | "-h" => {
                println!("matter-cli agent-chat [--provider PROVIDER] [--profile PROFILE] [--model MODEL] [--system PROMPT] [--api-key KEY] [--list-models]");
                println!("Providers: openai, nvidia");
                println!("Profiles: coding, balanced, cheap, max");
                println!("Flags: --specialist");
                println!("Env fallback: OPENAI_API_KEY, MATTER_AGENT_API_KEY, MATTER_AGENT_MODEL, MATTER_AGENT_PROFILE, MATTER_AGENT_PROVIDER");
                println!("Commands: /exit, /quit, /clear, /context, /memory, /remember <text>, /forget, /status, /check-cli, /clippy-cli, /check-workspace, /test-workspace, /fix-plan, /autofix-plan, /autofix-plan-json, /autofix-apply [--file path], /doctor, /doctor-json, /snapshot, /save-session [tag], /sessions [--json] [--tag T] [--provider P] [--model M] [--contains TXT] [--from TS] [--to TS] [--last 30m|12h|7d] [--sort saved_at|messages] [--limit N], /load-session latest|<path>, /session-info, /delete-session latest|<path>, /prune-sessions <N>, /session-search <term>, /session-tail latest|<path> [N], /session-stats, /session-report [--json|--md|--csv|--html] [--from TS] [--to TS] [--last 30m|12h|7d] [--top N], /reports [--json] [--latest] [--limit N] [--ext md|csv|html] [--sort modified|name|size], /report-open latest [--ext md|csv|html], /report-delete latest [--ext md|csv|html], /report-prune N [--ext md|csv|html], /session-export-md latest|<path>, /session-diff [--json] A B (A/B: latest|latest-N|latest@tag|path), /run <cmd>");
                return;
            }
            other => {
                eprintln!("Unknown option: {}", other);
                process::exit(1);
            }
        }
    }

    if specialist {
        if let Some(loaded) = load_specialist_system_prompt() {
            system_prompt = loaded;
        }
    }

    if env::var("MATTER_AGENT_MODEL").is_err() && !args.iter().any(|a| a == "--model") {
        model = resolve_agent_model(&provider, &profile).to_string();
    }

    api_key = resolve_agent_api_key(&api_key).unwrap_or_default();
    if api_key.trim().is_empty() {
        eprintln!("agent-chat requires OPENAI_API_KEY or MATTER_AGENT_API_KEY.");
        process::exit(1);
    }
    if provider.eq_ignore_ascii_case("openai") && api_key.starts_with("nvapi-") {
        eprintln!("NVAPI key detected with provider=openai. Use --provider nvidia.");
        process::exit(1);
    }
    if provider.eq_ignore_ascii_case("nvidia") && !api_key.starts_with("nvapi-") {
        eprintln!("Provider nvidia expects an nvapi-* key.");
        process::exit(1);
    }

    println!("Matter Agent Chat");
    println!(
        "provider={} profile={} model={} specialist={}  (/exit to quit, /clear to reset context)",
        provider, profile, model, specialist
    );

    let stdin = io::stdin();
    let mut messages = vec![json!({
        "role": "system",
        "content": system_prompt
    })];
    let mut loaded_session_source = String::new();
    if specialist {
        if let Some(memory_prompt) = build_memory_prompt() {
            messages.push(json!({
                "role": "system",
                "content": memory_prompt
            }));
        }
    }

    loop {
        print!("you> ");
        let _ = io::stdout().flush();

        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() {
            eprintln!("Failed to read input.");
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "/exit" || input == "/quit" {
            break;
        }
        if input == "/clear" {
            let system = messages
                .first()
                .cloned()
                .unwrap_or_else(|| json!({"role":"system","content":"You are Matter Agent."}));
            messages.clear();
            messages.push(system);
            println!("context cleared");
            continue;
        }
        if input == "/context" {
            if let Some(ctx) = load_workspace_context() {
                messages.push(json!({
                    "role": "user",
                    "content": format!("Workspace context snapshot:\n\n{}", ctx)
                }));
                println!("workspace context injected");
            } else {
                println!("workspace context file not found (env/agent_workspace_context.md)");
            }
            continue;
        }
        if input == "/memory" {
            let memory = load_agent_memory();
            if memory.is_empty() {
                println!("memory is empty");
            } else {
                println!("memory:");
                for (idx, note) in memory.iter().enumerate() {
                    println!("  {}. {}", idx + 1, note);
                }
            }
            continue;
        }
        if let Some(note) = input.strip_prefix("/remember ") {
            let note = note.trim();
            if note.is_empty() {
                println!("usage: /remember <text>");
                continue;
            }
            let mut memory = load_agent_memory();
            memory.push(note.to_string());
            match save_agent_memory(&memory) {
                Ok(_) => println!("saved to memory"),
                Err(err) => eprintln!("memory save failed: {}", err),
            }
            continue;
        }
        if input == "/forget" {
            match save_agent_memory(&Vec::new()) {
                Ok(_) => println!("memory cleared"),
                Err(err) => eprintln!("memory clear failed: {}", err),
            }
            continue;
        }
        if input == "/status" {
            match run_local_command_capture("git status --short") {
                Ok(out) => {
                    if out.trim().is_empty() {
                        println!("working tree clean");
                    } else {
                        println!("{}", out.trim_end());
                    }
                }
                Err(err) => eprintln!("status failed: {}", err),
            }
            continue;
        }
        if input == "/check-cli" {
            match run_local_command_capture("cargo check -p matter-cli") {
                Ok(out) => println!("{}", out.trim_end()),
                Err(err) => eprintln!("check failed: {}", err),
            }
            continue;
        }
        if input == "/clippy-cli" {
            match run_local_command_capture(
                "cargo clippy -p matter-cli --all-targets -- -D warnings",
            ) {
                Ok(out) => println!("{}", out.trim_end()),
                Err(err) => eprintln!("clippy failed: {}", err),
            }
            continue;
        }
        if input == "/check-workspace" {
            match run_local_command_capture("cargo check --workspace") {
                Ok(out) => print_summarized_command_output("cargo check --workspace", &out),
                Err(err) => eprintln!("check-workspace failed: {}", err),
            }
            continue;
        }
        if input == "/test-workspace" {
            match run_local_command_capture("cargo test --workspace -q") {
                Ok(out) => print_summarized_command_output("cargo test --workspace -q", &out),
                Err(err) => eprintln!("test-workspace failed: {}", err),
            }
            continue;
        }
        if input == "/fix-plan" {
            let check_out = run_local_command_capture("cargo check --workspace");
            let clippy_out = run_local_command_capture(
                "cargo clippy --workspace --exclude matter-llvm --all-targets -- -D warnings",
            );
            let mut diagnostics = String::new();
            if let Ok(out) = check_out {
                diagnostics.push_str(&out);
                diagnostics.push('\n');
            }
            if let Ok(out) = clippy_out {
                diagnostics.push_str(&out);
            }
            if diagnostics.trim().is_empty() {
                println!("No diagnostics captured.");
            } else {
                print_fix_plan_from_output(&diagnostics);
            }
            continue;
        }
        if input == "/autofix-plan" {
            let check_out = run_local_command_capture("cargo check --workspace");
            let clippy_out = run_local_command_capture(
                "cargo clippy --workspace --exclude matter-llvm --all-targets -- -D warnings",
            );
            let mut diagnostics = String::new();
            if let Ok(out) = check_out {
                diagnostics.push_str(&out);
                diagnostics.push('\n');
            }
            if let Ok(out) = clippy_out {
                diagnostics.push_str(&out);
            }
            if diagnostics.trim().is_empty() {
                println!("No diagnostics captured.");
            } else {
                print_autofix_plan_from_output(&diagnostics);
            }
            continue;
        }
        if input == "/autofix-plan-json" {
            let diagnostics = collect_workspace_diagnostics();
            if diagnostics.trim().is_empty() {
                println!("{{\"ok\":true,\"message\":\"no diagnostics captured\",\"items\":[]}}");
            } else {
                print_autofix_plan_json_from_output(&diagnostics);
            }
            continue;
        }
        if input == "/autofix-apply" {
            let diagnostics = collect_workspace_diagnostics();
            if diagnostics.trim().is_empty() {
                println!("No diagnostics captured.");
            } else {
                print_autofix_apply_from_output(&diagnostics);
            }
            continue;
        }
        if let Some(arg) = input.strip_prefix("/autofix-apply ") {
            let diagnostics = collect_workspace_diagnostics();
            if diagnostics.trim().is_empty() {
                println!("No diagnostics captured.");
                continue;
            }
            if let Some(path) = arg.trim().strip_prefix("--file ") {
                print_autofix_apply_for_file_from_output(&diagnostics, path.trim());
            } else {
                println!("usage: /autofix-apply --file <path>");
            }
            continue;
        }
        if input == "/doctor" {
            run_agent_doctor();
            continue;
        }
        if input == "/doctor-json" {
            println!("{}", run_agent_doctor_json());
            continue;
        }
        if input == "/snapshot" {
            println!(
                "{}",
                build_agent_snapshot_json(&provider, &profile, &model, specialist, &messages)
            );
            continue;
        }
        if input == "/save-session" {
            match save_agent_session_json(&provider, &profile, &model, specialist, &messages, None)
            {
                Ok(path) => println!("session saved: {}", path),
                Err(err) => eprintln!("save-session failed: {}", err),
            }
            continue;
        }
        if let Some(tag) = input.strip_prefix("/save-session ") {
            match save_agent_session_json(
                &provider,
                &profile,
                &model,
                specialist,
                &messages,
                Some(tag.trim()),
            ) {
                Ok(path) => println!("session saved: {}", path),
                Err(err) => eprintln!("save-session failed: {}", err),
            }
            continue;
        }
        if input == "/sessions" || input.starts_with("/sessions ") {
            let args = input.strip_prefix("/sessions").unwrap_or("").trim();
            let Some(opts) = parse_sessions_args(args) else {
                println!("usage: /sessions [--json] [--tag T] [--provider P] [--model M] [--contains TXT] [--from TS] [--to TS] [--last 30m|12h|7d] [--sort saved_at|messages] [--limit N]");
                continue;
            };
            match list_saved_sessions_filtered(&opts) {
                Ok(entries) => {
                    if entries.is_empty() {
                        println!("no saved sessions");
                    } else if opts.json_output {
                        let sessions_json = entries
                            .iter()
                            .map(|entry| {
                                json!({
                                    "path": entry.path,
                                    "provider": entry.provider,
                                    "model": entry.model,
                                    "profile": entry.profile,
                                    "specialist": entry.specialist,
                                    "saved_at": entry.saved_at,
                                    "message_count": entry.message_count,
                                })
                            })
                            .collect::<Vec<_>>();
                        println!(
                            "{}",
                            json!({
                                "ok": true,
                                "count": entries.len(),
                                "filters": {
                                    "tag": opts.tag,
                                    "provider": opts.provider,
                                    "model": opts.model,
                                    "contains": opts.contains,
                                    "from": opts.from_saved_at,
                                    "to": opts.to_saved_at,
                                    "sort": opts.sort,
                                    "limit": opts.limit
                                },
                                "sessions": sessions_json
                            })
                        );
                    } else {
                        for entry in entries {
                            println!("{}", entry.path);
                        }
                    }
                }
                Err(err) => eprintln!("sessions failed: {}", err),
            }
            continue;
        }
        if let Some(arg) = input.strip_prefix("/load-session ") {
            let request = arg.trim();
            let path = if request.eq_ignore_ascii_case("latest") {
                match latest_saved_session_path() {
                    Some(p) => p,
                    None => {
                        println!("no saved sessions to load");
                        continue;
                    }
                }
            } else {
                request.to_string()
            };
            match load_agent_session_json(&path) {
                Ok(loaded) => {
                    messages = loaded;
                    loaded_session_source = path.clone();
                    println!("loaded session: {}", path);
                    println!("messages: {}", messages.len());
                }
                Err(err) => eprintln!("load-session failed: {}", err),
            }
            continue;
        }
        if input == "/session-info" {
            if loaded_session_source.is_empty() {
                println!("session source: (live)");
            } else {
                println!("session source: {}", loaded_session_source);
            }
            println!("messages: {}", messages.len());
            continue;
        }
        if let Some(arg) = input.strip_prefix("/delete-session ") {
            let request = arg.trim();
            let path = if request.eq_ignore_ascii_case("latest") {
                match latest_saved_session_path() {
                    Some(p) => p,
                    None => {
                        println!("no saved sessions to delete");
                        continue;
                    }
                }
            } else {
                request.to_string()
            };
            match delete_saved_session(&path) {
                Ok(_) => println!("deleted: {}", path),
                Err(err) => eprintln!("delete-session failed: {}", err),
            }
            continue;
        }
        if let Some(arg) = input.strip_prefix("/prune-sessions ") {
            let keep = match arg.trim().parse::<usize>() {
                Ok(v) => v,
                Err(_) => {
                    println!("usage: /prune-sessions <N>");
                    continue;
                }
            };
            match prune_saved_sessions(keep) {
                Ok(removed) => println!("pruned {} session(s), kept {}", removed, keep),
                Err(err) => eprintln!("prune-sessions failed: {}", err),
            }
            continue;
        }
        if let Some(term) = input.strip_prefix("/session-search ") {
            let term = term.trim();
            if term.is_empty() {
                println!("usage: /session-search <term>");
                continue;
            }
            match search_saved_sessions(term) {
                Ok(matches) => {
                    if matches.is_empty() {
                        println!("no matches");
                    } else {
                        for line in matches {
                            println!("{}", line);
                        }
                    }
                }
                Err(err) => eprintln!("session-search failed: {}", err),
            }
            continue;
        }
        if let Some(arg) = input.strip_prefix("/session-tail ") {
            let parts = arg.split_whitespace().collect::<Vec<_>>();
            if parts.is_empty() {
                println!("usage: /session-tail latest|<path> [N]");
                continue;
            }
            let source = parts[0];
            let n = if parts.len() >= 2 {
                parts[1].parse::<usize>().unwrap_or(5)
            } else {
                5
            };
            let path = if source.eq_ignore_ascii_case("latest") {
                match latest_saved_session_path() {
                    Some(p) => p,
                    None => {
                        println!("no saved sessions");
                        continue;
                    }
                }
            } else {
                source.to_string()
            };
            match load_agent_session_json(&path) {
                Ok(msgs) => print_session_tail(&path, &msgs, n),
                Err(err) => eprintln!("session-tail failed: {}", err),
            }
            continue;
        }
        if input == "/session-stats" {
            match session_stats() {
                Ok(stats) => {
                    println!("{}", stats);
                }
                Err(err) => eprintln!("session-stats failed: {}", err),
            }
            continue;
        }
        if input == "/session-report" || input.starts_with("/session-report ") {
            let args = input.strip_prefix("/session-report").unwrap_or("").trim();
            let Some(opts) = parse_session_report_args(args) else {
                println!("usage: /session-report [--json|--md|--csv|--html] [--from TS] [--to TS] [--last 30m|12h|7d] [--top N]");
                continue;
            };
            if opts.html_output {
                match session_report_html(
                    opts.from_saved_at.as_deref(),
                    opts.to_saved_at.as_deref(),
                    opts.top_n,
                ) {
                    Ok(path) => println!("session report html: {}", path),
                    Err(err) => eprintln!("session-report failed: {}", err),
                }
            } else if opts.csv_output {
                match session_report_csv(
                    opts.from_saved_at.as_deref(),
                    opts.to_saved_at.as_deref(),
                    opts.top_n,
                ) {
                    Ok(path) => println!("session report csv: {}", path),
                    Err(err) => eprintln!("session-report failed: {}", err),
                }
            } else if opts.markdown_output {
                match session_report_markdown(
                    opts.from_saved_at.as_deref(),
                    opts.to_saved_at.as_deref(),
                    opts.top_n,
                ) {
                    Ok(path) => println!("session report markdown: {}", path),
                    Err(err) => eprintln!("session-report failed: {}", err),
                }
            } else {
                match session_report(
                    opts.json_output,
                    opts.from_saved_at.as_deref(),
                    opts.to_saved_at.as_deref(),
                    opts.top_n,
                ) {
                    Ok(report) => println!("{}", report),
                    Err(err) => eprintln!("session-report failed: {}", err),
                }
            }
            continue;
        }
        if input == "/reports" || input.starts_with("/reports ") {
            let args = input.strip_prefix("/reports").unwrap_or("").trim();
            let Some(opts) = parse_reports_args(args) else {
                println!("usage: /reports [--json] [--latest] [--limit N] [--ext md|csv|html] [--sort modified|name|size]");
                continue;
            };
            match list_generated_reports(&opts) {
                Ok(paths) => {
                    if paths.is_empty() {
                        println!("no reports");
                    } else if opts.json_output {
                        let items = paths
                            .iter()
                            .map(|path| {
                                let (size_bytes, modified_ms) = report_file_metadata(path);
                                let ext = Path::new(path)
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("")
                                    .to_ascii_lowercase();
                                let timestamp = extract_report_timestamp(path);
                                json!({
                                    "path": path,
                                    "ext": ext,
                                    "timestamp": timestamp,
                                    "size_bytes": size_bytes,
                                    "modified_ms": modified_ms
                                })
                            })
                            .collect::<Vec<_>>();
                        println!(
                            "{}",
                            json!({
                                "ok": true,
                                "count": items.len(),
                                "filters": {
                                    "json": true,
                                    "latest": opts.latest_only,
                                    "limit": opts.limit,
                                    "ext": opts.extension,
                                    "sort": opts.sort
                                },
                                "reports": items
                            })
                        );
                    } else {
                        for path in paths {
                            println!("{}", path);
                        }
                    }
                }
                Err(err) => eprintln!("reports failed: {}", err),
            }
            continue;
        }
        if input == "/report-open latest" || input.starts_with("/report-open latest ") {
            let args = input
                .strip_prefix("/report-open latest")
                .unwrap_or("")
                .trim();
            let Some(ext) = parse_report_open_ext(args) else {
                println!("usage: /report-open latest [--ext md|csv|html]");
                continue;
            };
            let mut opts = ReportsListArgs {
                latest_only: true,
                ..ReportsListArgs::default()
            };
            opts.extension = ext;
            match list_generated_reports(&opts) {
                Ok(paths) => {
                    if let Some(path) = paths.first() {
                        println!("{}", path);
                    } else {
                        println!("no reports");
                    }
                }
                Err(err) => eprintln!("report-open failed: {}", err),
            }
            continue;
        }
        if input == "/report-delete latest" || input.starts_with("/report-delete latest ") {
            let args = input
                .strip_prefix("/report-delete latest")
                .unwrap_or("")
                .trim();
            let Some(ext) = parse_report_open_ext(args) else {
                println!("usage: /report-delete latest [--ext md|csv|html]");
                continue;
            };
            let mut opts = ReportsListArgs {
                latest_only: true,
                ..ReportsListArgs::default()
            };
            opts.extension = ext;
            match list_generated_reports(&opts) {
                Ok(paths) => {
                    if let Some(path) = paths.first() {
                        match delete_report_file(path) {
                            Ok(()) => println!("deleted report: {}", path),
                            Err(err) => eprintln!("report-delete failed: {}", err),
                        }
                    } else {
                        println!("no reports");
                    }
                }
                Err(err) => eprintln!("report-delete failed: {}", err),
            }
            continue;
        }
        if let Some(arg) = input.strip_prefix("/report-prune ") {
            let parts = arg.split_whitespace().collect::<Vec<_>>();
            if parts.is_empty() {
                println!("usage: /report-prune <N> [--ext md|csv|html]");
                continue;
            }
            let keep = match parts[0].parse::<usize>() {
                Ok(v) => v,
                Err(_) => {
                    println!("usage: /report-prune <N> [--ext md|csv|html]");
                    continue;
                }
            };
            let ext = if parts.len() == 3 && parts[1] == "--ext" {
                let e = parts[2].to_ascii_lowercase();
                if e == "md" || e == "csv" || e == "html" {
                    Some(e)
                } else {
                    println!("usage: /report-prune <N> [--ext md|csv|html]");
                    continue;
                }
            } else if parts.len() == 1 {
                None
            } else {
                println!("usage: /report-prune <N> [--ext md|csv|html]");
                continue;
            };
            match prune_generated_reports(keep, ext.as_deref()) {
                Ok(removed) => println!("pruned {} report(s), kept {}", removed, keep),
                Err(err) => eprintln!("report-prune failed: {}", err),
            }
            continue;
        }
        if let Some(arg) = input.strip_prefix("/session-export-md ") {
            let source = arg.trim();
            if source.is_empty() {
                println!("usage: /session-export-md latest|<path>");
                continue;
            }
            let path = if source.eq_ignore_ascii_case("latest") {
                match latest_saved_session_path() {
                    Some(p) => p,
                    None => {
                        println!("no saved sessions");
                        continue;
                    }
                }
            } else {
                source.to_string()
            };
            match export_session_markdown(&path) {
                Ok(out) => println!("session markdown: {}", out),
                Err(err) => eprintln!("session-export-md failed: {}", err),
            }
            continue;
        }
        if let Some(arg) = input.strip_prefix("/session-diff ") {
            let Some(parsed) = parse_session_diff_args(arg) else {
                println!("usage: /session-diff [--json] <A> <B> (A/B: latest | latest-N | latest@tag | path)");
                continue;
            };
            let left = resolve_session_ref(&parsed.left_ref);
            let right = resolve_session_ref(&parsed.right_ref);
            match (left, right) {
                (Some(a), Some(b)) => match session_diff(&a, &b, parsed.json_output) {
                    Ok(diff) => println!("{}", diff),
                    Err(err) => eprintln!("session-diff failed: {}", err),
                },
                _ => println!("session-diff: could not resolve one or both session refs"),
            }
            continue;
        }
        if let Some(cmd) = input.strip_prefix("/run ") {
            match run_whitelisted_command(cmd.trim()) {
                Ok(out) => print_summarized_command_output(cmd.trim(), &out),
                Err(err) => eprintln!("run failed: {}", err),
            }
            continue;
        }

        messages.push(json!({
            "role": "user",
            "content": input
        }));

        match chat_completion_via_curl(&provider, &api_key, &model, &messages) {
            Ok(answer) => {
                println!("agent> {}", answer);
                messages.push(json!({
                    "role": "assistant",
                    "content": answer
                }));
            }
            Err(err) => {
                eprintln!("agent error: {}", err);
                let _ = messages.pop();
            }
        }
    }
}

fn resolve_agent_model(provider: &str, profile: &str) -> &'static str {
    match provider.to_ascii_lowercase().as_str() {
        "nvidia" => match profile.to_ascii_lowercase().as_str() {
            "coding" => "moonshotai/kimi-k2.5",
            "balanced" => "meta/llama-3.3-70b-instruct",
            "cheap" => "nvidia/llama-3.1-nemotron-nano-8b-v1",
            "max" => "deepseek-ai/deepseek-r1",
            _ => "meta/llama-3.3-70b-instruct",
        },
        _ => match profile.to_ascii_lowercase().as_str() {
            "coding" => "gpt-5.2-codex",
            "balanced" => "gpt-5.2",
            "cheap" => "gpt-5-mini",
            "max" => "gpt-5.2-pro",
            _ => "gpt-5.2-codex",
        },
    }
}

fn print_agent_model_catalog() {
    println!("Matter Agent model profiles:");
    println!("  coding   -> gpt-5.2-codex");
    println!("  balanced -> gpt-5.2");
    println!("  cheap    -> gpt-5-mini");
    println!("  max      -> gpt-5.2-pro");
    println!();
    println!("Other suggested models:");
    println!("  gpt-5.2-chat-latest");
    println!("  gpt-5-nano");
    println!();
    println!("NVIDIA suggested models (provider=nvidia):");
    println!("  moonshotai/kimi-k2.5");
    println!("  meta/llama-3.3-70b-instruct");
    println!("  nvidia/llama-3.1-nemotron-nano-8b-v1");
    println!("  deepseek-ai/deepseek-r1");
}

fn load_specialist_system_prompt() -> Option<String> {
    let path = Path::new("env").join("agent_system_prompt.txt");
    let prompt = fs::read_to_string(path).ok()?;
    if let Some(ctx) = load_workspace_context() {
        Some(format!(
            "{}\n\nCurrent workspace context:\n\n{}",
            prompt, ctx
        ))
    } else {
        Some(prompt)
    }
}

fn load_workspace_context() -> Option<String> {
    let path = Path::new("env").join("agent_workspace_context.md");
    fs::read_to_string(path).ok()
}

fn agent_memory_path() -> PathBuf {
    Path::new("env").join("agent_memory.json")
}

fn load_agent_memory() -> Vec<String> {
    let path = agent_memory_path();
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let doc: JsonValue = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let notes = match doc.get("notes").and_then(|v| v.as_array()) {
        Some(v) => v,
        None => return Vec::new(),
    };
    notes
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect()
}

fn save_agent_memory(notes: &[String]) -> Result<(), String> {
    let path = agent_memory_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create env dir failed: {}", e))?;
    }
    let payload = json!({ "notes": notes });
    let raw = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("serialize memory failed: {}", e))?;
    fs::write(path, raw).map_err(|e| format!("write memory failed: {}", e))
}

fn build_memory_prompt() -> Option<String> {
    let memory = load_agent_memory();
    if memory.is_empty() {
        return None;
    }
    let mut out = String::from("Long-term memory notes (persisted):\n");
    for (idx, note) in memory.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", idx + 1, note));
    }
    Some(out)
}

fn run_local_command_capture(command: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", command])
        .output()
        .map_err(|e| format!("failed to execute command: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.trim().is_empty() {
            Err(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(stderr)
        }
    }
}

fn print_summarized_command_output(command: &str, output: &str) {
    let lines: Vec<&str> = output.lines().collect();
    println!("{} => {} lines", command, lines.len());
    if lines.is_empty() {
        println!("(no output)");
        return;
    }
    let head = lines.iter().take(12).copied().collect::<Vec<_>>();
    for line in head {
        println!("{}", line);
    }
    if lines.len() > 24 {
        println!("... (omitted {} lines) ...", lines.len() - 24);
    }
    let tail = lines
        .iter()
        .skip(lines.len().saturating_sub(12))
        .copied()
        .collect::<Vec<_>>();
    if lines.len() > 12 {
        for line in tail {
            println!("{}", line);
        }
    }
}

fn run_whitelisted_command(command: &str) -> Result<String, String> {
    let allowed = [
        "git status --short",
        "cargo check -p matter-cli",
        "cargo check --workspace",
        "cargo test -p matter-cli -q",
        "cargo test --workspace -q",
        "cargo clippy -p matter-cli --all-targets -- -D warnings",
        "cargo clippy --workspace --exclude matter-llvm --all-targets -- -D warnings",
    ];
    if !allowed.contains(&command) {
        return Err(format!(
            "command not allowed: '{}'. Use one of: {}",
            command,
            allowed.join(" | ")
        ));
    }
    run_local_command_capture(command)
}

fn print_fix_plan_from_output(output: &str) {
    let mut findings: Vec<String> = Vec::new();
    let mut by_file: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_error = String::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("error:") || trimmed.starts_with("warning:") {
            findings.push(trimmed.to_string());
            current_error = trimmed.to_string();
        } else if let Some(pointer) = trimmed.strip_prefix("--> ") {
            let file = pointer
                .split(':')
                .next()
                .unwrap_or(pointer)
                .trim()
                .to_string();
            if !file.is_empty() && !current_error.is_empty() {
                by_file.entry(file).or_default().push(current_error.clone());
            }
        }
    }
    findings.sort();
    findings.dedup();
    for values in by_file.values_mut() {
        values.sort();
        values.dedup();
    }

    println!("Fix plan:");
    if findings.is_empty() {
        println!("1. No explicit error/warning lines parsed.");
        println!("2. Re-run /check-workspace and /clippy-cli and inspect full logs.");
        return;
    }
    println!("Global findings:");
    for (idx, finding) in findings.iter().take(20).enumerate() {
        println!("{}. {}", idx + 1, finding);
    }
    if findings.len() > 20 {
        println!("... +{} additional findings", findings.len() - 20);
    }
    if !by_file.is_empty() {
        println!("Patch hints by file:");
        for (file, issues) in by_file.iter().take(20) {
            println!("- {}", file);
            for issue in issues.iter().take(3) {
                println!("  - {}", issue);
            }
            if issues.len() > 3 {
                println!("  - ... +{} more", issues.len() - 3);
            }
        }
    }
    println!("Next step: resolve findings top-down and re-run /clippy-cli.");
}

fn print_autofix_plan_from_output(output: &str) {
    let by_file = parse_diagnostics_by_file(output);

    println!("Autofix plan:");
    if by_file.is_empty() {
        println!("No file-linked diagnostics found.");
        println!("Run /check-workspace and /clippy-cli, then retry.");
        return;
    }

    let mut ranked: Vec<(String, Vec<String>)> = by_file.into_iter().collect();
    ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));

    for (idx, (file, issues)) in ranked.iter().take(20).enumerate() {
        println!("{}. {}", idx + 1, file);
        println!("   priority: {} issue(s)", issues.len());
        for issue in issues.iter().take(4) {
            println!("   - {}", issue);
            if let Some(hint) = autofix_hint_for_issue(issue) {
                println!("     patch hint: {}", hint);
            }
        }
        if issues.len() > 4 {
            println!("   - ... +{} more", issues.len() - 4);
        }
    }
    println!("Apply fixes by priority order and re-run /clippy-cli.");
}

fn print_autofix_apply_from_output(output: &str) {
    let by_file = parse_diagnostics_by_file(output);
    if by_file.is_empty() {
        println!("No file-linked diagnostics found.");
        return;
    }
    let mut ranked: Vec<(String, Vec<String>)> = by_file.into_iter().collect();
    ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    let (file, issues) = &ranked[0];

    println!("Autofix apply (assisted, no auto-write):");
    println!("target file: {}", file);
    println!("priority: {} issue(s)", issues.len());
    for (idx, issue) in issues.iter().take(5).enumerate() {
        println!("{}. {}", idx + 1, issue);
        if let Some(hint) = autofix_hint_for_issue(issue) {
            println!("   patch hint: {}", hint);
        }
    }
    println!("Suggested next commands:");
    println!("1. /run cargo check --workspace");
    println!("2. /run cargo clippy --workspace --exclude matter-llvm --all-targets -- -D warnings");
    println!(
        "3. Apply patch in {} and re-run /clippy-cli (or /check-workspace)",
        file
    );
}

fn print_autofix_apply_for_file_from_output(output: &str, target_file: &str) {
    let by_file = parse_diagnostics_by_file(output);
    let matched = by_file
        .iter()
        .find(|(file, _)| file.contains(target_file))
        .map(|(file, issues)| (file.clone(), issues.clone()));

    let (file, issues) = match matched {
        Some(v) => v,
        None => {
            println!("No diagnostics found for file filter: {}", target_file);
            return;
        }
    };

    println!("Autofix apply (file-filtered, assisted):");
    println!("target file: {}", file);
    println!("priority: {} issue(s)", issues.len());
    for (idx, issue) in issues.iter().take(8).enumerate() {
        println!("{}. {}", idx + 1, issue);
        if let Some(hint) = autofix_hint_for_issue(issue) {
            println!("   patch hint: {}", hint);
        }
    }
}

fn parse_diagnostics_by_file(output: &str) -> BTreeMap<String, Vec<String>> {
    let mut current_error = String::new();
    let mut by_file: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("error:") || trimmed.starts_with("warning:") {
            current_error = trimmed.to_string();
            continue;
        }
        if let Some(pointer) = trimmed.strip_prefix("--> ") {
            let file = pointer
                .split(':')
                .next()
                .unwrap_or(pointer)
                .trim()
                .to_string();
            if !file.is_empty() && !current_error.is_empty() {
                by_file.entry(file).or_default().push(current_error.clone());
            }
        }
    }
    for issues in by_file.values_mut() {
        issues.sort();
        issues.dedup();
    }
    by_file
}

fn collect_workspace_diagnostics() -> String {
    let check_out = run_local_command_capture("cargo check --workspace");
    let clippy_out = run_local_command_capture(
        "cargo clippy --workspace --exclude matter-llvm --all-targets -- -D warnings",
    );
    let mut diagnostics = String::new();
    if let Ok(out) = check_out {
        diagnostics.push_str(&out);
        diagnostics.push('\n');
    }
    if let Ok(out) = clippy_out {
        diagnostics.push_str(&out);
    }
    diagnostics
}

fn print_autofix_plan_json_from_output(output: &str) {
    let by_file = parse_diagnostics_by_file(output);
    let mut ranked: Vec<(String, Vec<String>)> = by_file.into_iter().collect();
    ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));

    let items: Vec<JsonValue> = ranked
        .into_iter()
        .map(|(file, issues)| {
            let issue_items: Vec<JsonValue> = issues
                .into_iter()
                .map(|issue| {
                    let hint = autofix_hint_for_issue(&issue).unwrap_or("");
                    json!({
                        "issue": issue,
                        "hint": hint
                    })
                })
                .collect();
            json!({
                "file": file,
                "priority": issue_items.len(),
                "issues": issue_items
            })
        })
        .collect();

    let payload = json!({
        "ok": true,
        "kind": "autofix-plan",
        "items": items
    });
    println!("{}", payload);
}

fn autofix_hint_for_issue(issue: &str) -> Option<&'static str> {
    let lower = issue.to_ascii_lowercase();
    if lower.contains("new_without_default") {
        return Some("add `impl Default for Type { fn default() -> Self { Self::new() } }`");
    }
    if lower.contains("manual_is_multiple_of") {
        return Some("replace `% N == 0` with `.is_multiple_of(N)`");
    }
    if lower.contains("redundant closure") {
        return Some("replace closure with direct function reference");
    }
    if lower.contains("assertions-on-constants") {
        return Some("replace `assert!(false, ...)` with `panic!(...)`");
    }
    if lower.contains("needless_borrows_for_generic_args") {
        return Some("remove unnecessary `&` in args arrays");
    }
    if lower.contains("manual_range_contains") {
        return Some("use `(a..=b).contains(&x)`");
    }
    if lower.contains("unused-enumerate-index") {
        return Some("remove `.enumerate()` when index is not used");
    }
    if lower.contains("result-large-err") {
        return Some("consider boxed error type or local allow with justification");
    }
    if lower.contains("approx_constant") {
        return Some("use std::f64::consts::PI (or exact constant)");
    }
    None
}

fn run_agent_doctor() {
    println!("Doctor report:");
    let checks = vec![
        (
            "git status --short",
            run_local_command_capture("git status --short").map(|_| String::new()),
        ),
        (
            "cargo check -p matter-cli",
            run_local_command_capture("cargo check -p matter-cli"),
        ),
        (
            "cargo clippy -p matter-cli --all-targets -- -D warnings",
            run_local_command_capture("cargo clippy -p matter-cli --all-targets -- -D warnings"),
        ),
        (
            "cargo test -p matter-cli -q",
            run_local_command_capture("cargo test -p matter-cli -q"),
        ),
    ];

    let mut ok_count = 0usize;
    for (name, result) in checks {
        match result {
            Ok(output) => {
                ok_count += 1;
                println!("[OK] {}", name);
                if !output.trim().is_empty() {
                    print_summarized_command_output(name, &output);
                }
            }
            Err(err) => {
                println!("[FAIL] {}", name);
                println!("{}", err.trim());
            }
        }
    }
    println!("Doctor summary: {}/4 checks passed", ok_count);
}

fn run_agent_doctor_json() -> JsonValue {
    let checks = vec![
        (
            "git status --short",
            run_local_command_capture("git status --short"),
        ),
        (
            "cargo check -p matter-cli",
            run_local_command_capture("cargo check -p matter-cli"),
        ),
        (
            "cargo clippy -p matter-cli --all-targets -- -D warnings",
            run_local_command_capture("cargo clippy -p matter-cli --all-targets -- -D warnings"),
        ),
        (
            "cargo test -p matter-cli -q",
            run_local_command_capture("cargo test -p matter-cli -q"),
        ),
    ];
    let mut items = Vec::new();
    let mut passed = 0usize;
    for (name, result) in checks {
        match result {
            Ok(output) => {
                passed += 1;
                items.push(json!({
                    "command": name,
                    "ok": true,
                    "lines": output.lines().count()
                }));
            }
            Err(err) => {
                items.push(json!({
                    "command": name,
                    "ok": false,
                    "error": err
                }));
            }
        }
    }
    json!({
        "ok": passed == 4,
        "passed": passed,
        "total": 4,
        "checks": items
    })
}

fn build_agent_snapshot_json(
    provider: &str,
    profile: &str,
    model: &str,
    specialist: bool,
    messages: &[JsonValue],
) -> JsonValue {
    let memory = load_agent_memory();
    json!({
        "ok": true,
        "provider": provider,
        "profile": profile,
        "model": model,
        "specialist": specialist,
        "message_count": messages.len(),
        "memory_notes": memory.len(),
        "has_workspace_context": load_workspace_context().is_some()
    })
}

fn save_agent_session_json(
    provider: &str,
    profile: &str,
    model: &str,
    specialist: bool,
    messages: &[JsonValue],
    tag: Option<&str>,
) -> Result<String, String> {
    let dir = Path::new("env").join("sessions");
    fs::create_dir_all(&dir).map_err(|e| format!("create sessions dir failed: {}", e))?;
    let ts = chrono_like_timestamp();
    let cleaned_tag = tag
        .map(sanitize_session_tag)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "chat".to_string());
    let base = format!("session_{}_{}_{}", provider, cleaned_tag, ts);
    let mut file = format!("{}.json", base);
    let mut path = dir.join(&file);
    let mut suffix = 1usize;
    while path.exists() {
        file = format!("{}_{}.json", base, suffix);
        path = dir.join(&file);
        suffix += 1;
    }
    let payload = json!({
        "provider": provider,
        "profile": profile,
        "model": model,
        "specialist": specialist,
        "saved_at": ts,
        "message_count": messages.len(),
        "messages": messages
    });
    let raw = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("serialize session failed: {}", e))?;
    fs::write(&path, raw).map_err(|e| format!("write session failed: {}", e))?;
    Ok(path.display().to_string())
}

fn list_saved_sessions() -> Result<Vec<String>, String> {
    let dir = Path::new("env").join("sessions");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("read sessions dir failed: {}", e))? {
        let entry = entry.map_err(|e| format!("read dir entry failed: {}", e))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(path.display().to_string());
        }
    }
    out.sort();
    Ok(out)
}

fn latest_saved_session_path() -> Option<String> {
    let mut paths = list_saved_sessions().ok()?;
    paths.sort();
    paths.pop()
}

fn resolve_session_ref(session_ref: &str) -> Option<String> {
    if session_ref.eq_ignore_ascii_case("latest") {
        return latest_saved_session_path();
    }
    if let Some(tag_raw) = session_ref.strip_prefix("latest@") {
        let tag = sanitize_session_tag(tag_raw.trim());
        if tag.is_empty() {
            return None;
        }
        let mut sessions = list_saved_sessions().ok()?;
        sessions.sort();
        let marker = format!("_{}_", tag);
        return sessions
            .into_iter()
            .rev()
            .find(|path| path.replace('\\', "/").contains(&marker));
    }
    if let Some(offset_raw) = session_ref.strip_prefix("latest-") {
        let offset = offset_raw.parse::<usize>().ok()?;
        let mut sessions = list_saved_sessions().ok()?;
        sessions.sort();
        if sessions.is_empty() {
            return None;
        }
        let idx_from_end = offset + 1;
        if idx_from_end > sessions.len() {
            return None;
        }
        return sessions
            .get(sessions.len().saturating_sub(idx_from_end))
            .cloned();
    }
    let p = Path::new(session_ref);
    if p.exists() {
        return Some(session_ref.to_string());
    }
    None
}

struct SessionDiffArgs {
    left_ref: String,
    right_ref: String,
    json_output: bool,
}

#[derive(Default)]
struct SessionReportArgs {
    json_output: bool,
    markdown_output: bool,
    csv_output: bool,
    html_output: bool,
    from_saved_at: Option<String>,
    to_saved_at: Option<String>,
    top_n: usize,
}

#[derive(Default)]
struct SessionsListOptions {
    json_output: bool,
    tag: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    contains: Option<String>,
    from_saved_at: Option<String>,
    to_saved_at: Option<String>,
    sort: String,
    limit: Option<usize>,
}

fn parse_session_report_args(raw: &str) -> Option<SessionReportArgs> {
    if raw.is_empty() {
        return Some(SessionReportArgs::default());
    }
    let parts = raw.split_whitespace().collect::<Vec<_>>();
    let mut opts = SessionReportArgs {
        top_n: 5,
        ..SessionReportArgs::default()
    };
    let mut i = 0usize;
    while i < parts.len() {
        match parts[i] {
            "--json" => {
                opts.json_output = true;
                i += 1;
            }
            "--md" => {
                opts.markdown_output = true;
                i += 1;
            }
            "--csv" => {
                opts.csv_output = true;
                i += 1;
            }
            "--html" => {
                opts.html_output = true;
                i += 1;
            }
            "--from" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                opts.from_saved_at = Some(parts[i + 1].to_string());
                i += 2;
            }
            "--to" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                opts.to_saved_at = Some(parts[i + 1].to_string());
                i += 2;
            }
            "--top" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                opts.top_n = parts[i + 1].parse::<usize>().ok()?;
                if opts.top_n == 0 {
                    return None;
                }
                i += 2;
            }
            "--last" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                let from = resolve_last_window(parts[i + 1])?;
                opts.from_saved_at = Some(from);
                if opts.to_saved_at.is_none() {
                    opts.to_saved_at = Some(current_timestamp_ms().to_string());
                }
                i += 2;
            }
            _ => return None,
        }
    }
    Some(opts)
}

struct SessionListEntry {
    path: String,
    provider: String,
    model: String,
    profile: String,
    specialist: bool,
    saved_at: String,
    message_count: usize,
}

#[derive(Default)]
struct ReportsListArgs {
    json_output: bool,
    latest_only: bool,
    limit: Option<usize>,
    extension: Option<String>,
    sort: String,
}

fn parse_sessions_args(raw: &str) -> Option<SessionsListOptions> {
    if raw.is_empty() {
        return Some(SessionsListOptions::default());
    }
    let parts = raw.split_whitespace().collect::<Vec<_>>();
    let mut opts = SessionsListOptions {
        sort: "saved_at".to_string(),
        ..SessionsListOptions::default()
    };
    let mut i = 0usize;
    while i < parts.len() {
        match parts[i] {
            "--json" => i += 1,
            "--tag" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                opts.tag = Some(sanitize_session_tag(parts[i + 1]));
                i += 2;
            }
            "--provider" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                opts.provider = Some(parts[i + 1].to_string());
                i += 2;
            }
            "--model" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                opts.model = Some(parts[i + 1].to_string());
                i += 2;
            }
            "--contains" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                opts.contains = Some(parts[i + 1].to_string());
                i += 2;
            }
            "--from" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                opts.from_saved_at = Some(parts[i + 1].to_string());
                i += 2;
            }
            "--to" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                opts.to_saved_at = Some(parts[i + 1].to_string());
                i += 2;
            }
            "--last" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                let from = resolve_last_window(parts[i + 1])?;
                opts.from_saved_at = Some(from);
                if opts.to_saved_at.is_none() {
                    opts.to_saved_at = Some(current_timestamp_ms().to_string());
                }
                i += 2;
            }
            "--sort" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                let candidate = parts[i + 1];
                if candidate != "saved_at" && candidate != "messages" {
                    return None;
                }
                opts.sort = candidate.to_string();
                i += 2;
            }
            "--limit" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                let limit = parts[i + 1].parse::<usize>().ok()?;
                opts.limit = Some(limit);
                i += 2;
            }
            _ => return None,
        }
    }
    opts.json_output = raw.split_whitespace().any(|p| p == "--json");
    Some(opts)
}

fn resolve_last_window(raw: &str) -> Option<String> {
    if raw.len() < 2 {
        return None;
    }
    let unit = raw.chars().last()?;
    let value = raw[..raw.len() - 1].parse::<u64>().ok()?;
    let mult = match unit {
        'm' | 'M' => 60_000u64,
        'h' | 'H' => 3_600_000u64,
        'd' | 'D' => 86_400_000u64,
        _ => return None,
    };
    let delta = value.checked_mul(mult)?;
    let now = current_timestamp_ms();
    Some(now.saturating_sub(delta).to_string())
}

fn parse_reports_args(raw: &str) -> Option<ReportsListArgs> {
    if raw.is_empty() {
        return Some(ReportsListArgs::default());
    }
    let parts = raw.split_whitespace().collect::<Vec<_>>();
    let mut opts = ReportsListArgs {
        sort: "modified".to_string(),
        ..ReportsListArgs::default()
    };
    let mut i = 0usize;
    while i < parts.len() {
        match parts[i] {
            "--json" => {
                opts.json_output = true;
                i += 1;
            }
            "--latest" => {
                opts.latest_only = true;
                i += 1;
            }
            "--limit" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                let n = parts[i + 1].parse::<usize>().ok()?;
                opts.limit = Some(n);
                i += 2;
            }
            "--ext" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                let ext = parts[i + 1].to_ascii_lowercase();
                if ext != "md" && ext != "csv" && ext != "html" {
                    return None;
                }
                opts.extension = Some(ext);
                i += 2;
            }
            "--sort" => {
                if i + 1 >= parts.len() {
                    return None;
                }
                let sort = parts[i + 1].to_ascii_lowercase();
                if sort != "modified" && sort != "name" && sort != "size" {
                    return None;
                }
                opts.sort = sort;
                i += 2;
            }
            _ => return None,
        }
    }
    Some(opts)
}

fn parse_report_open_ext(raw: &str) -> Option<Option<String>> {
    if raw.is_empty() {
        return Some(None);
    }
    let parts = raw.split_whitespace().collect::<Vec<_>>();
    if parts.len() == 2 && parts[0] == "--ext" {
        let ext = parts[1].to_ascii_lowercase();
        if ext == "md" || ext == "csv" || ext == "html" {
            return Some(Some(ext));
        }
    }
    None
}

fn extract_report_timestamp(path: &str) -> Option<String> {
    let stem = Path::new(path).file_stem()?.to_str()?;
    let ts = stem.strip_prefix("session_report_")?;
    if ts.chars().all(|c| c.is_ascii_digit()) {
        Some(ts.to_string())
    } else {
        None
    }
}

fn report_file_metadata(path: &str) -> (Option<u64>, Option<u128>) {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return (None, None),
    };
    let size = Some(meta.len());
    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis());
    (size, modified_ms)
}

fn list_generated_reports(opts: &ReportsListArgs) -> Result<Vec<String>, String> {
    let dir = Path::new("env").join("reports");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("read reports dir failed: {}", e))? {
        let entry = entry.map_err(|e| format!("read reports dir entry failed: {}", e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "md" && ext != "csv" && ext != "html" {
            continue;
        }
        if let Some(filter_ext) = &opts.extension {
            if &ext != filter_ext {
                continue;
            }
        }
        out.push(path.display().to_string());
    }
    match opts.sort.as_str() {
        "name" => {
            out.sort();
        }
        "size" => {
            out.sort_by_key(|p| report_file_metadata(p).0.unwrap_or(0));
            out.reverse();
        }
        _ => {
            out.sort_by_key(|p| report_file_metadata(p).1.unwrap_or(0));
            out.reverse();
        }
    }
    if opts.latest_only {
        return Ok(out.into_iter().take(1).collect());
    }
    if let Some(limit) = opts.limit {
        out.truncate(limit);
    }
    Ok(out)
}

fn delete_report_file(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err("report file does not exist".to_string());
    }
    fs::remove_file(p).map_err(|e| format!("delete report failed: {}", e))
}

fn prune_generated_reports(keep: usize, extension: Option<&str>) -> Result<usize, String> {
    let opts = ReportsListArgs {
        extension: extension.map(|s| s.to_string()),
        ..ReportsListArgs::default()
    };
    let reports = list_generated_reports(&opts)?;
    if reports.len() <= keep {
        return Ok(0);
    }
    let mut removed = 0usize;
    for path in reports.into_iter().skip(keep) {
        if delete_report_file(&path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

fn list_saved_sessions_filtered(
    opts: &SessionsListOptions,
) -> Result<Vec<SessionListEntry>, String> {
    let mut paths = list_saved_sessions()?;
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let raw = match fs::read_to_string(&path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let doc: JsonValue = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let provider = doc
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let model = doc
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let profile = doc
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let specialist = doc
            .get("specialist")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let saved_at = doc
            .get("saved_at")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        if let Some(from) = &opts.from_saved_at {
            if saved_at < *from {
                continue;
            }
        }
        if let Some(to) = &opts.to_saved_at {
            if saved_at > *to {
                continue;
            }
        }
        let message_count = doc
            .get("message_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        if let Some(needle) = &opts.contains {
            let messages = doc.get("messages").and_then(|v| v.as_array());
            let needle_lower = needle.to_ascii_lowercase();
            let mut hit = false;
            if let Some(messages) = messages {
                for msg in messages {
                    let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    if content.to_ascii_lowercase().contains(&needle_lower) {
                        hit = true;
                        break;
                    }
                }
            }
            if !hit {
                continue;
            }
        }

        if let Some(tag) = &opts.tag {
            let marker = format!("_{}_", tag);
            if !path.replace('\\', "/").contains(&marker) {
                continue;
            }
        }
        if let Some(filter_provider) = &opts.provider {
            if !provider.eq_ignore_ascii_case(filter_provider) {
                continue;
            }
        }
        if let Some(filter_model) = &opts.model {
            if model != *filter_model {
                continue;
            }
        }
        out.push(SessionListEntry {
            path,
            provider,
            model,
            profile,
            specialist,
            saved_at,
            message_count,
        });
    }
    if opts.sort == "messages" {
        out.sort_by_key(|v| (v.message_count, v.saved_at.clone()));
    } else {
        out.sort_by_key(|v| v.saved_at.clone());
    }
    out.reverse();
    if let Some(limit) = opts.limit {
        out.truncate(limit);
    }
    Ok(out)
}

fn parse_session_diff_args(input: &str) -> Option<SessionDiffArgs> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in input.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if in_quotes {
        return None;
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    let mut json_output = false;
    let refs = if tokens.first().map(|v| v.as_str()) == Some("--json") {
        json_output = true;
        &tokens[1..]
    } else {
        &tokens[..]
    };
    if refs.len() != 2 {
        return None;
    }
    Some(SessionDiffArgs {
        left_ref: refs[0].clone(),
        right_ref: refs[1].clone(),
        json_output,
    })
}

fn delete_saved_session(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err("session file does not exist".to_string());
    }
    fs::remove_file(p).map_err(|e| format!("remove session failed: {}", e))
}

fn prune_saved_sessions(keep: usize) -> Result<usize, String> {
    let mut sessions = list_saved_sessions()?;
    sessions.sort();
    if sessions.len() <= keep {
        return Ok(0);
    }
    let remove_count = sessions.len() - keep;
    let to_remove = sessions.into_iter().take(remove_count).collect::<Vec<_>>();
    let mut removed = 0usize;
    for path in to_remove {
        if delete_saved_session(&path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

fn load_agent_session_json(path: &str) -> Result<Vec<JsonValue>, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read session failed: {}", e))?;
    let doc: JsonValue =
        serde_json::from_str(&raw).map_err(|e| format!("parse session failed: {}", e))?;
    let arr = doc
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "session missing messages array".to_string())?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(item.clone());
    }
    if out.is_empty() {
        return Err("session has no messages".to_string());
    }
    Ok(out)
}

fn search_saved_sessions(term: &str) -> Result<Vec<String>, String> {
    let sessions = list_saved_sessions()?;
    let needle = term.to_ascii_lowercase();
    let mut out = Vec::new();
    for path in sessions {
        let raw = match fs::read_to_string(&path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let doc: JsonValue = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let messages = match doc.get("messages").and_then(|v| v.as_array()) {
            Some(v) => v,
            None => continue,
        };
        let mut hits = 0usize;
        for m in messages {
            let content = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
            if content.to_ascii_lowercase().contains(&needle) {
                hits += 1;
            }
        }
        if hits > 0 {
            out.push(format!("{} ({} hit(s))", path, hits));
        }
    }
    Ok(out)
}

fn print_session_tail(path: &str, messages: &[JsonValue], n: usize) {
    println!("session: {}", path);
    println!("tail: {} message(s)", n);
    let start = messages.len().saturating_sub(n);
    for (idx, m) in messages.iter().enumerate().skip(start) {
        let role = m.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let content = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let single_line = content.replace('\n', " ");
        println!("[{}] {}: {}", idx + 1, role, single_line);
    }
}

fn session_stats() -> Result<JsonValue, String> {
    let sessions = list_saved_sessions()?;
    let mut total_messages = 0usize;
    let mut providers: BTreeMap<String, usize> = BTreeMap::new();
    for path in &sessions {
        let raw = match fs::read_to_string(path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let doc: JsonValue = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let count = doc
            .get("messages")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        total_messages += count;
        let provider = doc
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        *providers.entry(provider).or_insert(0) += 1;
    }
    Ok(json!({
        "ok": true,
        "sessions": sessions.len(),
        "total_messages": total_messages,
        "providers": providers
    }))
}

fn session_report(
    json_output: bool,
    from_saved_at: Option<&str>,
    to_saved_at: Option<&str>,
    top_n: usize,
) -> Result<String, String> {
    let sessions = list_saved_sessions()?;
    let mut provider_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut model_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut tag_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_messages = 0usize;
    let mut filtered_sessions = 0usize;
    let mut min_saved_at = String::new();
    let mut max_saved_at = String::new();

    for path in &sessions {
        let raw = match fs::read_to_string(path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let doc: JsonValue = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let provider = doc
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let model = doc
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let saved_at = doc
            .get("saved_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(from) = from_saved_at {
            if !saved_at.is_empty() && saved_at.as_str() < from {
                continue;
            }
        }
        if let Some(to) = to_saved_at {
            if !saved_at.is_empty() && saved_at.as_str() > to {
                continue;
            }
        }
        let message_count = doc
            .get("messages")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        filtered_sessions += 1;

        *provider_counts.entry(provider).or_insert(0) += 1;
        *model_counts.entry(model).or_insert(0) += 1;
        total_messages += message_count;

        if !saved_at.is_empty() {
            if min_saved_at.is_empty() || saved_at < min_saved_at {
                min_saved_at = saved_at.clone();
            }
            if max_saved_at.is_empty() || saved_at > max_saved_at {
                max_saved_at = saved_at.clone();
            }
        }

        if let Some(tag) = infer_session_tag_from_path(path) {
            *tag_counts.entry(tag).or_insert(0) += 1;
        }
    }

    let top_providers = top_n_counts(&provider_counts, top_n);
    let top_models = top_n_counts(&model_counts, top_n);
    let top_tags = top_n_counts(&tag_counts, top_n);
    let avg_messages = if filtered_sessions == 0 {
        0.0
    } else {
        total_messages as f64 / filtered_sessions as f64
    };

    if json_output {
        return Ok(json!({
            "ok": true,
            "sessions": filtered_sessions,
            "messages_total": total_messages,
            "messages_avg": avg_messages,
            "filter_from": from_saved_at,
            "filter_to": to_saved_at,
            "top_n": top_n,
            "saved_at_min": if min_saved_at.is_empty() { JsonValue::Null } else { json!(min_saved_at) },
            "saved_at_max": if max_saved_at.is_empty() { JsonValue::Null } else { json!(max_saved_at) },
            "top_providers": top_providers,
            "top_models": top_models,
            "top_tags": top_tags
        }).to_string());
    }

    let mut out = String::new();
    out.push_str("session report\n");
    out.push_str(&format!("sessions: {}\n", filtered_sessions));
    out.push_str(&format!("messages total: {}\n", total_messages));
    out.push_str(&format!("messages avg: {:.2}\n", avg_messages));
    if let Some(from) = from_saved_at {
        out.push_str(&format!("filter from: {}\n", from));
    }
    if let Some(to) = to_saved_at {
        out.push_str(&format!("filter to: {}\n", to));
    }
    out.push_str(&format!("top n: {}\n", top_n));
    if !min_saved_at.is_empty() && !max_saved_at.is_empty() {
        out.push_str(&format!(
            "saved_at range: {} .. {}\n",
            min_saved_at, max_saved_at
        ));
    }
    out.push_str(&format!("top providers: {:?}\n", top_providers));
    out.push_str(&format!("top models: {:?}\n", top_models));
    out.push_str(&format!("top tags: {:?}", top_tags));
    Ok(out)
}

fn infer_session_tag_from_path(path: &str) -> Option<String> {
    let file = Path::new(path).file_stem()?.to_str()?;
    let rest = file.strip_prefix("session_")?;
    let parts = rest.split('_').collect::<Vec<_>>();
    if parts.len() < 3 {
        return None;
    }
    Some(parts[1..parts.len() - 1].join("_"))
}

fn top_n_counts(map: &BTreeMap<String, usize>, n: usize) -> Vec<JsonValue> {
    let mut pairs = map.iter().map(|(k, v)| (k.clone(), *v)).collect::<Vec<_>>();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    pairs
        .into_iter()
        .take(n)
        .map(|(name, count)| json!({ "name": name, "count": count }))
        .collect()
}

fn session_report_markdown(
    from_saved_at: Option<&str>,
    to_saved_at: Option<&str>,
    top_n: usize,
) -> Result<String, String> {
    let json_raw = session_report(true, from_saved_at, to_saved_at, top_n)?;
    let doc: JsonValue =
        serde_json::from_str(&json_raw).map_err(|e| format!("parse report json failed: {}", e))?;

    let dir = Path::new("env").join("reports");
    fs::create_dir_all(&dir).map_err(|e| format!("create report dir failed: {}", e))?;
    let path = dir.join(format!("session_report_{}.md", chrono_like_timestamp()));

    let mut md = String::new();
    md.push_str("# Session Report\n\n");
    md.push_str(&format!(
        "- Sessions: `{}`\n",
        doc.get("sessions").and_then(|v| v.as_u64()).unwrap_or(0)
    ));
    md.push_str(&format!(
        "- Total Messages: `{}`\n",
        doc.get("messages_total")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    ));
    md.push_str(&format!(
        "- Avg Messages: `{:.2}`\n",
        doc.get("messages_avg")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    ));
    md.push_str(&format!(
        "- Time Range: `{}` .. `{}`\n\n",
        doc.get("saved_at_min")
            .and_then(|v| v.as_str())
            .unwrap_or("n/a"),
        doc.get("saved_at_max")
            .and_then(|v| v.as_str())
            .unwrap_or("n/a")
    ));

    md.push_str("## Top Providers\n");
    if let Some(items) = doc.get("top_providers").and_then(|v| v.as_array()) {
        for it in items {
            let name = it.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
            let count = it.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
            md.push_str(&format!("- `{}`: `{}`\n", name, count));
        }
    }
    md.push_str("\n## Top Models\n");
    if let Some(items) = doc.get("top_models").and_then(|v| v.as_array()) {
        for it in items {
            let name = it.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
            let count = it.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
            md.push_str(&format!("- `{}`: `{}`\n", name, count));
        }
    }
    md.push_str("\n## Top Tags\n");
    if let Some(items) = doc.get("top_tags").and_then(|v| v.as_array()) {
        for it in items {
            let name = it.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
            let count = it.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
            md.push_str(&format!("- `{}`: `{}`\n", name, count));
        }
    }

    fs::write(&path, md).map_err(|e| format!("write report markdown failed: {}", e))?;
    Ok(path.display().to_string())
}

fn session_report_csv(
    from_saved_at: Option<&str>,
    to_saved_at: Option<&str>,
    top_n: usize,
) -> Result<String, String> {
    let sessions = list_saved_sessions()?;
    let dir = Path::new("env").join("reports");
    fs::create_dir_all(&dir).map_err(|e| format!("create report dir failed: {}", e))?;
    let path = dir.join(format!("session_report_{}.csv", chrono_like_timestamp()));

    let mut csv = String::new();
    csv.push_str(&format!("# top_n={}\n", top_n));
    csv.push_str("path,provider,model,tag,saved_at,message_count\n");
    for session_path in sessions {
        let raw = match fs::read_to_string(&session_path) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let doc: JsonValue = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let saved_at = doc
            .get("saved_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(from) = from_saved_at {
            if !saved_at.is_empty() && saved_at.as_str() < from {
                continue;
            }
        }
        if let Some(to) = to_saved_at {
            if !saved_at.is_empty() && saved_at.as_str() > to {
                continue;
            }
        }
        let provider = doc
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let model = doc
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let tag =
            infer_session_tag_from_path(&session_path).unwrap_or_else(|| "unknown".to_string());
        let message_count = doc
            .get("messages")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{}\n",
            session_path.replace('"', "'"),
            provider.replace('"', "'"),
            model.replace('"', "'"),
            tag.replace('"', "'"),
            saved_at.replace('"', "'"),
            message_count
        ));
    }

    fs::write(&path, csv).map_err(|e| format!("write report csv failed: {}", e))?;
    Ok(path.display().to_string())
}

fn session_report_html(
    from_saved_at: Option<&str>,
    to_saved_at: Option<&str>,
    top_n: usize,
) -> Result<String, String> {
    let json_raw = session_report(true, from_saved_at, to_saved_at, top_n)?;
    let doc: JsonValue =
        serde_json::from_str(&json_raw).map_err(|e| format!("parse report json failed: {}", e))?;
    let dir = Path::new("env").join("reports");
    fs::create_dir_all(&dir).map_err(|e| format!("create report dir failed: {}", e))?;
    let path = dir.join(format!("session_report_{}.html", chrono_like_timestamp()));

    let render_items = |arr: Option<&Vec<JsonValue>>| -> String {
        let mut out = String::new();
        if let Some(items) = arr {
            for it in items {
                let name = it.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                let count = it.get("count").and_then(|v| v.as_u64()).unwrap_or(0);
                out.push_str(&format!("<li><strong>{}</strong>: {}</li>", name, count));
            }
        }
        out
    };

    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Session Report</title><style>body{{font-family:Arial,sans-serif;max-width:900px;margin:24px auto;padding:0 12px}}h1{{margin-bottom:8px}}.kpi{{display:grid;grid-template-columns:repeat(3,minmax(120px,1fr));gap:12px;margin:16px 0}}.card{{border:1px solid #ddd;border-radius:8px;padding:12px}}ul{{margin:8px 0 0 20px}}</style></head><body><h1>Session Report</h1><p>Range: {} .. {}</p><div class=\"kpi\"><div class=\"card\"><div>Sessions</div><strong>{}</strong></div><div class=\"card\"><div>Total Messages</div><strong>{}</strong></div><div class=\"card\"><div>Avg Messages</div><strong>{:.2}</strong></div></div><div class=\"card\"><h3>Top Providers</h3><ul>{}</ul></div><div class=\"card\"><h3>Top Models</h3><ul>{}</ul></div><div class=\"card\"><h3>Top Tags</h3><ul>{}</ul></div></body></html>",
        doc.get("saved_at_min").and_then(|v| v.as_str()).unwrap_or("n/a"),
        doc.get("saved_at_max").and_then(|v| v.as_str()).unwrap_or("n/a"),
        doc.get("sessions").and_then(|v| v.as_u64()).unwrap_or(0),
        doc.get("messages_total").and_then(|v| v.as_u64()).unwrap_or(0),
        doc.get("messages_avg").and_then(|v| v.as_f64()).unwrap_or(0.0),
        render_items(doc.get("top_providers").and_then(|v| v.as_array())),
        render_items(doc.get("top_models").and_then(|v| v.as_array())),
        render_items(doc.get("top_tags").and_then(|v| v.as_array())),
    );

    fs::write(&path, html).map_err(|e| format!("write report html failed: {}", e))?;
    Ok(path.display().to_string())
}

fn export_session_markdown(path: &str) -> Result<String, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read session failed: {}", e))?;
    let doc: JsonValue =
        serde_json::from_str(&raw).map_err(|e| format!("parse session failed: {}", e))?;
    let messages = doc
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "session missing messages".to_string())?;
    let out_path = Path::new(path).with_extension("md");

    let mut md = String::new();
    md.push_str("# Agent Session Export\n\n");
    md.push_str(&format!("- Source: `{}`\n", path));
    md.push_str(&format!(
        "- Provider: `{}`\n",
        doc.get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
    ));
    md.push_str(&format!(
        "- Model: `{}`\n",
        doc.get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
    ));
    md.push_str(&format!("- Messages: `{}`\n\n", messages.len()));

    for (idx, m) in messages.iter().enumerate() {
        let role = m.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let content = m.get("content").and_then(|v| v.as_str()).unwrap_or("");
        md.push_str(&format!("## {}. {}\n\n", idx + 1, role));
        md.push_str(content);
        md.push_str("\n\n");
    }

    fs::write(&out_path, md).map_err(|e| format!("write markdown failed: {}", e))?;
    Ok(out_path.display().to_string())
}

fn session_diff(a_path: &str, b_path: &str, json_output: bool) -> Result<String, String> {
    let a_raw =
        fs::read_to_string(a_path).map_err(|e| format!("read left session failed: {}", e))?;
    let b_raw =
        fs::read_to_string(b_path).map_err(|e| format!("read right session failed: {}", e))?;

    let a_doc: JsonValue =
        serde_json::from_str(&a_raw).map_err(|e| format!("parse left session failed: {}", e))?;
    let b_doc: JsonValue =
        serde_json::from_str(&b_raw).map_err(|e| format!("parse right session failed: {}", e))?;

    let a_msgs = a_doc
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "left session missing messages".to_string())?;
    let b_msgs = b_doc
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "right session missing messages".to_string())?;

    let mut a_roles: BTreeMap<String, usize> = BTreeMap::new();
    let mut b_roles: BTreeMap<String, usize> = BTreeMap::new();
    for m in a_msgs {
        let role = m
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        *a_roles.entry(role).or_insert(0) += 1;
    }
    for m in b_msgs {
        let role = m
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        *b_roles.entry(role).or_insert(0) += 1;
    }

    let a_provider = a_doc
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let b_provider = b_doc
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let a_model = a_doc
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let b_model = b_doc
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let min_len = a_msgs.len().min(b_msgs.len());
    let mut first_divergence: Option<usize> = None;
    for i in 0..min_len {
        if a_msgs[i] != b_msgs[i] {
            first_divergence = Some(i + 1);
            break;
        }
    }

    let a_last = a_msgs
        .last()
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .replace('\n', " ");
    let b_last = b_msgs
        .last()
        .and_then(|m| m.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .replace('\n', " ");

    let first_divergence_json = first_divergence.map(|i| i as u64);
    if json_output {
        return Ok(json!({
            "ok": true,
            "a_path": a_path,
            "b_path": b_path,
            "provider_a": a_provider,
            "provider_b": b_provider,
            "model_a": a_model,
            "model_b": b_model,
            "messages_a": a_msgs.len(),
            "messages_b": b_msgs.len(),
            "delta_messages": b_msgs.len() as isize - a_msgs.len() as isize,
            "roles_a": a_roles,
            "roles_b": b_roles,
            "first_divergence": first_divergence_json,
            "last_a": truncate_preview(&a_last, 240),
            "last_b": truncate_preview(&b_last, 240),
        })
        .to_string());
    }

    let mut out = String::new();
    out.push_str("session diff\n");
    out.push_str(&format!("A: {}\n", a_path));
    out.push_str(&format!("B: {}\n", b_path));
    out.push_str(&format!(
        "provider: {} vs {}\nmodel: {} vs {}\n",
        a_provider, b_provider, a_model, b_model
    ));
    out.push_str(&format!(
        "messages: {} vs {} (delta: {:+})\n",
        a_msgs.len(),
        b_msgs.len(),
        b_msgs.len() as isize - a_msgs.len() as isize
    ));
    out.push_str(&format!("roles A: {:?}\nroles B: {:?}\n", a_roles, b_roles));
    match first_divergence {
        Some(i) => out.push_str(&format!("first divergence at message #{}\n", i)),
        None => {
            if a_msgs.len() == b_msgs.len() {
                out.push_str("messages are identical\n");
            } else {
                out.push_str("shared prefix identical; one session has extra tail messages\n");
            }
        }
    }

    out.push_str(&format!("last A: {}\n", truncate_preview(&a_last, 240)));
    out.push_str(&format!("last B: {}", truncate_preview(&b_last, 240)));
    Ok(out)
}

fn truncate_preview(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let mut out = input.chars().take(max_chars).collect::<String>();
    out.push_str("...");
    out
}

fn chrono_like_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    now.to_string()
}

fn sanitize_session_tag(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn chat_completion_via_curl(
    provider: &str,
    api_key: &str,
    model: &str,
    messages: &[JsonValue],
) -> Result<String, String> {
    let payload = json!({
        "model": model,
        "messages": messages
    });
    let payload_str =
        serde_json::to_string(&payload).map_err(|e| format!("serialize payload failed: {}", e))?;

    let output = Command::new("curl")
        .arg("-sS")
        .arg(format!("{}/chat/completions", provider_base_url(provider)))
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("Authorization: Bearer {}", api_key))
        .arg("-d")
        .arg(payload_str)
        .output()
        .map_err(|e| format!("failed to execute curl: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let doc: JsonValue =
        serde_json::from_str(&body).map_err(|e| format!("invalid API JSON: {}", e))?;

    if let Some(message) = doc
        .get("choices")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("message"))
        .and_then(|v| v.get("content"))
        .and_then(|v| v.as_str())
    {
        return Ok(message.to_string());
    }

    if let Some(api_error) = doc
        .get("error")
        .and_then(|v| v.get("message"))
        .and_then(|v| v.as_str())
    {
        return Err(api_error.to_string());
    }

    Err("missing assistant message in API response".to_string())
}

fn provider_base_url(provider: &str) -> &'static str {
    match provider.to_ascii_lowercase().as_str() {
        "nvidia" => "https://integrate.api.nvidia.com/v1",
        _ => "https://api.openai.com/v1",
    }
}

fn resolve_agent_api_key(cli_key: &str) -> Option<String> {
    if !cli_key.trim().is_empty() {
        return Some(cli_key.to_string());
    }
    env::var("OPENAI_API_KEY")
        .ok()
        .or_else(|| env::var("MATTER_AGENT_API_KEY").ok())
}

fn list_models_via_curl(provider: &str, api_key: &str) -> Result<Vec<String>, String> {
    let output = Command::new("curl")
        .arg("-sS")
        .arg(format!("{}/models", provider_base_url(provider)))
        .arg("-H")
        .arg(format!("Authorization: Bearer {}", api_key))
        .output()
        .map_err(|e| format!("failed to execute curl: {}", e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let body = String::from_utf8_lossy(&output.stdout);
    let doc: JsonValue =
        serde_json::from_str(&body).map_err(|e| format!("invalid model list JSON: {}", e))?;
    let mut out = Vec::new();
    if let Some(arr) = doc.get("data").and_then(|d| d.as_array()) {
        for item in arr {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                out.push(id.to_string());
            }
        }
    }
    Ok(out)
}

fn run_lsp() {
    eprintln!("Starting Matter Language Server...");
    eprintln!("Listening on stdio");

    // Start the LSP server using tokio runtime
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to initialize tokio runtime: {}", e);
            process::exit(1);
        }
    };
    rt.block_on(async {
        matter_lsp::start_server().await;
    });
}

fn run_debug(file: &str) {
    use matter_debugger::{DebugAdapter, DebugInfo, InstrumentedVM};

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║              Matter Debugger - Interactive Debug               ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    // Read source
    let source = if file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
        buffer
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        })
    };

    // Parse
    let mut parser = Parser::from_source(&source);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    // Build bytecode
    let builder = BytecodeBuilder::new();
    let bytecode = match builder.build_checked(&program) {
        Ok(bytecode) => bytecode,
        Err(e) => {
            eprintln!("Semantic error: {}", e);
            process::exit(1);
        }
    };

    // Create debug info (basic for now)
    let debug_info = DebugInfo::new();

    // Create instrumented VM
    let vm = InstrumentedVM::new(bytecode, debug_info);
    let mut adapter = DebugAdapter::new(vm);

    if let Err(e) = adapter.initialize() {
        eprintln!("Failed to initialize debugger adapter: {}", e);
        process::exit(1);
    }

    println!("Debug session started. Type 'help' for commands.");
    println!();

    // Debug REPL
    loop {
        print!("(debug) ");
        if let Err(e) = io::stdout().flush() {
            eprintln!("Error flushing stdout: {}", e);
            break;
        }

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let command = parts[0];

        match command {
            "help" | "h" => {
                println!("Debug Commands:");
                println!("  break <line>        Set breakpoint at line");
                println!("  continue, c         Continue execution");
                println!("  step, s             Step into");
                println!("  next, n             Step over");
                println!("  out, o              Step out");
                println!("  list, l             List breakpoints");
                println!("  locals              Show local variables");
                println!("  globals             Show global variables");
                println!("  stack               Show call stack");
                println!("  quit, q             Exit debugger");
                println!("  help, h             Show this help");
            }
            "break" | "b" => {
                if parts.len() < 2 {
                    println!("Usage: break <line>");
                    continue;
                }

                if let Ok(line) = parts[1].parse::<usize>() {
                    let file_name = if file == "-" { "stdin" } else { file };
                    let ids = adapter.set_breakpoints(file_name.to_string(), vec![line]);
                    println!("Breakpoint {} set at line {}", ids[0], line);
                } else {
                    println!("Invalid line number");
                }
            }
            "continue" | "c" => match adapter.continue_execution() {
                Ok(_) => {
                    if adapter.state() == matter_debugger::DebugState::Stopped {
                        println!("Program finished.");
                    } else {
                        println!("Breakpoint hit.");
                    }
                }
                Err(e) => println!("Error: {}", e),
            },
            "step" | "s" => match adapter.step_into() {
                Ok(_) => println!("Stepped."),
                Err(e) => println!("Error: {}", e),
            },
            "next" | "n" => match adapter.step_over() {
                Ok(_) => println!("Stepped over."),
                Err(e) => println!("Error: {}", e),
            },
            "out" | "o" => match adapter.step_out() {
                Ok(_) => println!("Stepped out."),
                Err(e) => println!("Error: {}", e),
            },
            "locals" => {
                let locals = adapter.get_variables("locals");
                if locals.is_empty() {
                    println!("No local variables.");
                } else {
                    println!("Local variables:");
                    for (name, value) in locals {
                        println!("  {} = {:?}", name, value);
                    }
                }
            }
            "globals" => {
                let globals = adapter.get_variables("globals");
                if globals.is_empty() {
                    println!("No global variables.");
                } else {
                    println!("Global variables:");
                    for (name, value) in globals {
                        println!("  {} = {:?}", name, value);
                    }
                }
            }
            "stack" => {
                let frames = adapter.get_stack_trace();
                println!("Call stack:");
                for frame in frames {
                    println!(
                        "  #{}: {} at {}:{}",
                        frame.id, frame.name, frame.file, frame.line
                    );
                }
            }
            "quit" | "q" => {
                println!("Exiting debugger.");
                break;
            }
            _ => {
                println!("Unknown command: {}. Type 'help' for commands.", command);
            }
        }
    }
}

fn run_format(file: &str, write: bool) {
    use matter_formatter::Formatter;

    // Read source
    let source = if file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
        buffer
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        })
    };

    // Format
    let formatter = Formatter::with_default_config();
    let formatted = match formatter.format(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Format error: {}", e);
            process::exit(1);
        }
    };

    if write && file != "-" {
        // Write back to file
        fs::write(file, formatted).unwrap_or_else(|e| {
            eprintln!("Error writing file: {}", e);
            process::exit(1);
        });
        println!("Formatted {}", file);
    } else {
        // Print to stdout
        println!("{}", formatted);
    }
}

fn run_lint(file: &str) {
    use matter_linter::Linter;

    // Read source
    let source = if file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
        buffer
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        })
    };

    // Lint
    let linter = Linter::with_default_config();
    let diagnostics = match linter.lint(&source) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Lint error: {}", e);
            process::exit(1);
        }
    };

    if diagnostics.is_empty() {
        println!("✓ No issues found");
    } else {
        println!("Found {} issue(s):\n", diagnostics.len());
        for diag in diagnostics {
            let severity_str = match diag.severity {
                matter_linter::Severity::Error => "Error",
                matter_linter::Severity::Warning => "Warning",
                matter_linter::Severity::Info => "Info",
                matter_linter::Severity::Hint => "Hint",
            };
            println!("{}: {} [{}]", severity_str, diag.message, diag.rule);
        }
        process::exit(1);
    }
}

// Sprint 24 Phase 4: GC Commands

fn gc_stats(file: &str) {
    // Read and compile source
    let source = if file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
        buffer
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        })
    };

    let bytecode = match compile_source(&source, file) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            process::exit(1);
        }
    };

    // Run program
    let mut runtime = Runtime::new_silent(bytecode);
    if let Err(e) = runtime.run() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }

    // Get and display GC statistics
    println!("=== Memory Management Statistics ===\n");

    // Memory Pool Stats
    let pool_stats = runtime.vm().memory_pool_stats();
    println!("{}", pool_stats);

    // Cycle Detector Stats
    let cycle_stats = runtime.vm().cycle_detector_stats();
    println!("{}", cycle_stats);

    // Summary
    println!("=== Summary ===");
    println!("  GC Threshold:       {}", runtime.vm().gc_threshold());
    println!(
        "  Pool Efficiency:    {:.2}%",
        if pool_stats.total_allocated > 0 {
            (pool_stats.total_used as f64 / pool_stats.total_allocated as f64) * 100.0
        } else {
            0.0
        }
    );
}

fn gc_collect(file: &str) {
    // Read and compile source
    let source = if file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
        buffer
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        })
    };

    let bytecode = match compile_source(&source, file) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            process::exit(1);
        }
    };

    // Run program
    let mut runtime = Runtime::new_silent(bytecode);
    if let Err(e) = runtime.run() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }

    // Force GC collection
    println!("Running garbage collection...\n");
    let result = runtime.vm().force_gc();

    println!("=== GC Collection Results ===");
    println!("  Cycles found:       {}", result.cycles_found);
    println!("  Objects collected:  {}", result.objects_collected);

    if result.cycles_found > 0 {
        println!(
            "\n⚠ Warning: {} cycle(s) detected and collected",
            result.cycles_found
        );
    } else {
        println!("\n✓ No cycles detected");
    }

    // Show updated stats
    let stats = runtime.vm().cycle_detector_stats();
    println!("\n=== Updated Statistics ===");
    println!("  Total collections:  {}", stats.collections_run);
    println!("  Total cycles:       {}", stats.cycles_detected);
    println!("  Total collected:    {}", stats.objects_collected);
}

fn gc_profile(file: &str) {
    // Read and compile source
    let source = if file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
        buffer
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        })
    };

    let bytecode = match compile_source(&source, file) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            process::exit(1);
        }
    };

    // Profile memory usage during execution
    println!("=== Memory Profiling ===\n");
    println!("Running program and profiling memory usage...\n");

    let mut runtime = Runtime::new_silent(bytecode);

    // Get initial stats
    let pool_before = runtime.vm().memory_pool_stats();
    let cycle_before = runtime.vm().cycle_detector_stats();

    // Run program
    if let Err(e) = runtime.run() {
        eprintln!("Runtime error: {}", e);
        process::exit(1);
    }

    // Get final stats
    let pool_after = runtime.vm().memory_pool_stats();
    let cycle_after = runtime.vm().cycle_detector_stats();

    // Display profile
    println!("=== Memory Pool Profile ===");
    println!(
        "  Chunks allocated:   {} -> {}",
        pool_before.chunk_count, pool_after.chunk_count
    );
    println!(
        "  Total allocated:    {} -> {} bytes",
        pool_before.total_allocated, pool_after.total_allocated
    );
    println!(
        "  Total used:         {} -> {} bytes",
        pool_before.total_used, pool_after.total_used
    );
    println!(
        "  Allocations:        {} -> {}",
        pool_before.allocation_count, pool_after.allocation_count
    );
    println!(
        "  Fragmentation:      {:.2}% -> {:.2}%",
        pool_before.fragmentation, pool_after.fragmentation
    );

    println!("\n=== Cycle Detector Profile ===");
    println!(
        "  Tracked objects:    {} -> {}",
        cycle_before.tracked_objects, cycle_after.tracked_objects
    );
    println!(
        "  Collections run:    {} -> {}",
        cycle_before.collections_run, cycle_after.collections_run
    );
    println!(
        "  Cycles detected:    {} -> {}",
        cycle_before.cycles_detected, cycle_after.cycles_detected
    );
    println!(
        "  Objects collected:  {} -> {}",
        cycle_before.objects_collected, cycle_after.objects_collected
    );

    // Analysis
    println!("\n=== Analysis ===");
    let allocations_delta = pool_after.allocation_count - pool_before.allocation_count;
    let collections_delta = cycle_after.collections_run - cycle_before.collections_run;

    if allocations_delta > 0 {
        println!("  ✓ {} allocation(s) during execution", allocations_delta);
    }

    if collections_delta > 0 {
        println!("  ✓ {} GC collection(s) triggered", collections_delta);
    }

    if cycle_after.cycles_detected > cycle_before.cycles_detected {
        println!(
            "  ⚠ {} cycle(s) detected",
            cycle_after.cycles_detected - cycle_before.cycles_detected
        );
    } else {
        println!("  ✓ No memory leaks detected");
    }

    // Recommendations
    println!("\n=== Recommendations ===");
    if pool_after.fragmentation > 50.0 {
        println!(
            "  • High fragmentation ({:.2}%) - consider resetting memory pool periodically",
            pool_after.fragmentation
        );
    }

    if cycle_after.tracked_objects > 1000 {
        println!(
            "  • Many tracked objects ({}) - consider running GC more frequently",
            cycle_after.tracked_objects
        );
    }

    let gc_threshold = runtime.vm().gc_threshold();
    if allocations_delta > gc_threshold * 2 {
        println!(
            "  • High allocation rate - consider lowering GC threshold (current: {})",
            gc_threshold
        );
    }
}

// Sprint 25: LLVM Backend Commands

/// Show LLVM IR for a Matter source file
fn show_llvm_ir(file: &str) {
    // Read source
    let source = if file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
        buffer
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        })
    };

    // Compile to bytecode
    let _bytecode = match compile_source(&source, file) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            process::exit(1);
        }
    };

    // Generate LLVM IR
    #[cfg(feature = "llvm")]
    {
        match matter_llvm::get_llvm_ir(&_bytecode) {
            Ok(ir) => {
                println!("{}", ir);
            }
            Err(e) => {
                eprintln!("LLVM IR generation error: {}", e);
                process::exit(1);
            }
        }
    }

    #[cfg(not(feature = "llvm"))]
    {
        eprintln!("Error: LLVM support not enabled");
        eprintln!("Rebuild with: cargo build --features llvm");
        process::exit(1);
    }
}

/// Compile Matter source to native executable
fn compile_to_native(file: &str, output: &str, opt_level: Option<&str>) {
    // Read source
    let source = if file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
        buffer
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        })
    };

    // Compile to bytecode
    let bytecode = match compile_source(&source, file) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            process::exit(1);
        }
    };

    // Parse optimization level
    let opt = match opt_level {
        Some("-O0") => matter_native::OptLevel::O0,
        Some("-O1") => matter_native::OptLevel::O1,
        Some("-O2") => matter_native::OptLevel::O2,
        Some("-O3") | None => matter_native::OptLevel::O3,
        Some(other) => {
            eprintln!("Error: Unknown optimization level: {}", other);
            eprintln!("Valid options: -O0, -O1, -O2, -O3");
            process::exit(1);
        }
    };

    // Create compiler configuration
    let config = matter_native::CompileConfig {
        arch: matter_native::Architecture::X86_64,
        os: if cfg!(target_os = "windows") {
            matter_native::OperatingSystem::Windows
        } else if cfg!(target_os = "linux") {
            matter_native::OperatingSystem::Linux
        } else {
            matter_native::OperatingSystem::MacOS
        },
        opt_level: opt,
        debug_info: false,
        enable_lto: opt != matter_native::OptLevel::O0,
    };

    // Create compiler
    let compiler = matter_native::NativeCompiler::with_config(config);

    // Add .exe extension on Windows if not present
    let output_path = if cfg!(target_os = "windows") && !output.ends_with(".exe") {
        format!("{}.exe", output)
    } else {
        output.to_string()
    };

    // Compile to native executable
    println!("Compiling {} to native executable...", file);
    println!("  Architecture: x86-64");
    println!("  Optimization: {:?}", opt);
    println!("  Output: {}", output_path);

    match compiler.compile_to_executable(&bytecode, &output_path) {
        Ok(_) => {
            println!("✓ Successfully compiled to native: {}", output_path);
            println!(
                "  Run with: {}",
                if cfg!(target_os = "windows") {
                    output_path.clone()
                } else {
                    format!("./{}", output_path)
                }
            );
        }
        Err(e) => {
            eprintln!("Native compilation error: {}", e);
            process::exit(1);
        }
    }
}

/// Compile and run Matter source as native code
fn run_native(file: &str, opt_level: Option<&str>) {
    // Read source
    let source = if file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
        buffer
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        })
    };

    // Compile to bytecode
    let bytecode = match compile_source(&source, file) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            process::exit(1);
        }
    };

    // Parse optimization level
    let opt = match opt_level {
        Some("-O0") => matter_native::OptLevel::O0,
        Some("-O1") => matter_native::OptLevel::O1,
        Some("-O2") => matter_native::OptLevel::O2,
        Some("-O3") | None => matter_native::OptLevel::O3,
        Some(other) => {
            eprintln!("Error: Unknown optimization level: {}", other);
            eprintln!("Valid options: -O0, -O1, -O2, -O3");
            process::exit(1);
        }
    };

    // Create compiler configuration
    let config = matter_native::CompileConfig {
        arch: matter_native::Architecture::X86_64,
        os: if cfg!(target_os = "windows") {
            matter_native::OperatingSystem::Windows
        } else if cfg!(target_os = "linux") {
            matter_native::OperatingSystem::Linux
        } else {
            matter_native::OperatingSystem::MacOS
        },
        opt_level: opt,
        debug_info: false,
        enable_lto: opt != matter_native::OptLevel::O0,
    };

    // Create compiler
    let compiler = matter_native::NativeCompiler::with_config(config);

    // Compile to temporary executable
    let temp_output = if cfg!(target_os = "windows") {
        ".matter_temp_native.exe"
    } else {
        ".matter_temp_native"
    };
    let temp_command = if cfg!(target_os = "windows") {
        format!(".\\{}", temp_output)
    } else {
        format!("./{}", temp_output)
    };

    println!("Compiling to native and running...");

    match compiler.compile_to_executable(&bytecode, temp_output) {
        Ok(_) => {
            // Run the executable
            let output = std::process::Command::new(&temp_command)
                .output()
                .unwrap_or_else(|e| {
                    eprintln!("Error running native executable: {}", e);
                    // Clean up
                    let _ = fs::remove_file(temp_output);
                    process::exit(1);
                });

            // Print output
            print!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));

            // Clean up temporary executable
            let _ = fs::remove_file(temp_output);

            // Exit with same code as the program
            if !output.status.success() {
                process::exit(output.status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("Native compilation error: {}", e);
            process::exit(1);
        }
    }
}

#[derive(Debug, Clone)]
struct BenchmarkStats {
    average: std::time::Duration,
    median: std::time::Duration,
    p95: std::time::Duration,
    min: std::time::Duration,
    max: std::time::Duration,
    stddev_ns: f64,
}

#[derive(Debug, Clone)]
struct BenchmarkGateOptions {
    report_path: String,
    max_average_ns: Option<u64>,
    max_median_ns: Option<u64>,
    max_p95_ns: Option<u64>,
    ci_exit_codes: bool,
}

fn parse_benchmark_options(args: &[String]) -> (usize, bool) {
    let mut iterations = 10usize;
    let mut json_output = false;
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "--iterations" => {
                if let Some(value) = args.get(i + 1) {
                    iterations = value.parse::<usize>().unwrap_or(10);
                    i += 1;
                }
            }
            "--json" => json_output = true,
            _ => {}
        }
        i += 1;
    }

    (iterations.max(1), json_output)
}

fn parse_benchmark_gate_options(report_path: &str, args: &[String]) -> BenchmarkGateOptions {
    let mut options = BenchmarkGateOptions {
        report_path: report_path.to_string(),
        max_average_ns: None,
        max_median_ns: None,
        max_p95_ns: None,
        ci_exit_codes: false,
    };
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "--max-average-ns" => {
                if let Some(value) = args.get(i + 1) {
                    options.max_average_ns = value.parse::<u64>().ok();
                    i += 1;
                }
            }
            "--max-median-ns" => {
                if let Some(value) = args.get(i + 1) {
                    options.max_median_ns = value.parse::<u64>().ok();
                    i += 1;
                }
            }
            "--max-p95-ns" => {
                if let Some(value) = args.get(i + 1) {
                    options.max_p95_ns = value.parse::<u64>().ok();
                    i += 1;
                }
            }
            "--ci-exit-codes" => options.ci_exit_codes = true,
            _ => {}
        }
        i += 1;
    }

    options
}

fn benchmark_metric(report: &JsonValue, metric: &str) -> Option<u64> {
    report
        .get("bytecode")
        .and_then(|v| v.get("stats"))
        .and_then(|v| v.get(metric))
        .and_then(|v| v.as_u64())
}

fn benchmark_gate_check(name: &str, actual: Option<u64>, limit: Option<u64>) -> JsonValue {
    match (actual, limit) {
        (Some(actual), Some(limit)) => json!({
            "metric": name,
            "actual_ns": actual,
            "limit_ns": limit,
            "passed": actual <= limit
        }),
        (Some(actual), None) => json!({
            "metric": name,
            "actual_ns": actual,
            "limit_ns": null,
            "passed": true,
            "skipped": true
        }),
        _ => json!({
            "metric": name,
            "actual_ns": null,
            "limit_ns": limit,
            "passed": false,
            "error": "metric missing"
        }),
    }
}

fn evaluate_benchmark_gate(
    report_source: &str,
    options: &BenchmarkGateOptions,
) -> (bool, JsonValue) {
    let report_json = report_source
        .find('{')
        .map(|start| &report_source[start..])
        .unwrap_or(report_source);
    let report = match serde_json::from_str::<JsonValue>(report_json) {
        Ok(report) => report,
        Err(e) => {
            return (
                false,
                json!({
                    "status": "fail",
                    "passed": false,
                    "error": format!("invalid benchmark JSON: {}", e)
                }),
            )
        }
    };

    let checks = vec![
        benchmark_gate_check(
            "average_ns",
            benchmark_metric(&report, "average_ns"),
            options.max_average_ns,
        ),
        benchmark_gate_check(
            "median_ns",
            benchmark_metric(&report, "median_ns"),
            options.max_median_ns,
        ),
        benchmark_gate_check(
            "p95_ns",
            benchmark_metric(&report, "p95_ns"),
            options.max_p95_ns,
        ),
    ];
    let passed = checks.iter().all(|check| {
        check
            .get("passed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    });

    (
        passed,
        json!({
            "status": if passed { "pass" } else { "fail" },
            "passed": passed,
            "file": report.get("file").cloned().unwrap_or(JsonValue::Null),
            "iterations": report.get("iterations").cloned().unwrap_or(JsonValue::Null),
            "checks": checks
        }),
    )
}

fn duration_ns(duration: std::time::Duration) -> u64 {
    duration.as_nanos().min(u64::MAX as u128) as u64
}

fn benchmark_stats(times: &[std::time::Duration]) -> BenchmarkStats {
    let mut sorted = times.to_vec();
    sorted.sort_unstable();

    let len = sorted.len().max(1);
    let total_ns: u128 = sorted.iter().map(|d| d.as_nanos()).sum();
    let average_ns = total_ns / len as u128;
    let average = std::time::Duration::from_nanos(average_ns.min(u64::MAX as u128) as u64);
    let median = sorted[len / 2];
    let p95_index = (((len as f64) * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(len - 1);
    let p95 = sorted[p95_index];
    let min = sorted[0];
    let max = sorted[len - 1];
    let avg_f64 = average_ns as f64;
    let variance = sorted
        .iter()
        .map(|d| {
            let delta = d.as_nanos() as f64 - avg_f64;
            delta * delta
        })
        .sum::<f64>()
        / len as f64;

    BenchmarkStats {
        average,
        median,
        p95,
        min,
        max,
        stddev_ns: variance.sqrt(),
    }
}

fn benchmark_stats_json(stats: &BenchmarkStats) -> JsonValue {
    json!({
        "average_ns": duration_ns(stats.average),
        "median_ns": duration_ns(stats.median),
        "p95_ns": duration_ns(stats.p95),
        "min_ns": duration_ns(stats.min),
        "max_ns": duration_ns(stats.max),
        "stddev_ns": stats.stddev_ns,
        "average_human": format!("{:?}", stats.average),
        "median_human": format!("{:?}", stats.median),
        "p95_human": format!("{:?}", stats.p95),
        "min_human": format!("{:?}", stats.min),
        "max_human": format!("{:?}", stats.max),
    })
}

fn json_object_u64_sum(value: &JsonValue) -> u64 {
    value
        .as_object()
        .map(|object| object.values().filter_map(|value| value.as_u64()).sum())
        .unwrap_or(0)
}

fn json_object_u64_get(value: &JsonValue, key: &str) -> u64 {
    value
        .as_object()
        .and_then(|object| object.get(key))
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}

fn perf_diagnose_report(file: &str, program: &Program, bytecode: &Bytecode) -> JsonValue {
    let ast_reflection: JsonValue =
        serde_json::from_str(&ast_reflection_json(program)).expect("valid ast reflection JSON");
    let bytecode_reflection: JsonValue = serde_json::from_str(&bytecode_reflection_json(bytecode))
        .expect("valid bytecode reflection JSON");

    let statement_kinds = &ast_reflection["statement_kinds"];
    let opcode_histogram = &bytecode_reflection["opcode_histogram"];
    let total_statements = ast_reflection["total_statements"].as_u64().unwrap_or(0);
    let functions = bytecode.functions.len() as u64;
    let instructions = bytecode_reflection["summary"]["instructions"]
        .as_u64()
        .unwrap_or(0);
    let function_instructions: u64 = bytecode
        .functions
        .values()
        .map(|function| function.instructions.len() as u64)
        .sum();
    let event_instructions: u64 = bytecode
        .event_handlers
        .values()
        .map(|handler| handler.instructions.len() as u64)
        .sum();
    let total_bytecode_instructions = instructions + function_instructions + event_instructions;
    let loops = json_object_u64_get(statement_kinds, "While")
        + json_object_u64_get(statement_kinds, "For")
        + json_object_u64_get(statement_kinds, "Loop");
    let calls = json_object_u64_sum(&ast_reflection["calls"]);
    let backend_calls = json_object_u64_sum(&ast_reflection["backend_calls"]);
    let recursive_functions = direct_recursive_functions(program);
    let call_opcodes = json_object_u64_get(opcode_histogram, "Call")
        + json_object_u64_get(opcode_histogram, "CallNamed")
        + json_object_u64_get(opcode_histogram, "BackendCall");
    let branch_opcodes = json_object_u64_get(opcode_histogram, "Jump")
        + json_object_u64_get(opcode_histogram, "JumpIfFalse");
    let allocation_opcodes = json_object_u64_get(opcode_histogram, "NewList")
        + json_object_u64_get(opcode_histogram, "NewMap")
        + json_object_u64_get(opcode_histogram, "NewStruct");

    let complexity_score = total_bytecode_instructions
        + total_statements * 3
        + loops * 25
        + call_opcodes * 8
        + backend_calls * 15
        + recursive_functions.len() as u64 * 40;
    let pressure = if complexity_score >= 500 {
        "high"
    } else if complexity_score >= 180 {
        "medium"
    } else {
        "low"
    };

    let mut checks = Vec::new();
    checks.push(json!({
        "name": "bytecode_size",
        "severity": if total_bytecode_instructions > 1000 { "warn" } else { "pass" },
        "actual": total_bytecode_instructions,
        "limit": 1000,
        "message": "large bytecode should be benchmarked and gated before release"
    }));
    checks.push(json!({
        "name": "loop_pressure",
        "severity": if loops > 0 { "warn" } else { "pass" },
        "actual": loops,
        "message": "loops are likely hot paths; benchmark representative input sizes"
    }));
    checks.push(json!({
        "name": "call_pressure",
        "severity": if call_opcodes > 25 { "warn" } else { "pass" },
        "actual": call_opcodes,
        "limit": 25,
        "message": "heavy call density can dominate VM execution time"
    }));
    checks.push(json!({
        "name": "recursion_pressure",
        "severity": if recursive_functions.is_empty() { "pass" } else { "warn" },
        "functions": recursive_functions,
        "message": "recursive functions need explicit benchmark budgets"
    }));
    checks.push(json!({
        "name": "backend_boundary_pressure",
        "severity": if backend_calls > 0 { "warn" } else { "pass" },
        "actual": backend_calls,
        "message": "backend/FFI calls should be batched when possible"
    }));

    #[cfg(feature = "llvm")]
    let native_acceleration = json!({
        "enabled": true,
        "status": "available",
        "message": "LLVM feature is enabled for native benchmark/build paths"
    });
    #[cfg(not(feature = "llvm"))]
    let native_acceleration = json!({
        "enabled": false,
        "status": "not_enabled",
        "message": "rebuild matter-cli with the llvm feature to benchmark native acceleration"
    });

    let status = if checks
        .iter()
        .any(|check| check["severity"].as_str() == Some("fail"))
    {
        "fail"
    } else if pressure != "low"
        || checks
            .iter()
            .any(|check| check["severity"].as_str() == Some("warn"))
    {
        "warn"
    } else {
        "pass"
    };

    json!({
        "ok": true,
        "status": status,
        "file": file,
        "engine": "matter-performance-diagnose-v1",
        "pressure": pressure,
        "complexity_score": complexity_score,
        "metrics": {
            "top_level_statements": ast_reflection["top_level_statements"].as_u64().unwrap_or(0),
            "total_statements": total_statements,
            "functions": functions,
            "event_handlers": bytecode.event_handlers.len(),
            "main_instructions": instructions,
            "function_instructions": function_instructions,
            "event_instructions": event_instructions,
            "total_bytecode_instructions": total_bytecode_instructions,
            "loops": loops,
            "calls": calls,
            "call_opcodes": call_opcodes,
            "branch_opcodes": branch_opcodes,
            "allocation_opcodes": allocation_opcodes,
            "backend_calls": backend_calls
        },
        "native_acceleration": native_acceleration,
        "checks": checks,
        "next_actions": [
            "run benchmark-json with representative inputs",
            "gate median and p95 latency with benchmark-gate-json",
            "batch backend/FFI calls on hot paths",
            "enable LLVM/native checks when the local LLVM headers are installed"
        ]
    })
}

fn perf_diagnose_json(file: &str) {
    let source = read_source_or_exit(file);
    let mut parser = Parser::from_source(&source);
    let program = match parser.parse() {
        Ok(program) => program,
        Err(error) => {
            print_parse_error_json(file, &[], &error);
            process::exit(1);
        }
    };
    let bytecode = BytecodeBuilder::new()
        .build_checked(&program)
        .unwrap_or_else(|error| {
            print_semantic_error_json(file, &[], &error);
            process::exit(1);
        });

    println!("{}", perf_diagnose_report(file, &program, &bytecode));
}

/// Benchmark Matter program: bytecode vs native
fn benchmark_program(file: &str, iterations: usize, json_output: bool) {
    use std::time::Instant;
    let iterations = iterations.max(1);

    // Read source
    let source = if file == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
        buffer
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        })
    };

    // Compile to bytecode
    let bytecode = match compile_source(&source, file) {
        Ok(bc) => bc,
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            process::exit(1);
        }
    };

    if !json_output {
        println!("=== Matter Benchmark ===");
        println!("File: {}", file);
        println!("Iterations: {}", iterations);
        println!();
    }

    // Benchmark bytecode execution
    if !json_output {
        println!("Running bytecode benchmark...");
    }
    let mut bytecode_times = Vec::new();

    for i in 0..iterations {
        let mut runtime = Runtime::new_silent(bytecode.clone());
        runtime.set_stdout_enabled(false);

        let start = Instant::now();
        if let Err(e) = runtime.run() {
            eprintln!("Runtime error in iteration {}: {}", i + 1, e);
            process::exit(1);
        }
        let duration = start.elapsed();
        bytecode_times.push(duration);
    }

    let bytecode_stats = benchmark_stats(&bytecode_times);

    if !json_output {
        println!("✓ Bytecode execution:");
        println!("  Average: {:?}", bytecode_stats.average);
        println!("  Median:  {:?}", bytecode_stats.median);
        println!("  P95:     {:?}", bytecode_stats.p95);
        println!("  Min:     {:?}", bytecode_stats.min);
        println!("  Max:     {:?}", bytecode_stats.max);
        println!();
    }

    #[allow(unused_mut)]
    let mut native_json = json!({
        "enabled": false,
        "status": "skipped",
        "reason": "LLVM not enabled"
    });

    // Benchmark native execution (if LLVM is available)
    #[cfg(feature = "llvm")]
    {
        if !json_output {
            println!("Running native benchmark...");
        }

        // Compile to native
        let temp_output = ".matter_bench_native";
        match matter_llvm::compile_to_native(&bytecode, temp_output) {
            Ok(_) => {
                let mut native_times = Vec::new();

                #[cfg(target_os = "windows")]
                let exe_path = format!("{}.exe", temp_output);
                #[cfg(not(target_os = "windows"))]
                let exe_path = format!("./{}", temp_output);

                for i in 0..iterations {
                    let start = Instant::now();
                    let output = std::process::Command::new(&exe_path)
                        .output()
                        .unwrap_or_else(|e| {
                            eprintln!("Error running native in iteration {}: {}", i + 1, e);
                            let _ = fs::remove_file(&exe_path);
                            process::exit(1);
                        });
                    let duration = start.elapsed();

                    if !output.status.success() {
                        eprintln!("Native execution failed in iteration {}", i + 1);
                        let _ = fs::remove_file(&exe_path);
                        process::exit(1);
                    }

                    native_times.push(duration);
                }

                // Clean up
                let _ = fs::remove_file(&exe_path);

                let native_stats = benchmark_stats(&native_times);

                if !json_output {
                    println!("✓ Native execution:");
                    println!("  Average: {:?}", native_stats.average);
                    println!("  Median:  {:?}", native_stats.median);
                    println!("  P95:     {:?}", native_stats.p95);
                    println!("  Min:     {:?}", native_stats.min);
                    println!("  Max:     {:?}", native_stats.max);
                    println!();
                }

                // Calculate speedup
                let speedup = bytecode_stats.average.as_nanos() as f64
                    / native_stats.average.as_nanos() as f64;

                native_json = json!({
                    "enabled": true,
                    "status": "ok",
                    "stats": benchmark_stats_json(&native_stats),
                    "speedup": speedup
                });

                if !json_output {
                    println!("=== Results ===");
                    println!("Speedup: {:.2}x faster", speedup);

                    if speedup > 10.0 {
                        println!("Excellent! Native is significantly faster.");
                    } else if speedup > 2.0 {
                        println!("Good! Native provides meaningful speedup.");
                    } else if speedup > 1.0 {
                        println!("Native is faster, but not dramatically.");
                    } else {
                        println!("Native is slower (possible overhead from compilation).");
                    }
                }
            }
            Err(e) => {
                native_json = json!({
                    "enabled": true,
                    "status": "skipped",
                    "reason": format!("Native compilation error: {}", e)
                });
                if !json_output {
                    eprintln!("Native compilation error: {}", e);
                    eprintln!("Skipping native benchmark.");
                }
            }
        }
    }

    #[cfg(not(feature = "llvm"))]
    {
        if !json_output {
            println!("Native benchmark skipped (LLVM not enabled)");
            println!("Rebuild with: cargo build --features llvm");
        }
    }

    if json_output {
        println!(
            "{}",
            json!({
                "file": file,
                "iterations": iterations,
                "bytecode": {
                    "status": "ok",
                    "stats": benchmark_stats_json(&bytecode_stats)
                },
                "native": native_json
            })
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_options_parse_iterations_and_json() {
        let args = vec![
            "--json".to_string(),
            "--iterations".to_string(),
            "25".to_string(),
        ];

        assert_eq!(parse_benchmark_options(&args), (25, true));
    }

    #[test]
    fn benchmark_stats_compute_distribution() {
        let stats = benchmark_stats(&[
            std::time::Duration::from_nanos(10),
            std::time::Duration::from_nanos(20),
            std::time::Duration::from_nanos(30),
            std::time::Duration::from_nanos(40),
        ]);

        assert_eq!(duration_ns(stats.average), 25);
        assert_eq!(duration_ns(stats.median), 30);
        assert_eq!(duration_ns(stats.p95), 40);
        assert_eq!(duration_ns(stats.min), 10);
        assert_eq!(duration_ns(stats.max), 40);
    }

    #[test]
    fn benchmark_gate_passes_within_budget() {
        let report = json!({
            "file": "benchmarks/fibonacci.matter",
            "iterations": 20,
            "bytecode": {
                "stats": {
                    "average_ns": 100,
                    "median_ns": 90,
                    "p95_ns": 120
                }
            }
        });
        let options = BenchmarkGateOptions {
            report_path: "report.json".to_string(),
            max_average_ns: Some(100),
            max_median_ns: Some(100),
            max_p95_ns: Some(130),
            ci_exit_codes: true,
        };

        let source = format!("shell banner\n{}", report);
        let (passed, payload) = evaluate_benchmark_gate(&source, &options);

        assert!(passed);
        assert_eq!(payload["status"], "pass");
    }

    #[test]
    fn benchmark_gate_fails_over_budget() {
        let report = json!({
            "file": "benchmarks/fibonacci.matter",
            "iterations": 20,
            "bytecode": {
                "stats": {
                    "average_ns": 100,
                    "median_ns": 150,
                    "p95_ns": 200
                }
            }
        });
        let options = BenchmarkGateOptions {
            report_path: "report.json".to_string(),
            max_average_ns: Some(100),
            max_median_ns: Some(100),
            max_p95_ns: Some(180),
            ci_exit_codes: true,
        };

        let (passed, payload) = evaluate_benchmark_gate(&report.to_string(), &options);

        assert!(!passed);
        assert_eq!(payload["status"], "fail");
    }

    #[test]
    fn emnr_gate_status_respects_configured_thresholds() {
        let policy = EmnrGatePolicy {
            risk_threshold: 0.20,
            confidence_threshold: 0.75,
            energy_threshold: 40.0,
            ..EmnrGatePolicy::default()
        };
        let stream = test_emnr_stream(0, 0, 0, 0, 0.25, 0.80, 80.0);
        assert_eq!(emnr_gate_status(&stream, &policy), "warn");

        let relaxed = EmnrGatePolicy {
            risk_threshold: 0.30,
            confidence_threshold: 0.70,
            energy_threshold: 30.0,
            ..EmnrGatePolicy::default()
        };
        assert_eq!(emnr_gate_status(&stream, &relaxed), "pass");
    }

    #[test]
    fn emnr_gate_status_fails_on_isolation_or_critical_energy() {
        let policy = EmnrGatePolicy::default();
        assert_eq!(
            emnr_gate_status(&test_emnr_stream(0, 1, 0, 0, 0.1, 0.9, 80.0), &policy),
            "fail"
        );
        assert_eq!(
            emnr_gate_status(&test_emnr_stream(0, 0, 0, 0, 0.1, 0.9, 5.0), &policy),
            "fail"
        );
    }

    #[test]
    fn emnr_gate_policy_json_overrides_defaults() {
        let mut policy = EmnrGatePolicy::default();
        apply_emnr_gate_policy_json(
            &mut policy,
            &json!({
                "warn_as_fail": true,
                "risk_threshold": 0.25,
                "confidence_threshold": 0.80,
                "energy_threshold": 50.0,
                "critical_energy": 5.0
            }),
        );

        assert!(policy.warn_as_fail);
        assert_eq!(policy.risk_threshold, 0.25);
        assert_eq!(policy.confidence_threshold, 0.80);
        assert_eq!(policy.energy_threshold, 50.0);
        assert_eq!(policy.critical_energy, 5.0);
    }

    #[test]
    fn emnr_train_args_parse_epochs_and_state_paths() {
        let args = vec![
            "examples/fibonacci.matter".to_string(),
            "--epochs".to_string(),
            "3".to_string(),
            "--state-in".to_string(),
            "in.json".to_string(),
            "--state-out".to_string(),
            "out.json".to_string(),
            "--out".to_string(),
            "report.json".to_string(),
        ];

        let options = parse_emnr_train_args(&args);

        assert_eq!(options.paths, vec!["examples/fibonacci.matter"]);
        assert_eq!(options.epochs, 3);
        assert_eq!(options.state_input.as_deref(), Some("in.json"));
        assert_eq!(options.state_output.as_deref(), Some("out.json"));
        assert_eq!(options.output.as_deref(), Some("report.json"));
    }

    #[test]
    fn emnr_promote_args_parse_required_states_and_thresholds() {
        let args = vec![
            "examples/fibonacci.matter".to_string(),
            "--baseline-state".to_string(),
            "base.json".to_string(),
            "--candidate-state".to_string(),
            "candidate.json".to_string(),
            "--max-risk-regression".to_string(),
            "0.03".to_string(),
            "--max-error-regression".to_string(),
            "0.04".to_string(),
            "--max-confidence-drop".to_string(),
            "0.05".to_string(),
            "--min-energy".to_string(),
            "30".to_string(),
            "--out".to_string(),
            "promote.json".to_string(),
        ];

        let options = parse_emnr_promote_args(&args);

        assert_eq!(options.paths, vec!["examples/fibonacci.matter"]);
        assert_eq!(options.baseline_state, "base.json");
        assert_eq!(options.candidate_state, "candidate.json");
        assert_eq!(options.policy.max_risk_regression, 0.03);
        assert_eq!(options.policy.max_error_regression, 0.04);
        assert_eq!(options.policy.max_confidence_drop, 0.05);
        assert_eq!(options.policy.min_energy, 30.0);
        assert_eq!(options.output.as_deref(), Some("promote.json"));
    }

    #[test]
    fn emnr_promote_status_rejects_regressions() {
        let options = EmnrPromoteOptions {
            paths: vec!["examples/fibonacci.matter".to_string()],
            baseline_state: "base.json".to_string(),
            candidate_state: "candidate.json".to_string(),
            output: None,
            policy: EmnrPromotePolicy {
                max_risk_regression: 0.02,
                max_error_regression: 0.02,
                max_confidence_drop: 0.02,
                min_energy: 20.0,
            },
        };
        let baseline = test_emnr_stream(0, 0, 0, 0, 0.10, 0.90, 80.0);
        let candidate = test_emnr_stream(0, 0, 0, 0, 0.13, 0.91, 79.0);

        assert_eq!(
            emnr_promote_status(&baseline, &candidate, &options, 0.03, 0.0, 0.01),
            "reject"
        );
        assert_eq!(
            emnr_promote_status(&baseline, &candidate, &options, 0.01, 0.0, 0.01),
            "promote"
        );
    }

    #[test]
    fn emnr_audit_report_summarizes_gate_train_and_promote() {
        let reports = vec![
            (
                "gate.json".to_string(),
                json!({
                    "ok": true,
                    "status": "pass",
                    "emnr_stream": {
                        "summary": {
                            "final_energy": 90.0,
                            "mean_confidence": 0.90,
                            "mean_risk": 0.10,
                            "mean_reward": 0.80,
                            "memory_records": 1
                        }
                    }
                }),
            ),
            (
                "train.json".to_string(),
                json!({
                    "ok": true,
                    "status": "trained",
                    "finished": { "energy": 80.0, "memory_records": 3 },
                    "learning_delta": {
                        "confidence": 0.01,
                        "risk": -0.01,
                        "prediction_error": -0.02
                    }
                }),
            ),
            (
                "promote.json".to_string(),
                json!({
                    "ok": true,
                    "status": "promote",
                    "metrics": {
                        "candidate": {
                            "final_energy": 70.0,
                            "mean_confidence": 0.91,
                            "mean_risk": 0.08,
                            "mean_prediction_error": 0.12,
                            "memory_records": 4
                        },
                        "delta": {
                            "confidence": 0.02,
                            "risk": -0.02,
                            "prediction_error": -0.03
                        }
                    }
                }),
            ),
        ];

        let audit = emnr_audit_report(&reports, false);

        assert_eq!(audit["status"], "pass");
        assert_eq!(audit["summary"]["pass"], 3);
        assert_eq!(audit["health_score"], 100.0);
        assert_eq!(audit["items"][0]["kind"], "gate");
        assert_eq!(audit["items"][2]["kind"], "promote");
    }

    #[test]
    fn emnr_audit_fail_on_warn_promotes_warn_to_failure() {
        let reports = vec![(
            "gate.json".to_string(),
            json!({
                "ok": true,
                "status": "warn",
                "emnr_stream": {
                    "summary": {
                        "final_energy": 22.0,
                        "mean_confidence": 0.60,
                        "mean_risk": 0.35,
                        "mean_reward": 0.20,
                        "memory_records": 2
                    }
                }
            }),
        )];

        let audit = emnr_audit_report(&reports, true);

        assert_eq!(audit["status"], "fail");
        assert_eq!(audit["exit_code"], 1);
        assert_eq!(audit["summary"]["warn"], 1);
    }

    #[test]
    fn emnr_pipeline_args_parse_epochs_output_and_warn_policy() {
        let args = vec![
            "examples/fibonacci.matter".to_string(),
            "--epochs".to_string(),
            "4".to_string(),
            "--out-dir".to_string(),
            ".build-artifacts/emnr-pipeline-test".to_string(),
            "--fail-on-warn".to_string(),
        ];

        let options = parse_emnr_pipeline_args(&args);

        assert_eq!(options.paths, vec!["examples/fibonacci.matter"]);
        assert_eq!(options.epochs, 4);
        assert_eq!(options.out_dir, ".build-artifacts/emnr-pipeline-test");
        assert!(options.fail_on_warn);
    }

    #[test]
    fn emnr_pipeline_policy_json_overrides_gate_and_promotion() {
        let mut gate = EmnrGatePolicy::default();
        let mut promote = EmnrPromotePolicy::default();
        apply_emnr_pipeline_policy_json(
            &mut gate,
            &mut promote,
            &json!({
                "gate": {
                    "warn_as_fail": true,
                    "risk_threshold": 0.30,
                    "confidence_threshold": 0.70,
                    "energy_threshold": 40.0,
                    "critical_energy": 15.0
                },
                "promotion": {
                    "max_risk_regression": 0.01,
                    "max_error_regression": 0.015,
                    "max_confidence_drop": 0.025,
                    "min_energy": 35.0
                }
            }),
        );

        assert!(gate.warn_as_fail);
        assert_eq!(gate.risk_threshold, 0.30);
        assert_eq!(gate.confidence_threshold, 0.70);
        assert_eq!(gate.energy_threshold, 40.0);
        assert_eq!(gate.critical_energy, 15.0);
        assert_eq!(promote.max_risk_regression, 0.01);
        assert_eq!(promote.max_error_regression, 0.015);
        assert_eq!(promote.max_confidence_drop, 0.025);
        assert_eq!(promote.min_energy, 35.0);
    }

    #[test]
    fn emnr_pipeline_manifest_exposes_stable_contract() {
        let options = EmnrPipelineOptions {
            paths: vec!["examples/fibonacci.matter".to_string()],
            epochs: 2,
            out_dir: ".build-artifacts/emnr-manifest-test".to_string(),
            fail_on_warn: false,
            gate_policy: EmnrGatePolicy::default(),
            promote_policy: EmnrPromotePolicy::default(),
            policy_path: None,
        };
        let artifacts = json!({
            "baseline_state": "baseline-state.json",
            "candidate_state": "candidate-state.json",
            "manifest": "manifest.json"
        });
        let policy = emnr_pipeline_policy_json(options.gate_policy, options.promote_policy);
        let manifest = emnr_pipeline_manifest_json(
            &options,
            &artifacts,
            &policy,
            &json!({ "status": "pass" }),
            &json!({ "status": "trained", "learning_delta": { "risk": -0.01 } }),
            &json!({ "status": "promote", "metrics": { "delta": { "confidence": 0.01 } } }),
            &json!({ "status": "pass", "ok": true, "health_score": 100.0 }),
        );

        assert_eq!(manifest["schema"], "matter.emnr.pipeline.manifest.v1");
        assert_eq!(manifest["command"], "emnr-pipeline-json");
        assert_eq!(manifest["summary"]["status"], "pass");
        assert_eq!(manifest["artifacts"]["manifest"], "manifest.json");
    }

    #[test]
    fn emnr_verify_manifest_report_accepts_valid_manifest() {
        let manifest = json!({
            "schema": "matter.emnr.pipeline.manifest.v1",
            "command": "emnr-pipeline-json",
            "inputs": ["examples/fibonacci.matter"],
            "artifacts": {
                "baseline_state": "Cargo.toml",
                "candidate_state": "Cargo.toml",
                "gate_report": "Cargo.toml",
                "train_report": "Cargo.toml",
                "promote_report": "Cargo.toml",
                "audit_report": "Cargo.toml",
                "manifest": "Cargo.toml"
            },
            "summary": {
                "status": "pass",
                "health_score": 100.0
            }
        });

        let report = emnr_verify_manifest_report("manifest.json", &manifest, true);

        assert_eq!(report["status"], "pass");
        assert_eq!(report["exit_code"], 0);
        assert_eq!(report["fail_count"], 0);
    }

    #[test]
    fn emnr_verify_manifest_report_rejects_non_pass_when_required() {
        let manifest = json!({
            "schema": "matter.emnr.pipeline.manifest.v1",
            "command": "emnr-pipeline-json",
            "inputs": ["examples/fibonacci.matter"],
            "artifacts": {
                "baseline_state": "Cargo.toml",
                "candidate_state": "Cargo.toml",
                "gate_report": "Cargo.toml",
                "train_report": "Cargo.toml",
                "promote_report": "Cargo.toml",
                "audit_report": "Cargo.toml",
                "manifest": "Cargo.toml"
            },
            "summary": {
                "status": "warn",
                "health_score": 80.0
            }
        });

        let report = emnr_verify_manifest_report("manifest.json", &manifest, true);

        assert_eq!(report["status"], "fail");
        assert_eq!(report["exit_code"], 1);
        assert_eq!(report["fail_count"], 1);
    }

    #[test]
    fn emnr_compare_manifests_reports_improvement() {
        let baseline = test_pipeline_manifest(90.0, "pass", 0.01, -0.01, -0.01);
        let candidate = test_pipeline_manifest(95.0, "pass", 0.02, -0.02, -0.02);

        let report = emnr_compare_manifests_report(
            "baseline.json",
            "candidate.json",
            &baseline,
            &candidate,
            true,
        );

        assert_eq!(report["status"], "pass");
        assert_eq!(report["verdict"], "improved");
        assert_eq!(report["deltas"]["health_score"], 5.0);
    }

    #[test]
    fn emnr_compare_manifests_fails_required_non_regression() {
        let baseline = test_pipeline_manifest(95.0, "pass", 0.02, -0.02, -0.02);
        let candidate = test_pipeline_manifest(90.0, "pass", 0.01, 0.01, 0.01);

        let report = emnr_compare_manifests_report(
            "baseline.json",
            "candidate.json",
            &baseline,
            &candidate,
            true,
        );

        assert_eq!(report["status"], "fail");
        assert_eq!(report["verdict"], "regressed");
        assert!(report["fail_count"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn emnr_compare_manifests_tolerates_float_noise() {
        let baseline = test_pipeline_manifest(100.0, "pass", 0.000_746, -0.000_007, -0.002_232);
        let candidate = test_pipeline_manifest(100.0, "pass", 0.000_744, -0.000_003, -0.002_231);

        let report = emnr_compare_manifests_report(
            "baseline.json",
            "candidate.json",
            &baseline,
            &candidate,
            true,
        );

        assert_eq!(report["status"], "pass");
        assert_eq!(report["verdict"], "equivalent");
        assert_eq!(report["fail_count"], 0);
    }

    #[test]
    fn emnr_ci_args_parse_pipeline_and_ci_flags() {
        let args = vec![
            "examples/fibonacci.matter".to_string(),
            "--epochs".to_string(),
            "3".to_string(),
            "--out-dir".to_string(),
            ".build-artifacts/emnr-ci-test".to_string(),
            "--baseline-manifest".to_string(),
            "baseline.json".to_string(),
            "--baseline-index".to_string(),
            "index.json".to_string(),
            "--require-non-regression".to_string(),
            "--require-pass".to_string(),
            "--out".to_string(),
            "ci-report.json".to_string(),
        ];

        let options = parse_emnr_ci_args(&args);

        assert_eq!(options.pipeline.paths, vec!["examples/fibonacci.matter"]);
        assert_eq!(options.pipeline.epochs, 3);
        assert_eq!(options.pipeline.out_dir, ".build-artifacts/emnr-ci-test");
        assert_eq!(options.baseline_manifest.as_deref(), Some("baseline.json"));
        assert_eq!(options.baseline_index.as_deref(), Some("index.json"));
        assert!(options.require_non_regression);
        assert!(options.require_pass);
        assert_eq!(options.output.as_deref(), Some("ci-report.json"));
    }

    #[test]
    fn emnr_ci_scorecard_approves_clean_ci_report() {
        let report = json!({
            "status": "pass",
            "exit_code": 0,
            "baseline_manifest": "baseline.json",
            "baseline_index": "index.json",
            "pipeline": {
                "status": "pass",
                "manifest": "manifest.json",
                "health_score": 100.0
            },
            "verify": {
                "status": "pass",
                "fail_count": 0,
                "warn_count": 0,
                "summary": {
                    "learning_delta": { "confidence": 0.01 },
                    "promotion_delta": { "risk": -0.01 }
                }
            },
            "compare": {
                "status": "pass",
                "verdict": "equivalent",
                "fail_count": 0,
                "deltas": { "health_score": 0.0 }
            }
        });

        let scorecard = emnr_ci_scorecard_report("ci-report.json", &report, true);

        assert_eq!(scorecard["status"], "pass");
        assert_eq!(scorecard["decision"], "approve");
        assert_eq!(scorecard["exit_code"], 0);
    }

    #[test]
    fn emnr_ci_scorecard_blocks_regressed_ci_report() {
        let report = json!({
            "status": "fail",
            "exit_code": 1,
            "pipeline": {
                "status": "pass",
                "manifest": "manifest.json",
                "health_score": 100.0
            },
            "verify": {
                "status": "pass",
                "fail_count": 0,
                "warn_count": 0,
                "summary": {
                    "learning_delta": {},
                    "promotion_delta": {}
                }
            },
            "compare": {
                "status": "fail",
                "verdict": "regressed",
                "fail_count": 1,
                "deltas": { "health_score": -1.0 }
            }
        });

        let scorecard = emnr_ci_scorecard_report("ci-report.json", &report, true);

        assert_eq!(scorecard["status"], "fail");
        assert_eq!(scorecard["decision"], "block");
        assert_eq!(scorecard["exit_code"], 1);
    }

    #[test]
    fn emnr_ci_history_reports_stable_approved_line() {
        let scorecards = vec![
            (
                "scorecard-1.json".to_string(),
                json!({
                    "schema": "matter.emnr.ci.scorecard.v1",
                    "status": "pass",
                    "decision": "approve",
                    "health_score": 99.0,
                    "compare": { "verdict": "equivalent" }
                }),
            ),
            (
                "scorecard-2.json".to_string(),
                json!({
                    "schema": "matter.emnr.ci.scorecard.v1",
                    "status": "pass",
                    "decision": "approve",
                    "health_score": 100.0,
                    "compare": { "verdict": "improved" }
                }),
            ),
        ];

        let history = emnr_ci_history_report(&scorecards, true);

        assert_eq!(history["status"], "pass");
        assert_eq!(history["stable"], true);
        assert_eq!(history["trend"], "improving");
        assert_eq!(history["summary"]["approve"], 2);
    }

    #[test]
    fn emnr_ci_history_blocks_unstable_line_when_required() {
        let scorecards = vec![
            (
                "scorecard-1.json".to_string(),
                json!({
                    "schema": "matter.emnr.ci.scorecard.v1",
                    "status": "pass",
                    "decision": "approve",
                    "health_score": 100.0,
                    "compare": { "verdict": "equivalent" }
                }),
            ),
            (
                "scorecard-2.json".to_string(),
                json!({
                    "schema": "matter.emnr.ci.scorecard.v1",
                    "status": "fail",
                    "decision": "block",
                    "health_score": 100.0,
                    "compare": { "verdict": "regressed" }
                }),
            ),
        ];

        let history = emnr_ci_history_report(&scorecards, true);

        assert_eq!(history["status"], "fail");
        assert_eq!(history["stable"], false);
        assert_eq!(history["trend"], "blocked");
        assert_eq!(history["exit_code"], 1);
    }

    #[test]
    fn emnr_ci_bundle_report_packages_scorecard_and_history() {
        let out_dir = env::temp_dir().join(format!(
            "emnr-ci-bundle-report-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let report = json!({
            "status": "pass",
            "exit_code": 0,
            "baseline_manifest": "baseline.json",
            "baseline_index": "index.json",
            "pipeline": {
                "status": "pass",
                "manifest": "manifest.json",
                "health_score": 100.0
            },
            "verify": {
                "status": "pass",
                "fail_count": 0,
                "warn_count": 0,
                "summary": {
                    "learning_delta": { "confidence": 0.01 },
                    "promotion_delta": { "risk": -0.01 }
                }
            },
            "compare": {
                "status": "pass",
                "verdict": "equivalent",
                "fail_count": 0,
                "deltas": { "health_score": 0.0 }
            }
        });

        let bundle = emnr_ci_bundle_report(
            "ci-report.json",
            &out_dir.display().to_string(),
            &report,
            true,
            true,
        );

        assert_eq!(bundle["status"], "pass");
        assert_eq!(bundle["decision"], "approve");
        assert_eq!(bundle["stable"], true);
        assert!(out_dir.join("ci-report.json").exists());
        assert!(out_dir.join("scorecard.json").exists());
        assert!(out_dir.join("history.json").exists());
        assert!(out_dir.join("bundle.json").exists());
        let _ = fs::remove_dir_all(out_dir);
    }

    #[test]
    fn emnr_verify_ci_bundle_report_accepts_valid_bundle() {
        let out_dir = env::temp_dir().join(format!(
            "emnr-verify-ci-bundle-report-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&out_dir).unwrap();
        let ci_report = out_dir.join("ci-report.json");
        let scorecard = out_dir.join("scorecard.json");
        let history = out_dir.join("history.json");
        let bundle_path = out_dir.join("bundle.json");
        fs::write(&ci_report, "{}").unwrap();
        fs::write(&scorecard, "{}").unwrap();
        fs::write(&history, "{}").unwrap();
        fs::write(&bundle_path, "{}").unwrap();
        let bundle = json!({
            "schema": "matter.emnr.ci.bundle.v1",
            "decision": "approve",
            "stable": true,
            "trend": "stable",
            "artifacts": {
                "ci_report": ci_report,
                "scorecard": scorecard,
                "history": history,
                "bundle": bundle_path
            },
            "scorecard": {
                "decision": "approve"
            },
            "history": {
                "stable": true,
                "trend": "stable"
            }
        });

        let report = emnr_verify_ci_bundle_report("bundle.json", &bundle, true);

        assert_eq!(report["status"], "pass");
        assert_eq!(report["fail_count"], 0);
        let _ = fs::remove_dir_all(out_dir);
    }

    #[test]
    fn emnr_verify_ci_bundle_report_rejects_unstable_bundle_when_required() {
        let bundle = json!({
            "schema": "matter.emnr.ci.bundle.v1",
            "decision": "review",
            "stable": false,
            "trend": "degrading",
            "artifacts": {
                "ci_report": "missing-ci-report.json",
                "scorecard": "missing-scorecard.json",
                "history": "missing-history.json",
                "bundle": "missing-bundle.json"
            },
            "scorecard": {
                "decision": "review"
            },
            "history": {
                "stable": false,
                "trend": "degrading"
            }
        });

        let report = emnr_verify_ci_bundle_report("bundle.json", &bundle, true);

        assert_eq!(report["status"], "fail");
        assert_ne!(report["exit_code"], 0);
        assert!(report["fail_count"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn emnr_ci_bundle_summary_markdown_includes_decision_and_artifacts() {
        let bundle = json!({
            "schema": "matter.emnr.ci.bundle.v1",
            "status": "pass",
            "decision": "approve",
            "stable": true,
            "trend": "stable",
            "artifacts": {
                "ci_report": "ci-report.json",
                "scorecard": "scorecard.json",
                "history": "history.json",
                "bundle": "bundle.json"
            },
            "scorecard": {
                "health_score": 100.0,
                "verify": {
                    "fail_count": 0,
                    "warn_count": 0
                },
                "compare": {
                    "verdict": "equivalent"
                }
            },
            "history": {
                "count": 1,
                "health": {
                    "delta": 0.0
                }
            }
        });

        let markdown = emnr_ci_bundle_summary_markdown("bundle.json", &bundle);

        assert!(markdown.contains("# EMNR CI Bundle"));
        assert!(markdown.contains("- decision: `approve`"));
        assert!(markdown.contains("- ci_report: `ci-report.json`"));
    }

    #[test]
    fn emnr_promote_ci_bundle_report_updates_release_index() {
        let out_dir = env::temp_dir().join(format!(
            "emnr-promote-ci-bundle-report-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&out_dir).unwrap();
        let ci_report = out_dir.join("ci-report.json");
        let scorecard = out_dir.join("scorecard.json");
        let history = out_dir.join("history.json");
        let bundle_file = out_dir.join("bundle.json");
        let index = out_dir.join("index.json");
        fs::write(&ci_report, "{}").unwrap();
        fs::write(&scorecard, "{}").unwrap();
        fs::write(&history, "{}").unwrap();
        fs::write(&bundle_file, "{}").unwrap();
        let bundle = json!({
            "schema": "matter.emnr.ci.bundle.v1",
            "status": "pass",
            "decision": "approve",
            "stable": true,
            "trend": "stable",
            "artifacts": {
                "ci_report": ci_report,
                "scorecard": scorecard,
                "history": history,
                "bundle": bundle_file
            },
            "scorecard": {
                "decision": "approve",
                "health_score": 100.0,
                "compare": {
                    "verdict": "equivalent"
                }
            },
            "history": {
                "stable": true,
                "trend": "stable"
            }
        });

        let report = emnr_promote_ci_bundle_report(
            "bundle.json",
            &index.display().to_string(),
            &bundle,
            true,
        );

        assert_eq!(report["status"], "promoted");
        assert_eq!(report["release_count"], 1);
        assert_eq!(report["current"]["decision"], "approve");
        assert!(index.exists());
        let _ = fs::remove_dir_all(out_dir);
    }

    #[test]
    fn emnr_promote_ci_bundle_report_rejects_invalid_bundle() {
        let bundle = json!({
            "schema": "matter.emnr.ci.bundle.v1",
            "decision": "review",
            "stable": false,
            "trend": "degrading",
            "artifacts": {
                "ci_report": "missing-ci-report.json",
                "scorecard": "missing-scorecard.json",
                "history": "missing-history.json",
                "bundle": "missing-bundle.json"
            },
            "scorecard": {
                "decision": "review"
            },
            "history": {
                "stable": false,
                "trend": "degrading"
            }
        });

        let report = emnr_promote_ci_bundle_report("bundle.json", "index.json", &bundle, true);

        assert_eq!(report["status"], "rejected");
        assert_ne!(report["exit_code"], 0);
    }

    #[test]
    fn emnr_ci_bundle_index_report_summarizes_releases() {
        let index = json!({
            "schema": "matter.emnr.ci.bundle.index.v1",
            "current": {
                "bundle": "bundle-2.json",
                "decision": "approve",
                "stable": true,
                "health_score": 100.0
            },
            "release_count": 2,
            "releases": [
                {
                    "sequence": 1,
                    "bundle": "bundle-1.json",
                    "decision": "review",
                    "stable": true,
                    "health_score": 90.0
                },
                {
                    "sequence": 2,
                    "bundle": "bundle-2.json",
                    "decision": "approve",
                    "stable": true,
                    "health_score": 100.0
                }
            ]
        });

        let report = emnr_ci_bundle_index_report("index.json", &index, 1, true);

        assert_eq!(report["status"], "pass");
        assert_eq!(report["current_approved"], true);
        assert_eq!(report["summary"]["approve"], 1);
        assert_eq!(report["summary"]["review"], 1);
        assert_eq!(report["mean_health_score"], 95.0);
        assert_eq!(report["latest"][0]["bundle"], "bundle-2.json");
    }

    #[test]
    fn emnr_ci_bundle_index_report_fails_when_current_is_not_approved() {
        let index = json!({
            "schema": "matter.emnr.ci.bundle.index.v1",
            "current": {
                "bundle": "bundle-1.json",
                "decision": "review",
                "stable": true,
                "health_score": 90.0
            },
            "release_count": 1,
            "releases": [
                {
                    "sequence": 1,
                    "bundle": "bundle-1.json",
                    "decision": "review",
                    "stable": true,
                    "health_score": 90.0
                }
            ]
        });

        let report = emnr_ci_bundle_index_report("index.json", &index, 1, true);

        assert_eq!(report["status"], "fail");
        assert_eq!(report["current_approved"], false);
        assert_eq!(report["exit_code"], 1);
    }

    #[test]
    fn emnr_export_ci_bundle_index_report_rejects_missing_current_bundle() {
        let index = json!({
            "schema": "matter.emnr.ci.bundle.index.v1",
            "current": {
                "bundle": "missing-bundle.json",
                "decision": "approve",
                "stable": true
            },
            "release_count": 1,
            "releases": []
        });

        let report = emnr_export_ci_bundle_index_report("index.json", "out", &index);

        assert_eq!(report["status"], "fail");
        assert_eq!(report["error"]["current_exists"], false);
    }

    #[test]
    fn emnr_verify_ci_bundle_package_report_rejects_missing_artifacts() {
        let package = json!({
            "schema": "matter.emnr.ci.bundle.package.v1",
            "artifacts": {
                "index": "missing-index.json",
                "bundle": "missing-bundle.json"
            }
        });

        let report = emnr_verify_ci_bundle_package_report("package.json", &package, true);

        assert_eq!(report["status"], "fail");
        assert!(report["fail_count"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn emnr_import_ci_bundle_package_report_rejects_invalid_package() {
        let package = json!({
            "schema": "matter.emnr.ci.bundle.package.v1",
            "artifacts": {
                "index": "missing-index.json",
                "bundle": "missing-bundle.json"
            }
        });

        let report = emnr_import_ci_bundle_package_report("package.json", "out", &package, true);

        assert_eq!(report["status"], "rejected");
        assert_ne!(report["exit_code"], 0);
    }

    #[test]
    fn emnr_init_write_json_skips_existing_without_force() {
        let dir = env::temp_dir().join(format!(
            "emnr-init-write-json-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("policy.json");
        fs::write(&path, "{}").unwrap();

        let report = emnr_init_write_json(&path, &json!({ "changed": true }), false);

        assert_eq!(report["status"], "exists");
        assert_eq!(report["written"], false);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn emnr_init_seed_json_uses_fallback_when_missing() {
        let fallback = json!({ "schema": "fallback" });

        let seeded = emnr_init_seed_json("definitely-missing-emnr-seed.json", fallback);

        assert_eq!(seeded["schema"], "fallback");
    }

    #[test]
    fn emnr_sync_copy_json_skips_existing_without_force() {
        let dir = env::temp_dir().join(format!(
            "emnr-sync-copy-skip-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.json");
        let destination = dir.join("destination.json");
        fs::write(&source, r#"{"schema":"source"}"#).unwrap();
        fs::write(&destination, r#"{"schema":"existing"}"#).unwrap();

        let report = emnr_sync_copy_json(&source, &destination, false);

        assert_eq!(report["status"], "exists");
        assert_eq!(report["written"], false);
        let stored = fs::read_to_string(&destination).unwrap();
        assert!(stored.contains("existing"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn emnr_sync_copy_json_overwrites_with_force() {
        let dir = env::temp_dir().join(format!(
            "emnr-sync-copy-force-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.json");
        let destination = dir.join("destination.json");
        fs::write(&source, r#"{"schema":"source"}"#).unwrap();
        fs::write(&destination, r#"{"schema":"existing"}"#).unwrap();

        let report = emnr_sync_copy_json(&source, &destination, true);

        assert_eq!(report["status"], "written");
        assert_eq!(report["written"], true);
        let stored: JsonValue =
            serde_json::from_str(&fs::read_to_string(&destination).unwrap()).unwrap();
        assert_eq!(stored["schema"], "source");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn emnr_sync_copy_json_reports_missing_source() {
        let dir = env::temp_dir().join(format!(
            "emnr-sync-copy-missing-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let source = dir.join("missing.json");
        let destination = dir.join("destination.json");

        let report = emnr_sync_copy_json(&source, &destination, true);

        assert_eq!(report["status"], "missing_source");
        assert_eq!(report["written"], false);
        assert!(!destination.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn emnr_release_gate_allows_approved_stable_bundle() {
        let policy = json!({
            "schema": "matter.emnr.policy.v1",
            "release": {
                "require_approved_bundle": true,
                "require_stable_history": true,
                "min_health_score": 95.0,
                "reject_trends": ["blocked", "degrading"]
            }
        });
        let index = json!({
            "schema": "matter.emnr.ci.bundle.index.v1",
            "current": {
                "decision": "approve",
                "stable": true,
                "trend": "stable",
                "health_score": 100.0
            },
            "releases": [
                {
                    "decision": "approve",
                    "stable": true,
                    "trend": "stable",
                    "health_score": 100.0
                }
            ]
        });

        let report = emnr_release_gate_report("policy.json", "index.json", &policy, &index, true);

        assert_eq!(report["status"], "pass");
        assert_eq!(report["gate"], "allow");
        assert_eq!(report["fail_count"], 0);
    }

    #[test]
    fn emnr_release_gate_blocks_degrading_bundle() {
        let policy = json!({
            "schema": "matter.emnr.policy.v1",
            "release": {
                "require_approved_bundle": true,
                "require_stable_history": true,
                "min_health_score": 95.0,
                "reject_trends": ["blocked", "degrading"]
            }
        });
        let index = json!({
            "schema": "matter.emnr.ci.bundle.index.v1",
            "current": {
                "decision": "approve",
                "stable": false,
                "trend": "degrading",
                "health_score": 80.0
            },
            "releases": [
                {
                    "decision": "approve",
                    "stable": false,
                    "trend": "degrading",
                    "health_score": 80.0
                }
            ]
        });

        let report = emnr_release_gate_report("policy.json", "index.json", &policy, &index, true);

        assert_eq!(report["status"], "fail");
        assert_eq!(report["gate"], "block");
        assert!(report["fail_count"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn emnr_agent_advice_approves_clean_gate() {
        let gate = json!({
            "gate": "allow",
            "checks": [
                { "name": "approved_bundle", "status": "pass" }
            ],
            "current": {
                "decision": "approve"
            }
        });

        let report = emnr_agent_advice_report(&gate, true);

        assert_eq!(report["status"], "pass");
        assert_eq!(report["advice"], "approve");
        assert_eq!(report["recommended_action"], "promote_or_release");
        assert_eq!(report["reason_codes"][0], "emnr_release_gate_clean");
    }

    #[test]
    fn emnr_agent_advice_blocks_failed_gate() {
        let gate = json!({
            "gate": "block",
            "checks": [
                { "name": "health_score", "status": "fail" }
            ],
            "current": {
                "decision": "approve"
            }
        });

        let report = emnr_agent_advice_report(&gate, true);

        assert_eq!(report["status"], "fail");
        assert_eq!(report["advice"], "block");
        assert_eq!(
            report["recommended_action"],
            "block_release_and_rebuild_emnr_bundle"
        );
        assert_eq!(report["reason_codes"][0], "health_score");
    }

    #[test]
    fn emnr_status_report_summarizes_release_state() {
        let policy = json!({
            "schema": "matter.emnr.policy.v1",
            "release": {
                "require_approved_bundle": true,
                "require_stable_history": true,
                "min_health_score": 95.0,
                "reject_trends": ["blocked", "degrading"]
            }
        });
        let index = json!({
            "schema": "matter.emnr.ci.bundle.index.v1",
            "current": {
                "bundle": "bundle.json",
                "decision": "approve",
                "stable": true,
                "trend": "stable",
                "health_score": 100.0
            },
            "release_count": 1,
            "releases": [
                {
                    "bundle": "bundle.json",
                    "decision": "approve",
                    "stable": true,
                    "trend": "stable",
                    "health_score": 100.0
                }
            ]
        });

        let report = emnr_status_report("policy.json", "index.json", &policy, &index, false);

        assert_eq!(report["status"], "pass");
        assert_eq!(report["decision"]["gate"], "allow");
        assert_eq!(report["decision"]["advice"], "approve");
    }

    #[test]
    fn emnr_status_markdown_includes_decision_and_artifacts() {
        let status = json!({
            "status": "pass",
            "decision": {
                "gate": "allow",
                "advice": "approve",
                "recommended_action": "promote_or_release"
            },
            "current": {
                "health_score": 100.0,
                "stable": true,
                "trend": "stable"
            },
            "health": {
                "doctor": {
                    "pass": 10,
                    "total": 10
                },
                "release_fail_count": 0,
                "release_warn_count": 0
            },
            "artifacts": {
                "policy": ".emnr/policy.json",
                "bundle_index": ".emnr/bundle-index.json",
                "bundle": ".emnr/current-bundle.json"
            }
        });

        let markdown = emnr_status_markdown(&status);

        assert!(markdown.contains("# EMNR Status"));
        assert!(markdown.contains("- gate: `allow`"));
        assert!(markdown.contains("- bundle: `.emnr/current-bundle.json`"));
    }

    #[test]
    fn emnr_doctor_file_check_reports_missing_path() {
        let check = emnr_doctor_file_check("missing", "definitely-missing-emnr-artifact.json");

        assert_eq!(check["status"], "missing");
        assert_eq!(check["ok"], false);
    }

    #[test]
    fn emnr_doctor_json_check_reports_valid_artifact() {
        let dir = env::temp_dir().join(format!(
            "emnr-doctor-json-check-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let artifact = dir.join("artifact.json");
        fs::write(&artifact, r#"{"ok":true}"#).unwrap();

        let check = emnr_doctor_json_check(
            "artifact",
            &artifact.display().to_string(),
            |_path, _value| json!({ "exit_code": 0, "status": "pass" }),
        );

        assert_eq!(check["status"], "pass");
        assert_eq!(check["ok"], true);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn emnr_baseline_promote_args_require_pass_by_default() {
        let args = vec![
            "manifest.json".to_string(),
            "--baseline-out".to_string(),
            "baseline.json".to_string(),
            "--index-out".to_string(),
            "index.json".to_string(),
            "--out".to_string(),
            "report.json".to_string(),
        ];

        let options = parse_emnr_baseline_promote_args(&args);

        assert_eq!(options.manifest, "manifest.json");
        assert_eq!(options.baseline_output, "baseline.json");
        assert!(options.require_pass);
        assert_eq!(options.index_output.as_deref(), Some("index.json"));
        assert_eq!(options.output.as_deref(), Some("report.json"));
    }

    #[test]
    fn emnr_baseline_index_records_promotion_summary() {
        let options = EmnrBaselinePromoteOptions {
            manifest: "manifest.json".to_string(),
            baseline_output: "baseline.json".to_string(),
            require_pass: true,
            index_output: Some("index.json".to_string()),
            output: None,
        };
        let manifest = test_pipeline_manifest(100.0, "pass", 0.02, -0.02, -0.02);

        let index = emnr_update_baseline_index("missing-index.json", &options, &manifest);

        assert_eq!(index["schema"], "matter.emnr.baseline.index.v1");
        assert_eq!(index["promotion_count"], 1);
        assert_eq!(index["current"]["manifest"], "manifest.json");
        assert_eq!(index["promotions"][0]["health_score"], 100.0);
    }

    #[test]
    fn emnr_baseline_index_report_returns_latest_promotions() {
        let index = json!({
            "schema": "matter.emnr.baseline.index.v1",
            "current": { "manifest": "m2.json", "status": "pass", "health_score": 90.0 },
            "promotions": [
                { "sequence": 1, "manifest": "m1.json", "health_score": 80.0 },
                { "sequence": 2, "manifest": "m2.json", "health_score": 90.0 }
            ]
        });

        let report = emnr_baseline_index_report("index.json", &index, 1);

        assert_eq!(report["status"], "pass");
        assert_eq!(report["promotion_count"], 2);
        assert_eq!(report["mean_health_score"], 85.0);
        assert_eq!(report["latest"][0]["manifest"], "m2.json");
    }

    #[test]
    fn emnr_export_baseline_report_rejects_missing_current_baseline() {
        let index = json!({
            "schema": "matter.emnr.baseline.index.v1",
            "current": {
                "baseline_output": "missing-baseline.json",
                "status": "pass",
                "health_score": 100.0
            },
            "promotion_count": 1,
            "promotions": []
        });

        let report = emnr_export_baseline_report("index.json", "out", &index);

        assert_eq!(report["status"], "fail");
        assert_eq!(report["error"]["baseline_exists"], false);
    }

    #[test]
    fn emnr_verify_baseline_package_report_rejects_missing_artifacts() {
        let package = json!({
            "schema": "matter.emnr.baseline.package.v1",
            "artifacts": {
                "index": "missing-index.json",
                "manifest": "missing-manifest.json"
            }
        });

        let report = emnr_verify_baseline_package_report("package.json", &package, true);

        assert_eq!(report["status"], "fail");
        assert!(report["fail_count"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn emnr_import_baseline_package_report_rejects_invalid_package() {
        let package = json!({
            "schema": "matter.emnr.baseline.package.v1",
            "artifacts": {
                "index": "missing-index.json",
                "manifest": "missing-manifest.json"
            }
        });

        let report = emnr_import_baseline_package_report("package.json", "out", &package, true);

        assert_eq!(report["status"], "rejected");
        assert_ne!(report["exit_code"], 0);
    }

    #[test]
    fn emnr_baseline_doctor_report_rejects_missing_directory_artifacts() {
        let report = emnr_baseline_doctor_report("missing-baseline-dir", true);

        assert_eq!(report["status"], "fail");
        assert_eq!(report["ci_ready"], false);
        assert!(report["fail_count"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn perf_diagnose_passes_small_linear_program() {
        let source = r#"
let x = 10
let y = 32
print x + y
"#;
        let mut parser = Parser::from_source(source);
        let program = parser.parse().unwrap();
        let bytecode = BytecodeBuilder::new().build_checked(&program).unwrap();
        let report = perf_diagnose_report("inline.matter", &program, &bytecode);

        assert_eq!(report["ok"], true);
        assert_eq!(report["status"], "pass");
        assert_eq!(report["pressure"], "low");
        assert_eq!(report["metrics"]["loops"], 0);
    }

    #[test]
    fn perf_diagnose_warns_on_recursive_loop_hot_path() {
        let source = r#"
fn fib(n) {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

let i = 0
while i < 3 {
    print fib(i)
    set i = i + 1
}
"#;
        let mut parser = Parser::from_source(source);
        let program = parser.parse().unwrap();
        let bytecode = BytecodeBuilder::new().build_checked(&program).unwrap();
        let report = perf_diagnose_report("hot.matter", &program, &bytecode);

        assert_eq!(report["status"], "warn");
        assert_eq!(report["metrics"]["loops"], 1);
        assert_eq!(report["checks"][1]["name"], "loop_pressure");
        assert_eq!(report["checks"][3]["severity"], "warn");
    }

    #[test]
    fn reflection_reports_ast_calls_and_bytecode_opcodes() {
        let source = r#"
fn fib(n) {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

print fib(4)
"#;
        let mut parser = Parser::from_source(source);
        let program = parser.parse().unwrap();
        let bytecode = BytecodeBuilder::new().build_checked(&program).unwrap();

        let ast_payload: JsonValue = serde_json::from_str(&ast_reflection_json(&program)).unwrap();
        let bytecode_payload: JsonValue =
            serde_json::from_str(&bytecode_reflection_json(&bytecode)).unwrap();

        assert_eq!(ast_payload["calls"]["fib"], 3);
        assert_eq!(ast_payload["statement_kinds"]["FunctionDef"], 1);
        assert_eq!(bytecode_payload["summary"]["functions"], 1);
        assert_eq!(bytecode_payload["opcode_histogram"]["CallNamed"], 3);
    }

    #[test]
    fn reflexive_guard_warns_on_direct_recursion() {
        let source = r#"
fn fib(n) {
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

print fib(4)
"#;
        let mut parser = Parser::from_source(source);
        let program = parser.parse().unwrap();
        let bytecode = BytecodeBuilder::new().build_checked(&program).unwrap();
        let report = reflexive_guard_report(&program, &bytecode, &ReflexiveGuardOptions::default());

        assert_eq!(report["status"], "warn");
        assert_eq!(report["metrics"]["direct_recursive_functions"][0], "fib");
    }

    #[test]
    fn reflexive_guard_fails_when_statement_budget_is_exceeded() {
        let source = r#"
print 1
print 2
"#;
        let mut parser = Parser::from_source(source);
        let program = parser.parse().unwrap();
        let bytecode = BytecodeBuilder::new().build_checked(&program).unwrap();
        let options = ReflexiveGuardOptions {
            max_statements: 1,
            max_functions: 50,
            allow_backends: false,
        };
        let report = reflexive_guard_report(&program, &bytecode, &options);

        assert_eq!(report["status"], "fail");
        assert_eq!(report["metrics"]["total_statements"], 2);
    }

    #[test]
    fn doctor_core_pipeline_reports_expected_output() {
        assert_eq!(doctor_core_pipeline_check().unwrap(), "42");
    }

    #[test]
    fn scaffold_project_creates_runnable_manifest_and_entry() {
        let dir = std::env::temp_dir().join(format!(
            "matter_init_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let options = InitOptions {
            dir: dir.clone(),
            name: Some("My App".to_string()),
            template: InitTemplate::Basic,
        };

        let result = scaffold_project(&options).unwrap();

        assert_eq!(result.name, "my-app");
        assert!(result.manifest_path.exists());
        assert!(result.entry_path.exists());

        let manifest_source = fs::read_to_string(&result.manifest_path).unwrap();
        let manifest = parse_package_manifest(&manifest_source).unwrap();
        assert_eq!(manifest.name, "my-app");
        assert_eq!(manifest.entry, "src/main.matter");

        let entry_source = fs::read_to_string(&result.entry_path).unwrap();
        let mut parser = Parser::from_source(&entry_source);
        let program = parser.parse().unwrap();
        let bytecode = BytecodeBuilder::new().build_checked(&program).unwrap();
        let mut runtime = Runtime::new_silent(bytecode);
        runtime.set_stdout_enabled(false);
        runtime.run().unwrap();
        assert_eq!(
            runtime.take_output(),
            init_template_run_output("my-app", InitTemplate::Basic)
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn scaffold_project_event_template_creates_event_handler() {
        let dir = std::env::temp_dir().join(format!(
            "matter_init_event_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let options = InitOptions {
            dir: dir.clone(),
            name: Some("Event App".to_string()),
            template: InitTemplate::Event,
        };

        let result = scaffold_project(&options).unwrap();
        let entry_source = fs::read_to_string(&result.entry_path).unwrap();
        assert!(entry_source.contains("on boot"));

        let mut parser = Parser::from_source(&entry_source);
        let program = parser.parse().unwrap();
        let bytecode = BytecodeBuilder::new().build_checked(&program).unwrap();
        assert!(bytecode.event_handlers.contains_key("boot"));

        let mut runtime = Runtime::new_silent(bytecode);
        runtime.set_stdout_enabled(false);
        runtime.run().unwrap();
        assert_eq!(
            runtime.take_output(),
            init_template_run_output("event-app", InitTemplate::Event)
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn scaffold_project_refuses_to_overwrite_manifest() {
        let dir = std::env::temp_dir().join(format!(
            "matter_init_existing_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("matter.toml"), "existing").unwrap();

        let options = InitOptions {
            dir: dir.clone(),
            name: Some("existing".to_string()),
            template: InitTemplate::Basic,
        };
        let error = scaffold_project(&options).expect_err("expected overwrite protection");
        assert!(error.contains("matter.toml"));
        assert!(error.contains("already exists"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn energy_fragment_is_empty_when_disabled() {
        assert_eq!(energy_json_fragment(false, 10.0, 20.0), "");
    }

    #[test]
    fn energy_fragment_has_stable_schema_when_enabled() {
        let fragment = energy_json_fragment(true, 12.34, 56.78);
        assert!(fragment.starts_with(",\"energy\":{"));
        assert!(fragment.contains("\"instruction_cost\":12.34"));
        assert!(fragment.contains("\"backend_cost\":56.78"));
        assert!(fragment.ends_with("}"));
    }

    #[test]
    fn run_json_payload_with_energy_has_expected_keys() {
        let payload = format!(
            "{{\"ok\":true,\"input\":\"{}\",\"output\":{}{}}}",
            json_escape("example.matter"),
            json_string_array(&["ok".to_string()]),
            energy_json_fragment(true, 1.5, 3.0)
        );
        assert!(payload.contains("\"ok\":true"));
        assert!(payload.contains("\"input\":\"example.matter\""));
        assert!(payload.contains("\"output\":[\"ok\"]"));
        assert!(payload.contains("\"energy\":{"));
        assert!(payload.contains("\"instruction_cost\":1.50"));
        assert!(payload.contains("\"backend_cost\":3.00"));
    }

    #[test]
    fn emit_json_payload_with_energy_has_expected_keys() {
        let payload = format!(
            "{{\"ok\":true,\"input\":\"{}\",\"event\":\"{}\",\"output\":{}{}}}",
            json_escape("example.matter"),
            json_escape("tick"),
            json_string_array(&["done".to_string()]),
            energy_json_fragment(true, 2.0, 4.0)
        );
        assert!(payload.contains("\"event\":\"tick\""));
        assert!(payload.contains("\"energy\":{"));
        assert!(payload.contains("\"instruction_cost\":2.00"));
        assert!(payload.contains("\"backend_cost\":4.00"));
    }

    #[test]
    fn bytecode_json_payload_without_energy_stays_compatible() {
        let payload = format!(
            "{{\"ok\":true,\"input\":\"{}\",\"output\":{}{}}}",
            json_escape("program.mbc"),
            json_string_array(&["42".to_string()]),
            energy_json_fragment(false, 1.0, 1.0)
        );
        assert!(payload.contains("\"input\":\"program.mbc\""));
        assert!(!payload.contains("\"energy\":{"));
    }

    #[test]
    fn visual_step_payload_with_energy_has_expected_schema() {
        let payload = format!(
            "{{\"ok\":true,\"input\":\"{}\",\"events\":\"{}\",\"deltaMs\":{},\"result\":{},\"output\":{}{}}}",
            json_escape("demo.matter"),
            json_escape("events.json"),
            16,
            "null",
            json_string_array(&["frame".to_string()]),
            energy_json_fragment(true, 3.25, 7.5)
        );
        assert!(payload.contains("\"events\":\"events.json\""));
        assert!(payload.contains("\"deltaMs\":16"));
        assert!(payload.contains("\"energy\":{"));
        assert!(payload.contains("\"instruction_cost\":3.25"));
        assert!(payload.contains("\"backend_cost\":7.50"));
    }

    #[test]
    fn project_run_build_payload_with_energy_has_expected_schema() {
        let payload = format!(
            "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"output_file\":\"{}\",\"lock_fingerprint\":\"{}\",\"bytecode_fingerprint\":\"{}\",\"bytecode_bytes\":{},\"files_count\":{},\"imports_count\":{},\"dependencies_count\":{},\"output\":{},\"summary\":{}{} }}",
            json_escape("matter-app"),
            json_escape("matter.toml"),
            json_escape("main.matter"),
            json_escape("out.mbc"),
            json_escape("lock123"),
            json_escape("bc123"),
            512,
            3,
            2,
            1,
            json_string_array(&["ok".to_string()]),
            "{}",
            energy_json_fragment(true, 12.0, 30.0)
        );
        assert!(payload.contains("\"package\":\"matter-app\""));
        assert!(payload.contains("\"output_file\":\"out.mbc\""));
        assert!(payload.contains("\"bytecode_bytes\":512"));
        assert!(payload.contains("\"energy\":{"));
        assert!(payload.contains("\"instruction_cost\":12.00"));
        assert!(payload.contains("\"backend_cost\":30.00"));
    }

    #[test]
    fn project_visual_payload_with_energy_has_expected_schema() {
        let payload = format!(
            "{{\"ok\":true,\"package\":\"{}\",\"manifest\":\"{}\",\"input\":\"{}\",\"events\":\"{}\",\"frames\":{},\"deltaMs\":{},\"result\":{},\"output\":{}{}}}",
            json_escape("matter-app"),
            json_escape("matter.toml"),
            json_escape("main.matter"),
            json_escape("events.json"),
            10,
            16,
            "null",
            json_string_array(&["frame".to_string()]),
            energy_json_fragment(true, 9.0, 15.0)
        );
        assert!(payload.contains("\"events\":\"events.json\""));
        assert!(payload.contains("\"frames\":10"));
        assert!(payload.contains("\"deltaMs\":16"));
        assert!(payload.contains("\"energy\":{"));
        assert!(payload.contains("\"instruction_cost\":9.00"));
        assert!(payload.contains("\"backend_cost\":15.00"));
    }

    #[test]
    fn native_studio_renderer_contains_surface_regions_and_output() {
        let model = NativeStudioModel {
            input: "demo.matter".to_string(),
            output: vec!["declared".to_string()],
            status: vec!["visual model ready".to_string()],
            surface_name: "matter_studio".to_string(),
            surface_width: 1280,
            surface_height: 720,
            regions: vec![NativeStudioRegion {
                name: "button_run".to_string(),
                x: 940,
                y: 520,
                w: 76,
                h: 34,
                text: "Run".to_string(),
                semantic: "primary_action".to_string(),
                event: "run_source".to_string(),
                state: "active".to_string(),
            }],
            instruction_cost: 1.0,
            backend_cost: 2.0,
        };

        let frame = render_native_studio(&model, false);

        assert!(frame.contains("Matter Studio Native"));
        assert!(frame.contains("matter_studio 1280x720"));
        assert!(frame.contains("Run"));
        assert!(frame.contains("primary_action"));
        assert!(frame.contains("declared"));
    }

    #[test]
    fn native_studio_tap_dispatches_region_action() {
        let source = r#"
visual.surface("matter_studio", 1280, 720)
visual.region("button_run", 940, 520, 76, 34)
visual.set("button_run", "text", "Run")
visual.set("button_run", "semantic", "primary_action")
visual.set("button_run", "event", "run_source")
print "declared"
"#;
        let model = build_native_studio_model(source, "tap-test.matter").unwrap();

        let status = native_studio_tap_status(source, &model, "Run");

        assert!(status
            .iter()
            .any(|line| line.contains("tap Run -> run_source")));
        assert!(status.iter().any(|line| line.contains("run ok")));
        assert!(status.iter().any(|line| line.contains("out: declared")));
    }

    #[test]
    fn native_studio_model_json_contains_regions_contract() {
        let model = NativeStudioModel {
            input: "demo.matter".to_string(),
            output: vec![],
            status: vec!["visual model ready".to_string()],
            surface_name: "matter_studio".to_string(),
            surface_width: 1280,
            surface_height: 720,
            regions: vec![NativeStudioRegion {
                name: "button_guard".to_string(),
                x: 1,
                y: 2,
                w: 3,
                h: 4,
                text: "Guard".to_string(),
                semantic: "".to_string(),
                event: "guard_source".to_string(),
                state: "".to_string(),
            }],
            instruction_cost: 1.0,
            backend_cost: 2.0,
        };

        let payload: JsonValue = serde_json::from_str(&native_studio_model_json(&model)).unwrap();

        assert_eq!(payload["ok"], true);
        assert_eq!(payload["surface"]["name"], "matter_studio");
        assert_eq!(payload["regions"][0]["text"], "Guard");
        assert_eq!(payload["regions"][0]["event"], "guard_source");
    }

    #[test]
    fn sentinel_pvmbc_encoder_emits_pvm2_manifest_and_opcodes() {
        let model = NativeStudioModel {
            input: "demo.matter".to_string(),
            output: vec![],
            status: vec![],
            surface_name: "matter_studio".to_string(),
            surface_width: 1280,
            surface_height: 720,
            regions: vec![NativeStudioRegion {
                name: "button_run".to_string(),
                x: 940,
                y: 520,
                w: 76,
                h: 34,
                text: "Run".to_string(),
                semantic: "primary_action".to_string(),
                event: "run_source".to_string(),
                state: "active".to_string(),
            }],
            instruction_cost: 0.0,
            backend_cost: 0.0,
        };

        let bytes = encode_sentinel_pvmbc(&model, "matter-studio");

        assert_eq!(&bytes[0..4], b"PVM2");
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 2);
        assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), 13);
        assert_eq!(
            u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            1280
        );
        assert_eq!(
            u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
            720
        );
        assert_eq!(
            u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]),
            5
        );
        assert_eq!(&bytes[36..49], b"matter-studio");
        assert_eq!(bytes[49], 0);
        assert_eq!(bytes[54], 1);
        assert!(bytes.contains(&3));
        assert!(bytes.contains(&2));
        assert_eq!(*bytes.last().unwrap(), 4);
    }

    #[test]
    fn sentinel_pvmbc_inspector_reports_generated_pvm2_package() {
        let model = NativeStudioModel {
            input: "demo.matter".to_string(),
            output: vec![],
            status: vec![],
            surface_name: "matter_studio".to_string(),
            surface_width: 640,
            surface_height: 480,
            regions: vec![NativeStudioRegion {
                name: "button_guard".to_string(),
                x: 10,
                y: 20,
                w: 30,
                h: 40,
                text: "Guard".to_string(),
                semantic: "".to_string(),
                event: "guard_source".to_string(),
                state: "".to_string(),
            }],
            instruction_cost: 0.0,
            backend_cost: 0.0,
        };
        let bytes = encode_sentinel_pvmbc(&model, "guard-app");

        let report = matter_sentinel_abi::inspect_pvmbc(&bytes).unwrap();

        assert_eq!(report.format, matter_sentinel_abi::PvmFormat::Pvm2);
        assert_eq!(report.format_version, 2);
        assert_eq!(report.name, b"guard-app");
        assert_eq!(report.width, 640);
        assert_eq!(report.height, 480);
        assert_eq!(report.declared_opcodes, report.decoded_opcodes);
        assert_eq!(report.frame_count, 1);
        assert_eq!(report.opcode_counts[PvmOpcodeTag::Clear as usize], 1);
        assert_eq!(report.opcode_counts[PvmOpcodeTag::FillRect as usize], 1);
        assert_eq!(report.opcode_counts[PvmOpcodeTag::SetBehavior as usize], 1);
        assert_eq!(report.opcode_counts[PvmOpcodeTag::Pulse as usize], 1);
        assert_eq!(report.opcode_counts[PvmOpcodeTag::Frame as usize], 1);
    }

    #[test]
    fn sentinel_rust_byte_array_formats_kernel_constant() {
        let output = sentinel_rust_byte_array("matter-studio pvmbc", &[0x50, 0x56, 0x4D, 0x32]);

        assert!(output.starts_with("pub const MATTER_STUDIO_PVMBC: &[u8] = &["));
        assert!(output.contains("0x50, 0x56, 0x4D, 0x32,"));
        assert!(output.ends_with("];\n"));
    }

    #[test]
    fn serialize_bytecode_or_exit_emits_mbc1_bytes() {
        let bytecode = Bytecode::new();
        let bytes = serialize_bytecode_or_exit(&bytecode, "test");
        let output = sentinel_rust_byte_array("matter boot mbc1", &bytes);

        assert_eq!(&bytes[0..4], b"MBC1");
        assert!(output.starts_with("pub const MATTER_BOOT_MBC1: &[u8] = &["));
        assert!(output.contains("0x4D, 0x42, 0x43, 0x31,"));
    }

    #[test]
    fn sentinel_mbc1_kernel_check_payload_runs_integer_subset() {
        let source = r#"
fn fact(n) {
    if n <= 1 {
        return 1
    }
    return n * fact(n - 1)
}

let result = fact(5)
sentinel.telemetry(result)
print result
"#;

        let (ok, payload) = sentinel_mbc1_kernel_check_payload(source, "kernel-test.matter", 256);

        assert!(ok, "payload: {}", payload);
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["format"], "MBC1");
        assert_eq!(payload["run"]["prints"][0]["value"], 120);
        assert_eq!(payload["run"]["syscalls"][0]["backend"], "sentinel");
        assert_eq!(payload["run"]["syscalls"][0]["method"], "telemetry");
        assert_eq!(payload["run"]["syscalls"][0]["args"][0]["value"], 120);
    }

    #[test]
    fn sentinel_mbc1_kernel_check_payload_reports_unsupported_subset() {
        let source = r#"print "kernel strings are not supported""#;

        let (ok, payload) = sentinel_mbc1_kernel_check_payload(source, "kernel-test.matter", 64);

        assert!(!ok);
        assert_eq!(payload["ok"], false);
        assert_eq!(payload["stage"], "run");
        assert_eq!(payload["error"]["kind"], "TypeError");
    }

    #[test]
    fn sanitize_rust_const_name_produces_valid_identifier() {
        assert_eq!(
            sanitize_rust_const_name("25 matter-studio pvmbc"),
            "_25_MATTER_STUDIO_PVMBC"
        );
        assert_eq!(sanitize_rust_const_name(""), "MATTER_PVMBC");
    }

    #[test]
    fn json_to_backend_value_converts_object_and_list() {
        let parsed = serde_json::json!({
            "from_name": "planner",
            "facts": ["a", "b"],
            "priority": 3,
            "active": true
        });
        let value = json_to_backend_value(&parsed).expect("conversion should succeed");
        let Value::Map(entries) = value else {
            panic!("expected map");
        };
        assert_eq!(
            entries.get("from_name"),
            Some(&Value::new_string("planner".to_string()))
        );
        assert_eq!(entries.get("priority"), Some(&Value::Int(3)));
        assert_eq!(entries.get("active"), Some(&Value::Bool(true)));
        assert!(matches!(entries.get("facts"), Some(Value::List(_))));
    }

    #[test]
    fn normalize_strategy_accepts_aliases() {
        assert_eq!(normalize_strategy("auto"), Some("auto"));
        assert_eq!(normalize_strategy("latest"), Some("prefer_latest"));
        assert_eq!(normalize_strategy("blocked"), Some("prefer_blocked"));
        assert_eq!(normalize_strategy("terminal"), Some("prefer_terminal"));
        assert_eq!(normalize_strategy("prefer_latest"), Some("prefer_latest"));
        assert_eq!(normalize_strategy("nope"), None);
    }

    #[test]
    fn normalize_energy_mode_accepts_known_values() {
        assert!(matches!(
            normalize_energy_mode("eco"),
            Some(PipelineEnergyMode::Eco)
        ));
        assert!(matches!(
            normalize_energy_mode("balanced"),
            Some(PipelineEnergyMode::Balanced)
        ));
        assert!(matches!(
            normalize_energy_mode("performance"),
            Some(PipelineEnergyMode::Performance)
        ));
        assert!(normalize_energy_mode("invalid").is_none());
    }

    #[test]
    fn recommended_energy_mode_tracks_ci_decision() {
        assert!(matches!(
            recommended_energy_mode_for_ci_decision("pass", PipelineEnergyMode::Balanced),
            PipelineEnergyMode::Balanced
        ));
        assert!(matches!(
            recommended_energy_mode_for_ci_decision("warn", PipelineEnergyMode::Balanced),
            PipelineEnergyMode::Adaptive
        ));
        assert!(matches!(
            recommended_energy_mode_for_ci_decision("fail", PipelineEnergyMode::Performance),
            PipelineEnergyMode::Critical
        ));
    }

    #[test]
    fn confidence_profile_thresholds_known_profiles() {
        assert_eq!(confidence_profile_thresholds("strict"), Some((8.0, 16.0)));
        assert_eq!(confidence_profile_thresholds("balanced"), Some((5.0, 12.0)));
        assert_eq!(confidence_profile_thresholds("relaxed"), Some((3.0, 8.0)));
        assert_eq!(confidence_profile_thresholds("x"), None);
    }

    #[test]
    fn tool_ci_catalog_json_has_stable_reason_codes() {
        let payload = tool_ci_catalog_json_string();
        assert!(payload.contains("\"catalog_hash\":\"fnv1a64:"));
        assert!(payload.contains("\"catalog_hash_mismatch\":20"));
        assert!(payload.contains("\"matched_fail_status\":140"));
        assert!(payload.contains("\"strict_degraded\":150"));
        assert!(payload.contains("\"unknown\":999"));
        assert!(payload.contains("\"reason_metadata\":{"));
        let doc: JsonValue = serde_json::from_str(&payload).expect("catalog payload must be json");
        let metadata = &doc["reason_metadata"]["matched_fail_status"];
        assert_eq!(metadata["deprecation"], "");
        assert_eq!(metadata["replacement_reason"], "");
        assert_eq!(metadata["since_version"], "1");
        assert_eq!(metadata["last_updated"], "2026-05-11");
    }

    #[test]
    fn ci_reason_code_map_is_stable() {
        assert_eq!(ci_reason_code("healthy"), 0);
        assert_eq!(ci_reason_code("low_confidence"), 10);
        assert_eq!(ci_reason_code("mkdir_failed"), 100);
        assert_eq!(ci_reason_code("write_frames_failed"), 110);
        assert_eq!(ci_reason_code("invoke_a_failed"), 120);
        assert_eq!(ci_reason_code("invoke_b_failed"), 121);
        assert_eq!(ci_reason_code("extract_wire_failed"), 130);
        assert_eq!(ci_reason_code("matched_fail_status"), 140);
        assert_eq!(ci_reason_code("strict_degraded"), 150);
        assert_eq!(ci_reason_code("missing_reason"), 999);
    }

    #[test]
    fn tool_ci_verify_reports_expected_match() {
        let (expected_ok, match_ok) = tool_ci_verify("strict_degraded", 150);
        assert_eq!(expected_ok, 150);
        assert!(match_ok);

        let (expected_bad, match_bad) = tool_ci_verify("strict_degraded", 140);
        assert_eq!(expected_bad, 150);
        assert!(!match_bad);
    }

    #[test]
    fn tool_ci_contract_json_exposes_catalog_and_compatibility() {
        let payload = tool_ci_contract_json_string();
        assert!(payload.contains("\"contract\":\"ci_reason_contract\""));
        assert!(payload.contains("\"catalog_hash\":\"fnv1a64:"));
        assert!(payload.contains("\"compatibility\":{"));
        assert!(payload.contains("\"changed_reason_code\":\"breaking\""));
    }

    #[test]
    fn pipeline_contract_validator_accepts_valid_payload() {
        let doc = json!({
            "ok": true,
            "contractVersion": "1",
            "catalogHash": "fnv1a64:abc",
            "ciDecision": "pass",
            "ciDecisionReason": "healthy",
            "ciDecisionCode": 0
        });
        assert!(validate_pipeline_contract_doc(&doc).is_ok());
    }

    #[test]
    fn pipeline_contract_validator_rejects_missing_fields() {
        let doc = json!({
            "ok": true,
            "contractVersion": "1"
        });
        let err = validate_pipeline_contract_doc(&doc).unwrap_err();
        assert!(err.contains("missing required field"));
    }

    #[test]
    fn pipeline_contract_normalizer_fills_required_fields() {
        let legacy = json!({
            "ok": true,
            "ci_decision": "warn",
            "ci_decision_reason": "low_confidence",
            "ci_decision_code": 10
        });
        let normalized = normalize_pipeline_contract_doc(&legacy);
        let obj = normalized.as_object().expect("normalized object");
        assert!(obj.contains_key("contractVersion"));
        assert!(obj.contains_key("catalogHash"));
        assert_eq!(obj.get("ciDecision").and_then(|v| v.as_str()), Some("warn"));
        assert_eq!(
            obj.get("ciDecisionReason").and_then(|v| v.as_str()),
            Some("low_confidence")
        );
        assert_eq!(obj.get("ciDecisionCode").and_then(|v| v.as_i64()), Some(10));
    }

    #[test]
    fn pipeline_contract_examples_have_success_and_failure_shapes() {
        let payload = tool_pipeline_contract_example_json_string();
        let parsed = serde_json::from_str::<JsonValue>(&payload).expect("valid example json");
        let examples = parsed
            .get("examples")
            .and_then(|v| v.as_object())
            .expect("examples object");
        let success = examples
            .get("success")
            .and_then(|v| v.as_object())
            .expect("success example");
        assert_eq!(success.get("ok").and_then(|v| v.as_bool()), Some(true));
        assert!(success.contains_key("ciDecisionCode"));

        let failure = examples
            .get("failure")
            .and_then(|v| v.as_object())
            .expect("failure example");
        assert_eq!(failure.get("ok").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            failure.get("ciDecisionReason").and_then(|v| v.as_str()),
            Some("strict_degraded")
        );
    }

    #[test]
    fn pipeline_contract_selftest_reports_summary() {
        let doc = tool_pipeline_contract_selftest_doc();
        let obj = doc.as_object().expect("selftest object");
        assert!(obj.contains_key("ok"));
        assert!(obj.contains_key("checks"));
        assert!(obj.contains_key("summary"));
        let summary = obj
            .get("summary")
            .and_then(|v| v.as_object())
            .expect("summary object");
        assert!(summary.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
    }

    #[test]
    fn derive_ci_gate_maps_pass_warn_fail() {
        assert_eq!(derive_ci_gate("pass", false), "pass");
        assert_eq!(derive_ci_gate("PASS", false), "pass");
        assert_eq!(derive_ci_gate("warn", false), "warn");
        assert_eq!(derive_ci_gate("fail", false), "fail");
        assert_eq!(derive_ci_gate("unknown", false), "fail");
    }

    #[test]
    fn derive_ci_gate_warn_can_be_promoted_to_fail() {
        assert_eq!(derive_ci_gate("warn", true), "fail");
        assert_eq!(derive_ci_gate("WARN", true), "fail");
        assert_eq!(derive_ci_gate("pass", true), "pass");
    }

    #[test]
    fn contract_diff_classifies_catalog_hash_change_as_non_breaking() {
        let baseline = json!({
            "ok": true,
            "contractVersion": "1",
            "catalogHash": "fnv1a64:a",
            "ciDecision": "pass",
            "ciDecisionReason": "healthy",
            "ciDecisionCode": 0
        });
        let candidate = json!({
            "ok": true,
            "contractVersion": "1",
            "catalogHash": "fnv1a64:b",
            "ciDecision": "pass",
            "ciDecisionReason": "healthy",
            "ciDecisionCode": 0
        });

        let (compat, breaking, non_breaking) =
            classify_pipeline_contract_compatibility(&baseline, &candidate);
        assert_eq!(compat, "compatible");
        assert!(breaking.is_empty());
        assert!(!non_breaking.is_empty());
    }

    #[test]
    fn contract_diff_classifies_code_change_as_breaking() {
        let baseline = json!({
            "ok": true,
            "contractVersion": "1",
            "catalogHash": "fnv1a64:a",
            "ciDecision": "pass",
            "ciDecisionReason": "healthy",
            "ciDecisionCode": 0
        });
        let candidate = json!({
            "ok": true,
            "contractVersion": "1",
            "catalogHash": "fnv1a64:a",
            "ciDecision": "pass",
            "ciDecisionReason": "healthy",
            "ciDecisionCode": 150
        });

        let (compat, breaking, non_breaking) =
            classify_pipeline_contract_compatibility(&baseline, &candidate);
        assert_eq!(compat, "breaking");
        assert!(!breaking.is_empty());
        assert!(non_breaking.is_empty());
    }

    #[test]
    fn upgrade_advice_is_present_for_breaking_contracts() {
        let breaking = vec!["ciDecisionCode changed: 0 -> 150".to_string()];
        let non_breaking: Vec<String> = Vec::new();
        let advice = pipeline_contract_upgrade_advice("breaking", &breaking, &non_breaking);
        assert!(!advice.is_empty());
        assert!(advice
            .iter()
            .any(|line| line.contains("ciDecisionCode mapping")));
    }

    #[test]
    fn upgrade_advice_for_compatible_contracts_is_progressive() {
        let breaking: Vec<String> = Vec::new();
        let non_breaking = vec!["catalogHash changed: 'a' -> 'b'".to_string()];
        let advice = pipeline_contract_upgrade_advice("compatible", &breaking, &non_breaking);
        assert!(!advice.is_empty());
        assert!(advice
            .iter()
            .any(|line| line.contains("Safe to rollout progressively")));
    }

    #[test]
    fn bundle_compatibility_derivation_matches_diff_logic() {
        let baseline = json!({
            "ok": true,
            "contractVersion": "1",
            "catalogHash": "fnv1a64:a",
            "ciDecision": "pass",
            "ciDecisionReason": "healthy",
            "ciDecisionCode": 0
        });
        let candidate = json!({
            "ok": true,
            "contractVersion": "1",
            "catalogHash": "fnv1a64:b",
            "ciDecision": "pass",
            "ciDecisionReason": "healthy",
            "ciDecisionCode": 0
        });
        let (compat, breaking, non_breaking) =
            classify_pipeline_contract_compatibility(&baseline, &candidate);
        let gate = if compat == "compatible" {
            "pass"
        } else {
            "fail"
        };
        let rollout = if compat == "breaking" {
            "controlled_migration"
        } else {
            "progressive_rollout"
        };
        let advice = pipeline_contract_upgrade_advice(&compat, &breaking, &non_breaking);
        assert_eq!(compat, "compatible");
        assert_eq!(gate, "pass");
        assert_eq!(rollout, "progressive_rollout");
        assert!(!advice.is_empty());
    }

    #[test]
    fn bundle_example_payload_has_usage_and_bundle_sections() {
        let payload = tool_pipeline_contract_bundle_example_json_string();
        assert!(payload.contains("\"usage\""));
        assert!(payload.contains("\"baseline\""));
        assert!(payload.contains("\"candidate\""));
        assert!(payload.contains("\"bundle\""));
        assert!(payload.contains("tool-pipeline-contract-bundle-json"));
    }

    #[test]
    fn scoring_preset_factory_returns_known_profiles() {
        assert!(PipelineScoring::from_preset("balanced").is_some());
        assert!(PipelineScoring::from_preset("conservative").is_some());
        assert!(PipelineScoring::from_preset("aggressive").is_some());
        assert!(PipelineScoring::from_preset("custom").is_none());
    }

    #[test]
    fn conservative_preset_penalizes_blocked_more_than_balanced() {
        let balanced = PipelineScoring::balanced();
        let conservative = PipelineScoring::conservative();
        assert!(conservative.penalty_blocked > balanced.penalty_blocked);
        assert!(conservative.status_degraded < balanced.status_degraded);
    }

    #[test]
    fn pipeline_demo_dry_run_writes_prefixed_artifacts() {
        let test_dir = env::temp_dir().join(format!(
            "matter_pipeline_demo_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&test_dir);
        let out_dir = test_dir.to_string_lossy().to_string();

        tool_pipeline_demo_json(
            &out_dir,
            false,
            "prefer_blocked",
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            "both",
            None,
            false,
            true,
            "batchx",
            &PipelineScoring::balanced(),
            Some(PipelineEnergyMode::Balanced),
            5.0,
            12.0,
            "balanced",
            None,
            None,
            None,
            3,
        );

        assert!(test_dir.join("batchx_frame_a.json").exists());
        assert!(test_dir.join("batchx_frame_b.json").exists());
        assert!(test_dir.join("batchx_report.json").exists());
        assert!(test_dir.join("batchx_summary.md").exists());
        assert!(test_dir.join("batchx_summary.json").exists());

        let report_text =
            fs::read_to_string(test_dir.join("batchx_report.json")).unwrap_or_default();
        assert!(report_text.contains("\"dry_run\":true"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn pipeline_demo_dry_run_without_prefix_uses_default_names() {
        let test_dir = env::temp_dir().join(format!(
            "matter_pipeline_demo_no_prefix_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&test_dir);
        let out_dir = test_dir.to_string_lossy().to_string();

        tool_pipeline_demo_json(
            &out_dir,
            false,
            "prefer_latest",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            "json",
            None,
            false,
            true,
            "",
            &PipelineScoring::balanced(),
            Some(PipelineEnergyMode::Balanced),
            5.0,
            12.0,
            "balanced",
            None,
            None,
            None,
            3,
        );

        assert!(test_dir.join("frame_a.json").exists());
        assert!(test_dir.join("report.json").exists());
        assert!(test_dir.join("summary.json").exists());

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn pipeline_demo_auto_strategy_and_manifest_emit_files() {
        let test_dir = env::temp_dir().join(format!(
            "matter_pipeline_demo_auto_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&test_dir);
        let out_dir = test_dir.to_string_lossy().to_string();

        tool_pipeline_demo_json(
            &out_dir,
            false,
            "auto",
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            "json",
            None,
            false,
            true,
            "p2",
            &PipelineScoring::balanced(),
            Some(PipelineEnergyMode::Balanced),
            5.0,
            12.0,
            "balanced",
            None,
            None,
            None,
            3,
        );

        assert!(test_dir.join("p2_report_compare.json").exists());
        assert!(test_dir.join("p2_report_compare_dashboard.json").exists());
        assert!(test_dir.join("p2_artifact_manifest.json").exists());

        let report_text = fs::read_to_string(test_dir.join("p2_report.json")).unwrap_or_default();
        assert!(report_text.contains("\"strategy_requested\":\"auto\""));
        assert!(report_text.contains("\"strategy_effective\":"));
        assert!(report_text.contains("\"confidence_profile\":\"balanced\""));
        assert!(report_text.contains("\"ci_decision_reason\":"));
        assert!(report_text.contains("\"ci_decision_code\":"));
        let dashboard_text = fs::read_to_string(test_dir.join("p2_report_compare_dashboard.json"))
            .unwrap_or_default();
        assert!(dashboard_text.contains("\"energy_mode\":\"balanced\""));
        assert!(dashboard_text.contains("\"policy_rationale\":"));
        assert!(dashboard_text.contains("\"policy_factors\":"));
        assert!(dashboard_text.contains("\"penalty_blocked\":"));
        assert!(dashboard_text.contains("\"decision_confidence\":"));
        assert!(dashboard_text.contains("\"decision_gap\":"));
        assert!(dashboard_text.contains("\"caution\":"));
        assert!(dashboard_text.contains("\"emitted_runtime_events\":"));
        assert!(dashboard_text.contains("\"confidence_profile\":\"balanced\""));
        assert!(dashboard_text.contains("\"ci_decision_reason\":"));
        assert!(dashboard_text.contains("\"ci_decision_code\":"));
        let manifest_text =
            fs::read_to_string(test_dir.join("p2_artifact_manifest.json")).unwrap_or_default();
        assert!(manifest_text.contains("\"energy_mode\":\"balanced\""));
        assert!(manifest_text.contains("\"policy_rationale\":"));
        assert!(manifest_text.contains("\"policy_factors\":"));
        assert!(manifest_text.contains("\"action_execute\":"));
        assert!(manifest_text.contains("\"confidence_profile\":\"balanced\""));
        assert!(manifest_text.contains("\"ci_decision_reason\":"));
        assert!(manifest_text.contains("\"ci_decision_code\":"));
        assert!(report_text.contains("\"decision_confidence\":"));
        assert!(report_text.contains("\"emitted_runtime_events\":"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn pipeline_demo_emit_contract_bundle_writes_bundle_artifacts() {
        let test_dir = env::temp_dir().join(format!(
            "matter_pipeline_demo_bundle_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&test_dir);
        let out_dir = test_dir.to_string_lossy().to_string();

        tool_pipeline_demo_json(
            &out_dir,
            false,
            "prefer_latest",
            false,
            false,
            true,
            false,
            false,
            false,
            false,
            "json",
            None,
            false,
            true,
            "bundlex",
            &PipelineScoring::balanced(),
            Some(PipelineEnergyMode::Balanced),
            5.0,
            12.0,
            "balanced",
            None,
            None,
            None,
            3,
        );

        assert!(test_dir.join("bundlex_contract_baseline.json").exists());
        assert!(test_dir.join("bundlex_contract_candidate.json").exists());
        assert!(test_dir.join("bundlex_contract_bundle.json").exists());
        assert!(test_dir.join("bundlex_next_cycle_config.json").exists());

        let bundle_text =
            fs::read_to_string(test_dir.join("bundlex_contract_bundle.json")).unwrap_or_default();
        assert!(bundle_text.contains("\"diff\""));
        assert!(bundle_text.contains("\"upgrade\""));
        assert!(bundle_text.contains("\"baselineFallbackReason\""));
        assert!(bundle_text.contains("\"nextCycleMode\""));
        let next_cycle_text =
            fs::read_to_string(test_dir.join("bundlex_next_cycle_config.json")).unwrap_or_default();
        assert!(next_cycle_text.contains("\"next_cycle\""));
        assert!(next_cycle_text.contains("\"energy_mode_recommended\""));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn pipeline_demo_writes_custom_next_cycle_config_path() {
        let test_dir = env::temp_dir().join(format!(
            "matter_pipeline_demo_next_cycle_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&test_dir);
        let out_dir = test_dir.to_string_lossy().to_string();
        let custom_next_cycle = test_dir.join("my_next_cycle.json");
        let custom_next_cycle_str = custom_next_cycle.to_string_lossy().to_string();

        tool_pipeline_demo_json(
            &out_dir,
            false,
            "prefer_latest",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            "json",
            None,
            false,
            true,
            "nx",
            &PipelineScoring::balanced(),
            Some(PipelineEnergyMode::Balanced),
            5.0,
            12.0,
            "balanced",
            None,
            None,
            Some(&custom_next_cycle_str),
            3,
        );

        assert!(custom_next_cycle.exists());
        let content = fs::read_to_string(&custom_next_cycle).unwrap_or_default();
        assert!(content.contains("\"next_cycle\""));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn next_cycle_config_contains_chain_guard_fields() {
        let test_dir = env::temp_dir().join(format!(
            "matter_pipeline_demo_chain_guard_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&test_dir);
        let out_dir = test_dir.to_string_lossy().to_string();

        tool_pipeline_demo_json(
            &out_dir,
            false,
            "prefer_latest",
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            "json",
            None,
            false,
            true,
            "guardx",
            &PipelineScoring::balanced(),
            Some(PipelineEnergyMode::Balanced),
            5.0,
            12.0,
            "balanced",
            None,
            None,
            None,
            2,
        );

        let next_cycle =
            fs::read_to_string(test_dir.join("guardx_next_cycle_config.json")).unwrap_or_default();
        assert!(next_cycle.contains("\"chain_id\""));
        assert!(next_cycle.contains("\"hop\""));
        assert!(next_cycle.contains("\"max_hops\":2"));

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn rust_ffi_args_source_reads_at_file_and_strips_bom() {
        let test_dir = env::temp_dir().join(format!(
            "matter_rust_ffi_args_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&test_dir);
        let args_path = test_dir.join("args.json");
        fs::write(&args_path, "\u{feff}[]").unwrap();

        let source = rust_ffi_args_source(&format!("@{}", args_path.display())).unwrap();
        assert_eq!(source, "[]");

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn rust_ffi_value_json_roundtrips_nested_values() {
        let mut map = std::collections::HashMap::new();
        map.insert("n".to_string(), Value::Int(7));
        map.insert("name".to_string(), Value::new_string("matter".to_string()));

        let value = Value::new_list(vec![
            Value::Bool(true),
            Value::new_map(map),
            Value::new_function("run".to_string()),
        ]);

        let json = encode_value_json(&value);
        let decoded = rust_ffi_value_from_json(&json).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn rust_ffi_args_from_json_rejects_invalid_payload() {
        let err = rust_ffi_args_from_json(&json!({ "type": "int", "value": 1 })).unwrap_err();
        assert!(err.contains("must be an array"));

        let err = rust_ffi_args_from_json(&json!([{ "type": "int" }])).unwrap_err();
        assert!(err.contains("int is invalid"));
    }

    #[test]
    fn rust_ffi_validate_args_payload_reports_ok() {
        let payload = rust_ffi_validate_args_payload(
            r#"[{"type":"int","value":41},{"type":"string","value":"matter"}]"#,
        )
        .unwrap();

        assert_eq!(payload.get("ok"), Some(&JsonValue::Bool(true)));
        assert_eq!(payload.get("args_count"), Some(&JsonValue::from(2)));
    }

    #[test]
    fn rust_ffi_validate_args_payload_reports_decode_errors() {
        let payload = rust_ffi_validate_args_payload(r#"[{"type":"int"}]"#).unwrap_err();

        assert_eq!(payload.get("ok"), Some(&JsonValue::Bool(false)));
        assert_eq!(
            payload.get("stage").and_then(JsonValue::as_str),
            Some("decode_args")
        );
    }

    fn test_emnr_stream(
        failed_count: u64,
        isolate_count: u64,
        throttle_count: u64,
        observe_count: u64,
        mean_risk: f64,
        mean_confidence: f64,
        final_energy: f32,
    ) -> EmnrStreamReport {
        EmnrStreamReport {
            report: json!({}),
            failed_count,
            isolate_count,
            throttle_count,
            observe_count,
            mean_risk,
            mean_reward: 0.0,
            mean_confidence,
            mean_prediction_error: 0.0,
            final_energy,
            memory_records: 0,
        }
    }

    fn test_pipeline_manifest(
        health_score: f64,
        status: &str,
        promotion_confidence: f64,
        promotion_risk: f64,
        promotion_error: f64,
    ) -> JsonValue {
        json!({
            "schema": "matter.emnr.pipeline.manifest.v1",
            "command": "emnr-pipeline-json",
            "summary": {
                "status": status,
                "health_score": health_score,
                "learning_delta": {
                    "confidence": 0.0,
                    "risk": 0.0,
                    "prediction_error": 0.0,
                    "energy": 0.0,
                    "memory_records": 0
                },
                "promotion_delta": {
                    "confidence": promotion_confidence,
                    "risk": promotion_risk,
                    "prediction_error": promotion_error,
                    "energy": 0.0,
                    "memory_records": 0
                }
            }
        })
    }
}
