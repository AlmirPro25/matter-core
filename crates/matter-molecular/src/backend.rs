//! Matter backend for molecular dynamics

use crate::*;
use matter_backend::{Backend, Value};

pub struct MolecularBackend {
    simulations: Vec<MDSimulation>,
    folders: Vec<ProteinFolder>,
}

impl MolecularBackend {
    pub fn new() -> Self {
        Self {
            simulations: Vec::new(),
            folders: Vec::new(),
        }
    }
}

impl Default for MolecularBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for MolecularBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> std::result::Result<Value, String> {
        match method {
            "fold_protein" => {
                let seq = args
                    .first()
                    .ok_or("Expected sequence")?
                    .as_string()?;

                let mut folder = ProteinFolder::new(seq);
                folder.fold().map_err(|e| format!("{}", e))?;

                let id = self.folders.len();
                self.folders.push(folder);

                Ok(Value::Int(id as i64))
            }

            "optimize_protein" => {
                let id = args
                    .first()
                    .ok_or("Expected folder ID")?
                    .as_int()? as usize;

                let steps = args
                    .get(1)
                    .map(|v| v.as_int().unwrap_or(1000))
                    .unwrap_or(1000) as usize;

                let folder = self.folders.get_mut(id).ok_or("Folder not found")?;

                folder.optimize(steps).map_err(|e| format!("{}", e))?;

                Ok(Value::Unit)
            }

            "protein_atoms" => {
                let id = args
                    .first()
                    .ok_or("Expected folder ID")?
                    .as_int()? as usize;

                let folder = self.folders.get(id).ok_or("Folder not found")?;

                let n_atoms = folder
                    .structure
                    .as_ref()
                    .map(|m| m.atoms.len())
                    .unwrap_or(0);

                Ok(Value::Int(n_atoms as i64))
            }

            _ => Err(format!("Unknown method: {}", method)),
        }
    }
}
