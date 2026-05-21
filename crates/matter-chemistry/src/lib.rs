//! # Matter Chemistry
//!
//! Motor de Química Matemática: Tabelas periódicas, configurações eletrônicas,
//! regras de Slater, ligações químicas, parsing de fórmulas moleculares e balanceador estequiométrico.

use std::collections::HashMap;

pub mod backend;

/// Estrutura para representar dados físicos de um elemento químico.
#[derive(Debug, Clone)]
pub struct Element {
    pub name: &'static str,
    pub symbol: &'static str,
    pub atomic_number: u32,
    pub atomic_mass: f64,
    pub electronegativity: Option<f64>,
    pub ionization_energy: Option<f64>, // eV
    pub atomic_radius: Option<f64>,     // pm
    pub group: u32,
    pub period: u32,
    pub is_metal: bool,
}

/// Banco de dados estático contendo os primeiros 20 elementos + metais de transição e pesados cruciais.
pub fn get_element_db() -> HashMap<&'static str, Element> {
    let mut db = HashMap::new();

    let elements = vec![
        Element {
            name: "Hydrogen",
            symbol: "H",
            atomic_number: 1,
            atomic_mass: 1.008,
            electronegativity: Some(2.20),
            ionization_energy: Some(13.598),
            atomic_radius: Some(53.0),
            group: 1,
            period: 1,
            is_metal: false,
        },
        Element {
            name: "Helium",
            symbol: "He",
            atomic_number: 2,
            atomic_mass: 4.0026,
            electronegativity: None,
            ionization_energy: Some(24.587),
            atomic_radius: Some(31.0),
            group: 18,
            period: 1,
            is_metal: false,
        },
        Element {
            name: "Lithium",
            symbol: "Li",
            atomic_number: 3,
            atomic_mass: 6.94,
            electronegativity: Some(0.98),
            ionization_energy: Some(5.392),
            atomic_radius: Some(167.0),
            group: 1,
            period: 2,
            is_metal: true,
        },
        Element {
            name: "Beryllium",
            symbol: "Be",
            atomic_number: 4,
            atomic_mass: 9.0122,
            electronegativity: Some(1.57),
            ionization_energy: Some(9.323),
            atomic_radius: Some(112.0),
            group: 2,
            period: 2,
            is_metal: true,
        },
        Element {
            name: "Boron",
            symbol: "B",
            atomic_number: 5,
            atomic_mass: 10.81,
            electronegativity: Some(2.04),
            ionization_energy: Some(8.298),
            atomic_radius: Some(87.0),
            group: 13,
            period: 2,
            is_metal: false,
        },
        Element {
            name: "Carbon",
            symbol: "C",
            atomic_number: 6,
            atomic_mass: 12.011,
            electronegativity: Some(2.55),
            ionization_energy: Some(11.260),
            atomic_radius: Some(67.0),
            group: 14,
            period: 2,
            is_metal: false,
        },
        Element {
            name: "Nitrogen",
            symbol: "N",
            atomic_number: 7,
            atomic_mass: 14.007,
            electronegativity: Some(3.04),
            ionization_energy: Some(14.534),
            atomic_radius: Some(56.0),
            group: 15,
            period: 2,
            is_metal: false,
        },
        Element {
            name: "Oxygen",
            symbol: "O",
            atomic_number: 8,
            atomic_mass: 15.999,
            electronegativity: Some(3.44),
            ionization_energy: Some(13.618),
            atomic_radius: Some(48.0),
            group: 16,
            period: 2,
            is_metal: false,
        },
        Element {
            name: "Fluorine",
            symbol: "F",
            atomic_number: 9,
            atomic_mass: 18.998,
            electronegativity: Some(3.98),
            ionization_energy: Some(17.423),
            atomic_radius: Some(42.0),
            group: 17,
            period: 2,
            is_metal: false,
        },
        Element {
            name: "Neon",
            symbol: "Ne",
            atomic_number: 10,
            atomic_mass: 20.180,
            electronegativity: None,
            ionization_energy: Some(21.565),
            atomic_radius: Some(38.0),
            group: 18,
            period: 2,
            is_metal: false,
        },
        Element {
            name: "Sodium",
            symbol: "Na",
            atomic_number: 11,
            atomic_mass: 22.990,
            electronegativity: Some(0.93),
            ionization_energy: Some(5.139),
            atomic_radius: Some(190.0),
            group: 1,
            period: 3,
            is_metal: true,
        },
        Element {
            name: "Magnesium",
            symbol: "Mg",
            atomic_number: 12,
            atomic_mass: 24.305,
            electronegativity: Some(1.31),
            ionization_energy: Some(7.646),
            atomic_radius: Some(145.0),
            group: 2,
            period: 3,
            is_metal: true,
        },
        Element {
            name: "Aluminum",
            symbol: "Al",
            atomic_number: 13,
            atomic_mass: 26.982,
            electronegativity: Some(1.61),
            ionization_energy: Some(5.986),
            atomic_radius: Some(118.0),
            group: 13,
            period: 3,
            is_metal: true,
        },
        Element {
            name: "Silicon",
            symbol: "Si",
            atomic_number: 14,
            atomic_mass: 28.085,
            electronegativity: Some(1.90),
            ionization_energy: Some(8.152),
            atomic_radius: Some(111.0),
            group: 14,
            period: 3,
            is_metal: false,
        },
        Element {
            name: "Phosphorus",
            symbol: "P",
            atomic_number: 15,
            atomic_mass: 30.974,
            electronegativity: Some(2.19),
            ionization_energy: Some(10.487),
            atomic_radius: Some(98.0),
            group: 15,
            period: 3,
            is_metal: false,
        },
        Element {
            name: "Sulfur",
            symbol: "S",
            atomic_number: 16,
            atomic_mass: 32.06,
            electronegativity: Some(2.58),
            ionization_energy: Some(10.360),
            atomic_radius: Some(88.0),
            group: 16,
            period: 3,
            is_metal: false,
        },
        Element {
            name: "Chlorine",
            symbol: "Cl",
            atomic_number: 17,
            atomic_mass: 35.45,
            electronegativity: Some(3.16),
            ionization_energy: Some(12.968),
            atomic_radius: Some(79.0),
            group: 17,
            period: 3,
            is_metal: false,
        },
        Element {
            name: "Argon",
            symbol: "Ar",
            atomic_number: 18,
            atomic_mass: 39.948,
            electronegativity: None,
            ionization_energy: Some(15.760),
            atomic_radius: Some(71.0),
            group: 18,
            period: 3,
            is_metal: false,
        },
        Element {
            name: "Potassium",
            symbol: "K",
            atomic_number: 19,
            atomic_mass: 39.098,
            electronegativity: Some(0.82),
            ionization_energy: Some(4.341),
            atomic_radius: Some(243.0),
            group: 1,
            period: 4,
            is_metal: true,
        },
        Element {
            name: "Calcium",
            symbol: "Ca",
            atomic_number: 20,
            atomic_mass: 40.078,
            electronegativity: Some(1.00),
            ionization_energy: Some(6.113),
            atomic_radius: Some(194.0),
            group: 2,
            period: 4,
            is_metal: true,
        },
        // Elementos de Transição e Pesados Importantes
        Element {
            name: "Iron",
            symbol: "Fe",
            atomic_number: 26,
            atomic_mass: 55.845,
            electronegativity: Some(1.83),
            ionization_energy: Some(7.902),
            atomic_radius: Some(156.0),
            group: 8,
            period: 4,
            is_metal: true,
        },
        Element {
            name: "Copper",
            symbol: "Cu",
            atomic_number: 29,
            atomic_mass: 63.546,
            electronegativity: Some(1.90),
            ionization_energy: Some(7.726),
            atomic_radius: Some(145.0),
            group: 11,
            period: 4,
            is_metal: true,
        },
        Element {
            name: "Gold",
            symbol: "Au",
            atomic_number: 79,
            atomic_mass: 196.97,
            electronegativity: Some(2.54),
            ionization_energy: Some(9.226),
            atomic_radius: Some(174.0),
            group: 11,
            period: 6,
            is_metal: true,
        },
        Element {
            name: "Uranium",
            symbol: "U",
            atomic_number: 92,
            atomic_mass: 238.03,
            electronegativity: Some(1.38),
            ionization_energy: Some(6.194),
            atomic_radius: Some(156.0),
            group: 3,
            period: 7,
            is_metal: true,
        },
    ];

    for el in elements {
        db.insert(el.symbol, el);
    }
    db
}

