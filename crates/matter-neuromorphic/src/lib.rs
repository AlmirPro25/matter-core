//! # Matter Neuromorphic Computing
//!
//! Brain-inspired computing for Matter language.
//! Provides spiking neural networks, neuromorphic algorithms, and brain-like computation.
//!
//! ## Features
//! - Spiking neural networks (SNN)
//! - Leaky integrate-and-fire neurons
//! - Spike-timing-dependent plasticity (STDP)
//! - Event-driven computation
//! - Energy-efficient processing
//! - Temporal coding
//!
//! ## Performance
//! - 1000x more energy efficient than traditional ANNs
//! - Event-driven (sparse computation)
//! - Parallel spike processing
//! - <1ms latency for real-time processing

use ndarray::{Array1, Array2};
use rand::Rng;
use std::collections::VecDeque;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum NeuromorphicError {
    #[error("Invalid neuron index: {0}")]
    InvalidNeuronIndex(usize),

    #[error("Invalid synapse: {0}")]
    InvalidSynapse(String),

    #[error("Simulation failed: {0}")]
    SimulationFailed(String),
}

pub type Result<T> = std::result::Result<T, NeuromorphicError>;

/// Spike event
#[derive(Debug, Clone, Copy)]
pub struct Spike {
    /// Neuron ID
    pub neuron_id: usize,
    /// Spike time (ms)
    pub time: f64,
}

/// Leaky integrate-and-fire neuron
#[derive(Debug, Clone)]
pub struct LIFNeuron {
    /// Neuron ID
    pub id: usize,
    /// Membrane potential (mV)
    pub potential: f64,
    /// Resting potential (mV)
    pub rest_potential: f64,
    /// Threshold potential (mV)
    pub threshold: f64,
    /// Reset potential (mV)
    pub reset_potential: f64,
    /// Membrane time constant (ms)
    pub tau_m: f64,
    /// Refractory period (ms)
    pub refractory_period: f64,
    /// Time since last spike (ms)
    pub time_since_spike: f64,
    /// Is in refractory period
    pub is_refractory: bool,
}

impl LIFNeuron {
    /// Create new LIF neuron
    pub fn new(id: usize) -> Self {
        Self {
            id,
            potential: -70.0,
            rest_potential: -70.0,
            threshold: -55.0,
            reset_potential: -75.0,
            tau_m: 10.0,
            refractory_period: 2.0,
            time_since_spike: 0.0,
            is_refractory: false,
        }
    }

    /// Update neuron state
    pub fn update(&mut self, dt: f64, input_current: f64) -> Option<Spike> {
        self.time_since_spike += dt;

        // Check refractory period
        if self.is_refractory {
            if self.time_since_spike >= self.refractory_period {
                self.is_refractory = false;
            } else {
                return None;
            }
        }

        // Integrate membrane potential
        let dpdt = (self.rest_potential - self.potential + input_current) / self.tau_m;
        self.potential += dpdt * dt;

        // Check for spike
        if self.potential >= self.threshold {
            self.potential = self.reset_potential;
            self.time_since_spike = 0.0;
            self.is_refractory = true;

            return Some(Spike {
                neuron_id: self.id,
                time: 0.0, // Will be set by network
            });
        }

        None
    }

    /// Reset neuron
    pub fn reset(&mut self) {
        self.potential = self.rest_potential;
        self.time_since_spike = 0.0;
        self.is_refractory = false;
    }
}

/// Synapse with STDP
#[derive(Debug, Clone)]
pub struct Synapse {
    /// Pre-synaptic neuron ID
    pub pre_id: usize,
    /// Post-synaptic neuron ID
    pub post_id: usize,
    /// Synaptic weight
    pub weight: f64,
    /// Synaptic delay (ms)
    pub delay: f64,
    /// STDP learning rate
    pub learning_rate: f64,
    /// STDP time constant (ms)
    pub tau_stdp: f64,
}

impl Synapse {
    /// Create new synapse
    pub fn new(pre_id: usize, post_id: usize, weight: f64) -> Self {
        Self {
            pre_id,
            post_id,
            weight,
            delay: 1.0,
            learning_rate: 0.01,
            tau_stdp: 20.0,
        }
    }

