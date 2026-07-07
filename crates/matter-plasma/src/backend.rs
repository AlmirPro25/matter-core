//! Matter backend for plasma physics

use crate::*;
use matter_backend::{Backend, Value};

pub struct PlasmaBackend {
    simulator: PlasmaSimulator,
}

impl PlasmaBackend {
    pub fn new() -> Self {
        Self {
            simulator: PlasmaSimulator::new(),
        }
    }
}

impl Default for PlasmaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for PlasmaBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_plasma" => {
                let temp = args.first().ok_or("Expected temperature")?.as_float()?;
                let density = args.get(1).ok_or("Expected density")?.as_float()?;
                let b_field = args.get(2).ok_or("Expected B field")?.as_float()?;

                self.simulator.create_plasma(temp, density, b_field);
                Ok(Value::Unit)
            }

            "create_tokamak" => {
                let r_major = args.first().ok_or("Expected R_major")?.as_float()?;
                let r_minor = args.get(1).ok_or("Expected R_minor")?.as_float()?;
                let temp = args.get(2).ok_or("Expected temperature")?.as_float()?;
                let density = args.get(3).ok_or("Expected density")?.as_float()?;
                let b_field = args.get(4).ok_or("Expected B field")?.as_float()?;

                self.simulator
                    .create_tokamak(r_major, r_minor, temp, density, b_field);
                Ok(Value::Unit)
            }

            "create_icf" => {
                let radius = args.first().ok_or("Expected radius")?.as_float()?;
                let energy = args.get(1).ok_or("Expected energy")?.as_float()?;

                self.simulator.create_icf(radius, energy);
                Ok(Value::Unit)
            }

            "debye_length" => {
                let lambda = self
                    .simulator
                    .debye_length()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(lambda))
            }

            "plasma_frequency" => {
                let omega = self
                    .simulator
                    .plasma_frequency()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(omega))
            }

            "tokamak_q_factor" => {
                let heating = args.first().ok_or("Expected heating power")?.as_float()?;
                let tau = args.get(1).ok_or("Expected confinement time")?.as_float()?;

                let q = self
                    .simulator
                    .tokamak_q_factor(heating, tau)
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(q))
            }

            "tokamak_is_ignition" => {
                let heating = args.first().ok_or("Expected heating power")?.as_float()?;
                let tau = args.get(1).ok_or("Expected confinement time")?.as_float()?;

                let ignition = self
                    .simulator
                    .tokamak_is_ignition(heating, tau)
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Bool(ignition))
            }

            "icf_can_ignite" => {
                let can_ignite = self
                    .simulator
                    .icf_can_ignite()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Bool(can_ignite))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
