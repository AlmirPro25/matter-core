use emnr_core::BrainAction;
use emnr_runtime::{BrainInput, BrainOutput, MolecularBrainRuntime};
use emnr_tensor::TensorSignal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatterRuntimeEvent {
    pub module: String,
    pub phase: MatterPhase,
    pub cpu_cost: f32,
    pub memory_pressure: f32,
    pub error_rate: f32,
    pub latency_ms: f32,
    pub cache_hit_rate: f32,
    pub optimization_confidence: f32,
    pub security_risk: f32,
    pub novelty_score: f32,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MatterPhase {
    Parse,
    Compile,
    Optimize,
    Execute,
    Schedule,
    Bridge,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MatterOperationalDecision {
    Proceed,
    ExploreAlternative,
    ApplyOptimization,
    IsolateRisk,
    ThrottleRuntime,
    ObserveOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatterDecision {
    pub module: String,
    pub phase: MatterPhase,
    pub decision: MatterOperationalDecision,
    pub neural_action: BrainAction,
    pub prediction_error: f32,
    pub confidence: f32,
    pub risk_score: f32,
    pub reward_score: f32,
    pub novelty_score: f32,
    pub energy: f32,
    pub reason: String,
    pub telemetry: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatterSessionReport {
    pub events: usize,
    pub decisions: Vec<MatterDecision>,
    pub summary: MatterSessionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatterSessionSummary {
    pub decision_counts: BTreeMap<String, u64>,
    pub mean_prediction_error: f32,
    pub mean_confidence: f32,
    pub mean_risk: f32,
    pub mean_reward: f32,
    pub final_energy: f32,
    pub memory_records: usize,
    pub final_field: emnr_core::GlobalField,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatterBridgeSession {
    pub runtime: MolecularBrainRuntime,
}

impl MatterRuntimeEvent {
    pub fn to_brain_input(&self) -> BrainInput {
        let cpu = normalize(self.cpu_cost, 100.0);
        let memory = normalize(self.memory_pressure, 100.0);
        let error = self.error_rate.clamp(0.0, 1.0);
        let latency = normalize(self.latency_ms, 1_000.0);
        let cache_miss = 1.0 - self.cache_hit_rate.clamp(0.0, 1.0);
        let risk = self.risk_score();
        let reward = self.reward_score();

        BrainInput {
            signal: TensorSignal {
                values: vec![cpu, memory, error.max(latency), cache_miss],
                shape: vec![4],
                energy: 1.0,
                timestamp: 0,
                label: self.label.clone(),
            },
            reward,
            danger: risk,
            novelty: self.novelty_score.clamp(0.0, 1.0),
            label: Some(format!("matter::{:?}::{}", self.phase, self.module)),
        }
    }

    pub fn risk_score(&self) -> f32 {
        let resource_risk =
            normalize(self.cpu_cost, 100.0) * 0.25 + normalize(self.memory_pressure, 100.0) * 0.25;
        let reliability_risk =
            self.error_rate.clamp(0.0, 1.0) * 0.30 + normalize(self.latency_ms, 1_000.0) * 0.10;
        let security_risk = self.security_risk.clamp(0.0, 1.0) * 0.35;
        (resource_risk + reliability_risk + security_risk).clamp(0.0, 1.0)
    }

    pub fn reward_score(&self) -> f32 {
        let efficiency = (1.0 - normalize(self.cpu_cost, 100.0)) * 0.20
            + (1.0 - normalize(self.memory_pressure, 100.0)) * 0.20
            + self.cache_hit_rate.clamp(0.0, 1.0) * 0.20;
        let correctness = (1.0 - self.error_rate.clamp(0.0, 1.0)) * 0.25;
        let optimization = self.optimization_confidence.clamp(0.0, 1.0) * 0.15;
        (efficiency + correctness + optimization).clamp(0.0, 1.0)
    }
}

pub fn decide_event(
    runtime: &mut MolecularBrainRuntime,
    event: MatterRuntimeEvent,
) -> MatterDecision {
    let risk_score = event.risk_score();
    let reward_score = event.reward_score();
    let novelty_score = event.novelty_score.clamp(0.0, 1.0);
    let output = runtime.tick(event.to_brain_input());
    build_decision(event, output, risk_score, reward_score, novelty_score)
}

impl MatterBridgeSession {
    pub fn new() -> Self {
        Self {
            runtime: MolecularBrainRuntime::new(),
        }
    }

    pub fn decide(&mut self, event: MatterRuntimeEvent) -> MatterDecision {
        decide_event(&mut self.runtime, event)
    }

    pub fn run(&mut self, events: Vec<MatterRuntimeEvent>) -> MatterSessionReport {
        let decisions = events
            .into_iter()
            .map(|event| self.decide(event))
            .collect::<Vec<_>>();
        let summary = MatterSessionSummary::from_decisions(&decisions, &self.runtime);
        MatterSessionReport {
            events: decisions.len(),
            decisions,
            summary,
        }
    }
}

impl Default for MatterBridgeSession {
    fn default() -> Self {
        Self::new()
    }
}

impl MatterSessionSummary {
    fn from_decisions(decisions: &[MatterDecision], runtime: &MolecularBrainRuntime) -> Self {
        let mut decision_counts = BTreeMap::new();
        for decision in decisions {
            *decision_counts
                .entry(format!("{:?}", decision.decision))
                .or_default() += 1;
        }

        Self {
            decision_counts,
            mean_prediction_error: mean(decisions.iter().map(|d| d.prediction_error)),
            mean_confidence: mean(decisions.iter().map(|d| d.confidence)),
            mean_risk: mean(decisions.iter().map(|d| d.risk_score)),
            mean_reward: mean(decisions.iter().map(|d| d.reward_score)),
            final_energy: runtime.energy.current,
            memory_records: runtime.memory.records.len(),
            final_field: runtime.field.clone(),
        }
    }
}

fn build_decision(
    event: MatterRuntimeEvent,
    output: BrainOutput,
    risk_score: f32,
    reward_score: f32,
    novelty_score: f32,
) -> MatterDecision {
    let decision = if risk_score > 0.70 || event.security_risk > 0.85 || event.error_rate > 0.50 {
        MatterOperationalDecision::IsolateRisk
    } else {
        match output.action {
            BrainAction::Avoid => MatterOperationalDecision::IsolateRisk,
            BrainAction::Rest => MatterOperationalDecision::ThrottleRuntime,
            BrainAction::Explore => MatterOperationalDecision::ExploreAlternative,
            BrainAction::Approach if event.optimization_confidence > 0.65 => {
                MatterOperationalDecision::ApplyOptimization
            }
            BrainAction::Approach => MatterOperationalDecision::Proceed,
            BrainAction::Observe | BrainAction::Speak(_) => MatterOperationalDecision::ObserveOnly,
        }
    };

    let confidence = (reward_score + (1.0 - risk_score) + (1.0 - output.prediction_error)) / 3.0;
    let reason = decision_reason(&decision, risk_score, reward_score, novelty_score);

    MatterDecision {
        module: event.module,
        phase: event.phase,
        decision,
        neural_action: output.action,
        prediction_error: output.prediction_error,
        confidence: confidence.clamp(0.0, 1.0),
        risk_score,
        reward_score,
        novelty_score,
        energy: output.energy,
        reason,
        telemetry: json!({
            "brain": output.telemetry,
            "matter_scores": {
                "risk": risk_score,
                "reward": reward_score,
                "novelty": novelty_score,
            }
        }),
    }
}

fn decision_reason(
    decision: &MatterOperationalDecision,
    risk_score: f32,
    reward_score: f32,
    novelty_score: f32,
) -> String {
    match decision {
        MatterOperationalDecision::IsolateRisk if risk_score > 0.70 => {
            format!("risk score {risk_score:.3} crossed the runtime isolation threshold")
        }
        MatterOperationalDecision::IsolateRisk => {
            format!("neural action favored isolation under elevated risk score {risk_score:.3}")
        }
        MatterOperationalDecision::ThrottleRuntime => {
            "runtime energy is low; reduce scheduling pressure".to_string()
        }
        MatterOperationalDecision::ExploreAlternative => {
            format!("novelty score {novelty_score:.3} favors testing an alternate path")
        }
        MatterOperationalDecision::ApplyOptimization => {
            format!("reward score {reward_score:.3} supports applying the optimization")
        }
        MatterOperationalDecision::Proceed => {
            format!("reward score {reward_score:.3} is stronger than risk {risk_score:.3}")
        }
        MatterOperationalDecision::ObserveOnly => {
            "signal is inconclusive; collect more telemetry before acting".to_string()
        }
    }
}

fn normalize(value: f32, max: f32) -> f32 {
    if max <= f32::EPSILON {
        0.0
    } else {
        (value / max).clamp(0.0, 1.0)
    }
}

fn mean(values: impl Iterator<Item = f32>) -> f32 {
    let mut sum = 0.0;
    let mut count = 0;
    for value in values {
        sum += value;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_maps_to_brain_input_with_risk_and_reward() {
        let event = sample_event(0.05, 0.9);
        let input = event.to_brain_input();
        assert_eq!(input.signal.len(), 4);
        assert!(input.reward > 0.5);
        assert!(input.danger < 0.5);
    }

    #[test]
    fn dangerous_event_is_isolated() {
        let mut runtime = MolecularBrainRuntime::new();
        let event = sample_event(0.95, 0.1);
        let decision = decide_event(&mut runtime, event);
        assert_eq!(decision.decision, MatterOperationalDecision::IsolateRisk);
        assert_eq!(decision.neural_action, BrainAction::Avoid);
    }

    #[test]
    fn session_keeps_runtime_state_across_events() {
        let mut session = MatterBridgeSession::new();
        let report = session.run(vec![sample_event(0.05, 0.9), sample_event(0.95, 0.1)]);
        assert_eq!(report.events, 2);
        assert_eq!(report.decisions.len(), 2);
        assert!(report.summary.final_energy < 100.0);
        assert!(report.summary.memory_records > 0);
        assert_eq!(report.summary.decision_counts.get("IsolateRisk"), Some(&1));
    }

    fn sample_event(security_risk: f32, optimization_confidence: f32) -> MatterRuntimeEvent {
        MatterRuntimeEvent {
            module: "matter-vm".to_string(),
            phase: MatterPhase::Execute,
            cpu_cost: if security_risk > 0.8 { 95.0 } else { 30.0 },
            memory_pressure: if security_risk > 0.8 { 90.0 } else { 25.0 },
            error_rate: if security_risk > 0.8 { 0.75 } else { 0.05 },
            latency_ms: if security_risk > 0.8 { 900.0 } else { 80.0 },
            cache_hit_rate: 0.85,
            optimization_confidence,
            security_risk,
            novelty_score: 0.3,
            label: Some("sample".to_string()),
        }
    }
}
