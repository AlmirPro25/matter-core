//! # Matter Neuromorphic Hardware
//!
//! Hardware integration for neuromorphic computing platforms.
//! Supports Intel Loihi, IBM TrueNorth, SpiNNaker, and custom chips.

use std::collections::HashMap;

// ============================================================================
// HARDWARE ABSTRACTION LAYER
// ============================================================================

/// Neuromorphic hardware platform
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HardwarePlatform {
    IntelLoihi,   // Intel's neuromorphic chip
    IntelLoihi2,  // Intel Loihi 2nd gen
    IBMTrueNorth, // IBM's neuromorphic chip
    SpiNNaker,    // Manchester's SpiNNaker
    BrainScaleS,  // Heidelberg's BrainScaleS
    Custom,       // Custom neuromorphic hardware
}

impl HardwarePlatform {
    pub fn max_neurons(&self) -> usize {
        match self {
            HardwarePlatform::IntelLoihi => 131_072,     // 128K neurons
            HardwarePlatform::IntelLoihi2 => 1_048_576,  // 1M neurons
            HardwarePlatform::IBMTrueNorth => 1_000_000, // 1M neurons
            HardwarePlatform::SpiNNaker => 1_000_000,    // 1M neurons per board
            HardwarePlatform::BrainScaleS => 512_000,    // 512K neurons
            HardwarePlatform::Custom => 1_000_000,       // Configurable
        }
    }

    pub fn max_synapses_per_neuron(&self) -> usize {
        match self {
            HardwarePlatform::IntelLoihi => 4_096,
            HardwarePlatform::IntelLoihi2 => 8_192,
            HardwarePlatform::IBMTrueNorth => 256,
            HardwarePlatform::SpiNNaker => 1_000,
            HardwarePlatform::BrainScaleS => 10_000,
            HardwarePlatform::Custom => 4_096,
        }
    }

    pub fn power_per_neuron_mw(&self) -> f64 {
        match self {
            HardwarePlatform::IntelLoihi => 0.001,     // 1 µW
            HardwarePlatform::IntelLoihi2 => 0.0005,   // 0.5 µW
            HardwarePlatform::IBMTrueNorth => 0.00007, // 70 nW
            HardwarePlatform::SpiNNaker => 0.01,       // 10 µW
            HardwarePlatform::BrainScaleS => 0.001,    // 1 µW
            HardwarePlatform::Custom => 0.001,
        }
    }

    pub fn supports_stdp(&self) -> bool {
        match self {
            HardwarePlatform::IntelLoihi | HardwarePlatform::IntelLoihi2 => true,
            HardwarePlatform::IBMTrueNorth => false,
            HardwarePlatform::SpiNNaker => true,
            HardwarePlatform::BrainScaleS => true,
            HardwarePlatform::Custom => true,
        }
    }
}

/// Hardware configuration
#[derive(Debug, Clone)]
pub struct HardwareConfig {
    pub platform: HardwarePlatform,
    pub num_cores: usize,
    pub timestep_us: f64,
    pub voltage_mv: f64,
    pub temperature_c: f64,
}

impl HardwareConfig {
    pub fn new(platform: HardwarePlatform) -> Self {
        Self {
            platform,
            num_cores: match platform {
                HardwarePlatform::IntelLoihi => 128,
                HardwarePlatform::IntelLoihi2 => 128,
                HardwarePlatform::IBMTrueNorth => 4096,
                HardwarePlatform::SpiNNaker => 48,
                HardwarePlatform::BrainScaleS => 352,
                HardwarePlatform::Custom => 64,
            },
            timestep_us: 1.0,
            voltage_mv: 1000.0,
            temperature_c: 25.0,
        }
    }

    pub fn total_neurons(&self) -> usize {
        self.platform.max_neurons() * self.num_cores
    }

    pub fn total_power_mw(&self) -> f64 {
        self.platform.power_per_neuron_mw() * self.total_neurons() as f64
    }
}

// ============================================================================
// INTEL LOIHI INTEGRATION
// ============================================================================

/// Intel Loihi neuron model
#[derive(Debug, Clone)]
pub struct LoihiNeuron {
    pub id: usize,
    pub core_id: usize,
    pub voltage: i32,
    pub threshold: i32,
    pub decay: u8,
    pub refractory_period: u8,
    pub current_refractory: u8,
}

impl LoihiNeuron {
    pub fn new(id: usize, core_id: usize) -> Self {
        Self {
            id,
            core_id,
            voltage: 0,
            threshold: 100,
            decay: 4,
            refractory_period: 2,
            current_refractory: 0,
        }
    }

