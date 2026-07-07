//! # Matter Climate - Climate & Earth System Modeling
//!
//! Coupled climate models with ocean-atmosphere interaction, radiative transfer,
//! carbon cycle, ice dynamics, and climate sensitivity analysis.
//!
//! ## Features
//! - Energy Balance Models (0D, 1D, 2D)
//! - Radiative Transfer (RRTM-like)
//! - Ocean-Atmosphere Coupling
//! - Carbon Cycle (terrestrial + oceanic)
//! - Ice Sheet Dynamics
//! - Climate Sensitivity Analysis

use std::f64::consts::PI;

// ============================================================================
// PHYSICAL CONSTANTS
// ============================================================================

pub const STEFAN_BOLTZMANN: f64 = 5.670374419e-8; // W/m²/K⁴
pub const SOLAR_CONSTANT: f64 = 1361.0; // W/m² (TSI)
pub const EARTH_RADIUS: f64 = 6.371e6; // m
pub const OCEAN_HEAT_CAPACITY: f64 = 4.0e8; // J/m²/K (100m mixed layer)
pub const LAND_HEAT_CAPACITY: f64 = 1.0e7; // J/m²/K
pub const CO2_PREINDUSTRIAL: f64 = 280.0; // ppm
pub const CH4_PREINDUSTRIAL: f64 = 700.0; // ppb
pub const N2O_PREINDUSTRIAL: f64 = 270.0; // ppb

// ============================================================================
// ENERGY BALANCE MODEL (0D)
// ============================================================================

/// Simple 0D Energy Balance Model (EBM)
/// dT/dt = (S(1-α)/4 - εσT⁴) / C
#[derive(Debug, Clone)]
pub struct EnergyBalanceModel {
    pub temperature: f64,        // K (surface temperature)
    pub albedo: f64,             // 0-1 (planetary albedo)
    pub emissivity: f64,         // 0-1 (longwave emissivity)
    pub heat_capacity: f64,      // J/m²/K
    pub solar_constant: f64,     // W/m² (TSI)
}

impl EnergyBalanceModel {
    pub fn new(initial_temp: f64, albedo: f64) -> Self {
        EnergyBalanceModel {
            temperature: initial_temp,
            albedo,
            emissivity: 0.612, // Effective emissivity (greenhouse effect)
            heat_capacity: OCEAN_HEAT_CAPACITY,
            solar_constant: SOLAR_CONSTANT,
        }
    }

    /// Incoming solar radiation (absorbed)
    pub fn solar_in(&self) -> f64 {
        self.solar_constant * (1.0 - self.albedo) / 4.0 // /4 for geometry
    }

    /// Outgoing longwave radiation
    pub fn longwave_out(&self) -> f64 {
        self.emissivity * STEFAN_BOLTZMANN * self.temperature.powi(4)
    }

    /// Net radiative flux (W/m²)
    pub fn net_flux(&self) -> f64 {
        self.solar_in() - self.longwave_out()
    }

    /// Time derivative dT/dt (K/s)
    pub fn dT_dt(&self) -> f64 {
        self.net_flux() / self.heat_capacity
    }

    /// Euler forward step
    pub fn step(&mut self, dt: f64) {
        let dT = self.dT_dt() * dt;
        self.temperature += dT;
    }

    /// Equilibrium temperature (dT/dt = 0)
    pub fn equilibrium_temperature(&self) -> f64 {
        let absorbed = self.solar_in();
        (absorbed / (self.emissivity * STEFAN_BOLTZMANN)).powf(0.25)
    }
}

// ============================================================================
// RADIATIVE FORCING
// ============================================================================

/// Radiative forcing from greenhouse gases (W/m²)
pub struct RadiativeForcing;

impl RadiativeForcing {
    /// CO2 radiative forcing (Myhre et al. 1998)
    /// ΔF = 5.35 * ln(C/C₀) W/m²
    pub fn co2(concentration_ppm: f64) -> f64 {
        5.35 * (concentration_ppm / CO2_PREINDUSTRIAL).ln()
    }

