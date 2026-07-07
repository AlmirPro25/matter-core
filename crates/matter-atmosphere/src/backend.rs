//! Backend integration for Matter Atmosphere

use matter_backend::{Backend, Value};
use crate::*;

pub struct AtmosphereBackend;

impl Backend for AtmosphereBackend {
    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, String> {
        match function {
            // Atmosphere functions
            "atmosphere_at_altitude" => {
                let altitude = args.get(0).ok_or("Missing altitude")?.as_float()?;
                
                let atm = Atmosphere::at_altitude(altitude);
                Ok(Value::new_string(format!("Atmosphere(T={:.1}K, P={:.0}Pa)", 
                    atm.temperature, atm.pressure)))
            }
            
            "atmosphere_temperature" => {
                let altitude = args.get(0).ok_or("Missing altitude")?.as_float()?;
                
                let atm = Atmosphere::at_altitude(altitude);
                Ok(Value::Float(atm.temperature))
            }
            
            "atmosphere_pressure" => {
                let altitude = args.get(0).ok_or("Missing altitude")?.as_float()?;
                
                let atm = Atmosphere::at_altitude(altitude);
                Ok(Value::Float(atm.pressure))
            }
            
            "atmosphere_sound_speed" => {
                let altitude = args.get(0).ok_or("Missing altitude")?.as_float()?;
                
                let atm = Atmosphere::at_altitude(altitude);
                let c = atm.sound_speed();
                Ok(Value::Float(c))
            }
            
            // Humidity functions
            "humidity_new" => {
                let temperature = args.get(0).ok_or("Missing temperature")?.as_float()?;
                let pressure = args.get(1).ok_or("Missing pressure")?.as_float()?;
                let rh = args.get(2).ok_or("Missing relative_humidity")?.as_float()?;
                
                let _humidity = Humidity::new(temperature, pressure, rh);
                Ok(Value::new_string(format!("Humidity(T={:.1}K, RH={:.0}%)", 
                    temperature, rh * 100.0)))
            }
            
            "humidity_dew_point" => {
                let temperature = args.get(0).ok_or("Missing temperature")?.as_float()?;
                let rh = args.get(1).ok_or("Missing relative_humidity")?.as_float()?;
                
                let humidity = Humidity::new(temperature, 101325.0, rh);
                let t_d = humidity.dew_point();
                Ok(Value::Float(t_d))
            }
            
            "humidity_vapor_pressure" => {
                let temperature = args.get(0).ok_or("Missing temperature")?.as_float()?;
                let rh = args.get(1).ok_or("Missing relative_humidity")?.as_float()?;
                
                let humidity = Humidity::new(temperature, 101325.0, rh);
                let e = humidity.vapor_pressure();
                Ok(Value::Float(e))
            }
            
            // Wind functions
            "wind_new" => {
                let u = args.get(0).ok_or("Missing u")?.as_float()?;
                let v = args.get(1).ok_or("Missing v")?.as_float()?;
                let latitude = args.get(2).ok_or("Missing latitude")?.as_float()?;
                
                let wind = Wind::new(u, v, latitude);
                Ok(Value::new_string(format!("Wind(speed={:.1}m/s, dir={:.0}°)", 
                    wind.speed(), wind.direction())))
            }
            
            "wind_speed" => {
                let u = args.get(0).ok_or("Missing u")?.as_float()?;
                let v = args.get(1).ok_or("Missing v")?.as_float()?;
                
                let wind = Wind::new(u, v, 0.0);
                let speed = wind.speed();
                Ok(Value::Float(speed))
            }
            
            "wind_direction" => {
                let u = args.get(0).ok_or("Missing u")?.as_float()?;
                let v = args.get(1).ok_or("Missing v")?.as_float()?;
                
                let wind = Wind::new(u, v, 0.0);
                let dir = wind.direction();
                Ok(Value::Float(dir))
            }
            
            "wind_coriolis" => {
                let latitude = args.get(0).ok_or("Missing latitude")?.as_float()?;
                
                let wind = Wind::new(0.0, 0.0, latitude);
                let f = wind.coriolis_parameter();
                Ok(Value::Float(f))
            }
            
            // Radiation functions
            "radiation_new" => {
                let albedo = args.get(0).ok_or("Missing albedo")?.as_float()?;
                let emissivity = args.get(1).ok_or("Missing emissivity")?.as_float()?;
                
                let _rad = Radiation::new(albedo, emissivity);
                Ok(Value::new_string(format!("Radiation(α={:.2}, ε={:.2})", 
                    albedo, emissivity)))
            }
            
            "radiation_equilibrium_temp" => {
                let albedo = args.get(0).ok_or("Missing albedo")?.as_float()?;
                let emissivity = args.get(1).ok_or("Missing emissivity")?.as_float()?;
                
                let rad = Radiation::new(albedo, emissivity);
                let t_eq = rad.equilibrium_temperature();
                Ok(Value::Float(t_eq))
            }
            
            "radiation_greenhouse_effect" => {
                let albedo = args.get(0).ok_or("Missing albedo")?.as_float()?;
                let emissivity = args.get(1).ok_or("Missing emissivity")?.as_float()?;
                
                let rad = Radiation::new(albedo, emissivity);
                let greenhouse = rad.greenhouse_effect();
                Ok(Value::Float(greenhouse))
            }
            
            // Pressure system functions
            "pressure_system_pressure" => {
                let system_type = args.get(0).ok_or("Missing system_type")?.as_string()?;
                
                let system = match system_type.as_str() {
                    "cyclone" => PressureSystem::Cyclone,
                    "anticyclone" => PressureSystem::Anticyclone,
                    _ => return Err(format!("Unknown system type: {}", system_type)),
                };
                
                let p = system.typical_pressure();
                Ok(Value::Float(p))
            }
            
            "pressure_system_weather" => {
                let system_type = args.get(0).ok_or("Missing system_type")?.as_string()?;
                
                let system = match system_type.as_str() {
                    "cyclone" => PressureSystem::Cyclone,
                    "anticyclone" => PressureSystem::Anticyclone,
                    _ => return Err(format!("Unknown system type: {}", system_type)),
                };
                
                let weather = system.typical_weather();
                Ok(Value::new_string(weather.to_string()))
            }
            
            // Storm functions
            "storm_new" => {
                let central_pressure = args.get(0).ok_or("Missing central_pressure")?.as_float()?;
                let max_wind = args.get(1).ok_or("Missing max_wind")?.as_float()?;
                let radius = args.get(2).ok_or("Missing radius")?.as_float()?;
                
                let storm = Storm::new(central_pressure, max_wind, radius);
                Ok(Value::new_string(format!("Storm(P={}Pa, V={}m/s, Cat={})", 
                    central_pressure as i64, max_wind as i64, storm.intensity_category())))
            }
            
            "storm_category" => {
                let max_wind = args.get(0).ok_or("Missing max_wind")?.as_float()?;
                
                let storm = Storm::new(95000.0, max_wind, 100.0);
                let category = storm.intensity_category();
                Ok(Value::Int(category as i64))
            }
            
            "storm_energy" => {
                let central_pressure = args.get(0).ok_or("Missing central_pressure")?.as_float()?;
                let max_wind = args.get(1).ok_or("Missing max_wind")?.as_float()?;
                let radius = args.get(2).ok_or("Missing radius")?.as_float()?;
                
                let storm = Storm::new(central_pressure, max_wind, radius);
                let energy = storm.kinetic_energy();
                Ok(Value::Float(energy))
            }
            
            _ => Err(format!("Unknown atmosphere function: {}", function)),
        }
    }
}
