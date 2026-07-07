//! Nanomaterials: graphene, carbon nanotubes, quantum dots, electronic properties

use ndarray::{Array1, Array2};
use num_complex::Complex64;
use std::f64::consts::PI;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum NanoError {
    #[error("Invalid structure: {0}")]
    InvalidStructure(String),
    #[error("Calculation failed: {0}")]
    CalculationFailed(String),
}

pub type Result<T> = std::result::Result<T, NanoError>;

/// Lattice structure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatticeType {
    Graphene,
    Diamond,
    FCC,
    BCC,
    HCP,
}

/// Carbon nanotube chirality
#[derive(Debug, Clone, Copy)]
pub struct Chirality {
    pub n: i32,
    pub m: i32,
}

impl Chirality {
    pub fn new(n: i32, m: i32) -> Self {
        Self { n, m }
    }

    /// Diameter (nm)
    pub fn diameter(&self) -> f64 {
        let a = 0.246; // graphene lattice constant (nm)
        a * ((self.n * self.n + self.m * self.m + self.n * self.m) as f64).sqrt() / PI
    }

    /// Chiral angle (degrees)
    pub fn chiral_angle(&self) -> f64 {
        let n = self.n as f64;
        let m = self.m as f64;
        (3.0_f64.sqrt() * m / (2.0 * n + m)).atan().to_degrees()
    }

    /// Is metallic?
    pub fn is_metallic(&self) -> bool {
        (self.n - self.m) % 3 == 0
    }

    /// Band gap (eV) for semiconducting tubes
    pub fn band_gap(&self) -> f64 {
        if self.is_metallic() {
            0.0
        } else {
            let d = self.diameter();
            0.8 / d // Empirical formula
        }
    }
}

/// Graphene sheet
pub struct Graphene {
    pub nx: usize,
    pub ny: usize,
    pub lattice_constant: f64,
    pub positions: Vec<Array1<f64>>,
}

impl Graphene {
    pub fn new(nx: usize, ny: usize) -> Self {
        let a = 0.246; // nm
        let mut positions = Vec::new();

        // Honeycomb lattice
        for i in 0..nx {
            for j in 0..ny {
                let x = i as f64 * a * 3.0_f64.sqrt();
                let y = j as f64 * a * 3.0;

                // A sublattice
                positions.push(Array1::from_vec(vec![x, y, 0.0]));

                // B sublattice
                positions.push(Array1::from_vec(vec![x + a * 3.0_f64.sqrt() / 2.0, y + a * 1.5, 0.0]));
            }
        }

        Self {
            nx,
            ny,
            lattice_constant: a,
            positions,
        }
    }

    pub fn n_atoms(&self) -> usize {
        self.positions.len()
    }

    /// Tight-binding Hamiltonian
    pub fn hamiltonian(&self, k: &Array1<f64>) -> Array2<Complex64> {
        let n = self.n_atoms();
        let mut h = Array2::zeros((n, n));

        let t = -2.7; // hopping parameter (eV)
        let a = self.lattice_constant;

        // Nearest-neighbor hopping
        for i in 0..n {
            for j in 0..n {
                let r = &self.positions[j] - &self.positions[i];
                let dist = r.iter().map(|x| x * x).sum::<f64>().sqrt();

                if dist < a * 1.5 && dist > 0.01 {
                    let phase = Complex64::new(0.0, k.dot(&r));
                    h[[i, j]] = Complex64::new(t, 0.0) * phase.exp();
                }
            }
        }

        h
    }

    /// Band structure at k-point
    pub fn band_structure(&self, k: &Array1<f64>) -> Vec<f64> {
        let h = self.hamiltonian(k);
        // Simplified: return diagonal (should use eigenvalue solver)
        (0..h.nrows()).map(|i| h[[i, i]].re).collect()
    }

    /// Density of states (simplified)
    pub fn dos(&self, energy: f64) -> f64 {
        // Van Hove singularities in graphene
        let t = 2.7;
        if energy.abs() < t {
            1.0 / (PI * t * t - energy * energy).sqrt()
        } else {
            0.0
        }
    }

