//! # Matter Atmosphere
//!
//! Atmospheric science and meteorology simulation for Matter Core.
//!
//! ## Features
//! - **Thermodynamics**: Temperature, pressure, humidity
//! - **Cloud Physics**: Formation, precipitation
//! - **Wind Dynamics**: Pressure gradients, Coriolis
//! - **Radiation**: Solar, terrestrial, greenhouse effect
//! - **Weather Systems**: Fronts, cyclones, anticyclones
//! - **Climate**: Energy balance, feedback loops
//!
//! ## Physics Basis
//! All simulations use peer-reviewed atmospheric physics:
//! - Ideal gas law: PV = nRT
//! - Clausius-Clapeyron equation
//! - Hydrostatic balance: dP/dz = -ρg
//! - Geostrophic wind: V_g = (1/ρf)∇P
//! - Stefan-Boltzmann law: E = σT⁴
//! - Radiative transfer equations

pub mod backend;

// ============================================================================
// ATMOSPHERIC PROPERTIES
// ============================================================================

/// Atmospheric layer
#[derive(Debug, Clone)]
pub struct Atmosphere {
    /// Temperature (K)
    pub temperature: f64,
    /// Pressure (Pa)
    pub pressure: f64,
    /// Density (kg/m³)
    pub density: f64,
    /// Altitude (m)
    pub altitude: f64,
}

impl Atmosphere {
    /// Create standard atmosphere at altitude
    pub fn at_altitude(altitude: f64) -> Self {
        // International Standard Atmosphere (ISA)
        let t0 = 288.15;  // Sea level temperature (K)
        let p0 = 101325.0;  // Sea level pressure (Pa)
        let lapse_rate = 0.0065;  // K/m
        const G: f64 = 9.80665;  // m/s²
        const R: f64 = 287.05;  // J/(kg·K) for dry air
        
        // Temperature at altitude
        let temperature = if altitude < 11000.0 {
            t0 - lapse_rate * altitude
        } else {
            // Stratosphere (isothermal)
            216.65
        };
        
        // Pressure at altitude (barometric formula)
        let pressure = if altitude < 11000.0 {
            p0 * (temperature / t0).powf(G / (R * lapse_rate))
        } else {
            22632.0 * (-G * (altitude - 11000.0) / (R * 216.65)).exp()
        };
        
        // Density from ideal gas law
        let density = pressure / (R * temperature);
        
        Self {
            temperature,
            pressure,
            density,
            altitude,
        }
    }
    
    /// Calculate scale height (m)
    /// H = RT/g
    pub fn scale_height(&self) -> f64 {
        const R: f64 = 287.05;
        const G: f64 = 9.80665;
        
        R * self.temperature / G
    }
    
    /// Calculate speed of sound (m/s)
    /// c = √(γRT)
    pub fn sound_speed(&self) -> f64 {
        const GAMMA: f64 = 1.4;  // Ratio of specific heats
        const R: f64 = 287.05;
        
        (GAMMA * R * self.temperature).sqrt()
    }
}

// ============================================================================
// HUMIDITY AND CLOUDS
// ============================================================================

/// Humidity and water vapor
#[derive(Debug, Clone)]
pub struct Humidity {
    /// Temperature (K)
    pub temperature: f64,
    /// Pressure (Pa)
    pub pressure: f64,
    /// Relative humidity (0-1)
    pub relative_humidity: f64,
}

impl Humidity {
    /// Create humidity state
    pub fn new(temperature: f64, pressure: f64, relative_humidity: f64) -> Self {
        Self {
            temperature,
            pressure,
            relative_humidity,
        }
    }
    
    /// Calculate saturation vapor pressure (Pa)
    /// Clausius-Clapeyron equation (simplified)
    pub fn saturation_vapor_pressure(&self) -> f64 {
        // August-Roche-Magnus formula
        let t_c = self.temperature - 273.15;  // Convert to Celsius
        611.2 * (17.67 * t_c / (t_c + 243.5)).exp()
    }
    
    /// Calculate actual vapor pressure (Pa)
    pub fn vapor_pressure(&self) -> f64 {
        self.saturation_vapor_pressure() * self.relative_humidity
    }
    
    /// Calculate dew point (K)
    pub fn dew_point(&self) -> f64 {
        let e = self.vapor_pressure();
        let ln_e = (e / 611.2).ln();
        
        // Inverse of August-Roche-Magnus
        let t_d = 243.5 * ln_e / (17.67 - ln_e);
        t_d + 273.15  // Convert to Kelvin
    }
    
    /// Calculate mixing ratio (kg/kg)
    pub fn mixing_ratio(&self) -> f64 {
        const EPSILON: f64 = 0.622;  // Ratio of molecular weights
        let e = self.vapor_pressure();
        
        EPSILON * e / (self.pressure - e)
    }
    
