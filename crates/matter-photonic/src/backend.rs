use crate::{OpticalSignal, PhotonicGate, PhotonicGateType, Waveguide, Wavelength};
use matter_backend::{Backend, Value};
use std::collections::HashMap;

pub struct PhotonicBackend {
    waveguide: Waveguide,
}

impl PhotonicBackend {
    pub fn new() -> Self {
        Self {
            waveguide: Waveguide::new(10.0), // 10 meters waveguide
        }
    }
}

impl Default for PhotonicBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for PhotonicBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            "status" | "info" | "capabilities" => Ok(frontier_status(
                "photonic",
                "optical_signal_model",
                "functional_model",
                &[
                    "and",
                    "or",
                    "not",
                    "metrics",
                    "truth_table",
                    "waveguide_loss",
                ],
            )),
            "and" => {
                if args.len() < 2 {
                    return Err("photonic.and: expected 2 arguments (intensity_a: float, intensity_b: float)".to_string());
                }
                let a = args[0].as_float()?;
                let b = args[1].as_float()?;

                let sig_a = OpticalSignal::new(Wavelength::c_band(0), a);
                let sig_b = OpticalSignal::new(Wavelength::c_band(0), b);
                let gate = PhotonicGate::new(PhotonicGateType::MZI);
                let out = gate.and(&sig_a, &sig_b);
                Ok(Value::Float(out.intensity))
            }
            "or" => {
                if args.len() < 2 {
                    return Err("photonic.or: expected 2 arguments (intensity_a: float, intensity_b: float)".to_string());
                }
                let a = args[0].as_float()?;
                let b = args[1].as_float()?;

                let sig_a = OpticalSignal::new(Wavelength::c_band(0), a);
                let sig_b = OpticalSignal::new(Wavelength::c_band(0), b);
                let gate = PhotonicGate::new(PhotonicGateType::Coupler);
                let out = gate.or(&sig_a, &sig_b);
                Ok(Value::Float(out.intensity))
            }
            "not" => {
                if args.is_empty() {
                    return Err(
                        "photonic.not: expected 1 argument (intensity_a: float)".to_string()
                    );
                }
                let a = args[0].as_float()?;

                let sig_a = OpticalSignal::new(Wavelength::c_band(0), a);
                let gate = PhotonicGate::new(PhotonicGateType::SOA);
                let out = gate.not(&sig_a);
                Ok(Value::Float(out.intensity))
            }
            "metrics" => {
                let sig = OpticalSignal::new(Wavelength::c_band(0), 1.0);
                let propagated = self.waveguide.propagate(&sig);
                let attenuation = -10.0 * (propagated.intensity / sig.intensity).log10();

                let mut map = HashMap::new();
                map.insert("attenuation_db".to_string(), Value::Float(attenuation));
                map.insert(
                    "optical_throughput_percent".to_string(),
                    Value::Float(propagated.intensity * 100.0),
                );
                map.insert("efficiency".to_string(), Value::Float(0.9995));
                map.insert("capacity_bps".to_string(), Value::Float(100e9)); // 100 Gbps channel base

                Ok(Value::new_map(map))
            }
            "truth_table" => Ok(truth_table()),
            "waveguide_loss" => {
                if args.len() < 2 {
                    return Err(
                        "photonic.waveguide_loss: expected (length_m, intensity)".to_string()
                    );
                }
                let length_m = args[0].as_float()?;
                let intensity = args[1].as_float()?;
                if length_m < 0.0 {
                    return Err("photonic.waveguide_loss: length_m must be >= 0".to_string());
                }
                if !(0.0..=1.0).contains(&intensity) {
                    return Err(
                        "photonic.waveguide_loss: intensity must be between 0 and 1".to_string()
                    );
                }
                Ok(waveguide_loss(length_m, intensity))
            }
            _ => Err(format!("Unknown photonic method: {}", method)),
        }
    }
}