    /// Fermi velocity (m/s)
    pub fn fermi_velocity(&self) -> f64 {
        let t = 2.7; // eV
        let a = self.lattice_constant * 1e-9; // m
        3.0 * t * 1.602e-19 * a / (2.0 * 1.055e-34) // v_F ≈ 10^6 m/s
    }
}

/// Carbon nanotube
pub struct CarbonNanotube {
    pub chirality: Chirality,
    pub length: f64,
    pub positions: Vec<Array1<f64>>,
}

impl CarbonNanotube {
    pub fn new(chirality: Chirality, length: f64) -> Self {
        let mut positions = Vec::new();
        let d = chirality.diameter();
        let r = d / 2.0;
        let a = 0.246;

        // Wrap graphene into tube
        let n_circ = (2.0 * PI * r / a) as usize;
        let n_length = (length / a) as usize;

        for i in 0..n_length {
            for j in 0..n_circ {
                let theta = 2.0 * PI * j as f64 / n_circ as f64;
                let z = i as f64 * a;
                let x = r * theta.cos();
                let y = r * theta.sin();

                positions.push(Array1::from_vec(vec![x, y, z]));
            }
        }

        Self {
            chirality,
            length,
            positions,
        }
    }

    pub fn n_atoms(&self) -> usize {
        self.positions.len()
    }

    pub fn diameter(&self) -> f64 {
        self.chirality.diameter()
    }

    pub fn is_metallic(&self) -> bool {
        self.chirality.is_metallic()
    }

    pub fn band_gap(&self) -> f64 {
        self.chirality.band_gap()
    }

    /// Conductance (quantum of conductance G0 = 2e²/h)
    pub fn conductance(&self) -> f64 {
        let g0 = 7.748e-5; // S (Siemens)
        if self.is_metallic() {
            2.0 * g0 // Two conducting channels
        } else {
            0.0
        }
    }
}

/// Quantum dot
pub struct QuantumDot {
    pub radius: f64,
    pub material: String,
    pub n_electrons: usize,
}

impl QuantumDot {
    pub fn new(radius: f64, material: String, n_electrons: usize) -> Self {
        Self {
            radius,
            material,
            n_electrons,
        }
    }

    /// Confinement energy (eV)
    pub fn confinement_energy(&self) -> f64 {
        let hbar = 1.055e-34; // J·s
        let m_e = 9.109e-31; // kg
        let r = self.radius * 1e-9; // m
        let e_j = hbar * hbar * PI * PI / (2.0 * m_e * r * r);
        e_j / 1.602e-19 // Convert to eV
    }

    /// Band gap (eV) - quantum confinement effect
    pub fn band_gap(&self) -> f64 {
        let bulk_gap = match self.material.as_str() {
            "CdSe" => 1.74,
            "CdS" => 2.42,
            "PbS" => 0.37,
            "InP" => 1.35,
            _ => 1.0,
        };

        bulk_gap + self.confinement_energy()
    }

    /// Emission wavelength (nm)
    pub fn emission_wavelength(&self) -> f64 {
        let h = 4.136e-15; // eV·s
        let c = 3e8; // m/s
        let e_gap = self.band_gap();
        h * c / e_gap * 1e9
    }

    /// Coulomb blockade energy (eV)
    pub fn coulomb_blockade(&self) -> f64 {
        let e = 1.602e-19; // C
        let epsilon_0 = 8.854e-12; // F/m
        let epsilon_r = 10.0; // Relative permittivity
        let r = self.radius * 1e-9; // m

        let c = 4.0 * PI * epsilon_0 * epsilon_r * r; // Capacitance
        (e * e / c) / 1.602e-19 // eV
    }
}

/// 2D material properties
pub struct Material2D {
    pub name: String,
    pub lattice_type: LatticeType,
    pub band_gap: f64,
    pub electron_mobility: f64,
    pub thermal_conductivity: f64,
}

impl Material2D {
    pub fn graphene() -> Self {
        Self {
            name: "Graphene".to_string(),
            lattice_type: LatticeType::Graphene,
            band_gap: 0.0,
            electron_mobility: 200000.0, // cm²/V·s
            thermal_conductivity: 5000.0, // W/m·K
        }
    }

    pub fn mos2() -> Self {
        Self {
            name: "MoS2".to_string(),
            lattice_type: LatticeType::Graphene,
            band_gap: 1.8,
            electron_mobility: 200.0,
            thermal_conductivity: 50.0,
        }
    }

