//! Matter backend integration for relativity

use crate::*;
use matter_backend::{Backend, Value};

pub struct RelativityBackend {
    black_holes: Vec<SchwarzschildMetric>,
    kerr_black_holes: Vec<KerrMetric>,
}

impl RelativityBackend {
    pub fn new() -> Self {
        Self {
            black_holes: Vec::new(),
            kerr_black_holes: Vec::new(),
        }
    }
}

impl Default for RelativityBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for RelativityBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "lorentz_factor" => {
                let velocity = args
                    .first()
                    .ok_or("Expected velocity")?
                    .as_float()?;

                let gamma = special_relativity::lorentz_factor(velocity)
                    .map_err(|e| format!("{}", e))?;

                Ok(Value::Float(gamma))
            }

            "time_dilation" => {
                let proper_time = args
                    .first()
                    .ok_or("Expected proper time")?
                    .as_float()?;

                let velocity = args
                    .get(1)
                    .ok_or("Expected velocity")?
                    .as_float()?;

                let dilated = special_relativity::time_dilation(proper_time, velocity)
                    .map_err(|e| format!("{}", e))?;

                Ok(Value::Float(dilated))
            }

            "create_black_hole" => {
                let mass = args
                    .first()
                    .ok_or("Expected mass")?
                    .as_float()?;

                let bh = SchwarzschildMetric::new(mass);
                let id = self.black_holes.len();
                self.black_holes.push(bh);

                Ok(Value::Int(id as i64))
            }

            "schwarzschild_radius" => {
                let id = args
                    .first()
                    .ok_or("Expected black hole ID")?
                    .as_int()? as usize;

                let bh = self
                    .black_holes
                    .get(id)
                    .ok_or("Black hole not found")?;

                Ok(Value::Float(bh.schwarzschild_radius))
            }

            "escape_velocity" => {
                let id = args
                    .first()
                    .ok_or("Expected black hole ID")?
                    .as_int()? as usize;

                let r = args
                    .get(1)
                    .ok_or("Expected radius")?
                    .as_float()?;

                let bh = self
                    .black_holes
                    .get(id)
                    .ok_or("Black hole not found")?;

                let v_esc = bh.escape_velocity(r)
                    .map_err(|e| format!("{}", e))?;

                Ok(Value::Float(v_esc))
            }

            "photon_sphere" => {
                let id = args
                    .first()
                    .ok_or("Expected black hole ID")?
                    .as_int()? as usize;

                let bh = self
                    .black_holes
                    .get(id)
                    .ok_or("Black hole not found")?;

                Ok(Value::Float(bh.photon_sphere_radius()))
            }

            "isco" => {
                let id = args
                    .first()
                    .ok_or("Expected black hole ID")?
                    .as_int()? as usize;

                let bh = self
                    .black_holes
                    .get(id)
                    .ok_or("Black hole not found")?;

                Ok(Value::Float(bh.isco_radius()))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
