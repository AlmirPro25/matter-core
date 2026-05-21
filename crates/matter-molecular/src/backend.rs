use crate::{DNAStrand, MolecularMemory};
use matter_backend::{Backend, Value};

pub struct MolecularBackend {
    memory: MolecularMemory,
}

impl MolecularBackend {
    pub fn new() -> Self {
        Self {
            memory: MolecularMemory::new(1024), // 1024 bytes DNA memory cell
        }
    }
}

impl Default for MolecularBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for MolecularBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "write" => {
                if args.is_empty() {
                    return Err("molecular.write: expected 1 argument (data: string)".to_string());
                }
                let data_str = args[0].as_string()?;

                // Clear previous cells to allow overwrite/fresh writes
                self.memory.cells.clear();

                self.memory.write(data_str.as_bytes())?;
                Ok(Value::Unit)
            }
            "read" => {
                let data = self.memory.read();
                let result_str = String::from_utf8(data)
                    .map_err(|e| format!("molecular.read: failed to decode UTF-8: {:?}", e))?;
                Ok(Value::new_string(result_str))
            }
            "hybridize" => {
                if args.len() < 2 {
                    return Err(
                        "molecular.hybridize: expected 2 arguments (seq1: string, seq2: string)"
                            .to_string(),
                    );
                }
                let seq1 = args[0].as_string()?;
                let seq2 = args[1].as_string()?;

                let strand1 = DNAStrand::new(&seq1).ok_or_else(|| {
                    format!("molecular.hybridize: invalid DNA sequence '{}'", seq1)
                })?;
                let strand2 = DNAStrand::new(&seq2).ok_or_else(|| {
                    format!("molecular.hybridize: invalid DNA sequence '{}'", seq2)
                })?;

                if strand1.bases.len() != strand2.bases.len() {
                    return Ok(Value::Bool(false));
                }

                let is_complement = strand1
                    .bases
                    .iter()
                    .zip(strand2.bases.iter())
                    .all(|(b1, b2)| b1.complement() == *b2);

                Ok(Value::Bool(is_complement))
            }
            _ => Err(format!("Unknown molecular method: {}", method)),
        }
    }
}
