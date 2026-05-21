use crate::{GeneralRelativity, QuantumGravityBridge, SpecialRelativity};
use matter_backend::{Backend, Value};
use std::collections::HashMap;

pub struct RelativityBackend;

impl RelativityBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RelativityBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for RelativityBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "lorentz_boost" => {
                if args.len() < 3 {
                    return Err("relativity.lorentz_boost: esperado v, x, t".to_string());
                }
                let v = args[0].as_float()?;
                let x = args[1].as_float()?;
                let t = args[2].as_float()?;

                let (x_p, t_p, gamma) = SpecialRelativity::lorentz_boost(v, x, t)?;
                let mut map = HashMap::new();
                map.insert("x_prime".to_string(), Value::Float(x_p));
                map.insert("t_prime".to_string(), Value::Float(t_p));
                map.insert("gamma".to_string(), Value::Float(gamma));
                Ok(Value::new_map(map))
            }
            "mass_energy" => {
                if args.len() < 2 {
                    return Err("relativity.mass_energy: esperado mass, velocity".to_string());
                }
                let mass = args[0].as_float()?;
                let velocity = args[1].as_float()?;

                let map_res = SpecialRelativity::mass_energy(mass, velocity)?;
                let mut map = HashMap::new();
                for (k, v) in map_res {
                    map.insert(k, Value::Float(v));
                }
                Ok(Value::new_map(map))
            }
            "gravitational_dilation" => {
                if args.len() < 3 {
                    return Err(
                        "relativity.gravitational_dilation: esperado mass, radius, coordinate_time"
                            .to_string(),
                    );
                }
                let mass = args[0].as_float()?;
                let radius = args[1].as_float()?;
                let coord_time = args[2].as_float()?;

                let dilated =
                    GeneralRelativity::gravitational_time_dilation(mass, radius, coord_time)?;
                Ok(Value::Float(dilated))
            }
            "geodesic_step" => {
                if args.len() < 6 {
                    return Err(
                        "relativity.geodesic_step: esperado mass, x, y, vx, vy, dt".to_string()
                    );
                }
                let mass = args[0].as_float()?;
                let x = args[1].as_float()?;
                let y = args[2].as_float()?;
                let vx = args[3].as_float()?;
                let vy = args[4].as_float()?;
                let dt = args[5].as_float()?;

                let (nx, ny, nvx, nvy) = GeneralRelativity::geodesic_step(mass, x, y, vx, vy, dt)?;
                let mut map = HashMap::new();
                map.insert("x".to_string(), Value::Float(nx));
                map.insert("y".to_string(), Value::Float(ny));
                map.insert("vx".to_string(), Value::Float(nvx));
                map.insert("vy".to_string(), Value::Float(nvy));
                Ok(Value::new_map(map))
            }
            "quantum_decoherence" => {
                if args.len() < 4 {
                    return Err("relativity.quantum_decoherence: esperado coherence_time, mass, radius, alpha".to_string());
                }
                let coherence = args[0].as_float()?;
                let mass = args[1].as_float()?;
                let radius = args[2].as_float()?;
                let alpha = args[3].as_float()?;

                let final_coherence = QuantumGravityBridge::gravitational_decoherence(
                    coherence, mass, radius, alpha,
                )?;
                Ok(Value::Float(final_coherence))
            }
            _ => Err(format!("Unknown relativity method: {}", method)),
        }
    }
}
