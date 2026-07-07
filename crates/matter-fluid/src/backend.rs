//! Matter backend for fluid dynamics

use crate::*;
use matter_backend::{Backend, Value};

pub struct FluidBackend {
    simulator: FluidSimulator,
}

impl FluidBackend {
    pub fn new() -> Self {
        Self {
            simulator: FluidSimulator::new(),
        }
    }
}

impl Default for FluidBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for FluidBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_water" => {
                self.simulator.create_water();
                Ok(Value::Unit)
            }

            "create_air" => {
                self.simulator.create_air();
                Ok(Value::Unit)
            }

            "create_flow" => {
                let v = args.first().ok_or("Expected velocity")?.as_float()?;
                let l = args.get(1).ok_or("Expected length")?.as_float()?;
                self.simulator.create_flow(v, l);
                Ok(Value::Unit)
            }

            "create_pipe" => {
                let v = args.first().ok_or("Expected velocity")?.as_float()?;
                let d = args.get(1).ok_or("Expected diameter")?.as_float()?;
                let l = args.get(2).ok_or("Expected length")?.as_float()?;
                self.simulator.create_pipe(v, d, l);
                Ok(Value::Unit)
            }

            "create_airfoil" => {
                let v = args.first().ok_or("Expected velocity")?.as_float()?;
                let chord = args.get(1).ok_or("Expected chord")?.as_float()?;
                let alpha = args.get(2).ok_or("Expected angle")?.as_float()?;
                self.simulator.create_airfoil(v, chord, alpha);
                Ok(Value::Unit)
            }

            "create_shock" => {
                let mach = args.first().ok_or("Expected Mach")?.as_float()?;
                self.simulator.create_shock(mach);
                Ok(Value::Unit)
            }

            "reynolds_number" => {
                let re = self
                    .simulator
                    .reynolds_number()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(re))
            }

            "pipe_pressure_drop" => {
                let dp = self
                    .simulator
                    .pipe_pressure_drop()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(dp))
            }

            "airfoil_lift" => {
                let span = args.first().ok_or("Expected span")?.as_float()?;
                let lift = self
                    .simulator
                    .airfoil_lift(span)
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(lift))
            }

            "shock_pressure_ratio" => {
                let ratio = self
                    .simulator
                    .shock_pressure_ratio()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(ratio))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
