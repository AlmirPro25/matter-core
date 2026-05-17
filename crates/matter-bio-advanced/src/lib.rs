//! # Matter Bio Advanced
//!
//! Advanced biological computing for Matter language.
//! Includes protein folding, molecular dynamics, CRISPR design, and synthetic biology.

use std::collections::HashMap;

// ============================================================================
// PROTEIN FOLDING (AlphaFold-like)
// ============================================================================

/// Amino acid representation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AminoAcid {
    Ala,
    Arg,
    Asn,
    Asp,
    Cys,
    Gln,
    Glu,
    Gly,
    His,
    Ile,
    Leu,
    Lys,
    Met,
    Phe,
    Pro,
    Ser,
    Thr,
    Trp,
    Tyr,
    Val,
}

impl AminoAcid {
    pub fn from_code(code: char) -> Option<Self> {
        match code {
            'A' => Some(AminoAcid::Ala),
            'R' => Some(AminoAcid::Arg),
            'N' => Some(AminoAcid::Asn),
            'D' => Some(AminoAcid::Asp),
            'C' => Some(AminoAcid::Cys),
            'Q' => Some(AminoAcid::Gln),
            'E' => Some(AminoAcid::Glu),
            'G' => Some(AminoAcid::Gly),
            'H' => Some(AminoAcid::His),
            'I' => Some(AminoAcid::Ile),
            'L' => Some(AminoAcid::Leu),
            'K' => Some(AminoAcid::Lys),
            'M' => Some(AminoAcid::Met),
            'F' => Some(AminoAcid::Phe),
            'P' => Some(AminoAcid::Pro),
            'S' => Some(AminoAcid::Ser),
            'T' => Some(AminoAcid::Thr),
            'W' => Some(AminoAcid::Trp),
            'Y' => Some(AminoAcid::Tyr),
            'V' => Some(AminoAcid::Val),
            _ => None,
        }
    }

    pub fn hydrophobicity(&self) -> f64 {
        match self {
            AminoAcid::Ile | AminoAcid::Val | AminoAcid::Leu => 4.5,
            AminoAcid::Phe | AminoAcid::Met => 3.8,
            AminoAcid::Trp | AminoAcid::Ala => 3.1,
            AminoAcid::Cys | AminoAcid::Gly => 2.5,
            AminoAcid::Tyr | AminoAcid::Pro => 1.9,
            AminoAcid::Thr | AminoAcid::Ser => -0.7,
            AminoAcid::His | AminoAcid::Glu => -3.2,
            AminoAcid::Asn | AminoAcid::Gln => -3.5,
            AminoAcid::Asp | AminoAcid::Lys => -3.9,
            AminoAcid::Arg => -4.5,
        }
    }

    pub fn charge(&self) -> f64 {
        match self {
            AminoAcid::Arg | AminoAcid::Lys => 1.0,
            AminoAcid::Asp | AminoAcid::Glu => -1.0,
            AminoAcid::His => 0.5,
            _ => 0.0,
        }
    }
}

/// 3D coordinate
#[derive(Debug, Clone, Copy)]
pub struct Coord3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Coord3D {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn distance(&self, other: &Coord3D) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Protein structure
#[derive(Debug, Clone)]
pub struct ProteinStructure {
    pub sequence: Vec<AminoAcid>,
    pub coordinates: Vec<Coord3D>,
    pub confidence: Vec<f64>,
}

impl ProteinStructure {
    pub fn new(sequence: Vec<AminoAcid>) -> Self {
        let len = sequence.len();
        Self {
            sequence,
            coordinates: vec![Coord3D::new(0.0, 0.0, 0.0); len],
            confidence: vec![0.0; len],
        }
    }

    pub fn from_sequence(seq: &str) -> Option<Self> {
        let sequence: Option<Vec<_>> = seq.chars().map(AminoAcid::from_code).collect();
        sequence.map(Self::new)
    }

