use crate::{MagneticTunnelJunction, SpinGate, SpinGateType, SpinState};
use matter_backend::{Backend, Value};
use std::collections::HashMap;

pub struct SpintronicsBackend {
    registers: HashMap<usize, MagneticTunnelJunction>,
    gate_ops: u64,
    total_energy_fj: f64,
    total_switching_ps: f64,
}

impl SpintronicsBackend {
    pub fn new() -> Self {
        Self {
            registers: HashMap::new(),
            gate_ops: 0,
            total_energy_fj: 0.0,
            total_switching_ps: 0.0,
        }
    }
}

impl Default for SpintronicsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for SpintronicsBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "write" => {
                if args.len() < 2 {
                    return Err(
                        "spintronics.write: expected 2 arguments (address: int, bit: int)"
                            .to_string(),
                    );
                }
                let address = args[0].as_int()? as usize;
                let bit = args[1].as_int()?;

                let spin = if bit != 0 {
                    SpinState::Up
                } else {
                    SpinState::Down
                };
                let mtj = self.registers.entry(address).or_default();
                mtj.write_stt(spin);

                // Writing spintronics consumes energy and switching time!
                self.gate_ops += 1;
                self.total_energy_fj += 0.5; // 0.5 fJ for write
                self.total_switching_ps += 100.0; // 100 ps for switching

                Ok(Value::Unit)
            }
            "read" => {
                if args.is_empty() {
                    return Err("spintronics.read: expected 1 argument (address: int)".to_string());
                }
                let address = args[0].as_int()? as usize;
                let mtj = self.registers.entry(address).or_default();
                let (_parallel, _resistance) = mtj.read();

                // free_layer state determines measured bit
                let val = match mtj.free_layer.measure() {
                    SpinState::Up => 1,
                    SpinState::Down => 0,
                    _ => 0,
                };

                // Reading consumes minimal energy
                self.gate_ops += 1;
                self.total_energy_fj += 0.1; // 0.1 fJ read
                self.total_switching_ps += 50.0; // 50 ps read

                Ok(Value::Int(val))
            }
            "gate" => {
                if args.len() < 3 {
                    return Err("spintronics.gate: expected 3 arguments (gate_name: string, in1_up: bool, in2_up: bool)".to_string());
                }
                let gate_name = args[0].as_string()?;
                let in1_up = args[1].as_bool()?;
                let in2_up = args[2].as_bool()?;

                let spin1 = if in1_up {
                    SpinState::Up
                } else {
                    SpinState::Down
                };
                let spin2 = if in2_up {
                    SpinState::Up
                } else {
                    SpinState::Down
                };

                let gate_type = match gate_name.to_uppercase().as_str() {
                    "AND" => SpinGateType::AND,
                    "OR" => SpinGateType::OR,
                    "XOR" => SpinGateType::XOR,
                    "NAND" => SpinGateType::NAND,
                    "NOR" => SpinGateType::NOR,
                    "XNOR" => SpinGateType::XNOR,
                    _ => return Err(format!("Unsupported spintronics gate: {}", gate_name)),
                };

                let gate = SpinGate::new(gate_type);
                let result = gate.execute(&[spin1, spin2]);
                let is_up = matches!(result, SpinState::Up);

                self.gate_ops += 1;
                self.total_energy_fj += gate.power_consumption;
                self.total_switching_ps += gate.switching_time;

                Ok(Value::Bool(is_up))
            }
            "stats" => {
                let mut map = HashMap::new();
                map.insert(
                    "gate_operations".to_string(),
                    Value::Int(self.gate_ops as i64),
                );
                map.insert(
                    "energy_consumption_fj".to_string(),
                    Value::Float(self.total_energy_fj),
                );
                map.insert(
                    "switching_latency_ps".to_string(),
                    Value::Float(self.total_switching_ps),
                );
                Ok(Value::new_map(map))
            }
            _ => Err(format!("Unknown spintronics method: {}", method)),
        }
    }
}
