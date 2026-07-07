//! Particle Accelerators: LHC, collisions, detectors, high energy physics

use std::f64::consts::PI;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum AcceleratorError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Calculation failed: {0}")]
    CalculationFailed(String),
}

pub type Result<T> = std::result::Result<T, AcceleratorError>;

/// Particle beam
#[derive(Clone)]
pub struct Beam {
    pub particle_mass: f64,
    pub particle_charge: f64,
    pub energy: f64,
    pub intensity: f64,
}

impl Beam {
    pub fn proton_beam(energy: f64) -> Self {
        Self {
            particle_mass: 1.673e-27, // kg
            particle_charge: 1.602e-19, // C
            energy,
            intensity: 1e14, // particles/s
        }
    }

    pub fn electron_beam(energy: f64) -> Self {
        Self {
            particle_mass: 9.109e-31,
            particle_charge: 1.602e-19,
            energy,
            intensity: 1e14,
        }
    }

    /// Lorentz factor: γ = E/mc²
    pub fn lorentz_factor(&self) -> f64 {
        let c = 3e8;
        let e_j = self.energy * 1.602e-19; // J
        let rest_energy = self.particle_mass * c * c;
        e_j / rest_energy
    }

    /// Velocity: v = c√(1 - 1/γ²)
    pub fn velocity(&self) -> f64 {
        let c = 3e8;
        let gamma = self.lorentz_factor();
        c * (1.0 - 1.0 / (gamma * gamma)).sqrt()
    }

    /// Momentum: p = γmv
    pub fn momentum(&self) -> f64 {
        let gamma = self.lorentz_factor();
        gamma * self.particle_mass * self.velocity()
    }

    /// Magnetic rigidity: Bρ = p/q
    pub fn magnetic_rigidity(&self) -> f64 {
        self.momentum() / self.particle_charge
    }
}

/// Synchrotron accelerator
pub struct Synchrotron {
    pub radius: f64,
    pub magnetic_field: f64,
    pub rf_frequency: f64,
    pub beam: Beam,
}

impl Synchrotron {
    pub fn new(radius: f64, b_field: f64, beam: Beam) -> Self {
        let v = beam.velocity();
        let rf_freq = v / (2.0 * PI * radius);

        Self {
            radius,
            magnetic_field: b_field,
            rf_frequency: rf_freq,
            beam,
        }
    }

    /// Circumference
    pub fn circumference(&self) -> f64 {
        2.0 * PI * self.radius
    }

    /// Revolution frequency
    pub fn revolution_frequency(&self) -> f64 {
        self.beam.velocity() / self.circumference()
    }

    /// Synchrotron radiation power: P = (q²c/6πε₀)(γ⁴/ρ²)
    pub fn synchrotron_power(&self) -> f64 {
        let c = 3e8;
        let epsilon_0 = 8.854e-12;
        let q = self.beam.particle_charge;
        let gamma = self.beam.lorentz_factor();
        let rho = self.radius;

        (q * q * c / (6.0 * PI * epsilon_0)) * (gamma.powi(4) / (rho * rho))
    }

    /// Energy loss per turn
    pub fn energy_loss_per_turn(&self) -> f64 {
        let p = self.synchrotron_power();
        let t = self.circumference() / self.beam.velocity();
        p * t / 1.602e-19 // eV
    }
}

/// Collider
pub struct Collider {
    pub beam1: Beam,
    pub beam2: Beam,
    pub luminosity: f64,
}

impl Collider {
    pub fn new(beam1: Beam, beam2: Beam) -> Self {
        Self {
            beam1,
            beam2,
            luminosity: 1e34, // cm⁻²s⁻¹ (LHC-like)
        }
    }

    /// Center-of-mass energy: √s = 2E (same particle, head-on)
    pub fn cms_energy(&self) -> f64 {
        2.0 * self.beam1.energy
    }

    /// Event rate: R = Lσ
    pub fn event_rate(&self, cross_section: f64) -> f64 {
        self.luminosity * cross_section
    }

    /// Higgs production rate (gg→H)
    pub fn higgs_rate(&self) -> f64 {
        let sigma_higgs = 50e-12; // 50 pb at 13 TeV
        self.event_rate(sigma_higgs)
    }

    /// Top quark pair production rate
    pub fn top_pair_rate(&self) -> f64 {
        let sigma_ttbar = 800e-12; // 800 pb at 13 TeV
        self.event_rate(sigma_ttbar)
    }
}

/// Detector
pub struct Detector {
    pub name: String,
    pub acceptance: f64,
    pub energy_resolution: f64,
    pub time_resolution: f64,
}

impl Detector {
    pub fn atlas() -> Self {
        Self {
            name: "ATLAS".to_string(),
            acceptance: 0.95,
            energy_resolution: 0.1,
            time_resolution: 1e-9, // 1 ns
        }
    }

    pub fn cms() -> Self {
        Self {
            name: "CMS".to_string(),
            acceptance: 0.95,
            energy_resolution: 0.1,
            time_resolution: 1e-9,
        }
    }

