use clap::{Parser, Subcommand};
use emnr_core::BrainAction;
use emnr_matter_bridge::{decide_event, MatterBridgeSession, MatterRuntimeEvent};
use emnr_runtime::{BrainInput, MolecularBrainRuntime};
use emnr_tensor::TensorSignal;
use std::collections::BTreeMap;
use std::{fs, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
enum CliError {
    #[error("failed to read {path}: {source}")]
    ReadInput {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse JSON from {path}: {source}")]
    ParseInput {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("failed to serialize output: {0}")]
    Serialize(serde_json::Error),
}

#[derive(Debug, Parser)]
#[command(name = "emnr-cli", about = "Energetic Molecular Neural Runtime CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Tick {
        #[arg(long)]
        input: PathBuf,
    },
    Run {
        #[arg(long, default_value_t = 100)]
        steps: u64,
    },
    Evaluate {
        #[arg(long, default_value_t = 100)]
        steps: u64,
    },
    Decide {
        #[arg(long)]
        event: PathBuf,
    },
    DecideStream {
        #[arg(long)]
        events: PathBuf,
    },
    InspectState,
}

fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    match cli.command {
        Command::Tick { input } => tick(input),
        Command::Run { steps } => run(steps),
        Command::Evaluate { steps } => evaluate(steps),
        Command::Decide { event } => decide(event),
        Command::DecideStream { events } => decide_stream(events),
        Command::InspectState => inspect_state(),
    }
}

fn tick(input_path: PathBuf) -> Result<(), CliError> {
    let input = read_input(&input_path)?;
    let mut runtime = MolecularBrainRuntime::new();
    let output = runtime.tick(input);
    print_json(&output)
}

fn run(steps: u64) -> Result<(), CliError> {
    let mut runtime = MolecularBrainRuntime::new();
    for step in 0..steps {
        let output = runtime.tick(simulated_input(step));
        print_json(&output)?;
    }
    Ok(())
}

fn evaluate(steps: u64) -> Result<(), CliError> {
    let mut runtime = MolecularBrainRuntime::new();
    let mut action_counts = BTreeMap::<String, u64>::new();
    let mut errors = Vec::with_capacity(steps as usize);
    let mut min_error = f32::MAX;
    let mut max_error = f32::MIN;

    for step in 0..steps {
        let output = runtime.tick(simulated_input(step));
        min_error = min_error.min(output.prediction_error);
        max_error = max_error.max(output.prediction_error);
        errors.push(output.prediction_error);
        *action_counts
            .entry(action_name(&output.action).to_string())
            .or_default() += 1;
    }

    let mean_prediction_error = mean(&errors);
    let trend_window = errors.len().min(10);
    let early_error = mean(&errors[..trend_window]);
    let late_error = mean(&errors[errors.len().saturating_sub(trend_window)..]);
    let error_delta = late_error - early_error;

    let report = serde_json::json!({
        "steps": steps,
        "mean_prediction_error": mean_prediction_error,
        "min_prediction_error": if steps == 0 { 0.0 } else { min_error },
        "max_prediction_error": if steps == 0 { 0.0 } else { max_error },
        "early_prediction_error": early_error,
        "late_prediction_error": late_error,
        "error_delta": error_delta,
        "action_counts": action_counts,
        "final_energy": runtime.energy.current,
        "memory_records": runtime.memory.records.len(),
        "field": runtime.field,
    });

    print_json(&report)
}

fn decide(event_path: PathBuf) -> Result<(), CliError> {
    let event = read_event(&event_path)?;
    let mut runtime = MolecularBrainRuntime::new();
    let decision = decide_event(&mut runtime, event);
    print_json(&decision)
}

fn decide_stream(events_path: PathBuf) -> Result<(), CliError> {
    let events = read_events(&events_path)?;
    let mut session = MatterBridgeSession::new();
    let report = session.run(events);
    print_json(&report)
}

fn inspect_state() -> Result<(), CliError> {
    let runtime = MolecularBrainRuntime::new();
    let state = serde_json::json!({
        "tick": runtime.tick,
        "field": runtime.field,
        "energy": runtime.energy.telemetry(),
        "memory": runtime.memory.telemetry(),
        "regions": {
            "attention": runtime.attention.telemetry(),
            "prediction": runtime.prediction.telemetry(),
            "emotion": runtime.emotion.telemetry(),
            "memory": runtime.memory_region.telemetry(),
            "action": runtime.action.telemetry(),
        }
    });
    print_json(&state)
}

fn read_input(path: &PathBuf) -> Result<BrainInput, CliError> {
    let raw = fs::read_to_string(path).map_err(|source| CliError::ReadInput {
        path: path.clone(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| CliError::ParseInput {
        path: path.clone(),
        source,
    })
}

fn read_event(path: &PathBuf) -> Result<MatterRuntimeEvent, CliError> {
    let raw = fs::read_to_string(path).map_err(|source| CliError::ReadInput {
        path: path.clone(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| CliError::ParseInput {
        path: path.clone(),
        source,
    })
}

fn read_events(path: &PathBuf) -> Result<Vec<MatterRuntimeEvent>, CliError> {
    let raw = fs::read_to_string(path).map_err(|source| CliError::ReadInput {
        path: path.clone(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| CliError::ParseInput {
        path: path.clone(),
        source,
    })
}

fn print_json(value: &impl serde::Serialize) -> Result<(), CliError> {
    let json = serde_json::to_string_pretty(value).map_err(CliError::Serialize)?;
    println!("{json}");
    Ok(())
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

fn action_name(action: &BrainAction) -> &str {
    match action {
        BrainAction::Observe => "Observe",
        BrainAction::Explore => "Explore",
        BrainAction::Approach => "Approach",
        BrainAction::Avoid => "Avoid",
        BrainAction::Rest => "Rest",
        BrainAction::Speak(_) => "Speak",
    }
}

fn simulated_input(step: u64) -> BrainInput {
    match step % 3 {
        1 => BrainInput {
            signal: TensorSignal {
                values: vec![0.2, 0.8, 0.4, 0.1],
                shape: vec![4],
                energy: 1.0,
                timestamp: step,
                label: Some("reward_signal".to_string()),
            },
            reward: 0.9,
            danger: 0.1,
            novelty: 0.4,
            label: Some("reward_test".to_string()),
        },
        2 => BrainInput {
            signal: TensorSignal {
                values: vec![0.9, 0.1, 0.2, 0.8],
                shape: vec![4],
                energy: 1.0,
                timestamp: step,
                label: Some("danger_signal".to_string()),
            },
            reward: 0.0,
            danger: 0.95,
            novelty: 0.7,
            label: Some("danger_test".to_string()),
        },
        _ => BrainInput {
            signal: TensorSignal {
                values: vec![0.3, 0.3, 0.3, 0.3],
                shape: vec![4],
                energy: 1.0,
                timestamp: step,
                label: Some("neutral_signal".to_string()),
            },
            reward: 0.1,
            danger: 0.1,
            novelty: 0.1,
            label: Some("neutral_test".to_string()),
        },
    }
}
