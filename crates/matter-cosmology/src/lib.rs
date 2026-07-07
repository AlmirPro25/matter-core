//! # Matter Cosmology
//!
//! Cosmological simulations for Matter Core.
//!
//! ## Features
//! - **Expanding Universe**: Hubble law, scale factor
//! - **Cosmic Microwave Background**: Temperature, power spectrum
//! - **Dark Matter**: Density, gravitational effects
//! - **Dark Energy**: Cosmological constant, acceleration
//! - **Big Bang**: Age, critical density, composition
//! - **Large Scale Structure**: Galaxy clustering, voids
//!
//! ## Physics Basis
//! All simulations use peer-reviewed cosmological equations:
//! - Hubble's Law: v = H₀d
//! - Friedmann Equations: (ȧ/a)² = (8πG/3)ρ - k/a²
//! - CMB Temperature: T = T₀/a
//! - Dark Energy: w = P/(ρc²)
//! - Critical Density: ρ_c = 3H²/(8πG)

pub mod backend;

// ============================================================================
// EXPANDING UNIVERSE
// ============================================================================

/// Universe expansion state
#[derive(Debug, Clone)]
pub struct Universe {
    /// Scale factor (a)
    pub scale_factor: f64,
    /// Hubble parameter (km/s/Mpc)
    pub hubble_parameter: f64,
    /// Age (Gyr)
    pub age: f64,
}

impl Universe {
    /// Create universe at current epoch
    pub fn current() -> Self {
        Self {
            scale_factor: 1.0,  // Present day
            hubble_parameter: 67.4,  // km/s/Mpc (Planck 2018)
            age: 13.8,  // Gyr
        }
    }
    
    /// Create universe at redshift z
    pub fn at_redshift(z: f64) -> Self {
        let scale_factor = 1.0 / (1.0 + z);
        let h0 = 67.4;  // Present Hubble constant
        
        // Simplified: assumes flat ΛCDM
        let omega_m = 0.315;  // Matter density
        let omega_l = 0.685;  // Dark energy density
        
        // Hubble parameter at redshift z
        let h_z = h0 * (omega_m * (1.0 + z).powi(3) + omega_l).sqrt();
        
        // Age at redshift z (simplified)
        let age = 13.8 * scale_factor.powf(1.5);
        
        Self {
            scale_factor,
            hubble_parameter: h_z,
            age,
        }
    }
    
    /// Calculate velocity from distance (Hubble's Law)
    /// v = H₀d
    pub fn recession_velocity(&self, distance_mpc: f64) -> f64 {
        self.hubble_parameter * distance_mpc
    }
    
    /// Calculate distance from redshift (comoving distance)
    /// Simplified for flat universe
    pub fn comoving_distance(&self, z: f64) -> f64 {
        const C: f64 = 299792.458;  // km/s
        let h0 = 67.4;
        
        // Simplified integral for ΛCDM
        C * z / h0
    }
    
    /// Calculate lookback time (Gyr)
    pub fn lookback_time(&self, z: f64) -> f64 {
        let age_then = Universe::at_redshift(z).age;
        self.age - age_then
    }
    
    /// Calculate critical density (kg/m³)
    /// ρ_c = 3H²/(8πG)
    pub fn critical_density(&self) -> f64 {
        const G: f64 = 6.674e-11;  // m³/(kg·s²)
        const MPC_TO_M: f64 = 3.086e22;  // m/Mpc
        
        // Convert H from km/s/Mpc to 1/s
        let h_si = self.hubble_parameter * 1000.0 / MPC_TO_M;
        
        3.0 * h_si * h_si / (8.0 * std::f64::consts::PI * G)
    }
}

// ============================================================================
// COSMIC MICROWAVE BACKGROUND (CMB)
// ============================================================================

/// CMB radiation properties
#[derive(Debug, Clone)]
pub struct CMB {
    /// Temperature (K)
    pub temperature: f64,
    /// Redshift
    pub redshift: f64,
}

impl CMB {
    /// Create CMB at current epoch
    pub fn current() -> Self {
        Self {
            temperature: 2.725,  // K (COBE/WMAP/Planck)
            redshift: 1089.0,    // Recombination redshift
        }
    }
    
    /// CMB temperature at redshift z
    /// T(z) = T₀(1 + z)
    pub fn temperature_at_redshift(z: f64) -> f64 {
        2.725 * (1.0 + z)
    }
    
    /// Calculate photon number density (m⁻³)
    /// n_γ ≈ 411 photons/cm³ today
    pub fn photon_density(&self) -> f64 {
        411e6 * (1.0 + self.redshift).powi(3)
    }
    
    /// Calculate energy density (J/m³)
    /// u = aT⁴
    pub fn energy_density(&self) -> f64 {
        const A: f64 = 7.566e-16;  // Radiation constant J/(m³·K⁴)
        A * self.temperature.powi(4)
    }
    
