//! # Matter Relativity Simulation
//!
//! Rigorous simulation of Special and General Relativity.
//! Based on Einstein's field equations and peer-reviewed physics.
//!
//! ## Features
//! - Special Relativity (Lorentz transforms, time dilation, length contraction)
//! - General Relativity (Einstein field equations, curved spacetime)
//! - Schwarzschild metric (black holes)
//! - Kerr metric (rotating black holes)
//! - Gravitational waves (linearized GR)
//! - Geodesics in curved spacetime
//! - Frame dragging (Lense-Thirring effect)
//! - Cosmology (Friedmann equations, expansion)
//!
//! ## Physics Accuracy
//! - Einstein field equations: Gμν + Λgμν = (8πG/c⁴)Tμν
//! - Schwarzschild radius: rs = 2GM/c²
//! - Time dilation: Δt' = γΔt, γ = 1/√(1-v²/c²)
//! - Geodesic equation: d²xμ/dτ² + Γμνρ(dxν/dτ)(dxρ/dτ) = 0

use ndarray::{Array1, Array2};
use std::f64::consts::PI;
use thiserror::Error;

pub mod backend;
pub mod black_holes;
pub mod cosmology;
pub mod geodesics;
pub mod gravitational_waves;

#[derive(Error, Debug)]
pub enum RelativityError {
    #[error("Invalid velocity: {0} (must be < c)")]
    SuperluminalVelocity(f64),

    #[error("Invalid metric: {0}")]
    InvalidMetric(String),

    #[error("Singularity encountered at r = {0}")]
    Singularity(f64),

    #[error("Computation failed: {0}")]
    ComputationFailed(String),
}

pub type Result<T> = std::result::Result<T, RelativityError>;

/// Physical constants (SI units)
pub mod constants {
    /// Speed of light (m/s)
    pub const C: f64 = 299_792_458.0;

    /// Gravitational constant (m³/kg/s²)
    pub const G: f64 = 6.674e-11;

    /// Solar mass (kg)
    pub const M_SUN: f64 = 1.989e30;

    /// Planck length (m)
    pub const L_PLANCK: f64 = 1.616e-35;

    /// Planck time (s)
    pub const T_PLANCK: f64 = 5.391e-44;
}

/// 4-vector in spacetime (t, x, y, z)
pub type FourVector = Array1<f64>;

/// Metric tensor gμν (4×4 matrix)
pub type MetricTensor = Array2<f64>;

/// Stress-energy tensor Tμν (4×4 matrix)
pub type StressEnergyTensor = Array2<f64>;

/// Riemann curvature tensor Rμνρσ (4×4×4×4 tensor)
pub type RiemannTensor = Vec<Vec<Vec<Vec<f64>>>>;

