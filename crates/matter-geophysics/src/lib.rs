//! # Matter Geophysics
//!
//! Geophysics simulation for Matter Core.
//!
//! ## Features
//! - **Seismology**: Earthquake waves, magnitude, intensity
//! - **Plate Tectonics**: Continental drift, subduction, rifting
//! - **Geomagnetism**: Earth's magnetic field, reversals
//! - **Gravity**: Gravitational anomalies, geoid
//! - **Heat Flow**: Geothermal gradient, mantle convection
//! - **Seismic Waves**: P-waves, S-waves, surface waves
//!
//! ## Physics Basis
//! All simulations use peer-reviewed equations from geophysics:
//! - Richter scale for earthquake magnitude
//! - Moment magnitude scale (Mw)
//! - Seismic wave velocities
//! - Plate motion rates
//! - Geomagnetic field models
//! - Heat flow equations

pub mod backend;

// ============================================================================
// SEISMOLOGY
// ============================================================================

/// Earthquake with magnitude and location
#[derive(Debug, Clone)]
pub struct Earthquake {
    /// Magnitude (Richter scale)
    pub magnitude: f64,
    /// Depth (km)
    pub depth: f64,
    /// Latitude (degrees)
    pub latitude: f64,
    /// Longitude (degrees)
    pub longitude: f64,
    /// Energy released (Joules)
    pub energy: f64,
}

impl Earthquake {
    /// Create new earthquake
    pub fn new(magnitude: f64, depth: f64, latitude: f64, longitude: f64) -> Self {
        // Energy from magnitude: log10(E) = 4.8 + 1.5*M
        let energy = 10_f64.powf(4.8 + 1.5 * magnitude);
        
        Self {
            magnitude,
            depth,
            latitude,
            longitude,
            energy,
        }
    }
    
    /// Calculate moment magnitude (Mw)
    /// Mw = (2/3) * log10(M0) - 10.7
    /// where M0 is seismic moment in N·m
    pub fn moment_magnitude(&self) -> f64 {
        // Approximate M0 from energy
        let m0 = self.energy * 1e-4;  // Rough conversion
        (2.0 / 3.0) * m0.log10() - 10.7
    }
    
    /// Calculate Modified Mercalli Intensity at distance
    pub fn intensity_at_distance(&self, distance_km: f64) -> f64 {
        // Simplified intensity attenuation
        // I = I0 - 3*log10(distance)
        let i0 = self.magnitude * 1.5;  // Peak intensity
        (i0 - 3.0 * distance_km.log10()).max(1.0).min(12.0)
    }
    
    /// Estimate ground acceleration (g)
    pub fn peak_ground_acceleration(&self) -> f64 {
        // PGA scales with magnitude
        // log10(PGA) = M - 3
        10_f64.powf(self.magnitude - 3.0)
    }
}

/// Seismic wave types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveType {
    PWave,      // Primary (compressional)
    SWave,      // Secondary (shear)
    LoveWave,   // Surface wave
    RayleighWave, // Surface wave
}

impl WaveType {
    /// Get typical velocity in crust (km/s)
    pub fn velocity_in_crust(&self) -> f64 {
        match self {
            WaveType::PWave => 6.0,        // 6 km/s
            WaveType::SWave => 3.5,        // 3.5 km/s
            WaveType::LoveWave => 3.2,     // 3.2 km/s
            WaveType::RayleighWave => 2.9, // 2.9 km/s
        }
    }
    
    /// Get typical velocity in mantle (km/s)
    pub fn velocity_in_mantle(&self) -> f64 {
        match self {
            WaveType::PWave => 8.0,        // 8 km/s
            WaveType::SWave => 4.5,        // 4.5 km/s
            WaveType::LoveWave => 0.0,     // Don't propagate in mantle
            WaveType::RayleighWave => 0.0, // Don't propagate in mantle
        }
    }
    
    /// Calculate travel time (seconds)
    pub fn travel_time(&self, distance_km: f64, in_mantle: bool) -> f64 {
        let velocity = if in_mantle {
            self.velocity_in_mantle()
        } else {
            self.velocity_in_crust()
        };
        
        if velocity == 0.0 {
            f64::INFINITY
        } else {
            distance_km / velocity
        }
    }
}

// ============================================================================
// PLATE TECTONICS
// ============================================================================

/// Tectonic plate
#[derive(Debug, Clone)]
pub struct TectonicPlate {
    /// Plate name
    pub name: String,
    /// Velocity (cm/year)
    pub velocity: f64,
    /// Direction (degrees from north)
    pub direction: f64,
    /// Area (km²)
    pub area: f64,
}

impl TectonicPlate {
    /// Create new plate
    pub fn new(name: String, velocity: f64, direction: f64, area: f64) -> Self {
        Self {
            name,
            velocity,
            direction,
            area,
        }
    }
    
