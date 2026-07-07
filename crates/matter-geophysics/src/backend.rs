//! Backend integration for Matter Geophysics

use matter_backend::{Backend, Value};
use crate::*;

pub struct GeophysicsBackend;

impl Backend for GeophysicsBackend {
    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, String> {
        match function {
            // Earthquake functions
            "earthquake_new" => {
                let magnitude = args.get(0).ok_or("Missing magnitude")?.as_float()?;
                let depth = args.get(1).ok_or("Missing depth")?.as_float()?;
                let latitude = args.get(2).ok_or("Missing latitude")?.as_float()?;
                let longitude = args.get(3).ok_or("Missing longitude")?.as_float()?;
                
                let eq = Earthquake::new(magnitude, depth, latitude, longitude);
                Ok(Value::new_string(format!("Earthquake(M={}, depth={}km)", magnitude, depth)))
            }
            
            "earthquake_energy" => {
                let magnitude = args.get(0).ok_or("Missing magnitude")?.as_float()?;
                let depth = args.get(1).ok_or("Missing depth")?.as_float()?;
                
                let eq = Earthquake::new(magnitude, depth, 0.0, 0.0);
                Ok(Value::Float(eq.energy))
            }
            
            "earthquake_pga" => {
                let magnitude = args.get(0).ok_or("Missing magnitude")?.as_float()?;
                
                let eq = Earthquake::new(magnitude, 10.0, 0.0, 0.0);
                let pga = eq.peak_ground_acceleration();
                Ok(Value::Float(pga))
            }
            
            "earthquake_intensity" => {
                let magnitude = args.get(0).ok_or("Missing magnitude")?.as_float()?;
                let distance = args.get(1).ok_or("Missing distance")?.as_float()?;
                
                let eq = Earthquake::new(magnitude, 10.0, 0.0, 0.0);
                let intensity = eq.intensity_at_distance(distance);
                Ok(Value::Float(intensity))
            }
            
            // Seismic wave functions
            "wave_velocity_crust" => {
                let wave_type = args.get(0).ok_or("Missing wave type")?.as_string()?;
                let wave = match wave_type.as_str() {
                    "P" => WaveType::PWave,
                    "S" => WaveType::SWave,
                    "Love" => WaveType::LoveWave,
                    "Rayleigh" => WaveType::RayleighWave,
                    _ => return Err(format!("Unknown wave type: {}", wave_type)),
                };
                
                let velocity = wave.velocity_in_crust();
                Ok(Value::Float(velocity))
            }
            
            "wave_travel_time" => {
                let wave_type = args.get(0).ok_or("Missing wave type")?.as_string()?;
                let distance = args.get(1).ok_or("Missing distance")?.as_float()?;
                
                let wave = match wave_type.as_str() {
                    "P" => WaveType::PWave,
                    "S" => WaveType::SWave,
                    "Love" => WaveType::LoveWave,
                    "Rayleigh" => WaveType::RayleighWave,
                    _ => return Err(format!("Unknown wave type: {}", wave_type)),
                };
                
                let time = wave.travel_time(distance, false);
                Ok(Value::Float(time))
            }
            
            // Plate tectonics functions
            "plate_new" => {
                let name = args.get(0).ok_or("Missing name")?.as_string()?;
                let velocity = args.get(1).ok_or("Missing velocity")?.as_float()?;
                let direction = args.get(2).ok_or("Missing direction")?.as_float()?;
                let area = args.get(3).ok_or("Missing area")?.as_float()?;
                
                let _plate = TectonicPlate::new(name.clone(), velocity, direction, area);
                Ok(Value::new_string(format!("Plate({})", name)))
            }
            
            "plate_relative_velocity" => {
                let v1 = args.get(0).ok_or("Missing v1")?.as_float()?;
                let d1 = args.get(1).ok_or("Missing d1")?.as_float()?;
                let v2 = args.get(2).ok_or("Missing v2")?.as_float()?;
                let d2 = args.get(3).ok_or("Missing d2")?.as_float()?;
                
                let plate1 = TectonicPlate::new("P1".to_string(), v1, d1, 1e8);
                let plate2 = TectonicPlate::new("P2".to_string(), v2, d2, 1e8);
                
                let rel_v = plate1.relative_velocity(&plate2);
                Ok(Value::Float(rel_v))
            }
            
            "boundary_magnitude_range" => {
                let boundary_type = args.get(0).ok_or("Missing boundary type")?.as_string()?;
                let boundary = match boundary_type.as_str() {
                    "divergent" => BoundaryType::Divergent,
                    "convergent" => BoundaryType::Convergent,
                    "transform" => BoundaryType::Transform,
                    _ => return Err(format!("Unknown boundary type: {}", boundary_type)),
                };
                
                let (min, max) = boundary.typical_magnitude_range();
                Ok(Value::new_string(format!("{}-{}", min, max)))
            }
            
            // Geomagnetism functions
            "geomagnetic_field" => {
                let latitude = args.get(0).ok_or("Missing latitude")?.as_float()?;
                
                let field = GeomagneticField::at_latitude(latitude);
                Ok(Value::new_string(format!("Field(I={:.1}°, B={:.0}nT)", 
                    field.inclination, field.intensity)))
            }
            
            "geomagnetic_intensity" => {
                let latitude = args.get(0).ok_or("Missing latitude")?.as_float()?;
                
                let field = GeomagneticField::at_latitude(latitude);
                Ok(Value::Float(field.intensity))
            }
            
            "geomagnetic_inclination" => {
                let latitude = args.get(0).ok_or("Missing latitude")?.as_float()?;
                
                let field = GeomagneticField::at_latitude(latitude);
                Ok(Value::Float(field.inclination))
            }
            
            // Gravity functions
            "gravity_anomaly" => {
                let latitude = args.get(0).ok_or("Missing latitude")?.as_float()?;
                let elevation = args.get(1).ok_or("Missing elevation")?.as_float()?;
                
                let anomaly = GravityAnomaly::at_location(latitude, elevation);
                Ok(Value::new_string(format!("Anomaly(FA={:.1}, B={:.1})", 
                    anomaly.free_air, anomaly.bouguer)))
            }
            
            "gravity_total" => {
                let latitude = args.get(0).ok_or("Missing latitude")?.as_float()?;
                let elevation = args.get(1).ok_or("Missing elevation")?.as_float()?;
                
                let anomaly = GravityAnomaly::at_location(latitude, elevation);
                let g = anomaly.total_gravity();
                Ok(Value::Float(g))
            }
            
            // Heat flow functions
            "heat_flow_new" => {
                let surface_heat_flow = args.get(0).ok_or("Missing surface_heat_flow")?.as_float()?;
                let gradient = args.get(1).ok_or("Missing gradient")?.as_float()?;
                
                let _heat = HeatFlow::new(surface_heat_flow, gradient);
                Ok(Value::new_string(format!("HeatFlow(q={}mW/m², grad={}°C/km)", 
                    surface_heat_flow, gradient)))
            }
            
            "heat_flow_temperature" => {
                let surface_heat_flow = args.get(0).ok_or("Missing surface_heat_flow")?.as_float()?;
                let gradient = args.get(1).ok_or("Missing gradient")?.as_float()?;
                let depth = args.get(2).ok_or("Missing depth")?.as_float()?;
                
                let heat = HeatFlow::new(surface_heat_flow, gradient);
                let temp = heat.temperature_at_depth(depth);
                Ok(Value::Float(temp))
            }
            
            "heat_flow_moho_temp" => {
                let surface_heat_flow = args.get(0).ok_or("Missing surface_heat_flow")?.as_float()?;
                let gradient = args.get(1).ok_or("Missing gradient")?.as_float()?;
                
                let heat = HeatFlow::new(surface_heat_flow, gradient);
                let temp = heat.moho_temperature();
                Ok(Value::Float(temp))
            }
            
            _ => Err(format!("Unknown geophysics function: {}", function)),
        }
    }
}
