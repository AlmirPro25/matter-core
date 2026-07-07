//! Backend integration for Matter Climate

use matter_backend::{Backend, Value};
use crate::*;

pub struct ClimateBackend;

impl Backend for ClimateBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            // Energy Balance Model
            "ebm_create" => {
                let temp = args[0].as_float()?;
                let albedo = args[1].as_float()?;
                let ebm = EnergyBalanceModel::new(temp, albedo);
                Ok(Value::new_string(format!("EBM created: T={:.2}K, α={:.2}", ebm.temperature, ebm.albedo)))
            }

            "ebm_solar_in" => {
                let temp = args[0].as_float()?;
                let albedo = args[1].as_float()?;
                let ebm = EnergyBalanceModel::new(temp, albedo);
                Ok(Value::Float(ebm.solar_in()))
            }

            "ebm_equilibrium" => {
                let temp = args[0].as_float()?;
                let albedo = args[1].as_float()?;
                let ebm = EnergyBalanceModel::new(temp, albedo);
                Ok(Value::Float(ebm.equilibrium_temperature()))
            }

            // Radiative Forcing
            "forcing_co2" => {
                let co2_ppm = args[0].as_float()?;
                Ok(Value::Float(RadiativeForcing::co2(co2_ppm)))
            }

            "forcing_ch4" => {
                let ch4_ppb = args[0].as_float()?;
                Ok(Value::Float(RadiativeForcing::ch4(ch4_ppb)))
            }

            "forcing_n2o" => {
                let n2o_ppb = args[0].as_float()?;
                Ok(Value::Float(RadiativeForcing::n2o(n2o_ppb)))
            }

            "forcing_total" => {
                let co2 = args[0].as_float()?;
                let ch4 = args[1].as_float()?;
                let n2o = args[2].as_float()?;
                Ok(Value::Float(RadiativeForcing::total(co2, ch4, n2o)))
            }

            // Climate Sensitivity
            "ecs" => {
                let lambda = args[0].as_float()?;
                Ok(Value::Float(ClimateSensitivity::ecs(lambda)))
            }

            "tcr" => {
                let ecs = args[0].as_float()?;
                Ok(Value::Float(ClimateSensitivity::tcr(ecs)))
            }

            // Carbon Cycle
            "carbon_create" => {
                let carbon = CarbonCycle::preindustrial();
                Ok(Value::new_string(format!("Carbon cycle: {:.0} GtC atmosphere", carbon.atmosphere_gtc)))
            }

            "carbon_co2_ppm" => {
                let gtc = args[0].as_float()?;
                let co2_ppm = gtc / 2.124;
                Ok(Value::Float(co2_ppm))
            }

            // Ice-Albedo Feedback
            "ice_albedo" => {
                let temp = args[0].as_float()?;
                Ok(Value::Float(IceAlbedoFeedback::albedo(temp)))
            }

            "ice_fraction" => {
                let temp = args[0].as_float()?;
                Ok(Value::Float(IceAlbedoFeedback::ice_fraction(temp)))
            }

            // Radiative Transfer
            "optical_depth" => {
                let co2_ppm = args[0].as_float()?;
                Ok(Value::Float(RadiativeTransfer::optical_depth_co2(co2_ppm)))
            }

            "emissivity" => {
                let tau = args[0].as_float()?;
                Ok(Value::Float(RadiativeTransfer::emissivity(tau)))
            }

            // Sea Level Rise
            "slr_thermal" => {
                let delta_t = args[0].as_float()?;
                let depth = args[1].as_float()?;
                Ok(Value::Float(SeaLevelRise::thermal_expansion(delta_t, depth)))
            }

            "slr_total" => {
                let delta_t = args[0].as_float()?;
                let depth = args[1].as_float()?;
                Ok(Value::Float(SeaLevelRise::total(delta_t, depth)))
            }

            // Integrated Model
            "integrated_create" => {
                let temp = args[0].as_float()?;
                let model = IntegratedModel::new(temp);
                Ok(Value::new_string(format!("Integrated model created at {:.2}K", model.ebm.temperature)))
            }

            _ => Err(format!("Unknown climate operation: {}", method)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebm_create() {
        let mut backend = ClimateBackend;
        let args = vec![Value::Float(288.15), Value::Float(0.3)];
        let result = backend.call("ebm_create", args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_forcing_co2() {
        let mut backend = ClimateBackend;
        let args = vec![Value::Float(560.0)];
        let result = backend.call("forcing_co2", args);
        assert!(result.is_ok());
        if let Ok(Value::Float(f)) = result {
            assert!(f > 3.0 && f < 4.0); // ~3.7 W/m²
        }
    }

    #[test]
    fn test_ecs() {
        let mut backend = ClimateBackend;
        let args = vec![Value::Float(0.8)];
        let result = backend.call("ecs", args);
        assert!(result.is_ok());
        if let Ok(Value::Float(ecs)) = result {
            assert!(ecs > 2.0 && ecs < 4.0);
        }
    }
}