    /// Calculate dipole anisotropy (peculiar velocity)
    /// ΔT/T = v/c
    pub fn dipole_velocity(&self, delta_t_over_t: f64) -> f64 {
        const C: f64 = 2.998e8;  // m/s
        delta_t_over_t * C
    }
}

// ============================================================================
// DARK MATTER
// ============================================================================

/// Dark matter properties
#[derive(Debug, Clone)]
pub struct DarkMatter {
    /// Density parameter (Ω_DM)
    pub omega_dm: f64,
    /// Halo mass (M☉)
    pub halo_mass: f64,
}

impl DarkMatter {
    /// Create dark matter with current parameters
    pub fn new() -> Self {
        Self {
            omega_dm: 0.265,  // Planck 2018
            halo_mass: 1e12,  // Typical galaxy halo
        }
    }
    
    /// Calculate dark matter density (kg/m³)
    pub fn density(&self, universe: &Universe) -> f64 {
        let rho_c = universe.critical_density();
        self.omega_dm * rho_c
    }
    
    /// Calculate virial velocity (km/s)
    /// V_vir = √(GM/R)
    pub fn virial_velocity(&self) -> f64 {
        const G: f64 = 4.3e-6;  // (km/s)²·kpc/M☉
        let r_vir = 200.0;  // kpc (typical)
        
        (G * self.halo_mass / r_vir).sqrt()
    }
    
    /// NFW profile density at radius r
    /// ρ(r) = ρ_s / [(r/r_s)(1 + r/r_s)²]
    pub fn nfw_density(&self, r_kpc: f64, r_s: f64, rho_s: f64) -> f64 {
        let x = r_kpc / r_s;
        rho_s / (x * (1.0 + x).powi(2))
    }
}

impl Default for DarkMatter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DARK ENERGY
// ============================================================================

/// Dark energy properties
#[derive(Debug, Clone)]
pub struct DarkEnergy {
    /// Density parameter (Ω_Λ)
    pub omega_lambda: f64,
    /// Equation of state (w)
    pub w: f64,
}

impl DarkEnergy {
    /// Create dark energy (cosmological constant)
    pub fn cosmological_constant() -> Self {
        Self {
            omega_lambda: 0.685,  // Planck 2018
            w: -1.0,  // Cosmological constant
        }
    }
    
    /// Calculate dark energy density (kg/m³)
    pub fn density(&self, universe: &Universe) -> f64 {
        let rho_c = universe.critical_density();
        self.omega_lambda * rho_c
    }
    
    /// Calculate pressure (Pa)
    /// P = w*ρ*c²
    pub fn pressure(&self, universe: &Universe) -> f64 {
        const C: f64 = 2.998e8;  // m/s
        let rho = self.density(universe);
        self.w * rho * C * C
    }
    
    /// Calculate acceleration scale factor
    /// ä/a = -(4πG/3)(ρ + 3P/c²)
    pub fn acceleration(&self, universe: &Universe) -> f64 {
        const G: f64 = 6.674e-11;
        const C: f64 = 2.998e8;
        
        let rho = self.density(universe);
        let p = self.pressure(universe);
        
        -(4.0 * std::f64::consts::PI * G / 3.0) * (rho + 3.0 * p / (C * C))
    }
}

// ============================================================================
// COSMIC COMPOSITION
// ============================================================================

/// Universe composition
#[derive(Debug, Clone)]
pub struct Composition {
    /// Baryonic matter (Ω_b)
    pub omega_baryon: f64,
    /// Dark matter (Ω_DM)
    pub omega_dark_matter: f64,
    /// Dark energy (Ω_Λ)
    pub omega_dark_energy: f64,
    /// Radiation (Ω_r)
    pub omega_radiation: f64,
}

impl Composition {
    /// Current universe composition (Planck 2018)
    pub fn current() -> Self {
        Self {
            omega_baryon: 0.049,
            omega_dark_matter: 0.265,
            omega_dark_energy: 0.685,
            omega_radiation: 9.2e-5,
        }
    }
    
    /// Calculate total density parameter
    /// Ω_total = Ω_b + Ω_DM + Ω_Λ + Ω_r
    pub fn total_omega(&self) -> f64 {
        self.omega_baryon + self.omega_dark_matter + 
        self.omega_dark_energy + self.omega_radiation
    }
    
    /// Check if universe is flat (Ω_total ≈ 1)
    pub fn is_flat(&self) -> bool {
        (self.total_omega() - 1.0).abs() < 0.01
    }
    
    /// Calculate matter-radiation equality redshift
    pub fn matter_radiation_equality(&self) -> f64 {
        let omega_m = self.omega_baryon + self.omega_dark_matter;
        omega_m / self.omega_radiation - 1.0
    }
    
    /// Calculate matter-dark energy equality redshift
    pub fn matter_dark_energy_equality(&self) -> f64 {
        let omega_m = self.omega_baryon + self.omega_dark_matter;
        (omega_m / self.omega_dark_energy).powf(1.0 / 3.0) - 1.0
    }
}