    pub fn calculate_energy(&self) -> f64 {
        let mut energy = 0.0;

        // Hydrophobic interactions
        for i in 0..self.sequence.len() {
            for j in (i + 1)..self.sequence.len() {
                let dist = self.coordinates[i].distance(&self.coordinates[j]);
                if dist < 8.0 {
                    let h1 = self.sequence[i].hydrophobicity();
                    let h2 = self.sequence[j].hydrophobicity();
                    energy -= h1 * h2 / (dist * dist);
                }
            }
        }

        // Electrostatic interactions
        for i in 0..self.sequence.len() {
            for j in (i + 1)..self.sequence.len() {
                let dist = self.coordinates[i].distance(&self.coordinates[j]);
                if dist < 12.0 {
                    let q1 = self.sequence[i].charge();
                    let q2 = self.sequence[j].charge();
                    energy += q1 * q2 / dist;
                }
            }
        }

        energy
    }
}

/// Protein folding predictor (simplified AlphaFold-like)
pub struct ProteinFolder {
    max_iterations: usize,
    learning_rate: f64,
}

impl ProteinFolder {
    pub fn new(max_iterations: usize, learning_rate: f64) -> Self {
        Self {
            max_iterations,
            learning_rate,
        }
    }

    pub fn fold(&self, structure: &mut ProteinStructure) -> f64 {
        // Initialize random structure
        for i in 0..structure.coordinates.len() {
            structure.coordinates[i] = Coord3D::new((i as f64) * 3.8, 0.0, 0.0);
        }

        let mut best_energy = structure.calculate_energy();

        // Gradient descent optimization
        for iteration in 0..self.max_iterations {
            let current_energy = structure.calculate_energy();

            // Update coordinates
            for i in 0..structure.coordinates.len() {
                let old_coord = structure.coordinates[i];

                // Try small perturbations
                for dx in [-1.0, 0.0, 1.0] {
                    for dy in [-1.0, 0.0, 1.0] {
                        for dz in [-1.0, 0.0, 1.0] {
                            structure.coordinates[i] = Coord3D::new(
                                old_coord.x + dx * self.learning_rate,
                                old_coord.y + dy * self.learning_rate,
                                old_coord.z + dz * self.learning_rate,
                            );

                            let new_energy = structure.calculate_energy();
                            if new_energy < best_energy {
                                best_energy = new_energy;
                            } else {
                                structure.coordinates[i] = old_coord;
                            }
                        }
                    }
                }
            }

            // Update confidence based on energy convergence
            if iteration > 0 {
                let convergence = (current_energy - best_energy).abs() / best_energy.abs().max(1.0);
                for i in 0..structure.confidence.len() {
                    structure.confidence[i] = 1.0 - convergence;
                }
            }
        }

        best_energy
    }

    pub fn predict_binding_site(&self, structure: &ProteinStructure) -> Vec<usize> {
        let mut sites = Vec::new();

        // Find hydrophobic pockets
        for i in 0..structure.sequence.len() {
            if structure.sequence[i].hydrophobicity() > 3.0 {
                let mut neighbor_count = 0;
                for j in 0..structure.sequence.len() {
                    if i != j {
                        let dist = structure.coordinates[i].distance(&structure.coordinates[j]);
                        if dist < 6.0 {
                            neighbor_count += 1;
                        }
                    }
                }
                if neighbor_count >= 3 {
                    sites.push(i);
                }
            }
        }

        sites
    }
}

// ============================================================================
// MOLECULAR DYNAMICS
// ============================================================================

/// Atom in molecular dynamics simulation
#[derive(Debug, Clone)]
pub struct Atom {
    pub position: Coord3D,
    pub velocity: Coord3D,
    pub force: Coord3D,
    pub mass: f64,
    pub charge: f64,
}

impl Atom {
    pub fn new(position: Coord3D, mass: f64, charge: f64) -> Self {
        Self {
            position,
            velocity: Coord3D::new(0.0, 0.0, 0.0),
            force: Coord3D::new(0.0, 0.0, 0.0),
            mass,
            charge,
        }
    }
}

/// Molecular dynamics simulator
pub struct MolecularDynamics {
    pub atoms: Vec<Atom>,
    pub timestep: f64,
    pub temperature: f64,
}

impl MolecularDynamics {
    pub fn new(atoms: Vec<Atom>, timestep: f64, temperature: f64) -> Self {
        Self {
            atoms,
            timestep,
            temperature,
        }
    }

