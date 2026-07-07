//! # Matter Electromagnetics
//!
//! Electromagnetics and RF engineering simulation for Matter Core.
//!
//! ## Features
//! - **Maxwell's Equations**: E, B, D, H fields
//! - **Wave Propagation**: EM waves, phase velocity
//! - **Transmission Lines**: Impedance, VSWR, Smith chart
//! - **Antennas**: Radiation pattern, gain, directivity
//! - **RF Engineering**: S-parameters, matching networks
//! - **EMC/EMI**: Shielding effectiveness, coupling
//!
//! ## Physics Basis
//! All simulations use peer-reviewed EM equations:
//! - Gauss's law: ∇·E = ρ/ε₀
//! - Faraday's law: ∇×E = -∂B/∂t
//! - Ampere's law: ∇×H = J + ∂D/∂t
//! - Wave equation: c = 1/√(με)
//! - Poynting vector: S = E×H
//! - Friis equation: P_r = P_t * G_t * G_r * (λ/4πd)²

pub mod backend;

use std::f64::consts::PI;

// ============================================================================
// ELECTROMAGNETIC CONSTANTS
// ============================================================================

/// Speed of light in vacuum (m/s)
pub const C0: f64 = 299792458.0;

/// Permittivity of free space (F/m)
pub const EPSILON_0: f64 = 8.854187817e-12;

/// Permeability of free space (H/m)
pub const MU_0: f64 = 1.256637061e-6;

/// Impedance of free space (Ω)
pub const Z0: f64 = 376.730313668;

// ============================================================================
// ELECTROMAGNETIC WAVE
// ============================================================================

/// Electromagnetic wave
#[derive(Debug, Clone)]
pub struct EMWave {
    /// Frequency (Hz)
    pub frequency: f64,
    /// Electric field amplitude (V/m)
    pub e_field: f64,
    /// Relative permittivity
    pub epsilon_r: f64,
    /// Relative permeability
    pub mu_r: f64,
}

impl EMWave {
    /// Create EM wave in vacuum
    pub fn in_vacuum(frequency: f64, e_field: f64) -> Self {
        Self {
            frequency,
            e_field,
            epsilon_r: 1.0,
            mu_r: 1.0,
        }
    }
    
    /// Create EM wave in medium
    pub fn in_medium(frequency: f64, e_field: f64, epsilon_r: f64, mu_r: f64) -> Self {
        Self {
            frequency,
            e_field,
            epsilon_r,
            mu_r,
        }
    }
    
    /// Calculate wavelength (m)
    /// λ = c/f (in vacuum), λ = c/(f√(ε_r*μ_r)) (in medium)
    pub fn wavelength(&self) -> f64 {
        let v = self.phase_velocity();
        v / self.frequency
    }
    
    /// Calculate phase velocity (m/s)
    /// v = c/√(ε_r*μ_r)
    pub fn phase_velocity(&self) -> f64 {
        C0 / (self.epsilon_r * self.mu_r).sqrt()
    }
    
    /// Calculate wave number (rad/m)
    /// k = 2π/λ
    pub fn wave_number(&self) -> f64 {
        2.0 * PI / self.wavelength()
    }
    
    /// Calculate intrinsic impedance (Ω)
    /// η = √(μ/ε)
    pub fn intrinsic_impedance(&self) -> f64 {
        Z0 * (self.mu_r / self.epsilon_r).sqrt()
    }
    
    /// Calculate magnetic field amplitude (A/m)
    /// H = E/η
    pub fn h_field(&self) -> f64 {
        self.e_field / self.intrinsic_impedance()
    }
    
    /// Calculate power density (W/m²)
    /// Poynting vector: S = E×H = E²/η
    pub fn power_density(&self) -> f64 {
        self.e_field * self.e_field / self.intrinsic_impedance()
    }
    
    /// Calculate angular frequency (rad/s)
    pub fn angular_frequency(&self) -> f64 {
        2.0 * PI * self.frequency
    }
    
