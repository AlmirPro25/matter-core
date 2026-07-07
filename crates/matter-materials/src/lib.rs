//! # Matter Materials
//!
//! Materials science and crystallography simulation for Matter Core.
//!
//! ## Features
//! - **Crystallography**: Lattice structures, unit cells
//! - **X-ray Diffraction**: Bragg's law, structure factors
//! - **Mechanical Properties**: Stress, strain, elasticity
//! - **Electronic Structure**: Band theory, DOS
//! - **Phase Diagrams**: Phase transitions, critical points
//! - **Defects**: Point defects, dislocations
//!
//! ## Physics Basis
//! All simulations use peer-reviewed materials science:
//! - Bragg's law: nλ = 2d*sin(θ)
//! - Hooke's law: σ = E*ε
//! - Hall-Petch relation: σ_y = σ_0 + k/√d
//! - Fermi-Dirac distribution: f(E) = 1/(1 + exp((E-E_F)/kT))
//! - Clausius-Clapeyron: dP/dT = ΔH/(T*ΔV)

pub mod backend;

use std::f64::consts::PI;

// ============================================================================
// CRYSTALLOGRAPHY
// ============================================================================

/// Crystal lattice type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LatticeType {
    SimpleCubic,
    BodyCenteredCubic,  // BCC
    FaceCenteredCubic,  // FCC
    Hexagonal,
    Tetragonal,
    Orthorhombic,
    Monoclinic,
    Triclinic,
}

impl LatticeType {
    /// Get number of atoms per unit cell
    pub fn atoms_per_cell(&self) -> usize {
        match self {
            LatticeType::SimpleCubic => 1,
            LatticeType::BodyCenteredCubic => 2,
            LatticeType::FaceCenteredCubic => 4,
            LatticeType::Hexagonal => 2,
            LatticeType::Tetragonal => 1,
            LatticeType::Orthorhombic => 1,
            LatticeType::Monoclinic => 1,
            LatticeType::Triclinic => 1,
        }
    }
    
    /// Get packing efficiency (fraction of volume occupied)
    pub fn packing_efficiency(&self) -> f64 {
        match self {
            LatticeType::SimpleCubic => PI / 6.0,  // ~0.524
            LatticeType::BodyCenteredCubic => PI * 3.0_f64.sqrt() / 8.0,  // ~0.680
            LatticeType::FaceCenteredCubic => PI / (3.0 * 2.0_f64.sqrt()),  // ~0.740
            LatticeType::Hexagonal => PI / (3.0 * 2.0_f64.sqrt()),  // ~0.740 (HCP)
            _ => 0.5,  // Approximate for others
        }
    }
    
    /// Get coordination number
    pub fn coordination_number(&self) -> usize {
        match self {
            LatticeType::SimpleCubic => 6,
            LatticeType::BodyCenteredCubic => 8,
            LatticeType::FaceCenteredCubic => 12,
            LatticeType::Hexagonal => 12,
            LatticeType::Tetragonal => 6,
            LatticeType::Orthorhombic => 6,
            _ => 6,
        }
    }
}

/// Crystal structure
#[derive(Debug, Clone)]
pub struct Crystal {
    /// Lattice type
    pub lattice_type: LatticeType,
    /// Lattice parameter a (Å)
    pub a: f64,
    /// Lattice parameter b (Å)
    pub b: f64,
    /// Lattice parameter c (Å)
    pub c: f64,
    /// Miller indices (h, k, l)
    pub miller_indices: (i32, i32, i32),
}

impl Crystal {
    /// Create cubic crystal
    pub fn cubic(lattice_type: LatticeType, a: f64) -> Self {
        Self {
            lattice_type,
            a,
            b: a,
            c: a,
            miller_indices: (1, 0, 0),
        }
    }
    
    /// Create hexagonal crystal
    pub fn hexagonal(a: f64, c: f64) -> Self {
        Self {
            lattice_type: LatticeType::Hexagonal,
            a,
            b: a,
            c,
            miller_indices: (1, 0, 0),
        }
    }
    
    /// Calculate unit cell volume (Ų)
    pub fn unit_cell_volume(&self) -> f64 {
        match self.lattice_type {
            LatticeType::Hexagonal => {
                // V = √3/2 * a² * c
                (3.0_f64.sqrt() / 2.0) * self.a * self.a * self.c
            }
            _ => {
                // Cubic: V = abc
                self.a * self.b * self.c
            }
        }
    }
    