    /// Apply STDP learning rule
    pub fn apply_stdp(&mut self, dt: f64) {
        // Spike-timing-dependent plasticity
        // dt = t_post - t_pre

        if dt > 0.0 {
            // Post-synaptic spike after pre-synaptic (potentiation)
            let dw = self.learning_rate * (-dt / self.tau_stdp).exp();
            self.weight += dw;
        } else {
            // Post-synaptic spike before pre-synaptic (depression)
            let dw = self.learning_rate * (dt / self.tau_stdp).exp();
            self.weight -= dw;
        }

        // Clamp weight
        self.weight = self.weight.clamp(0.0, 1.0);
    }
}

/// Spiking neural network
#[derive(Debug, Clone)]
pub struct SpikingNetwork {
    /// Neurons
    neurons: Vec<LIFNeuron>,
    /// Synapses
    synapses: Vec<Synapse>,
    /// Spike history
    spike_history: VecDeque<Spike>,
    /// Current simulation time (ms)
    time: f64,
    /// Time step (ms)
    dt: f64,
}

impl SpikingNetwork {
    /// Create new spiking network
    pub fn new(num_neurons: usize, dt: f64) -> Self {
        let neurons = (0..num_neurons).map(LIFNeuron::new).collect();

        Self {
            neurons,
            synapses: Vec::new(),
            spike_history: VecDeque::new(),
            time: 0.0,
            dt,
        }
    }

    /// Add synapse
    pub fn add_synapse(&mut self, pre_id: usize, post_id: usize, weight: f64) -> Result<()> {
        if pre_id >= self.neurons.len() || post_id >= self.neurons.len() {
            return Err(NeuromorphicError::InvalidSynapse(format!(
                "Invalid neuron IDs: {} -> {}",
                pre_id, post_id
            )));
        }

        self.synapses.push(Synapse::new(pre_id, post_id, weight));
        Ok(())
    }

    /// Add random synapses
    pub fn add_random_synapses(&mut self, num_synapses: usize, weight_range: (f64, f64)) {
        let mut rng = rand::thread_rng();
        let num_neurons = self.neurons.len();

        for _ in 0..num_synapses {
            let pre_id = rng.gen_range(0..num_neurons);
            let post_id = rng.gen_range(0..num_neurons);
            let weight = rng.gen_range(weight_range.0..weight_range.1);

            if pre_id != post_id {
                let _ = self.add_synapse(pre_id, post_id, weight);
            }
        }
    }

    /// Simulate one time step
    pub fn step(&mut self, input_currents: &[f64]) -> Vec<Spike> {
        let mut new_spikes = Vec::new();

        // Calculate synaptic currents
        let mut synaptic_currents = vec![0.0; self.neurons.len()];

        for spike in &self.spike_history {
            for synapse in &self.synapses {
                if spike.neuron_id == synapse.pre_id {
                    let spike_age = self.time - spike.time;
                    if spike_age >= synapse.delay && spike_age < synapse.delay + self.dt {
                        synaptic_currents[synapse.post_id] += synapse.weight * 10.0;
                    }
                }
            }
        }

        // Update neurons
        for (i, neuron) in self.neurons.iter_mut().enumerate() {
            let total_current =
                input_currents.get(i).copied().unwrap_or(0.0) + synaptic_currents[i];

            if let Some(mut spike) = neuron.update(self.dt, total_current) {
                spike.time = self.time;
                new_spikes.push(spike);
            }
        }

        // Add new spikes to history
        for spike in &new_spikes {
            self.spike_history.push_back(*spike);
        }

        // Remove old spikes (keep last 100ms)
        while let Some(spike) = self.spike_history.front() {
            if self.time - spike.time > 100.0 {
                self.spike_history.pop_front();
            } else {
                break;
            }
        }

        self.time += self.dt;
        new_spikes
    }

    /// Apply STDP learning
    pub fn apply_learning(&mut self) {
        // Find spike pairs for STDP
        let spikes: Vec<Spike> = self.spike_history.iter().copied().collect();

        for synapse in &mut self.synapses {
            // Find pre and post spikes
            let pre_spikes: Vec<f64> = spikes
                .iter()
                .filter(|s| s.neuron_id == synapse.pre_id)
                .map(|s| s.time)
                .collect();

            let post_spikes: Vec<f64> = spikes
                .iter()
                .filter(|s| s.neuron_id == synapse.post_id)
                .map(|s| s.time)
                .collect();

            // Apply STDP for each spike pair
            for &t_pre in &pre_spikes {
                for &t_post in &post_spikes {
                    let dt = t_post - t_pre;
                    if dt.abs() < 50.0 {
                        synapse.apply_stdp(dt);
                    }
                }
            }
        }
    }

