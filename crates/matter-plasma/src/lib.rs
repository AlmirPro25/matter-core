//! Plasma Physics: fusion, tokamaks, MHD, confinement

use ndarray::Array1;
use std::f64::consts::PI;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum PlasmaError {
    #[error("Invalid plasma: {0}")]
    InvalidPlasma(String),
    #[error("Calculation failed: {0}")]
    CalculationFailed(String),
}

pub type Result<T> = std::result::Result<T, PlasmaError>;

/// Plasma state
pub struct Plasma {
    pub temperature: f64,
    pub density: f64,
    pub magnetic_field: f64,
    pub ion_mass: f64,
    pub ion_charge: f64,
}

impl Plasma {
    pub fn new(temp: f64, density: f64, b_field: f64) -> Self {
        Self {
            temperature: temp,
            density,
            magnetic_field: b_field,
            ion_mass: 1.673e-27,
            ion_charge: 1.602e-19,
        }
    }

    /// Debye length: λ_D = √(ε₀kT/ne²)
    pub fn debye_length(&self) -> f64 {
        let epsilon_0 = 8.854e-12;
        let k_b = 1.381e-23;
        let e = 1.602e-19;
        let t = self.temperature;
        let n = self.density;

        (epsilon_0 * k_b * t / (n * e * e)).sqrt()
    }

    /// Plasma frequency: ω_p = √(ne²/ε₀m_e)
    pub fn plasma_frequency(&self) -> f64 {
        let epsilon_0 = 8.854e-12;
        let e = 1.602e-19;
        let m_e = 9.109e-31;
        let n = self.density;

        (n * e * e / (epsilon_0 * m_e)).sqrt()
    }

    /// Cyclotron frequency: ω_c = eB/m
    pub fn cyclotron_frequency(&self) -> f64 {
        let e = self.ion_charge;
        let b = self.magnetic_field;
        let m = self.ion_mass;

        e * b / m
    }

    /// Larmor radius: r_L = v_th/ω_c
    pub fn larmor_radius(&self) -> f64 {
        let k_b = 1.381e-23;
        let v_th = (k_b * self.temperature / self.ion_mass).sqrt();
        v_th / self.cyclotron_frequency()
    }

    /// Beta parameter: β = p/(B²/2μ₀)
    pub fn beta(&self) -> f64 {
        let k_b = 1.381e-23;
        let mu_0 = 4.0 * PI * 1e-7;
        let p = self.density * k_b * self.temperature;
        let b = self.magnetic_field;

        p / (b * b / (2.0 * mu_0))
    }
}

/// Tokamak fusion reactor
pub struct Tokamak {
    pub major_radius: f64,
    pub minor_radius: f64,
    pub plasma: Plasma,
    pub toroidal_field: f64,
    pub plasma_current: f64,
}

impl Tokamak {
    pub fn new(r_major: f64, r_minor: f64, temp: f64, density: f64, b_field: f64) -> Self {
        Self {
            major_radius: r_major,
            minor_radius: r_minor,
            plasma: Plasma::new(temp, density, b_field),
            toroidal_field: b_field,
            plasma_current: 1e6,
        }
    }

    /// Aspect ratio: A = R/a
    pub fn aspect_ratio(&self) -> f64 {
        self.major_radius / self.minor_radius
    }

    /// Safety factor: q = (r/R)(B_t/B_p)
    pub fn safety_factor(&self, r: f64) -> f64 {
        let mu_0 = 4.0 * PI * 1e-7;
        let b_p = mu_0 * self.plasma_current / (2.0 * PI * r);
        (r / self.major_radius) * (self.toroidal_field / b_p)
    }

    /// Lawson criterion: nτE > 10²⁰ s/m³
    pub fn lawson_parameter(&self, confinement_time: f64) -> f64 {
        self.plasma.density * confinement_time
    }

    /// Triple product: nTτE
    pub fn triple_product(&self, confinement_time: f64) -> f64 {
        self.plasma.density * self.plasma.temperature * confinement_time
    }

