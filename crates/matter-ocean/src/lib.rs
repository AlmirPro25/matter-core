//! # Matter Ocean
//!
//! Physical oceanography simulation for Matter Core.
//!
//! ## Features
//! - **Ocean Currents**: Surface and deep circulation
//! - **Waves**: Wind waves, tsunamis, tides
//! - **Thermohaline Circulation**: Global conveyor belt
//! - **Sea Level**: Pressure, density, temperature
//! - **Coriolis Effect**: Earth's rotation impact
//! - **Ekman Transport**: Wind-driven currents
//!
//! ## Physics Basis
//! All simulations use peer-reviewed oceanography equations:
//! - Navier-Stokes for fluid motion
//! - Coriolis parameter: f = 2Ω sin(φ)
//! - Ekman depth: D = π√(2ν/f)
//! - Wave dispersion: c = √(gλ/2π)
//! - Tsunami speed: c = √(gh)
//! - Equation of state for seawater

pub mod backend;

use std::f64::consts::PI;

// ============================================================================
// OCEAN PROPERTIES
// ============================================================================

/// Seawater properties
#[derive(Debug, Clone)]
pub struct Seawater {
    /// Temperature (°C)
    pub temperature: f64,
    /// Salinity (PSU - Practical Salinity Units)
    pub salinity: f64,
    /// Pressure (dbar)
    pub pressure: f64,
    /// Density (kg/m³)
    pub density: f64,
}

impl Seawater {
    /// Create seawater at conditions
    pub fn new(temperature: f64, salinity: f64, pressure: f64) -> Self {
        // UNESCO equation of state (simplified)
        // ρ = ρ0 + A*S + B*T + C*P
        let rho0 = 1000.0;  // Pure water density
        let a = 0.78;       // Salinity coefficient
        let b = -0.2;       // Temperature coefficient
        let c = 0.045;      // Pressure coefficient
        
        let density = rho0 + a * salinity + b * temperature + c * pressure;
        
        Self {
            temperature,
            salinity,
            pressure,
            density,
        }
    }
    
    /// Calculate sound speed (m/s)
    /// Mackenzie equation
    pub fn sound_speed(&self) -> f64 {
        let t = self.temperature;
        let s = self.salinity;
        let z = self.pressure / 10.0;  // Convert dbar to depth (m)
        
        1448.96 + 4.591 * t - 0.05304 * t * t + 0.0002374 * t * t * t
            + 0.0160 * z + (1.340 - 0.01025 * t) * (s - 35.0)
            + 1.675e-7 * z * z - 7.139e-13 * t * z * z * z
    }
    
    /// Calculate buoyancy frequency (rad/s)
    /// N² = -(g/ρ)(dρ/dz)
    pub fn buoyancy_frequency(&self, drho_dz: f64) -> f64 {
        const G: f64 = 9.81;  // m/s²
        let n_squared = -(G / self.density) * drho_dz;
        if n_squared > 0.0 {
            n_squared.sqrt()
        } else {
            0.0
        }
    }
}

// ============================================================================
// OCEAN WAVES
// ============================================================================

/// Ocean wave
#[derive(Debug, Clone)]
pub struct Wave {
    /// Wave height (m)
    pub height: f64,
    /// Wavelength (m)
    pub wavelength: f64,
    /// Period (s)
    pub period: f64,
    /// Depth (m)
    pub depth: f64,
}

impl Wave {
    /// Create wave
    pub fn new(height: f64, wavelength: f64, period: f64, depth: f64) -> Self {
        Self {
            height,
            wavelength,
            period,
            depth,
        }
    }
    
    /// Calculate wave speed (m/s)
    /// Deep water: c = √(gλ/2π)
    /// Shallow water: c = √(gh)
    pub fn speed(&self) -> f64 {
        const G: f64 = 9.81;
        
        // Check if deep or shallow water
        if self.depth > self.wavelength / 2.0 {
            // Deep water
            (G * self.wavelength / (2.0 * PI)).sqrt()
        } else {
            // Shallow water
            (G * self.depth).sqrt()
        }
    }
    
    /// Calculate wave energy per unit area (J/m²)
    /// E = (1/8)ρgH²
    pub fn energy_density(&self) -> f64 {
        const RHO: f64 = 1025.0;  // Seawater density
        const G: f64 = 9.81;
        
        0.125 * RHO * G * self.height * self.height
    }
    
    /// Calculate wave power per unit width (W/m)
    /// P = E * c_g
    /// where c_g = c/2 for deep water
    pub fn power_per_width(&self) -> f64 {
        let energy = self.energy_density();
        let speed = self.speed();
        let group_velocity = if self.depth > self.wavelength / 2.0 {
            speed / 2.0  // Deep water
        } else {
            speed  // Shallow water
        };
        
        energy * group_velocity
    }
}

/// Tsunami wave
#[derive(Debug, Clone)]
pub struct Tsunami {
    /// Initial height (m)
    pub height: f64,
    /// Ocean depth (m)
    pub depth: f64,
    /// Distance traveled (km)
    pub distance: f64,
}

