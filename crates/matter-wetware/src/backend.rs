use crate::{dopamine::DopamineSystem, OrganoidCulture};
use matter_backend::{Backend, Value};
use std::collections::HashMap;

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
            "status" | "info" | "capabilities" => Ok(self.status()),
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
            "bounded_state_probe" => Ok(bounded_state_probe()),
            _ => Err(format!("Unknown wetware method: {}", method)),
        }
    }
}

impl WetwareBackend {
    fn status(&self) -> Value {
        let mut map = HashMap::new();
        map.insert(
            "backend".to_string(),
            Value::new_string("wetware".to_string()),
        );
        map.insert(
            "model".to_string(),
            Value::new_string("simulated_organoid_mea".to_string()),
        );
        map.insert(
            "mode".to_string(),
            Value::new_string("declared_simulation".to_string()),
        );
        map.insert("stub".to_string(), Value::Bool(false));
        map.insert("hardware".to_string(), Value::Bool(false));
        map.insert("simulated".to_string(), Value::Bool(true));
        map.insert(
            "culture_id".to_string(),
            Value::new_string(self.culture.id.clone()),
        );
        map.insert(
            "neurons".to_string(),
            Value::Int(self.culture.neuron_count as i64),
        );
        map.insert(
            "capabilities".to_string(),
            Value::new_list(vec![
                Value::new_string("stimulate".to_string()),
                Value::new_string("reward".to_string()),
                Value::new_string("punish".to_string()),
                Value::new_string("tick".to_string()),
                Value::new_string("health".to_string()),
                Value::new_string("synapses".to_string()),
                Value::new_string("dopamine".to_string()),
                Value::new_string("bounded_state_probe".to_string()),
            ]),
        );
        Value::new_map(map)
    }
}

fn bounded_state_probe() -> Value {
    let mut culture = OrganoidCulture::new("quality_probe", 10_000);
    let mut dopamine = DopamineSystem::new();
    let initial_health = culture.health;
    let initial_dopamine = dopamine.concentration;

    culture.reward(0.5);
    dopamine.inject_reward(0.5);
    let rewarded_health = culture.health;
    let rewarded_dopamine = dopamine.concentration;

    culture.health = (culture.health - (0.25 * 0.05)).max(0.0);
    dopamine.inject_punishment(0.25);
    let punished_health = culture.health;
    let punished_dopamine = dopamine.concentration;

    dopamine.tick();
    let decayed_dopamine = dopamine.concentration;
    let bounded = [
        initial_health,
        initial_dopamine,
        rewarded_health,
        rewarded_dopamine,
        punished_health,
        punished_dopamine,
        decayed_dopamine,
    ]
    .into_iter()
    .all(|value| (0.0..=1.0).contains(&value));
    let passed = bounded
        && rewarded_health >= initial_health
        && rewarded_dopamine > initial_dopamine
        && punished_health <= rewarded_health
        && punished_dopamine <= rewarded_dopamine
        && decayed_dopamine < punished_dopamine;

    let mut map = HashMap::new();
    map.insert(
        "initial_health".to_string(),
        Value::Float(initial_health as f64),
    );
    map.insert(
        "rewarded_health".to_string(),
        Value::Float(rewarded_health as f64),
    );
    map.insert(
        "punished_health".to_string(),
        Value::Float(punished_health as f64),
    );
    map.insert(
        "initial_dopamine".to_string(),
        Value::Float(initial_dopamine as f64),
    );
    map.insert(
        "rewarded_dopamine".to_string(),
        Value::Float(rewarded_dopamine as f64),
    );
    map.insert(
        "punished_dopamine".to_string(),
        Value::Float(punished_dopamine as f64),
    );
    map.insert(
        "decayed_dopamine".to_string(),
        Value::Float(decayed_dopamine as f64),
    );
    map.insert("bounded".to_string(), Value::Bool(bounded));
    map.insert("passed".to_string(), Value::Bool(passed));
    map.insert(
        "model".to_string(),
        Value::new_string("simulated_organoid_bounded_state_probe".to_string()),
    );
    Value::new_map(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_state_probe_reports_bounded_decay() {
        let Value::Map(map) = bounded_state_probe() else {
            panic!("bounded_state_probe should return a map");
        };
        assert_eq!(map.get("bounded"), Some(&Value::Bool(true)));
        assert_eq!(map.get("passed"), Some(&Value::Bool(true)));
        assert!(
            map.get("decayed_dopamine").unwrap().as_float().unwrap()
                < map.get("punished_dopamine").unwrap().as_float().unwrap()
        );
    }

    #[test]
    fn backend_exposes_bounded_state_probe() {
        let mut backend = WetwareBackend::new();
        let Value::Map(map) = backend.call("bounded_state_probe", vec![]).unwrap() else {
            panic!("wetware.bounded_state_probe should return a map");
        };
        assert_eq!(map.get("passed"), Some(&Value::Bool(true)));
    }
}
