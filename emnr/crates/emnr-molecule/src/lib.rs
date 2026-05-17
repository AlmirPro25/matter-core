use emnr_core::{GlobalField, MoleculeId};
use emnr_tensor::TensorSignal;
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::json;

const FIELD_SIZE: usize = 8;
const WEIGHT_DECAY: f32 = 0.0005;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralMolecule {
    pub id: MoleculeId,
    pub input_size: usize,
    pub output_size: usize,
    pub weights: Vec<f32>,
    pub recurrent: Vec<f32>,
    pub modulation: Vec<f32>,
    pub bias: Vec<f32>,
    pub state: Vec<f32>,
    pub learning_rate: f32,
    pub energy_cost: f32,
}

impl NeuralMolecule {
    pub fn new(id: MoleculeId, input_size: usize, output_size: usize) -> Self {
        let mut rng = StdRng::seed_from_u64(id.wrapping_mul(31).wrapping_add(17));
        let mut init = || rng.gen_range(-0.2..0.2);
        Self {
            id,
            input_size,
            output_size,
            weights: (0..input_size * output_size).map(|_| init()).collect(),
            recurrent: (0..output_size * output_size)
                .map(|_| init() * 0.5)
                .collect(),
            modulation: (0..FIELD_SIZE * output_size)
                .map(|_| init() * 0.25)
                .collect(),
            bias: vec![0.0; output_size],
            state: vec![0.0; output_size],
            learning_rate: 0.025,
            energy_cost: 0.15 + output_size as f32 * 0.01,
        }
    }

    pub fn forward(&mut self, input: &TensorSignal, field: &GlobalField) -> TensorSignal {
        let phi = field.modulation_vector();
        let mut output = vec![0.0; self.output_size];

        for (out_idx, output_value) in output.iter_mut().enumerate() {
            let input_sum = (0..self.input_size)
                .map(|in_idx| {
                    let value = input.values.get(in_idx).copied().unwrap_or_default();
                    value * self.weights[out_idx * self.input_size + in_idx]
                })
                .sum::<f32>();

            let recurrent_sum = (0..self.output_size)
                .map(|state_idx| {
                    self.state[state_idx] * self.recurrent[out_idx * self.output_size + state_idx]
                })
                .sum::<f32>();

            let modulation_sum = phi
                .iter()
                .enumerate()
                .map(|(field_idx, value)| value * self.modulation[out_idx * FIELD_SIZE + field_idx])
                .sum::<f32>();

            *output_value =
                (input_sum + recurrent_sum + modulation_sum + self.bias[out_idx]).tanh();
        }

        self.state = output.clone();
        TensorSignal {
            values: output,
            shape: vec![self.output_size],
            energy: input.energy * field.energy.max(0.0) / 100.0,
            timestamp: input.timestamp,
            label: input.label.clone(),
        }
    }

    pub fn learn(&mut self, input: &TensorSignal, error: &TensorSignal, field: &GlobalField) {
        let gain = self.learning_rate * field.learning_gain.clamp(0.0, 2.0);
        for out_idx in 0..self.output_size {
            let err = error
                .values
                .get(out_idx)
                .copied()
                .unwrap_or_default()
                .clamp(-1.0, 1.0);
            for in_idx in 0..self.input_size {
                let input_value = input.values.get(in_idx).copied().unwrap_or_default();
                let weight = &mut self.weights[out_idx * self.input_size + in_idx];
                *weight += gain * err * input_value;
                *weight *= 1.0 - WEIGHT_DECAY;
                *weight = weight.clamp(-5.0, 5.0);
            }
            self.bias[out_idx] = (self.bias[out_idx] + gain * err * 0.1).clamp(-2.0, 2.0);
        }
    }

    pub fn energy_cost(&self) -> f32 {
        self.energy_cost
    }

    pub fn telemetry(&self) -> serde_json::Value {
        let state_abs_mean = if self.state.is_empty() {
            0.0
        } else {
            self.state.iter().map(|v| v.abs()).sum::<f32>() / self.state.len() as f32
        };
        let weight_abs_mean = if self.weights.is_empty() {
            0.0
        } else {
            self.weights.iter().map(|v| v.abs()).sum::<f32>() / self.weights.len() as f32
        };

        json!({
            "id": self.id,
            "input_size": self.input_size,
            "output_size": self.output_size,
            "learning_rate": self.learning_rate,
            "energy_cost": self.energy_cost,
            "state_abs_mean": state_abs_mean,
            "weight_abs_mean": weight_abs_mean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_produces_expected_output_shape() {
        let mut molecule = NeuralMolecule::new(1, 4, 3);
        let input = TensorSignal::new(vec![0.1, 0.2, 0.3, 0.4]);
        let output = molecule.forward(&input, &GlobalField::default());
        assert_eq!(output.len(), 3);
        assert!(output
            .values
            .iter()
            .all(|value| (-1.0..=1.0).contains(value)));
    }
}