fn truth_table() -> Value {
    let gate = PhotonicGate::new(PhotonicGateType::MZI);
    let mut rows = Vec::new();
    let mut correct = 0usize;

    for (a, b) in [(false, false), (false, true), (true, false), (true, true)] {
        let sig_a = OpticalSignal::new(Wavelength::c_band(0), if a { 1.0 } else { 0.0 });
        let sig_b = OpticalSignal::new(Wavelength::c_band(0), if b { 1.0 } else { 0.0 });
        let and_value = gate.and(&sig_a, &sig_b).intensity > 0.5;
        let or_value = gate.or(&sig_a, &sig_b).intensity > 0.5;
        let not_a_value = gate.not(&sig_a).intensity > 0.5;
        let passed = and_value == (a && b) && or_value == (a || b) && not_a_value == !a;
        if passed {
            correct += 1;
        }

        let mut row = HashMap::new();
        row.insert("input_a".to_string(), Value::Bool(a));
        row.insert("input_b".to_string(), Value::Bool(b));
        row.insert("and".to_string(), Value::Bool(and_value));
        row.insert("or".to_string(), Value::Bool(or_value));
        row.insert("not_a".to_string(), Value::Bool(not_a_value));
        row.insert("passed".to_string(), Value::Bool(passed));
        rows.push(Value::new_map(row));
    }

    let mut map = HashMap::new();
    map.insert("rows".to_string(), Value::new_list(rows));
    map.insert("row_count".to_string(), Value::Int(4));
    map.insert(
        "truth_table_accuracy".to_string(),
        Value::Float(correct as f64 / 4.0),
    );
    map.insert("passed".to_string(), Value::Bool(correct == 4));
    map.insert(
        "model".to_string(),
        Value::new_string("photonic_threshold_logic_truth_table".to_string()),
    );
    Value::new_map(map)
}

fn waveguide_loss(length_m: f64, intensity: f64) -> Value {
    let waveguide = Waveguide::new(length_m);
    let signal = OpticalSignal::new(Wavelength::c_band(0), intensity);
    let propagated = waveguide.propagate(&signal);
    let attenuation_db = if intensity > 0.0 {
        -10.0 * (propagated.intensity / intensity).log10()
    } else {
        0.0
    };
    let passed = propagated.intensity <= intensity && attenuation_db >= 0.0;

    let mut map = HashMap::new();
    map.insert("length_m".to_string(), Value::Float(length_m));
    map.insert("input_intensity".to_string(), Value::Float(intensity));
    map.insert(
        "output_intensity".to_string(),
        Value::Float(propagated.intensity),
    );
    map.insert("attenuation_db".to_string(), Value::Float(attenuation_db));
    map.insert(
        "throughput_percent".to_string(),
        Value::Float(propagated.intensity * 100.0),
    );
    map.insert("passed".to_string(), Value::Bool(passed));
    map.insert(
        "model".to_string(),
        Value::new_string("simplified_waveguide_loss".to_string()),
    );
    Value::new_map(map)
}

fn frontier_status(backend: &str, model: &str, mode: &str, capabilities: &[&str]) -> Value {
    let mut map = HashMap::new();
    map.insert(
        "backend".to_string(),
        Value::new_string(backend.to_string()),
    );
    map.insert("model".to_string(), Value::new_string(model.to_string()));
    map.insert("mode".to_string(), Value::new_string(mode.to_string()));
    map.insert("stub".to_string(), Value::Bool(false));
    map.insert("hardware".to_string(), Value::Bool(false));
    map.insert("simulated".to_string(), Value::Bool(true));
    map.insert(
        "capabilities".to_string(),
        Value::new_list(
            capabilities
                .iter()
                .map(|capability| Value::new_string((*capability).to_string()))
                .collect(),
        ),
    );
    Value::new_map(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truth_table_matches_threshold_logic() {
        let Value::Map(map) = truth_table() else {
            panic!("truth_table should return a map");
        };
        assert_eq!(map.get("row_count"), Some(&Value::Int(4)));
        assert_eq!(map.get("truth_table_accuracy"), Some(&Value::Float(1.0)));
        assert_eq!(map.get("passed"), Some(&Value::Bool(true)));
    }

    #[test]
    fn waveguide_loss_increases_with_length() {
        let Value::Map(short) = waveguide_loss(1.0, 1.0) else {
            panic!("waveguide_loss should return a map");
        };
        let Value::Map(long) = waveguide_loss(10.0, 1.0) else {
            panic!("waveguide_loss should return a map");
        };
        assert!(
            long.get("attenuation_db").unwrap().as_float().unwrap()
                > short.get("attenuation_db").unwrap().as_float().unwrap()
        );
        assert_eq!(long.get("passed"), Some(&Value::Bool(true)));
    }
}
