//! Topological Materials: topological insulators, quantum Hall effect, edge states

use ndarray::{Array1, Array2};
use num_complex::Complex64;
use std::f64::consts::PI;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum TopologyError {
    #[error("Invalid topology: {0}")]
    InvalidTopology(String),
    #[error("Calculation failed: {0}")]
    CalculationFailed(String),
}

pub type Result<T> = std::result::Result<T, TopologyError>;

/// Topological invariant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologicalInvariant {
    ChernNumber(i32),
    Z2Invariant(bool),
    WindingNumber(i32),
}

/// Quantum Hall system
pub struct QuantumHall {
    pub magnetic_field: f64,
    pub filling_factor: f64,
    pub temperature: f64,
}

impl QuantumHall {
    pub fn new(magnetic_field: f64, filling_factor: f64) -> Self {
        Self {
            magnetic_field,
            filling_factor,
            temperature: 0.0,
        }
    }

    /// Landau level energy: E_n = ℏω_c(n + 1/2)
    pub fn landau_level(&self, n: usize) -> f64 {
        let hbar = 1.055e-34; // J·s
        let e = 1.602e-19; // C
        let m_e = 9.109e-31; // kg
        let b = self.magnetic_field;

        let omega_c = e * b / m_e; // Cyclotron frequency
        let e_j = hbar * omega_c * (n as f64 + 0.5);
        e_j / 1.602e-19 // eV
    }

    /// Hall conductance: σ_xy = ν e²/h
    pub fn hall_conductance(&self) -> f64 {
        let e2_h = 3.874e-5; // e²/h in S
        self.filling_factor * e2_h
    }

    /// Chern number (topological invariant)
    pub fn chern_number(&self) -> i32 {
        self.filling_factor.round() as i32
    }

    /// Magnetic length: l_B = √(ℏ/eB)
    pub fn magnetic_length(&self) -> f64 {
        let hbar = 1.055e-34;
        let e = 1.602e-19;
        let b = self.magnetic_field;
        (hbar / (e * b)).sqrt() * 1e9 // nm
    }

    /// Edge state velocity (m/s)
    pub fn edge_velocity(&self) -> f64 {
        let e = 1.602e-19;
        let h = 6.626e-34;
        let b = self.magnetic_field;
        e * b / h * 1e-9 // Simplified
    }
}

/// Topological insulator (2D/3D)
pub struct TopologicalInsulator {
    pub dimension: usize,
    pub bulk_gap: f64,
    pub spin_orbit_coupling: f64,
    pub z2_invariant: bool,
}

impl TopologicalInsulator {
    pub fn new_2d(bulk_gap: f64, spin_orbit_coupling: f64) -> Self {
        Self {
            dimension: 2,
            bulk_gap,
            spin_orbit_coupling,
            z2_invariant: true,
        }
    }

    pub fn new_3d(bulk_gap: f64, spin_orbit_coupling: f64) -> Self {
        Self {
            dimension: 3,
            bulk_gap,
            spin_orbit_coupling,
            z2_invariant: true,
        }
    }

    /// BHZ (Bernevig-Hughes-Zhang) Hamiltonian for 2D TI
    pub fn bhz_hamiltonian(&self, k: &Array1<f64>) -> Array2<Complex64> {
        let kx = k[0];
        let ky = k[1];

        let m = 0.01; // Mass term
        let a = 3.65; // Lattice constant
        let b = 0.686; // Parameter

        let h00 = Complex64::new(m - b * (kx * kx + ky * ky), 0.0);
        let h01 = Complex64::new(0.0, a * kx);
        let h10 = Complex64::new(0.0, -a * kx);
        let h11 = Complex64::new(-m + b * (kx * kx + ky * ky), 0.0);

        Array2::from_shape_vec((2, 2), vec![h00, h01, h10, h11]).unwrap()
    }

