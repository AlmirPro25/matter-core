//! # Matter Universe Simulation
//!
//! Rigorous cosmological and N-body simulations.
//! Based on Friedmann equations, ΛCDM model, and Newtonian/relativistic gravity.
//!
//! ## Features
//! - N-body gravity simulation (Barnes-Hut algorithm)
//! - Cosmological expansion (Friedmann equations)
//! - Dark matter and dark energy (ΛCDM model)
//! - Galaxy formation and evolution
//! - Star formation and stellar evolution
//! - Planetary dynamics
//! - Big Bang initial conditions
//! - Hubble expansion
//!
//! ## Physics Accuracy
//! - Friedmann equations: H² = (8πG/3)ρ - k/a² + Λ/3
//! - Hubble parameter: H₀ ≈ 70 km/s/Mpc
//! - Dark energy: ΩΛ ≈ 0.68
//! - Dark matter: ΩDM ≈ 0.27
//! - Baryonic matter: ΩB ≈ 0.05
//! - Newton's law: F = Gm₁m₂/r²

use ndarray::Array1;
use rand::Rng;
use std::f64::consts::PI;
use thiserror::Error;

pub mod backend;
pub mod cosmology;
pub mod galaxy;
pub mod nbody;
pub mod stars;

#[derive(Error, Debug)]
pub enum UniverseError {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Simulation diverged: {0}")]
    Divergence(String),

    #[error("Computation failed: {0}")]
    ComputationFailed(String),
}

pub type Result<T> = std::result::Result<T, UniverseError>;

/// Physical constants (SI units)
pub mod constants {
    /// Gravitational constant (m³/kg/s²)
    pub const G: f64 = 6.674e-11;

    /// Speed of light (m/s)
    pub const C: f64 = 299_792_458.0;

    /// Hubble constant (km/s/Mpc)
    pub const H0: f64 = 70.0;

    /// Hubble constant (1/s)
    pub const H0_SI: f64 = 2.268e-18; // 70 km/s/Mpc in SI

    /// Solar mass (kg)
    pub const M_SUN: f64 = 1.989e30;

    /// Parsec (m)
    pub const PARSEC: f64 = 3.086e16;

    /// Megaparsec (m)
    pub const MPC: f64 = 3.086e22;

    /// Age of universe (s)
    pub const T_UNIVERSE: f64 = 4.35e17; // ~13.8 billion years

    /// Critical density (kg/m³)
    pub const RHO_CRIT: f64 = 9.47e-27;
}

/// ΛCDM cosmological parameters
#[derive(Debug, Clone)]
pub struct CosmologicalParameters {
    /// Hubble constant H₀ (km/s/Mpc)
    pub h0: f64,

    /// Dark energy density parameter ΩΛ
    pub omega_lambda: f64,

    /// Dark matter density parameter ΩDM
    pub omega_dark_matter: f64,

    /// Baryonic matter density parameter ΩB
    pub omega_baryonic: f64,

    /// Curvature parameter Ωk
    pub omega_curvature: f64,

    /// Cosmological constant Λ (1/m²)
    pub lambda: f64,
}

impl CosmologicalParameters {
    /// Standard ΛCDM model (Planck 2018)
    pub fn planck_2018() -> Self {
        Self {
            h0: 67.4,
            omega_lambda: 0.6847,
            omega_dark_matter: 0.2589,
            omega_baryonic: 0.0486,
            omega_curvature: 0.0,
            lambda: 1.1056e-52, // Λ = 3H₀²ΩΛ/c²
        }
    }

    /// Total matter density parameter
    pub fn omega_matter(&self) -> f64 {
        self.omega_dark_matter + self.omega_baryonic
    }

    /// Check if universe is flat (Ωtotal = 1)
    pub fn is_flat(&self) -> bool {
        let omega_total = self.omega_lambda + self.omega_matter() + self.omega_curvature;
        (omega_total - 1.0).abs() < 1e-6
    }

    /// Calculate critical density ρc = 3H₀²/(8πG)
    pub fn critical_density(&self) -> f64 {
        let h0_si = self.h0 * 1000.0 / constants::MPC; // Convert to SI
        3.0 * h0_si * h0_si / (8.0 * PI * constants::G)
    }
}

