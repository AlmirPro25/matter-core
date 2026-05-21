use crate::{QuantumGate, QuantumState};
use matter_backend::{Backend, Value};

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
