//! # Matter Molecular Computing
//!
//! Molecular and atomic-level computation.
//! Includes DNA computing, molecular logic gates, chemical reactions as computation.

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// DNA COMPUTING
// ============================================================================

/// DNA base
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DNABase {
    A, // Adenine
    T, // Thymine
    G, // Guanine
    C, // Cytosine
}

impl DNABase {
    pub fn complement(&self) -> Self {
        match self {
            DNABase::A => DNABase::T,
            DNABase::T => DNABase::A,
            DNABase::G => DNABase::C,
            DNABase::C => DNABase::G,
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'A' | 'a' => Some(DNABase::A),
            'T' | 't' => Some(DNABase::T),
            'G' | 'g' => Some(DNABase::G),
            'C' | 'c' => Some(DNABase::C),
            _ => None,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            DNABase::A => 'A',
            DNABase::T => 'T',
            DNABase::G => 'G',
            DNABase::C => 'C',
        }
    }
}

/// DNA strand
#[derive(Debug, Clone, PartialEq)]
pub struct DNAStrand {
    pub bases: Vec<DNABase>,
}

impl DNAStrand {
    pub fn new(sequence: &str) -> Option<Self> {
        let bases: Option<Vec<_>> = sequence.chars().map(DNABase::from_char).collect();
        bases.map(|b| Self { bases: b })
    }

    pub fn complement(&self) -> Self {
        Self {
            bases: self.bases.iter().map(|b| b.complement()).collect(),
        }
    }

    pub fn reverse_complement(&self) -> Self {
        Self {
            bases: self.bases.iter().rev().map(|b| b.complement()).collect(),
        }
    }

    pub fn hamming_distance(&self, other: &DNAStrand) -> usize {
        self.bases
            .iter()
            .zip(other.bases.iter())
            .filter(|(a, b)| a != b)
            .count()
    }

    pub fn gc_content(&self) -> f64 {
        let gc_count = self
            .bases
            .iter()
            .filter(|b| **b == DNABase::G || **b == DNABase::C)
            .count();
        gc_count as f64 / self.bases.len() as f64
    }
}

impl fmt::Display for DNAStrand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sequence: String = self.bases.iter().map(|b| b.to_char()).collect();
        write!(f, "{}", sequence)
    }
}

/// DNA computing operation
#[derive(Debug, Clone)]
pub enum DNAOperation {
    Hybridization, // Bind complementary strands
    Ligation,      // Join strands
    Cleavage,      // Cut strands
    Amplification, // PCR
    Separation,    // Gel electrophoresis
}

/// DNA computer
pub struct DNAComputer {
    pub strands: Vec<DNAStrand>,
    pub operations: Vec<DNAOperation>,
}

impl DNAComputer {
    pub fn new() -> Self {
        Self {
            strands: Vec::new(),
            operations: Vec::new(),
        }
    }

    pub fn add_strand(&mut self, strand: DNAStrand) {
        self.strands.push(strand);
    }

    pub fn hybridize(&mut self, strand1_idx: usize, strand2_idx: usize) -> Option<DNAStrand> {
        if strand1_idx >= self.strands.len() || strand2_idx >= self.strands.len() {
            return None;
        }

        let strand1 = &self.strands[strand1_idx];
        let strand2 = &self.strands[strand2_idx];

        // Check if strands are complementary
        if strand1.bases.len() == strand2.bases.len() {
            let is_complement = strand1
                .bases
                .iter()
                .zip(strand2.bases.iter())
                .all(|(b1, b2)| b1.complement() == *b2);

            if is_complement {
                self.operations.push(DNAOperation::Hybridization);
                return Some(strand1.clone());
            }
        }

        None
    }

    pub fn ligate(&mut self, strand1_idx: usize, strand2_idx: usize) -> Option<DNAStrand> {
        if strand1_idx >= self.strands.len() || strand2_idx >= self.strands.len() {
            return None;
        }

        let mut new_bases = self.strands[strand1_idx].bases.clone();
        new_bases.extend(self.strands[strand2_idx].bases.clone());

        self.operations.push(DNAOperation::Ligation);
        Some(DNAStrand { bases: new_bases })
    }