    /// Check if condensation occurs
    pub fn is_saturated(&self) -> bool {
        self.relative_humidity >= 1.0
    }
}

// ============================================================================
// WIND AND CIRCULATION
// ============================================================================

/// Wind vector
#[derive(Debug, Clone)]
pub struct Wind {
    /// U component (m/s, eastward)
    pub u: f64,
    /// V component (m/s, northward)
    pub v: f64,
    /// Latitude (degrees)
    pub latitude: f64,
}

impl Wind {
    /// Create wind
    pub fn new(u: f64, v: f64, latitude: f64) -> Self {
        Self { u, v, latitude }
    }
    
    /// Calculate wind speed (m/s)
    pub fn speed(&self) -> f64 {
        (self.u * self.u + self.v * self.v).sqrt()
    }
    
    /// Calculate wind direction (degrees from north)
    pub fn direction(&self) -> f64 {
        let dir = self.v.atan2(self.u).to_degrees();
        (90.0 - dir + 360.0) % 360.0
    }
    
    /// Calculate Coriolis parameter (1/s)
    pub fn coriolis_parameter(&self) -> f64 {
        const OMEGA: f64 = 7.2921e-5;  // Earth's angular velocity
        2.0 * OMEGA * self.latitude.to_radians().sin()
    }
    
    /// Calculate geostrophic wind from pressure gradient
    /// V_g = (1/ρf)∇P
    pub fn geostrophic_from_gradient(pressure_gradient: f64, density: f64, latitude: f64) -> Self {
        const OMEGA: f64 = 7.2921e-5;
        let f = 2.0 * OMEGA * latitude.to_radians().sin();
        
        if f.abs() > 1e-10 {
            let v_g = pressure_gradient / (density * f);
            Self::new(v_g, 0.0, latitude)
        } else {
            Self::new(0.0, 0.0, latitude)
        }
    }
}

// ============================================================================
// RADIATION
// ============================================================================

/// Radiation balance
#[derive(Debug, Clone)]
pub struct Radiation {
    /// Solar constant (W/m²)
    pub solar_constant: f64,
    /// Albedo (0-1)
    pub albedo: f64,
    /// Emissivity (0-1)
    pub emissivity: f64,
    /// Temperature (K)
    pub temperature: f64,
}

impl Radiation {
    /// Create radiation balance
    pub fn new(albedo: f64, emissivity: f64) -> Self {
        Self {
            solar_constant: 1361.0,  // W/m² at Earth's distance
            albedo,
            emissivity,
            temperature: 288.0,  // Initial guess
        }
    }
    
    /// Calculate incoming solar radiation (W/m²)
    /// Averaged over sphere: S/4
    pub fn incoming_solar(&self) -> f64 {
        self.solar_constant / 4.0
    }
    
    /// Calculate absorbed solar radiation (W/m²)
    pub fn absorbed_solar(&self) -> f64 {
        self.incoming_solar() * (1.0 - self.albedo)
    }
    
    /// Calculate outgoing terrestrial radiation (W/m²)
    /// Stefan-Boltzmann: E = εσT⁴
    pub fn outgoing_terrestrial(&self) -> f64 {
        const SIGMA: f64 = 5.67e-8;  // Stefan-Boltzmann constant
        self.emissivity * SIGMA * self.temperature.powi(4)
    }
    
    /// Calculate equilibrium temperature (K)
    /// Balance: absorbed solar = outgoing terrestrial
    pub fn equilibrium_temperature(&self) -> f64 {
        const SIGMA: f64 = 5.67e-8;
        let absorbed = self.absorbed_solar();
        
        (absorbed / (self.emissivity * SIGMA)).powf(0.25)
    }
    
    /// Calculate greenhouse effect (K)
    /// Difference between surface and effective temperature
    pub fn greenhouse_effect(&self) -> f64 {
        let t_eff = self.equilibrium_temperature();
        self.temperature - t_eff
    }
}

// ============================================================================
// WEATHER SYSTEMS
// ============================================================================

/// Pressure system
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PressureSystem {
    Cyclone,      // Low pressure
    Anticyclone,  // High pressure
}

impl PressureSystem {
    /// Get typical pressure (Pa)
    pub fn typical_pressure(&self) -> f64 {
        match self {
            PressureSystem::Cyclone => 98000.0,      // 980 mb
            PressureSystem::Anticyclone => 103000.0, // 1030 mb
        }
    }
    
    /// Get rotation direction in Northern Hemisphere
    pub fn rotation_nh(&self) -> &str {
        match self {
            PressureSystem::Cyclone => "counterclockwise",
            PressureSystem::Anticyclone => "clockwise",
        }
    }
    
