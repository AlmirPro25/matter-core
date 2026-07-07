//! Matter backend for nuclear physics

use crate::*;
use matter_backend::{Backend, Value};

pub struct NuclearBackend {
    simulator: NuclearSimulator,
}

impl NuclearBackend {
    pub fn new() -> Self {
        Self {
            simulator: NuclearSimulator::new(),
        }
    }
}

impl Default for NuclearBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for NuclearBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_nucleus" => {
                let z = args.first().ok_or("Expected Z")?.as_int()? as u32;
                let n = args.get(1).ok_or("Expected N")?.as_int()? as u32;

                self.simulator.create_nucleus(z, n);
                Ok(Value::Unit)
            }

            "create_fission" => {
                self.simulator.create_fission();
                Ok(Value::Unit)
            }

            "create_fusion_dt" => {
                self.simulator.create_fusion_dt();
                Ok(Value::Unit)
            }

            "create_reactor" => {
                let fuel_mass = args.first().ok_or("Expected fuel mass")?.as_float()?;
                self.simulator.create_reactor(fuel_mass);
                Ok(Value::Unit)
            }

            "binding_energy" => {
                let be = self
                    .simulator
                    .binding_energy()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(be))
            }

            "fission_energy" => {
                let e = self
                    .simulator
                    .fission_energy()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(e))
            }

            "fusion_energy" => {
                let e = self
                    .simulator
                    .fusion_energy()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(e))
            }

            "reactor_power" => {
                let p = self
                    .simulator
                    .reactor_power()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(p))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