    pub fn hbn() -> Self {
        Self {
            name: "h-BN".to_string(),
            lattice_type: LatticeType::Graphene,
            band_gap: 5.9,
            electron_mobility: 1.0,
            thermal_conductivity: 600.0,
        }
    }

    pub fn phosphorene() -> Self {
        Self {
            name: "Phosphorene".to_string(),
            lattice_type: LatticeType::Graphene,
            band_gap: 1.5,
            electron_mobility: 1000.0,
            thermal_conductivity: 30.0,
        }
    }
}

/// Nanomaterial simulator
pub struct NanoSimulator {
    pub graphene: Option<Graphene>,
    pub nanotube: Option<CarbonNanotube>,
    pub quantum_dot: Option<QuantumDot>,
}

impl NanoSimulator {
    pub fn new() -> Self {
        Self {
            graphene: None,
            nanotube: None,
            quantum_dot: None,
        }
    }

    pub fn create_graphene(&mut self, nx: usize, ny: usize) {
        self.graphene = Some(Graphene::new(nx, ny));
    }

    pub fn create_nanotube(&mut self, n: i32, m: i32, length: f64) {
        let chirality = Chirality::new(n, m);
        self.nanotube = Some(CarbonNanotube::new(chirality, length));
    }

    pub fn create_quantum_dot(&mut self, radius: f64, material: String, n_electrons: usize) {
        self.quantum_dot = Some(QuantumDot::new(radius, material, n_electrons));
    }

    pub fn graphene_fermi_velocity(&self) -> Result<f64> {
        self.graphene
            .as_ref()
            .map(|g| g.fermi_velocity())
            .ok_or_else(|| NanoError::InvalidStructure("No graphene".to_string()))
    }

    pub fn nanotube_conductance(&self) -> Result<f64> {
        self.nanotube
            .as_ref()
            .map(|nt| nt.conductance())
            .ok_or_else(|| NanoError::InvalidStructure("No nanotube".to_string()))
    }

    pub fn quantum_dot_wavelength(&self) -> Result<f64> {
        self.quantum_dot
            .as_ref()
            .map(|qd| qd.emission_wavelength())
            .ok_or_else(|| NanoError::InvalidStructure("No quantum dot".to_string()))
    }
}

impl Default for NanoSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_chirality() {
        let c = Chirality::new(10, 10);
        assert!(c.is_metallic());
        assert_relative_eq!(c.band_gap(), 0.0);

        let c2 = Chirality::new(10, 0);
        assert!(!c2.is_metallic());
        assert!(c2.band_gap() > 0.0);
    }

    #[test]
    fn test_graphene() {
        let g = Graphene::new(5, 5);
        assert_eq!(g.n_atoms(), 50);

        let vf = g.fermi_velocity();
        assert!(vf > 5e5 && vf < 2e6); // ~10^6 m/s
    }

    #[test]
    fn test_nanotube() {
        let nt = CarbonNanotube::new(Chirality::new(5, 5), 10.0);
        assert!(nt.is_metallic());
        assert!(nt.conductance() > 0.0);
    }

    #[test]
    fn test_quantum_dot() {
        let qd = QuantumDot::new(2.0, "CdSe".to_string(), 100);
        let wavelength = qd.emission_wavelength();
        assert!(wavelength > 400.0 && wavelength < 800.0); // Visible range
    }

    #[test]
    fn test_2d_materials() {
        let graphene = Material2D::graphene();
        assert_eq!(graphene.band_gap, 0.0);
        assert!(graphene.electron_mobility > 100000.0);

        let mos2 = Material2D::mos2();
        assert!(mos2.band_gap > 1.0);
    }

    #[test]
    fn test_simulator() {
        let mut sim = NanoSimulator::new();
        sim.create_graphene(10, 10);
        assert!(sim.graphene_fermi_velocity().is_ok());

        sim.create_nanotube(5, 5, 10.0);
        assert!(sim.nanotube_conductance().is_ok());

        sim.create_quantum_dot(3.0, "CdSe".to_string(), 100);
        assert!(sim.quantum_dot_wavelength().is_ok());
    }
}