    pub fn update(&mut self, input_current: i32) -> bool {
        if self.current_refractory > 0 {
            self.current_refractory -= 1;
            return false;
        }

        // Voltage decay
        self.voltage = self.voltage - (self.voltage >> self.decay);

        // Add input
        self.voltage += input_current;

        // Check threshold
        if self.voltage >= self.threshold {
            self.voltage = 0;
            self.current_refractory = self.refractory_period;
            true
        } else {
            false
        }
    }
}

/// Intel Loihi synapse
#[derive(Debug, Clone)]
pub struct LoihiSynapse {
    pub source_id: usize,
    pub target_id: usize,
    pub weight: i8,
    pub delay: u8,
}

impl LoihiSynapse {
    pub fn new(source_id: usize, target_id: usize, weight: i8, delay: u8) -> Self {
        Self {
            source_id,
            target_id,
            weight,
            delay,
        }
    }
}

/// Intel Loihi network
pub struct LoihiNetwork {
    pub config: HardwareConfig,
    pub neurons: Vec<LoihiNeuron>,
    pub synapses: Vec<LoihiSynapse>,
    pub spike_buffer: Vec<Vec<usize>>,
    pub time: usize,
}

impl LoihiNetwork {
    pub fn new(config: HardwareConfig) -> Self {
        Self {
            config,
            neurons: Vec::new(),
            synapses: Vec::new(),
            spike_buffer: vec![Vec::new(); 16],
            time: 0,
        }
    }

    pub fn add_neuron(&mut self, core_id: usize) -> usize {
        let id = self.neurons.len();
        self.neurons.push(LoihiNeuron::new(id, core_id));
        id
    }

    pub fn add_synapse(&mut self, source_id: usize, target_id: usize, weight: i8, delay: u8) {
        self.synapses
            .push(LoihiSynapse::new(source_id, target_id, weight, delay));
    }

    pub fn step(&mut self, input_spikes: &[usize]) -> Vec<usize> {
        let mut output_spikes = Vec::new();

        // Process delayed spikes
        let buffer_idx = self.time % self.spike_buffer.len();
        let delayed_spikes = std::mem::take(&mut self.spike_buffer[buffer_idx]);

        // Combine input and delayed spikes
        let mut all_spikes: Vec<usize> = input_spikes.to_vec();
        all_spikes.extend(delayed_spikes);

        // Calculate input currents
        let mut input_currents: HashMap<usize, i32> = HashMap::new();
        for spike_id in &all_spikes {
            for synapse in &self.synapses {
                if synapse.source_id == *spike_id {
                    if synapse.delay == 0 {
                        *input_currents.entry(synapse.target_id).or_insert(0) +=
                            synapse.weight as i32;
                    } else {
                        let delay_idx =
                            (self.time + synapse.delay as usize) % self.spike_buffer.len();
                        self.spike_buffer[delay_idx].push(*spike_id);
                    }
                }
            }
        }

        // Update neurons
        for neuron in &mut self.neurons {
            let current = input_currents.get(&neuron.id).copied().unwrap_or(0);
            if neuron.update(current) {
                output_spikes.push(neuron.id);
            }
        }

        self.time += 1;
        output_spikes
    }

    pub fn power_consumption_mw(&self) -> f64 {
        self.config.platform.power_per_neuron_mw() * self.neurons.len() as f64
    }
}

// ============================================================================
// IBM TRUENORTH INTEGRATION
// ============================================================================

/// IBM TrueNorth neuron
#[derive(Debug, Clone)]
pub struct TrueNorthNeuron {
    pub id: usize,
    pub core_id: usize,
    pub membrane_potential: i32,
    pub threshold: i32,
    pub leak: i32,
    pub reset: i32,
}

impl TrueNorthNeuron {
    pub fn new(id: usize, core_id: usize) -> Self {
        Self {
            id,
            core_id,
            membrane_potential: 0,
            threshold: 100,
            leak: 1,
            reset: 0,
        }
    }

    pub fn update(&mut self, input: i32) -> bool {
        // Leak
        self.membrane_potential -= self.leak;
        if self.membrane_potential < 0 {
            self.membrane_potential = 0;
        }

        // Add input
        self.membrane_potential += input;

        // Check threshold
        if self.membrane_potential >= self.threshold {
            self.membrane_potential = self.reset;
            true
        } else {
            false
        }
    }
}

/// IBM TrueNorth network
pub struct TrueNorthNetwork {
    pub config: HardwareConfig,
    pub neurons: Vec<TrueNorthNeuron>,
    pub connectivity: Vec<Vec<(usize, i32)>>,
    pub time: usize,
}