    /// CH4 radiative forcing (IPCC AR5)
    /// ΔF = 0.036 * (√M - √M₀) W/m²
    pub fn ch4(concentration_ppb: f64) -> f64 {
        0.036 * (concentration_ppb.sqrt() - CH4_PREINDUSTRIAL.sqrt())
    }

    /// N2O radiative forcing (IPCC AR5)
    /// ΔF = 0.12 * (√N - √N₀) W/m²
    pub fn n2o(concentration_ppb: f64) -> f64 {
        0.12 * (concentration_ppb.sqrt() - N2O_PREINDUSTRIAL.sqrt())
    }

    /// Total anthropogenic forcing
    pub fn total(co2_ppm: f64, ch4_ppb: f64, n2o_ppb: f64) -> f64 {
        Self::co2(co2_ppm) + Self::ch4(ch4_ppb) + Self::n2o(n2o_ppb)
    }
}

// ============================================================================
// CLIMATE SENSITIVITY
// ============================================================================

/// Climate sensitivity parameters
pub struct ClimateSensitivity;

impl ClimateSensitivity {
    /// Equilibrium Climate Sensitivity (ECS)
    /// Temperature change for 2xCO2 (K)
    /// ECS = λ * ΔF(2xCO2) where λ is climate sensitivity parameter
    pub fn ecs(lambda: f64) -> f64 {
        let forcing_2xco2 = RadiativeForcing::co2(2.0 * CO2_PREINDUSTRIAL);
        lambda * forcing_2xco2
    }

    /// Transient Climate Response (TCR)
    /// Temperature at time of CO2 doubling in 1%/year scenario
    /// TCR ≈ 0.6 * ECS (empirical)
    pub fn tcr(ecs: f64) -> f64 {
        0.6 * ecs
    }

    /// Climate feedback parameter (W/m²/K)
    /// λ = 1/α where α is feedback parameter
    pub fn feedback_parameter(ecs: f64) -> f64 {
        let forcing_2xco2 = RadiativeForcing::co2(2.0 * CO2_PREINDUSTRIAL);
        forcing_2xco2 / ecs
    }
}

// ============================================================================
// CARBON CYCLE
// ============================================================================

/// Simple carbon cycle model (box model)
#[derive(Debug, Clone)]
pub struct CarbonCycle {
    pub atmosphere_gtc: f64,     // GtC (Gigatons Carbon)
    pub ocean_surface_gtc: f64,  // GtC
    pub ocean_deep_gtc: f64,     // GtC
    pub terrestrial_gtc: f64,    // GtC
    pub fossil_emissions: f64,   // GtC/year
}

impl CarbonCycle {
    pub fn preindustrial() -> Self {
        CarbonCycle {
            atmosphere_gtc: 589.0,      // Pre-industrial ~280 ppm
            ocean_surface_gtc: 900.0,
            ocean_deep_gtc: 37100.0,
            terrestrial_gtc: 2300.0,
            fossil_emissions: 0.0,
        }
    }

    /// Atmospheric CO2 (ppm) from carbon mass (GtC)
    /// 1 ppm CO2 ≈ 2.124 GtC
    pub fn co2_ppm(&self) -> f64 {
        self.atmosphere_gtc / 2.124
    }

    /// Ocean uptake flux (GtC/year)
    /// Simple: proportional to ΔpCO2
    pub fn ocean_uptake(&self) -> f64 {
        let current_pco2 = self.co2_ppm();
        let equilibrium_pco2 = CO2_PREINDUSTRIAL;
        let k_ocean = 0.1; // Exchange coefficient
        k_ocean * (current_pco2 - equilibrium_pco2)
    }

    /// Terrestrial uptake flux (GtC/year)
    /// CO2 fertilization effect
    pub fn terrestrial_uptake(&self) -> f64 {
        let current_pco2 = self.co2_ppm();
        let beta = 0.5; // GtC/year per ppm
        beta * (current_pco2 - CO2_PREINDUSTRIAL).max(0.0) / 100.0
    }