    /// Get typical weather
    pub fn typical_weather(&self) -> &str {
        match self {
            PressureSystem::Cyclone => "cloudy, rainy, stormy",
            PressureSystem::Anticyclone => "clear, dry, calm",
        }
    }
}

/// Storm system
#[derive(Debug, Clone)]
pub struct Storm {
    /// Central pressure (Pa)
    pub central_pressure: f64,
    /// Maximum wind speed (m/s)
    pub max_wind: f64,
    /// Radius (km)
    pub radius: f64,
}

impl Storm {
    /// Create storm
    pub fn new(central_pressure: f64, max_wind: f64, radius: f64) -> Self {
        Self {
            central_pressure,
            max_wind,
            radius,
        }
    }
    
    /// Calculate pressure deficit (Pa)
    pub fn pressure_deficit(&self) -> f64 {
        101325.0 - self.central_pressure
    }
    
    /// Classify storm intensity (Saffir-Simpson for hurricanes)
    pub fn intensity_category(&self) -> u8 {
        let wind_kmh = self.max_wind * 3.6;
        
        if wind_kmh < 119.0 {
            0  // Tropical storm
        } else if wind_kmh < 153.0 {
            1  // Category 1
        } else if wind_kmh < 178.0 {
            2  // Category 2
        } else if wind_kmh < 209.0 {
            3  // Category 3
        } else if wind_kmh < 251.0 {
            4  // Category 4
        } else {
            5  // Category 5
        }
    }
    
    /// Estimate storm energy (J)
    pub fn kinetic_energy(&self) -> f64 {
        const RHO: f64 = 1.2;  // Air density
        let volume = std::f64::consts::PI * (self.radius * 1000.0).powi(2) * 10000.0;  // Rough estimate
        
        0.5 * RHO * volume * self.max_wind * self.max_wind
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_atmosphere() {
        let atm_sea = Atmosphere::at_altitude(0.0);
        let atm_10km = Atmosphere::at_altitude(10000.0);
        
        // Pressure decreases with altitude
        assert!(atm_10km.pressure < atm_sea.pressure);
        
        // Temperature decreases with altitude (troposphere)
        assert!(atm_10km.temperature < atm_sea.temperature);
        
        // Sound speed should be reasonable
        let c = atm_sea.sound_speed();
        assert!(c > 300.0 && c < 350.0);
    }
    
    #[test]
    fn test_humidity() {
        let humidity = Humidity::new(293.15, 101325.0, 0.6);
        
        // Saturation vapor pressure should be positive
        let e_sat = humidity.saturation_vapor_pressure();
        assert!(e_sat > 0.0);
        
        // Actual vapor pressure should be less than saturation
        let e = humidity.vapor_pressure();
        assert!(e < e_sat);
        
        // Dew point should be less than temperature
        let t_d = humidity.dew_point();
        assert!(t_d < humidity.temperature);
        
        // Should not be saturated at 60% RH
        assert!(!humidity.is_saturated());
    }
    
    #[test]
    fn test_wind() {
        let wind = Wind::new(10.0, 5.0, 45.0);
        
        // Speed should be positive
        let speed = wind.speed();
        assert!(speed > 0.0);
        
        // Direction should be in [0, 360)
        let dir = wind.direction();
        assert!(dir >= 0.0 && dir < 360.0);
        
        // Coriolis parameter should be positive in NH
        let f = wind.coriolis_parameter();
        assert!(f > 0.0);
    }
    
    #[test]
    fn test_radiation() {
        let rad = Radiation::new(0.3, 0.9);
        
        // Absorbed solar should be less than incoming
        let incoming = rad.incoming_solar();
        let absorbed = rad.absorbed_solar();
        assert!(absorbed < incoming);
        
        // Equilibrium temperature should be reasonable
        let t_eq = rad.equilibrium_temperature();
        assert!(t_eq > 200.0 && t_eq < 300.0);
        
        // Greenhouse effect should be positive
        let greenhouse = rad.greenhouse_effect();
        assert!(greenhouse > 0.0);
    }
    
    #[test]
    fn test_pressure_system() {
        let cyclone = PressureSystem::Cyclone;
        let anticyclone = PressureSystem::Anticyclone;
        
        // Cyclone has lower pressure
        assert!(cyclone.typical_pressure() < anticyclone.typical_pressure());
        
        // Different rotation directions
        assert_ne!(cyclone.rotation_nh(), anticyclone.rotation_nh());
    }
    
    #[test]
    fn test_storm() {
        let storm = Storm::new(95000.0, 50.0, 100.0);
        
        // Pressure deficit should be positive
        let deficit = storm.pressure_deficit();
        assert!(deficit > 0.0);
        
        // Should be classified as hurricane
        let category = storm.intensity_category();
        assert!(category >= 1);
        
        // Energy should be positive
        let energy = storm.kinetic_energy();
        assert!(energy > 0.0);
    }
}