    /// Edge state dispersion
    pub fn edge_dispersion(&self, k: f64) -> f64 {
        let v = 1e6; // Edge velocity (m/s)
        let hbar = 1.055e-34;
        let e_j = hbar * v * k;
        e_j / 1.602e-19 // eV
    }

    /// Is topologically non-trivial?
    pub fn is_topological(&self) -> bool {
        self.z2_invariant
    }

    /// Surface state density (3D TI)
    pub fn surface_dos(&self, energy: f64) -> f64 {
        if self.dimension == 3 && energy.abs() < self.bulk_gap / 2.0 {
            1.0 / (2.0 * PI) // Constant DOS for Dirac cone
        } else {
            0.0
        }
    }
}

/// Weyl semimetal
pub struct WeylSemimetal {
    pub weyl_points: Vec<Array1<f64>>,
    pub chirality: Vec<i32>,
}

impl WeylSemimetal {
    pub fn new() -> Self {
        Self {
            weyl_points: Vec::new(),
            chirality: Vec::new(),
        }
    }

    pub fn add_weyl_point(&mut self, k: Array1<f64>, chirality: i32) {
        self.weyl_points.push(k);
        self.chirality.push(chirality);
    }

    /// Weyl Hamiltonian: H = v·(σ·k)
    pub fn weyl_hamiltonian(&self, k: &Array1<f64>, idx: usize) -> Array2<Complex64> {
        if idx >= self.weyl_points.len() {
            return Array2::zeros((2, 2));
        }

        let k_rel = k - &self.weyl_points[idx];
        let kx = k_rel[0];
        let ky = k_rel[1];
        let kz = k_rel[2];

        let chi = self.chirality[idx] as f64;

        let h00 = Complex64::new(kz, 0.0);
        let h01 = Complex64::new(kx, -ky) * chi;
        let h10 = Complex64::new(kx, ky) * chi;
        let h11 = Complex64::new(-kz, 0.0);

        Array2::from_shape_vec((2, 2), vec![h00, h01, h10, h11]).unwrap()
    }

    /// Fermi arc on surface
    pub fn fermi_arc_length(&self) -> f64 {
        if self.weyl_points.len() < 2 {
            return 0.0;
        }

        let k1 = &self.weyl_points[0];
        let k2 = &self.weyl_points[1];
        let dk = k2 - k1;
        dk.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    /// Anomalous Hall conductivity
    pub fn anomalous_hall(&self) -> f64 {
        let e2_h = 3.874e-5; // S
        let total_chirality: i32 = self.chirality.iter().sum();
        total_chirality as f64 * e2_h
    }
}

impl Default for WeylSemimetal {
    fn default() -> Self {
        Self::new()
    }
}

/// Majorana fermion system
pub struct MajoranaSystem {
    pub superconducting_gap: f64,
    pub chemical_potential: f64,
    pub zeeman_field: f64,
}

impl MajoranaSystem {
    pub fn new(gap: f64, mu: f64, zeeman: f64) -> Self {
        Self {
            superconducting_gap: gap,
            chemical_potential: mu,
            zeeman_field: zeeman,
        }
    }

    /// Topological phase condition
    pub fn is_topological_phase(&self) -> bool {
        self.zeeman_field > (self.superconducting_gap.powi(2) + self.chemical_potential.powi(2)).sqrt()
    }

    /// Majorana zero mode energy
    pub fn majorana_energy(&self) -> f64 {
        if self.is_topological_phase() {
            0.0 // Zero energy mode
        } else {
            self.superconducting_gap
        }
    }

    /// Topological gap
    pub fn topological_gap(&self) -> f64 {
        let delta = self.superconducting_gap;
        let mu = self.chemical_potential;
        let vz = self.zeeman_field;

        (vz.powi(2) - delta.powi(2) - mu.powi(2)).abs().sqrt()
    }
}

/// Topological simulator
pub struct TopologySimulator {
    pub quantum_hall: Option<QuantumHall>,
    pub topological_insulator: Option<TopologicalInsulator>,
    pub weyl_semimetal: Option<WeylSemimetal>,
    pub majorana: Option<MajoranaSystem>,
}

impl TopologySimulator {
    pub fn new() -> Self {
        Self {
            quantum_hall: None,
            topological_insulator: None,
            weyl_semimetal: None,
            majorana: None,
        }
    }

