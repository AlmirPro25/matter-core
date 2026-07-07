//! Molecular Dynamics: protein folding, force fields, MD simulation

use ndarray::Array1;
use rand::Rng;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum MolecularError {
    #[error("Invalid structure: {0}")]
    InvalidStructure(String),
    #[error("Simulation diverged: {0}")]
    Divergence(String),
}

pub type Result<T> = std::result::Result<T, MolecularError>;

/// Atom types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomType {
    Carbon,
    Nitrogen,
    Oxygen,
    Hydrogen,
    Sulfur,
}

impl AtomType {
    pub fn mass(&self) -> f64 {
        match self {
            AtomType::Carbon => 12.011,
            AtomType::Nitrogen => 14.007,
            AtomType::Oxygen => 15.999,
            AtomType::Hydrogen => 1.008,
            AtomType::Sulfur => 32.065,
        }
    }

    pub fn vdw_radius(&self) -> f64 {
        match self {
            AtomType::Carbon => 1.70,
            AtomType::Nitrogen => 1.55,
            AtomType::Oxygen => 1.52,
            AtomType::Hydrogen => 1.20,
            AtomType::Sulfur => 1.80,
        }
    }
}

/// Atom in 3D space
#[derive(Debug, Clone)]
pub struct Atom {
    pub atom_type: AtomType,
    pub position: Array1<f64>,
    pub velocity: Array1<f64>,
    pub force: Array1<f64>,
    pub charge: f64,
}

impl Atom {
    pub fn new(atom_type: AtomType, position: Array1<f64>) -> Self {
        Self {
            atom_type,
            position,
            velocity: Array1::zeros(3),
            force: Array1::zeros(3),
            charge: 0.0,
        }
    }

    pub fn mass(&self) -> f64 {
        self.atom_type.mass()
    }
}

/// Chemical bond
#[derive(Debug, Clone)]
pub struct Bond {
    pub atom1: usize,
    pub atom2: usize,
    pub bond_type: BondType,
    pub equilibrium_length: f64,
    pub force_constant: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BondType {
    Single,
    Double,
    Triple,
}

/// Molecule structure
#[derive(Debug, Clone)]
pub struct Molecule {
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    pub name: String,
}

impl Molecule {
    pub fn new(name: String) -> Self {
        Self {
            atoms: Vec::new(),
            bonds: Vec::new(),
            name,
        }
    }

    pub fn add_atom(&mut self, atom: Atom) -> usize {
        self.atoms.push(atom);
        self.atoms.len() - 1
    }

    pub fn add_bond(&mut self, bond: Bond) {
        self.bonds.push(bond);
    }

    pub fn molecular_weight(&self) -> f64 {
        self.atoms.iter().map(|a| a.mass()).sum()
    }

    pub fn center_of_mass(&self) -> Array1<f64> {
        let mut com = Array1::zeros(3);
        let mut total_mass = 0.0;

        for atom in &self.atoms {
            com = &com + &(&atom.position * atom.mass());
            total_mass += atom.mass();
        }

        &com / total_mass
    }
}

/// Force field (AMBER-like)
pub struct ForceField {
    pub bond_k: f64,
    pub angle_k: f64,
    pub dihedral_k: f64,
    pub vdw_epsilon: f64,
    pub coulomb_k: f64,
}

impl ForceField {
    pub fn amber() -> Self {
        Self {
            bond_k: 1000.0,
            angle_k: 100.0,
            dihedral_k: 10.0,
            vdw_epsilon: 0.1,
            coulomb_k: 332.0,
        }
    }

    /// Bond stretching energy: E = k(r - r0)²
    pub fn bond_energy(&self, bond: &Bond, atoms: &[Atom]) -> f64 {
        let r1 = &atoms[bond.atom1].position;
        let r2 = &atoms[bond.atom2].position;
        let r = (r2 - r1).iter().map(|x| x * x).sum::<f64>().sqrt();
        let dr = r - bond.equilibrium_length;
        0.5 * bond.force_constant * dr * dr
    }

