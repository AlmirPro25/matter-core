//! Matter Core - Wetware Computing Backend
//!
//! This crate implements the Organoid Intelligence (OI) abstraction
//! for the Matter Core runtime. It provides simulated Microelectrode
//! Arrays (MEAs), Neural Network Protocols, and Dopamine Reward Systems
//! for in-vitro biocomputing.

pub mod backend;
pub mod dopamine;
pub mod mea;
pub mod network;

use matter_error::MatterError;

/// Simulates a brain organoid culture on a Microelectrode Array
pub struct OrganoidCulture {
    pub id: String,
    pub neuron_count: usize,
    pub active_synapses: usize,
    pub health: f32, // 0.0 to 1.0
}

impl OrganoidCulture {
    pub fn new(id: &str, target_neurons: usize) -> Self {
        Self {
            id: id.to_string(),
            neuron_count: target_neurons,
            active_synapses: 0,
            health: 1.0,
        }
    }

    /// Submits a spike train to the organoid via MEA
    #[allow(clippy::result_large_err)]
    pub fn stimulate(&mut self, spikes: &[bool]) -> Result<Vec<bool>, MatterError> {
        if self.health < 0.1 {
            return Err(MatterError::runtime_error("Organoid culture collapsed"));
        }

        // Simulating neural response mapping
        // In a real biological interface, this would transmit voltage
        // and read the action potential from the tissue.
        let mut response = Vec::with_capacity(spikes.len());
        for (i, &spike) in spikes.iter().enumerate() {
            // Simplified echo with slight variation
            response.push(if i % 3 == 0 { !spike } else { spike });
        }

        self.active_synapses += spikes.len();

        Ok(response)
    }

    /// Dispenses simulated dopamine to reward the organoid
    pub fn reward(&mut self, amount: f32) {
        // Increases health and promotes synaptic growth
        self.health = (self.health + (amount * 0.1)).clamp(0.0, 1.0);
        self.active_synapses += (amount * 100.0) as usize;
    }
}