    /// Calculate d-spacing for Miller indices (Å)
    /// For cubic: d = a/√(h² + k² + l²)
    pub fn d_spacing(&self, h: i32, k: i32, l: i32) -> f64 {
        let h2 = (h * h) as f64;
        let k2 = (k * k) as f64;
        let l2 = (l * l) as f64;
        
        match self.lattice_type {
            LatticeType::SimpleCubic | 
            LatticeType::BodyCenteredCubic | 
            LatticeType::FaceCenteredCubic => {
                self.a / (h2 + k2 + l2).sqrt()
            }
            LatticeType::Hexagonal => {
                // 1/d² = 4/3 * (h² + hk + k²)/a² + l²/c²
                let term1 = (4.0 / 3.0) * (h2 + (h * k) as f64 + k2) / (self.a * self.a);
                let term2 = l2 / (self.c * self.c);
                1.0 / (term1 + term2).sqrt()
            }
            _ => self.a / (h2 + k2 + l2).sqrt(),
        }
    }
    
    /// Calculate atomic density (atoms/ų)
    pub fn atomic_density(&self) -> f64 {
        let volume = self.unit_cell_volume();
        self.lattice_type.atoms_per_cell() as f64 / volume
    }
}

// ============================================================================
// X-RAY DIFFRACTION
// ============================================================================

/// X-ray diffraction pattern
#[derive(Debug, Clone)]
pub struct XRayDiffraction {
    /// Wavelength (Å)
    pub wavelength: f64,
    /// Crystal structure
    pub crystal: Crystal,
}

impl XRayDiffraction {
    /// Create XRD setup with Cu Kα radiation (1.5406 Å)
    pub fn new_cu_ka(crystal: Crystal) -> Self {
        Self {
            wavelength: 1.5406,  // Cu Kα
            crystal,
        }
    }
    
    /// Create XRD with custom wavelength
    pub fn new(wavelength: f64, crystal: Crystal) -> Self {
        Self { wavelength, crystal }
    }
    
    /// Calculate Bragg angle for reflection (degrees)
    /// Bragg's law: nλ = 2d*sin(θ)
    pub fn bragg_angle(&self, h: i32, k: i32, l: i32, n: i32) -> Option<f64> {
        let d = self.crystal.d_spacing(h, k, l);
        let sin_theta = (n as f64 * self.wavelength) / (2.0 * d);
        
        if sin_theta > 1.0 {
            None  // No diffraction possible
        } else {
            Some(sin_theta.asin().to_degrees())
        }
    }
    
    /// Calculate 2θ angle (degrees)
    pub fn two_theta(&self, h: i32, k: i32, l: i32, n: i32) -> Option<f64> {
        self.bragg_angle(h, k, l, n).map(|theta| 2.0 * theta)
    }
    
    /// Calculate structure factor amplitude (simplified)
    pub fn structure_factor(&self, h: i32, k: i32, l: i32) -> f64 {
        match self.crystal.lattice_type {
            LatticeType::SimpleCubic => {
                // F = f (always diffracts)
                1.0
            }
            LatticeType::BodyCenteredCubic => {
                // F = f(1 + exp(iπ(h+k+l)))
                // Diffracts only when h+k+l is even
                if (h + k + l) % 2 == 0 { 2.0 } else { 0.0 }
            }
            LatticeType::FaceCenteredCubic => {
                // Diffracts only when h, k, l are all even or all odd
                let all_even = h % 2 == 0 && k % 2 == 0 && l % 2 == 0;
                let all_odd = h % 2 != 0 && k % 2 != 0 && l % 2 != 0;
                if all_even || all_odd { 4.0 } else { 0.0 }
            }
            _ => 1.0,
        }
    }
    
    /// Calculate relative intensity (I ∝ |F|²)
    pub fn relative_intensity(&self, h: i32, k: i32, l: i32) -> f64 {
        let f = self.structure_factor(h, k, l);
        f * f
    }
}

// ============================================================================
// MECHANICAL PROPERTIES
// ============================================================================

/// Stress-strain behavior
#[derive(Debug, Clone)]
pub struct MechanicalProperties {
    /// Young's modulus (GPa)
    pub youngs_modulus: f64,
    /// Yield strength (MPa)
    pub yield_strength: f64,
    /// Ultimate tensile strength (MPa)
    pub ultimate_strength: f64,
    /// Grain size (μm)
    pub grain_size: f64,
}

impl MechanicalProperties {
    /// Create mechanical properties
    pub fn new(youngs_modulus: f64, yield_strength: f64, ultimate_strength: f64, grain_size: f64) -> Self {
        Self {
            youngs_modulus,
            yield_strength,
            ultimate_strength,
            grain_size,
        }
    }
    
