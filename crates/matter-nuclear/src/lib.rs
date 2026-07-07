//! Nuclear Physics: fission, fusion, decay, reactions

use std::f64::consts::E;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum NuclearError {
    #[error("Invalid nucleus: {0}")]
    InvalidNucleus(String),
    #[error("Calculation failed: {0}")]
    CalculationFailed(String),
}

pub type Result<T> = std::result::Result<T, NuclearError>;

/// Nucleus
#[derive(Debug, Clone)]
pub struct Nucleus {
    pub protons: u32,
    pub neutrons: u32,
    pub mass_number: u32,
}

impl Nucleus {
    pub fn new(z: u32, n: u32) -> Self {
        Self {
            protons: z,
            neutrons: n,
            mass_number: z + n,
        }
    }

    /// Atomic mass (u)
    pub fn atomic_mass(&self) -> f64 {
        let m_p = 1.007276; // u
        let m_n = 1.008665; // u
        let z = self.protons as f64;
        let n = self.neutrons as f64;

        z * m_p + n * m_n - self.binding_energy() / 931.5
    }

    /// Binding energy (MeV) - semi-empirical mass formula (SEMF)
    pub fn binding_energy(&self) -> f64 {
        let a = self.mass_number as f64;
        let z = self.protons as f64;
        let n = self.neutrons as f64;

        // SEMF coefficients
        let a_v = 15.75; // Volume
        let a_s = 17.8;  // Surface
        let a_c = 0.711; // Coulomb
        let a_a = 23.7;  // Asymmetry
        let a_p = 11.18; // Pairing

        let volume = a_v * a;
        let surface = -a_s * a.powf(2.0 / 3.0);
        let coulomb = -a_c * z * (z - 1.0) / a.powf(1.0 / 3.0);
        let asymmetry = -a_a * (n - z).powi(2) / a;

        let pairing = if self.protons % 2 == 0 && self.neutrons % 2 == 0 {
            a_p / a.sqrt()
        } else if self.protons % 2 == 1 && self.neutrons % 2 == 1 {
            -a_p / a.sqrt()
        } else {
            0.0
        };

        volume + surface + coulomb + asymmetry + pairing
    }

    /// Binding energy per nucleon
    pub fn binding_energy_per_nucleon(&self) -> f64 {
        self.binding_energy() / self.mass_number as f64
    }

    /// Is stable?
    pub fn is_stable(&self) -> bool {
        let z = self.protons;
        let n = self.neutrons;

        // Simplified stability check
        if z > 83 {
            return false; // All elements > Bi are unstable
        }

        let ratio = n as f64 / z as f64;
        ratio > 0.8 && ratio < 1.6
    }
}

/// Radioactive decay
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecayMode {
    Alpha,
    BetaMinus,
    BetaPlus,
    GammaRay,
    Fission,
}

pub struct RadioactiveNucleus {
    pub nucleus: Nucleus,
    pub half_life: f64,
    pub decay_mode: DecayMode,
}

impl RadioactiveNucleus {
    pub fn new(nucleus: Nucleus, half_life: f64, mode: DecayMode) -> Self {
        Self {
            nucleus,
            half_life,
            decay_mode: mode,
        }
    }

    /// Decay constant: λ = ln(2)/t₁/₂
    pub fn decay_constant(&self) -> f64 {
        2.0_f64.ln() / self.half_life
    }

    /// Activity: A = λN
    pub fn activity(&self, n_atoms: f64) -> f64 {
        self.decay_constant() * n_atoms
    }

    /// Remaining atoms: N(t) = N₀ e^(-λt)
    pub fn remaining_atoms(&self, n0: f64, time: f64) -> f64 {
        n0 * E.powf(-self.decay_constant() * time)
    }

    /// Q-value (energy released)
    pub fn q_value(&self) -> f64 {
        match self.decay_mode {
            DecayMode::Alpha => {
                let m_alpha = 4.002603; // u
                let m_parent = self.nucleus.atomic_mass();
                let m_daughter = Nucleus::new(
                    self.nucleus.protons - 2,
                    self.nucleus.neutrons - 2,
                )
                .atomic_mass();
                (m_parent - m_daughter - m_alpha) * 931.5 // MeV
            }
            DecayMode::BetaMinus => {
                let m_parent = self.nucleus.atomic_mass();
                let m_daughter = Nucleus::new(
                    self.nucleus.protons + 1,
                    self.nucleus.neutrons - 1,
                )
                .atomic_mass();
                (m_parent - m_daughter) * 931.5
            }
            _ => 0.0,
        }
    }
}

