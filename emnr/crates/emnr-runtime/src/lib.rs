use emnr_core::{BrainAction, GlobalField};
use emnr_energy::EnergySystem;
use emnr_memory::MemoryStore;
use emnr_region::BrainRegion;
use emnr_tensor::TensorSignal;
use serde::{Deserialize, Serialize};
use serde_json::json;

const ENRICHED_SIGNAL_SIZE: usize = 7;
const REGION_HIDDEN_SIZE: usize = 10;
const MOLECULES_PER_LAYER: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainInput {
    pub signal: TensorSignal,
    pub reward: f32,
    pub danger: f32,
    pub novelty: f32,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainOutput {
    pub tick: u64,
    pub action: BrainAction,
    pub prediction_error: f32,
    pub energy: f32,
    pub telemetry: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MolecularBrainRuntime {
    pub tick: u64,
    pub field: GlobalField,
    pub energy: EnergySystem,
    pub memory: MemoryStore,
    pub attention: BrainRegion,
    pub prediction: BrainRegion,
    pub emotion: BrainRegion,
    pub memory_region: BrainRegion,
    pub action: BrainRegion,
    pub last_prediction: Option<TensorSignal>,
}

impl MolecularBrainRuntime {
    pub fn new() -> Self {
        Self {
            tick: 0,
            field: GlobalField::default(),
            energy: EnergySystem::default(),
            memory: MemoryStore::new(256),
            attention: make_region(1, "AttentionRegion"),
            prediction: make_region(2, "PredictionRegion"),
            emotion: make_region(3, "EmotionRegion"),
            memory_region: make_region(4, "MemoryRegion"),
            action: make_region(5, "ActionRegion"),
            last_prediction: None,
        }
    }

    pub fn tick(&mut self, input: BrainInput) -> BrainOutput {
        self.tick += 1;
        self.energy.recover();
        self.memory.decay();

        let enriched = self.enrich_input(&input);
        let attended = self.attention.forward(&enriched, &self.field);
        let context = self.memory.retrieve(&attended, 3, &self.field, self.tick);
        let integrated = attended.combine(&context);
        let prediction = self.prediction.forward(&integrated, &self.field);

        let prediction_error = prediction.mse(&enriched);
        let error_signal = diff_signal(&enriched, &prediction, self.tick);
        let emotion_state = self.emotion.forward(&error_signal, &self.field);

        self.update_field(&input, prediction_error, &emotion_state);

        let memory_encoded = self.memory_region.forward(&integrated, &self.field);
        if prediction_error > 0.25 || input.reward > 0.5 || input.danger > 0.5 {
            let emotional_weight = (prediction_error + input.reward + input.danger).clamp(0.0, 1.0);
            self.memory
                .store(memory_encoded.clone(), emotional_weight, self.tick);
        }

        let action_state = self.action.forward(&memory_encoded, &self.field);
        let action = self.choose_action(&input);

        self.attention.learn(&enriched, &error_signal, &self.field);
        self.prediction
            .learn(&integrated, &error_signal, &self.field);
        self.emotion
            .learn(&error_signal, &error_signal, &self.field);
        self.memory_region
            .learn(&integrated, &error_signal, &self.field);
        self.action
            .learn(&memory_encoded, &error_signal, &self.field);

        let total_energy_cost = self.total_region_energy_cost() + action_energy_cost(&action);
        self.energy.spend(total_energy_cost);
        self.field.energy = self.energy.current;
        self.apply_energy_pressure();

        self.last_prediction = Some(prediction);

        let telemetry = self.telemetry(&action, prediction_error, &action_state);
        BrainOutput {
            tick: self.tick,
            action,
            prediction_error,
            energy: self.energy.current,
            telemetry,
        }
    }

    pub fn telemetry(
        &self,
        action: &BrainAction,
        prediction_error: f32,
        action_state: &TensorSignal,
    ) -> serde_json::Value {
        json!({
            "tick": self.tick,
            "action": action,
            "prediction_error": prediction_error,
            "field": self.field,
            "regions": {
                "attention": self.attention.telemetry(),
                "prediction": self.prediction.telemetry(),
                "emotion": self.emotion.telemetry(),
                "memory": self.memory_region.telemetry(),
                "action": self.action.telemetry(),
            },
            "memory": self.memory.telemetry(),
            "energy": self.energy.telemetry(),
            "action_state": {
                "values": action_state.values,
                "energy": action_state.energy,
            },
        })
    }

    fn enrich_input(&self, input: &BrainInput) -> TensorSignal {
        let mut values = input
            .signal
            .values
            .iter()
            .copied()
            .take(4)
            .collect::<Vec<_>>();
        values.resize(4, 0.0);
        values.push(input.reward.clamp(0.0, 1.0));
        values.push(input.danger.clamp(0.0, 1.0));
        values.push(input.novelty.clamp(0.0, 1.0));
        let mut signal = TensorSignal {
            values,
            shape: vec![ENRICHED_SIGNAL_SIZE],
            energy: input.signal.energy,
            timestamp: self.tick,
            label: input.label.clone().or_else(|| input.signal.label.clone()),
        };
        signal.normalize();
        signal
    }

    fn update_field(&mut self, input: &BrainInput, prediction_error: f32, emotion: &TensorSignal) {
        let emotion_drive = if emotion.is_empty() {
            0.0
        } else {
            emotion.values.iter().map(|value| value.abs()).sum::<f32>() / emotion.len() as f32
        };

        self.field.dopamine =
            (self.field.dopamine * 0.90 + input.reward.clamp(0.0, 1.0) * 0.25).clamp(0.0, 1.5);
        self.field.noradrenaline =
            (self.field.noradrenaline * 0.88 + input.danger.clamp(0.0, 1.0) * 0.35).clamp(0.0, 1.5);
        self.field.serotonin = (self.field.serotonin * 0.98
            + (1.0 - input.danger).clamp(0.0, 1.0) * 0.01)
            .clamp(0.0, 1.0);
        self.field.gaba =
            (self.field.gaba * 0.98 + (1.0 - prediction_error).max(0.0) * 0.01).clamp(0.0, 1.0);
        self.field.stress = (self.field.stress * 0.9
            + input.danger.clamp(0.0, 1.0) * 0.25
            + prediction_error * 0.15
            + emotion_drive * 0.05)
            .clamp(0.0, 1.5);
        self.field.attention_gain =
            (1.0 + prediction_error * 1.2 + input.novelty * 0.3).clamp(0.5, 2.5);
        self.field.learning_gain =
            (1.0 + self.field.dopamine * 0.25 - self.field.stress * 0.3).clamp(0.05, 2.0);
    }

    fn apply_energy_pressure(&mut self) {
        let pressure = self.energy.pressure();
        if pressure > 0.65 {
            self.field.stress = (self.field.stress + pressure * 0.15).clamp(0.0, 1.5);
            self.field.learning_gain *= (1.0 - pressure * 0.5).clamp(0.05, 1.0);
        }
    }

    fn choose_action(&self, input: &BrainInput) -> BrainAction {
        if input.danger > 0.7 {
            BrainAction::Avoid
        } else if self.energy.current < 20.0 {
            BrainAction::Rest
        } else if input.reward > 0.6 {
            BrainAction::Approach
        } else if input.novelty > 0.6 {
            BrainAction::Explore
        } else {
            BrainAction::Observe
        }
    }

    fn total_region_energy_cost(&self) -> f32 {
        self.attention.energy_cost()
            + self.prediction.energy_cost()
            + self.emotion.energy_cost()
            + self.memory_region.energy_cost()
            + self.action.energy_cost()
    }
}

impl Default for MolecularBrainRuntime {
    fn default() -> Self {
        Self::new()
    }
}

fn make_region(id: u64, name: &str) -> BrainRegion {
    BrainRegion::new(
        id,
        name,
        ENRICHED_SIGNAL_SIZE,
        REGION_HIDDEN_SIZE,
        ENRICHED_SIGNAL_SIZE,
        MOLECULES_PER_LAYER,
    )
}

fn diff_signal(target: &TensorSignal, prediction: &TensorSignal, tick: u64) -> TensorSignal {
    let len = target.len().max(prediction.len());
    TensorSignal {
        values: (0..len)
            .map(|idx| {
                target.values.get(idx).copied().unwrap_or_default()
                    - prediction.values.get(idx).copied().unwrap_or_default()
            })
            .collect(),
        shape: vec![len],
        energy: target.energy,
        timestamp: tick,
        label: Some("prediction_error".to_string()),
    }
}

fn action_energy_cost(action: &BrainAction) -> f32 {
    match action {
        BrainAction::Rest => 0.05,
        BrainAction::Observe => 0.1,
        BrainAction::Explore => 0.25,
        BrainAction::Approach => 0.2,
        BrainAction::Avoid => 0.3,
        BrainAction::Speak(text) => 0.2 + text.len() as f32 * 0.001,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_returns_action_and_telemetry() {
        let mut runtime = MolecularBrainRuntime::new();
        let output = runtime.tick(BrainInput {
            signal: TensorSignal::new(vec![0.2, 0.8, 0.4, 0.1]),
            reward: 0.9,
            danger: 0.1,
            novelty: 0.4,
            label: Some("reward_test".to_string()),
        });
        assert_eq!(output.tick, 1);
        assert_eq!(output.action, BrainAction::Approach);
        assert!(output.telemetry.get("regions").is_some());
        assert!(output.prediction_error >= 0.0);
    }
}
