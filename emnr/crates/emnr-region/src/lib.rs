use emnr_core::{GlobalField, RegionId};
use emnr_molecule::NeuralMolecule;
use emnr_tensor::TensorSignal;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MolecularLayer {
    pub molecules: Vec<NeuralMolecule>,
}

impl MolecularLayer {
    pub fn new(
        molecule_id_start: u64,
        molecule_count: usize,
        input_size: usize,
        output_size: usize,
    ) -> Self {
        let molecules = (0..molecule_count)
            .map(|idx| NeuralMolecule::new(molecule_id_start + idx as u64, input_size, output_size))
            .collect();
        Self { molecules }
    }

    pub fn forward(&mut self, input: &TensorSignal, field: &GlobalField) -> TensorSignal {
        if self.molecules.is_empty() {
            return input.clone();
        }

        let outputs = self
            .molecules
            .iter_mut()
            .map(|molecule| molecule.forward(input, field))
            .collect::<Vec<_>>();
        let output_size = outputs[0].len();
        let mut values = vec![0.0; output_size];
        for output in &outputs {
            for (idx, value) in output.values.iter().enumerate() {
                values[idx] += value / outputs.len() as f32;
            }
        }

        TensorSignal {
            values,
            shape: vec![output_size],
            energy: input.energy,
            timestamp: input.timestamp,
            label: input.label.clone(),
        }
    }

    pub fn learn(&mut self, input: &TensorSignal, error: &TensorSignal, field: &GlobalField) {
        for molecule in &mut self.molecules {
            molecule.learn(input, error, field);
        }
    }

    pub fn energy_cost(&self) -> f32 {
        self.molecules
            .iter()
            .map(NeuralMolecule::energy_cost)
            .sum::<f32>()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainRegion {
    pub id: RegionId,
    pub name: String,
    pub layers: Vec<MolecularLayer>,
    pub state: TensorSignal,
    pub local_energy: f32,
    pub plasticity: f32,
}

impl BrainRegion {
    pub fn new(
        id: RegionId,
        name: impl Into<String>,
        input_size: usize,
        hidden_size: usize,
        output_size: usize,
        molecule_count: usize,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            layers: vec![
                MolecularLayer::new(id * 10_000, molecule_count, input_size, hidden_size),
                MolecularLayer::new(
                    id * 10_000 + 1_000,
                    molecule_count,
                    hidden_size,
                    output_size,
                ),
            ],
            state: TensorSignal::zeros(output_size),
            local_energy: 1.0,
            plasticity: 1.0,
        }
    }

    pub fn forward(&mut self, input: &TensorSignal, field: &GlobalField) -> TensorSignal {
        let mut signal = input.clone();
        for layer in &mut self.layers {
            signal = layer.forward(&signal, field);
        }
        self.local_energy = (self.local_energy - self.energy_cost() * 0.001).clamp(0.0, 1.0);
        self.state = signal.clone();
        signal
    }

    pub fn learn(&mut self, input: &TensorSignal, error: &TensorSignal, field: &GlobalField) {
        let mut layer_input = input.clone();
        for layer in &mut self.layers {
            layer.learn(&layer_input, error, field);
            layer_input = layer.forward(&layer_input, field);
        }
        self.plasticity = (self.plasticity * 0.999 + field.learning_gain * 0.001).clamp(0.0, 2.0);
    }

    pub fn energy_cost(&self) -> f32 {
        self.layers
            .iter()
            .map(MolecularLayer::energy_cost)
            .sum::<f32>()
            * self.local_energy.max(0.1)
    }

    pub fn telemetry(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "name": self.name,
            "layers": self.layers.len(),
            "molecules": self.layers.iter().map(|layer| layer.molecules.len()).sum::<usize>(),
            "local_energy": self.local_energy,
            "plasticity": self.plasticity,
            "state": {
                "len": self.state.len(),
                "energy": self.state.energy,
                "values": self.state.values,
            },
        })
    }
}