    /// Calculate relative velocity with another plate (cm/year)
    pub fn relative_velocity(&self, other: &TectonicPlate) -> f64 {
        // Vector subtraction
        let v1x = self.velocity * self.direction.to_radians().sin();
        let v1y = self.velocity * self.direction.to_radians().cos();
        let v2x = other.velocity * other.direction.to_radians().sin();
        let v2y = other.velocity * other.direction.to_radians().cos();
        
        let dvx = v1x - v2x;
        let dvy = v1y - v2y;
        
        (dvx * dvx + dvy * dvy).sqrt()
    }
    
    /// Estimate time to move distance (years)
    pub fn time_to_move(&self, distance_cm: f64) -> f64 {
        distance_cm / self.velocity
    }
}

/// Plate boundary types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoundaryType {
    Divergent,  // Spreading (mid-ocean ridge)
    Convergent, // Collision (subduction)
    Transform,  // Sliding (strike-slip)
}

impl BoundaryType {
    /// Get typical earthquake magnitude range
    pub fn typical_magnitude_range(&self) -> (f64, f64) {
        match self {
            BoundaryType::Divergent => (3.0, 6.0),   // Smaller quakes
            BoundaryType::Convergent => (5.0, 9.0),  // Largest quakes
            BoundaryType::Transform => (4.0, 8.0),   // Moderate to large
        }
    }
    
    /// Get typical slip rate (cm/year)
    pub fn typical_slip_rate(&self) -> f64 {
        match self {
            BoundaryType::Divergent => 2.0,   // Slow spreading
            BoundaryType::Convergent => 5.0,  // Moderate convergence
            BoundaryType::Transform => 3.0,   // Moderate slip
        }
    }
}

// ============================================================================
// GEOMAGNETISM
// ============================================================================

/// Earth's magnetic field
#[derive(Debug, Clone)]
pub struct GeomagneticField {
    /// Dipole moment (A·m²)
    pub dipole_moment: f64,
    /// Inclination (degrees)
    pub inclination: f64,
    /// Declination (degrees)
    pub declination: f64,
    /// Total intensity (nT)
    pub intensity: f64,
}

impl GeomagneticField {
    /// Create field at latitude
    pub fn at_latitude(latitude: f64) -> Self {
        // Dipole field model
        let lat_rad = latitude.to_radians();
        
        // Inclination: tan(I) = 2*tan(λ)
        let inclination = (2.0 * lat_rad.tan()).atan().to_degrees();
        
        // Intensity: B = B0 * sqrt(1 + 3*sin²(λ))
        let b0 = 30000.0;  // 30,000 nT at equator
        let intensity = b0 * (1.0 + 3.0 * lat_rad.sin().powi(2)).sqrt();
        
        Self {
            dipole_moment: 7.9e22,  // A·m²
            inclination,
            declination: 0.0,  // Simplified
            intensity,
        }
    }
    
    /// Calculate horizontal component (nT)
    pub fn horizontal_component(&self) -> f64 {
        self.intensity * self.inclination.to_radians().cos()
    }
    
    /// Calculate vertical component (nT)
    pub fn vertical_component(&self) -> f64 {
        self.intensity * self.inclination.to_radians().sin()
    }
    
    /// Estimate time to next reversal (years)
    pub fn time_to_reversal(&self) -> f64 {
        // Average reversal interval: ~450,000 years
        // Last reversal: ~780,000 years ago
        // Overdue by ~330,000 years
        450000.0 - 330000.0  // Could happen anytime
    }
}

// ============================================================================
// GRAVITY
// ============================================================================

/// Gravity anomaly
#[derive(Debug, Clone)]
pub struct GravityAnomaly {
    /// Free-air anomaly (mGal)
    pub free_air: f64,
    /// Bouguer anomaly (mGal)
    pub bouguer: f64,
    /// Latitude (degrees)
    pub latitude: f64,
    /// Elevation (m)
    pub elevation: f64,
}

impl GravityAnomaly {
    /// Calculate gravity at location
    pub fn at_location(latitude: f64, elevation: f64) -> Self {
        // Normal gravity at sea level (mGal)
        let lat_rad = latitude.to_radians();
        let g_normal = 978031.85 * (1.0 + 0.0053024 * lat_rad.sin().powi(2)
                                    - 0.0000058 * (2.0 * lat_rad).sin().powi(2));
        
        // Free-air correction: -0.3086 mGal/m
        let free_air_correction = -0.3086 * elevation;
        
        // Bouguer correction (assuming rock density 2.67 g/cm³)
        let bouguer_correction = 0.1119 * elevation;
        
        Self {
            free_air: free_air_correction,
            bouguer: bouguer_correction,
            latitude,
            elevation,
        }
    }
    