    /// Detected events
    pub fn detected_events(&self, total_events: f64) -> f64 {
        total_events * self.acceptance
    }

    /// Energy measurement
    pub fn measure_energy(&self, true_energy: f64) -> f64 {
        true_energy * (1.0 + self.energy_resolution * 0.1)
    }
}

/// LHC (Large Hadron Collider)
pub struct LHC {
    pub synchrotron: Synchrotron,
    pub collider: Collider,
    pub detector: Detector,
}

impl LHC {
    pub fn new() -> Self {
        let beam1 = Beam::proton_beam(6.5e12); // 6.5 TeV
        let beam2 = Beam::proton_beam(6.5e12);

        let synchrotron = Synchrotron::new(4243.0, 8.33, beam1.clone());
        let collider = Collider::new(beam1, beam2);
        let detector = Detector::atlas();

        Self {
            synchrotron,
            collider,
            detector,
        }
    }

    /// Total collision energy
    pub fn collision_energy(&self) -> f64 {
        self.collider.cms_energy()
    }

    /// Higgs bosons per year
    pub fn higgs_per_year(&self) -> f64 {
        let rate = self.collider.higgs_rate();
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let efficiency = 0.3; // 30% uptime
        rate * seconds_per_year * efficiency
    }

    /// Discovery significance (5σ)
    pub fn discovery_significance(&self, signal: f64, background: f64) -> f64 {
        signal / background.sqrt()
    }
}

impl Default for LHC {
    fn default() -> Self {
        Self::new()
    }
}

/// Accelerator simulator
pub struct AcceleratorSimulator {
    pub beam: Option<Beam>,
    pub synchrotron: Option<Synchrotron>,
    pub collider: Option<Collider>,
    pub lhc: Option<LHC>,
}

impl AcceleratorSimulator {
    pub fn new() -> Self {
        Self {
            beam: None,
            synchrotron: None,
            collider: None,
            lhc: None,
        }
    }

    pub fn create_proton_beam(&mut self, energy: f64) {
        self.beam = Some(Beam::proton_beam(energy));
    }

    pub fn create_electron_beam(&mut self, energy: f64) {
        self.beam = Some(Beam::electron_beam(energy));
    }

    pub fn create_lhc(&mut self) {
        self.lhc = Some(LHC::new());
    }

    pub fn beam_velocity(&self) -> Result<f64> {
        self.beam
            .as_ref()
            .map(|b| b.velocity())
            .ok_or_else(|| AcceleratorError::InvalidConfig("No beam".to_string()))
    }

    pub fn beam_lorentz_factor(&self) -> Result<f64> {
        self.beam
            .as_ref()
            .map(|b| b.lorentz_factor())
            .ok_or_else(|| AcceleratorError::InvalidConfig("No beam".to_string()))
    }

    pub fn lhc_collision_energy(&self) -> Result<f64> {
        self.lhc
            .as_ref()
            .map(|lhc| lhc.collision_energy())
            .ok_or_else(|| AcceleratorError::InvalidConfig("No LHC".to_string()))
    }

    pub fn lhc_higgs_per_year(&self) -> Result<f64> {
        self.lhc
            .as_ref()
            .map(|lhc| lhc.higgs_per_year())
            .ok_or_else(|| AcceleratorError::InvalidConfig("No LHC".to_string()))
    }
}

impl Default for AcceleratorSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beam() {
        let beam = Beam::proton_beam(7e12); // 7 TeV
        let gamma = beam.lorentz_factor();
        assert!(gamma > 7000.0);

        let v = beam.velocity();
        let c = 3e8;
        assert!(v > 0.999 * c);
    }

    #[test]
    fn test_synchrotron() {
        let beam = Beam::proton_beam(7e12);
        let sync = Synchrotron::new(4243.0, 8.33, beam);

        let circ = sync.circumference();
        assert!(circ > 26000.0 && circ < 27000.0); // ~27 km
    }

    #[test]
    fn test_collider() {
        let beam1 = Beam::proton_beam(6.5e12);
        let beam2 = Beam::proton_beam(6.5e12);
        let collider = Collider::new(beam1, beam2);

        let cms = collider.cms_energy();
        assert!(cms > 12e12 && cms < 14e12); // ~13 TeV
    }

    #[test]
    fn test_lhc() {
        let lhc = LHC::new();
        let e_cms = lhc.collision_energy();
        assert!(e_cms > 12e12);

        let higgs_rate = lhc.higgs_per_year();
        assert!(higgs_rate > 0.0);
    }

    #[test]
    fn test_simulator() {
        let mut sim = AcceleratorSimulator::new();

        sim.create_proton_beam(7e12);
        assert!(sim.beam_velocity().is_ok());
        assert!(sim.beam_lorentz_factor().is_ok());

        sim.create_lhc();
        assert!(sim.lhc_collision_energy().is_ok());
        assert!(sim.lhc_higgs_per_year().is_ok());
    }
}