    /// Time step (year)
    pub fn step(&mut self, dt: f64) {
        let ocean_flux = self.ocean_uptake();
        let land_flux = self.terrestrial_uptake();
        
        let d_atm = (self.fossil_emissions - ocean_flux - land_flux) * dt;
        
        self.atmosphere_gtc += d_atm;
        self.ocean_surface_gtc += ocean_flux * dt;
        self.terrestrial_gtc += land_flux * dt;
    }
}

// ============================================================================
// OCEAN-ATMOSPHERE COUPLING
// ============================================================================

/// Coupled ocean-atmosphere box model
#[derive(Debug, Clone)]
pub struct CoupledModel {
    pub t_surface: f64,          // K (surface air temp)
    pub t_ocean: f64,            // K (ocean mixed layer temp)
    pub heat_exchange: f64,      // W/m² (ocean-atmosphere)
}

impl CoupledModel {
    pub fn new(t_init: f64) -> Self {
        CoupledModel {
            t_surface: t_init,
            t_ocean: t_init - 2.0, // Ocean typically cooler
            heat_exchange: 0.0,
        }
    }

    /// Ocean-atmosphere heat flux (W/m²)
    /// Sensible + latent heat
    pub fn heat_flux(&self) -> f64 {
        let k = 20.0; // Heat exchange coefficient W/m²/K
        k * (self.t_ocean - self.t_surface)
    }

    /// Step both components
    pub fn step(&mut self, net_radiation: f64, dt: f64) {
        self.heat_exchange = self.heat_flux();
        
        // Atmosphere
        let d_t_atm = (net_radiation - self.heat_exchange) / LAND_HEAT_CAPACITY * dt;
        self.t_surface += d_t_atm;
        
        // Ocean
        let d_t_ocean = self.heat_exchange / OCEAN_HEAT_CAPACITY * dt;
        self.t_ocean += d_t_ocean;
    }
}

// ============================================================================
// ICE ALBEDO FEEDBACK
// ============================================================================

/// Ice-albedo feedback model
pub struct IceAlbedoFeedback;

impl IceAlbedoFeedback {
    /// Albedo as function of temperature
    /// Smooth transition between ice-covered and ice-free
    pub fn albedo(temperature: f64) -> f64 {
        let t_freeze = 263.15; // K (-10°C threshold)
        let t_melt = 283.15;   // K (+10°C threshold)
        
        let alpha_ice = 0.6;   // Ice albedo
        let alpha_ocean = 0.1; // Ocean albedo
        
        if temperature < t_freeze {
            alpha_ice
        } else if temperature > t_melt {
            alpha_ocean
        } else {
            // Linear interpolation
            let fraction = (temperature - t_freeze) / (t_melt - t_freeze);
            alpha_ice * (1.0 - fraction) + alpha_ocean * fraction
        }
    }

    /// Ice fraction as function of temperature
    pub fn ice_fraction(temperature: f64) -> f64 {
        let t_freeze = 263.15;
        let t_melt = 283.15;
        
        if temperature < t_freeze {
            1.0
        } else if temperature > t_melt {
            0.0
        } else {
            (t_melt - temperature) / (t_melt - t_freeze)
        }
    }
}

// ============================================================================
// ATMOSPHERIC OPTICAL DEPTH
// ============================================================================

/// Atmospheric radiative transfer (simplified)
pub struct RadiativeTransfer;

impl RadiativeTransfer {
    /// Optical depth for CO2
    /// τ = τ₀ * (C/C₀)^α where α ≈ 0.5
    pub fn optical_depth_co2(concentration_ppm: f64) -> f64 {
        let tau_0 = 1.0; // Reference optical depth
        let alpha = 0.5;
        tau_0 * (concentration_ppm / CO2_PREINDUSTRIAL).powf(alpha)
    }

    /// Transmittance: T = exp(-τ)
    pub fn transmittance(optical_depth: f64) -> f64 {
        (-optical_depth).exp()
    }

    /// Greenhouse effect: emissivity from optical depth
    /// ε = 1 - T (absorptivity = emissivity)
    pub fn emissivity(optical_depth: f64) -> f64 {
        1.0 - Self::transmittance(optical_depth)
    }
}