/// Representação de uma subcamada eletrônica preenchida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subshell {
    pub n: u32,
    pub l_char: char, // 's', 'p', 'd', 'f'
    pub count: u32,
}

impl std::fmt::Display for Subshell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.n, self.l_char, self.count)
    }
}

impl Subshell {
    pub fn l_val(&self) -> u32 {
        match self.l_char {
            's' => 0,
            'p' => 1,
            'd' => 2,
            'f' => 3,
            _ => 0,
        }
    }
}

/// Gera a configuração eletrônica fundamental usando o Princípio de Aufbau e Hund.
pub fn ground_state_configuration(z: u32) -> Vec<Subshell> {
    if z == 0 {
        return vec![];
    }

    // Ordem de preenchimento (Aufbau: n+l)
    let fill_order = vec![
        (1, 's', 2),
        (2, 's', 2),
        (2, 'p', 6),
        (3, 's', 2),
        (3, 'p', 6),
        (4, 's', 2),
        (3, 'd', 10),
        (4, 'p', 6),
        (5, 's', 2),
        (4, 'd', 10),
        (5, 'p', 6),
        (6, 's', 2),
        (4, 'f', 14),
        (5, 'd', 10),
        (6, 'p', 6),
        (7, 's', 2),
        (5, 'f', 14),
        (6, 'd', 10),
        (7, 'p', 6),
    ];

    let mut remaining = z;
    let mut config = Vec::new();

    for &(n, l_char, max_capacity) in &fill_order {
        if remaining == 0 {
            break;
        }
        let take = remaining.min(max_capacity);
        config.push(Subshell {
            n,
            l_char,
            count: take,
        });
        remaining -= take;
    }

    // Aplicação das anomalias mais conhecidas (Chromium e Copper)
    // Chromium (Z=24): [Ar] 3d5 4s1 em vez de 3d4 4s2
    if z == 24 {
        // Encontra 4s e 3d na configuração e ajusta
        for sub in &mut config {
            if sub.n == 4 && sub.l_char == 's' {
                sub.count = 1;
            }
            if sub.n == 3 && sub.l_char == 'd' {
                sub.count = 5;
            }
        }
    }
    // Copper (Z=29): [Ar] 3d10 4s1 em vez de 3d9 4s2
    if z == 29 {
        for sub in &mut config {
            if sub.n == 4 && sub.l_char == 's' {
                sub.count = 1;
            }
            if sub.n == 3 && sub.l_char == 'd' {
                sub.count = 10;
            }
        }
    }

    config
}

