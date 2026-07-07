//! Superconductivity: BCS theory, Cooper pairs, Josephson junctions, qubits

use std::f64::consts::PI;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum SuperconductorError {
    #[error("Invalid state: {0}")]
    InvalidState(String),
    #[error("Calculation failed: {0}")]
    CalculationFailed(String),
}

pub type Result<T> = std::result::Result<T, SuperconductorError>;

/// BCS superconductor
pub struct BCSSuperconductor {
    pub critical_temperature: f64,
    pub temperature: f64,
    pub coherence_length: f64,
    pub penetration_depth: f64,
}

impl BCSSuperconductor {
    pub fn new(tc: f64, xi: f64, lambda: f64) -> Self {
        Self {
            critical_temperature: tc,
            temperature: 0.0,
            coherence_length: xi,
            penetration_depth: lambda,
        }
    }

    /// Superconducting gap: Δ(T) = Δ₀ tanh(1.74√(Tc/T - 1))
    pub fn gap(&self, temperature: f64) -> f64 {
        if temperature >= self.critical_temperature {
            return 0.0;
        }

        let delta_0 = 1.764 * 8.617e-5 * self.critical_temperature; // eV
        let ratio = self.critical_temperature / temperature - 1.0;
        delta_0 * (1.74 * ratio.sqrt()).tanh()
    }

    /// Cooper pair binding energy
    pub fn cooper_pair_energy(&self) -> f64 {
        2.0 * self.gap(0.0) // 2Δ₀
    }

    /// Critical magnetic field: Hc(T) = Hc(0)[1 - (T/Tc)²]
    pub fn critical_field(&self, temperature: f64) -> f64 {
        if temperature >= self.critical_temperature {
            return 0.0;
        }

        let hc0 = 0.1; // Tesla (typical)
        let t_ratio = temperature / self.critical_temperature;
        hc0 * (1.0 - t_ratio * t_ratio)
    }

    /// Ginzburg-Landau parameter: κ = λ/ξ
    pub fn gl_parameter(&self) -> f64 {
        self.penetration_depth / self.coherence_length
    }

    /// Type I or Type II?
    pub fn superconductor_type(&self) -> &str {
        let kappa = self.gl_parameter();
        if kappa < 1.0 / 2.0_f64.sqrt() {
            "Type I"
        } else {
            "Type II"
        }
    }

    /// Density of Cooper pairs
    pub fn cooper_pair_density(&self, temperature: f64) -> f64 {
        if temperature >= self.critical_temperature {
            return 0.0;
        }

        let n0 = 1e28; // m⁻³ (typical)
        let t_ratio = temperature / self.critical_temperature;
        n0 * (1.0 - t_ratio.powi(4))
    }
}

/// Josephson junction
pub struct JosephsonJunction {
    pub critical_current: f64,
    pub capacitance: f64,
    pub resistance: f64,
    pub phase_difference: f64,
}

impl JosephsonJunction {
    pub fn new(ic: f64, c: f64, r: f64) -> Self {
        Self {
            critical_current: ic,
            capacitance: c,
            resistance: r,
            phase_difference: 0.0,
        }
    }

    /// Josephson current: I = Ic sin(φ)
    pub fn current(&self) -> f64 {
        self.critical_current * self.phase_difference.sin()
    }

    /// Josephson energy: E = -EJ cos(φ), EJ = ℏIc/2e
    pub fn josephson_energy(&self) -> f64 {
        let hbar = 1.055e-34;
        let e = 1.602e-19;
        let ej = hbar * self.critical_current / (2.0 * e);
        -ej * self.phase_difference.cos() / 1.602e-19 // eV
    }

    /// Charging energy: EC = e²/2C
    pub fn charging_energy(&self) -> f64 {
        let e = 1.602e-19;
        let ec = e * e / (2.0 * self.capacitance);
        ec / 1.602e-19 // eV
    }