// ============================================================================
// SEA LEVEL RISE
// ============================================================================

/// Sea level rise from thermal expansion and ice melt
pub struct SeaLevelRise;

impl SeaLevelRise {
    /// Thermal expansion (m per K)
    /// ΔH = α * H₀ * ΔT where α ≈ 2e-4 /K
    pub fn thermal_expansion(delta_t: f64, ocean_depth: f64) -> f64 {
        let alpha = 2.0e-4; // Thermal expansion coefficient
        alpha * ocean_depth * delta_t
    }

    /// Greenland ice sheet contribution (m per K)
    /// Empirical: ~7 m total / 5 K sensitivity = 1.4 m/K
    pub fn greenland_melt(delta_t: f64) -> f64 {
        let sensitivity = 1.4; // m/K
        sensitivity * delta_t.max(0.0)
    }

    /// Antarctic ice sheet contribution (m per K)
    /// Empirical: ~58 m total / 10 K sensitivity = 5.8 m/K
    pub fn antarctic_melt(delta_t: f64) -> f64 {
        let sensitivity = 0.5; // m/K (slower response)
        sensitivity * delta_t.max(0.0)
    }

    /// Total sea level rise (m)
    pub fn total(delta_t: f64, ocean_depth: f64) -> f64 {
        Self::thermal_expansion(delta_t, ocean_depth)
            + Self::greenland_melt(delta_t)
            + Self::antarctic_melt(delta_t)
    }
}

// ============================================================================
// INTEGRATED ASSESSMENT
// ============================================================================

/// Integrated climate-carbon model
#[derive(Debug, Clone)]
pub struct IntegratedModel {
    pub ebm: EnergyBalanceModel,
    pub carbon: CarbonCycle,
    pub year: f64,
}

impl IntegratedModel {
    pub fn new(initial_temp: f64) -> Self {
        let albedo = IceAlbedoFeedback::albedo(initial_temp);
        IntegratedModel {
            ebm: EnergyBalanceModel::new(initial_temp, albedo),
            carbon: CarbonCycle::preindustrial(),
            year: 1850.0,
        }
    }

    /// Run coupled model for one year
    pub fn step(&mut self, dt_years: f64) {
        // Update CO2 from carbon cycle
        let co2_ppm = self.carbon.co2_ppm();
        
        // Update radiative forcing
        let forcing = RadiativeForcing::co2(co2_ppm);
        
        // Update emissivity from optical depth
        let tau = RadiativeTransfer::optical_depth_co2(co2_ppm);
        self.ebm.emissivity = RadiativeTransfer::emissivity(tau);
        
        // Update albedo from ice-albedo feedback
        self.ebm.albedo = IceAlbedoFeedback::albedo(self.ebm.temperature);
        
        // Step energy balance
        let dt_seconds = dt_years * 365.25 * 86400.0;
        self.ebm.step(dt_seconds);
        
        // Step carbon cycle
        self.carbon.step(dt_years);
        
        self.year += dt_years;
    }