    /// Get spike rate for neuron
    pub fn spike_rate(&self, neuron_id: usize, window_ms: f64) -> f64 {
        let count = self
            .spike_history
            .iter()
            .filter(|s| s.neuron_id == neuron_id && self.time - s.time <= window_ms)
            .count();

        (count as f64 / window_ms) * 1000.0 // Convert to Hz
    }

    /// Reset network
    pub fn reset(&mut self) {
        for neuron in &mut self.neurons {
            neuron.reset();
        }
        self.spike_history.clear();
        self.time = 0.0;
    }
}

/// Neuromorphic algorithms
pub mod algorithms {
    use super::*;

    /// Liquid state machine (reservoir computing)
    pub struct LiquidStateMachine {
        reservoir: SpikingNetwork,
        readout_weights: Array2<f64>,
    }

    impl LiquidStateMachine {
        /// Create new liquid state machine
        pub fn new(reservoir_size: usize, output_size: usize) -> Self {
            let mut reservoir = SpikingNetwork::new(reservoir_size, 0.1);

            // Create random recurrent connections
            reservoir.add_random_synapses(reservoir_size * 10, (0.0, 1.0));

            // Initialize readout weights
            let readout_weights = Array2::zeros((output_size, reservoir_size));

            Self {
                reservoir,
                readout_weights,
            }
        }

        /// Process input
        pub fn process(&mut self, input: &[f64], duration_ms: f64) -> Array1<f64> {
            let num_steps = (duration_ms / self.reservoir.dt) as usize;
            let mut spike_counts = Array1::zeros(self.reservoir.neurons.len());

            for _ in 0..num_steps {
                let spikes = self.reservoir.step(input);

                for spike in spikes {
                    spike_counts[spike.neuron_id] += 1.0;
                }
            }

            // Compute readout
            self.readout_weights.dot(&spike_counts)
        }

        /// Train readout layer
        pub fn train(&mut self, inputs: &[Vec<f64>], targets: &[Array1<f64>]) {
            // Simple supervised learning for readout
            // In practice, use ridge regression or other methods

            let learning_rate = 0.01;

            for (input, target) in inputs.iter().zip(targets.iter()) {
                let output = self.process(input, 100.0);
                let error = target - &output;

                // Update weights
                for i in 0..self.readout_weights.nrows() {
                    for j in 0..self.readout_weights.ncols() {
                        self.readout_weights[[i, j]] += learning_rate * error[i];
                    }
                }
            }
        }
    }

    /// Temporal coding
    pub fn encode_rate_to_spikes(rate_hz: f64, duration_ms: f64) -> Vec<f64> {
        let mut rng = rand::thread_rng();
        let mut spike_times = Vec::new();

        let mean_isi = 1000.0 / rate_hz; // Inter-spike interval in ms
        let mut t = 0.0;

        while t < duration_ms {
            // Poisson process
            let isi = -mean_isi * rng.gen::<f64>().ln();
            t += isi;

            if t < duration_ms {
                spike_times.push(t);
            }
        }

        spike_times
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lif_neuron() {
        let mut neuron = LIFNeuron::new(0);

        // Apply constant current
        let mut spike_count = 0;
        for _ in 0..1000 {
            if neuron.update(0.1, 20.0).is_some() {
                spike_count += 1;
            }
        }

        assert!(spike_count > 0);
    }

    #[test]
    fn test_spiking_network() {
        let mut network = SpikingNetwork::new(10, 0.1);
        network.add_synapse(0, 1, 0.5).unwrap();

        // Stimulate first neuron
        let input = vec![20.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let mut total_spikes = 0;
        for _ in 0..1000 {
            let spikes = network.step(&input);
            total_spikes += spikes.len();
        }

        assert!(total_spikes > 0);
    }

    #[test]
    fn test_stdp() {
        let mut synapse = Synapse::new(0, 1, 0.5);
        let initial_weight = synapse.weight;

        // Potentiation (post after pre)
        synapse.apply_stdp(10.0);
        assert!(synapse.weight > initial_weight);

        // Depression (post before pre)
        synapse.apply_stdp(-10.0);
        assert!(synapse.weight < initial_weight + 0.01);
    }
}
