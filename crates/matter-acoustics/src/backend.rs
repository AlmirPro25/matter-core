//! Backend integration for Matter Acoustics

use matter_backend::{Backend, Value};
use crate::*;

pub struct AcousticsBackend;

impl Backend for AcousticsBackend {
    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, String> {
        match function {
            // Sound wave functions
            "wave_new" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let amplitude = args.get(1).ok_or("Missing amplitude")?.as_float()?;
                let medium_type = args.get(2).ok_or("Missing medium")?.as_string()?;
                
                let medium = match medium_type.as_str() {
                    "air" => Medium::air_standard(),
                    "water" => Medium::water_standard(),
                    "steel" => Medium::steel(),
                    _ => return Err(format!("Unknown medium: {}", medium_type)),
                };
                
                let _wave = SoundWave::new(frequency, amplitude, medium);
                Ok(Value::new_string(format!("Wave({}Hz, {}Pa)", frequency, amplitude)))
            }
            
            "wave_wavelength" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let medium_type = args.get(1).ok_or("Missing medium")?.as_string()?;
                
                let medium = match medium_type.as_str() {
                    "air" => Medium::air_standard(),
                    "water" => Medium::water_standard(),
                    "steel" => Medium::steel(),
                    _ => return Err(format!("Unknown medium: {}", medium_type)),
                };
                
                let wave = SoundWave::new(frequency, 0.02, medium);
                let lambda = wave.wavelength();
                Ok(Value::Float(lambda))
            }
            
            "wave_spl" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let amplitude = args.get(1).ok_or("Missing amplitude")?.as_float()?;
                
