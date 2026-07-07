use crate::{QuantumGate, QuantumState};
use matter_backend::{Backend, Value};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

pub struct QuantumBackend;

impl QuantumBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuantumBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for QuantumBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "status" | "info" | "capabilities" => Ok(frontier_status(
                "quantum",
                "state_vector_simulator",
                "functional_simulator",
                &["bell_state", "bell_stats", "grover", "qft"],
            )),
            "bell_stats" => {
                if args.len() < 2 {
                    return Err("quantum.bell_stats: expected (shots, seed)".to_string());
                }
                let shots = args[0].as_int()?;
                let seed = args[1].as_int()?;
                if shots <= 0 || shots > 100_000 {
                    return Err(
                        "quantum.bell_stats: shots must be between 1 and 100000".to_string()
                    );
                }
                Ok(bell_stats(shots as usize, seed as u64))
            }
            "bell_state" => {
                let mut state = QuantumState::new(2);
                state
                    .apply_gate(&QuantumGate::H, 0)
                    .map_err(|e| format!("{:?}", e))?;
                state
                    .apply_controlled_gate(&QuantumGate::X, 0, 1)
                    .map_err(|e| format!("{:?}", e))?;
                let result = state.measure();
                let q0 = result & 1;
                let q1 = (result >> 1) & 1;
                Ok(Value::new_list(vec![
                    Value::Int(q0 as i64),
                    Value::Int(q1 as i64),
                ]))
            }
            "grover" => {
                if args.is_empty() {
                    return Err("quantum.grover: expected target index".to_string());
                }
                let target = args[0].as_int()? as usize;
                if target >= 8 {
                    return Err(
                        "quantum.grover: target index must be < 8 for 3-qubit search".to_string(),
                    );
                }
                let circuit = crate::algorithms::grover_search(3, target);
                let mut state = QuantumState::new(3);
                circuit
                    .execute(&mut state)
                    .map_err(|e| format!("{:?}", e))?;
                let measured = state.measure();
                Ok(Value::Int(measured as i64))
            }
            "qft" => {
                if args.is_empty() {
                    return Err("quantum.qft: expected qubit count".to_string());
                }
                let qubits = args[0].as_int()? as usize;
                if qubits == 0 || qubits > 6 {
                    return Err(
                        "quantum.qft: qubit count must be between 1 and 6 for safety".to_string(),
                    );
                }
                let circuit = crate::algorithms::qft(qubits);
                let mut state = QuantumState::new(qubits);
                circuit
                    .execute(&mut state)
                    .map_err(|e| format!("{:?}", e))?;
                let measured = state.measure();
                Ok(Value::Int(measured as i64))
            }
            _ => Err(format!("Unknown quantum method: {}", method)),
        }
    }
}

fn bell_probabilities() -> Result<[f64; 4], String> {
    let mut state = QuantumState::new(2);
    state
        .apply_gate(&QuantumGate::H, 0)
        .map_err(|e| format!("{:?}", e))?;
    state
        .apply_controlled_gate(&QuantumGate::X, 0, 1)
        .map_err(|e| format!("{:?}", e))?;
    Ok([
        state.probability(0),
        state.probability(1),
        state.probability(2),
        state.probability(3),
    ])
}

fn sample_distribution(probabilities: &[f64; 4], rng: &mut StdRng) -> usize {
    let r: f64 = rng.gen();
    let mut cumulative = 0.0;
    for (index, probability) in probabilities.iter().enumerate() {
        cumulative += probability;
        if r <= cumulative {
            return index;
        }
    }
    probabilities.len() - 1
}

