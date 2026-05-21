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
            _ => Err(format!("Unknown photonic method: {}", method)),
        }
    }
}