    /// Lennard-Jones potential: E = 4ε[(σ/r)¹² - (σ/r)⁶]
    pub fn vdw_energy(&self, atom1: &Atom, atom2: &Atom) -> f64 {
        let r_vec = &atom2.position - &atom1.position;
        let r = r_vec.iter().map(|x| x * x).sum::<f64>().sqrt();

        if r < 0.1 {
            return 1e10;
        }

        let sigma = (atom1.atom_type.vdw_radius() + atom2.atom_type.vdw_radius()) / 2.0;
        let sr6 = (sigma / r).powi(6);
        let sr12 = sr6 * sr6;

        4.0 * self.vdw_epsilon * (sr12 - sr6)
    }

    /// Coulomb energy: E = k q1 q2 / r
    pub fn coulomb_energy(&self, atom1: &Atom, atom2: &Atom) -> f64 {
        let r_vec = &atom2.position - &atom1.position;
        let r = r_vec.iter().map(|x| x * x).sum::<f64>().sqrt();

        if r < 0.1 {
            return 0.0;
        }

        self.coulomb_k * atom1.charge * atom2.charge / r
    }

    /// Total potential energy
    pub fn total_energy(&self, molecule: &Molecule) -> f64 {
        let mut energy = 0.0;

        // Bond energy
        for bond in &molecule.bonds {
            energy += self.bond_energy(bond, &molecule.atoms);
        }

        // Non-bonded interactions
        let n = molecule.atoms.len();
        for i in 0..n {
            for j in (i + 1)..n {
                energy += self.vdw_energy(&molecule.atoms[i], &molecule.atoms[j]);
                energy += self.coulomb_energy(&molecule.atoms[i], &molecule.atoms[j]);
            }
        }

        energy
    }

    /// Calculate forces on all atoms
    pub fn calculate_forces(&self, molecule: &mut Molecule) {
        // Reset forces
        for atom in &mut molecule.atoms {
            atom.force = Array1::zeros(3);
        }

        // Bond forces
        for bond in &molecule.bonds {
            let r1 = molecule.atoms[bond.atom1].position.clone();
            let r2 = molecule.atoms[bond.atom2].position.clone();
            let r_vec = &r2 - &r1;
            let r = r_vec.iter().map(|x| x * x).sum::<f64>().sqrt();

            if r < 0.01 {
                continue;
            }

            let dr = r - bond.equilibrium_length;
            let f_mag = -bond.force_constant * dr;
            let force = &r_vec * (f_mag / r);

            molecule.atoms[bond.atom1].force = &molecule.atoms[bond.atom1].force - &force;
            molecule.atoms[bond.atom2].force = &molecule.atoms[bond.atom2].force + &force;
        }

        // Non-bonded forces (simplified)
        let n = molecule.atoms.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let r1 = molecule.atoms[i].position.clone();
                let r2 = molecule.atoms[j].position.clone();
                let r_vec = &r2 - &r1;
                let r = r_vec.iter().map(|x| x * x).sum::<f64>().sqrt();

                if r < 0.1 {
                    continue;
                }

                // LJ force
                let sigma = (molecule.atoms[i].atom_type.vdw_radius()
                    + molecule.atoms[j].atom_type.vdw_radius())
                    / 2.0;
                let sr6 = (sigma / r).powi(6);
                let sr12 = sr6 * sr6;
                let f_lj = 24.0 * self.vdw_epsilon * (2.0 * sr12 - sr6) / r;

                // Coulomb force
                let f_coul = self.coulomb_k * molecule.atoms[i].charge * molecule.atoms[j].charge
                    / (r * r);

                let f_total = (f_lj + f_coul) / r;
                let force = &r_vec * f_total;

                molecule.atoms[i].force = &molecule.atoms[i].force - &force;
                molecule.atoms[j].force = &molecule.atoms[j].force + &force;
            }
        }
    }
}

