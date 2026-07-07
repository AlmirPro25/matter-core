//! Backend integration for Matter Electromagnetics

use matter_backend::{Backend, Value};
use crate::*;

pub struct ElectromagneticsBackend;

impl Backend for ElectromagneticsBackend {
    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, String> {
        match function {
            // EM wave functions
            "wave_vacuum" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let e_field = args.get(1).ok_or("Missing e_field")?.as_float()?;
                
                let _wave = EMWave::in_vacuum(frequency, e_field);
                Ok(Value::new_string(format!("EMWave({}Hz, {}V/m)", frequency, e_field)))
            }
            
            "wave_wavelength" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                
                let wave = EMWave::in_vacuum(frequency, 1.0);
                let lambda = wave.wavelength();
                Ok(Value::Float(lambda))
            }
            
            "wave_power_density" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let e_field = args.get(1).ok_or("Missing e_field")?.as_float()?;
                
                let wave = EMWave::in_vacuum(frequency, e_field);
                let power = wave.power_density();
                Ok(Value::Float(power))
            }
            
            "wave_impedance" => {
                let epsilon_r = args.get(0).ok_or("Missing epsilon_r")?.as_float()?;
                let mu_r = args.get(1).ok_or("Missing mu_r")?.as_float()?;
                
                let wave = EMWave::in_medium(1e9, 1.0, epsilon_r, mu_r);
                let z = wave.intrinsic_impedance();
                Ok(Value::Float(z))
            }
            
            // Transmission line functions
            "tline_new" => {
                let z0 = args.get(0).ok_or("Missing z0")?.as_float()?;
                let z_load = args.get(1).ok_or("Missing z_load")?.as_float()?;
                let length = args.get(2).ok_or("Missing length")?.as_float()?;
                let frequency = args.get(3).ok_or("Missing frequency")?.as_float()?;
                
                let _line = TransmissionLine::new(z0, z_load, length, frequency);
                Ok(Value::new_string(format!("TLine(Z₀={}Ω, Z_L={}Ω)", z0, z_load)))
            }
            
            "tline_vswr" => {
                let z0 = args.get(0).ok_or("Missing z0")?.as_float()?;
                let z_load = args.get(1).ok_or("Missing z_load")?.as_float()?;
                
                let line = TransmissionLine::new(z0, z_load, 1.0, 1e9);
                let vswr = line.vswr();
                Ok(Value::Float(vswr))
            }
            
            "tline_return_loss" => {
                let z0 = args.get(0).ok_or("Missing z0")?.as_float()?;
                let z_load = args.get(1).ok_or("Missing z_load")?.as_float()?;
                
                let line = TransmissionLine::new(z0, z_load, 1.0, 1e9);
                let rl = line.return_loss();
                Ok(Value::Float(rl))
            }
            
            "tline_reflection" => {
                let z0 = args.get(0).ok_or("Missing z0")?.as_float()?;
                let z_load = args.get(1).ok_or("Missing z_load")?.as_float()?;
                
                let line = TransmissionLine::new(z0, z_load, 1.0, 1e9);
                let gamma = line.reflection_coefficient();
                Ok(Value::Float(gamma))
            }
            
            // Antenna functions
            "antenna_dipole" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                
                let _ant = Antenna::dipole_half_wave(frequency);
                Ok(Value::new_string(format!("Dipole({}MHz)", frequency / 1e6)))
            }
            
            "antenna_gain" => {
                let antenna_type = args.get(0).ok_or("Missing antenna_type")?.as_string()?;
                let frequency = args.get(1).ok_or("Missing frequency")?.as_float()?;
                
                let ant = match antenna_type.as_str() {
                    "dipole" => Antenna::dipole_half_wave(frequency),
                    "monopole" => Antenna::monopole_quarter_wave(frequency),
                    _ => return Err(format!("Unknown antenna type: {}", antenna_type)),
                };
                
                Ok(Value::Float(ant.gain_dbi))
            }
            
            "antenna_length" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                
                let ant = Antenna::dipole_half_wave(frequency);
                let length = ant.physical_length();
                Ok(Value::Float(length))
            }
            
            "antenna_beamwidth" => {
                let gain_dbi = args.get(0).ok_or("Missing gain_dbi")?.as_float()?;
                
                let ant = Antenna::new(AntennaType::Dipole, gain_dbi, 1e9, 0.95);
                let bw = ant.beamwidth_3db();
                Ok(Value::Float(bw))
            }
            
            // Link budget functions
            "link_new" => {
                let tx_power = args.get(0).ok_or("Missing tx_power")?.as_float()?;
                let tx_gain = args.get(1).ok_or("Missing tx_gain")?.as_float()?;
                let rx_gain = args.get(2).ok_or("Missing rx_gain")?.as_float()?;
                let frequency = args.get(3).ok_or("Missing frequency")?.as_float()?;
                let distance = args.get(4).ok_or("Missing distance")?.as_float()?;
                
                let _link = LinkBudget::new(tx_power, tx_gain, rx_gain, frequency, distance);
                Ok(Value::new_string(format!("Link({}m, {}MHz)", distance, frequency / 1e6)))
            }
            
            "link_path_loss" => {
                let frequency = args.get(0).ok_or("Missing frequency")?.as_float()?;
                let distance = args.get(1).ok_or("Missing distance")?.as_float()?;
                
                let link = LinkBudget::new(1.0, 0.0, 0.0, frequency, distance);
                let fspl = link.path_loss();
                Ok(Value::Float(fspl))
            }
            
            "link_received_power" => {
                let tx_power = args.get(0).ok_or("Missing tx_power")?.as_float()?;
                let tx_gain = args.get(1).ok_or("Missing tx_gain")?.as_float()?;
                let rx_gain = args.get(2).ok_or("Missing rx_gain")?.as_float()?;
                let frequency = args.get(3).ok_or("Missing frequency")?.as_float()?;
                let distance = args.get(4).ok_or("Missing distance")?.as_float()?;
                
                let link = LinkBudget::new(tx_power, tx_gain, rx_gain, frequency, distance);
                let p_rx = link.received_power_dbm();
                Ok(Value::Float(p_rx))
            }
            
            "link_margin" => {
                let tx_power = args.get(0).ok_or("Missing tx_power")?.as_float()?;
                let distance = args.get(1).ok_or("Missing distance")?.as_float()?;
                let sensitivity = args.get(2).ok_or("Missing sensitivity")?.as_float()?;
                
                let link = LinkBudget::new(tx_power, 0.0, 0.0, 2.4e9, distance);
                let margin = link.link_margin(sensitivity);
                Ok(Value::Float(margin))
            }
            
            // Shielding functions
            "shielding_copper" => {
                let thickness = args.get(0).ok_or("Missing thickness")?.as_float()?;
                let frequency = args.get(1).ok_or("Missing frequency")?.as_float()?;
                
                let _shield = Shielding::copper(thickness, frequency);
                Ok(Value::new_string(format!("Shield(Cu, {}mm)", thickness * 1000.0)))
            }
            
            "shielding_skin_depth" => {
                let material = args.get(0).ok_or("Missing material")?.as_string()?;
                let frequency = args.get(1).ok_or("Missing frequency")?.as_float()?;
                
                let shield = match material.as_str() {
                    "copper" => Shielding::copper(0.001, frequency),
                    "aluminum" => Shielding::aluminum(0.001, frequency),
                    _ => return Err(format!("Unknown material: {}", material)),
                };
                
                let delta = shield.skin_depth();
                Ok(Value::Float(delta))
            }
            
            "shielding_effectiveness" => {
                let material = args.get(0).ok_or("Missing material")?.as_string()?;
                let thickness = args.get(1).ok_or("Missing thickness")?.as_float()?;
                let frequency = args.get(2).ok_or("Missing frequency")?.as_float()?;
                
                let shield = match material.as_str() {
                    "copper" => Shielding::copper(thickness, frequency),
                    "aluminum" => Shielding::aluminum(thickness, frequency),
                    _ => return Err(format!("Unknown material: {}", material)),
                };
                
                let se = shield.shielding_effectiveness();
                Ok(Value::Float(se))
            }
            
            _ => Err(format!("Unknown electromagnetics function: {}", function)),
        }
    }
}
