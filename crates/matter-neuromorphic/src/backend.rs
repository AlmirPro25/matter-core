use crate::SpikingNetwork;
use matter_backend::{Backend, Value};
use std::collections::HashMap;

pub struct NeuromorphicBackend {
    network: Option<SpikingNetwork>,
}

impl NeuromorphicBackend {
    pub fn new() -> Self {
        Self { network: None }
    }
}

impl Default for NeuromorphicBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for NeuromorphicBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "status" | "info" | "capabilities" => Ok(self.status()),
            "init" => {
                if args.len() < 2 {
                    return Err("neuromorphic.init: esperado (num_neurons, dt)".to_string());
                }
                let num_neurons = args[0].as_int()? as usize;
                let dt = args[1].as_float()?;
                self.network = Some(SpikingNetwork::new(num_neurons, dt));
                Ok(Value::Unit)
            }
            "add_synapse" => {
                let net = self.network.as_mut().ok_or(
                    "Rede neuromorfica nao inicializada. Chame neuromorphic.init primeiro.",
                )?;
                if args.len() < 3 {
                    return Err(
                        "neuromorphic.add_synapse: esperado (pre_id, post_id, weight)".to_string(),
                    );
                }
                let pre_id = args[0].as_int()? as usize;
                let post_id = args[1].as_int()? as usize;
                let weight = args[2].as_float()?;

                net.add_synapse(pre_id, post_id, weight)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Unit)
            }
            "add_random_synapses" => {
                let net = self.network.as_mut().ok_or(
                    "Rede neuromorfica nao inicializada. Chame neuromorphic.init primeiro.",
                )?;
                if args.len() < 3 {
                    return Err("neuromorphic.add_random_synapses: esperado (num_synapses, min_weight, max_weight)".to_string());
                }
                let num_synapses = args[0].as_int()? as usize;
                let min_w = args[1].as_float()?;
                let max_w = args[2].as_float()?;

                net.add_random_synapses(num_synapses, (min_w, max_w));
                Ok(Value::Unit)
            }
            "step" => {
                let net = self.network.as_mut().ok_or(
                    "Rede neuromorfica nao inicializada. Chame neuromorphic.init primeiro.",
                )?;
                if args.is_empty() {
                    return Err("neuromorphic.step: esperado (input_currents)".to_string());
                }

                let currents = match &args[0] {
                    Value::List(items) => {
                        let mut vec = Vec::with_capacity(items.len());
                        for item in items.iter() {
                            vec.push(item.as_float()?);
                        }
                        vec
                    }
                    other => {
                        return Err(format!(
                            "neuromorphic.step: esperado lista de correntes, obteve {:?}",
                            other
                        ))
                    }
                };

                let spikes = net.step(&currents);
                let result_vals = spikes
                    .into_iter()
                    .map(|s| Value::Int(s.neuron_id as i64))
                    .collect();
                Ok(Value::new_list(result_vals))
            }
            "lif_threshold_probe" => {
                if args.len() < 2 {
                    return Err(
                        "neuromorphic.lif_threshold_probe: esperado (input_current, steps)"
                            .to_string(),
                    );
                }
                let input_current = args[0].as_float()?;
                let steps = args[1].as_int()?;
                if steps <= 0 || steps > 100_000 {
                    return Err(
                        "neuromorphic.lif_threshold_probe: steps must be between 1 and 100000"
                            .to_string(),
                    );
                }
                Ok(lif_threshold_probe(input_current, steps as usize))
            }
            "apply_learning" => {
                let net = self.network.as_mut().ok_or(
                    "Rede neuromorfica nao inicializada. Chame neuromorphic.init primeiro.",
                )?;
                net.apply_learning();
                Ok(Value::Unit)
            }
            "spike_rate" => {
                let net = self.network.as_ref().ok_or(
                    "Rede neuromorfica nao inicializada. Chame neuromorphic.init primeiro.",
                )?;
                if args.len() < 2 {
                    return Err(
                        "neuromorphic.spike_rate: esperado (neuron_id, window_ms)".to_string()
                    );
                }
                let neuron_id = args[0].as_int()? as usize;
                let window_ms = args[1].as_float()?;
                let rate = net.spike_rate(neuron_id, window_ms);
                Ok(Value::Float(rate))
            }
            "reset" => {
                let net = self.network.as_mut().ok_or(
                    "Rede neuromorfica nao inicializada. Chame neuromorphic.init primeiro.",
                )?;
                net.reset();
                Ok(Value::Unit)
            }
            _ => Err(format!("Unknown neuromorphic method: {}", method)),
        }
    }
}

