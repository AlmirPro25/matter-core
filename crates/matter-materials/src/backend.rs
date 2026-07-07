//! Backend integration for Matter Materials

use matter_backend::{Backend, Value};
use crate::*;

pub struct MaterialsBackend;

impl Backend for MaterialsBackend {
    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, String> {
        match function {
            // Crystal functions
            "crystal_cubic" => {
                let lattice_type = args.get(0).ok_or("Missing lattice_type")?.as_string()?;
                let a = args.get(1).ok_or("Missing a")?.as_float()?;
                
                let lattice = match lattice_type.as_str() {
                    "simple" => LatticeType::SimpleCubic,
                    "bcc" => LatticeType::BodyCenteredCubic,
                    "fcc" => LatticeType::FaceCenteredCubic,
                    _ => return Err(format!("Unknown lattice type: {}", lattice_type)),
                };
                
                let _crystal = Crystal::cubic(lattice, a);
                Ok(Value::new_string(format!("Crystal({:?}, a={}Å)", lattice, a)))
            }
            
            "crystal_volume" => {
                let lattice_type = args.get(0).ok_or("Missing lattice_type")?.as_string()?;
                let a = args.get(1).ok_or("Missing a")?.as_float()?;
                
                let lattice = match lattice_type.as_str() {
                    "simple" => LatticeType::SimpleCubic,
                    "bcc" => LatticeType::BodyCenteredCubic,
                    "fcc" => LatticeType::FaceCenteredCubic,
                    _ => return Err(format!("Unknown lattice type: {}", lattice_type)),
                };
                
                let crystal = Crystal::cubic(lattice, a);
                let volume = crystal.unit_cell_volume();
                Ok(Value::Float(volume))
            }
            
            "crystal_d_spacing" => {
                let a = args.get(0).ok_or("Missing a")?.as_float()?;
                let h = args.get(1).ok_or("Missing h")?.as_int()? as i32;
                let k = args.get(2).ok_or("Missing k")?.as_int()? as i32;
                let l = args.get(3).ok_or("Missing l")?.as_int()? as i32;
                
                let crystal = Crystal::cubic(LatticeType::SimpleCubic, a);
                let d = crystal.d_spacing(h, k, l);
                Ok(Value::Float(d))
            }
            
            "crystal_packing" => {
                let lattice_type = args.get(0).ok_or("Missing lattice_type")?.as_string()?;
                
                let lattice = match lattice_type.as_str() {
                    "simple" => LatticeType::SimpleCubic,
                    "bcc" => LatticeType::BodyCenteredCubic,
                    "fcc" => LatticeType::FaceCenteredCubic,
                    _ => return Err(format!("Unknown lattice type: {}", lattice_type)),
                };
                
                let packing = lattice.packing_efficiency();
                Ok(Value::Float(packing))
            }
            
            // XRD functions
            "xrd_new" => {
                let wavelength = args.get(0).ok_or("Missing wavelength")?.as_float()?;
                let a = args.get(1).ok_or("Missing a")?.as_float()?;
                
                let crystal = Crystal::cubic(LatticeType::SimpleCubic, a);
                let _xrd = XRayDiffraction::new(wavelength, crystal);
                Ok(Value::new_string(format!("XRD(λ={}Å)", wavelength)))
            }
            
            "xrd_bragg_angle" => {
                let wavelength = args.get(0).ok_or("Missing wavelength")?.as_float()?;
                let a = args.get(1).ok_or("Missing a")?.as_float()?;
                let h = args.get(2).ok_or("Missing h")?.as_int()? as i32;
                let k = args.get(3).ok_or("Missing k")?.as_int()? as i32;
                let l = args.get(4).ok_or("Missing l")?.as_int()? as i32;
                
                let crystal = Crystal::cubic(LatticeType::FaceCenteredCubic, a);
                let xrd = XRayDiffraction::new(wavelength, crystal);
                
                match xrd.bragg_angle(h, k, l, 1) {
                    Some(angle) => Ok(Value::Float(angle)),
                    None => Err("No diffraction at this angle".to_string()),
                }
            }
            
            "xrd_two_theta" => {
                let wavelength = args.get(0).ok_or("Missing wavelength")?.as_float()?;
                let a = args.get(1).ok_or("Missing a")?.as_float()?;
                let h = args.get(2).ok_or("Missing h")?.as_int()? as i32;
                let k = args.get(3).ok_or("Missing k")?.as_int()? as i32;
                let l = args.get(4).ok_or("Missing l")?.as_int()? as i32;
                
                let crystal = Crystal::cubic(LatticeType::FaceCenteredCubic, a);
                let xrd = XRayDiffraction::new(wavelength, crystal);
                
                match xrd.two_theta(h, k, l, 1) {
                    Some(angle) => Ok(Value::Float(angle)),
                    None => Err("No diffraction at this angle".to_string()),
                }
            }
            
            "xrd_intensity" => {
                let wavelength = args.get(0).ok_or("Missing wavelength")?.as_float()?;
                let a = args.get(1).ok_or("Missing a")?.as_float()?;
                let h = args.get(2).ok_or("Missing h")?.as_int()? as i32;
                let k = args.get(3).ok_or("Missing k")?.as_int()? as i32;
                let l = args.get(4).ok_or("Missing l")?.as_int()? as i32;
                
                let crystal = Crystal::cubic(LatticeType::FaceCenteredCubic, a);
                let xrd = XRayDiffraction::new(wavelength, crystal);
                let intensity = xrd.relative_intensity(h, k, l);
                Ok(Value::Float(intensity))
            }
            
            // Mechanical properties functions
            "mechanical_new" => {
                let youngs = args.get(0).ok_or("Missing youngs_modulus")?.as_float()?;
                let yield_str = args.get(1).ok_or("Missing yield_strength")?.as_float()?;
                let ultimate = args.get(2).ok_or("Missing ultimate_strength")?.as_float()?;
                let grain = args.get(3).ok_or("Missing grain_size")?.as_float()?;
                
                let _props = MechanicalProperties::new(youngs, yield_str, ultimate, grain);
                Ok(Value::new_string(format!("Material(E={}GPa, σ_y={}MPa)", youngs, yield_str)))
            }
            
            "mechanical_stress" => {
                let youngs = args.get(0).ok_or("Missing youngs_modulus")?.as_float()?;
                let strain = args.get(1).ok_or("Missing strain")?.as_float()?;
                
                let props = MechanicalProperties::new(youngs, 250.0, 400.0, 10.0);
                let stress = props.stress_from_strain(strain);
                Ok(Value::Float(stress))
            }
            
            "mechanical_strain" => {
                let youngs = args.get(0).ok_or("Missing youngs_modulus")?.as_float()?;
                let stress = args.get(1).ok_or("Missing stress")?.as_float()?;
                
                let props = MechanicalProperties::new(youngs, 250.0, 400.0, 10.0);
                let strain = props.strain_from_stress(stress);
                Ok(Value::Float(strain))
            }
            
            "mechanical_hall_petch" => {
                let grain_size = args.get(0).ok_or("Missing grain_size")?.as_float()?;
                let sigma_0 = args.get(1).ok_or("Missing sigma_0")?.as_float()?;
                let k = args.get(2).ok_or("Missing k")?.as_float()?;
                
                let props = MechanicalProperties::new(200.0, 250.0, 400.0, grain_size);
                let yield_str = props.hall_petch_yield(sigma_0, k);
                Ok(Value::Float(yield_str))
            }
            
            // Band structure functions
            "band_new" => {
                let band_gap = args.get(0).ok_or("Missing band_gap")?.as_float()?;
                let fermi = args.get(1).ok_or("Missing fermi_energy")?.as_float()?;
                let m_e = args.get(2).ok_or("Missing electron_mass")?.as_float()?;
                let m_h = args.get(3).ok_or("Missing hole_mass")?.as_float()?;
                
                let band = BandStructure::new(band_gap, fermi, m_e, m_h);
                Ok(Value::new_string(format!("Band(E_g={}eV, {})", band_gap, band.material_type())))
            }
            
            "band_type" => {
                let band_gap = args.get(0).ok_or("Missing band_gap")?.as_float()?;
                
                let band = BandStructure::new(band_gap, 0.0, 1.0, 1.0);
                let mat_type = band.material_type();
                Ok(Value::new_string(mat_type.to_string()))
            }
            
            "band_fermi_dirac" => {
                let fermi = args.get(0).ok_or("Missing fermi_energy")?.as_float()?;
                let energy = args.get(1).ok_or("Missing energy")?.as_float()?;
                let temperature = args.get(2).ok_or("Missing temperature")?.as_float()?;
                
                let band = BandStructure::new(1.0, fermi, 1.0, 1.0);
                let occupation = band.fermi_dirac(energy, temperature);
                Ok(Value::Float(occupation))
            }
            
            "band_carrier_density" => {
                let band_gap = args.get(0).ok_or("Missing band_gap")?.as_float()?;
                let temperature = args.get(1).ok_or("Missing temperature")?.as_float()?;
                
                let band = BandStructure::new(band_gap, band_gap / 2.0, 1.0, 1.0);
                let n_i = band.intrinsic_carrier_density(temperature);
                Ok(Value::Float(n_i))
            }
            
            "band_conductivity" => {
                let band_gap = args.get(0).ok_or("Missing band_gap")?.as_float()?;
                let temperature = args.get(1).ok_or("Missing temperature")?.as_float()?;
                
                let band = BandStructure::new(band_gap, band_gap / 2.0, 1.0, 1.0);
                let sigma = band.intrinsic_conductivity(temperature);
                Ok(Value::Float(sigma))
            }
            
            // Phase transition functions
            "phase_new" => {
                let temperature = args.get(0).ok_or("Missing temperature")?.as_float()?;
                let pressure = args.get(1).ok_or("Missing pressure")?.as_float()?;
                let enthalpy = args.get(2).ok_or("Missing enthalpy_change")?.as_float()?;
                let volume = args.get(3).ok_or("Missing volume_change")?.as_float()?;
                
                let _phase = PhaseTransition::new(
                    temperature,
                    pressure,
                    Phase::Solid,
                    Phase::Liquid,
                    enthalpy,
                    volume,
                );
                Ok(Value::new_string(format!("Phase(T={}K, P={}Pa)", temperature, pressure)))
            }
            
            "phase_slope" => {
                let temperature = args.get(0).ok_or("Missing temperature")?.as_float()?;
                let enthalpy = args.get(1).ok_or("Missing enthalpy_change")?.as_float()?;
                let volume = args.get(2).ok_or("Missing volume_change")?.as_float()?;
                
                let phase = PhaseTransition::new(
                    temperature,
                    101325.0,
                    Phase::Solid,
                    Phase::Liquid,
                    enthalpy,
                    volume,
                );
                let slope = phase.phase_boundary_slope();
                Ok(Value::Float(slope))
            }
            
            "phase_pressure" => {
                let temperature = args.get(0).ok_or("Missing temperature")?.as_float()?;
                let new_temp = args.get(1).ok_or("Missing new_temperature")?.as_float()?;
                let enthalpy = args.get(2).ok_or("Missing enthalpy_change")?.as_float()?;
                let volume = args.get(3).ok_or("Missing volume_change")?.as_float()?;
                
                let phase = PhaseTransition::new(
                    temperature,
                    101325.0,
                    Phase::Solid,
                    Phase::Liquid,
                    enthalpy,
                    volume,
                );
                let pressure = phase.pressure_at_temperature(new_temp);
                Ok(Value::Float(pressure))
            }
            
            _ => Err(format!("Unknown materials function: {}", function)),
        }
    }
}