/// Calcula os números quânticos (n, l, ml, ms) para o último elétron inserido.
pub fn last_electron_quantum_numbers(z: u32) -> Result<(u32, u32, i32, f64), String> {
    let config = ground_state_configuration(z);
    let last = config
        .last()
        .ok_or_else(|| "Z = 0 não possui elétrons".to_string())?;

    let n = last.n;
    let l = last.l_val();
    let k = last.count; // elétrons na última subcamada

    let orbitals_count = 2 * l + 1;

    // Regra de Hund: preenche os spins positivos (up) em todos os orbitais primeiro, depois emparelha
    if k <= orbitals_count {
        // Elétrons desemparelhados: spin = +0.5
        let orbital_idx = (k as i32) - 1;
        let ml = orbital_idx - (l as i32);
        let ms = 0.5;
        Ok((n, l, ml, ms))
    } else {
        // Elétrons emparelhados: spin = -0.5
        let orbital_idx = (k - orbitals_count) as i32 - 1;
        let ml = orbital_idx - (l as i32);
        let ms = -0.5;
        Ok((n, l, ml, ms))
    }
}

/// Calcula a Carga Nuclear Efetiva (Zeff) de Slater para o elétron mais externo (valência).
pub fn slater_z_effective(z: u32) -> Result<f64, String> {
    if z == 0 {
        return Err("Z = 0 não possui elétrons".to_string());
    }

    let config = ground_state_configuration(z);

    // Encontrar o elétron de valência (maior n e depois maior l)
    let mut valence_n = 0;
    let mut valence_l_char = 's';
    let mut valence_l_val = 0;

    for sub in &config {
        let l_val = sub.l_val();
        if sub.n > valence_n || (sub.n == valence_n && l_val > valence_l_val) {
            valence_n = sub.n;
            valence_l_char = sub.l_char;
            valence_l_val = l_val;
        }
    }

    let mut shielding = 0.0;

    // Slater groups:
    // (1s), (2s,2p), (3s,3p), (3d), (4s,4p), (4d), (4f)...
    // Nós calculamos as contribuições de cada elétron na configuração.
    for sub in &config {
        let is_same_n = sub.n == valence_n;

        if valence_l_char == 's' || valence_l_char == 'p' {
            // Regra para s/p:
            if is_same_n {
                let other_electrons = if sub.l_char == 's' || sub.l_char == 'p' {
                    // Contribuição do mesmo grupo (ns, np)
                    // Subtraímos 1 para desconsiderar o próprio elétron de teste
                    if sub.n == 1 {
                        (sub.count - 1) as f64 * 0.30
                    } else {
                        // ns e np são somados
                        let total_in_group = config
                            .iter()
                            .filter(|s| s.n == valence_n && (s.l_char == 's' || s.l_char == 'p'))
                            .map(|s| s.count)
                            .sum::<u32>();
                        (total_in_group - 1) as f64 * 0.35
                    }
                } else {
                    // d ou f no mesmo nível (n) não blindam s/p, então contribuem com 0.0
                    0.0
                };
                shielding += other_electrons;
            } else if sub.n == valence_n - 1 {
                // Nível (n-1) blinda 0.85
                shielding += sub.count as f64 * 0.85;
            } else if sub.n <= valence_n - 2 {
                // Nível (n-2) ou inferior blinda 1.00
                shielding += sub.count as f64 * 1.00;
            }
        } else {
            // Regra para d/f:
            if is_same_n && sub.l_char == valence_l_char {
                // Outros elétrons no mesmo grupo (nd ou nf) blindam 0.35
                shielding += (sub.count - 1) as f64 * 0.35;
            } else if sub.n < valence_n || (is_same_n && sub.l_val() < valence_l_val) {
                // Grupos internos (todos com energia inferior) blindam 1.00
                shielding += sub.count as f64 * 1.00;
            }
        }
    }

    let z_eff = (z as f64) - shielding;
    Ok(z_eff)
}

