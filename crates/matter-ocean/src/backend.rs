//! Backend integration for Matter Ocean

use matter_backend::{Backend, Value};
use crate::*;

pub struct OceanBackend;

impl Backend for OceanBackend {
    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, String> {
        match function {
            // Seawater functions
            "seawater_new" => {
                let temperature = args.get(0).ok_or("Missing temperature")?.as_float()?;
                let salinity = args.get(1).ok_or("Missing salinity")?.as_float()?;
                let pressure = args.get(2).ok_or("Missing pressure")?.as_float()?;
                
                let water = Seawater::new(temperature, salinity, pressure);
                Ok(Value::new_string(format!("Seawater(T={}°C, S={}PSU, ρ={}kg/m³)", 
                    temperature, salinity, water.density as i64)))
            }
            
            "seawater_density" => {
                let temperature = args.get(0).ok_or("Missing temperature")?.as_float()?;
                let salinity = args.get(1).ok_or("Missing salinity")?.as_float()?;
                let pressure = args.get(2).ok_or("Missing pressure")?.as_float()?;
                
                let water = Seawater::new(temperature, salinity, pressure);
                Ok(Value::Float(water.density))
            }
            
            "seawater_sound_speed" => {
                let temperature = args.get(0).ok_or("Missing temperature")?.as_float()?;
                let salinity = args.get(1).ok_or("Missing salinity")?.as_float()?;
                let pressure = args.get(2).ok_or("Missing pressure")?.as_float()?;
                
                let water = Seawater::new(temperature, salinity, pressure);
                let c = water.sound_speed();
                Ok(Value::Float(c))
            }
            
            // Wave functions
            "wave_new" => {
                let height = args.get(0).ok_or("Missing height")?.as_float()?;
                let wavelength = args.get(1).ok_or("Missing wavelength")?.as_float()?;
                let period = args.get(2).ok_or("Missing period")?.as_float()?;
                let depth = args.get(3).ok_or("Missing depth")?.as_float()?;
                
                let _wave = Wave::new(height, wavelength, period, depth);
                Ok(Value::new_string(format!("Wave(H={}m, λ={}m)", height, wavelength)))
            }
            
            "wave_speed" => {
                let height = args.get(0).ok_or("Missing height")?.as_float()?;
                let wavelength = args.get(1).ok_or("Missing wavelength")?.as_float()?;
                let depth = args.get(2).ok_or("Missing depth")?.as_float()?;
                
                let wave = Wave::new(height, wavelength, 10.0, depth);
                let speed = wave.speed();
                Ok(Value::Float(speed))
            }
            
            "wave_energy" => {
                let height = args.get(0).ok_or("Missing height")?.as_float()?;
                let wavelength = args.get(1).ok_or("Missing wavelength")?.as_float()?;
                
                let wave = Wave::new(height, wavelength, 10.0, 1000.0);
                let energy = wave.energy_density();
                Ok(Value::Float(energy))
            }
            
            // Tsunami functions
            "tsunami_new" => {
                let height = args.get(0).ok_or("Missing height")?.as_float()?;
                let depth = args.get(1).ok_or("Missing depth")?.as_float()?;
                
                let _tsunami = Tsunami::new(height, depth);
                Ok(Value::new_string(format!("Tsunami(H={}m, d={}m)", height, depth)))
            }
            
            "tsunami_speed" => {
                let depth = args.get(0).ok_or("Missing depth")?.as_float()?;
                
                let tsunami = Tsunami::new(1.0, depth);
                let speed = tsunami.speed();
                Ok(Value::Float(speed))
            }
            
            "tsunami_travel_time" => {
                let depth = args.get(0).ok_or("Missing depth")?.as_float()?;
                let distance = args.get(1).ok_or("Missing distance")?.as_float()?;
                
                let tsunami = Tsunami::new(1.0, depth);
                let time = tsunami.travel_time(distance);
                Ok(Value::Float(time))
            }
            
            "tsunami_height_at_shore" => {
                let initial_height = args.get(0).ok_or("Missing initial_height")?.as_float()?;
                let ocean_depth = args.get(1).ok_or("Missing ocean_depth")?.as_float()?;
                let shore_depth = args.get(2).ok_or("Missing shore_depth")?.as_float()?;
                
                let tsunami = Tsunami::new(initial_height, ocean_depth);
                let h = tsunami.height_at_shore(shore_depth);
                Ok(Value::Float(h))
            }
            
            // Current functions
            "current_new" => {
                let velocity = args.get(0).ok_or("Missing velocity")?.as_float()?;
                let direction = args.get(1).ok_or("Missing direction")?.as_float()?;
                let depth = args.get(2).ok_or("Missing depth")?.as_float()?;
                let latitude = args.get(3).ok_or("Missing latitude")?.as_float()?;
                
                let _current = Current::new(velocity, direction, depth, latitude);
                Ok(Value::new_string(format!("Current(v={}m/s, dir={}°)", velocity, direction)))
            }
            
            "current_coriolis" => {
                let latitude = args.get(0).ok_or("Missing latitude")?.as_float()?;
                
                let current = Current::new(0.5, 0.0, 100.0, latitude);
                let f = current.coriolis_parameter();
                Ok(Value::Float(f))
            }
            
            "current_rossby_radius" => {
                let latitude = args.get(0).ok_or("Missing latitude")?.as_float()?;
                let wave_speed = args.get(1).ok_or("Missing wave_speed")?.as_float()?;
                
                let current = Current::new(0.5, 0.0, 100.0, latitude);
                let r = current.rossby_radius(wave_speed);
                Ok(Value::Float(r))
            }
            
            // Ekman functions
            "ekman_new" => {
                let wind_stress = args.get(0).ok_or("Missing wind_stress")?.as_float()?;
                let latitude = args.get(1).ok_or("Missing latitude")?.as_float()?;
                
                let _ekman = EkmanSpiral::new(wind_stress, latitude);
                Ok(Value::new_string(format!("Ekman(τ={}N/m², lat={}°)", wind_stress, latitude)))
            }
            
            "ekman_depth" => {
                let wind_stress = args.get(0).ok_or("Missing wind_stress")?.as_float()?;
                let latitude = args.get(1).ok_or("Missing latitude")?.as_float()?;
                
                let ekman = EkmanSpiral::new(wind_stress, latitude);
                let depth = ekman.ekman_depth();
                Ok(Value::Float(depth))
            }
            
            "ekman_transport" => {
                let wind_stress = args.get(0).ok_or("Missing wind_stress")?.as_float()?;
                let latitude = args.get(1).ok_or("Missing latitude")?.as_float()?;
                
                let ekman = EkmanSpiral::new(wind_stress, latitude);
                let transport = ekman.ekman_transport();
                Ok(Value::Float(transport))
            }
            
            // Tide functions
            "tide_new" => {
                let amplitude = args.get(0).ok_or("Missing amplitude")?.as_float()?;
                let period = args.get(1).ok_or("Missing period")?.as_float()?;
                let phase = args.get(2).ok_or("Missing phase")?.as_float()?;
                
                let _tide = Tide::new(amplitude, period, phase);
                Ok(Value::new_string(format!("Tide(A={}m, T={}h)", amplitude, period)))
            }
            
            "tide_height" => {
                let amplitude = args.get(0).ok_or("Missing amplitude")?.as_float()?;
                let period = args.get(1).ok_or("Missing period")?.as_float()?;
                let time = args.get(2).ok_or("Missing time")?.as_float()?;
                
                let tide = Tide::new(amplitude, period, 0.0);
                let h = tide.height_at_time(time);
                Ok(Value::Float(h))
            }
            
            "tidal_system_new" => {
                let _system = TidalSystem::new();
                Ok(Value::new_string("TidalSystem(M2+S2+K1)".to_string()))
            }
            
            "tidal_system_height" => {
                let time = args.get(0).ok_or("Missing time")?.as_float()?;
                
                let system = TidalSystem::new();
                let h = system.total_height(time);
                Ok(Value::Float(h))
            }
            
            "tidal_system_range" => {
                let system = TidalSystem::new();
                let range = system.tidal_range();
                Ok(Value::Float(range))
            }
            
            _ => Err(format!("Unknown ocean function: {}", function)),
        }
    }
}
