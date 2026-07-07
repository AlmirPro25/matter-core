//! Backend integration for Matter Biophysics

use matter_backend::{Backend, Value};
use crate::*;

pub struct BiophysicsBackend;

impl Backend for BiophysicsBackend {
    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, String> {
        match function {
            // Membrane functions
            "membrane_new" => {
                let membrane = Membrane::new();
                Ok(Value::new_string(format!("Membrane(T={}K)", membrane.temperature)))
            }
            
            "nernst_potential" => {
                let ion_str = args.get(0).ok_or("Missing ion type")?.as_string()?;
                let ion = match ion_str.as_str() {
                    "Na" => IonType::Sodium,
                    "K" => IonType::Potassium,
                    "Ca" => IonType::Calcium,
                    "Cl" => IonType::Chloride,
                    _ => return Err(format!("Unknown ion: {}", ion_str)),
                };
                
                let membrane = Membrane::new();
                let e = membrane.nernst_potential(ion);
                Ok(Value::Float(e))
            }
            
            "ghk_potential" => {
                let p_na = args.get(0).ok_or("Missing p_na")?.as_float()?;
                let p_k = args.get(1).ok_or("Missing p_k")?.as_float()?;
                let p_cl = args.get(2).ok_or("Missing p_cl")?.as_float()?;
                
                let membrane = Membrane::new();
                let v = membrane.ghk_potential(p_na, p_k, p_cl);
                Ok(Value::Float(v))
            }
            
            // Hodgkin-Huxley functions
            "hh_new" => {
                let neuron = HodgkinHuxley::new();
                Ok(Value::new_string(format!("HH(V={:.1}mV)", neuron.v)))
            }
            
            "hh_step" => {
                let i_ext = args.get(0).ok_or("Missing i_ext")?.as_float()?;
                let dt = args.get(1).ok_or("Missing dt")?.as_float()?;
                
                let mut neuron = HodgkinHuxley::new();
                neuron.step(i_ext, dt);
                
                Ok(Value::Float(neuron.v))
            }
            
            "hh_action_potential" => {
                let i_ext = args.get(0).ok_or("Missing i_ext")?.as_float()?;
                let duration = args.get(1).ok_or("Missing duration")?.as_float()?;
                
                let mut neuron = HodgkinHuxley::new();
                let dt = 0.01;  // 0.01 ms
                let steps = (duration / dt) as usize;
                
                let mut max_v = neuron.v;
                for _ in 0..steps {
                    neuron.step(i_ext, dt);
                    if neuron.v > max_v {
                        max_v = neuron.v;
                    }
                }
                
                Ok(Value::Float(max_v))
            }
            
            // Enzyme functions
            "enzyme_new" => {
                let v_max = args.get(0).ok_or("Missing v_max")?.as_float()?;
                let k_m = args.get(1).ok_or("Missing k_m")?.as_float()?;
                
                let enzyme = Enzyme::new(v_max, k_m);
                Ok(Value::new_string(format!("Enzyme(Vmax={}, Km={})", v_max, k_m)))
            }
            
            "enzyme_rate" => {
                let v_max = args.get(0).ok_or("Missing v_max")?.as_float()?;
                let k_m = args.get(1).ok_or("Missing k_m")?.as_float()?;
                let substrate = args.get(2).ok_or("Missing substrate")?.as_float()?;
                
                let mut enzyme = Enzyme::new(v_max, k_m);
                enzyme.substrate = substrate;
                
                let rate = enzyme.reaction_rate();
                Ok(Value::Float(rate))
            }
            
            "enzyme_efficiency" => {
                let v_max = args.get(0).ok_or("Missing v_max")?.as_float()?;
                let k_m = args.get(1).ok_or("Missing k_m")?.as_float()?;
                
                let enzyme = Enzyme::new(v_max, k_m);
                let eff = enzyme.catalytic_efficiency();
                Ok(Value::Float(eff))
            }
            
            // Protein functions
            "protein_new" => {
                let length = args.get(0).ok_or("Missing length")?.as_int()? as usize;
                
                let protein = Protein::new(length);
                Ok(Value::new_string(format!("Protein(len={})", length)))
            }
            
            "protein_folding_prob" => {
                let length = args.get(0).ok_or("Missing length")?.as_int()? as usize;
                
                let protein = Protein::new(length);
                let p = protein.folding_probability();
                Ok(Value::Float(p))
            }
            
            "protein_folding_rate" => {
                let length = args.get(0).ok_or("Missing length")?.as_int()? as usize;
                
                let protein = Protein::new(length);
                let k = protein.folding_rate();
                Ok(Value::Float(k))
            }
            
            // DNA functions
            "dna_gc_content" => {
                let seq_str = args.get(0).ok_or("Missing sequence")?.as_string()?;
                
                let sequence: Vec<Base> = seq_str.chars().filter_map(|c| {
                    match c.to_ascii_uppercase() {
                        'A' => Some(Base::Adenine),
                        'T' => Some(Base::Thymine),
                        'G' => Some(Base::Guanine),
                        'C' => Some(Base::Cytosine),
                        _ => None,
                    }
                }).collect();
                
                let dna = DNA::new(sequence);
                let gc = dna.gc_content();
                Ok(Value::Float(gc))
            }
            
            "dna_melting_temp" => {
                let seq_str = args.get(0).ok_or("Missing sequence")?.as_string()?;
                
                let sequence: Vec<Base> = seq_str.chars().filter_map(|c| {
                    match c.to_ascii_uppercase() {
                        'A' => Some(Base::Adenine),
                        'T' => Some(Base::Thymine),
                        'G' => Some(Base::Guanine),
                        'C' => Some(Base::Cytosine),
                        _ => None,
                    }
                }).collect();
                
                let dna = DNA::new(sequence);
                let tm = dna.melting_temperature();
                Ok(Value::Float(tm))
            }
            
            "dna_stability" => {
                let seq_str = args.get(0).ok_or("Missing sequence")?.as_string()?;
                
                let sequence: Vec<Base> = seq_str.chars().filter_map(|c| {
                    match c.to_ascii_uppercase() {
                        'A' => Some(Base::Adenine),
                        'T' => Some(Base::Thymine),
                        'G' => Some(Base::Guanine),
                        'C' => Some(Base::Cytosine),
                        _ => None,
                    }
                }).collect();
                
                let dna = DNA::new(sequence);
                let stability = dna.stability();
                Ok(Value::Float(stability))
            }
            
            // Cell functions
            "cell_new" => {
                let radius = args.get(0).ok_or("Missing radius")?.as_float()?;
                
                let cell = Cell::new(radius);
                Ok(Value::new_string(format!("Cell(r={}μm)", radius)))
            }
            
            "cell_laplace_pressure" => {
                let radius = args.get(0).ok_or("Missing radius")?.as_float()?;
                
                let cell = Cell::new(radius);
                let p = cell.laplace_pressure();
                Ok(Value::Float(p))
            }
            
            "cell_volume" => {
                let radius = args.get(0).ok_or("Missing radius")?.as_float()?;
                
                let cell = Cell::new(radius);
                let v = cell.volume();
                Ok(Value::Float(v))
            }
            
            "cell_deformation" => {
                let radius = args.get(0).ok_or("Missing radius")?.as_float()?;
                let force = args.get(1).ok_or("Missing force")?.as_float()?;
                
                let cell = Cell::new(radius);
                let d = cell.deformation(force);
                Ok(Value::Float(d))
            }
            
            _ => Err(format!("Unknown biophysics function: {}", function)),
        }
    }
}
