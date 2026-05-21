use crate::{
    balance_reaction, get_element_db, ground_state_configuration, last_electron_quantum_numbers,
    molecular_mass_and_composition, predict_bond, slater_z_effective,
};
use matter_backend::{Backend, Value};
use std::collections::HashMap;

pub struct ChemistryBackend;

impl ChemistryBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ChemistryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for ChemistryBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "element_info" => {
                if args.is_empty() {
                    return Err(
                        "chemistry.element_info: esperado símbolo (string) ou número atômico (int)"
                            .to_string(),
                    );
                }

                let db = get_element_db();
                let element = match &args[0] {
                    Value::String(sym) => db.get(sym.as_str()).cloned(),
                    Value::Int(z) => db
                        .values()
                        .find(|el| el.atomic_number == *z as u32)
                        .cloned(),
                    _ => {
                        return Err(
                            "chemistry.element_info: argumento deve ser String ou Int".to_string()
                        )
                    }
                };

                let el = element
                    .ok_or_else(|| "Elemento não encontrado no banco de dados".to_string())?;

                let mut map = HashMap::new();
                map.insert("name".to_string(), Value::new_string(el.name.to_string()));
                map.insert(
                    "symbol".to_string(),
                    Value::new_string(el.symbol.to_string()),
                );
                map.insert(
                    "atomic_number".to_string(),
                    Value::Int(el.atomic_number as i64),
                );
                map.insert("atomic_mass".to_string(), Value::Float(el.atomic_mass));
                map.insert(
                    "electronegativity".to_string(),
                    el.electronegativity
                        .map(Value::Float)
                        .unwrap_or(Value::Unit),
                );
                map.insert(
                    "ionization_energy".to_string(),
                    el.ionization_energy
                        .map(Value::Float)
                        .unwrap_or(Value::Unit),
                );
                map.insert(
                    "atomic_radius".to_string(),
                    el.atomic_radius.map(Value::Float).unwrap_or(Value::Unit),
                );
                map.insert("group".to_string(), Value::Int(el.group as i64));
                map.insert("period".to_string(), Value::Int(el.period as i64));
                map.insert("is_metal".to_string(), Value::Bool(el.is_metal));

                Ok(Value::new_map(map))
            }
            "electron_configuration" => {
                if args.is_empty() {
                    return Err(
                        "chemistry.electron_configuration: esperado número atômico (int)"
                            .to_string(),
                    );
                }
                let z = args[0].as_int()? as u32;

                let config = ground_state_configuration(z);
                let config_str = config
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");

                let mut subshells_list = Vec::new();
                for sub in &config {
                    let mut sub_map = HashMap::new();
                    sub_map.insert("n".to_string(), Value::Int(sub.n as i64));
                    sub_map.insert("l".to_string(), Value::new_string(sub.l_char.to_string()));
                    sub_map.insert("count".to_string(), Value::Int(sub.count as i64));
                    subshells_list.push(Value::new_map(sub_map));
                }

                let mut map = HashMap::new();
                map.insert("configuration".to_string(), Value::new_string(config_str));
                map.insert("subshells".to_string(), Value::new_list(subshells_list));

                if let Ok((qn_n, qn_l, qn_ml, qn_ms)) = last_electron_quantum_numbers(z) {
                    let mut qn_map = HashMap::new();
                    qn_map.insert("n".to_string(), Value::Int(qn_n as i64));
                    qn_map.insert("l".to_string(), Value::Int(qn_l as i64));
                    qn_map.insert("ml".to_string(), Value::Int(qn_ml as i64));
                    qn_map.insert("ms".to_string(), Value::Float(qn_ms));
                    map.insert("quantum_numbers".to_string(), Value::new_map(qn_map));
                }

                if let Ok(z_eff) = slater_z_effective(z) {
                    map.insert("slater_z_eff".to_string(), Value::Float(z_eff));
                }

                Ok(Value::new_map(map))
            }
            "bond_info" => {
                if args.len() < 2 {
                    return Err("chemistry.bond_info: esperado simbolo_a, simbolo_b".to_string());
                }
                let a = args[0].as_string()?;
                let b = args[1].as_string()?;

                let bond = predict_bond(&a, &b)?;
                let mut map = HashMap::new();
                map.insert(
                    "delta_electronegativity".to_string(),
                    Value::Float(bond.delta_electronegativity),
                );
                map.insert(
                    "ionic_character_percent".to_string(),
                    Value::Float(bond.ionic_character_percent),
                );
                map.insert("bond_type".to_string(), Value::new_string(bond.bond_type));

                Ok(Value::new_map(map))
            }
            "molecular_mass" => {
                if args.is_empty() {
                    return Err(
                        "chemistry.molecular_mass: esperado fórmula química (string)".to_string(),
                    );
                }
                let formula = args[0].as_string()?;

                let (mass, composition) = molecular_mass_and_composition(&formula)?;
                let mut map = HashMap::new();
                map.insert("molecular_mass".to_string(), Value::Float(mass));

                let mut comp_map = HashMap::new();
                for (sym, pct) in composition {
                    comp_map.insert(sym, Value::Float(pct));
                }
                map.insert("composition".to_string(), Value::new_map(comp_map));

                Ok(Value::new_map(map))
            }
            "balance_reaction" => {
                if args.is_empty() {
                    return Err(
                        "chemistry.balance_reaction: esperado reação química (string)".to_string(),
                    );
                }
                let reaction = args[0].as_string()?;
                let balanced = balance_reaction(&reaction)?;
                Ok(Value::new_string(balanced))
            }
            _ => Err(format!("Unknown chemistry method: {}", method)),
        }
    }
}
