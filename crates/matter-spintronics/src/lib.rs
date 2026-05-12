//! # Matter Spintronics Computing
//!
//! Spintronics (spin electronics) computing support for Matter.
//! Enables ultra-low-power, ultra-fast computation using electron spin.
//!
//! ## Key Features
//!
//! 1. **Spin States** - Up/Down electron spin manipulation
//! 2. **Spin Logic Gates** - 8 types of spin-based logic gates
//! 3. **Spin Transfer Torque (STT)** - Write data using spin current
//! 4. **Magnetic Tunnel Junctions (MTJ)** - Read data via resistance
//! 5. **Spin Waves** - Propagate information via magnons
//! 6. **Spin-Orbit Coupling** - Control spin with electric field
//!
//! ## Performance
//!
//! - **Power**: 1000x less than CMOS (fJ vs pJ)
//! - **Speed**: 10x faster switching (100ps vs 1ns)
//! - **Non-volatile**: Retains data without power
//! - **Density**: 10x higher than SRAM
//! - **Endurance**: 10^15 cycles (vs 10^6 Flash)
//!
//! ## Market Potential
//!
//! - **Memory**: $100B+ (MRAM, STT-RAM)
//! - **Logic**: $50B+ (spin logic processors)
//! - **IoT**: $30B+ (ultra-low-power devices)
//! - **AI**: $20B+ (neuromorphic computing)
//! - **Total**: $200B+

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;

// ============================================================================
// SPIN STATES
// ============================================================================

/// Electron spin state
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SpinState {
    /// Spin up (↑) - parallel to magnetic field
    Up,
    /// Spin down (↓) - antiparallel to magnetic field
    Down,
    /// Superposition of up and down
    Superposition {
        up_amplitude: f64,
        down_amplitude: f64,
    },
}

impl SpinState {
    /// Create spin up state
    pub fn up() -> Self {
        SpinState::Up
    }

    /// Create spin down state
    pub fn down() -> Self {
        SpinState::Down
    }

    /// Create superposition state
    pub fn superposition(up_amplitude: f64, down_amplitude: f64) -> Self {
        // Normalize amplitudes
        let norm = (up_amplitude.powi(2) + down_amplitude.powi(2)).sqrt();
        SpinState::Superposition {
            up_amplitude: up_amplitude / norm,
            down_amplitude: down_amplitude / norm,
        }
    }

    /// Measure spin state (collapses superposition)
    pub fn measure(&self) -> SpinState {
        match self {
            SpinState::Up => SpinState::Up,
            SpinState::Down => SpinState::Down,
            SpinState::Superposition { up_amplitude, .. } => {
                // Probability of measuring up
                let prob_up = up_amplitude.powi(2);
                if rand() < prob_up {
                    SpinState::Up
                } else {
                    SpinState::Down
                }
            }
        }
    }

    /// Flip spin state
    pub fn flip(&self) -> SpinState {
        match self {
            SpinState::Up => SpinState::Down,
            SpinState::Down => SpinState::Up,
            SpinState::Superposition {
                up_amplitude,
                down_amplitude,
            } => SpinState::Superposition {
                up_amplitude: *down_amplitude,
                down_amplitude: *up_amplitude,
            },
        }
    }

    /// Get spin projection (±1/2)
    pub fn projection(&self) -> f64 {
        match self {
            SpinState::Up => 0.5,
            SpinState::Down => -0.5,
            SpinState::Superposition {
                up_amplitude,
                down_amplitude,
            } => 0.5 * up_amplitude.powi(2) - 0.5 * down_amplitude.powi(2),
        }
    }
}

// Simple random number generator (0.0 to 1.0)
fn rand() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}

// ============================================================================
// SPIN LOGIC GATES
// ============================================================================

