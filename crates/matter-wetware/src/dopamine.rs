//! Dopamine Reward System
//!
//! Manages simulated chemical reinforcement learning in biological neural networks.

pub struct DopamineSystem {
    pub concentration: f32, // Current chemical concentration in the substrate (0.0 to 1.0)
    pub decay_rate: f32,    // Rate at which dopamine dissipates per tick
}

impl DopamineSystem {
    pub fn new() -> Self {
        Self {
            concentration: 0.0,
            decay_rate: 0.05,
        }
    }

    /// Injects dopamine into the organoid environment to reinforce the previous action
    pub fn inject_reward(&mut self, amount: f32) {
        self.concentration = (self.concentration + amount).clamp(0.0, 1.0);
    }

    /// Punishes the organoid by removing dopamine (simulated glutamate reduction / inhibition)
    pub fn inject_punishment(&mut self, amount: f32) {
        self.concentration = (self.concentration - amount).clamp(0.0, 1.0);
    }

    /// Simulates time passing, allowing chemical dissipation
    pub fn tick(&mut self) {
        self.concentration = (self.concentration - self.decay_rate).max(0.0);
    }
}

impl Default for DopamineSystem {
    fn default() -> Self {
        Self::new()
    }
}
