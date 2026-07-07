//! Condensed Matter Physics: crystals, bands, phonons, solid properties

use std::f64::consts::PI;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum CondensedError {
    #[error("Invalid structure: {0}")]
    InvalidStructure(String),
    #[error("Calculation failed: {0}")]
    CalculationFailed(String),
}

pub type Result<T> = std::result::Result<T, CondensedError>;

/// Crystal lattice
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatticeType {
    SimpleCubic,
    BCC,
    FCC,
    Diamond,
    Hexagonal,
}

#[derive(Clone)]
pub struct Crystal {
    pub lattice_type: LatticeType,
    pub lattice_constant: f64,
    pub atoms_per_cell: usize,
}

impl Crystal {
    pub fn new(lattice_type: LatticeType, a: f64) -> Self {
        let atoms = match lattice_type {
            LatticeType::SimpleCubic => 1,
            LatticeType::BCC => 2,
            LatticeType::FCC => 4,
            LatticeType::Diamond => 8,
            LatticeType::Hexagonal => 2,
        };

        Self {
            lattice_type,
            lattice_constant: a,
            atoms_per_cell: atoms,
        }
    }

    /// Unit cell volume
    pub fn cell_volume(&self) -> f64 {
        let a = self.lattice_constant;
        match self.lattice_type {
            LatticeType::SimpleCubic | LatticeType::BCC | LatticeType::FCC | LatticeType::Diamond => {
                a * a * a
            }
            LatticeType::Hexagonal => {
                3.0_f64.sqrt() / 2.0 * a * a * (1.633 * a) // c/a ≈ 1.633
            }
        }
    }

    /// Atomic density (atoms/m³)
    pub fn atomic_density(&self) -> f64 {
        self.atoms_per_cell as f64 / self.cell_volume()
    }

    /// Nearest neighbor distance
    pub fn nearest_neighbor(&self) -> f64 {
        let a = self.lattice_constant;
        match self.lattice_type {
            LatticeType::SimpleCubic => a,
            LatticeType::BCC => a * 3.0_f64.sqrt() / 2.0,
            LatticeType::FCC => a / 2.0_f64.sqrt(),
            LatticeType::Diamond => a * 3.0_f64.sqrt() / 4.0,
            LatticeType::Hexagonal => a,
        }
    }

    /// Coordination number
    pub fn coordination_number(&self) -> usize {
        match self.lattice_type {
            LatticeType::SimpleCubic => 6,
            LatticeType::BCC => 8,
            LatticeType::FCC => 12,
            LatticeType::Diamond => 4,
            LatticeType::Hexagonal => 12,
        }
    }
}

/// Electronic band structure
pub struct BandStructure {
    pub crystal: Crystal,
    pub fermi_energy: f64,
    pub band_gap: f64,
}

impl BandStructure {
    pub fn new(crystal: Crystal, e_f: f64, e_g: f64) -> Self {
        Self {
            crystal,
            fermi_energy: e_f,
            band_gap: e_g,
        }
    }

    /// Energy dispersion: E(k) = E₀ + ℏ²k²/2m*
    pub fn dispersion(&self, k: f64, effective_mass: f64) -> f64 {
        let hbar = 1.055e-34;
        let e0 = self.fermi_energy;
        e0 + (hbar * hbar * k * k) / (2.0 * effective_mass * 1.602e-19) // eV
    }

    /// Density of states (3D): g(E) ∝ √E
    pub fn dos(&self, energy: f64) -> f64 {
        if energy < 0.0 {
            return 0.0;
        }
        let m_e = 9.109e-31_f64;
        let hbar = 1.055e-34_f64;
        let v = self.crystal.cell_volume();

        (1.0 / (2.0 * PI * PI)) * ((2.0 * m_e) / (hbar * hbar)).powf(1.5) * energy.sqrt() / v
    }

    /// Is metal, semiconductor, or insulator?
    pub fn material_type(&self) -> &str {
        if self.band_gap < 0.01 {
            "Metal"
        } else if self.band_gap < 3.0 {
            "Semiconductor"
        } else {
            "Insulator"
        }
    }

    /// Carrier concentration (intrinsic)
    pub fn intrinsic_carrier_density(&self, temperature: f64) -> f64 {
        let k_b = 8.617e-5; // eV/K
        let m_e = 9.109e-31;
        let h = 6.626e-34;

        let nc = 2.0 * ((2.0 * PI * m_e * k_b * temperature) / (h * h)).powf(1.5);
        nc * (-self.band_gap / (2.0 * k_b * temperature)).exp()
    }
}

/// Phonons (lattice vibrations)
pub struct Phonon {
    pub crystal: Crystal,
    pub mode: PhononMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhononMode {
    Acoustic,
    Optical,
}

impl Phonon {
    pub fn new(crystal: Crystal, mode: PhononMode) -> Self {
        Self { crystal, mode }
    }

    /// Dispersion relation: ω(k) for acoustic phonons
    pub fn dispersion(&self, k: f64) -> f64 {
        let a = self.crystal.lattice_constant;
        let v_s = 5000.0; // Sound velocity (m/s)

        match self.mode {
            PhononMode::Acoustic => {
                if k * a < 0.1 {
                    v_s * k // Linear at small k
                } else {
                    2.0 * v_s / a * (k * a / 2.0).sin().abs() // Full dispersion
                }
            }
            PhononMode::Optical => {
                let omega_0 = 1e13; // rad/s
                omega_0 * (1.0 - 0.1 * (k * a).cos())
            }
        }
    }

    /// Debye frequency
    pub fn debye_frequency(&self) -> f64 {
        let v_s = 5000.0;
        let n = self.crystal.atomic_density();
        v_s * (6.0 * PI * PI * n).powf(1.0 / 3.0)
    }

