//! Astrophysics: stars, supernovae, black holes, stellar evolution

use std::f64::consts::PI;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum AstroError {
    #[error("Invalid object: {0}")]
    InvalidObject(String),
    #[error("Calculation failed: {0}")]
    CalculationFailed(String),
}

pub type Result<T> = std::result::Result<T, AstroError>;

/// Stellar classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectralType {
    O, // Blue, hottest
    B,
    A,
    F,
    G, // Sun-like
    K,
    M, // Red, coolest
}

/// Star
pub struct Star {
    pub mass: f64,
    pub radius: f64,
    pub temperature: f64,
    pub age: f64,
    pub spectral_type: SpectralType,
}

impl Star {
    pub fn new(mass: f64) -> Self {
        let (radius, temperature, spectral_type) = Self::main_sequence_properties(mass);

        Self {
            mass,
            radius,
            temperature,
            age: 0.0,
            spectral_type,
        }
    }

    /// Main sequence properties from mass
    fn main_sequence_properties(mass: f64) -> (f64, f64, SpectralType) {
        let m_sun = 1.989e30; // kg
        let r_sun = 6.96e8; // m
        let m = mass / m_sun;

        let radius = if m < 1.0 {
            r_sun * m.powf(0.8)
        } else {
            r_sun * m.powf(0.57)
        };

        let temperature = 5778.0 * m.powf(0.5);

        let spectral_type = if temperature > 30000.0 {
            SpectralType::O
        } else if temperature > 10000.0 {
            SpectralType::B
        } else if temperature > 7500.0 {
            SpectralType::A
        } else if temperature > 6000.0 {
            SpectralType::F
        } else if temperature > 5200.0 {
            SpectralType::G
        } else if temperature > 3700.0 {
            SpectralType::K
        } else {
            SpectralType::M
        };

        (radius, temperature, spectral_type)
    }

    /// Luminosity: L = 4πR²σT⁴
    pub fn luminosity(&self) -> f64 {
        let sigma = 5.67e-8; // Stefan-Boltzmann constant
        4.0 * PI * self.radius * self.radius * sigma * self.temperature.powi(4)
    }

    /// Main sequence lifetime: τ ∝ M/L ∝ M⁻²·⁵
    pub fn main_sequence_lifetime(&self) -> f64 {
        let m_sun = 1.989e30;
        let tau_sun = 10e9; // years
        let m = self.mass / m_sun;
        tau_sun * m.powf(-2.5)
    }

    /// Schwarzschild radius
    pub fn schwarzschild_radius(&self) -> f64 {
        let g = 6.674e-11;
        let c = 3e8;
        2.0 * g * self.mass / (c * c)
    }

    /// Escape velocity
    pub fn escape_velocity(&self) -> f64 {
        let g = 6.674e-11;
        (2.0 * g * self.mass / self.radius).sqrt()
    }

    /// Surface gravity
    pub fn surface_gravity(&self) -> f64 {
        let g = 6.674e-11;
        g * self.mass / (self.radius * self.radius)
    }

    /// Evolve star
    pub fn evolve(&mut self, dt: f64) {
        self.age += dt;

        // Simplified evolution
        if self.age > self.main_sequence_lifetime() {
            // Red giant phase
            self.radius *= 1.01;
            self.temperature *= 0.99;
        }
    }
}

/// Supernova
pub struct Supernova {
    pub progenitor_mass: f64,
    pub explosion_energy: f64,
    pub ejecta_mass: f64,
    pub remnant_mass: f64,
    pub supernova_type: SupernovaType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupernovaType {
    TypeIa,
    TypeII,
    TypeIb,
    TypeIc,
}

impl Supernova {
    pub fn new(mass: f64) -> Self {
        let m_sun = 1.989e30;
        let m = mass / m_sun;

        let (sn_type, remnant_mass) = if m > 25.0 {
            (SupernovaType::TypeII, 3.0 * m_sun) // Black hole
        } else if m > 8.0 {
            (SupernovaType::TypeII, 1.4 * m_sun) // Neutron star
        } else {
            (SupernovaType::TypeIa, 0.0) // White dwarf
        };

        Self {
            progenitor_mass: mass,
            explosion_energy: 1e44, // J (1 foe)
            ejecta_mass: mass - remnant_mass,
            remnant_mass,
            supernova_type: sn_type,
        }
    }