    pub fn calculate_forces(&mut self) {
        // Reset forces
        for atom in &mut self.atoms {
            atom.force = Coord3D::new(0.0, 0.0, 0.0);
        }

        // Lennard-Jones potential
        for i in 0..self.atoms.len() {
            for j in (i + 1)..self.atoms.len() {
                let dx = self.atoms[j].position.x - self.atoms[i].position.x;
                let dy = self.atoms[j].position.y - self.atoms[i].position.y;
                let dz = self.atoms[j].position.z - self.atoms[i].position.z;
                let r2 = dx * dx + dy * dy + dz * dz;
                let r = r2.sqrt();

                if r < 10.0 {
                    // Lennard-Jones force
                    let sigma = 3.4; // Angstroms
                    let epsilon = 0.238; // kcal/mol
                    let sr6 = (sigma / r).powi(6);
                    let sr12 = sr6 * sr6;
                    let force_mag = 24.0 * epsilon * (2.0 * sr12 - sr6) / r;

                    let fx = force_mag * dx / r;
                    let fy = force_mag * dy / r;
                    let fz = force_mag * dz / r;

                    self.atoms[i].force.x += fx;
                    self.atoms[i].force.y += fy;
                    self.atoms[i].force.z += fz;
                    self.atoms[j].force.x -= fx;
                    self.atoms[j].force.y -= fy;
                    self.atoms[j].force.z -= fz;

                    // Coulomb force
                    let ke = 332.0; // kcal*A/(mol*e^2)
                    let coulomb_force = ke * self.atoms[i].charge * self.atoms[j].charge / r2;
                    let cfx = coulomb_force * dx / r;
                    let cfy = coulomb_force * dy / r;
                    let cfz = coulomb_force * dz / r;

                    self.atoms[i].force.x += cfx;
                    self.atoms[i].force.y += cfy;
                    self.atoms[i].force.z += cfz;
                    self.atoms[j].force.x -= cfx;
                    self.atoms[j].force.y -= cfy;
                    self.atoms[j].force.z -= cfz;
                }
            }
        }
    }

    pub fn step(&mut self) {
        self.calculate_forces();

        // Velocity Verlet integration
        for atom in &mut self.atoms {
            // Update positions
            atom.position.x += atom.velocity.x * self.timestep
                + 0.5 * atom.force.x / atom.mass * self.timestep * self.timestep;
            atom.position.y += atom.velocity.y * self.timestep
                + 0.5 * atom.force.y / atom.mass * self.timestep * self.timestep;
            atom.position.z += atom.velocity.z * self.timestep
                + 0.5 * atom.force.z / atom.mass * self.timestep * self.timestep;

            // Update velocities (half step)
            atom.velocity.x += 0.5 * atom.force.x / atom.mass * self.timestep;
            atom.velocity.y += 0.5 * atom.force.y / atom.mass * self.timestep;
            atom.velocity.z += 0.5 * atom.force.z / atom.mass * self.timestep;
        }

        self.calculate_forces();

        // Complete velocity update
        for atom in &mut self.atoms {
            atom.velocity.x += 0.5 * atom.force.x / atom.mass * self.timestep;
            atom.velocity.y += 0.5 * atom.force.y / atom.mass * self.timestep;
            atom.velocity.z += 0.5 * atom.force.z / atom.mass * self.timestep;
        }
    }

    pub fn calculate_energy(&self) -> (f64, f64) {
        let mut kinetic = 0.0;
        let mut potential = 0.0;

        // Kinetic energy
        for atom in &self.atoms {
            let v2 = atom.velocity.x * atom.velocity.x
                + atom.velocity.y * atom.velocity.y
                + atom.velocity.z * atom.velocity.z;
            kinetic += 0.5 * atom.mass * v2;
        }

        // Potential energy
        for i in 0..self.atoms.len() {
            for j in (i + 1)..self.atoms.len() {
                let r = self.atoms[i].position.distance(&self.atoms[j].position);
                if r < 10.0 {
                    let sigma = 3.4;
                    let epsilon = 0.238;
                    let sr6 = (sigma / r).powi(6);
                    let sr12 = sr6 * sr6;
                    potential += 4.0 * epsilon * (sr12 - sr6);

                    let ke = 332.0;
                    potential += ke * self.atoms[i].charge * self.atoms[j].charge / r;
                }
            }
        }

        (kinetic, potential)
    }
}

// ============================================================================
// CRISPR DESIGN
// ============================================================================

/// CRISPR guide RNA
#[derive(Debug, Clone)]
pub struct GuideRNA {
    pub sequence: String,
    pub pam: String,
    pub target_position: usize,
}

impl GuideRNA {
    pub fn new(sequence: String, pam: String, target_position: usize) -> Self {
        Self {
            sequence,
            pam,
            target_position,
        }
    }