                let wave = SoundWave::new(frequency, amplitude, Medium::air_standard());
                let spl = wave.sound_pressure_level();
                Ok(Value::Float(spl))
            }
            
            "wave_intensity" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let amplitude = args.get(1).ok_or("Missing amplitude")?.as_float()?;
                
                let wave = SoundWave::new(frequency, amplitude, Medium::air_standard());
                let intensity = wave.intensity();
                Ok(Value::Float(intensity))
            }
            
            // Doppler effect functions
            "doppler_new" => {
                let source_freq = args.get(0).ok_or("Missing source_frequency")?.as_float()?;
                let speed = args.get(1).ok_or("Missing speed_of_sound")?.as_float()?;
                let v_obs = args.get(2).ok_or("Missing observer_velocity")?.as_float()?;
                let v_src = args.get(3).ok_or("Missing source_velocity")?.as_float()?;
                
                let _doppler = DopplerEffect::new(source_freq, speed, v_obs, v_src);
                Ok(Value::new_string(format!("Doppler(f={}Hz)", source_freq)))
            }
            
            "doppler_frequency" => {
                let source_freq = args.get(0).ok_or("Missing source_frequency")?.as_float()?;
                let v_obs = args.get(1).ok_or("Missing observer_velocity")?.as_float()?;
                let v_src = args.get(2).ok_or("Missing source_velocity")?.as_float()?;
                
                let doppler = DopplerEffect::new(source_freq, 343.0, v_obs, v_src);
                let f_obs = doppler.observed_frequency();
                Ok(Value::Float(f_obs))
            }
            
            "doppler_shift" => {
                let source_freq = args.get(0).ok_or("Missing source_frequency")?.as_float()?;
                let v_obs = args.get(1).ok_or("Missing observer_velocity")?.as_float()?;
                let v_src = args.get(2).ok_or("Missing source_velocity")?.as_float()?;
                
                let doppler = DopplerEffect::new(source_freq, 343.0, v_obs, v_src);
                let shift = doppler.frequency_shift();
                Ok(Value::Float(shift))
            }
            
            // Room acoustics functions
            "room_new" => {
                let volume = args.get(0).ok_or("Missing volume")?.as_float()?;
                let surface = args.get(1).ok_or("Missing surface_area")?.as_float()?;
                let absorption = args.get(2).ok_or("Missing absorption")?.as_float()?;
                
                let _room = Room::new(volume, surface, absorption);
                Ok(Value::new_string(format!("Room(V={}m³)", volume)))
            }
            
            "room_rectangular" => {
                let length = args.get(0).ok_or("Missing length")?.as_float()?;
                let width = args.get(1).ok_or("Missing width")?.as_float()?;
                let height = args.get(2).ok_or("Missing height")?.as_float()?;
                let absorption = args.get(3).ok_or("Missing absorption")?.as_float()?;
                
                let _room = Room::rectangular(length, width, height, absorption);
                Ok(Value::new_string(format!("Room({}×{}×{}m)", length, width, height)))
            }
            
            "room_rt60" => {
                let volume = args.get(0).ok_or("Missing volume")?.as_float()?;
                let surface = args.get(1).ok_or("Missing surface_area")?.as_float()?;
                let absorption = args.get(2).ok_or("Missing absorption")?.as_float()?;
                
                let room = Room::new(volume, surface, absorption);
                let rt60 = room.reverberation_time();
                Ok(Value::Float(rt60))
            }
            
            "room_quality" => {
                let volume = args.get(0).ok_or("Missing volume")?.as_float()?;
                let surface = args.get(1).ok_or("Missing surface_area")?.as_float()?;
                let absorption = args.get(2).ok_or("Missing absorption")?.as_float()?;
                
                let room = Room::new(volume, surface, absorption);
                let quality = room.acoustics_quality();
                Ok(Value::new_string(quality.to_string()))
            }
            
            // Attenuation functions
            "attenuation_new" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let temperature = args.get(1).ok_or("Missing temperature")?.as_float()?;
                let humidity = args.get(2).ok_or("Missing humidity")?.as_float()?;
                let pressure = args.get(3).ok_or("Missing pressure")?.as_float()?;
                
                let _atten = Attenuation::new(frequency, temperature, humidity, pressure);
                Ok(Value::new_string(format!("Attenuation({}Hz)", frequency)))
            }
            
            "attenuation_coefficient" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let temperature = args.get(1).ok_or("Missing temperature")?.as_float()?;
                let humidity = args.get(2).ok_or("Missing humidity")?.as_float()?;
                
                let atten = Attenuation::new(frequency, temperature, humidity, 101.325);
                let alpha = atten.absorption_coefficient();
                Ok(Value::Float(alpha))
            }
            
            "attenuation_at_distance" => {
                let spl_source = args.get(0).ok_or("Missing spl_source")?.as_float()?;
                let frequency = args.get(1).ok_or("Missing frequency")?.as_float()?;
                let distance = args.get(2).ok_or("Missing distance")?.as_float()?;
                
                let atten = Attenuation::new(frequency, 20.0, 50.0, 101.325);
                let spl = atten.sound_level_at_distance(spl_source, distance);
                Ok(Value::Float(spl))
            }
            
            // Ultrasound functions
            "ultrasound_new" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let diameter = args.get(1).ok_or("Missing diameter")?.as_float()?;
                let medium_type = args.get(2).ok_or("Missing medium")?.as_string()?;
                
                let medium = match medium_type.as_str() {
                    "water" => Medium::water_standard(),
                    "tissue" => Medium {
                        density: 1050.0,
                        speed_of_sound: 1540.0,
                        temperature: 310.0,
                    },
                    _ => return Err(format!("Unknown medium: {}", medium_type)),
                };
                
                let _us = Ultrasound::new(frequency, diameter, medium);
                Ok(Value::new_string(format!("Ultrasound({}MHz)", frequency / 1e6)))
            }
            
            "ultrasound_near_field" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let diameter = args.get(1).ok_or("Missing diameter")?.as_float()?;
                
                let us = Ultrasound::new(frequency, diameter, Medium::water_standard());
                let nf = us.near_field_length();
                Ok(Value::Float(nf))
            }
            
            "ultrasound_divergence" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let diameter = args.get(1).ok_or("Missing diameter")?.as_float()?;
                
                let us = Ultrasound::new(frequency, diameter, Medium::water_standard());
                let div = us.beam_divergence();
                Ok(Value::Float(div))
            }
            
            "ultrasound_penetration" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                
                let us = Ultrasound::new(frequency, 0.01, Medium::water_standard());
                let depth = us.penetration_depth_medical();
                Ok(Value::Float(depth))
            }
            
            // Resonator functions
            "resonator_new" => {
                let nat_freq = args.get(0).ok_or("Missing natural_frequency")?.as_float()?;
                let q_factor = args.get(1).ok_or("Missing quality_factor")?.as_float()?;
                let mass = args.get(2).ok_or("Missing mass")?.as_float()?;
                
                let _res = Resonator::new(nat_freq, q_factor, mass);
                Ok(Value::new_string(format!("Resonator(f₀={}Hz, Q={})", nat_freq, q_factor)))
            }
            
            "resonator_bandwidth" => {
                let nat_freq = args.get(0).ok_or("Missing natural_frequency")?.as_float()?;
                let q_factor = args.get(1).ok_or("Missing quality_factor")?.as_float()?;
                
                let res = Resonator::new(nat_freq, q_factor, 1.0);
                let bw = res.bandwidth();
                Ok(Value::Float(bw))
            }
            
            "resonator_response" => {
                let nat_freq = args.get(0).ok_or("Missing natural_frequency")?.as_float()?;
                let q_factor = args.get(1).ok_or("Missing quality_factor")?.as_float()?;
                let frequency = args.get(2).ok_or("Missing frequency")?.as_float()?;
                
                let res = Resonator::new(nat_freq, q_factor, 1.0);
                let amp = res.amplitude_response(frequency);
                Ok(Value::Float(amp))
            }
            
            _ => Err(format!("Unknown acoustics function: {}", function)),
        }
    }
}