impl Tsunami {
    /// Create tsunami
    pub fn new(height: f64, depth: f64) -> Self {
        Self {
            height,
            depth,
            distance: 0.0,
        }
    }
    
    /// Calculate tsunami speed (m/s)
    /// c = √(gh)
    pub fn speed(&self) -> f64 {
        const G: f64 = 9.81;
        (G * self.depth).sqrt()
    }
    
    /// Calculate travel time (hours)
    pub fn travel_time(&self, distance_km: f64) -> f64 {
        let speed_kmh = self.speed() * 3.6;  // Convert m/s to km/h
        distance_km / speed_kmh
    }
    
    /// Estimate height at shore (m)
    /// Height amplifies in shallow water
    pub fn height_at_shore(&self, shore_depth: f64) -> f64 {
        // Green's law: H ∝ d^(-1/4)
        self.height * (self.depth / shore_depth).powf(0.25)
    }
}

// ============================================================================
// OCEAN CURRENTS
// ============================================================================

/// Ocean current
#[derive(Debug, Clone)]
pub struct Current {
    /// Velocity (m/s)
    pub velocity: f64,
    /// Direction (degrees from north)
    pub direction: f64,
    /// Depth (m)
    pub depth: f64,
    /// Latitude (degrees)
    pub latitude: f64,
}

impl Current {
    /// Create current
    pub fn new(velocity: f64, direction: f64, depth: f64, latitude: f64) -> Self {
        Self {
            velocity,
            direction,
            depth,
            latitude,
        }
    }
    
    /// Calculate Coriolis parameter (1/s)
    /// f = 2Ω sin(φ)
    pub fn coriolis_parameter(&self) -> f64 {
        const OMEGA: f64 = 7.2921e-5;  // Earth's angular velocity (rad/s)
        2.0 * OMEGA * self.latitude.to_radians().sin()
    }
    
    /// Calculate Rossby radius (km)
    /// R = c/f
    pub fn rossby_radius(&self, wave_speed: f64) -> f64 {
        let f = self.coriolis_parameter().abs();
        if f > 0.0 {
            (wave_speed / f) / 1000.0  // Convert to km
        } else {
            f64::INFINITY
        }
    }
    
    /// Calculate geostrophic velocity (m/s)
    /// v_g = (g/f)(dh/dx)
    pub fn geostrophic_velocity(&self, sea_level_gradient: f64) -> f64 {
        const G: f64 = 9.81;
        let f = self.coriolis_parameter();
        
        if f.abs() > 1e-10 {
            (G / f) * sea_level_gradient
        } else {
            0.0
        }
    }
}

/// Ekman spiral (wind-driven current)
#[derive(Debug, Clone)]
pub struct EkmanSpiral {
    /// Wind stress (N/m²)
    pub wind_stress: f64,
    /// Latitude (degrees)
    pub latitude: f64,
    /// Eddy viscosity (m²/s)
    pub viscosity: f64,
}

impl EkmanSpiral {
    /// Create Ekman spiral
    pub fn new(wind_stress: f64, latitude: f64) -> Self {
        Self {
            wind_stress,
            latitude,
            viscosity: 0.01,  // Typical value
        }
    }
    
    /// Calculate Ekman depth (m)
    /// D = π√(2ν/f)
    pub fn ekman_depth(&self) -> f64 {
        const OMEGA: f64 = 7.2921e-5;
        let f = 2.0 * OMEGA * self.latitude.to_radians().sin().abs();
        
        if f > 0.0 {
            PI * (2.0 * self.viscosity / f).sqrt()
        } else {
            f64::INFINITY
        }
    }
    
    /// Calculate Ekman transport (m²/s)
    /// M = τ/f
    pub fn ekman_transport(&self) -> f64 {
        const OMEGA: f64 = 7.2921e-5;
        const RHO: f64 = 1025.0;
        let f = 2.0 * OMEGA * self.latitude.to_radians().sin();
        
        if f.abs() > 1e-10 {
            self.wind_stress / (RHO * f)
        } else {
            0.0
        }
    }
    
    /// Calculate surface velocity (m/s)
    pub fn surface_velocity(&self) -> f64 {
        const RHO: f64 = 1025.0;
        let depth = self.ekman_depth();
        
        if depth > 0.0 && depth.is_finite() {
            self.wind_stress / (RHO * depth)
        } else {
            0.0
        }
    }
}

// ============================================================================
// TIDES
// ============================================================================

/// Tidal component
#[derive(Debug, Clone)]
pub struct Tide {
    /// Amplitude (m)
    pub amplitude: f64,
    /// Period (hours)
    pub period: f64,
    /// Phase (degrees)
    pub phase: f64,
}

impl Tide {
    /// Create tide
    pub fn new(amplitude: f64, period: f64, phase: f64) -> Self {
        Self {
            amplitude,
            period,
            phase,
        }
    }
    
