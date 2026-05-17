use emnr_core::GlobalField;
use emnr_tensor::TensorSignal;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: u64,
    pub vector: TensorSignal,
    pub emotional_weight: f32,
    pub access_count: u64,
    pub confidence: f32,
    pub created_at: u64,
    pub last_access: u64,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStore {
    pub records: Vec<MemoryRecord>,
    pub max_records: usize,
    next_id: u64,
}

impl MemoryStore {
    pub fn new(max_records: usize) -> Self {
        Self {
            records: Vec::new(),
            max_records,
            next_id: 1,
        }
    }

    pub fn store(&mut self, signal: TensorSignal, emotional_weight: f32, tick: u64) -> u64 {
        if self.records.len() >= self.max_records {
            self.records.sort_by(|a, b| {
                let a_score = a.confidence + a.emotional_weight + a.access_count as f32 * 0.01;
                let b_score = b.confidence + b.emotional_weight + b.access_count as f32 * 0.01;
                a_score.total_cmp(&b_score)
            });
            self.records.remove(0);
        }

        let id = self.next_id;
        self.next_id += 1;
        self.records.push(MemoryRecord {
            id,
            label: signal.label.clone(),
            vector: signal,
            emotional_weight: emotional_weight.clamp(0.0, 1.0),
            access_count: 0,
            confidence: 0.5,
            created_at: tick,
            last_access: tick,
        });
        id
    }

    pub fn retrieve(
        &mut self,
        query: &TensorSignal,
        k: usize,
        field: &GlobalField,
        tick: u64,
    ) -> TensorSignal {
        if self.records.is_empty() || k == 0 {
            return TensorSignal::zeros(query.len());
        }

        let mut scored = self
            .records
            .iter()
            .enumerate()
            .map(|(idx, record)| {
                let similarity = cosine_similarity(query, &record.vector);
                let access_count_bonus = (record.access_count as f32).ln_1p() * 0.05;
                let age_penalty = tick.saturating_sub(record.last_access) as f32 * 0.001;
                let emotional = record.emotional_weight * (1.0 + field.stress * 0.25);
                let score = similarity + emotional + access_count_bonus - age_penalty;
                (idx, score)
            })
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));

        let len = query.len();
        let mut values = vec![0.0; len];
        let mut total_weight = 0.0;

        for (idx, score) in scored.into_iter().take(k) {
            let weight = score.max(0.001);
            total_weight += weight;
            if let Some(record) = self.records.get_mut(idx) {
                record.access_count += 1;
                record.last_access = tick;
                record.confidence = (record.confidence + 0.02).min(1.0);
                for (value_idx, value) in values.iter_mut().enumerate() {
                    *value += record
                        .vector
                        .values
                        .get(value_idx)
                        .copied()
                        .unwrap_or_default()
                        * weight;
                }
            }
        }

        if total_weight > f32::EPSILON {
            for value in &mut values {
                *value /= total_weight;
            }
        }

        TensorSignal {
            values,
            shape: vec![len],
            energy: query.energy,
            timestamp: tick,
            label: Some("retrieved_context".to_string()),
        }
    }

    pub fn reinforce(&mut self, memory_id: u64, amount: f32) {
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.id == memory_id)
        {
            record.emotional_weight = (record.emotional_weight + amount).clamp(0.0, 1.0);
            record.confidence = (record.confidence + amount * 0.5).clamp(0.0, 1.0);
        }
    }

    pub fn decay(&mut self) {
        for record in &mut self.records {
            record.emotional_weight *= 0.995;
            record.confidence *= 0.999;
        }
    }

    pub fn telemetry(&self) -> serde_json::Value {
        let mean_confidence = if self.records.is_empty() {
            0.0
        } else {
            self.records
                .iter()
                .map(|record| record.confidence)
                .sum::<f32>()
                / self.records.len() as f32
        };
        json!({
            "records": self.records.len(),
            "max_records": self.max_records,
            "mean_confidence": mean_confidence,
        })
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new(256)
    }
}

fn cosine_similarity(a: &TensorSignal, b: &TensorSignal) -> f32 {
    let dot = a.dot(b);
    let a_norm = a
        .values
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    let b_norm = b
        .values
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if a_norm <= f32::EPSILON || b_norm <= f32::EPSILON {
        0.0
    } else {
        dot / (a_norm * b_norm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_retrieve_returns_related_context() {
        let mut store = MemoryStore::new(8);
        let signal = TensorSignal::new(vec![1.0, 0.0, 0.0, 0.0]);
        store.store(signal, 0.8, 1);
        let query = TensorSignal::new(vec![0.9, 0.1, 0.0, 0.0]);
        let retrieved = store.retrieve(&query, 1, &GlobalField::default(), 2);
        assert_eq!(retrieved.len(), 4);
        assert!(retrieved.values[0] > retrieved.values[1]);
        assert_eq!(store.records[0].access_count, 1);
    }
}