    /// Fusion power (D-T reaction)
    pub fn fusion_power(&self, confinement_time: f64) -> f64 {
        let n = self.plasma.density;
        let t = self.plasma.temperature;
        let volume = 2.0 * PI * PI * self.major_radius * self.minor_radius * self.minor_radius;

        // Simplified fusion rate
        let sigma_v = if t > 1e7 {
            1e-22 // m³/s (peak at ~70 keV)
        } else {
            0.0
        };

        let q_fusion = 17.6 * 1.602e-19; // J per reaction
        0.25 * n * n * sigma_v * q_fusion * volume
    }

    /// Q factor (fusion gain)
    pub fn q_factor(&self, heating_power: f64, confinement_time: f64) -> f64 {
        let p_fusion = self.fusion_power(confinement_time);
        p_fusion / heating_power
    }

    /// Is ignition achieved? (Q > 1)
    pub fn is_ignition(&self, heating_power: f64, confinement_time: f64) -> bool {
        self.q_factor(heating_power, confinement_time) > 1.0
    }
}

/// Magnetohydrodynamics (MHD)
pub struct MHD {
    pub velocity: Array1<f64>,
    pub magnetic_field: Array1<f64>,
    pub density: f64,
    pub pressure: f64,
}

impl MHD {
    pub fn new(v: Array1<f64>, b: Array1<f64>, rho: f64, p: f64) -> Self {
        Self {
            velocity: v,
            magnetic_field: b,
            density: rho,
            pressure: p,
        }
    }

    /// Alfvén velocity: v_A = B/√(μ₀ρ)
    pub fn alfven_velocity(&self) -> f64 {
        let mu_0 = 4.0 * PI * 1e-7;
        let b = self.magnetic_field.iter().map(|x| x * x).sum::<f64>().sqrt();
        b / (mu_0 * self.density).sqrt()
    }

    /// Magnetic pressure: p_B = B²/2μ₀
    pub fn magnetic_pressure(&self) -> f64 {
        let mu_0 = 4.0 * PI * 1e-7;
        let b2 = self.magnetic_field.iter().map(|x| x * x).sum::<f64>();
        b2 / (2.0 * mu_0)
    }

    /// Plasma beta: β = p/(B²/2μ₀)
    pub fn beta(&self) -> f64 {
        self.pressure / self.magnetic_pressure()
    }

    /// Magnetic Reynolds number: Rm = vLμ₀σ
    pub fn magnetic_reynolds(&self, length_scale: f64, conductivity: f64) -> f64 {
        let mu_0 = 4.0 * PI * 1e-7;
        let v = self.velocity.iter().map(|x| x * x).sum::<f64>().sqrt();
        v * length_scale * mu_0 * conductivity
    }
}

/// Inertial confinement fusion (ICF)
pub struct ICF {
    pub target_radius: f64,
    pub laser_energy: f64,
    pub compression_ratio: f64,
    pub fuel_density: f64,
}

impl ICF {
    pub fn new(radius: f64, energy: f64) -> Self {
        Self {
            target_radius: radius,
            laser_energy: energy,
            compression_ratio: 1000.0,
            fuel_density: 1e31,
        }
    }

    /// Compressed density
    pub fn compressed_density(&self) -> f64 {
        self.fuel_density * self.compression_ratio
    }

    /// Lawson criterion for ICF: ρR > 1 g/cm²
    pub fn rho_r(&self) -> f64 {
        let rho = self.compressed_density() * 1.673e-27; // kg/m³
        let r = self.target_radius * 1e-6; // m
        rho * r * 1e-1 // g/cm²
    }

    /// Is ignition possible?
    pub fn can_ignite(&self) -> bool {
        self.rho_r() > 1.0
    }

    /// Fusion yield
    pub fn fusion_yield(&self) -> f64 {
        if self.can_ignite() {
            self.laser_energy * 10.0 // Gain of 10x
        } else {
            0.0
        }
    }
}

/// Plasma simulator
pub struct PlasmaSimulator {
    pub plasma: Option<Plasma>,
    pub tokamak: Option<Tokamak>,
    pub mhd: Option<MHD>,
    pub icf: Option<ICF>,
}

