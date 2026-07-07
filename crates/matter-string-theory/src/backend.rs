//! Matter backend integration for string theory

use crate::*;
use matter_backend::{Backend, Value};

pub struct StringTheoryBackend {
    strings: Vec<StringState>,
    manifold: Option<CalabiYauManifold>,
    branes: Vec<DBrane>,
}

impl StringTheoryBackend {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            manifold: None,
            branes: Vec::new(),
        }
    }
}

impl Default for StringTheoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for StringTheoryBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "create_string" => {
                let theory_str = args
                    .first()
                    .and_then(|v| v.as_string())
                    .ok_or("Expected theory type")?;

                let theory = match theory_str.as_str() {
                    "TypeIIA" => StringTheoryType::TypeIIA,
                    "TypeIIB" => StringTheoryType::TypeIIB,
                    "MTheory" => StringTheoryType::MTheory,
                    _ => return Err(format!("Unknown theory: {}", theory_str)),
                };

                let state = StringState::ground_state(theory)
                    .map_err(|e| format!("Failed to create string: {}", e))?;

                let id = self.strings.len();
                self.strings.push(state);

                Ok(Value::Int(id as i64))
            }

            "string_mass" => {
                let id = args
                    .first()
                    .and_then(|v| v.as_int())
                    .ok_or("Expected string ID")? as usize;

                let state = self
                    .strings
                    .get(id)
                    .ok_or("String not found")?;

                Ok(Value::Float(state.mass()))
            }

            "excite_string" => {
                let id = args
                    .first()
                    .and_then(|v| v.as_int())
                    .ok_or("Expected string ID")? as usize;

                let level = args
                    .get(1)
                    .and_then(|v| v.as_int())
                    .ok_or("Expected level")? as usize;

                let state = self
                    .strings
                    .get_mut(id)
                    .ok_or("String not found")?;

                state.excite_mode(level, 0, Complex64::new(1.0, 0.0));

                Ok(Value::Unit)
            }

            "create_calabi_yau" => {
                let topology = args
                    .first()
                    .and_then(|v| v.as_string())
                    .ok_or("Expected topology")?;

                let manifold = match topology.as_str() {
                    "K3" => CalabiYauManifold::k3(),
                    "Quintic" => CalabiYauManifold::quintic(),
                    _ => return Err(format!("Unknown topology: {}", topology)),
                };

                self.manifold = Some(manifold);

                Ok(Value::Unit)
            }

            "generations" => {
                let manifold = self
                    .manifold
                    .as_ref()
                    .ok_or("No Calabi-Yau manifold created")?;

                Ok(Value::Int(manifold.generations() as i64))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