    pub fn amplify(&mut self, strand_idx: usize, copies: usize) -> Vec<DNAStrand> {
        if strand_idx >= self.strands.len() {
            return Vec::new();
        }

        self.operations.push(DNAOperation::Amplification);
        vec![self.strands[strand_idx].clone(); copies]
    }

    pub fn solve_hamiltonian_path(
        &mut self,
        graph: &[(usize, usize)],
        num_vertices: usize,
    ) -> Vec<Vec<usize>> {
        // Encode graph as DNA strands
        let mut vertex_strands = Vec::new();
        for v in 0..num_vertices {
            let sequence = format!("ATCG{:04b}", v)
                .replace('0', "AT")
                .replace('1', "GC");
            if let Some(strand) = DNAStrand::new(&sequence) {
                vertex_strands.push(strand);
            }
        }

        // Generate all possible paths
        let mut paths = Vec::new();
        self.generate_paths(0, num_vertices, &mut vec![0], &mut paths);

        // Filter valid paths
        paths
            .into_iter()
            .filter(|path| self.is_valid_path(path, graph))
            .collect()
    }

    fn generate_paths(
        &self,
        _current: usize,
        num_vertices: usize,
        path: &mut Vec<usize>,
        paths: &mut Vec<Vec<usize>>,
    ) {
        if path.len() == num_vertices {
            paths.push(path.clone());
            return;
        }

        for next in 0..num_vertices {
            if !path.contains(&next) {
                path.push(next);
                self.generate_paths(next, num_vertices, path, paths);
                path.pop();
            }
        }
    }

    fn is_valid_path(&self, path: &[usize], graph: &[(usize, usize)]) -> bool {
        for i in 0..path.len() - 1 {
            let edge = (path[i], path[i + 1]);
            if !graph.contains(&edge) {
                return false;
            }
        }
        true
    }

    pub fn storage_capacity_bytes(&self) -> f64 {
        // DNA can store ~215 petabytes per gram
        // Each base pair stores 2 bits
        let total_bases: usize = self.strands.iter().map(|s| s.bases.len()).sum();
        let bits = total_bases * 2;
        bits as f64 / 8.0 // Convert to bytes
    }
}

impl Default for DNAComputer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MOLECULAR LOGIC GATES
// ============================================================================

/// Molecular logic gate type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GateType {
    AND,
    OR,
    NOT,
    NAND,
    NOR,
    XOR,
    XNOR,
}

/// Molecular signal (concentration)
#[derive(Debug, Clone, Copy)]
pub struct MolecularSignal {
    pub concentration: f64, // nM (nanomolar)
}

impl MolecularSignal {
    pub fn new(concentration: f64) -> Self {
        Self { concentration }
    }

    pub fn is_high(&self, threshold: f64) -> bool {
        self.concentration >= threshold
    }

    pub fn to_bool(&self, threshold: f64) -> bool {
        self.is_high(threshold)
    }
}

/// Molecular logic gate
pub struct MolecularGate {
    pub gate_type: GateType,
    pub threshold: f64,
    pub output_high: f64,
    pub output_low: f64,
}

impl MolecularGate {
    pub fn new(gate_type: GateType) -> Self {
        Self {
            gate_type,
            threshold: 50.0,    // 50 nM threshold
            output_high: 100.0, // 100 nM high
            output_low: 0.0,    // 0 nM low
        }
    }

    pub fn compute(&self, inputs: &[MolecularSignal]) -> MolecularSignal {
        let bool_inputs: Vec<bool> = inputs.iter().map(|s| s.to_bool(self.threshold)).collect();

        let result = match self.gate_type {
            GateType::AND => bool_inputs.iter().all(|&x| x),
            GateType::OR => bool_inputs.iter().any(|&x| x),
            GateType::NOT => !bool_inputs[0],
            GateType::NAND => !bool_inputs.iter().all(|&x| x),
            GateType::NOR => !bool_inputs.iter().any(|&x| x),
            GateType::XOR => bool_inputs.iter().filter(|&&x| x).count() % 2 == 1,
            GateType::XNOR => bool_inputs.iter().filter(|&&x| x).count() % 2 == 0,
        };

        MolecularSignal::new(if result {
            self.output_high
        } else {
            self.output_low
        })
    }
}