    /// Peak luminosity
    pub fn peak_luminosity(&self) -> f64 {
        let l_sun = 3.828e26; // W
        match self.supernova_type {
            SupernovaType::TypeIa => 5e9 * l_sun,
            SupernovaType::TypeII => 1e9 * l_sun,
            _ => 5e8 * l_sun,
        }
    }

    /// Ejecta velocity
    pub fn ejecta_velocity(&self) -> f64 {
        (2.0 * self.explosion_energy / self.ejecta_mass).sqrt()
    }

    /// Forms black hole?
    pub fn forms_black_hole(&self) -> bool {
        let m_sun = 1.989e30;
        self.remnant_mass > 3.0 * m_sun
    }

    /// Forms neutron star?
    pub fn forms_neutron_star(&self) -> bool {
        let m_sun = 1.989e30;
        self.remnant_mass > 1.2 * m_sun && self.remnant_mass < 3.0 * m_sun
    }
}

/// Black hole
pub struct BlackHole {
    pub mass: f64,
    pub spin: f64,
}

impl BlackHole {
    pub fn new(mass: f64, spin: f64) -> Self {
        Self { mass, spin }
    }

    /// Schwarzschild radius: rs = 2GM/c²
    pub fn schwarzschild_radius(&self) -> f64 {
        let g = 6.674e-11;
        let c = 3e8;
        2.0 * g * self.mass / (c * c)
    }

    /// Event horizon radius (Kerr)
    pub fn event_horizon(&self) -> f64 {
        let rs = self.schwarzschild_radius();
        let a = self.spin;
        rs * (1.0 + (1.0 - a * a).sqrt()) / 2.0
    }

    /// Innermost stable circular orbit (ISCO)
    pub fn isco_radius(&self) -> f64 {
        let rs = self.schwarzschild_radius();
        let a = self.spin;

        let z1 = 1.0 + (1.0 - a * a).powf(1.0 / 3.0) * ((1.0 + a).powf(1.0 / 3.0) + (1.0 - a).powf(1.0 / 3.0));
        let z2 = (3.0 * a * a + z1 * z1).sqrt();

        rs * (3.0 + z2 - ((3.0 - z1) * (3.0 + z1 + 2.0 * z2)).sqrt())
    }

    /// Hawking temperature: T = ℏc³/8πGMk
    pub fn hawking_temperature(&self) -> f64 {
        let hbar = 1.055e-34;
        let c = 3e8;
        let g = 6.674e-11;
        let k = 1.381e-23;

        hbar * c * c * c / (8.0 * PI * g * self.mass * k)
    }

    /// Hawking radiation power
    pub fn hawking_power(&self) -> f64 {
        let hbar = 1.055e-34;
        let c = 3e8;
        let g = 6.674e-11;

        hbar * c * c * c * c * c * c / (15360.0 * PI * g * g * self.mass * self.mass)
    }

    /// Evaporation time
    pub fn evaporation_time(&self) -> f64 {
        let g = 6.674e-11;
        let c = 3e8_f64;
        let hbar = 1.055e-34;

        5120.0 * PI * g * g * self.mass.powi(3) / (hbar * c.powi(4))
    }
}

/// Neutron star
pub struct NeutronStar {
    pub mass: f64,
    pub radius: f64,
    pub rotation_period: f64,
}

impl NeutronStar {
    pub fn new(mass: f64) -> Self {
        Self {
            mass,
            radius: 12e3, // ~12 km
            rotation_period: 0.001, // 1 ms (pulsar)
        }
    }

    /// Surface gravity
    pub fn surface_gravity(&self) -> f64 {
        let g = 6.674e-11;
        g * self.mass / (self.radius * self.radius)
    }

    /// Escape velocity
    pub fn escape_velocity(&self) -> f64 {
        let g = 6.674e-11;
        (2.0 * g * self.mass / self.radius).sqrt()
    }

    /// Compactness: M/R
    pub fn compactness(&self) -> f64 {
        let g = 6.674e-11;
        let c = 3e8;
        g * self.mass / (c * c * self.radius)
    }