/// Molecular Dynamics simulation
pub struct MDSimulation {
    pub molecule: Molecule,
    pub force_field: ForceField,
    pub temperature: f64,
    pub timestep: f64,
    pub time: f64,
}

impl MDSimulation {
    pub fn new(molecule: Molecule, temperature: f64) -> Self {
        Self {
            molecule,
            force_field: ForceField::amber(),
            temperature,
            timestep: 0.001,
            time: 0.0,
        }
    }

    /// Initialize velocities (Maxwell-Boltzmann)
    pub fn initialize_velocities(&mut self) {
        let kb = 0.001987; // kcal/mol/K
        let mut rng = rand::thread_rng();

        for atom in &mut self.molecule.atoms {
            let sigma = (kb * self.temperature / atom.mass()).sqrt();
            atom.velocity = Array1::from_vec(vec![
                rng.gen::<f64>() * sigma - sigma / 2.0,
                rng.gen::<f64>() * sigma - sigma / 2.0,
                rng.gen::<f64>() * sigma - sigma / 2.0,
            ]);
        }
    }

    /// Velocity Verlet integration
    pub fn step(&mut self) {
        let dt = self.timestep;

        // Update positions: r(t+dt) = r(t) + v(t)dt + 0.5a(t)dt²
        for atom in &mut self.molecule.atoms {
            let acc = &atom.force / atom.mass();
            atom.position = &atom.position + &(&atom.velocity * dt) + &(&acc * (0.5 * dt * dt));
        }

        // Calculate new forces
        self.force_field.calculate_forces(&mut self.molecule);

        // Update velocities: v(t+dt) = v(t) + 0.5[a(t) + a(t+dt)]dt
        for atom in &mut self.molecule.atoms {
            let acc = &atom.force / atom.mass();
            atom.velocity = &atom.velocity + &(&acc * dt);
        }

        self.time += dt;
    }

    /// Run simulation for n steps
    pub fn run(&mut self, steps: usize) {
        self.force_field.calculate_forces(&mut self.molecule);

        for _ in 0..steps {
            self.step();
        }
    }

    /// Calculate kinetic energy
    pub fn kinetic_energy(&self) -> f64 {
        self.molecule
            .atoms
            .iter()
            .map(|atom| {
                let v2: f64 = atom.velocity.iter().map(|v| v * v).sum();
                0.5 * atom.mass() * v2
            })
            .sum()
    }

    /// Calculate total energy
    pub fn total_energy(&self) -> f64 {
        self.kinetic_energy() + self.force_field.total_energy(&self.molecule)
    }

    /// Calculate temperature from kinetic energy
    pub fn current_temperature(&self) -> f64 {
        let kb = 0.001987;
        let n_atoms = self.molecule.atoms.len();
        let ke = self.kinetic_energy();
        2.0 * ke / (3.0 * n_atoms as f64 * kb)
    }
}

/// Protein structure prediction (simplified AlphaFold-like)
pub struct ProteinFolder {
    pub sequence: String,
    pub structure: Option<Molecule>,
}

impl ProteinFolder {
    pub fn new(sequence: String) -> Self {
        Self {
            sequence,
            structure: None,
        }
    }

