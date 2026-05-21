use crate::{computation::DNAComputer, DNASequence, ProteinSequence, RNASequence};
use matter_backend::{Backend, Value};

pub struct BiologicalBackend;

impl BiologicalBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BiologicalBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for BiologicalBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "transcribe" => {
                if args.is_empty() {
                    return Err("biology.transcribe: esperado (dna_sequence)".to_string());
                }
                let dna_str = args[0].as_string()?;
                let dna = DNASequence::new(&dna_str).map_err(|e| e.to_string())?;
                let rna = dna.transcribe();
                Ok(Value::new_string(rna.to_string()))
            }
            "translate" => {
                if args.is_empty() {
                    return Err("biology.translate: esperado (rna_sequence)".to_string());
                }
                let rna_str = args[0].as_string()?;
                let rna = RNASequence::new(&rna_str).map_err(|e| e.to_string())?;
                let protein = rna.translate();
                Ok(Value::new_string(protein.to_string()))
            }
            "gc_content" => {
                if args.is_empty() {
                    return Err("biology.gc_content: esperado (dna_sequence)".to_string());
                }
                let dna_str = args[0].as_string()?;
                let dna = DNASequence::new(&dna_str).map_err(|e| e.to_string())?;
                Ok(Value::Float(dna.gc_content()))
            }
            "protein_weight" => {
                if args.is_empty() {
                    return Err("biology.protein_weight: esperado (protein_sequence)".to_string());
                }
                let protein_str = args[0].as_string()?;
                let protein = ProteinSequence::new(&protein_str).map_err(|e| e.to_string())?;
                Ok(Value::Float(protein.molecular_weight()))
            }
            "dna_hybridize" => {
                if args.is_empty() {
                    return Err("biology.dna_hybridize: esperado (list_of_dna_strands)".to_string());
                }

                let strands_val = match &args[0] {
                    Value::List(items) => items,
                    other => {
                        return Err(format!(
                            "biology.dna_hybridize: esperado lista, obteve {:?}",
                            other
                        ))
                    }
                };

                let mut comp = DNAComputer::new();

                for val in strands_val.iter() {
                    let dna_str = val.as_string()?;
                    let dna = DNASequence::new(&dna_str).map_err(|e| e.to_string())?;
                    comp.add_strand(dna);
                }

                let pairs = comp.hybridize();
                let mut result_list = Vec::new();
                for (i, j) in pairs {
                    let pair_vals = vec![Value::Int(i as i64), Value::Int(j as i64)];
                    result_list.push(Value::new_list(pair_vals));
                }

                Ok(Value::new_list(result_list))
            }
            "dna_ligate" => {
                if args.len() < 2 {
                    return Err("biology.dna_ligate: esperado (dna_a, dna_b)".to_string());
                }
                let dna_a_str = args[0].as_string()?;
                let dna_b_str = args[1].as_string()?;

                let dna_a = DNASequence::new(&dna_a_str).map_err(|e| e.to_string())?;
                let dna_b = DNASequence::new(&dna_b_str).map_err(|e| e.to_string())?;

                let mut comp = DNAComputer::new();
                comp.add_strand(dna_a);
                comp.add_strand(dna_b);

                let ligated = comp.ligate(0, 1).map_err(|e| e.to_string())?;
                Ok(Value::new_string(ligated.to_string()))
            }
            _ => Err(format!("Unknown biology method: {}", method)),
        }
    }
}