    /// Magnetic field (typical pulsar)
    pub fn magnetic_field(&self) -> f64 {
        1e8 // Tesla
    }
}

/// Astrophysics simulator
pub struct AstroSimulator {
    pub star: Option<Star>,
    pub supernova: Option<Supernova>,
    pub black_hole: Option<BlackHole>,
    pub neutron_star: Option<NeutronStar>,
}

impl AstroSimulator {
    pub fn new() -> Self {
        Self {
            star: None,
            supernova: None,
            black_hole: None,
            neutron_star: None,
        }
    }

    pub fn create_star(&mut self, mass: f64) {
        self.star = Some(Star::new(mass));
    }

    pub fn create_supernova(&mut self, mass: f64) {
        self.supernova = Some(Supernova::new(mass));
    }

    pub fn create_black_hole(&mut self, mass: f64, spin: f64) {
        self.black_hole = Some(BlackHole::new(mass, spin));
    }

    pub fn create_neutron_star(&mut self, mass: f64) {
        self.neutron_star = Some(NeutronStar::new(mass));
    }

    pub fn star_luminosity(&self) -> Result<f64> {
        self.star
            .as_ref()
            .map(|s| s.luminosity())
            .ok_or_else(|| AstroError::InvalidObject("No star".to_string()))
    }

    pub fn star_lifetime(&self) -> Result<f64> {
        self.star
            .as_ref()
            .map(|s| s.main_sequence_lifetime())
            .ok_or_else(|| AstroError::InvalidObject("No star".to_string()))
    }

    pub fn supernova_forms_black_hole(&self) -> Result<bool> {
        self.supernova
            .as_ref()
            .map(|sn| sn.forms_black_hole())
            .ok_or_else(|| AstroError::InvalidObject("No supernova".to_string()))
    }

    pub fn black_hole_hawking_temp(&self) -> Result<f64> {
        self.black_hole
            .as_ref()
            .map(|bh| bh.hawking_temperature())
            .ok_or_else(|| AstroError::InvalidObject("No black hole".to_string()))
    }

    pub fn neutron_star_gravity(&self) -> Result<f64> {
        self.neutron_star
            .as_ref()
            .map(|ns| ns.surface_gravity())
            .ok_or_else(|| AstroError::InvalidObject("No neutron star".to_string()))
    }
}

impl Default for AstroSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_star() {
        let m_sun = 1.989e30;
        let star = Star::new(m_sun);
        assert_eq!(star.spectral_type, SpectralType::G);

        let lifetime = star.main_sequence_lifetime();
        assert!(lifetime > 9e9 && lifetime < 11e9); // ~10 Gyr
    }

    #[test]
    fn test_supernova() {
        let m_sun = 1.989e30;
        let sn = Supernova::new(20.0 * m_sun);
        assert_eq!(sn.supernova_type, SupernovaType::TypeII);
        assert!(sn.forms_neutron_star());
    }

    #[test]
    fn test_black_hole() {
        let m_sun = 1.989e30;
        let bh = BlackHole::new(10.0 * m_sun, 0.0);
        let rs = bh.schwarzschild_radius();
        assert!(rs > 0.0);

        let temp = bh.hawking_temperature();
        assert!(temp > 0.0);
    }

    #[test]
    fn test_neutron_star() {
        let m_sun = 1.989e30;
        let ns = NeutronStar::new(1.4 * m_sun);
        let g = ns.surface_gravity();
        assert!(g > 1e12); // Extreme gravity
    }

    #[test]
    fn test_simulator() {
        let mut sim = AstroSimulator::new();
        let m_sun = 1.989e30;

        sim.create_star(m_sun);
        assert!(sim.star_luminosity().is_ok());
        assert!(sim.star_lifetime().is_ok());

        sim.create_supernova(20.0 * m_sun);
        assert!(sim.supernova_forms_black_hole().is_ok());

        sim.create_black_hole(10.0 * m_sun, 0.5);
        assert!(sim.black_hole_hawking_temp().is_ok());

        sim.create_neutron_star(1.4 * m_sun);
        assert!(sim.neutron_star_gravity().is_ok());
    }
}
