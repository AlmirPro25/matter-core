use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TensorSignal {
    pub values: Vec<f32>,
    pub shape: Vec<usize>,
    pub energy: f32,
    pub timestamp: u64,
    pub label: Option<String>,
}

impl TensorSignal {
    pub fn new(values: Vec<f32>) -> Self {
        let len = values.len();
        Self {
            values,
            shape: vec![len],
            energy: 1.0,
            timestamp: 0,
            label: None,
        }
    }

    pub fn zeros(size: usize) -> Self {
        Self::new(vec![0.0; size])
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn normalize(&mut self) {
        let norm = self.values.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > f32::EPSILON {
            for value in &mut self.values {
                *value /= norm;
            }
        }
    }

    pub fn dot(&self, other: &TensorSignal) -> f32 {
        self.values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a * b)
            .sum()
    }

    pub fn combine(&self, other: &TensorSignal) -> TensorSignal {
        let len = self.len().max(other.len());
        let values = (0..len)
            .map(|i| {
                let a = self.values.get(i).copied().unwrap_or_default();
                let b = other.values.get(i).copied().unwrap_or_default();
                (a + b) * 0.5
            })
            .collect::<Vec<_>>();
        TensorSignal {
            shape: vec![len],
            values,
            energy: (self.energy + other.energy) * 0.5,
            timestamp: self.timestamp.max(other.timestamp),
            label: self.label.clone().or_else(|| other.label.clone()),
        }
    }

    pub fn mse(&self, other: &TensorSignal) -> f32 {
        let len = self.len().max(other.len());
        if len == 0 {
            return 0.0;
        }
        let sum = (0..len)
            .map(|i| {
                let a = self.values.get(i).copied().unwrap_or_default();
                let b = other.values.get(i).copied().unwrap_or_default();
                let diff = a - b;
                diff * diff
            })
            .sum::<f32>();
        sum / len as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mse_uses_mean_squared_error() {
        let a = TensorSignal::new(vec![1.0, 2.0, 3.0]);
        let b = TensorSignal::new(vec![1.0, 4.0, 1.0]);
        assert!((a.mse(&b) - 8.0 / 3.0).abs() < 0.0001);
    }

    #[test]
    fn combine_averages_matching_values() {
        let a = TensorSignal::new(vec![0.2, 0.8]);
        let b = TensorSignal::new(vec![0.6, 0.0, 1.0]);
        let combined = a.combine(&b);
        assert_eq!(combined.values, vec![0.4, 0.4, 0.5]);
        assert_eq!(combined.shape, vec![3]);
    }
}