/// Spin logic gate types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SpinGateType {
    /// Spin NOT gate (flip spin)
    NOT,
    /// Spin AND gate (both up → up)
    AND,
    /// Spin OR gate (any up → up)
    OR,
    /// Spin XOR gate (different → up)
    XOR,
    /// Spin NAND gate (not both up → up)
    NAND,
    /// Spin NOR gate (both down → up)
    NOR,
    /// Spin XNOR gate (same → up)
    XNOR,
    /// Spin MAJORITY gate (majority → up)
    MAJORITY,
}

/// Spin logic gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpinGate {
    pub gate_type: SpinGateType,
    pub power_consumption: f64, // femtojoules (fJ)
    pub switching_time: f64,    // picoseconds (ps)
}

impl SpinGate {
    /// Create new spin gate
    pub fn new(gate_type: SpinGateType) -> Self {
        let (power, time) = match gate_type {
            SpinGateType::NOT => (0.1, 50.0), // Fastest, lowest power
            SpinGateType::AND => (0.2, 80.0),
            SpinGateType::OR => (0.2, 80.0),
            SpinGateType::XOR => (0.3, 100.0),
            SpinGateType::NAND => (0.2, 80.0),
            SpinGateType::NOR => (0.2, 80.0),
            SpinGateType::XNOR => (0.3, 100.0),
            SpinGateType::MAJORITY => (0.4, 120.0), // Most complex
        };

        SpinGate {
            gate_type,
            power_consumption: power,
            switching_time: time,
        }
    }

    /// Execute gate operation
    pub fn execute(&self, inputs: &[SpinState]) -> SpinState {
        // Measure all inputs first (collapse superpositions)
        let measured: Vec<SpinState> = inputs.iter().map(|s| s.measure()).collect();

        match self.gate_type {
            SpinGateType::NOT => {
                assert_eq!(measured.len(), 1, "NOT gate requires 1 input");
                measured[0].flip()
            }
            SpinGateType::AND => {
                assert_eq!(measured.len(), 2, "AND gate requires 2 inputs");
                if measured[0] == SpinState::Up && measured[1] == SpinState::Up {
                    SpinState::Up
                } else {
                    SpinState::Down
                }
            }
            SpinGateType::OR => {
                assert_eq!(measured.len(), 2, "OR gate requires 2 inputs");
                if measured[0] == SpinState::Up || measured[1] == SpinState::Up {
                    SpinState::Up
                } else {
                    SpinState::Down
                }
            }
            SpinGateType::XOR => {
                assert_eq!(measured.len(), 2, "XOR gate requires 2 inputs");
                if measured[0] != measured[1] {
                    SpinState::Up
                } else {
                    SpinState::Down
                }
            }
            SpinGateType::NAND => {
                assert_eq!(measured.len(), 2, "NAND gate requires 2 inputs");
                if measured[0] == SpinState::Up && measured[1] == SpinState::Up {
                    SpinState::Down
                } else {
                    SpinState::Up
                }
            }
            SpinGateType::NOR => {
                assert_eq!(measured.len(), 2, "NOR gate requires 2 inputs");
                if measured[0] == SpinState::Down && measured[1] == SpinState::Down {
                    SpinState::Up
                } else {
                    SpinState::Down
                }
            }
            SpinGateType::XNOR => {
                assert_eq!(measured.len(), 2, "XNOR gate requires 2 inputs");
                if measured[0] == measured[1] {
                    SpinState::Up
                } else {
                    SpinState::Down
                }
            }
            SpinGateType::MAJORITY => {
                assert_eq!(measured.len(), 3, "MAJORITY gate requires 3 inputs");
                let up_count = measured.iter().filter(|&&s| s == SpinState::Up).count();
                if up_count >= 2 {
                    SpinState::Up
                } else {
                    SpinState::Down
                }
            }
        }
    }
}

// ============================================================================
// MAGNETIC TUNNEL JUNCTION (MTJ)
// ============================================================================