/// Spacetime point (t, x, y, z)
#[derive(Debug, Clone)]
pub struct SpacetimePoint {
    pub t: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl SpacetimePoint {
    pub fn new(t: f64, x: f64, y: f64, z: f64) -> Self {
        Self { t, x, y, z }
    }

    pub fn to_four_vector(&self) -> FourVector {
        Array1::from_vec(vec![self.t, self.x, self.y, self.z])
    }

    /// Calculate proper distance from origin
    pub fn proper_distance(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

/// Special Relativity calculations
pub mod special_relativity {
    use super::*;

    /// Calculate Lorentz factor γ = 1/√(1-v²/c²)
    pub fn lorentz_factor(velocity: f64) -> Result<f64> {
        let c = constants::C;

        if velocity >= c {
            return Err(RelativityError::SuperluminalVelocity(velocity));
        }

        let beta = velocity / c;
        let gamma = 1.0 / (1.0 - beta * beta).sqrt();

        Ok(gamma)
    }

    /// Calculate time dilation: Δt' = γΔt
    pub fn time_dilation(proper_time: f64, velocity: f64) -> Result<f64> {
        let gamma = lorentz_factor(velocity)?;
        Ok(gamma * proper_time)
    }

    /// Calculate length contraction: L' = L/γ
    pub fn length_contraction(proper_length: f64, velocity: f64) -> Result<f64> {
        let gamma = lorentz_factor(velocity)?;
        Ok(proper_length / gamma)
    }

    /// Calculate relativistic momentum: p = γmv
    pub fn relativistic_momentum(mass: f64, velocity: f64) -> Result<f64> {
        let gamma = lorentz_factor(velocity)?;
        Ok(gamma * mass * velocity)
    }

    /// Calculate relativistic energy: E = γmc²
    pub fn relativistic_energy(mass: f64, velocity: f64) -> Result<f64> {
        let gamma = lorentz_factor(velocity)?;
        let c = constants::C;
        Ok(gamma * mass * c * c)
    }

    /// Calculate rest energy: E₀ = mc²
    pub fn rest_energy(mass: f64) -> f64 {
        let c = constants::C;
        mass * c * c
    }

    /// Lorentz transformation matrix (boost in x-direction)
    pub fn lorentz_boost_x(velocity: f64) -> Result<MetricTensor> {
        let gamma = lorentz_factor(velocity)?;
        let beta = velocity / constants::C;

        let mut matrix = Array2::zeros((4, 4));

        // Boost in x-direction
        matrix[[0, 0]] = gamma;
        matrix[[0, 1]] = -gamma * beta;
        matrix[[1, 0]] = -gamma * beta;
        matrix[[1, 1]] = gamma;
        matrix[[2, 2]] = 1.0;
        matrix[[3, 3]] = 1.0;

        Ok(matrix)
    }

    /// Apply Lorentz transformation to 4-vector
    pub fn transform_four_vector(
        four_vector: &FourVector,
        boost: &MetricTensor,
    ) -> FourVector {
        boost.dot(four_vector)
    }
}

/// Schwarzschild metric (non-rotating black hole)
#[derive(Debug, Clone)]
pub struct SchwarzschildMetric {
    /// Black hole mass (kg)
    pub mass: f64,

    /// Schwarzschild radius rs = 2GM/c² (m)
    pub schwarzschild_radius: f64,
}

impl SchwarzschildMetric {
    /// Create Schwarzschild metric for given mass
    pub fn new(mass: f64) -> Self {
        let g = constants::G;
        let c = constants::C;
        let rs = 2.0 * g * mass / (c * c);

        Self {
            mass,
            schwarzschild_radius: rs,
        }
    }

    /// Create metric for solar mass black hole
    pub fn solar_mass() -> Self {
        Self::new(constants::M_SUN)
    }

    /// Get metric tensor at radius r
    /// ds² = -(1-rs/r)c²dt² + (1-rs/r)⁻¹dr² + r²dΩ²
    pub fn metric_tensor(&self, r: f64) -> Result<MetricTensor> {
        if r <= self.schwarzschild_radius {
            return Err(RelativityError::Singularity(r));
        }

        let rs = self.schwarzschild_radius;
        let c = constants::C;

        let mut g = Array2::zeros((4, 4));

        // g_tt = -(1 - rs/r)c²
        g[[0, 0]] = -(1.0 - rs / r) * c * c;

        // g_rr = (1 - rs/r)⁻¹
        g[[1, 1]] = 1.0 / (1.0 - rs / r);

        // g_θθ = r²
        g[[2, 2]] = r * r;

        // g_φφ = r²sin²θ (assuming θ = π/2 for equatorial plane)
        g[[3, 3]] = r * r;

        Ok(g)
    }

    /// Calculate gravitational time dilation at radius r
    /// Δt' = Δt√(1 - rs/r)
    pub fn time_dilation(&self, r: f64, proper_time: f64) -> Result<f64> {
        if r <= self.schwarzschild_radius {
            return Err(RelativityError::Singularity(r));
        }

        let factor = (1.0 - self.schwarzschild_radius / r).sqrt();
        Ok(proper_time / factor)
    }

    /// Calculate escape velocity at radius r
    /// v_esc = c√(rs/r)
    pub fn escape_velocity(&self, r: f64) -> Result<f64> {
        if r <= self.schwarzschild_radius {
            return Err(RelativityError::Singularity(r));
        }

        let c = constants::C;
        let v = c * (self.schwarzschild_radius / r).sqrt();

        Ok(v)
    }

    /// Calculate orbital velocity for circular orbit at radius r
    /// v_orb = c√(rs/2r)
    pub fn orbital_velocity(&self, r: f64) -> Result<f64> {
        if r <= 1.5 * self.schwarzschild_radius {
            return Err(RelativityError::ComputationFailed(
                "No stable circular orbits inside 1.5rs (ISCO)".to_string(),
            ));
        }

        let c = constants::C;
        let v = c * (self.schwarzschild_radius / (2.0 * r)).sqrt();

        Ok(v)
    }

    /// Calculate photon sphere radius (r = 1.5rs)
    pub fn photon_sphere_radius(&self) -> f64 {
        1.5 * self.schwarzschild_radius
    }

    /// Calculate innermost stable circular orbit (ISCO) radius (r = 3rs)
    pub fn isco_radius(&self) -> f64 {
        3.0 * self.schwarzschild_radius
    }

    /// Check if point is inside event horizon
    pub fn is_inside_horizon(&self, r: f64) -> bool {
        r <= self.schwarzschild_radius
    }
}

/// Kerr metric (rotating black hole)
#[derive(Debug, Clone)]
pub struct KerrMetric {
    /// Black hole mass (kg)
    pub mass: f64,

    /// Angular momentum per unit mass: a = J/(Mc)
    pub spin_parameter: f64,

    /// Schwarzschild radius
    pub schwarzschild_radius: f64,
}

impl KerrMetric {
    /// Create Kerr metric
    pub fn new(mass: f64, spin_parameter: f64) -> Result<Self> {
        let g = constants::G;
        let c = constants::C;
        let rs = 2.0 * g * mass / (c * c);

        if spin_parameter.abs() > 1.0 {
            return Err(RelativityError::InvalidMetric(
                "Spin parameter must be |a| ≤ 1 (extremal limit)".to_string(),
            ));
        }

        Ok(Self {
            mass,
            spin_parameter,
            schwarzschild_radius: rs,
        })
    }

    /// Calculate outer event horizon radius
    /// r+ = rs/2 + √((rs/2)² - a²)
    pub fn outer_horizon(&self) -> f64 {
        let rs = self.schwarzschild_radius;
        let a = self.spin_parameter * rs / 2.0;

        rs / 2.0 + ((rs / 2.0).powi(2) - a * a).sqrt()
    }

    /// Calculate inner event horizon radius
    /// r- = rs/2 - √((rs/2)² - a²)
    pub fn inner_horizon(&self) -> f64 {
        let rs = self.schwarzschild_radius;
        let a = self.spin_parameter * rs / 2.0;

        rs / 2.0 - ((rs / 2.0).powi(2) - a * a).sqrt()
    }

    /// Calculate ergosphere radius (at equator)
    /// r_ergo = rs
    pub fn ergosphere_radius(&self) -> f64 {
        self.schwarzschild_radius
    }

    /// Check if extremal (a = 1, maximum rotation)
    pub fn is_extremal(&self) -> bool {
        (self.spin_parameter.abs() - 1.0).abs() < 1e-10
    }
}

/// Geodesic (path of free-falling particle)
#[derive(Debug, Clone)]
pub struct Geodesic {
    /// Spacetime points along geodesic
    pub points: Vec<SpacetimePoint>,

    /// 4-velocity at each point
    pub velocities: Vec<FourVector>,

    /// Proper time parameter
    pub proper_times: Vec<f64>,
}

impl Geodesic {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            velocities: Vec::new(),
            proper_times: Vec::new(),
        }
    }

    /// Add point to geodesic
    pub fn add_point(&mut self, point: SpacetimePoint, velocity: FourVector, tau: f64) {
        self.points.push(point);
        self.velocities.push(velocity);
        self.proper_times.push(tau);
    }

    /// Get total proper time
    pub fn total_proper_time(&self) -> f64 {
        self.proper_times.last().copied().unwrap_or(0.0)
    }
}

impl Default for Geodesic {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate geodesic in Schwarzschild spacetime (simplified)
pub fn schwarzschild_geodesic(
    metric: &SchwarzschildMetric,
    initial_r: f64,
    initial_velocity: f64,
    steps: usize,
    dt: f64,
) -> Result<Geodesic> {
    let mut geodesic = Geodesic::new();

    let mut r = initial_r;
    let mut v_r = initial_velocity;
    let mut t = 0.0;

    for _ in 0..steps {
        // Check if inside horizon
        if r <= metric.schwarzschild_radius {
            break;
        }

        // Add point
        let point = SpacetimePoint::new(t, r, 0.0, 0.0);
        let velocity = Array1::from_vec(vec![1.0, v_r, 0.0, 0.0]);
        geodesic.add_point(point, velocity, t);

        // Simple Euler integration (for demonstration)
        // Real implementation would use Runge-Kutta or symplectic integrator
        let rs = metric.schwarzschild_radius;
        let acceleration = -constants::G * metric.mass / (r * r) * (1.0 - rs / r);

        v_r += acceleration * dt;
        r += v_r * dt;
        t += dt;
    }

    Ok(geodesic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_lorentz_factor() {
        let v = 0.5 * constants::C;
        let gamma = special_relativity::lorentz_factor(v).unwrap();

        // γ = 1/√(1-0.25) = 1/√0.75 ≈ 1.1547
        assert_relative_eq!(gamma, 1.1547, epsilon = 1e-4);
    }

    #[test]
    fn test_time_dilation() {
        let proper_time = 1.0;
        let v = 0.866 * constants::C; // √3/2 * c

        let dilated = special_relativity::time_dilation(proper_time, v).unwrap();

        // γ = 2 for v = √3/2 * c
        assert_relative_eq!(dilated, 2.0, epsilon = 1e-2);
    }

    #[test]
    fn test_schwarzschild_radius() {
        let bh = SchwarzschildMetric::solar_mass();

        // rs = 2GM☉/c² ≈ 2953 m
        assert_relative_eq!(bh.schwarzschild_radius, 2953.0, epsilon = 1.0);
    }

    #[test]
    fn test_escape_velocity() {
        let bh = SchwarzschildMetric::solar_mass();
        let r = 2.0 * bh.schwarzschild_radius;

        let v_esc = bh.escape_velocity(r).unwrap();

        // v_esc = c/√2 at r = 2rs
        assert_relative_eq!(v_esc, constants::C / 2.0_f64.sqrt(), epsilon = 1e-6);
    }

    #[test]
    fn test_kerr_horizons() {
        let kerr = KerrMetric::new(constants::M_SUN, 0.5).unwrap();

        let r_plus = kerr.outer_horizon();
        let r_minus = kerr.inner_horizon();

        assert!(r_plus > r_minus);
        assert!(r_plus < kerr.schwarzschild_radius);
    }

    #[test]
    fn test_photon_sphere() {
        let bh = SchwarzschildMetric::solar_mass();
        let r_photon = bh.photon_sphere_radius();

        assert_relative_eq!(r_photon, 1.5 * bh.schwarzschild_radius, epsilon = 1e-10);
    }

    #[test]
    fn test_isco() {
        let bh = SchwarzschildMetric::solar_mass();
        let r_isco = bh.isco_radius();

        assert_relative_eq!(r_isco, 3.0 * bh.schwarzschild_radius, epsilon = 1e-10);
    }
}