/// Informações sobre a ligação química entre dois elementos.
#[derive(Debug, Clone)]
pub struct BondInfo {
    pub delta_electronegativity: f64,
    pub ionic_character_percent: f64,
    pub bond_type: String,
}

pub fn predict_bond(symbol_a: &str, symbol_b: &str) -> Result<BondInfo, String> {
    let db = get_element_db();
    let el_a = db
        .get(symbol_a)
        .ok_or_else(|| format!("Elemento não encontrado: {}", symbol_a))?;
    let el_b = db
        .get(symbol_b)
        .ok_or_else(|| format!("Elemento não encontrado: {}", symbol_b))?;

    let (chi_a, chi_b) = match (el_a.electronegativity, el_b.electronegativity) {
        (Some(ca), Some(cb)) => (ca, cb),
        _ => {
            return Err(
                "Um ou ambos os elementos não possuem eletronegatividade definida".to_string(),
            )
        }
    };

    let delta_chi = (chi_a - chi_b).abs();
    // Fórmula de Pauling para Caráter Iônico: % = (1 - e^(-0.25 * DeltaChi^2)) * 100
    let ionic_percent = (1.0 - (-0.25 * delta_chi * delta_chi).exp()) * 100.0;

    let bond_type = if el_a.is_metal && el_b.is_metal {
        "Metallic".to_string()
    } else if delta_chi >= 1.7 {
        "Ionic".to_string()
    } else if delta_chi >= 0.4 {
        "Polar Covalent".to_string()
    } else {
        "Covalent Apolar".to_string()
    };

    Ok(BondInfo {
        delta_electronegativity: delta_chi,
        ionic_character_percent: ionic_percent,
        bond_type,
    })
}

// ============================================================================
// MOLECULAR FORMULA PARSER
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Element(String),
    Number(u32),
    LeftParen,
    RightParen,
}