/// Molecular circuit
pub struct MolecularCircuit {
    pub gates: Vec<MolecularGate>,
    pub connections: Vec<(usize, usize)>, // (from_gate, to_gate)
}

impl MolecularCircuit {
    pub fn new() -> Self {
        Self {
            gates: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn add_gate(&mut self, gate: MolecularGate) -> usize {
        let id = self.gates.len();
        self.gates.push(gate);
        id
    }

    pub fn connect(&mut self, from: usize, to: usize) {
        self.connections.push((from, to));
    }

    pub fn evaluate(&self, inputs: &[MolecularSignal]) -> Vec<MolecularSignal> {
        let mut signals: HashMap<usize, MolecularSignal> = HashMap::new();

        // Initialize inputs
        for (i, &signal) in inputs.iter().enumerate() {
            signals.insert(i, signal);
        }

        // Evaluate gates in topological order
        for (gate_id, gate) in self.gates.iter().enumerate() {
            let gate_inputs: Vec<MolecularSignal> = self
                .connections
                .iter()
                .filter(|(_, to)| *to == gate_id)
                .filter_map(|(from, _)| signals.get(from).copied())
                .collect();

            if !gate_inputs.is_empty() {
                let output = gate.compute(&gate_inputs);
                signals.insert(gate_id, output);
            }
        }

        // Collect outputs
        self.gates
            .iter()
            .enumerate()
            .filter_map(|(id, _)| signals.get(&id).copied())
            .collect()
    }
}

impl Default for MolecularCircuit {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CHEMICAL REACTION COMPUTING
// ============================================================================

/// Chemical species
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Species {
    pub name: String,
    pub concentration: u64, // Number of molecules
}

impl Species {
    pub fn new(name: String, concentration: u64) -> Self {
        Self {
            name,
            concentration,
        }
    }
}

/// Chemical reaction
#[derive(Debug, Clone)]
pub struct Reaction {
    pub reactants: Vec<(String, u64)>, // (species, stoichiometry)
    pub products: Vec<(String, u64)>,
    pub rate: f64,
}

impl Reaction {
    pub fn new(reactants: Vec<(String, u64)>, products: Vec<(String, u64)>, rate: f64) -> Self {
        Self {
            reactants,
            products,
            rate,
        }
    }

    pub fn can_occur(&self, concentrations: &HashMap<String, u64>) -> bool {
        self.reactants
            .iter()
            .all(|(species, stoich)| concentrations.get(species).unwrap_or(&0) >= stoich)
    }

    pub fn apply(&self, concentrations: &mut HashMap<String, u64>) {
        // Consume reactants
        for (species, stoich) in &self.reactants {
            *concentrations.get_mut(species).unwrap() -= stoich;
        }

        // Produce products
        for (species, stoich) in &self.products {
            *concentrations.entry(species.clone()).or_insert(0) += stoich;
        }
    }
}

/// Chemical reaction network
pub struct ReactionNetwork {
    pub species: HashMap<String, u64>,
    pub reactions: Vec<Reaction>,
    pub time: f64,
}

impl ReactionNetwork {
    pub fn new() -> Self {
        Self {
            species: HashMap::new(),
            reactions: Vec::new(),
            time: 0.0,
        }
    }

    pub fn add_species(&mut self, name: String, concentration: u64) {
        self.species.insert(name, concentration);
    }

    pub fn add_reaction(&mut self, reaction: Reaction) {
        self.reactions.push(reaction);
    }

    pub fn step(&mut self, dt: f64) {
        // Gillespie algorithm (stochastic simulation)
        let mut total_propensity = 0.0;
        let mut propensities = Vec::new();

        for reaction in &self.reactions {
            if reaction.can_occur(&self.species) {
                let propensity = reaction.rate;
                propensities.push(propensity);
                total_propensity += propensity;
            } else {
                propensities.push(0.0);
            }
        }

        if total_propensity > 0.0 {
            // Select reaction proportional to propensity
            let mut cumulative = 0.0;
            let threshold = total_propensity * 0.5; // Simplified selection

            for (i, &prop) in propensities.iter().enumerate() {
                cumulative += prop;
                if cumulative >= threshold {
                    self.reactions[i].apply(&mut self.species);
                    break;
                }
            }
        }

        self.time += dt;
    }

    pub fn get_concentration(&self, species: &str) -> u64 {
        *self.species.get(species).unwrap_or(&0)
    }

    pub fn compute_boolean_and(&mut self, a: bool, b: bool) -> bool {
        // Implement AND gate using chemical reactions
        // A + B -> C (output)
        self.add_species("A".to_string(), if a { 100 } else { 0 });
        self.add_species("B".to_string(), if b { 100 } else { 0 });
        self.add_species("C".to_string(), 0);

        self.add_reaction(Reaction::new(
            vec![("A".to_string(), 1), ("B".to_string(), 1)],
            vec![("C".to_string(), 1)],
            1.0,
        ));

        for _ in 0..100 {
            self.step(0.01);
        }

        self.get_concentration("C") > 50
    }
}

impl Default for ReactionNetwork {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MOLECULAR MEMORY
// ============================================================================

/// Molecular memory cell
pub struct MolecularMemory {
    pub cells: Vec<DNAStrand>,
    pub capacity_bytes: usize,
}

impl MolecularMemory {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            cells: Vec::new(),
            capacity_bytes,
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<(), String> {
        // Encode bytes as DNA
        for &byte in data {
            let sequence = format!("{:08b}", byte)
                .replace('0', "AT")
                .replace('1', "GC");

            if let Some(strand) = DNAStrand::new(&sequence) {
                self.cells.push(strand);
            } else {
                return Err("Failed to encode data".to_string());
            }
        }

        Ok(())
    }

    pub fn read(&self) -> Vec<u8> {
        let mut data = Vec::new();

        for strand in &self.cells {
            let binary: String = strand.to_string().replace("AT", "0").replace("GC", "1");

            if let Ok(byte) = u8::from_str_radix(&binary, 2) {
                data.push(byte);
            }
        }

        data
    }

    pub fn storage_density_pb_per_gram() -> f64 {
        215.0 // 215 petabytes per gram
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dna_complement() {
        let strand = DNAStrand::new("ATCG").unwrap();
        let complement = strand.complement();
        assert_eq!(complement.to_string(), "TAGC");
    }

    #[test]
    fn test_dna_reverse_complement() {
        let strand = DNAStrand::new("ATCG").unwrap();
        let rc = strand.reverse_complement();
        assert_eq!(rc.to_string(), "CGAT");
    }

    #[test]
    fn test_dna_hybridization() {
        let mut computer = DNAComputer::new();
        computer.add_strand(DNAStrand::new("ATCG").unwrap());
        computer.add_strand(DNAStrand::new("TAGC").unwrap());

        let result = computer.hybridize(0, 1);
        assert!(result.is_some());
    }

    #[test]
    fn test_molecular_and_gate() {
        let gate = MolecularGate::new(GateType::AND);

        let high = MolecularSignal::new(100.0);
        let low = MolecularSignal::new(0.0);

        let result = gate.compute(&[high, high]);
        assert!(result.is_high(50.0));

        let result = gate.compute(&[high, low]);
        assert!(!result.is_high(50.0));
    }

    #[test]
    fn test_molecular_or_gate() {
        let gate = MolecularGate::new(GateType::OR);

        let high = MolecularSignal::new(100.0);
        let low = MolecularSignal::new(0.0);

        let result = gate.compute(&[high, low]);
        assert!(result.is_high(50.0));

        let result = gate.compute(&[low, low]);
        assert!(!result.is_high(50.0));
    }

    #[test]
    fn test_chemical_reaction() {
        let mut network = ReactionNetwork::new();
        network.add_species("A".to_string(), 100);
        network.add_species("B".to_string(), 0);

        network.add_reaction(Reaction::new(
            vec![("A".to_string(), 1)],
            vec![("B".to_string(), 1)],
            1.0,
        ));

        for _ in 0..10 {
            network.step(0.1);
        }

        assert!(network.get_concentration("B") > 0);
    }

    #[test]
    fn test_molecular_memory() {
        let mut memory = MolecularMemory::new(1024);
        let data = vec![0x42, 0xFF, 0x00];

        memory.write(&data).unwrap();
        let read_data = memory.read();

        assert_eq!(data, read_data);
    }
}