/// Nuclear fission
pub struct Fission {
    pub fuel: Nucleus,
    pub fragments: Vec<Nucleus>,
    pub neutrons_released: u32,
}

impl Fission {
    pub fn u235() -> Self {
        let fuel = Nucleus::new(92, 143); // U-235
        let frag1 = Nucleus::new(56, 87); // Ba-143
        let frag2 = Nucleus::new(36, 54); // Kr-90

        Self {
            fuel,
            fragments: vec![frag1, frag2],
            neutrons_released: 2,
        }
    }

    /// Energy released (MeV)
    pub fn energy_released(&self) -> f64 {
        let m_fuel = self.fuel.atomic_mass();
        let m_fragments: f64 = self.fragments.iter().map(|f| f.atomic_mass()).sum();
        let m_neutron = 1.008665;
        let m_neutrons = m_neutron * self.neutrons_released as f64;

        (m_fuel - m_fragments - m_neutrons) * 931.5
    }

    /// Critical mass (kg) - simplified
    pub fn critical_mass(&self) -> f64 {
        52.0 // kg for U-235 sphere
    }

    /// Multiplication factor
    pub fn k_factor(&self, mass: f64) -> f64 {
        if mass < self.critical_mass() {
            0.9 // Subcritical
        } else if mass < self.critical_mass() * 1.1 {
            1.0 // Critical
        } else {
            1.5 // Supercritical
        }
    }
}

/// Nuclear fusion
pub struct Fusion {
    pub reactants: Vec<Nucleus>,
    pub products: Vec<Nucleus>,
}

impl Fusion {
    /// D-T fusion: ²H + ³H → ⁴He + n
    pub fn deuterium_tritium() -> Self {
        let d = Nucleus::new(1, 1); // Deuterium
        let t = Nucleus::new(1, 2); // Tritium
        let he4 = Nucleus::new(2, 2); // Helium-4
        let n = Nucleus::new(0, 1); // Neutron

        Self {
            reactants: vec![d, t],
            products: vec![he4, n],
        }
    }

    /// D-D fusion: ²H + ²H → ³He + n
    pub fn deuterium_deuterium() -> Self {
        let d = Nucleus::new(1, 1);
        let he3 = Nucleus::new(2, 1);
        let n = Nucleus::new(0, 1);

        Self {
            reactants: vec![d.clone(), d],
            products: vec![he3, n],
        }
    }

    /// Proton-proton chain (Sun)
    pub fn proton_proton() -> Self {
        let p = Nucleus::new(1, 0);
        let he4 = Nucleus::new(2, 2);

        Self {
            reactants: vec![p.clone(), p.clone(), p.clone(), p],
            products: vec![he4],
        }
    }

    /// Q-value (energy released)
    pub fn q_value(&self) -> f64 {
        let m_reactants: f64 = self.reactants.iter().map(|r| r.atomic_mass()).sum();
        let m_products: f64 = self.products.iter().map(|p| p.atomic_mass()).sum();

        (m_reactants - m_products).abs() * 931.5 // MeV
    }

    /// Cross-section (barn) at temperature
    pub fn cross_section(&self, temperature: f64) -> f64 {
        // Simplified Gamow factor
        let t_kev = temperature / 1.16e7; // Convert K to keV
        if t_kev < 1.0 {
            1e-30 // Very small
        } else if t_kev < 100.0 {
            1e-28 * (t_kev / 10.0).powf(2.0)
        } else {
            1e-24 // Peak at ~70 keV
        }
    }

    /// Lawson criterion: nτ > 10²⁰ s/m³
    pub fn lawson_criterion(&self, density: f64, confinement_time: f64) -> bool {
        density * confinement_time > 1e20
    }
}

/// Nuclear reactor
pub struct Reactor {
    pub fuel_mass: f64,
    pub control_rod_position: f64,
    pub coolant_temperature: f64,
    pub power_output: f64,
}

impl Reactor {
    pub fn new(fuel_mass: f64) -> Self {
        Self {
            fuel_mass,
            control_rod_position: 0.5,
            coolant_temperature: 300.0,
            power_output: 0.0,
        }
    }