    /// Calculate stress from strain (Hooke's law)
    /// σ = E*ε (for elastic region)
    pub fn stress_from_strain(&self, strain: f64) -> f64 {
        self.youngs_modulus * 1000.0 * strain  // GPa to MPa
    }
    
    /// Calculate strain from stress
    pub fn strain_from_stress(&self, stress: f64) -> f64 {
        stress / (self.youngs_modulus * 1000.0)
    }
    
    /// Calculate yield strength from grain size (Hall-Petch relation)
    /// σ_y = σ_0 + k/√d
    pub fn hall_petch_yield(&self, sigma_0: f64, k: f64) -> f64 {
        sigma_0 + k / self.grain_size.sqrt()
    }
    
    /// Calculate elastic strain energy density (MJ/m³)
    /// U = (1/2)σε = (1/2)Eε²
    pub fn elastic_energy_density(&self, strain: f64) -> f64 {
        0.5 * self.youngs_modulus * strain * strain
    }
    
    /// Check if stress is in elastic region
    pub fn is_elastic(&self, stress: f64) -> bool {
        stress < self.yield_strength
    }
    
    /// Check if material has failed
    pub fn has_failed(&self, stress: f64) -> bool {
        stress >= self.ultimate_strength
    }
}

// ============================================================================
// ELECTRONIC STRUCTURE
// ============================================================================

/// Electronic band structure
#[derive(Debug, Clone)]
pub struct BandStructure {
    /// Band gap energy (eV)
    pub band_gap: f64,
    /// Fermi energy (eV)
    pub fermi_energy: f64,
    /// Effective electron mass (m_e)
    pub electron_mass: f64,
    /// Effective hole mass (m_e)
    pub hole_mass: f64,
}

impl BandStructure {
    /// Create band structure
    pub fn new(band_gap: f64, fermi_energy: f64, electron_mass: f64, hole_mass: f64) -> Self {
        Self {
            band_gap,
            fermi_energy,
            electron_mass,
            hole_mass,
        }
    }
    
    /// Classify material type
    pub fn material_type(&self) -> &str {
        if self.band_gap == 0.0 {
            "Metal"
        } else if self.band_gap < 2.0 {
            "Semiconductor"
        } else {
            "Insulator"
        }
    }
    
    /// Calculate Fermi-Dirac occupation at energy E and temperature T
    /// f(E) = 1/(1 + exp((E - E_F)/(k_B*T)))
    pub fn fermi_dirac(&self, energy: f64, temperature: f64) -> f64 {
        const K_B: f64 = 8.617333e-5;  // eV/K
        let exponent = (energy - self.fermi_energy) / (K_B * temperature);
        1.0 / (1.0 + exponent.exp())
    }
    
    /// Calculate intrinsic carrier concentration (cm⁻³)
    /// n_i = √(N_c * N_v) * exp(-E_g/(2k_B*T))
    pub fn intrinsic_carrier_density(&self, temperature: f64) -> f64 {
        const K_B: f64 = 8.617333e-5;  // eV/K
        
        // Effective density of states (simplified)
        let n_c = 2.5e19 * (self.electron_mass * temperature / 300.0).powf(1.5);
        let n_v = 2.5e19 * (self.hole_mass * temperature / 300.0).powf(1.5);
        
        (n_c * n_v).sqrt() * (-self.band_gap / (2.0 * K_B * temperature)).exp()
    }
    
    /// Calculate conductivity (S/cm) for intrinsic semiconductor
    pub fn intrinsic_conductivity(&self, temperature: f64) -> f64 {
        const Q: f64 = 1.602e-19;  // Elementary charge (C)
        
        let n_i = self.intrinsic_carrier_density(temperature);
        
        // Mobility (simplified, cm²/V·s)
        let mu_e = 1000.0 / self.electron_mass;
        let mu_h = 400.0 / self.hole_mass;
        
        Q * n_i * (mu_e + mu_h)
    }
}

// ============================================================================
// PHASE DIAGRAMS
// ============================================================================

/// Phase of matter
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
    Plasma,
}

/// Phase transition
#[derive(Debug, Clone)]
pub struct PhaseTransition {
    /// Temperature (K)
    pub temperature: f64,
    /// Pressure (Pa)
    pub pressure: f64,
    /// Initial phase
    pub from_phase: Phase,
    /// Final phase
    pub to_phase: Phase,
    /// Enthalpy change (J/mol)
    pub enthalpy_change: f64,
    /// Volume change (m³/mol)
    pub volume_change: f64,
}