impl TrueNorthNetwork {
    pub fn new(config: HardwareConfig) -> Self {
        Self {
            config,
            neurons: Vec::new(),
            connectivity: Vec::new(),
            time: 0,
        }
    }

    pub fn add_neuron(&mut self, core_id: usize) -> usize {
        let id = self.neurons.len();
        self.neurons.push(TrueNorthNeuron::new(id, core_id));
        self.connectivity.push(Vec::new());
        id
    }

    pub fn connect(&mut self, source_id: usize, target_id: usize, weight: i32) {
        if source_id < self.connectivity.len() {
            self.connectivity[source_id].push((target_id, weight));
        }
    }

    pub fn step(&mut self, input_spikes: &[usize]) -> Vec<usize> {
        let mut output_spikes = Vec::new();
        let mut inputs: HashMap<usize, i32> = HashMap::new();

        // Propagate spikes
        for &spike_id in input_spikes {
            if spike_id < self.connectivity.len() {
                for &(target_id, weight) in &self.connectivity[spike_id] {
                    *inputs.entry(target_id).or_insert(0) += weight;
                }
            }
        }

        // Update neurons
        for neuron in &mut self.neurons {
            let input = inputs.get(&neuron.id).copied().unwrap_or(0);
            if neuron.update(input) {
                output_spikes.push(neuron.id);
            }
        }

        self.time += 1;
        output_spikes
    }

    pub fn power_consumption_mw(&self) -> f64 {
        self.config.platform.power_per_neuron_mw() * self.neurons.len() as f64
    }
}

// ============================================================================
// SPINNAKER INTEGRATION
// ============================================================================

/// SpiNNaker neuron (PyNN compatible)
#[derive(Debug, Clone)]
pub struct SpiNNakerNeuron {
    pub id: usize,
    pub board_id: usize,
    pub v_mem: f64,
    pub v_rest: f64,
    pub v_thresh: f64,
    pub v_reset: f64,
    pub tau_m: f64,
    pub tau_refrac: f64,
    pub refrac_timer: f64,
}

impl SpiNNakerNeuron {
    pub fn new(id: usize, board_id: usize) -> Self {
        Self {
            id,
            board_id,
            v_mem: -65.0,
            v_rest: -65.0,
            v_thresh: -50.0,
            v_reset: -65.0,
            tau_m: 20.0,
            tau_refrac: 2.0,
            refrac_timer: 0.0,
        }
    }

    pub fn update(&mut self, dt: f64, i_syn: f64) -> bool {
        if self.refrac_timer > 0.0 {
            self.refrac_timer -= dt;
            return false;
        }

        // Leaky integrate
        let dv = (-(self.v_mem - self.v_rest) + i_syn) / self.tau_m;
        self.v_mem += dv * dt;

        // Check threshold
        if self.v_mem >= self.v_thresh {
            self.v_mem = self.v_reset;
            self.refrac_timer = self.tau_refrac;
            true
        } else {
            false
        }
    }
}

/// SpiNNaker network
pub struct SpiNNakerNetwork {
    pub config: HardwareConfig,
    pub neurons: Vec<SpiNNakerNeuron>,
    pub weights: Vec<Vec<(usize, f64)>>,
    pub time: f64,
}

impl SpiNNakerNetwork {
    pub fn new(config: HardwareConfig) -> Self {
        Self {
            config,
            neurons: Vec::new(),
            weights: Vec::new(),
            time: 0.0,
        }
    }

    pub fn add_neuron(&mut self, board_id: usize) -> usize {
        let id = self.neurons.len();
        self.neurons.push(SpiNNakerNeuron::new(id, board_id));
        self.weights.push(Vec::new());
        id
    }

    pub fn connect(&mut self, source_id: usize, target_id: usize, weight: f64) {
        if source_id < self.weights.len() {
            self.weights[source_id].push((target_id, weight));
        }
    }

    pub fn step(&mut self, dt: f64, input_spikes: &[usize]) -> Vec<usize> {
        let mut output_spikes = Vec::new();
        let mut currents: HashMap<usize, f64> = HashMap::new();

        // Calculate synaptic currents
        for &spike_id in input_spikes {
            if spike_id < self.weights.len() {
                for &(target_id, weight) in &self.weights[spike_id] {
                    *currents.entry(target_id).or_insert(0.0) += weight;
                }
            }
        }

        // Update neurons
        for neuron in &mut self.neurons {
            let current = currents.get(&neuron.id).copied().unwrap_or(0.0);
            if neuron.update(dt, current) {
                output_spikes.push(neuron.id);
            }
        }

        self.time += dt;
        output_spikes
    }

    pub fn power_consumption_mw(&self) -> f64 {
        self.config.platform.power_per_neuron_mw() * self.neurons.len() as f64
    }
}