    /// Thermal power (MW)
    pub fn thermal_power(&self) -> f64 {
        let fission_rate = self.fuel_mass * 1e24; // atoms/s
        let energy_per_fission = 200.0 * 1.602e-13; // J
        fission_rate * energy_per_fission / 1e6 // MW
    }

    /// Electrical power (MW)
    pub fn electrical_power(&self) -> f64 {
        let efficiency = 0.33; // 33% thermal efficiency
        self.thermal_power() * efficiency
    }

    /// Adjust control rods
    pub fn set_control_rods(&mut self, position: f64) {
        self.control_rod_position = position.clamp(0.0, 1.0);
        self.power_output = self.thermal_power() * (1.0 - self.control_rod_position);
    }
}

/// Nuclear simulator
pub struct NuclearSimulator {
    pub nucleus: Option<Nucleus>,
    pub radioactive: Option<RadioactiveNucleus>,
    pub fission: Option<Fission>,
    pub fusion: Option<Fusion>,
    pub reactor: Option<Reactor>,
}

impl NuclearSimulator {
    pub fn new() -> Self {
        Self {
            nucleus: None,
            radioactive: None,
            fission: None,
            fusion: None,
            reactor: None,
        }
    }

    pub fn create_nucleus(&mut self, z: u32, n: u32) {
        self.nucleus = Some(Nucleus::new(z, n));
    }

    pub fn create_fission(&mut self) {
        self.fission = Some(Fission::u235());
    }

    pub fn create_fusion_dt(&mut self) {
        self.fusion = Some(Fusion::deuterium_tritium());
    }

    pub fn create_reactor(&mut self, fuel_mass: f64) {
        self.reactor = Some(Reactor::new(fuel_mass));
    }

    pub fn binding_energy(&self) -> Result<f64> {
        self.nucleus
            .as_ref()
            .map(|n| n.binding_energy())
            .ok_or_else(|| NuclearError::InvalidNucleus("No nucleus".to_string()))
    }

    pub fn fission_energy(&self) -> Result<f64> {
        self.fission
            .as_ref()
            .map(|f| f.energy_released())
            .ok_or_else(|| NuclearError::InvalidNucleus("No fission".to_string()))
    }

    pub fn fusion_energy(&self) -> Result<f64> {
        self.fusion
            .as_ref()
            .map(|f| f.q_value())
            .ok_or_else(|| NuclearError::InvalidNucleus("No fusion".to_string()))
    }

    pub fn reactor_power(&self) -> Result<f64> {
        self.reactor
            .as_ref()
            .map(|r| r.electrical_power())
            .ok_or_else(|| NuclearError::InvalidNucleus("No reactor".to_string()))
    }
}

impl Default for NuclearSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nucleus() {
        let fe56 = Nucleus::new(26, 30); // Iron-56
        let be = fe56.binding_energy_per_nucleon();
        assert!(be > 8.0 && be < 9.0); // ~8.8 MeV (most stable)
    }

    #[test]
    fn test_radioactive_decay() {
        let c14 = Nucleus::new(6, 8);
        let rad = RadioactiveNucleus::new(c14, 5730.0 * 365.25 * 24.0 * 3600.0, DecayMode::BetaMinus);

        let n0 = 1e12;
        let n_half = rad.remaining_atoms(n0, rad.half_life);
        assert!((n_half - n0 / 2.0).abs() < 1e10);
    }

    #[test]
    fn test_fission() {
        let fission = Fission::u235();
        let energy = fission.energy_released();
        assert!(energy > 0.0); // Positive energy release
    }

    #[test]
    fn test_fusion() {
        let fusion = Fusion::deuterium_tritium();
        let q = fusion.q_value();
        assert!(q > 0.0); // Positive Q-value
    }

    #[test]
    fn test_reactor() {
        let mut reactor = Reactor::new(100.0);
        reactor.set_control_rods(0.3);
        let power = reactor.electrical_power();
        assert!(power > 0.0);
    }

    #[test]
    fn test_simulator() {
        let mut sim = NuclearSimulator::new();

        sim.create_nucleus(26, 30);
        assert!(sim.binding_energy().is_ok());

        sim.create_fission();
        assert!(sim.fission_energy().is_ok());

        sim.create_fusion_dt();
        assert!(sim.fusion_energy().is_ok());

        sim.create_reactor(100.0);
        assert!(sim.reactor_power().is_ok());
    }
}
