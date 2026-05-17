use serde::{Deserialize, Serialize};

pub type TickId = u64;
pub type MoleculeId = u64;
pub type RegionId = u64;
pub type Energy = f32;
pub type PredictionError = f32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BrainAction {
    Observe,
    Explore,
    Approach,
    Avoid,
    Rest,
    Speak(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainTelemetry {
    pub tick: TickId,
    pub action: BrainAction,
    pub prediction_error: PredictionError,
    pub field: GlobalField,
    pub regions: serde_json::Value,
    pub memory: serde_json::Value,
    pub energy: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalField {
    pub energy: f32,
    pub dopamine: f32,
    pub serotonin: f32,
    pub gaba: f32,
    pub noradrenaline: f32,
    pub stress: f32,
    pub attention_gain: f32,
    pub learning_gain: f32,
}

impl Default for GlobalField {
    fn default() -> Self {
        Self {
            energy: 100.0,
            dopamine: 0.1,
            serotonin: 0.5,
            gaba: 0.3,
            noradrenaline: 0.1,
            stress: 0.1,
            attention_gain: 1.0,
            learning_gain: 1.0,
        }
    }
}

impl GlobalField {
    pub fn modulation_vector(&self) -> [f32; 8] {
        [
            self.energy / 100.0,
            self.dopamine,
            self.serotonin,
            self.gaba,
            self.noradrenaline,
            self.stress,
            self.attention_gain,
            self.learning_gain,
        ]
    }
}
