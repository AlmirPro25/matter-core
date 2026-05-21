use crate::SpikingNetwork;
use matter_backend::{Backend, Value};

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
