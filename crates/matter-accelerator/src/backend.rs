//! Matter backend for particle accelerators

use crate::*;
use matter_backend::{Backend, Value};

pub struct AcceleratorBackend {
    simulator: AcceleratorSimulator,
}

impl AcceleratorBackend {
    pub fn new() -> Self {
        Self {
            simulator: AcceleratorSimulator::new(),
        }
    }
}

impl Default for AcceleratorBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for AcceleratorBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_proton_beam" => {
                let energy = args.first().ok_or("Expected energy")?.as_float()?;
                self.simulator.create_proton_beam(energy);
                Ok(Value::Unit)
            }

            "create_electron_beam" => {
                let energy = args.first().ok_or("Expected energy")?.as_float()?;
                self.simulator.create_electron_beam(energy);
                Ok(Value::Unit)
            }

            "create_lhc" => {
                self.simulator.create_lhc();
                Ok(Value::Unit)
            }

            "beam_velocity" => {
                let v = self
                    .simulator
                    .beam_velocity()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(v))
            }

            "beam_lorentz_factor" => {
                let gamma = self
                    .simulator
                    .beam_lorentz_factor()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(gamma))
            }

            "lhc_collision_energy" => {
                let e = self
                    .simulator
                    .lhc_collision_energy()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(e))
            }

            "lhc_higgs_per_year" => {
                let n = self
                    .simulator
                    .lhc_higgs_per_year()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(n))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