impl Default for CosmologicalParameters {
    fn default() -> Self {
        Self::planck_2018()
    }
}

/// Scale factor a(t) of the universe
#[derive(Debug, Clone)]
pub struct ScaleFactor {
    /// Current scale factor (a₀ = 1 today)
    pub a: f64,

    /// Time derivative da/dt
    pub a_dot: f64,

    /// Cosmic time (s)
    pub time: f64,
}

impl ScaleFactor {
    /// Create scale factor at present day (a = 1)
    pub fn present_day() -> Self {
        Self {
            a: 1.0,
            a_dot: 0.0,
            time: constants::T_UNIVERSE,
        }
    }

    /// Create scale factor at redshift z
    /// a = 1/(1+z)
    pub fn at_redshift(z: f64) -> Self {
        Self {
            a: 1.0 / (1.0 + z),
            a_dot: 0.0,
            time: 0.0,
        }
    }

    /// Calculate redshift z = 1/a - 1
    pub fn redshift(&self) -> f64 {
        1.0 / self.a - 1.0
    }
}

/// Friedmann equations solver
pub struct FriedmannSolver {
    params: CosmologicalParameters,
}

impl FriedmannSolver {
    pub fn new(params: CosmologicalParameters) -> Self {
        Self { params }
    }

    /// Calculate Hubble parameter H(a) = H₀√(ΩM/a³ + ΩΛ + Ωk/a²)
    pub fn hubble_parameter(&self, a: f64) -> f64 {
        let h0_si = self.params.h0 * 1000.0 / constants::MPC;
        let omega_m = self.params.omega_matter();
        let omega_l = self.params.omega_lambda;
        let omega_k = self.params.omega_curvature;

        h0_si * (omega_m / (a * a * a) + omega_l + omega_k / (a * a)).sqrt()
    }

    /// Calculate acceleration equation: ä/a = -4πG(ρ + 3p)/3 + Λ/3
    pub fn acceleration(&self, a: f64) -> f64 {
        let h = self.hubble_parameter(a);
        let omega_m = self.params.omega_matter();
        let omega_l = self.params.omega_lambda;

        // ä/a = H²(-ΩM/(2a³) + ΩΛ)
        h * h * (-omega_m / (2.0 * a * a * a) + omega_l)
    }

    /// Evolve scale factor by timestep dt
    pub fn evolve(&self, scale: &mut ScaleFactor, dt: f64) {
        // Simple Euler integration (for demonstration)
        // Real implementation would use Runge-Kutta
        let h = self.hubble_parameter(scale.a);
        let a_ddot = self.acceleration(scale.a);

        scale.a_dot = h * scale.a;
        scale.a += scale.a_dot * dt;
        scale.a_dot += a_ddot * dt;
        scale.time += dt;
    }

    /// Calculate age of universe at scale factor a
    pub fn age_at_scale(&self, a: f64) -> f64 {
        // Simplified calculation
        // Real implementation would integrate 1/H(a') da' from 0 to a
        let h0_si = self.params.h0 * 1000.0 / constants::MPC;
        1.0 / h0_si * (a / self.params.omega_matter().powf(1.0 / 3.0))
    }
}

/// Particle in N-body simulation
#[derive(Debug, Clone)]
pub struct Particle {
    /// Mass (kg)
    pub mass: f64,

    /// Position (m)
    pub position: Array1<f64>,

    /// Velocity (m/s)
    pub velocity: Array1<f64>,

    /// Acceleration (m/s²)
    pub acceleration: Array1<f64>,

    /// Particle type
    pub particle_type: ParticleType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleType {
    DarkMatter,
    Baryonic,
    Star,
    BlackHole,
}

impl Particle {
    /// Create dark matter particle
    pub fn dark_matter(mass: f64, position: Array1<f64>, velocity: Array1<f64>) -> Self {
        Self {
            mass,
            position,
            velocity,
            acceleration: Array1::zeros(3),
            particle_type: ParticleType::DarkMatter,
        }
    }