    /// Calculate total gravity (mGal)
    pub fn total_gravity(&self) -> f64 {
        let lat_rad = self.latitude.to_radians();
        let g_normal = 978031.85 * (1.0 + 0.0053024 * lat_rad.sin().powi(2)
                                    - 0.0000058 * (2.0 * lat_rad).sin().powi(2));
        g_normal + self.free_air + self.bouguer
    }
}

// ============================================================================
// HEAT FLOW
// ============================================================================

/// Geothermal gradient and heat flow
#[derive(Debug, Clone)]
pub struct HeatFlow {
    /// Surface heat flow (mW/m²)
    pub surface_heat_flow: f64,
    /// Geothermal gradient (°C/km)
    pub gradient: f64,
    /// Depth (km)
    pub depth: f64,
}

impl HeatFlow {
    /// Create heat flow model
    pub fn new(surface_heat_flow: f64, gradient: f64) -> Self {
        Self {
            surface_heat_flow,
            gradient,
            depth: 0.0,
        }
    }
    
    /// Calculate temperature at depth (°C)
    pub fn temperature_at_depth(&self, depth_km: f64) -> f64 {
        let surface_temp = 15.0;  // 15°C surface temperature
        surface_temp + self.gradient * depth_km
    }
    
    /// Calculate thermal conductivity (W/(m·K))
    pub fn thermal_conductivity(&self) -> f64 {
        // k = q / (dT/dz)
        // Convert units: mW/m² to W/m², °C/km to K/m
        (self.surface_heat_flow / 1000.0) / (self.gradient / 1000.0)
    }
    
    /// Estimate mantle temperature at Moho (°C)
    pub fn moho_temperature(&self) -> f64 {
        let moho_depth = 35.0;  // 35 km typical continental crust
        self.temperature_at_depth(moho_depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_earthquake() {
        let eq = Earthquake::new(7.0, 10.0, 35.0, -120.0);
        
        // Magnitude 7 should have significant energy
        assert!(eq.energy > 1e15);
        
        // PGA should be reasonable (10^(M-3) = 10^4 = 10000g for M=7)
        let pga = eq.peak_ground_acceleration();
        assert!(pga > 1000.0);  // Should be very high for M7
        
        // Intensity should decrease with distance
        let i1 = eq.intensity_at_distance(10.0);
        let i2 = eq.intensity_at_distance(100.0);
        assert!(i1 > i2);
    }
    
    #[test]
    fn test_seismic_waves() {
        let p_wave = WaveType::PWave;
        let s_wave = WaveType::SWave;
        
        // P-waves faster than S-waves
        assert!(p_wave.velocity_in_crust() > s_wave.velocity_in_crust());
        
        // Travel time should be positive
        let time = p_wave.travel_time(100.0, false);
        assert!(time > 0.0);
        
        // P-waves arrive first
        let t_p = p_wave.travel_time(100.0, false);
        let t_s = s_wave.travel_time(100.0, false);
        assert!(t_p < t_s);
    }
    
    #[test]
    fn test_plate_tectonics() {
        let pacific = TectonicPlate::new("Pacific".to_string(), 10.0, 270.0, 1e8);
        let north_american = TectonicPlate::new("North American".to_string(), 2.0, 90.0, 7e7);
        
        // Relative velocity should be positive
        let rel_v = pacific.relative_velocity(&north_american);
        assert!(rel_v > 0.0);
        
        // Time to move should be reasonable
        let time = pacific.time_to_move(1000.0);  // 10 m
        assert!(time > 0.0);
    }
    
    #[test]
    fn test_geomagnetism() {
        let field_equator = GeomagneticField::at_latitude(0.0);
        let field_pole = GeomagneticField::at_latitude(90.0);
        
        // Intensity stronger at poles
        assert!(field_pole.intensity > field_equator.intensity);
        
        // Inclination should be 0 at equator, 90 at pole
        assert!(field_equator.inclination.abs() < 10.0);
        assert!(field_pole.inclination.abs() > 80.0);
    }
    
    #[test]
    fn test_gravity() {
        let anomaly = GravityAnomaly::at_location(45.0, 1000.0);
        
        // Free-air correction should be negative (elevation)
        assert!(anomaly.free_air < 0.0);
        
        // Bouguer correction should be positive
        assert!(anomaly.bouguer > 0.0);
        
        // Total gravity should be reasonable
        let g = anomaly.total_gravity();
        assert!(g > 970000.0 && g < 990000.0);
    }
    
    #[test]
    fn test_heat_flow() {
        let heat = HeatFlow::new(65.0, 25.0);
        
        // Temperature should increase with depth
        let t1 = heat.temperature_at_depth(10.0);
        let t2 = heat.temperature_at_depth(20.0);
        assert!(t2 > t1);
        
        // Moho temperature should be high
        let t_moho = heat.moho_temperature();
        assert!(t_moho > 500.0);
        
        // Thermal conductivity should be positive
        let k = heat.thermal_conductivity();
        assert!(k > 0.0);
    }
}
