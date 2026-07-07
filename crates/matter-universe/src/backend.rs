//! Matter backend integration for universe simulation

use crate::*;
use matter_backend::{Backend, Value};

pub struct UniverseBackend {
    simulations: Vec<NBodySimulation>,
}

impl UniverseBackend {
    pub fn new() -> Self {
        Self {
            simulations: Vec::new(),
        }
    }
}

impl Default for UniverseBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for UniverseBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_universe" => {
                let sim = big_bang_initial_conditions();
                let id = self.simulations.len();
                self.simulations.push(sim);
                Ok(Value::Int(id as i64))
            }

            "hubble_constant" => {
                let params = CosmologicalParameters::planck_2018();
                Ok(Value::Float(params.h0))
            }

            "dark_energy" => {
                let params = CosmologicalParameters::planck_2018();
                Ok(Value::Float(params.omega_lambda))
            }

            "dark_matter" => {
                let params = CosmologicalParameters::planck_2018();
                Ok(Value::Float(params.omega_dark_matter))
            }

            "evolve_universe" => {
                let id = args
                    .first()
                    .and_then(|v| v.as_int())
                    .ok_or("Expected simulation ID")? as usize;

                let dt = args
                    .get(1)
                    .and_then(|v| v.as_float())
                    .ok_or("Expected timestep")?;

                let sim = self
                    .simulations
                    .get_mut(id)
                    .ok_or("Simulation not found")?;

                sim.evolve(dt)
                    .map_err(|e| format!("{}", e))?;

                Ok(Value::Unit)
            }

            "total_energy" => {
                let id = args
                    .first()
                    .and_then(|v| v.as_int())
                    .ok_or("Expected simulation ID")? as usize;

                let sim = self
                    .simulations
                    .get(id)
                    .ok_or("Simulation not found")?;

                Ok(Value::Float(sim.total_energy()))
            }

            "particle_count" => {
                let id = args
                    .first()
                    .and_then(|v| v.as_int())
                    .ok_or("Expected simulation ID")? as usize;

                let sim = self
                    .simulations
                    .get(id)
                    .ok_or("Simulation not found")?;

                Ok(Value::Int(sim.particles.len() as i64))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
