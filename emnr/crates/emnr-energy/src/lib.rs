use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergySystem {
    pub capacity: f32,
    pub current: f32,
    pub recovery_rate: f32,
    pub fatigue: f32,
}

impl Default for EnergySystem {
    fn default() -> Self {
        Self {
            capacity: 100.0,
            current: 100.0,
            recovery_rate: 3.0,
            fatigue: 0.0,
        }
    }
}

impl EnergySystem {
    pub fn spend(&mut self, amount: f32) -> bool {
        if !self.can_spend(amount) {
            self.fatigue = (self.fatigue + 0.05).clamp(0.0, 1.0);
            return false;
        }
        self.current = (self.current - amount).max(0.0);
        self.fatigue = (self.fatigue + amount / self.capacity * 0.02).clamp(0.0, 1.0);
        true
    }

    pub fn recover(&mut self) {
        let recovery = self.recovery_rate * (1.0 - self.fatigue * 0.5);
        self.current = (self.current + recovery).min(self.capacity);
        self.fatigue = (self.fatigue - 0.01).max(0.0);
    }

    pub fn can_spend(&self, amount: f32) -> bool {
        self.current >= amount
    }

    pub fn pressure(&self) -> f32 {
        (1.0 - self.current / self.capacity).clamp(0.0, 1.0)
    }

    pub fn telemetry(&self) -> serde_json::Value {
        json!({
            "capacity": self.capacity,
            "current": self.current,
            "recovery_rate": self.recovery_rate,
            "fatigue": self.fatigue,
            "pressure": self.pressure(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spend_reduces_energy_when_available() {
        let mut energy = EnergySystem::default();
        assert!(energy.spend(25.0));
        assert_eq!(energy.current, 75.0);
        assert!(!energy.spend(100.0));
    }
}