    /// Create baryonic matter particle
    pub fn baryonic(mass: f64, position: Array1<f64>, velocity: Array1<f64>) -> Self {
        Self {
            mass,
            position,
            velocity,
            acceleration: Array1::zeros(3),
            particle_type: ParticleType::Baryonic,
        }
    }

    /// Create star
    pub fn star(mass: f64, position: Array1<f64>, velocity: Array1<f64>) -> Self {
        Self {
            mass,
            position,
            velocity,
            acceleration: Array1::zeros(3),
            particle_type: ParticleType::Star,
        }
    }

    /// Calculate kinetic energy
    pub fn kinetic_energy(&self) -> f64 {
        let v_squared: f64 = self.velocity.iter().map(|v| v * v).sum();
        0.5 * self.mass * v_squared
    }
}

/// N-body simulation
pub struct NBodySimulation {
    /// Particles
    pub particles: Vec<Particle>,

    /// Cosmological parameters
    pub cosmology: CosmologicalParameters,

    /// Scale factor
    pub scale_factor: ScaleFactor,

    /// Simulation time (s)
    pub time: f64,

    /// Softening length (m) to avoid singularities
    pub softening: f64,
}

impl NBodySimulation {
    /// Create new N-body simulation
    pub fn new(cosmology: CosmologicalParameters) -> Self {
        Self {
            particles: Vec::new(),
            cosmology,
            scale_factor: ScaleFactor::present_day(),
            time: 0.0,
            softening: 1e20, // ~3 kpc
        }
    }

    /// Add particle
    pub fn add_particle(&mut self, particle: Particle) {
        self.particles.push(particle);
    }

    /// Calculate gravitational force between two particles
    fn gravitational_force(&self, p1: &Particle, p2: &Particle) -> Array1<f64> {
        let r = &p2.position - &p1.position;
        let r_mag = (r.iter().map(|x| x * x).sum::<f64>() + self.softening * self.softening).sqrt();

        // F = G m1 m2 / r²
        let f_mag = constants::G * p1.mass * p2.mass / (r_mag * r_mag);

        // Force direction
        &r * (f_mag / r_mag)
    }

    /// Calculate accelerations for all particles (direct summation O(N²))
    pub fn calculate_accelerations(&mut self) {
        let n = self.particles.len();

        // Reset accelerations
        for particle in &mut self.particles {
            particle.acceleration = Array1::zeros(3);
        }

        // Calculate pairwise forces
        for i in 0..n {
            for j in (i + 1)..n {
                let force = {
                    let p1 = &self.particles[i];
                    let p2 = &self.particles[j];
                    self.gravitational_force(p1, p2)
                };

                // Newton's third law: F_ij = -F_ji
                let m_i = self.particles[i].mass;
                let m_j = self.particles[j].mass;

                self.particles[i].acceleration = &self.particles[i].acceleration + &(&force / m_i);
                self.particles[j].acceleration = &self.particles[j].acceleration - &(&force / m_j);
            }
        }
    }

    /// Evolve simulation by timestep dt (leapfrog integrator)
    pub fn evolve(&mut self, dt: f64) -> Result<()> {
        // Leapfrog integration (symplectic, energy-conserving)

        // Half-step velocity update
        for particle in &mut self.particles {
            particle.velocity = &particle.velocity + &(&particle.acceleration * (dt / 2.0));
        }

        // Full-step position update
        for particle in &mut self.particles {
            particle.position = &particle.position + &(&particle.velocity * dt);
        }

        // Calculate new accelerations
        self.calculate_accelerations();

        // Half-step velocity update
        for particle in &mut self.particles {
            particle.velocity = &particle.velocity + &(&particle.acceleration * (dt / 2.0));
        }

        self.time += dt;

        Ok(())
    }

    /// Calculate total energy (kinetic + potential)
    pub fn total_energy(&self) -> f64 {
        let mut kinetic = 0.0;
        let mut potential = 0.0;

        // Kinetic energy
        for particle in &self.particles {
            kinetic += particle.kinetic_energy();
        }

        // Potential energy
        let n = self.particles.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let r = &self.particles[j].position - &self.particles[i].position;
                let r_mag = (r.iter().map(|x| x * x).sum::<f64>() + self.softening * self.softening).sqrt();

                potential -= constants::G * self.particles[i].mass * self.particles[j].mass / r_mag;
            }
        }