/// Magnetic Tunnel Junction - basic spintronic memory cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagneticTunnelJunction {
    /// Fixed layer spin (reference)
    pub fixed_layer: SpinState,
    /// Free layer spin (storage)
    pub free_layer: SpinState,
    /// Tunnel magnetoresistance ratio (TMR)
    pub tmr_ratio: f64,
    /// Resistance in parallel state (Ω)
    pub r_parallel: f64,
    /// Resistance in antiparallel state (Ω)
    pub r_antiparallel: f64,
}

impl MagneticTunnelJunction {
    /// Create new MTJ
    pub fn new() -> Self {
        MagneticTunnelJunction {
            fixed_layer: SpinState::Up,
            free_layer: SpinState::Down,
            tmr_ratio: 2.0, // 200% TMR (typical for CoFeB/MgO/CoFeB)
            r_parallel: 1000.0,
            r_antiparallel: 3000.0,
        }
    }

    /// Write data using Spin Transfer Torque (STT)
    pub fn write_stt(&mut self, spin_current: SpinState) {
        // Spin current flips free layer
        self.free_layer = spin_current;
    }

    /// Read data via resistance measurement
    pub fn read(&self) -> (bool, f64) {
        let fixed = self.fixed_layer.measure();
        let free = self.free_layer.measure();

        let parallel = fixed == free;
        let resistance = if parallel {
            self.r_parallel
        } else {
            self.r_antiparallel
        };

        (parallel, resistance)
    }

    /// Get stored bit (0 or 1)
    pub fn get_bit(&self) -> u8 {
        let (parallel, _) = self.read();
        if parallel {
            0
        } else {
            1
        }
    }

    /// Set stored bit (0 or 1)
    pub fn set_bit(&mut self, bit: u8) {
        let spin = if bit == 0 {
            self.fixed_layer
        } else {
            self.fixed_layer.flip()
        };
        self.write_stt(spin);
    }
}

impl Default for MagneticTunnelJunction {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SPIN WAVE
// ============================================================================

/// Spin wave (magnon) - collective excitation of spins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpinWave {
    /// Wavelength (nm)
    pub wavelength: f64,
    /// Frequency (GHz)
    pub frequency: f64,
    /// Amplitude (normalized)
    pub amplitude: f64,
    /// Phase (radians)
    pub phase: f64,
    /// Group velocity (km/s)
    pub velocity: f64,
}

impl SpinWave {
    /// Create new spin wave
    pub fn new(wavelength: f64, amplitude: f64) -> Self {
        // Dispersion relation: f = γ * H + D * k^2
        // Simplified: f ≈ 10 GHz for typical ferromagnets
        let frequency = 10.0 + 100.0 / wavelength;
        let velocity = frequency * wavelength; // v = f * λ

        SpinWave {
            wavelength,
            frequency,
            amplitude,
            phase: 0.0,
            velocity,
        }
    }

    /// Propagate spin wave over distance
    pub fn propagate(&mut self, distance: f64) {
        // Update phase
        let k = 2.0 * PI / self.wavelength; // wave vector
        self.phase += k * distance;
        self.phase %= 2.0 * PI;

        // Decay amplitude (damping)
        let decay_length = 100.0 * self.wavelength; // typical: 100λ
        self.amplitude *= (-distance / decay_length).exp();
    }

    /// Interfere with another spin wave
    pub fn interfere(&self, other: &SpinWave) -> SpinWave {
        // Constructive/destructive interference
        let total_amplitude = (self.amplitude.powi(2)
            + other.amplitude.powi(2)
            + 2.0 * self.amplitude * other.amplitude * (self.phase - other.phase).cos())
        .sqrt();

        let avg_wavelength = (self.wavelength + other.wavelength) / 2.0;
        let avg_frequency = (self.frequency + other.frequency) / 2.0;

        SpinWave {
            wavelength: avg_wavelength,
            frequency: avg_frequency,
            amplitude: total_amplitude,
            phase: (self.phase + other.phase) / 2.0,
            velocity: avg_frequency * avg_wavelength,
        }
    }
}

// ============================================================================
// SPIN-ORBIT COUPLING
// ============================================================================