fn lex_formula(formula: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = formula.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == ' ' {
            i += 1;
            continue;
        }

        if c == '(' {
            tokens.push(Token::LeftParen);
            i += 1;
        } else if c == ')' {
            tokens.push(Token::RightParen);
            i += 1;
        } else if c.is_ascii_uppercase() {
            let mut sym = String::new();
            sym.push(c);
            i += 1;
            if i < chars.len() && chars[i].is_ascii_lowercase() {
                sym.push(chars[i]);
                i += 1;
            }
            tokens.push(Token::Element(sym));
        } else if c.is_ascii_digit() {
            let mut num_str = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                num_str.push(chars[i]);
                i += 1;
            }
            let val = num_str.parse::<u32>().map_err(|e| e.to_string())?;
            tokens.push(Token::Number(val));
        } else {
            return Err(format!("Caractere inválido na fórmula: '{}'", c));
        }
    }

    Ok(tokens)
}

fn parse_tokens(tokens: &[Token], index: &mut usize) -> Result<HashMap<String, u32>, String> {
    let mut counts = HashMap::new();

    while *index < tokens.len() {
        match &tokens[*index] {
            Token::Element(sym) => {
                *index += 1;
                let count = if *index < tokens.len() {
                    if let Token::Number(val) = tokens[*index] {
                        *index += 1;
                        val
                    } else {
                        1
                    }
                } else {
                    1
                };
                *counts.entry(sym.clone()).or_insert(0) += count;
            }
            Token::LeftParen => {
                *index += 1;
                let sub_counts = parse_tokens(tokens, index)?;
                // Deve haver um RightParen correspondente
                if *index >= tokens.len() || tokens[*index] != Token::RightParen {
                    return Err("Parêntese não fechado na fórmula".to_string());
                }
                *index += 1; // consome RightParen

                // Opcionalmente lê o multiplicador do grupo
                let multiplier = if *index < tokens.len() {
                    if let Token::Number(val) = tokens[*index] {
                        *index += 1;
                        val
                    } else {
                        1
                    }
                } else {
                    1
                };

                for (sym, sub_count) in sub_counts {
                    *counts.entry(sym).or_insert(0) += sub_count * multiplier;
                }
            }
            Token::RightParen => {
                // Retorna para o grupo chamador sem consumir (o chamador consome)
                break;
            }
            Token::Number(val) => {
                return Err(format!("Número órfão sem elemento correspondente: {}", val));
            }
        }
    }

    Ok(counts)
}

/// Realiza o parsing de fórmulas complexas (ex: "C6H12O6", "Ca(OH)2", "Fe2(SO4)3").
pub fn parse_formula(formula: &str) -> Result<HashMap<String, u32>, String> {
    let tokens = lex_formula(formula)?;
    let mut index = 0;
    let res = parse_tokens(&tokens, &mut index)?;
    if index < tokens.len() {
        return Err("Erro inesperado de sintaxe no final da fórmula".to_string());
    }
    Ok(res)
}

/// Calcula a massa molecular total e os percentuais de massa de cada elemento na fórmula.
pub fn molecular_mass_and_composition(
    formula: &str,
) -> Result<(f64, HashMap<String, f64>), String> {
    let composition = parse_formula(formula)?;
    let db = get_element_db();

    let mut total_mass = 0.0;
    let mut element_masses = HashMap::new();

    for (sym, count) in &composition {
        let el = db
            .get(sym.as_str())
            .ok_or_else(|| format!("Elemento desconhecido na fórmula: {}", sym))?;
        let m = el.atomic_mass * (*count as f64);
        total_mass += m;
        element_masses.insert(sym.clone(), m);
    }

    let mut percentages = HashMap::new();
    for (sym, m) in element_masses {
        let pct = (m / total_mass) * 100.0;
        percentages.insert(sym, pct);
    }

    Ok((total_mass, percentages))
}

// ============================================================================
// STOICHIOMETRIC MATRIX REACTION BALANCER
// ============================================================================

