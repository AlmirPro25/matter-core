//! Matter backend for astrophysics

use crate::*;
use matter_backend::{Backend, Value};

pub struct AstroBackend {
    simulator: AstroSimulator,
}

impl AstroBackend {
    pub fn new() -> Self {
        Self {
            simulator: AstroSimulator::new(),
        }
    }
}

impl Default for AstroBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for AstroBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_star" => {
                let mass = args.first().ok_or("Expected mass")?.as_float()?;
                self.simulator.create_star(mass);
                Ok(Value::Unit)
            }

            "create_supernova" => {
                let mass = args.first().ok_or("Expected mass")?.as_float()?;
                self.simulator.create_supernova(mass);
                Ok(Value::Unit)
            }

            "create_black_hole" => {
                let mass = args.first().ok_or("Expected mass")?.as_float()?;
                let spin = args.get(1).ok_or("Expected spin")?.as_float()?;
                self.simulator.create_black_hole(mass, spin);
                Ok(Value::Unit)
            }

            "create_neutron_star" => {
                let mass = args.first().ok_or("Expected mass")?.as_float()?;
                self.simulator.create_neutron_star(mass);
                Ok(Value::Unit)
            }

            "star_luminosity" => {
                let lum = self
                    .simulator
                    .star_luminosity()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(lum))
            }

            "star_lifetime" => {
                let lifetime = self
                    .simulator
                    .star_lifetime()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(lifetime))
            }

            "supernova_forms_black_hole" => {
                let forms_bh = self
                    .simulator
                    .supernova_forms_black_hole()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Bool(forms_bh))
            }

            "black_hole_hawking_temp" => {
                let temp = self
                    .simulator
                    .black_hole_hawking_temp()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(temp))
            }

            "neutron_star_gravity" => {
                let g = self
                    .simulator
                    .neutron_star_gravity()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(g))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