fn bell_stats(shots: usize, seed: u64) -> Value {
    let probabilities = bell_probabilities().unwrap_or([0.0, 0.0, 0.0, 0.0]);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut counts = [0usize; 4];
    for _ in 0..shots {
        let measured = sample_distribution(&probabilities, &mut rng);
        counts[measured] += 1;
    }

    let p00 = counts[0] as f64 / shots as f64;
    let p01 = counts[1] as f64 / shots as f64;
    let p10 = counts[2] as f64 / shots as f64;
    let p11 = counts[3] as f64 / shots as f64;
    let correlated_rate = p00 + p11;
    let forbidden_rate = p01 + p10;
    let balance_error = (p00 - p11).abs();
    let passed = correlated_rate >= 0.95 && forbidden_rate <= 0.05 && balance_error <= 0.10;

    let mut map = HashMap::new();
    map.insert("shots".to_string(), Value::Int(shots as i64));
    map.insert("seed".to_string(), Value::Int(seed as i64));
    map.insert("count_00".to_string(), Value::Int(counts[0] as i64));
    map.insert("count_01".to_string(), Value::Int(counts[1] as i64));
    map.insert("count_10".to_string(), Value::Int(counts[2] as i64));
    map.insert("count_11".to_string(), Value::Int(counts[3] as i64));
    map.insert("p00".to_string(), Value::Float(p00));
    map.insert("p01".to_string(), Value::Float(p01));
    map.insert("p10".to_string(), Value::Float(p10));
    map.insert("p11".to_string(), Value::Float(p11));
    map.insert("correlated_rate".to_string(), Value::Float(correlated_rate));
    map.insert("forbidden_rate".to_string(), Value::Float(forbidden_rate));
    map.insert("balance_error".to_string(), Value::Float(balance_error));
    map.insert("passed".to_string(), Value::Bool(passed));
    map.insert(
        "model".to_string(),
        Value::new_string("bell_state_state_vector_sampling".to_string()),
    );
    Value::new_map(map)
}

fn frontier_status(backend: &str, model: &str, mode: &str, capabilities: &[&str]) -> Value {
    let mut map = HashMap::new();
    map.insert(
        "backend".to_string(),
        Value::new_string(backend.to_string()),
    );
    map.insert("model".to_string(), Value::new_string(model.to_string()));
    map.insert("mode".to_string(), Value::new_string(mode.to_string()));
    map.insert("stub".to_string(), Value::Bool(false));
    map.insert("hardware".to_string(), Value::Bool(false));
    map.insert("simulated".to_string(), Value::Bool(true));
    map.insert(
        "capabilities".to_string(),
        Value::new_list(
            capabilities
                .iter()
                .map(|capability| Value::new_string((*capability).to_string()))
                .collect(),
        ),
    );
    Value::new_map(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bell_stats_is_seeded_and_correlated() {
        let stats = bell_stats(1000, 42);
        let Value::Map(map) = stats else {
            panic!("bell_stats should return a map");
        };

        assert_eq!(map.get("shots"), Some(&Value::Int(1000)));
        assert_eq!(map.get("seed"), Some(&Value::Int(42)));
        assert_eq!(map.get("passed"), Some(&Value::Bool(true)));

        let correlated = map
            .get("correlated_rate")
            .expect("correlated_rate")
            .as_float()
            .unwrap();
        let forbidden = map
            .get("forbidden_rate")
            .expect("forbidden_rate")
            .as_float()
            .unwrap();
        let balance = map
            .get("balance_error")
            .expect("balance_error")
            .as_float()
            .unwrap();

        assert!(correlated >= 0.95);
        assert!(forbidden <= 0.05);
        assert!(balance <= 0.10);
    }

    #[test]
    fn backend_exposes_bell_stats() {
        let mut backend = QuantumBackend::new();
        let stats = backend
            .call("bell_stats", vec![Value::Int(1000), Value::Int(42)])
            .unwrap();
        let Value::Map(map) = stats else {
            panic!("quantum.bell_stats should return a map");
        };
        assert_eq!(map.get("passed"), Some(&Value::Bool(true)));

        let status = backend.call("status", vec![]).unwrap();
        let Value::Map(status_map) = status else {
            panic!("quantum.status should return a map");
        };
        let capabilities = status_map.get("capabilities").expect("capabilities");
        let Value::List(items) = capabilities else {
            panic!("capabilities should be a list");
        };
        assert!(items
            .iter()
            .any(|item| { matches!(item, Value::String(text) if text.as_str() == "bell_stats") }));
    }
}