// ============================================================================
// STRUCTURE FORMATION
// ============================================================================

/// Large scale structure
#[derive(Debug, Clone)]
pub struct Structure {
    /// Mass scale (M☉)
    pub mass: f64,
    /// Size scale (Mpc)
    pub size: f64,
}

impl Structure {
    /// Create structure (galaxy, cluster, etc.)
    pub fn new(mass: f64, size: f64) -> Self {
        Self { mass, size }
    }
    
    /// Calculate Jeans mass (M☉)
    /// M_J = (5kT/GμmH)^(3/2) * (3/4πρ)^(1/2)
    pub fn jeans_mass(temperature: f64, density: f64) -> f64 {
        const K: f64 = 1.381e-23;  // J/K
        const G: f64 = 6.674e-11;  // m³/(kg·s²)
        const MH: f64 = 1.673e-27;  // kg
        const MSUN: f64 = 1.989e30;  // kg
        
        let mu = 0.6;  // Mean molecular weight
        let cs2 = 5.0 * K * temperature / (mu * MH * G);
        let m_j = cs2.powf(1.5) * (3.0 / (4.0 * std::f64::consts::PI * density * G)).sqrt();
        
        m_j / MSUN
    }
    
    /// Calculate free fall time (s)
    /// t_ff = √(3π/32Gρ)
    pub fn free_fall_time(density: f64) -> f64 {
        const G: f64 = 6.674e-11;
        (3.0 * std::f64::consts::PI / (32.0 * G * density)).sqrt()
    }
    
    /// Calculate two-point correlation function
    /// ξ(r) = (r₀/r)^γ
    pub fn correlation_function(r_mpc: f64) -> f64 {
        let r0 = 5.0;  // Mpc (correlation length)
        let gamma = 1.8;  // Power law index
        
        (r0 / r_mpc).powf(gamma)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_universe() {
        let universe = Universe::current();
        
        // Check current values
        assert!((universe.scale_factor - 1.0).abs() < 0.01);
        assert!(universe.hubble_parameter > 60.0 && universe.hubble_parameter < 75.0);
        assert!(universe.age > 13.0 && universe.age < 15.0);
        
        // Hubble's law
        let v = universe.recession_velocity(100.0);  // 100 Mpc
        assert!(v > 6000.0);  // Should be ~6740 km/s
        
        // Critical density should be positive
        let rho_c = universe.critical_density();
        assert!(rho_c > 0.0);
    }
    
    #[test]
    fn test_redshift() {
        let z = 2.0;
        let universe = Universe::at_redshift(z);
        
        // Scale factor
        assert!((universe.scale_factor - 1.0/3.0).abs() < 0.01);
        
        // Lookback time
        let current = Universe::current();
        let t_look = current.lookback_time(z);
        assert!(t_look > 0.0 && t_look < current.age);
    }
    
    #[test]
    fn test_cmb() {
        let cmb = CMB::current();
        
        // Temperature
        assert!((cmb.temperature - 2.725).abs() < 0.01);
        
        // Temperature at recombination
        let t_rec = CMB::temperature_at_redshift(1089.0);
        assert!(t_rec > 2900.0 && t_rec < 3100.0);
        
        // Photon density
        let n_gamma = cmb.photon_density();
        assert!(n_gamma > 0.0);
    }
    
    #[test]
    fn test_dark_matter() {
        let dm = DarkMatter::new();
        let universe = Universe::current();
        
        // Density
        let rho = dm.density(&universe);
        assert!(rho > 0.0);
        
        // Virial velocity
        let v_vir = dm.virial_velocity();
        assert!(v_vir > 100.0 && v_vir < 500.0);
    }
    
    #[test]
    fn test_dark_energy() {
        let de = DarkEnergy::cosmological_constant();
        let universe = Universe::current();
        
        // Equation of state
        assert!((de.w + 1.0).abs() < 0.01);
        
        // Density
        let rho = de.density(&universe);
        assert!(rho > 0.0);
        
        // Pressure (negative for dark energy)
        let p = de.pressure(&universe);
        assert!(p < 0.0);
    }
    
    #[test]
    fn test_composition() {
        let comp = Composition::current();
        
        // Total should be ~1 (flat universe)
        let total = comp.total_omega();
        assert!((total - 1.0).abs() < 0.01);
        
        // Should be flat
        assert!(comp.is_flat());
        
        // Matter-radiation equality should be in past
        let z_eq = comp.matter_radiation_equality();
        assert!(z_eq > 3000.0);
    }
    
    #[test]
    fn test_structure() {
        // Jeans mass
        let m_j = Structure::jeans_mass(1e4, 1e-20);
        assert!(m_j > 0.0);
        
        // Correlation function
        let xi = Structure::correlation_function(10.0);
        assert!(xi > 0.0);
    }
}