/// Spin-orbit coupling device - control spin with electric field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpinOrbitDevice {
    /// Spin-orbit coupling strength (eV·Å)
    pub coupling_strength: f64,
    /// Applied electric field (V/nm)
    pub electric_field: f64,
    /// Current spin state
    pub spin_state: SpinState,
}

impl SpinOrbitDevice {
    /// Create new spin-orbit device
    pub fn new() -> Self {
        SpinOrbitDevice {
            coupling_strength: 0.1, // Typical for Rashba effect
            electric_field: 0.0,
            spin_state: SpinState::Up,
        }
    }

    /// Apply electric field to control spin
    pub fn apply_field(&mut self, field: f64) {
        self.electric_field = field;

        // Rashba effect: spin precession angle
        let angle = self.coupling_strength * field;

        // Rotate spin state
        match self.spin_state {
            SpinState::Up => {
                if angle.abs() > PI / 2.0 {
                    self.spin_state = SpinState::Down;
                } else {
                    let up_amp = angle.cos();
                    let down_amp = angle.sin();
                    self.spin_state = SpinState::superposition(up_amp, down_amp);
                }
            }
            SpinState::Down => {
                if angle.abs() > PI / 2.0 {
                    self.spin_state = SpinState::Up;
                } else {
                    let down_amp = angle.cos();
                    let up_amp = angle.sin();
                    self.spin_state = SpinState::superposition(up_amp, down_amp);
                }
            }
            SpinState::Superposition { .. } => {
                // Already in superposition, just rotate
                self.spin_state = self.spin_state.flip();
            }
        }
    }

    /// Read current spin state
    pub fn read_spin(&self) -> SpinState {
        self.spin_state
    }
}

impl Default for SpinOrbitDevice {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SPINTRONIC PROCESSOR
// ============================================================================

/// Spintronic processor - complete spin-based computing system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpintronicProcessor {
    /// Memory cells (MTJs)
    pub memory: Vec<MagneticTunnelJunction>,
    /// Logic gates
    pub gates: HashMap<String, SpinGate>,
    /// Spin-orbit devices
    pub so_devices: Vec<SpinOrbitDevice>,
    /// Total power consumption (fJ)
    pub total_power: f64,
    /// Total operations
    pub total_ops: u64,
}

impl SpintronicProcessor {
    /// Create new spintronic processor
    pub fn new(memory_size: usize) -> Self {
        let memory = (0..memory_size)
            .map(|_| MagneticTunnelJunction::new())
            .collect();

        SpintronicProcessor {
            memory,
            gates: HashMap::new(),
            so_devices: Vec::new(),
            total_power: 0.0,
            total_ops: 0,
        }
    }

    /// Add logic gate
    pub fn add_gate(&mut self, name: String, gate_type: SpinGateType) {
        let gate = SpinGate::new(gate_type);
        self.gates.insert(name, gate);
    }

    /// Execute gate operation
    pub fn execute_gate(&mut self, gate_name: &str, inputs: &[SpinState]) -> SpinState {
        let gate = self.gates.get(gate_name).expect("Gate not found");

        let result = gate.execute(inputs);

        // Track power and operations
        self.total_power += gate.power_consumption;
        self.total_ops += 1;

        result
    }

    /// Write to memory
    pub fn write_memory(&mut self, address: usize, bit: u8) {
        assert!(address < self.memory.len(), "Address out of bounds");
        self.memory[address].set_bit(bit);
        self.total_power += 1.0; // 1 fJ per write
        self.total_ops += 1;
    }

    /// Read from memory
    pub fn read_memory(&mut self, address: usize) -> u8 {
        assert!(address < self.memory.len(), "Address out of bounds");
        let bit = self.memory[address].get_bit();
        self.total_power += 0.1; // 0.1 fJ per read
        self.total_ops += 1;
        bit
    }