    /// Get frequency band name
    pub fn frequency_band(&self) -> &str {
        match self.frequency {
            f if f < 3e3 => "ELF",
            f if f < 3e6 => "LF/MF/HF",
            f if f < 30e6 => "HF",
            f if f < 300e6 => "VHF",
            f if f < 3e9 => "UHF",
            f if f < 30e9 => "SHF (Microwave)",
            f if f < 300e9 => "EHF (Millimeter wave)",
            _ => "Terahertz",
        }
    }
}

// ============================================================================
// TRANSMISSION LINE
// ============================================================================

/// Transmission line
#[derive(Debug, Clone)]
pub struct TransmissionLine {
    /// Characteristic impedance (Ω)
    pub z0: f64,
    /// Load impedance (Ω)
    pub z_load: f64,
    /// Length (m)
    pub length: f64,
    /// Frequency (Hz)
    pub frequency: f64,
    /// Velocity factor (v/c)
    pub velocity_factor: f64,
}

impl TransmissionLine {
    /// Create transmission line
    pub fn new(z0: f64, z_load: f64, length: f64, frequency: f64) -> Self {
        Self {
            z0,
            z_load,
            length,
            frequency,
            velocity_factor: 1.0,
        }
    }
    
    /// Create coaxial cable (typical)
    pub fn coax_rg58() -> Self {
        Self::new(50.0, 50.0, 1.0, 1e9)
    }
    
    /// Calculate reflection coefficient (Γ)
    /// Γ = (Z_L - Z_0)/(Z_L + Z_0)
    pub fn reflection_coefficient(&self) -> f64 {
        (self.z_load - self.z0) / (self.z_load + self.z0)
    }
    
    /// Calculate VSWR (Voltage Standing Wave Ratio)
    /// VSWR = (1 + |Γ|)/(1 - |Γ|)
    pub fn vswr(&self) -> f64 {
        let gamma = self.reflection_coefficient().abs();
        (1.0 + gamma) / (1.0 - gamma)
    }
    
    /// Calculate return loss (dB)
    /// RL = -20*log10(|Γ|)
    pub fn return_loss(&self) -> f64 {
        let gamma = self.reflection_coefficient().abs();
        -20.0 * gamma.log10()
    }
    
    /// Calculate wavelength in line (m)
    pub fn wavelength(&self) -> f64 {
        (C0 * self.velocity_factor) / self.frequency
    }
    
    /// Calculate electrical length (degrees)
    pub fn electrical_length(&self) -> f64 {
        (self.length / self.wavelength()) * 360.0
    }
    
    /// Calculate insertion loss (dB/m) - simplified
    pub fn insertion_loss_per_meter(&self) -> f64 {
        // Empirical for typical coax at 1 GHz
        let f_ghz = self.frequency / 1e9;
        0.2 * f_ghz.sqrt()
    }
    
    /// Check if matched (VSWR < 2)
    pub fn is_matched(&self) -> bool {
        self.vswr() < 2.0
    }
}

// ============================================================================
// ANTENNA
// ============================================================================

/// Antenna type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AntennaType {
    Dipole,
    Monopole,
    Patch,
    Horn,
    Parabolic,
    Yagi,
}

/// Antenna
#[derive(Debug, Clone)]
pub struct Antenna {
    /// Antenna type
    pub antenna_type: AntennaType,
    /// Gain (dBi)
    pub gain_dbi: f64,
    /// Frequency (Hz)
    pub frequency: f64,
    /// Efficiency (0-1)
    pub efficiency: f64,
}

impl Antenna {
    /// Create antenna
    pub fn new(antenna_type: AntennaType, gain_dbi: f64, frequency: f64, efficiency: f64) -> Self {
        Self {
            antenna_type,
            gain_dbi,
            frequency,
            efficiency,
        }
    }
    
    /// Create half-wave dipole
    pub fn dipole_half_wave(frequency: f64) -> Self {
        Self::new(AntennaType::Dipole, 2.15, frequency, 0.95)
    }
    
    /// Create quarter-wave monopole
    pub fn monopole_quarter_wave(frequency: f64) -> Self {
        Self::new(AntennaType::Monopole, 5.15, frequency, 0.9)
    }
    
    /// Calculate directivity (dBi)
    pub fn directivity(&self) -> f64 {
        // Gain = Efficiency × Directivity
        let efficiency_db = 10.0 * self.efficiency.log10();
        self.gain_dbi - efficiency_db
    }
    