impl PlasmaSimulator {
    pub fn new() -> Self {
        Self {
            plasma: None,
            tokamak: None,
            mhd: None,
            icf: None,
        }
    }

    pub fn create_plasma(&mut self, temp: f64, density: f64, b_field: f64) {
        self.plasma = Some(Plasma::new(temp, density, b_field));
    }

    pub fn create_tokamak(&mut self, r_major: f64, r_minor: f64, temp: f64, density: f64, b_field: f64) {
        self.tokamak = Some(Tokamak::new(r_major, r_minor, temp, density, b_field));
    }

    pub fn create_icf(&mut self, radius: f64, energy: f64) {
        self.icf = Some(ICF::new(radius, energy));
    }

    pub fn debye_length(&self) -> Result<f64> {
        self.plasma
            .as_ref()
            .map(|p| p.debye_length())
            .ok_or_else(|| PlasmaError::InvalidPlasma("No plasma".to_string()))
    }

    pub fn plasma_frequency(&self) -> Result<f64> {
        self.plasma
            .as_ref()
            .map(|p| p.plasma_frequency())
            .ok_or_else(|| PlasmaError::InvalidPlasma("No plasma".to_string()))
    }

    pub fn tokamak_q_factor(&self, heating_power: f64, confinement_time: f64) -> Result<f64> {
        self.tokamak
            .as_ref()
            .map(|t| t.q_factor(heating_power, confinement_time))
            .ok_or_else(|| PlasmaError::InvalidPlasma("No tokamak".to_string()))
    }

    pub fn tokamak_is_ignition(&self, heating_power: f64, confinement_time: f64) -> Result<bool> {
        self.tokamak
            .as_ref()
            .map(|t| t.is_ignition(heating_power, confinement_time))
            .ok_or_else(|| PlasmaError::InvalidPlasma("No tokamak".to_string()))
    }

    pub fn icf_can_ignite(&self) -> Result<bool> {
        self.icf
            .as_ref()
            .map(|i| i.can_ignite())
            .ok_or_else(|| PlasmaError::InvalidPlasma("No ICF".to_string()))
    }
}

impl Default for PlasmaSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_plasma() {
        let plasma = Plasma::new(1e7, 1e20, 5.0);
        let lambda_d = plasma.debye_length();
        assert!(lambda_d > 0.0);

        let omega_p = plasma.plasma_frequency();
        assert!(omega_p > 0.0);
    }

    #[test]
    fn test_tokamak() {
        let tokamak = Tokamak::new(6.2, 2.0, 1e8, 1e20, 5.3);
        let aspect = tokamak.aspect_ratio();
        assert_relative_eq!(aspect, 3.1, epsilon = 0.1);

        let q = tokamak.safety_factor(1.0);
        assert!(q > 0.0);
    }

    #[test]
    fn test_mhd() {
        let v = Array1::from_vec(vec![1e5, 0.0, 0.0]);
        let b = Array1::from_vec(vec![0.0, 5.0, 0.0]);
        let mhd = MHD::new(v, b, 1e-8, 1e5);

        let v_a = mhd.alfven_velocity();
        assert!(v_a > 0.0);

        let beta = mhd.beta();
        assert!(beta > 0.0);
    }

    #[test]
    fn test_icf() {
        let icf = ICF::new(1.0, 1e6);
        let rho_r = icf.rho_r();
        assert!(rho_r > 0.0);
    }

    #[test]
    fn test_simulator() {
        let mut sim = PlasmaSimulator::new();

        sim.create_plasma(1e7, 1e20, 5.0);
        assert!(sim.debye_length().is_ok());
        assert!(sim.plasma_frequency().is_ok());

        sim.create_tokamak(6.2, 2.0, 1e8, 1e20, 5.3);
        assert!(sim.tokamak_q_factor(5e7, 1.0).is_ok());

        sim.create_icf(1.0, 1e6);
        assert!(sim.icf_can_ignite().is_ok());
    }
}