    /// Debye temperature
    pub fn debye_temperature(&self) -> f64 {
        let hbar = 1.055e-34;
        let k_b = 1.381e-23;
        let omega_d = self.debye_frequency();
        hbar * omega_d / k_b
    }

    /// Specific heat (Debye model)
    pub fn specific_heat(&self, temperature: f64) -> f64 {
        let t_d = self.debye_temperature();
        let k_b = 1.381e-23;

        if temperature < t_d / 10.0 {
            // Low T: C ∝ T³
            12.0 * PI.powi(4) / 5.0 * k_b * (temperature / t_d).powi(3)
        } else {
            // High T: C → 3k_B (Dulong-Petit)
            3.0 * k_b
        }
    }
}

/// Electrical conductivity
pub struct Conductor {
    pub carrier_density: f64,
    pub mobility: f64,
    pub charge: f64,
}

impl Conductor {
    pub fn new(n: f64, mu: f64) -> Self {
        Self {
            carrier_density: n,
            mobility: mu,
            charge: 1.602e-19,
        }
    }

    /// Conductivity: σ = nqμ
    pub fn conductivity(&self) -> f64 {
        self.carrier_density * self.charge * self.mobility
    }

    /// Resistivity: ρ = 1/σ
    pub fn resistivity(&self) -> f64 {
        1.0 / self.conductivity()
    }

    /// Drude relaxation time
    pub fn relaxation_time(&self) -> f64 {
        let m_e = 9.109e-31;
        self.mobility * m_e / self.charge
    }

    /// Mean free path
    pub fn mean_free_path(&self, fermi_velocity: f64) -> f64 {
        fermi_velocity * self.relaxation_time()
    }
}

/// Magnetic properties
pub struct MagneticMaterial {
    pub magnetization: f64,
    pub susceptibility: f64,
    pub curie_temperature: f64,
}

impl MagneticMaterial {
    pub fn ferromagnet(tc: f64) -> Self {
        Self {
            magnetization: 1e6, // A/m
            susceptibility: 1000.0,
            curie_temperature: tc,
        }
    }

    /// Is ferromagnetic at temperature T?
    pub fn is_ferromagnetic(&self, temperature: f64) -> bool {
        temperature < self.curie_temperature
    }

    /// Magnetization vs temperature (mean field)
    pub fn magnetization_at(&self, temperature: f64) -> f64 {
        if temperature >= self.curie_temperature {
            0.0
        } else {
            let t_ratio = temperature / self.curie_temperature;
            self.magnetization * (1.0 - t_ratio).powf(0.5)
        }
    }
}

/// Condensed matter simulator
pub struct CondensedSimulator {
    pub crystal: Option<Crystal>,
    pub bands: Option<BandStructure>,
    pub phonon: Option<Phonon>,
    pub conductor: Option<Conductor>,
    pub magnetic: Option<MagneticMaterial>,
}

impl CondensedSimulator {
    pub fn new() -> Self {
        Self {
            crystal: None,
            bands: None,
            phonon: None,
            conductor: None,
            magnetic: None,
        }
    }

    pub fn create_crystal(&mut self, lattice_type: LatticeType, a: f64) {
        self.crystal = Some(Crystal::new(lattice_type, a));
    }

    pub fn create_bands(&mut self, e_f: f64, e_g: f64) {
        if let Some(crystal) = self.crystal.clone() {
            self.bands = Some(BandStructure::new(crystal, e_f, e_g));
        }
    }

    pub fn create_conductor(&mut self, n: f64, mu: f64) {
        self.conductor = Some(Conductor::new(n, mu));
    }

    pub fn atomic_density(&self) -> Result<f64> {
        self.crystal
            .as_ref()
            .map(|c| c.atomic_density())
            .ok_or_else(|| CondensedError::InvalidStructure("No crystal".to_string()))
    }

    pub fn material_type(&self) -> Result<String> {
        self.bands
            .as_ref()
            .map(|b| b.material_type().to_string())
            .ok_or_else(|| CondensedError::InvalidStructure("No bands".to_string()))
    }

    pub fn conductivity(&self) -> Result<f64> {
        self.conductor
            .as_ref()
            .map(|c| c.conductivity())
            .ok_or_else(|| CondensedError::InvalidStructure("No conductor".to_string()))
    }
}

impl Default for CondensedSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crystal() {
        let fcc = Crystal::new(LatticeType::FCC, 3.61e-10);
        assert_eq!(fcc.atoms_per_cell, 4);
        assert_eq!(fcc.coordination_number(), 12);
    }

    #[test]
    fn test_band_structure() {
        let crystal = Crystal::new(LatticeType::FCC, 5e-10);
        let bands = BandStructure::new(crystal, 5.0, 1.1);
        assert_eq!(bands.material_type(), "Semiconductor");
    }

    #[test]
    fn test_phonon() {
        let crystal = Crystal::new(LatticeType::SimpleCubic, 5e-10);
        let phonon = Phonon::new(crystal, PhononMode::Acoustic);
        let t_d = phonon.debye_temperature();
        assert!(t_d > 0.0);
    }

    #[test]
    fn test_conductor() {
        let conductor = Conductor::new(1e28, 0.01);
        let sigma = conductor.conductivity();
        assert!(sigma > 0.0);
    }

    #[test]
    fn test_simulator() {
        let mut sim = CondensedSimulator::new();

        sim.create_crystal(LatticeType::FCC, 4e-10);
        assert!(sim.atomic_density().is_ok());

        sim.create_bands(5.0, 1.1);
        assert!(sim.material_type().is_ok());

        sim.create_conductor(1e28, 0.01);
        assert!(sim.conductivity().is_ok());
    }
}