    /// Plasma frequency: ωp = √(2eIc/ℏC)
    pub fn plasma_frequency(&self) -> f64 {
        let hbar = 1.055e-34;
        let e = 1.602e-19;
        (2.0 * e * self.critical_current / (hbar * self.capacitance)).sqrt()
    }

    /// Set phase difference
    pub fn set_phase(&mut self, phase: f64) {
        self.phase_difference = phase;
    }

    /// Evolve phase (AC Josephson effect)
    pub fn evolve(&mut self, voltage: f64, dt: f64) {
        let hbar = 1.055e-34;
        let e = 1.602e-19;
        let omega = 2.0 * e * voltage / hbar;
        self.phase_difference += omega * dt;
    }
}

/// Superconducting qubit (transmon)
pub struct Transmon {
    pub josephson_energy: f64,
    pub charging_energy: f64,
    pub frequency: f64,
    pub anharmonicity: f64,
}

impl Transmon {
    pub fn new(ej: f64, ec: f64) -> Self {
        let freq = (8.0 * ej * ec).sqrt() - ec;
        let anharm = -ec;

        Self {
            josephson_energy: ej,
            charging_energy: ec,
            frequency: freq,
            anharmonicity: anharm,
        }
    }

    /// Energy level: E_n ≈ ℏω(n + 1/2) + EC n²/2 (anharmonic oscillator)
    pub fn energy_level(&self, n: usize) -> f64 {
        let hbar = 1.055e-34;
        let omega = self.frequency;
        let ec = self.charging_energy * 1.602e-19; // J

        let e_j = hbar * omega * (n as f64 + 0.5) + ec * (n * n) as f64 / 2.0;
        e_j / 1.602e-19 // eV
    }

    /// Qubit frequency (0→1 transition)
    pub fn qubit_frequency(&self) -> f64 {
        self.frequency / (2.0 * PI) // Hz
    }

    /// T1 coherence time (simplified)
    pub fn t1_time(&self) -> f64 {
        100e-6 // 100 μs (typical)
    }

    /// T2 coherence time (simplified)
    pub fn t2_time(&self) -> f64 {
        50e-6 // 50 μs (typical, T2 < 2T1)
    }

    /// Quality factor
    pub fn quality_factor(&self) -> f64 {
        let omega = self.frequency;
        let t1 = self.t1_time();
        omega * t1
    }
}

/// SQUID (Superconducting Quantum Interference Device)
pub struct SQUID {
    pub junction1: JosephsonJunction,
    pub junction2: JosephsonJunction,
    pub loop_inductance: f64,
}

impl SQUID {
    pub fn new(ic1: f64, ic2: f64, l: f64) -> Self {
        Self {
            junction1: JosephsonJunction::new(ic1, 1e-15, 1e3),
            junction2: JosephsonJunction::new(ic2, 1e-15, 1e3),
            loop_inductance: l,
        }
    }

    /// Critical current modulation: Ic(Φ) = 2Ic|cos(πΦ/Φ₀)|
    pub fn critical_current(&self, flux: f64) -> f64 {
        let phi_0 = 2.067e-15; // Flux quantum (Wb)
        let ic_avg = (self.junction1.critical_current + self.junction2.critical_current) / 2.0;
        2.0 * ic_avg * (PI * flux / phi_0).cos().abs()
    }

    /// Flux quantum
    pub fn flux_quantum() -> f64 {
        2.067e-15 // Wb
    }

    /// Magnetic field sensitivity
    pub fn sensitivity(&self) -> f64 {
        let phi_0 = Self::flux_quantum();
        phi_0 / (2.0 * PI) // Wb/rad
    }
}

/// Superconductor simulator
pub struct SuperconductorSimulator {
    pub bcs: Option<BCSSuperconductor>,
    pub junction: Option<JosephsonJunction>,
    pub transmon: Option<Transmon>,
    pub squid: Option<SQUID>,
}