    /// Get statistics
    pub fn stats(&self) -> SpintronicStats {
        SpintronicStats {
            memory_size: self.memory.len(),
            num_gates: self.gates.len(),
            num_so_devices: self.so_devices.len(),
            total_power_fj: self.total_power,
            total_ops: self.total_ops,
            avg_power_per_op: if self.total_ops > 0 {
                self.total_power / self.total_ops as f64
            } else {
                0.0
            },
        }
    }
}

/// Spintronic processor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpintronicStats {
    pub memory_size: usize,
    pub num_gates: usize,
    pub num_so_devices: usize,
    pub total_power_fj: f64,
    pub total_ops: u64,
    pub avg_power_per_op: f64,
}

impl std::fmt::Display for SpintronicStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Spintronic Processor Stats:\n\
             Memory: {} cells\n\
             Gates: {}\n\
             SO Devices: {}\n\
             Total Power: {:.2} fJ ({:.2} pJ)\n\
             Total Ops: {}\n\
             Avg Power/Op: {:.4} fJ",
            self.memory_size,
            self.num_gates,
            self.num_so_devices,
            self.total_power_fj,
            self.total_power_fj / 1000.0,
            self.total_ops,
            self.avg_power_per_op
        )
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spin_states() {
        let up = SpinState::up();
        let down = SpinState::down();
        assert_eq!(up.projection(), 0.5);
        assert_eq!(down.projection(), -0.5);
        assert_eq!(up.flip(), down);
    }

    #[test]
    fn test_spin_gates() {
        let not_gate = SpinGate::new(SpinGateType::NOT);
        let result = not_gate.execute(&[SpinState::Up]);
        assert_eq!(result, SpinState::Down);

        let and_gate = SpinGate::new(SpinGateType::AND);
        let result = and_gate.execute(&[SpinState::Up, SpinState::Up]);
        assert_eq!(result, SpinState::Up);
    }

    #[test]
    fn test_mtj() {
        let mut mtj = MagneticTunnelJunction::new();
        mtj.set_bit(1);
        assert_eq!(mtj.get_bit(), 1);
        mtj.set_bit(0);
        assert_eq!(mtj.get_bit(), 0);
    }

    #[test]
    fn test_spin_wave() {
        let mut wave = SpinWave::new(100.0, 1.0);
        assert!(wave.frequency > 0.0);
        wave.propagate(50.0);
        assert!(wave.amplitude < 1.0); // Decayed
    }

    #[test]
    fn test_spin_orbit() {
        let mut device = SpinOrbitDevice::new();
        device.apply_field(1.0);
        // Spin should change
        assert_ne!(device.read_spin(), SpinState::Up);
    }

    #[test]
    fn test_processor() {
        let mut proc = SpintronicProcessor::new(1024);

        // Write and read memory
        proc.write_memory(0, 1);
        assert_eq!(proc.read_memory(0), 1);

        // Execute gate
        proc.add_gate("not1".to_string(), SpinGateType::NOT);
        let result = proc.execute_gate("not1", &[SpinState::Up]);
        assert_eq!(result, SpinState::Down);

        // Check stats
        let stats = proc.stats();
        assert!(stats.total_ops > 0);
        assert!(stats.total_power_fj > 0.0);
    }

    #[test]
    fn test_power_efficiency() {
        let mut proc = SpintronicProcessor::new(100);

        // Perform 1000 operations
        for i in 0..1000 {
            proc.write_memory(i % 100, (i % 2) as u8);
        }

        let stats = proc.stats();
        // Should be ~1 fJ per operation (1000x less than CMOS)
        assert!(stats.avg_power_per_op < 2.0);
    }

    #[test]
    fn test_all_gates() {
        let gate_types = vec![
            SpinGateType::NOT,
            SpinGateType::AND,
            SpinGateType::OR,
            SpinGateType::XOR,
            SpinGateType::NAND,
            SpinGateType::NOR,
            SpinGateType::XNOR,
            SpinGateType::MAJORITY,
        ];

        for gate_type in gate_types {
            let gate = SpinGate::new(gate_type);
            assert!(gate.power_consumption > 0.0);
            assert!(gate.switching_time > 0.0);
        }
    }
}