    /// Calculate tidal height at time (m)
    pub fn height_at_time(&self, time_hours: f64) -> f64 {
        let omega = 2.0 * PI / self.period;
        let phase_rad = self.phase.to_radians();
        self.amplitude * (omega * time_hours + phase_rad).cos()
    }
    
    /// Calculate tidal current velocity (m/s)
    /// v = (g/h) * A * sin(ωt)
    pub fn current_velocity(&self, time_hours: f64, depth: f64) -> f64 {
        const G: f64 = 9.81;
        let omega = 2.0 * PI / self.period;
        let phase_rad = self.phase.to_radians();
        
        (G / depth) * self.amplitude * (omega * time_hours + phase_rad).sin()
    }
}

/// Tidal system (multiple components)
#[derive(Debug, Clone)]
pub struct TidalSystem {
    /// M2 (principal lunar semidiurnal)
    pub m2: Tide,
    /// S2 (principal solar semidiurnal)
    pub s2: Tide,
    /// K1 (lunisolar diurnal)
    pub k1: Tide,
}

impl TidalSystem {
    /// Create typical tidal system
    pub fn new() -> Self {
        Self {
            m2: Tide::new(0.5, 12.42, 0.0),   // M2: 12.42 hours
            s2: Tide::new(0.2, 12.0, 0.0),    // S2: 12 hours
            k1: Tide::new(0.3, 23.93, 0.0),   // K1: 23.93 hours
        }
    }
    
    /// Calculate total tidal height (m)
    pub fn total_height(&self, time_hours: f64) -> f64 {
        self.m2.height_at_time(time_hours)
            + self.s2.height_at_time(time_hours)
            + self.k1.height_at_time(time_hours)
    }
    
    /// Calculate tidal range (m)
    pub fn tidal_range(&self) -> f64 {
        2.0 * (self.m2.amplitude + self.s2.amplitude + self.k1.amplitude)
    }
}

impl Default for TidalSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_seawater() {
        let water = Seawater::new(15.0, 35.0, 100.0);
        
        // Density should be around 1025 kg/m³
        assert!(water.density > 1020.0 && water.density < 1030.0);
        
        // Sound speed should be around 1500 m/s
        let c = water.sound_speed();
        assert!(c > 1450.0 && c < 1550.0);
    }
    
    #[test]
    fn test_wave() {
        let wave = Wave::new(2.0, 100.0, 8.0, 1000.0);
        
        // Wave speed should be positive
        let speed = wave.speed();
        assert!(speed > 0.0);
        
        // Energy should be positive
        let energy = wave.energy_density();
        assert!(energy > 0.0);
        
        // Power should be positive
        let power = wave.power_per_width();
        assert!(power > 0.0);
    }
    
    #[test]
    fn test_tsunami() {
        let tsunami = Tsunami::new(1.0, 4000.0);
        
        // Tsunami speed should be around 200 m/s in deep ocean
        let speed = tsunami.speed();
        assert!(speed > 190.0 && speed < 210.0);
        
        // Travel time should be reasonable
        let time = tsunami.travel_time(1000.0);
        assert!(time > 0.0);
        
        // Height should amplify at shore
        let h_shore = tsunami.height_at_shore(10.0);
        assert!(h_shore > tsunami.height);
    }
    
    #[test]
    fn test_current() {
        let current = Current::new(0.5, 90.0, 100.0, 45.0);
        
        // Coriolis parameter should be positive in NH
        let f = current.coriolis_parameter();
        assert!(f > 0.0);
        
        // Rossby radius should be positive
        let r = current.rossby_radius(2.0);
        assert!(r > 0.0);
    }
    
    #[test]
    fn test_ekman() {
        let ekman = EkmanSpiral::new(0.1, 45.0);
        
        // Ekman depth should be positive
        let depth = ekman.ekman_depth();
        assert!(depth > 0.0 && depth < 1000.0);
        
        // Transport should be reasonable
        let transport = ekman.ekman_transport();
        assert!(transport.abs() > 0.0);
        
        // Surface velocity should be positive
        let v = ekman.surface_velocity();
        assert!(v > 0.0);
    }
    
    #[test]
    fn test_tide() {
        let tide = Tide::new(1.0, 12.42, 0.0);
        
        // Height at t=0 should be amplitude
        let h0 = tide.height_at_time(0.0);
        assert!((h0 - 1.0).abs() < 0.01);
        
        // Height at half period should be -amplitude
        let h_half = tide.height_at_time(6.21);
        assert!((h_half + 1.0).abs() < 0.1);
    }
    
    #[test]
    fn test_tidal_system() {
        let system = TidalSystem::new();
        
        // Total height should vary
        let h1 = system.total_height(0.0);
        let h2 = system.total_height(6.0);
        assert!(h1 != h2);
        
        // Tidal range should be positive
        let range = system.tidal_range();
        assert!(range > 0.0);
    }
}
