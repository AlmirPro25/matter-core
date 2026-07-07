//! Backend integration for Matter Cosmology

use matter_backend::{Backend, Value};
use crate::*;

pub struct CosmologyBackend;

impl Backend for CosmologyBackend {
    fn call(&mut self, function: &str, args: Vec<Value>) -> Result<Value, String> {
        match function {
            // Universe functions
            "universe_current" => {
                let universe = Universe::current();
                Ok(Value::new_string(format!("Universe(a={:.2}, H={:.1}km/s/Mpc, age={:.1}Gyr)", 
                    universe.scale_factor, universe.hubble_parameter, universe.age)))
            }
            
            "universe_at_redshift" => {
                let z = args.get(0).ok_or("Missing redshift")?.as_float()?;
                
                let universe = Universe::at_redshift(z);
                Ok(Value::new_string(format!("Universe(z={:.2}, a={:.2}, H={:.1})", 
                    z, universe.scale_factor, universe.hubble_parameter)))
            }
            
            "universe_recession_velocity" => {
                let distance = args.get(0).ok_or("Missing distance")?.as_float()?;
                
                let universe = Universe::current();
                let v = universe.recession_velocity(distance);
                Ok(Value::Float(v))
            }
            
            "universe_comoving_distance" => {
                let z = args.get(0).ok_or("Missing redshift")?.as_float()?;
                
                let universe = Universe::current();
                let d = universe.comoving_distance(z);
                Ok(Value::Float(d))
            }
            
            "universe_lookback_time" => {
                let z = args.get(0).ok_or("Missing redshift")?.as_float()?;
                
                let universe = Universe::current();
                let t = universe.lookback_time(z);
                Ok(Value::Float(t))
            }
            
            "universe_critical_density" => {
                let universe = Universe::current();
                let rho_c = universe.critical_density();
                Ok(Value::Float(rho_c))
            }
            
            // CMB functions
            "cmb_current" => {
                let cmb = CMB::current();
                Ok(Value::new_string(format!("CMB(T={:.3}K, z_rec={})", 
                    cmb.temperature, cmb.redshift as i64)))
            }
            
            "cmb_temperature" => {
                let z = args.get(0).ok_or("Missing redshift")?.as_float()?;
                
                let t = CMB::temperature_at_redshift(z);
                Ok(Value::Float(t))
            }
            
            "cmb_photon_density" => {
                let cmb = CMB::current();
                let n = cmb.photon_density();
                Ok(Value::Float(n))
            }
            
            "cmb_energy_density" => {
                let cmb = CMB::current();
                let u = cmb.energy_density();
                Ok(Value::Float(u))
            }
            
            "cmb_dipole_velocity" => {
                let delta_t = args.get(0).ok_or("Missing delta_t/t")?.as_float()?;
                
                let cmb = CMB::current();
                let v = cmb.dipole_velocity(delta_t);
                Ok(Value::Float(v))
            }
            
            // Dark matter functions
            "dark_matter_new" => {
                let dm = DarkMatter::new();
                Ok(Value::new_string(format!("DarkMatter(Ω={:.3}, M={:.1e}M☉)", 
                    dm.omega_dm, dm.halo_mass)))
            }
            
            "dark_matter_density" => {
                let dm = DarkMatter::new();
                let universe = Universe::current();
                let rho = dm.density(&universe);
                Ok(Value::Float(rho))
            }
            
            "dark_matter_virial_velocity" => {
                let mass = args.get(0).ok_or("Missing mass")?.as_float()?;
                
                let mut dm = DarkMatter::new();
                dm.halo_mass = mass;
                let v = dm.virial_velocity();
                Ok(Value::Float(v))
            }
            
            "dark_matter_nfw_density" => {
                let r = args.get(0).ok_or("Missing radius")?.as_float()?;
                let r_s = args.get(1).ok_or("Missing scale_radius")?.as_float()?;
                let rho_s = args.get(2).ok_or("Missing scale_density")?.as_float()?;
                
                let dm = DarkMatter::new();
                let rho = dm.nfw_density(r, r_s, rho_s);
                Ok(Value::Float(rho))
            }
            
            // Dark energy functions
            "dark_energy_new" => {
                let de = DarkEnergy::cosmological_constant();
                Ok(Value::new_string(format!("DarkEnergy(Ω={:.3}, w={:.1})", 
                    de.omega_lambda, de.w)))
            }
            
            "dark_energy_density" => {
                let de = DarkEnergy::cosmological_constant();
                let universe = Universe::current();
                let rho = de.density(&universe);
                Ok(Value::Float(rho))
            }
            
            "dark_energy_pressure" => {
                let de = DarkEnergy::cosmological_constant();
                let universe = Universe::current();
                let p = de.pressure(&universe);
                Ok(Value::Float(p))
            }
            
            "dark_energy_acceleration" => {
                let de = DarkEnergy::cosmological_constant();
                let universe = Universe::current();
                let a = de.acceleration(&universe);
                Ok(Value::Float(a))
            }
            
            // Composition functions
            "composition_current" => {
                let comp = Composition::current();
                Ok(Value::new_string(format!("Composition(Ω_b={:.3}, Ω_DM={:.3}, Ω_Λ={:.3})", 
                    comp.omega_baryon, comp.omega_dark_matter, comp.omega_dark_energy)))
            }
            
            "composition_total_omega" => {
                let comp = Composition::current();
                let total = comp.total_omega();
                Ok(Value::Float(total))
            }
            
            "composition_is_flat" => {
                let comp = Composition::current();
                let flat = comp.is_flat();
                Ok(Value::Bool(flat))
            }
            
            "composition_matter_radiation_eq" => {
                let comp = Composition::current();
                let z_eq = comp.matter_radiation_equality();
                Ok(Value::Float(z_eq))
            }
            
            "composition_matter_de_eq" => {
                let comp = Composition::current();
                let z_eq = comp.matter_dark_energy_equality();
                Ok(Value::Float(z_eq))
            }
            
            // Structure functions
            "structure_new" => {
                let mass = args.get(0).ok_or("Missing mass")?.as_float()?;
                let size = args.get(1).ok_or("Missing size")?.as_float()?;
                
                let _structure = Structure::new(mass, size);
                Ok(Value::new_string(format!("Structure(M={:.1e}M☉, R={}Mpc)", mass, size)))
            }
            
            "structure_jeans_mass" => {
                let temperature = args.get(0).ok_or("Missing temperature")?.as_float()?;
                let density = args.get(1).ok_or("Missing density")?.as_float()?;
                
                let m_j = Structure::jeans_mass(temperature, density);
                Ok(Value::Float(m_j))
            }
            
            "structure_free_fall_time" => {
                let density = args.get(0).ok_or("Missing density")?.as_float()?;
                
                let t_ff = Structure::free_fall_time(density);
                Ok(Value::Float(t_ff))
            }
            
            "structure_correlation" => {
                let r = args.get(0).ok_or("Missing distance")?.as_float()?;
                
                let xi = Structure::correlation_function(r);
                Ok(Value::Float(xi))
            }
            
            _ => Err(format!("Unknown cosmology function: {}", function)),
        }
    }
}
