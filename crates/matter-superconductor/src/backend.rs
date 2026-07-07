//! Matter backend for superconductors

use crate::*;
use matter_backend::{Backend, Value};

pub struct SuperconductorBackend {
    simulator: SuperconductorSimulator,
}

impl SuperconductorBackend {
    pub fn new() -> Self {
        Self {
            simulator: SuperconductorSimulator::new(),
        }
    }
}

impl Default for SuperconductorBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for SuperconductorBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_bcs" => {
                let tc = args.first().ok_or("Expected Tc")?.as_float()?;
                let xi = args.get(1).ok_or("Expected xi")?.as_float()?;
                let lambda = args.get(2).ok_or("Expected lambda")?.as_float()?;

                self.simulator.create_bcs(tc, xi, lambda);
                Ok(Value::Unit)
            }

            "create_junction" => {
                let ic = args.first().ok_or("Expected Ic")?.as_float()?;
                let c = args.get(1).ok_or("Expected C")?.as_float()?;
                let r = args.get(2).ok_or("Expected R")?.as_float()?;

                self.simulator.create_junction(ic, c, r);
                Ok(Value::Unit)
            }

            "create_transmon" => {
                let ej = args.first().ok_or("Expected EJ")?.as_float()?;
                let ec = args.get(1).ok_or("Expected EC")?.as_float()?;

                self.simulator.create_transmon(ej, ec);
                Ok(Value::Unit)
            }

            "create_squid" => {
                let ic1 = args.first().ok_or("Expected Ic1")?.as_float()?;
                let ic2 = args.get(1).ok_or("Expected Ic2")?.as_float()?;
                let l = args.get(2).ok_or("Expected L")?.as_float()?;

                self.simulator.create_squid(ic1, ic2, l);
                Ok(Value::Unit)
            }

            "gap" => {
                let temp = args.first().ok_or("Expected temperature")?.as_float()?;
                let gap = self.simulator.gap(temp).map_err(|e| format!("{}", e))?;
                Ok(Value::Float(gap))
            }

            "junction_current" => {
                let current = self
                    .simulator
                    .junction_current()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(current))
            }

            "transmon_frequency" => {
                let freq = self
                    .simulator
                    .transmon_frequency()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(freq))
            }

            "squid_critical_current" => {
                let flux = args.first().ok_or("Expected flux")?.as_float()?;
                let ic = self
                    .simulator
                    .squid_critical_current(flux)
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(ic))
            }

            "set_junction_phase" => {
                let phase = args.first().ok_or("Expected phase")?.as_float()?;
                if let Some(junction) = &mut self.simulator.junction {
                    junction.set_phase(phase);
                    Ok(Value::Unit)
                } else {
                    Err("No junction".to_string())
                }
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