    pub fn calculate_specificity(&self, genome: &str) -> f64 {
        let mut matches = 0;
        let target = &self.sequence;

        for i in 0..genome.len().saturating_sub(target.len()) {
            let window = &genome[i..i + target.len()];
            let mismatches = target
                .chars()
                .zip(window.chars())
                .filter(|(a, b)| a != b)
                .count();

            if mismatches <= 3 {
                matches += 1;
            }
        }

        1.0 / (matches as f64).max(1.0)
    }

    pub fn calculate_efficiency(&self) -> f64 {
        let gc_content = self
            .sequence
            .chars()
            .filter(|&c| c == 'G' || c == 'C')
            .count() as f64
            / self.sequence.len() as f64;

        // Optimal GC content is 40-60%
        if (0.4..=0.6).contains(&gc_content) {
            1.0
        } else {
            1.0 - (gc_content - 0.5).abs() * 2.0
        }
    }
}

/// CRISPR design tool
pub struct CRISPRDesigner {
    pam_sequence: String,
    guide_length: usize,
}

impl CRISPRDesigner {
    pub fn new(pam_sequence: String, guide_length: usize) -> Self {
        Self {
            pam_sequence,
            guide_length,
        }
    }

    pub fn design_guides(&self, target_gene: &str, num_guides: usize) -> Vec<GuideRNA> {
        let mut guides = Vec::new();

        // Find PAM sites
        for i in 0..target_gene.len().saturating_sub(self.pam_sequence.len()) {
            if target_gene[i..i + self.pam_sequence.len()] == self.pam_sequence
                && i >= self.guide_length
            {
                let guide_seq = target_gene[i - self.guide_length..i].to_string();
                let guide =
                    GuideRNA::new(guide_seq, self.pam_sequence.clone(), i - self.guide_length);
                guides.push(guide);
            }
        }

        // Sort by efficiency
        guides.sort_by(|a, b| {
            b.calculate_efficiency()
                .partial_cmp(&a.calculate_efficiency())
                .unwrap()
        });

        guides.into_iter().take(num_guides).collect()
    }

    pub fn predict_off_targets(&self, guide: &GuideRNA, genome: &str) -> Vec<(usize, usize)> {
        let mut off_targets = Vec::new();
        let target = &guide.sequence;

        for i in 0..genome.len().saturating_sub(target.len()) {
            let window = &genome[i..i + target.len()];
            let mismatches = target
                .chars()
                .zip(window.chars())
                .filter(|(a, b)| a != b)
                .count();

            if mismatches > 0 && mismatches <= 3 {
                off_targets.push((i, mismatches));
            }
        }

        off_targets
    }
}

// ============================================================================
// SYNTHETIC BIOLOGY CIRCUITS
// ============================================================================

/// Genetic part (promoter, RBS, CDS, terminator)
#[derive(Debug, Clone)]
pub enum GeneticPart {
    Promoter { name: String, strength: f64 },
    RBS { name: String, efficiency: f64 },
    CDS { name: String, protein: String },
    Terminator { name: String, efficiency: f64 },
}

/// Genetic circuit
#[derive(Debug, Clone)]
pub struct GeneticCircuit {
    pub parts: Vec<GeneticPart>,
    pub name: String,
}

impl GeneticCircuit {
    pub fn new(name: String) -> Self {
        Self {
            parts: Vec::new(),
            name,
        }
    }

    pub fn add_part(&mut self, part: GeneticPart) {
        self.parts.push(part);
    }

    pub fn calculate_expression(&self) -> f64 {
        let mut expression = 1.0;

        for part in &self.parts {
            match part {
                GeneticPart::Promoter { strength, .. } => expression *= strength,
                GeneticPart::RBS { efficiency, .. } => expression *= efficiency,
                GeneticPart::Terminator { efficiency, .. } => expression *= efficiency,
                _ => {}
            }
        }

        expression
    }

    pub fn validate(&self) -> bool {
        let mut has_promoter = false;
        let mut has_cds = false;
        let mut has_terminator = false;

        for part in &self.parts {
            match part {
                GeneticPart::Promoter { .. } => has_promoter = true,
                GeneticPart::CDS { .. } => has_cds = true,
                GeneticPart::Terminator { .. } => has_terminator = true,
                _ => {}
            }
        }

        has_promoter && has_cds && has_terminator
    }
}

/// Circuit simulator
pub struct CircuitSimulator {
    pub circuits: Vec<GeneticCircuit>,
    pub time: f64,
    pub concentrations: HashMap<String, f64>,
}

impl CircuitSimulator {
    pub fn new() -> Self {
        Self {
            circuits: Vec::new(),
            time: 0.0,
            concentrations: HashMap::new(),
        }
    }

