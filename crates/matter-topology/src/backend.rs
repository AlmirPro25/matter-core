//! Matter backend for topological materials

use crate::*;
use matter_backend::{Backend, Value};

pub struct TopologyBackend {
    simulator: TopologySimulator,
}

impl TopologyBackend {
    pub fn new() -> Self {
        Self {
            simulator: TopologySimulator::new(),
        }
    }
}

impl Default for TopologyBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for TopologyBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_quantum_hall" => {
                let b_field = args.first().ok_or("Expected B field")?.as_float()?;
                let filling = args.get(1).ok_or("Expected filling")?.as_float()?;

                self.simulator.create_quantum_hall(b_field, filling);
                Ok(Value::Unit)
            }

            "create_topological_insulator" => {
                let dim = args.first().ok_or("Expected dimension")?.as_int()? as usize;
                let gap = args.get(1).ok_or("Expected gap")?.as_float()?;
                let soc = args.get(2).ok_or("Expected SOC")?.as_float()?;

                self.simulator.create_topological_insulator(dim, gap, soc);
                Ok(Value::Unit)
            }

            "create_majorana" => {
                let gap = args.first().ok_or("Expected gap")?.as_float()?;
                let mu = args.get(1).ok_or("Expected mu")?.as_float()?;
                let zeeman = args.get(2).ok_or("Expected Zeeman")?.as_float()?;

                self.simulator.create_majorana(gap, mu, zeeman);
                Ok(Value::Unit)
            }

            "hall_conductance" => {
                let sigma = self
                    .simulator
                    .hall_conductance()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Float(sigma))
            }

            "chern_number" => {
                let c = self.simulator.chern_number().map_err(|e| format!("{}", e))?;
                Ok(Value::Int(c as i64))
            }

            "is_topological" => {
                let is_topo = self
                    .simulator
                    .is_topological()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Bool(is_topo))
            }

            "majorana_is_topological" => {
                let is_topo = self
                    .simulator
                    .majorana_is_topological()
                    .map_err(|e| format!("{}", e))?;
                Ok(Value::Bool(is_topo))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