/// Balanceia uma reação química (ex: "C3H8 + O2 -> CO2 + H2O").
/// Retorna a string balanceada e os coeficientes ordenados de reagentes e produtos.
pub fn balance_reaction(reaction: &str) -> Result<String, String> {
    let parts: Vec<&str> = reaction.split("->").collect();
    if parts.len() != 2 {
        return Err("A reação deve conter exatamente um símbolo '->'".to_string());
    }

    let reactants_raw: Vec<&str> = parts[0]
        .split('+')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let products_raw: Vec<&str> = parts[1]
        .split('+')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if reactants_raw.is_empty() || products_raw.is_empty() {
        return Err("A reação precisa ter reagentes e produtos".to_string());
    }

    let mut formula_compositions = Vec::new();
    let mut unique_elements = Vec::new();

    // Parse de todos os reagentes e produtos
    for &f in &reactants_raw {
        let comp = parse_formula(f)?;
        for el in comp.keys() {
            if !unique_elements.contains(el) {
                unique_elements.push(el.clone());
            }
        }
        formula_compositions.push((comp, true)); // true = reagente
    }

    for &f in &products_raw {
        let comp = parse_formula(f)?;
        for el in comp.keys() {
            if !unique_elements.contains(el) {
                unique_elements.push(el.clone());
            }
        }
        formula_compositions.push((comp, false)); // false = produto
    }

    unique_elements.sort(); // Determinação determinística de linhas

    let r_count = unique_elements.len();
    let c_count = formula_compositions.len();

    // Construção da Matriz Estequiométrica M
    // M[row, col] = reagentes (+), produtos (-)
    let mut m = vec![vec![0.0; c_count]; r_count];

    for (c, (comp, is_reactant)) in formula_compositions.iter().enumerate() {
        for (r, el) in unique_elements.iter().enumerate() {
            if let Some(&count) = comp.get(el) {
                let val = count as f64;
                m[r][c] = if *is_reactant { val } else { -val };
            }
        }
    }

    // Resolve RREF da matriz estequiométrica
    rref(&mut m);

    // Encontra uma solução inteira nula não trivial positiva
    // Usamos um resolvedor de força bruta racional para pequenos coeficientes (1..100)
    let coeffs = find_integer_solution(&m, c_count)?;

    // Formata o output
    let mut balanced_reactants = Vec::new();
    for i in 0..reactants_raw.len() {
        let c = coeffs[i];
        if c == 1 {
            balanced_reactants.push(reactants_raw[i].to_string());
        } else {
            balanced_reactants.push(format!("{} {}", c, reactants_raw[i]));
        }
    }

    let mut balanced_products = Vec::new();
    for (i, product) in products_raw.iter().enumerate() {
        let idx = reactants_raw.len() + i;
        let c = coeffs[idx];
        if c == 1 {
            balanced_products.push(product.to_string());
        } else {
            balanced_products.push(format!("{} {}", c, product));
        }
    }

    Ok(format!(
        "{} -> {}",
        balanced_reactants.join(" + "),
        balanced_products.join(" + ")
    ))
}

/// Reduz a matriz para a Forma Escalonada Reduzida por Linhas (RREF)
fn rref(m: &mut [Vec<f64>]) {
    let rows = m.len();
    if rows == 0 {
        return;
    }
    let cols = m[0].len();
    let mut lead = 0;

    for r in 0..rows {
        if lead >= cols {
            break;
        }
        let mut i = r;
        while m[i][lead].abs() < 1e-9 {
            i += 1;
            if i == rows {
                i = r;
                lead += 1;
                if lead == cols {
                    return;
                }
            }
        }
        m.swap(i, r);
        let lv = m[r][lead];
        if lv.abs() > 1e-9 {
            for val in &mut m[r] {
                *val /= lv;
            }
        }
        for i in 0..rows {
            if i != r {
                let lv2 = m[i][lead];
                #[allow(clippy::needless_range_loop)]
                for c in 0..cols {
                    m[i][c] -= lv2 * m[r][c];
                }
            }
        }
        lead += 1;
    }
}