// ============================================================================
// UNIFIED HARDWARE INTERFACE
// ============================================================================

/// Unified neuromorphic hardware interface
pub enum NeuromorphicHardware {
    Loihi(LoihiNetwork),
    TrueNorth(TrueNorthNetwork),
    SpiNNaker(SpiNNakerNetwork),
}

impl NeuromorphicHardware {
    pub fn new_loihi(num_cores: usize) -> Self {
        let mut config = HardwareConfig::new(HardwarePlatform::IntelLoihi);
        config.num_cores = num_cores;
        NeuromorphicHardware::Loihi(LoihiNetwork::new(config))
    }

    pub fn new_truenorth(num_cores: usize) -> Self {
        let mut config = HardwareConfig::new(HardwarePlatform::IBMTrueNorth);
        config.num_cores = num_cores;
        NeuromorphicHardware::TrueNorth(TrueNorthNetwork::new(config))
    }

    pub fn new_spinnaker(num_boards: usize) -> Self {
        let mut config = HardwareConfig::new(HardwarePlatform::SpiNNaker);
        config.num_cores = num_boards;
        NeuromorphicHardware::SpiNNaker(SpiNNakerNetwork::new(config))
    }

    pub fn add_neuron(&mut self, core_id: usize) -> usize {
        match self {
            NeuromorphicHardware::Loihi(net) => net.add_neuron(core_id),
            NeuromorphicHardware::TrueNorth(net) => net.add_neuron(core_id),
            NeuromorphicHardware::SpiNNaker(net) => net.add_neuron(core_id),
        }
    }

    pub fn step(&mut self, input_spikes: &[usize]) -> Vec<usize> {
        match self {
            NeuromorphicHardware::Loihi(net) => net.step(input_spikes),
            NeuromorphicHardware::TrueNorth(net) => net.step(input_spikes),
            NeuromorphicHardware::SpiNNaker(net) => net.step(0.001, input_spikes),
        }
    }

    pub fn power_consumption_mw(&self) -> f64 {
        match self {
            NeuromorphicHardware::Loihi(net) => net.power_consumption_mw(),
            NeuromorphicHardware::TrueNorth(net) => net.power_consumption_mw(),
            NeuromorphicHardware::SpiNNaker(net) => net.power_consumption_mw(),
        }
    }

    pub fn platform(&self) -> HardwarePlatform {
        match self {
            NeuromorphicHardware::Loihi(net) => net.config.platform,
            NeuromorphicHardware::TrueNorth(net) => net.config.platform,
            NeuromorphicHardware::SpiNNaker(net) => net.config.platform,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loihi_neuron() {
        let mut neuron = LoihiNeuron::new(0, 0);

        // Should not spike with small input
        assert!(!neuron.update(10));

        // Should spike with large input
        assert!(neuron.update(100));

        // Should be in refractory period
        assert!(!neuron.update(100));
    }

    #[test]
    fn test_loihi_network() {
        let config = HardwareConfig::new(HardwarePlatform::IntelLoihi);
        let mut network = LoihiNetwork::new(config);

        let n0 = network.add_neuron(0);
        let n1 = network.add_neuron(0);
        network.add_synapse(n0, n1, 50, 0);

        // Spike neuron 0
        let spikes = network.step(&[n0]);

        // Should propagate to neuron 1
        assert!(spikes.contains(&n1) || network.neurons[n1].voltage > 0);
    }

    #[test]
    fn test_truenorth_neuron() {
        let mut neuron = TrueNorthNeuron::new(0, 0);

        assert!(!neuron.update(10));
        assert!(neuron.update(100));
    }

    #[test]
    fn test_spinnaker_neuron() {
        let mut neuron = SpiNNakerNeuron::new(0, 0);

        assert!(!neuron.update(1.0, 10.0));
        assert!(neuron.update(1.0, 1000.0));
    }

    #[test]
    fn test_hardware_platforms() {
        let loihi = HardwarePlatform::IntelLoihi;
        assert_eq!(loihi.max_neurons(), 131_072);
        assert!(loihi.supports_stdp());

        let truenorth = HardwarePlatform::IBMTrueNorth;
        assert_eq!(truenorth.max_neurons(), 1_000_000);
        assert!(!truenorth.supports_stdp());
    }

    #[test]
    fn test_power_consumption() {
        let config = HardwareConfig::new(HardwarePlatform::IntelLoihi);
        let mut network = LoihiNetwork::new(config);

        for _ in 0..1000 {
            network.add_neuron(0);
        }

        let power = network.power_consumption_mw();
        assert!(power > 0.0);
        assert!(power < 10.0); // Should be very low power
    }
}
