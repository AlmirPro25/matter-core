use crate::{dopamine::DopamineSystem, OrganoidCulture};
use matter_backend::{Backend, Value};

pub struct WetwareBackend {
    culture: OrganoidCulture,
    dopamine: DopamineSystem,
}

impl WetwareBackend {
    pub fn new() -> Self {
        Self {
            culture: OrganoidCulture::new("alpha_1", 10_000),
            dopamine: DopamineSystem::new(),
        }
    }
}

impl Default for WetwareBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for WetwareBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "stimulate" => {
                if args.is_empty() {
                    return Err("wetware.stimulate requires a spike train array".to_string());
                }

                // Extract list from Value::List(Rc<Vec<Value>>)
                let spikes: Vec<bool> = match &args[0] {
                    Value::List(elements) => elements
                        .iter()
                        .map(|v| match v {
                            Value::Bool(b) => *b,
                            Value::Int(n) => *n != 0,
                            _ => false,
                        })
                        .collect(),
                    _ => return Err("wetware.stimulate: expected list of spikes".to_string()),
                };

                let response = self
                    .culture
                    .stimulate(&spikes)
                    .map_err(|e| format!("{:?}", e))?;

                let response_vals: Vec<Value> = response.into_iter().map(Value::Bool).collect();
                Ok(Value::new_list(response_vals))
            }
            "reward" => {
                if args.is_empty() {
                    return Err("wetware.reward requires a float amount".to_string());
                }
                let amount = args[0]
                    .as_float()
                    .map_err(|e| format!("wetware.reward: {}", e))?;
                self.culture.reward(amount as f32);
                self.dopamine.inject_reward(amount as f32);
                Ok(Value::Unit)
            }
            "health" => Ok(Value::Float(self.culture.health as f64)),
            "synapses" => Ok(Value::Int(self.culture.active_synapses as i64)),
            "neurons" => Ok(Value::Int(self.culture.neuron_count as i64)),
            "id" => Ok(Value::new_string(self.culture.id.clone())),
            "tick" => {
                self.dopamine.tick();
                Ok(Value::Unit)
            }
            "dopamine" => Ok(Value::Float(self.dopamine.concentration as f64)),
            "punish" => {
                if args.is_empty() {
                    return Err("wetware.punish requires a float amount".to_string());
                }
                let amount = args[0]
                    .as_float()
                    .map_err(|e| format!("wetware.punish: {}", e))?;
                self.dopamine.inject_punishment(amount as f32);
                self.culture.health = (self.culture.health - (amount as f32 * 0.05)).max(0.0);
                Ok(Value::Unit)
            }
            _ => Err(format!("Unknown wetware method: {}", method)),
        }
    }
}