/// Tenta encontrar um multiplicador inteiro positivo para a base do espaço nulo
fn find_integer_solution(rref_m: &[Vec<f64>], c_count: usize) -> Result<Vec<u32>, String> {
    // Nós assumimos que para a maioria das reações químicas o espaço nulo tem dimensão 1.
    // Assim, podemos definir a variável livre x[c_count - 1] = D (multiplicador)
    // E calcular as outras variáveis. Se houver algum multiplicador de 1 até 1000 que resulte
    // em todos os coeficientes inteiros e estritamente positivos (> 0), retornamos essa solução.
    for d in 1..1000 {
        let mut coeffs = vec![0.0; c_count];
        coeffs[c_count - 1] = d as f64;

        let mut valid = true;

        // Para cada linha pivotada na RREF, resolvemos a variável básica correspondente
        for row in rref_m.iter() {
            // Encontra o pivot da linha
            let mut pivot_col = None;
            for (c, &val) in row.iter().enumerate().take(c_count) {
                if val.abs() > 1e-5 {
                    pivot_col = Some(c);
                    break;
                }
            }

            if let Some(pc) = pivot_col {
                if pc < c_count - 1 {
                    // x[pc] + rref_m[r][c_count-1] * x[c_count-1] = 0
                    // => x[pc] = -rref_m[r][c_count-1] * d
                    let val = -row[c_count - 1] * (d as f64);
                    coeffs[pc] = val;
                }
            }
        }

        // Validação da solução candidata
        let mut int_coeffs = vec![0; c_count];
        for (i, &v) in coeffs.iter().enumerate() {
            let rounded = v.round();
            if (v - rounded).abs() > 1e-4 || rounded < 0.9 {
                valid = false;
                break;
            }
            int_coeffs[i] = rounded as u32;
        }

        if valid {
            return Ok(int_coeffs);
        }
    }

    Err("Não foi possível balancear a reação com coeficientes inteiros pequenos".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_electron_configuration() {
        let config = ground_state_configuration(6); // Carbon
        assert_eq!(config[0].to_string(), "1s2");
        assert_eq!(config[1].to_string(), "2s2");
        assert_eq!(config[2].to_string(), "2p2");

        // Anomalias
        let copper = ground_state_configuration(29);
        assert!(copper
            .iter()
            .any(|s| s.n == 4 && s.l_char == 's' && s.count == 1));
        assert!(copper
            .iter()
            .any(|s| s.n == 3 && s.l_char == 'd' && s.count == 10));
    }

    #[test]
    fn test_quantum_numbers() {
        let (n, l, ml, ms) = last_electron_quantum_numbers(6).unwrap(); // Carbon Z=6: last is 2p2
        assert_eq!(n, 2);
        assert_eq!(l, 1);
        assert_eq!(ml, -1 + 1); // 2nd electron in p subshell -> ml = 0
        assert_eq!(ms, 0.5);
    }

    #[test]
    fn test_slater_z_effective() {
        let z_eff_h = slater_z_effective(1).unwrap();
        assert_eq!(z_eff_h, 1.0);

        let z_eff_he = slater_z_effective(2).unwrap();
        assert_eq!(z_eff_he, 1.7); // 2 - 0.3 = 1.7
    }

    #[test]
    fn test_predict_bond() {
        let bond_nacl = predict_bond("Na", "Cl").unwrap();
        assert_eq!(bond_nacl.bond_type, "Ionic");

        let bond_h2o = predict_bond("H", "O").unwrap();
        assert_eq!(bond_h2o.bond_type, "Polar Covalent");
    }

    #[test]
    fn test_formula_parser() {
        let h2o = parse_formula("H2O").unwrap();
        assert_eq!(h2o.get("H"), Some(&2));
        assert_eq!(h2o.get("O"), Some(&1));

        let alcohol = parse_formula("C2H5OH").unwrap();
        assert_eq!(alcohol.get("C"), Some(&2));
        assert_eq!(alcohol.get("H"), Some(&6));
        assert_eq!(alcohol.get("O"), Some(&1));

        let alum = parse_formula("Al2(SO4)3").unwrap();
        assert_eq!(alum.get("Al"), Some(&2));
        assert_eq!(alum.get("S"), Some(&3));
        assert_eq!(alum.get("O"), Some(&12));
    }

    #[test]
    fn test_molecular_mass() {
        let (mass, pcts) = molecular_mass_and_composition("H2O").unwrap();
        assert!((mass - 18.015).abs() < 1e-2);
        assert!(pcts.get("H").unwrap() > &11.0);
    }

    #[test]
    fn test_balance_reaction() {
        let reaction1 = balance_reaction("H2 + O2 -> H2O").unwrap();
        assert_eq!(reaction1, "2 H2 + O2 -> 2 H2O");

        let reaction2 = balance_reaction("C3H8 + O2 -> CO2 + H2O").unwrap();
        assert_eq!(reaction2, "C3H8 + 5 O2 -> 3 CO2 + 4 H2O");
    }
}
