//! Matter backend for nanomaterials

use crate::*;
use matter_backend::{Backend, Value};

pub struct NanoBackend {
    simulator: NanoSimulator,
}

impl NanoBackend {
    pub fn new() -> Self {
        Self {
            simulator: NanoSimulator::new(),
        }
    }
}

impl Default for NanoBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for NanoBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_graphene" => {
                let nx = args.first().ok_or("Expected nx")?.as_int()? as usize;
                let ny = args.get(1).ok_or("Expected ny")?.as_int()? as usize;

                self.simulator.create_graphene(nx, ny);
                Ok(Value::Unit)
            }

            "create_nanotube" => {
                let n = args.first().ok_or("Expected n")?.as_int()? as i32;
                let m = args.get(1).ok_or("Expected m")?.as_int()? as i32;
                let length = args.get(2).ok_or("Expected length")?.as_float()?;

                self.simulator.create_nanotube(n, m, length);
                Ok(Value::Unit)
            }

            "create_quantum_dot" => {
                let radius = args.first().ok_or("Expected radius")?.as_float()?;
                let material = args.get(1).ok_or("Expected material")?.as_string()?;
                let n_electrons = args.get(2).ok_or("Expected n_electrons")?.as_int()? as usize;

                self.simulator
                    .create_quantum_dot(radius, material, n_electrons);
                Ok(Value::Unit)
            }

            "graphene_fermi_velocity" => {
                let vf = self
                    .simulator
                    .graphene_fermi_velocity()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(vf))
            }

            "nanotube_conductance" => {
                let g = self
                    .simulator
                    .nanotube_conductance()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(g))
            }

            "nanotube_is_metallic" => {
                let is_metallic = self
                    .simulator
                    .nanotube
                    .as_ref()
                    .map(|nt| nt.is_metallic())
                    .ok_or("No nanotube")?;
                Ok(Value::Bool(is_metallic))
            }

            "quantum_dot_wavelength" => {
                let wl = self
                    .simulator
                    .quantum_dot_wavelength()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(wl))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