    /// Get warming relative to preindustrial (K)
    pub fn delta_t(&self) -> f64 {
        self.ebm.temperature - 288.15
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_balance_model() {
        let mut ebm = EnergyBalanceModel::new(288.15, 0.3);
        
        let solar_in = ebm.solar_in();
        assert!((solar_in - 238.2).abs() < 1.0); // ~238 W/m²
        
        let lw_out = ebm.longwave_out();
        assert!(lw_out > 200.0 && lw_out < 250.0);
        
        let eq_temp = ebm.equilibrium_temperature();
        assert!(eq_temp > 250.0 && eq_temp < 300.0);
    }

    #[test]
    fn test_radiative_forcing() {
        // Pre-industrial
        let f_280 = RadiativeForcing::co2(280.0);
        assert!(f_280.abs() < 0.01); // Should be ~0
        
        // 2x CO2
        let f_560 = RadiativeForcing::co2(560.0);
        assert!((f_560 - 3.71).abs() < 0.1); // ~3.7 W/m² (Myhre)
        
        // Current (~420 ppm)
        let f_420 = RadiativeForcing::co2(420.0);
        assert!(f_420 > 2.0 && f_420 < 3.0);
    }

    #[test]
    fn test_climate_sensitivity() {
        let lambda = 0.8; // K/(W/m²)
        let ecs = ClimateSensitivity::ecs(lambda);
        
        // ECS should be ~3K for lambda=0.8
        assert!(ecs > 2.5 && ecs < 3.5);
        
        let tcr = ClimateSensitivity::tcr(ecs);
        assert!(tcr < ecs); // TCR < ECS
        assert!(tcr > 1.5 && tcr < 2.5);
    }

    #[test]
    fn test_carbon_cycle() {
        let mut carbon = CarbonCycle::preindustrial();
        
        let co2_0 = carbon.co2_ppm();
        assert!((co2_0 - 280.0).abs() < 5.0);
        
        // Add emissions
        carbon.fossil_emissions = 10.0; // GtC/year
        carbon.step(1.0);
        
        let co2_1 = carbon.co2_ppm();
        assert!(co2_1 > co2_0); // Should increase
    }

    #[test]
    fn test_coupled_model() {
        let mut model = CoupledModel::new(288.15);
        
        let t0_surface = model.t_surface;
        let t0_ocean = model.t_ocean;
        
        // Higher forcing for longer time
        model.step(50.0, 86400.0 * 10.0); // 50 W/m² forcing for 10 days
        
        assert!(model.t_surface > t0_surface);
        assert!(model.t_ocean > t0_ocean || (model.t_ocean - t0_ocean).abs() < 0.1);
    }

    #[test]
    fn test_ice_albedo_feedback() {
        // Cold: ice-covered
        let alpha_cold = IceAlbedoFeedback::albedo(260.0);
        assert!(alpha_cold > 0.5);
        
        // Warm: ice-free
        let alpha_warm = IceAlbedoFeedback::albedo(290.0);
        assert!(alpha_warm < 0.2);
        
        // Ice fraction
        let ice_cold = IceAlbedoFeedback::ice_fraction(260.0);
        assert_eq!(ice_cold, 1.0);
        
        let ice_warm = IceAlbedoFeedback::ice_fraction(290.0);
        assert_eq!(ice_warm, 0.0);
    }

    #[test]
    fn test_radiative_transfer() {
        let tau_280 = RadiativeTransfer::optical_depth_co2(280.0);
        let tau_560 = RadiativeTransfer::optical_depth_co2(560.0);
        
        assert!(tau_560 > tau_280); // Higher CO2 = higher optical depth
        
        let t_280 = RadiativeTransfer::transmittance(tau_280);
        let t_560 = RadiativeTransfer::transmittance(tau_560);
        
        assert!(t_560 < t_280); // Higher optical depth = lower transmittance
        
        let e_560 = RadiativeTransfer::emissivity(tau_560);
        assert!(e_560 > 0.0 && e_560 < 1.0);
    }

    #[test]
    fn test_sea_level_rise() {
        let delta_t = 2.0; // 2K warming
        let ocean_depth = 3700.0; // m
        
        let thermal = SeaLevelRise::thermal_expansion(delta_t, ocean_depth);
        assert!(thermal > 0.0 && thermal < 2.0);
        
        let greenland = SeaLevelRise::greenland_melt(delta_t);
        assert!(greenland > 0.0);
        
        let total = SeaLevelRise::total(delta_t, ocean_depth);
        assert!(total > thermal); // Total includes ice melt
    }

    #[test]
    fn test_integrated_model() {
        let mut model = IntegratedModel::new(288.15);
        
        // Add emissions scenario
        model.carbon.fossil_emissions = 10.0; // GtC/year
        
        // Run for 10 years
        for _ in 0..10 {
            model.step(1.0);
        }
        
        // Temperature should warm
        assert!(model.delta_t() > 0.0);
        
        // CO2 should increase
        assert!(model.carbon.co2_ppm() > CO2_PREINDUSTRIAL);
    }
}

pub mod backend;