impl NeuromorphicBackend {
    fn status(&self) -> Value {
        let mut map = HashMap::new();
        map.insert(
            "backend".to_string(),
            Value::new_string("neuromorphic".to_string()),
        );
        map.insert(
            "model".to_string(),
            Value::new_string("lif_snn_stdp_simulator".to_string()),
        );
        map.insert(
            "mode".to_string(),
            Value::new_string("functional_simulator".to_string()),
        );
        map.insert("stub".to_string(), Value::Bool(false));
        map.insert("hardware".to_string(), Value::Bool(false));
        map.insert("simulated".to_string(), Value::Bool(true));
        map.insert(
            "initialized".to_string(),
            Value::Bool(self.network.is_some()),
        );
        map.insert(
            "capabilities".to_string(),
            Value::new_list(vec![
                Value::new_string("init".to_string()),
                Value::new_string("add_synapse".to_string()),
                Value::new_string("add_random_synapses".to_string()),
                Value::new_string("step".to_string()),
                Value::new_string("lif_threshold_probe".to_string()),
                Value::new_string("apply_learning".to_string()),
                Value::new_string("spike_rate".to_string()),
                Value::new_string("reset".to_string()),
            ]),
        );
        Value::new_map(map)
    }
}

fn lif_threshold_probe(input_current: f64, steps: usize) -> Value {
    let dt = 0.1;
    let mut net = SpikingNetwork::new(1, dt);
    let mut first_spike_step: Option<usize> = None;
    let mut spike_count = 0usize;

    for step in 0..steps {
        let spikes = net.step(&[input_current]);
        if !spikes.is_empty() {
            spike_count += spikes.len();
            if first_spike_step.is_none() {
                first_spike_step = Some(step);
            }
        }
    }

    let spiked = first_spike_step.is_some();
    let latency_ms = first_spike_step
        .map(|step| step as f64 * dt)
        .unwrap_or(-1.0);
    let duration_ms = steps as f64 * dt;
    let spike_rate_hz = if duration_ms > 0.0 {
        (spike_count as f64 / duration_ms) * 1000.0
    } else {
        0.0
    };
    let passed = input_current >= 15.0 && spiked && latency_ms >= 0.0 && latency_ms <= duration_ms;

    let mut map = HashMap::new();
    map.insert("input_current".to_string(), Value::Float(input_current));
    map.insert("steps".to_string(), Value::Int(steps as i64));
    map.insert("dt_ms".to_string(), Value::Float(dt));
    map.insert("duration_ms".to_string(), Value::Float(duration_ms));
    map.insert("spiked".to_string(), Value::Bool(spiked));
    map.insert("spike_count".to_string(), Value::Int(spike_count as i64));
    map.insert("latency_ms".to_string(), Value::Float(latency_ms));
    map.insert("spike_rate_hz".to_string(), Value::Float(spike_rate_hz));
    map.insert("passed".to_string(), Value::Bool(passed));
    map.insert(
        "model".to_string(),
        Value::new_string("lif_single_neuron_threshold_probe".to_string()),
    );
    Value::new_map(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lif_threshold_probe_spikes_under_strong_current() {
        let result = lif_threshold_probe(20.0, 200);
        let Value::Map(map) = result else {
            panic!("lif_threshold_probe should return a map");
        };

        assert_eq!(map.get("spiked"), Some(&Value::Bool(true)));
        assert_eq!(map.get("passed"), Some(&Value::Bool(true)));
        assert!(map.get("spike_count").unwrap().as_int().unwrap() > 0);
        assert!(map.get("latency_ms").unwrap().as_float().unwrap() >= 0.0);
    }

    #[test]
    fn backend_exposes_lif_threshold_probe() {
        let mut backend = NeuromorphicBackend::new();
        let result = backend
            .call(
                "lif_threshold_probe",
                vec![Value::Float(20.0), Value::Int(200)],
            )
            .unwrap();
        let Value::Map(map) = result else {
            panic!("neuromorphic.lif_threshold_probe should return a map");
        };
        assert_eq!(map.get("passed"), Some(&Value::Bool(true)));
    }
}