    pub fn add_circuit(&mut self, circuit: GeneticCircuit) {
        self.circuits.push(circuit);
    }

    pub fn step(&mut self, dt: f64) {
        self.time += dt;

        // Update protein concentrations
        for circuit in &self.circuits {
            let expression = circuit.calculate_expression();

            for part in &circuit.parts {
                if let GeneticPart::CDS { protein, .. } = part {
                    let current = self.concentrations.get(protein).unwrap_or(&0.0);
                    let production = expression * dt;
                    let degradation = current * 0.1 * dt; // 10% degradation per time unit
                    self.concentrations
                        .insert(protein.clone(), current + production - degradation);
                }
            }
        }
    }

    pub fn get_concentration(&self, protein: &str) -> f64 {
        *self.concentrations.get(protein).unwrap_or(&0.0)
    }
}

impl Default for CircuitSimulator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protein_folding() {
        let mut structure = ProteinStructure::from_sequence("MKTAYIAKQRQISFVKSHFSRQLEERL").unwrap();

        let folder = ProteinFolder::new(10, 0.1);
        let energy = folder.fold(&mut structure);

        assert!(energy.is_finite());
        assert!(structure.confidence[0] > 0.0); // Should have some confidence
    }

    #[test]
    fn test_molecular_dynamics() {
        let atoms = vec![
            Atom::new(Coord3D::new(0.0, 0.0, 0.0), 12.0, 0.0),
            Atom::new(Coord3D::new(3.0, 0.0, 0.0), 12.0, 0.0),
        ];

        let mut md = MolecularDynamics::new(atoms, 0.0001, 300.0);

        let (ke_before, pe_before) = md.calculate_energy();

        for _ in 0..10 {
            md.step();
        }

        let (ke_after, pe_after) = md.calculate_energy();
        let total_before = ke_before + pe_before;
        let total_after = ke_after + pe_after;

        assert!(total_before.is_finite());
        assert!(total_after.is_finite());
        assert!(ke_after >= ke_before);
        assert!((pe_after - pe_before).abs() < 1.0);
    }

    #[test]
    fn test_crispr_design() {
        let designer = CRISPRDesigner::new("NGG".to_string(), 20);
        let target = "ATCGATCGATCGATCGATCGNGGACGTACGTACGTACGTACG";

        let guides = designer.design_guides(target, 5);

        assert!(!guides.is_empty());
        assert!(guides[0].calculate_efficiency() > 0.0);
    }

    #[test]
    fn test_genetic_circuit() {
        let mut circuit = GeneticCircuit::new("test_circuit".to_string());
        circuit.add_part(GeneticPart::Promoter {
            name: "pLac".to_string(),
            strength: 0.8,
        });
        circuit.add_part(GeneticPart::RBS {
            name: "B0034".to_string(),
            efficiency: 0.9,
        });
        circuit.add_part(GeneticPart::CDS {
            name: "gfp".to_string(),
            protein: "GFP".to_string(),
        });
        circuit.add_part(GeneticPart::Terminator {
            name: "T1".to_string(),
            efficiency: 0.95,
        });

        assert!(circuit.validate());

        let expression = circuit.calculate_expression();
        assert!(expression > 0.0 && expression < 1.0);
    }

    #[test]
    fn test_circuit_simulator() {
        let mut circuit = GeneticCircuit::new("gfp_circuit".to_string());
        circuit.add_part(GeneticPart::Promoter {
            name: "pLac".to_string(),
            strength: 0.8,
        });
        circuit.add_part(GeneticPart::RBS {
            name: "B0034".to_string(),
            efficiency: 0.9,
        });
        circuit.add_part(GeneticPart::CDS {
            name: "gfp".to_string(),
            protein: "GFP".to_string(),
        });
        circuit.add_part(GeneticPart::Terminator {
            name: "T1".to_string(),
            efficiency: 0.95,
        });

        let mut simulator = CircuitSimulator::new();
        simulator.add_circuit(circuit);

        for _ in 0..100 {
            simulator.step(0.1);
        }

        let gfp_concentration = simulator.get_concentration("GFP");
        assert!(gfp_concentration > 0.0);
    }
}
