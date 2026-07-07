//! Matter backend for condensed matter physics

use crate::*;
use matter_backend::{Backend, Value};

pub struct CondensedBackend {
    simulator: CondensedSimulator,
}

impl CondensedBackend {
    pub fn new() -> Self {
        Self {
            simulator: CondensedSimulator::new(),
        }
    }
}

impl Default for CondensedBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for CondensedBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_crystal_fcc" => {
                let a = args.first().ok_or("Expected lattice constant")?.as_float()?;
                self.simulator.create_crystal(LatticeType::FCC, a);
                Ok(Value::Unit)
            }

            "create_crystal_bcc" => {
                let a = args.first().ok_or("Expected lattice constant")?.as_float()?;
                self.simulator.create_crystal(LatticeType::BCC, a);
                Ok(Value::Unit)
            }

            "create_bands" => {
                let e_f = args.first().ok_or("Expected Fermi energy")?.as_float()?;
                let e_g = args.get(1).ok_or("Expected band gap")?.as_float()?;
                self.simulator.create_bands(e_f, e_g);
                Ok(Value::Unit)
            }

            "create_conductor" => {
                let n = args.first().ok_or("Expected carrier density")?.as_float()?;
                let mu = args.get(1).ok_or("Expected mobility")?.as_float()?;
                self.simulator.create_conductor(n, mu);
                Ok(Value::Unit)
            }

            "atomic_density" => {
                let density = self
                    .simulator
                    .atomic_density()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(density))
            }

            "material_type" => {
                let mat_type = self
                    .simulator
                    .material_type()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::new_string(mat_type))
            }

            "conductivity" => {
                let sigma = self
                    .simulator
                    .conductivity()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(sigma))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