        kinetic + potential
    }

    /// Calculate center of mass
    pub fn center_of_mass(&self) -> Array1<f64> {
        let mut com = Array1::zeros(3);
        let mut total_mass = 0.0;

        for particle in &self.particles {
            com = &com + &(&particle.position * particle.mass);
            total_mass += particle.mass;
        }

        &com / total_mass
    }

    /// Generate random initial conditions (uniform sphere)
    pub fn generate_uniform_sphere(&mut self, n_particles: usize, radius: f64, total_mass: f64) {
        let mut rng = rand::thread_rng();
        let particle_mass = total_mass / n_particles as f64;

        for _ in 0..n_particles {
            // Random position in sphere
            let theta = rng.gen::<f64>() * 2.0 * PI;
            let phi = (rng.gen::<f64>() * 2.0 - 1.0).acos();
            let r = radius * rng.gen::<f64>().cbrt();

            let x = r * phi.sin() * theta.cos();
            let y = r * phi.sin() * theta.sin();
            let z = r * phi.cos();

            let position = Array1::from_vec(vec![x, y, z]);

            // Zero initial velocity (cold start)
            let velocity = Array1::zeros(3);

            let particle = Particle::dark_matter(particle_mass, position, velocity);
            self.add_particle(particle);
        }
    }
}

/// Big Bang initial conditions
pub fn big_bang_initial_conditions() -> NBodySimulation {
    let cosmology = CosmologicalParameters::planck_2018();
    let mut sim = NBodySimulation::new(cosmology);

    // Start at very early universe (a = 0.001, z = 999)
    sim.scale_factor = ScaleFactor::at_redshift(999.0);

    // Generate primordial density fluctuations
    // (simplified - real implementation would use power spectrum)
    sim.generate_uniform_sphere(1000, 1e24, 1e42); // ~1000 galaxy masses

    sim
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_planck_parameters() {
        let params = CosmologicalParameters::planck_2018();
        assert!(params.is_flat());
        assert_relative_eq!(params.omega_lambda, 0.6847, epsilon = 1e-4);
    }

    #[test]
    fn test_hubble_parameter() {
        let params = CosmologicalParameters::planck_2018();
        let solver = FriedmannSolver::new(params);

        let h_today = solver.hubble_parameter(1.0);
        let h0_si = 67.4 * 1000.0 / constants::MPC;

        assert_relative_eq!(h_today, h0_si, epsilon = 1e-10);
    }

    #[test]
    fn test_redshift() {
        let scale = ScaleFactor::at_redshift(1.0);
        assert_relative_eq!(scale.a, 0.5, epsilon = 1e-10);
        assert_relative_eq!(scale.redshift(), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_particle_energy() {
        let position = Array1::from_vec(vec![0.0, 0.0, 0.0]);
        let velocity = Array1::from_vec(vec![1000.0, 0.0, 0.0]);
        let particle = Particle::dark_matter(1e30, position, velocity);

        let ke = particle.kinetic_energy();
        assert_relative_eq!(ke, 0.5 * 1e30 * 1e6, epsilon = 1e-6);
    }

    #[test]
    fn test_nbody_energy_conservation() {
        let mut sim = NBodySimulation::new(CosmologicalParameters::planck_2018());

        // Two-body problem
        let p1 = Particle::dark_matter(
            1e30,
            Array1::from_vec(vec![0.0, 0.0, 0.0]),
            Array1::from_vec(vec![0.0, 0.0, 0.0]),
        );
        let p2 = Particle::dark_matter(
            1e30,
            Array1::from_vec(vec![1e20, 0.0, 0.0]),
            Array1::from_vec(vec![0.0, 1e3, 0.0]),
        );

        sim.add_particle(p1);
        sim.add_particle(p2);

        let e0 = sim.total_energy();

        // Evolve for a few steps
        for _ in 0..10 {
            sim.evolve(1e10).unwrap();
        }

        let e1 = sim.total_energy();

        // Energy should be conserved (within numerical error)
        assert_relative_eq!(e0, e1, epsilon = 1e-2);
    }
}
