use crate::GenesisEngine;
use matter_backend::{Backend, Value};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct GenesisBackend {
    engine: Mutex<GenesisEngine>,
}

impl GenesisBackend {
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(GenesisEngine::new()),
        }
    }
}

impl Default for GenesisBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for GenesisBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        let mut engine = self.engine.lock().map_err(|e| e.to_string())?;

        match method {
            "create_particle" => {
                if args.len() < 5 {
                    return Err(
                        "genesis.create_particle: esperado (id, mass, charge, x0, p0)".to_string(),
                    );
                }
                let id = args[0].as_string()?;
                let mass = args[1].as_float()?;
                let charge = args[2].as_float()?;
                let x0 = args[3].as_float()?;
                let p0 = args[4].as_float()?;

                engine.create_particle(id, mass, charge, x0, p0);
                Ok(Value::Unit)
            }
            "evolve" => {
                if args.len() < 4 {
                    return Err(
                        "genesis.evolve: esperado (id, steps, dt, gravity_mass)".to_string()
                    );
                }
                let id = args[0].as_string()?;
                let steps = args[1].as_int()? as usize;
                let dt = args[2].as_float()?;
                let gravity_mass = args[3].as_float()?;

                let particle = engine
                    .particles
                    .get_mut(&id)
                    .ok_or_else(|| format!("Partícula '{}' não encontrada", id))?;

                for _ in 0..steps {
                    particle.evolve_step(dt, gravity_mass);
                }

                Ok(Value::Unit)
            }
            "measure" => {
                if args.is_empty() {
                    return Err("genesis.measure: esperado (id)".to_string());
                }
                let id = args[0].as_string()?;

                let particle = engine
                    .particles
                    .get_mut(&id)
                    .ok_or_else(|| format!("Partícula '{}' não encontrada", id))?;

                let pos = particle.measure();
                Ok(Value::Float(pos))
            }
            "measure_relative" => {
                if args.len() < 2 {
                    return Err("genesis.measure_relative: esperado (id, v_obs)".to_string());
                }
                let id = args[0].as_string()?;
                let v_obs = args[1].as_float()?;

                let particle = engine
                    .particles
                    .get_mut(&id)
                    .ok_or_else(|| format!("Partícula '{}' não encontrada", id))?;

                let pos = particle.measure_relative(v_obs);
                Ok(Value::Float(pos))
            }
            "entangle" => {
                if args.len() < 2 {
                    return Err("genesis.entangle: esperado (id_a, id_b)".to_string());
                }
                let id_a = args[0].as_string()?;
                let id_b = args[1].as_string()?;

                engine.entangle(id_a, id_b);
                Ok(Value::Unit)
            }
            "measure_entangled" => {
                if args.len() < 3 {
                    return Err(
                        "genesis.measure_entangled: esperado (id_a, id_b, v_obs)".to_string()
                    );
                }
                let id_a = args[0].as_string()?;
                let id_b = args[1].as_string()?;
                let v_obs = args[2].as_float()?;

                let p_a_ptr = engine
                    .particles
                    .get_mut(&id_a)
                    .ok_or_else(|| format!("Partícula '{}' não encontrada", id_a))?
                    as *mut crate::GenesisParticle;
                let p_b_ptr = engine
                    .particles
                    .get_mut(&id_b)
                    .ok_or_else(|| format!("Partícula '{}' não encontrada", id_b))?
                    as *mut crate::GenesisParticle;

                if p_a_ptr == p_b_ptr {
                    return Err(
                        "genesis.measure_entangled: id_a e id_b não podem ser iguais".to_string(),
                    );
                }

                let (obs_a, obs_b) =
                    unsafe { (&mut *p_a_ptr).measure_entangled(&mut *p_b_ptr, v_obs) };

                let list = vec![Value::Float(obs_a), Value::Float(obs_b)];
                Ok(Value::new_list(list))
            }
            "overlap" => {
                if args.len() < 2 {
                    return Err("genesis.overlap: esperado (id_a, id_b)".to_string());
                }
                let id_a = args[0].as_string()?;
                let id_b = args[1].as_string()?;

                let p_a = engine
                    .particles
                    .get(&id_a)
                    .ok_or_else(|| format!("Partícula '{}' não encontrada", id_a))?;
                let p_b = engine
                    .particles
                    .get(&id_b)
                    .ok_or_else(|| format!("Partícula '{}' não encontrada", id_b))?;

                let ov = p_a.overlap(p_b);
                Ok(Value::Float(ov))
            }
            "particle_info" => {
                if args.is_empty() {
                    return Err("genesis.particle_info: esperado (id)".to_string());
                }
                let id = args[0].as_string()?;

                let p = engine
                    .particles
                    .get(&id)
                    .ok_or_else(|| format!("Partícula '{}' não encontrada", id))?;

                let mut map = HashMap::new();
                map.insert("id".to_string(), Value::new_string(p.id.clone()));
                map.insert("rest_mass".to_string(), Value::Float(p.rest_mass));
                map.insert("charge".to_string(), Value::Float(p.charge));
                map.insert(
                    "expectation_x".to_string(),
                    Value::Float(p.expectation_position()),
                );

                let pos_list = p.pos_4.iter().map(|&v| Value::Float(v)).collect::<Vec<_>>();
                map.insert("pos_4".to_string(), Value::new_list(pos_list));

                let mom_list = p.mom_4.iter().map(|&v| Value::Float(v)).collect::<Vec<_>>();
                map.insert("mom_4".to_string(), Value::new_list(mom_list));

                // Expondo a magnitude local da função de onda (densidade) para fins de plot/vis
                let mut density_list = Vec::new();
                for psi in &p.wavefunction {
                    density_list.push(Value::Float(psi.norm_sq()));
                }
                map.insert("wave_density".to_string(), Value::new_list(density_list));

                Ok(Value::new_map(map))
            }
            _ => Err(format!("Unknown genesis method: {}", method)),
        }
    }
}