impl SuperconductorSimulator {
    pub fn new() -> Self {
        Self {
            bcs: None,
            junction: None,
            transmon: None,
            squid: None,
        }
    }

    pub fn create_bcs(&mut self, tc: f64, xi: f64, lambda: f64) {
        self.bcs = Some(BCSSuperconductor::new(tc, xi, lambda));
    }

    pub fn create_junction(&mut self, ic: f64, c: f64, r: f64) {
        self.junction = Some(JosephsonJunction::new(ic, c, r));
    }

    pub fn create_transmon(&mut self, ej: f64, ec: f64) {
        self.transmon = Some(Transmon::new(ej, ec));
    }

    pub fn create_squid(&mut self, ic1: f64, ic2: f64, l: f64) {
        self.squid = Some(SQUID::new(ic1, ic2, l));
    }

    pub fn gap(&self, temperature: f64) -> Result<f64> {
        self.bcs
            .as_ref()
            .map(|bcs| bcs.gap(temperature))
            .ok_or_else(|| SuperconductorError::InvalidState("No BCS".to_string()))
    }

    pub fn junction_current(&self) -> Result<f64> {
        self.junction
            .as_ref()
            .map(|j| j.current())
            .ok_or_else(|| SuperconductorError::InvalidState("No junction".to_string()))
    }

    pub fn transmon_frequency(&self) -> Result<f64> {
        self.transmon
            .as_ref()
            .map(|t| t.qubit_frequency())
            .ok_or_else(|| SuperconductorError::InvalidState("No transmon".to_string()))
    }

    pub fn squid_critical_current(&self, flux: f64) -> Result<f64> {
        self.squid
            .as_ref()
            .map(|s| s.critical_current(flux))
            .ok_or_else(|| SuperconductorError::InvalidState("No SQUID".to_string()))
    }
}

impl Default for SuperconductorSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_bcs() {
        let bcs = BCSSuperconductor::new(9.2, 38.0, 39.0);
        let gap_0 = bcs.gap(0.0);
        assert!(gap_0 > 0.0);

        let gap_tc = bcs.gap(9.2);
        assert_relative_eq!(gap_tc, 0.0, epsilon = 1e-6);

        // κ = λ/ξ = 39/38 ≈ 1.03 > 1/√2 ≈ 0.707 → Type II
        assert_eq!(bcs.superconductor_type(), "Type II");
    }

    #[test]
    fn test_josephson() {
        let mut junction = JosephsonJunction::new(1e-6, 1e-15, 1e3);
        junction.set_phase(PI / 2.0);

        let current = junction.current();
        assert_relative_eq!(current, 1e-6, epsilon = 1e-9);
    }

    #[test]
    fn test_transmon() {
        let transmon = Transmon::new(20.0, 0.3);
        let freq = transmon.qubit_frequency();
        assert!(freq > 0.0); // Positive frequency

        let e0 = transmon.energy_level(0);
        let e1 = transmon.energy_level(1);
        assert!(e1 > e0);
    }

    #[test]
    fn test_squid() {
        let squid = SQUID::new(1e-6, 1e-6, 1e-12);
        let ic_0 = squid.critical_current(0.0);
        assert_relative_eq!(ic_0, 2e-6, epsilon = 1e-9);

        let phi_0 = SQUID::flux_quantum();
        assert_relative_eq!(phi_0, 2.067e-15, epsilon = 1e-18);
    }

    #[test]
    fn test_simulator() {
        let mut sim = SuperconductorSimulator::new();

        sim.create_bcs(9.2, 38.0, 39.0);
        assert!(sim.gap(0.0).is_ok());

        sim.create_junction(1e-6, 1e-15, 1e3);
        assert!(sim.junction_current().is_ok());

        sim.create_transmon(20.0, 0.3);
        assert!(sim.transmon_frequency().is_ok());

        sim.create_squid(1e-6, 1e-6, 1e-12);
        assert!(sim.squid_critical_current(0.0).is_ok());
    }
}