impl PhaseTransition {
    /// Create phase transition
    pub fn new(
        temperature: f64,
        pressure: f64,
        from_phase: Phase,
        to_phase: Phase,
        enthalpy_change: f64,
        volume_change: f64,
    ) -> Self {
        Self {
            temperature,
            pressure,
            from_phase,
            to_phase,
            enthalpy_change,
            volume_change,
        }
    }
    
    /// Calculate slope of phase boundary (Clausius-Clapeyron)
    /// dP/dT = ΔH/(T*ΔV)
    pub fn phase_boundary_slope(&self) -> f64 {
        if self.volume_change.abs() < 1e-20 {
            return 0.0;
        }
        self.enthalpy_change / (self.temperature * self.volume_change)
    }
    
    /// Estimate pressure at different temperature
    pub fn pressure_at_temperature(&self, new_temperature: f64) -> f64 {
        let slope = self.phase_boundary_slope();
        self.pressure + slope * (new_temperature - self.temperature)
    }
    
    /// Check if transition is first-order (discontinuous)
    pub fn is_first_order(&self) -> bool {
        self.enthalpy_change.abs() > 1e-6
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_crystal_lattice() {
        let fcc = Crystal::cubic(LatticeType::FaceCenteredCubic, 3.61);  // Copper
        
        // Volume should be a³
        assert!((fcc.unit_cell_volume() - 47.05).abs() < 0.1);
        
        // FCC has 4 atoms per cell
        assert_eq!(fcc.lattice_type.atoms_per_cell(), 4);
        
        // FCC packing efficiency ~0.74
        assert!((fcc.lattice_type.packing_efficiency() - 0.74).abs() < 0.01);
        
        // d-spacing for (111)
        let d = fcc.d_spacing(1, 1, 1);
        assert!((d - 2.08).abs() < 0.01);
    }
    
    #[test]
    fn test_xrd() {
        let crystal = Crystal::cubic(LatticeType::FaceCenteredCubic, 3.61);
        let xrd = XRayDiffraction::new_cu_ka(crystal);
        
        // (111) reflection should exist for FCC
        let angle = xrd.bragg_angle(1, 1, 1, 1);
        assert!(angle.is_some());
        assert!(angle.unwrap() > 0.0 && angle.unwrap() < 90.0);
        
        // FCC: (100) should have zero intensity
        let intensity_100 = xrd.relative_intensity(1, 0, 0);
        assert_eq!(intensity_100, 0.0);
        
        // FCC: (111) should have non-zero intensity
        let intensity_111 = xrd.relative_intensity(1, 1, 1);
        assert!(intensity_111 > 0.0);
    }
    
    #[test]
    fn test_mechanical_properties() {
        let steel = MechanicalProperties::new(200.0, 250.0, 400.0, 10.0);
        
        // Hooke's law
        let stress = steel.stress_from_strain(0.001);
        assert!((stress - 200.0).abs() < 1.0);
        
        // Should be elastic
        assert!(steel.is_elastic(200.0));
        assert!(!steel.is_elastic(300.0));
        
        // Hall-Petch
        let yield_strength = steel.hall_petch_yield(50.0, 0.5);
        assert!(yield_strength > 50.0);
    }
    
    #[test]
    fn test_band_structure() {
        let silicon = BandStructure::new(1.12, 0.56, 1.08, 0.81);
        
        // Silicon is a semiconductor
        assert_eq!(silicon.material_type(), "Semiconductor");
        
        // Fermi-Dirac at T=0 should be step function
        assert!((silicon.fermi_dirac(0.0, 0.1) - 1.0).abs() < 0.01);
        assert!(silicon.fermi_dirac(1.0, 0.1) < 0.01);
        
        // Carrier density should increase with temperature
        let n_300 = silicon.intrinsic_carrier_density(300.0);
        let n_400 = silicon.intrinsic_carrier_density(400.0);
        assert!(n_400 > n_300);
    }
    
    #[test]
    fn test_phase_transition() {
        // Water melting
        let melting = PhaseTransition::new(
            273.15,
            101325.0,
            Phase::Solid,
            Phase::Liquid,
            6010.0,      // J/mol
            -1.6e-6,     // m³/mol (ice expands)
        );
        
        // Clausius-Clapeyron slope
        let slope = melting.phase_boundary_slope();
        assert!(slope < 0.0);  // Negative slope for water
        
        // Should be first-order transition
        assert!(melting.is_first_order());
        
        // Pressure increases if temperature decreases
        let p_lower = melting.pressure_at_temperature(270.0);
        assert!(p_lower > melting.pressure);
    }
}