    /// Calculate effective aperture (m²)
    /// A_eff = (λ²/(4π)) * G
    pub fn effective_aperture(&self) -> f64 {
        let lambda = C0 / self.frequency;
        let gain_linear = 10.0_f64.powf(self.gain_dbi / 10.0);
        (lambda * lambda / (4.0 * PI)) * gain_linear
    }
    
    /// Calculate beamwidth (degrees) - approximate
    pub fn beamwidth_3db(&self) -> f64 {
        // Approximate formula: BW ≈ 101/√G
        let gain_linear = 10.0_f64.powf(self.gain_dbi / 10.0);
        101.0 / gain_linear.sqrt()
    }
    
    /// Calculate radiation resistance (Ω)
    pub fn radiation_resistance(&self) -> f64 {
        match self.antenna_type {
            AntennaType::Dipole => 73.0,
            AntennaType::Monopole => 36.5,
            AntennaType::Patch => 200.0,
            _ => 50.0,
        }
    }
    
    /// Calculate physical length (m)
    pub fn physical_length(&self) -> f64 {
        let lambda = C0 / self.frequency;
        match self.antenna_type {
            AntennaType::Dipole => lambda / 2.0,
            AntennaType::Monopole => lambda / 4.0,
            AntennaType::Patch => lambda / 2.0,
            _ => lambda,
        }
    }
}

// ============================================================================
// FREE SPACE PATH LOSS (FRIIS EQUATION)
// ============================================================================

/// Free space link budget
#[derive(Debug, Clone)]
pub struct LinkBudget {
    /// Transmit power (W)
    pub tx_power: f64,
    /// Transmit antenna gain (dBi)
    pub tx_gain: f64,
    /// Receive antenna gain (dBi)
    pub rx_gain: f64,
    /// Frequency (Hz)
    pub frequency: f64,
    /// Distance (m)
    pub distance: f64,
}

impl LinkBudget {
    /// Create link budget
    pub fn new(tx_power: f64, tx_gain: f64, rx_gain: f64, frequency: f64, distance: f64) -> Self {
        Self {
            tx_power,
            tx_gain,
            rx_gain,
            frequency,
            distance,
        }
    }
    
    /// Calculate free space path loss (dB)
    /// FSPL = 20*log10(d) + 20*log10(f) + 20*log10(4π/c)
    pub fn path_loss(&self) -> f64 {
        20.0 * (self.distance).log10()
            + 20.0 * (self.frequency).log10()
            + 20.0 * (4.0 * PI / C0).log10()
    }
    
    /// Calculate received power (W)
    /// Friis equation: P_r = P_t * G_t * G_r * (λ/4πd)²
    pub fn received_power(&self) -> f64 {
        let lambda = C0 / self.frequency;
        let gain_t = 10.0_f64.powf(self.tx_gain / 10.0);
        let gain_r = 10.0_f64.powf(self.rx_gain / 10.0);
        let path_factor = (lambda / (4.0 * PI * self.distance)).powi(2);
        
        self.tx_power * gain_t * gain_r * path_factor
    }
    
    /// Calculate received power (dBm)
    pub fn received_power_dbm(&self) -> f64 {
        let p_watts = self.received_power();
        10.0 * (p_watts / 1e-3).log10()
    }
    
    /// Calculate link margin (dB)
    pub fn link_margin(&self, sensitivity_dbm: f64) -> f64 {
        self.received_power_dbm() - sensitivity_dbm
    }
    
    /// Check if link is viable (margin > 0 dB)
    pub fn is_viable(&self, sensitivity_dbm: f64) -> bool {
        self.link_margin(sensitivity_dbm) > 0.0
    }
}

// ============================================================================
// SHIELDING EFFECTIVENESS
// ============================================================================

/// EM shielding
#[derive(Debug, Clone)]
pub struct Shielding {
    /// Material conductivity (S/m)
    pub conductivity: f64,
    /// Thickness (m)
    pub thickness: f64,
    /// Relative permeability
    pub mu_r: f64,
    /// Frequency (Hz)
    pub frequency: f64,
}