    pub fn create_quantum_hall(&mut self, b_field: f64, filling: f64) {
        self.quantum_hall = Some(QuantumHall::new(b_field, filling));
    }

    pub fn create_topological_insulator(&mut self, dim: usize, gap: f64, soc: f64) {
        self.topological_insulator = Some(if dim == 2 {
            TopologicalInsulator::new_2d(gap, soc)
        } else {
            TopologicalInsulator::new_3d(gap, soc)
        });
    }

    pub fn create_weyl_semimetal(&mut self) {
        self.weyl_semimetal = Some(WeylSemimetal::new());
    }

    pub fn create_majorana(&mut self, gap: f64, mu: f64, zeeman: f64) {
        self.majorana = Some(MajoranaSystem::new(gap, mu, zeeman));
    }

    pub fn hall_conductance(&self) -> Result<f64> {
        self.quantum_hall
            .as_ref()
            .map(|qh| qh.hall_conductance())
            .ok_or_else(|| TopologyError::InvalidTopology("No quantum Hall system".to_string()))
    }

    pub fn chern_number(&self) -> Result<i32> {
        self.quantum_hall
            .as_ref()
            .map(|qh| qh.chern_number())
            .ok_or_else(|| TopologyError::InvalidTopology("No quantum Hall system".to_string()))
    }

    pub fn is_topological(&self) -> Result<bool> {
        self.topological_insulator
            .as_ref()
            .map(|ti| ti.is_topological())
            .ok_or_else(|| TopologyError::InvalidTopology("No TI".to_string()))
    }

    pub fn majorana_is_topological(&self) -> Result<bool> {
        self.majorana
            .as_ref()
            .map(|m| m.is_topological_phase())
            .ok_or_else(|| TopologyError::InvalidTopology("No Majorana system".to_string()))
    }
}

impl Default for TopologySimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_quantum_hall() {
        let qh = QuantumHall::new(10.0, 2.0);
        let sigma = qh.hall_conductance();
        assert_relative_eq!(sigma, 2.0 * 3.874e-5, epsilon = 1e-6);
        assert_eq!(qh.chern_number(), 2);
    }

    #[test]
    fn test_topological_insulator() {
        let ti = TopologicalInsulator::new_2d(0.3, 0.5);
        assert!(ti.is_topological());
        assert_eq!(ti.dimension, 2);
    }

    #[test]
    fn test_weyl_semimetal() {
        let mut weyl = WeylSemimetal::new();
        weyl.add_weyl_point(Array1::from_vec(vec![0.1, 0.0, 0.0]), 1);
        weyl.add_weyl_point(Array1::from_vec(vec![-0.1, 0.0, 0.0]), -1);

        let arc_length = weyl.fermi_arc_length();
        assert_relative_eq!(arc_length, 0.2, epsilon = 0.01);
    }

    #[test]
    fn test_majorana() {
        let m = MajoranaSystem::new(1.0, 0.5, 2.0);
        assert!(m.is_topological_phase());
        assert_relative_eq!(m.majorana_energy(), 0.0);
    }

    #[test]
    fn test_simulator() {
        let mut sim = TopologySimulator::new();

        sim.create_quantum_hall(10.0, 1.0);
        assert!(sim.hall_conductance().is_ok());
        assert_eq!(sim.chern_number().unwrap(), 1);

        sim.create_topological_insulator(2, 0.3, 0.5);
        assert!(sim.is_topological().unwrap());

        sim.create_majorana(1.0, 0.5, 2.0);
        assert!(sim.majorana_is_topological().unwrap());
    }
}