    /// Predict structure (simplified)
    pub fn fold(&mut self) -> Result<()> {
        let mut molecule = Molecule::new("protein".to_string());

        // Build backbone (simplified)
        let mut prev_c = None;
        for (i, _aa) in self.sequence.chars().enumerate() {
            let x = i as f64 * 3.8;

            // N
            let n = Atom::new(
                AtomType::Nitrogen,
                Array1::from_vec(vec![x, 0.0, 0.0]),
            );
            let n_idx = molecule.add_atom(n);

            // CA
            let ca = Atom::new(
                AtomType::Carbon,
                Array1::from_vec(vec![x + 1.5, 0.0, 0.0]),
            );
            let ca_idx = molecule.add_atom(ca);

            // C
            let c = Atom::new(
                AtomType::Carbon,
                Array1::from_vec(vec![x + 3.0, 0.0, 0.0]),
            );
            let c_idx = molecule.add_atom(c);

            // Bonds
            molecule.add_bond(Bond {
                atom1: n_idx,
                atom2: ca_idx,
                bond_type: BondType::Single,
                equilibrium_length: 1.47,
                force_constant: 1000.0,
            });

            molecule.add_bond(Bond {
                atom1: ca_idx,
                atom2: c_idx,
                bond_type: BondType::Single,
                equilibrium_length: 1.52,
                force_constant: 1000.0,
            });

            // Peptide bond
            if let Some(prev_c_idx) = prev_c {
                molecule.add_bond(Bond {
                    atom1: prev_c_idx,
                    atom2: n_idx,
                    bond_type: BondType::Single,
                    equilibrium_length: 1.33,
                    force_constant: 1000.0,
                });
            }

            prev_c = Some(c_idx);
        }

        self.structure = Some(molecule);
        Ok(())
    }

    /// Optimize structure with MD
    pub fn optimize(&mut self, steps: usize) -> Result<()> {
        if let Some(molecule) = self.structure.take() {
            let mut sim = MDSimulation::new(molecule, 300.0);
            sim.initialize_velocities();
            sim.run(steps);
            self.structure = Some(sim.molecule);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_atom_mass() {
        let atom = Atom::new(AtomType::Carbon, Array1::zeros(3));
        assert_relative_eq!(atom.mass(), 12.011, epsilon = 0.001);
    }

    #[test]
    fn test_molecule_weight() {
        let mut mol = Molecule::new("test".to_string());
        mol.add_atom(Atom::new(AtomType::Carbon, Array1::zeros(3)));
        mol.add_atom(Atom::new(AtomType::Oxygen, Array1::zeros(3)));

        let mw = mol.molecular_weight();
        assert_relative_eq!(mw, 28.01, epsilon = 0.01);
    }

    #[test]
    fn test_bond_energy() {
        let mut mol = Molecule::new("test".to_string());
        let a1 = mol.add_atom(Atom::new(
            AtomType::Carbon,
            Array1::from_vec(vec![0.0, 0.0, 0.0]),
        ));
        let a2 = mol.add_atom(Atom::new(
            AtomType::Carbon,
            Array1::from_vec(vec![1.5, 0.0, 0.0]),
        ));

        let bond = Bond {
            atom1: a1,
            atom2: a2,
            bond_type: BondType::Single,
            equilibrium_length: 1.54,
            force_constant: 1000.0,
        };

        let ff = ForceField::amber();
        let energy = ff.bond_energy(&bond, &mol.atoms);

        assert!(energy >= 0.0);
    }

    #[test]
    fn test_md_energy_conservation() {
        let mut mol = Molecule::new("test".to_string());
        mol.add_atom(Atom::new(
            AtomType::Carbon,
            Array1::from_vec(vec![0.0, 0.0, 0.0]),
        ));
        mol.add_atom(Atom::new(
            AtomType::Carbon,
            Array1::from_vec(vec![1.5, 0.0, 0.0]),
        ));

        let mut sim = MDSimulation::new(mol, 300.0);
        sim.initialize_velocities();

        let e0 = sim.total_energy();
        sim.run(100);
        let e1 = sim.total_energy();

        // Energy should be roughly conserved
        assert_relative_eq!(e0, e1, epsilon = 0.5);
    }

    #[test]
    fn test_protein_folding() {
        let mut folder = ProteinFolder::new("ACDEFGHIKLMNPQRSTVWY".to_string());
        assert!(folder.fold().is_ok());
        assert!(folder.structure.is_some());
    }
}