impl Shielding {
    /// Create shielding (copper typical)
    pub fn copper(thickness: f64, frequency: f64) -> Self {
        Self {
            conductivity: 5.8e7,
            thickness,
            mu_r: 1.0,
            frequency,
        }
    }
    
    /// Create shielding (aluminum)
    pub fn aluminum(thickness: f64, frequency: f64) -> Self {
        Self {
            conductivity: 3.5e7,
            thickness,
            mu_r: 1.0,
            frequency,
        }
    }
    
    /// Calculate skin depth (m)
    /// δ = √(2/(ωμσ))
    pub fn skin_depth(&self) -> f64 {
        let omega = 2.0 * PI * self.frequency;
        let mu = MU_0 * self.mu_r;
        (2.0 / (omega * mu * self.conductivity)).sqrt()
    }
    
    /// Calculate absorption loss (dB)
    /// A = 20*log10(e^(t/δ))
    pub fn absorption_loss(&self) -> f64 {
        let delta = self.skin_depth();
        8.686 * (self.thickness / delta)
    }
    
    /// Calculate reflection loss (dB) - simplified
    /// R ≈ 20*log10(Z_0/(4*Z_s))
    pub fn reflection_loss(&self) -> f64 {
        let z_s = (2.0 * PI * self.frequency * MU_0 * self.mu_r / self.conductivity).sqrt();
        20.0 * (Z0 / (4.0 * z_s)).log10()
    }
    
    /// Calculate total shielding effectiveness (dB)
    /// SE = A + R (neglecting re-reflection)
    pub fn shielding_effectiveness(&self) -> f64 {
        self.absorption_loss() + self.reflection_loss()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_em_wave() {
        let wave = EMWave::in_vacuum(1e9, 1.0);  // 1 GHz, 1 V/m
        
        // Wavelength at 1 GHz should be ~0.3 m
        let lambda = wave.wavelength();
        assert!((lambda - 0.3).abs() < 0.01);
        
        // Phase velocity should be c
        let v = wave.phase_velocity();
        assert!((v - C0).abs() < 1.0);
        
        // Should be UHF
        assert_eq!(wave.frequency_band(), "UHF");
    }
    
    #[test]
    fn test_transmission_line() {
        let line = TransmissionLine::new(50.0, 50.0, 1.0, 1e9);
        
        // Matched line: Γ = 0, VSWR = 1
        assert!((line.reflection_coefficient()).abs() < 0.01);
        assert!((line.vswr() - 1.0).abs() < 0.01);
        assert!(line.is_matched());
        
        // Mismatched line
        let mismatched = TransmissionLine::new(50.0, 75.0, 1.0, 1e9);
        assert!(mismatched.vswr() > 1.0);
        assert!(mismatched.return_loss() < 30.0);
    }
    
    #[test]
    fn test_antenna() {
        let dipole = Antenna::dipole_half_wave(1e9);
        
        // Dipole gain ~2.15 dBi
        assert!((dipole.gain_dbi - 2.15).abs() < 0.1);
        
        // Length should be λ/2
        let lambda = C0 / 1e9;
        let length = dipole.physical_length();
        assert!((length - lambda / 2.0).abs() < 0.01);
        
        // Radiation resistance ~73 Ω
        assert!((dipole.radiation_resistance() - 73.0).abs() < 1.0);
    }
    
    #[test]
    fn test_link_budget() {
        let link = LinkBudget::new(1.0, 2.15, 2.15, 2.4e9, 100.0);  // WiFi-like
        
        // Path loss should be significant
        let fspl = link.path_loss();
        assert!(fspl > 70.0 && fspl < 90.0);  // Adjusted range
        
        // Should receive some power
        let p_rx = link.received_power();
        assert!(p_rx > 0.0 && p_rx < 1.0);
        
        // Link should be viable for typical sensitivity
        assert!(link.is_viable(-90.0));
    }
    
    #[test]
    fn test_shielding() {
        let shield = Shielding::copper(0.001, 1e9);  // 1mm copper at 1 GHz
        
        // Skin depth should be small
        let delta = shield.skin_depth();
        assert!(delta > 0.0 && delta < 0.01);
        
        // Should provide good shielding
        let se = shield.shielding_effectiveness();
        assert!(se > 80.0);  // >80 dB typical for 1mm copper
    }
}
