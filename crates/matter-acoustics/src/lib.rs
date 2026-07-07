//! # Matter Acoustics
//!
//! Acoustics and wave propagation simulation for Matter Core.
//!
//! ## Features
//! - **Sound Propagation**: Speed, wavelength, frequency
//! - **Ultrasound**: Medical imaging, NDT
//! - **Architectural Acoustics**: Reverberation, absorption
//! - **Noise Control**: Sound pressure level, attenuation
//! - **Vibrations**: Modal analysis, resonance
//! - **Doppler Effect**: Moving sources and observers
//!
//! ## Physics Basis
//! All simulations use peer-reviewed acoustics equations:
//! - Wave equation: c = fλ
//! - Sound pressure level: SPL = 20*log10(p/p₀)
//! - Sabine equation: RT60 = 0.161*V/A
//! - Doppler shift: f' = f*(v + v_o)/(v - v_s)
//! - Acoustic impedance: Z = ρc
//! - Intensity: I = p²/(ρc)

pub mod backend;

use std::f64::consts::PI;

// ============================================================================
// SOUND WAVE PROPERTIES
// ============================================================================

/// Acoustic medium
#[derive(Debug, Clone)]
pub struct Medium {
    /// Density (kg/m³)
    pub density: f64,
    /// Speed of sound (m/s)
    pub speed_of_sound: f64,
    /// Temperature (K)
    pub temperature: f64,
}

impl Medium {
    /// Create air at standard conditions (20°C, 1 atm)
    pub fn air_standard() -> Self {
        Self {
            density: 1.204,
            speed_of_sound: 343.0,
            temperature: 293.15,
        }
    }
    
    /// Create water at standard conditions
    pub fn water_standard() -> Self {
        Self {
            density: 1000.0,
            speed_of_sound: 1482.0,
            temperature: 293.15,
        }
    }
    
    /// Create steel
    pub fn steel() -> Self {
        Self {
            density: 7850.0,
            speed_of_sound: 5960.0,
            temperature: 293.15,
        }
    }
    
    /// Calculate acoustic impedance (Rayl = kg/(m²·s))
    /// Z = ρc
    pub fn acoustic_impedance(&self) -> f64 {
        self.density * self.speed_of_sound
    }
    
    /// Calculate speed of sound in air from temperature
    /// c = 331.3 + 0.606*T (°C)
    pub fn speed_from_temperature(temperature_celsius: f64) -> f64 {
        331.3 + 0.606 * temperature_celsius
    }
}

/// Sound wave
#[derive(Debug, Clone)]
pub struct SoundWave {
    /// Frequency (Hz)
    pub frequency: f64,
    /// Amplitude (Pa)
    pub amplitude: f64,
    /// Medium
    pub medium: Medium,
}

impl SoundWave {
    /// Create sound wave
    pub fn new(frequency: f64, amplitude: f64, medium: Medium) -> Self {
        Self {
            frequency,
            amplitude,
            medium,
        }
    }
    
    /// Calculate wavelength (m)
    /// λ = c/f
    pub fn wavelength(&self) -> f64 {
        self.medium.speed_of_sound / self.frequency
    }
    
    /// Calculate angular frequency (rad/s)
    /// ω = 2πf
    pub fn angular_frequency(&self) -> f64 {
        2.0 * PI * self.frequency
    }
    
    /// Calculate wave number (rad/m)
    /// k = 2π/λ = ω/c
    pub fn wave_number(&self) -> f64 {
        self.angular_frequency() / self.medium.speed_of_sound
    }
    
    /// Calculate acoustic intensity (W/m²)
    /// I = p²/(2ρc)
    pub fn intensity(&self) -> f64 {
        let z = self.medium.acoustic_impedance();
        (self.amplitude * self.amplitude) / (2.0 * z)
    }
    
    /// Calculate sound pressure level (dB)
    /// SPL = 20*log10(p/p₀), p₀ = 20 μPa
    pub fn sound_pressure_level(&self) -> f64 {
        const P0: f64 = 20e-6;  // Reference pressure (Pa)
        20.0 * (self.amplitude / P0).log10()
    }
    
    /// Calculate sound intensity level (dB)
    /// SIL = 10*log10(I/I₀), I₀ = 10⁻¹² W/m²
    pub fn sound_intensity_level(&self) -> f64 {
        const I0: f64 = 1e-12;  // Reference intensity
        10.0 * (self.intensity() / I0).log10()
    }
    
    /// Check if ultrasound (>20 kHz)
    pub fn is_ultrasound(&self) -> bool {
        self.frequency > 20000.0
    }
    
    /// Check if infrasound (<20 Hz)
    pub fn is_infrasound(&self) -> bool {
        self.frequency < 20.0
    }
}

// ============================================================================
// DOPPLER EFFECT
// ============================================================================

/// Doppler shift for moving sources/observers
#[derive(Debug, Clone)]
pub struct DopplerEffect {
    /// Source frequency (Hz)
    pub source_frequency: f64,
    /// Speed of sound (m/s)
    pub speed_of_sound: f64,
    /// Observer velocity (m/s, positive = towards source)
    pub observer_velocity: f64,
    /// Source velocity (m/s, positive = towards observer)
    pub source_velocity: f64,
}

impl DopplerEffect {
    /// Create Doppler effect
    pub fn new(source_frequency: f64, speed_of_sound: f64, observer_velocity: f64, source_velocity: f64) -> Self {
        Self {
            source_frequency,
            speed_of_sound,
            observer_velocity,
            source_velocity,
        }
    }
    
    /// Calculate observed frequency (Hz)
    /// f' = f * (v + v_o) / (v - v_s)
    pub fn observed_frequency(&self) -> f64 {
        let numerator = self.speed_of_sound + self.observer_velocity;
        let denominator = self.speed_of_sound - self.source_velocity;
        self.source_frequency * numerator / denominator
    }
    
    /// Calculate frequency shift (Hz)
    pub fn frequency_shift(&self) -> f64 {
        self.observed_frequency() - self.source_frequency
    }
    
    /// Calculate relative frequency change
    pub fn relative_shift(&self) -> f64 {
        self.frequency_shift() / self.source_frequency
    }
}

// ============================================================================
// ARCHITECTURAL ACOUSTICS
// ============================================================================

/// Room acoustics
#[derive(Debug, Clone)]
pub struct Room {
    /// Volume (m³)
    pub volume: f64,
    /// Total surface area (m²)
    pub surface_area: f64,
    /// Average absorption coefficient (0-1)
    pub absorption_coefficient: f64,
}

impl Room {
    /// Create room
    pub fn new(volume: f64, surface_area: f64, absorption_coefficient: f64) -> Self {
        Self {
            volume,
            surface_area,
            absorption_coefficient,
        }
    }
    
    /// Create rectangular room
    pub fn rectangular(length: f64, width: f64, height: f64, absorption: f64) -> Self {
        let volume = length * width * height;
        let surface_area = 2.0 * (length * width + length * height + width * height);
        Self::new(volume, surface_area, absorption)
    }
    
    /// Calculate total absorption (Sabins)
    /// A = α * S
    pub fn total_absorption(&self) -> f64 {
        self.absorption_coefficient * self.surface_area
    }
    
    /// Calculate reverberation time RT60 (s)
    /// Sabine equation: RT60 = 0.161 * V / A
    pub fn reverberation_time(&self) -> f64 {
        let absorption = self.total_absorption();
        if absorption > 0.0 {
            0.161 * self.volume / absorption
        } else {
            f64::INFINITY
        }
    }
    
    /// Calculate Eyring reverberation time (more accurate)
    /// RT60 = 0.161 * V / (-S * ln(1-α))
    pub fn reverberation_time_eyring(&self) -> f64 {
        if self.absorption_coefficient >= 1.0 {
            return 0.0;
        }
        let denominator = -self.surface_area * (1.0 - self.absorption_coefficient).ln();
        if denominator > 0.0 {
            0.161 * self.volume / denominator
        } else {
            f64::INFINITY
        }
    }
    
    /// Calculate critical distance (m)
    /// Beyond this, reverberant field dominates
    pub fn critical_distance(&self) -> f64 {
        let absorption = self.total_absorption();
        0.057 * (absorption / PI).sqrt()
    }
    
    /// Classify room acoustics
    pub fn acoustics_quality(&self) -> &str {
        let rt60 = self.reverberation_time();
        
        if rt60 < 0.5 {
            "Dead (too absorptive)"
        } else if rt60 < 1.0 {
            "Good for speech"
        } else if rt60 < 2.0 {
            "Good for music"
        } else if rt60 < 3.0 {
            "Reverberant"
        } else {
            "Too reverberant"
        }
    }
}

// ============================================================================
// SOUND ATTENUATION
// ============================================================================

/// Sound attenuation in air
#[derive(Debug, Clone)]
pub struct Attenuation {
    /// Frequency (Hz)
    pub frequency: f64,
    /// Temperature (°C)
    pub temperature: f64,
    /// Relative humidity (%)
    pub humidity: f64,
    /// Atmospheric pressure (kPa)
    pub pressure: f64,
}

impl Attenuation {
    /// Create attenuation calculator
    pub fn new(frequency: f64, temperature: f64, humidity: f64, pressure: f64) -> Self {
        Self {
            frequency,
            temperature,
            humidity,
            pressure,
        }
    }
    
    /// Calculate atmospheric absorption coefficient (dB/m)
    /// ISO 9613-1 simplified
    pub fn absorption_coefficient(&self) -> f64 {
        // Simplified empirical formula
        let f_khz = self.frequency / 1000.0;
        let base = 0.1068 * f_khz * f_khz / (1.0 + f_khz * f_khz);
        
        // Temperature and humidity correction (simplified)
        let temp_factor = (self.temperature + 273.15) / 293.15;
        let humidity_factor = (50.0 / self.humidity.max(1.0)).sqrt();
        
        base * temp_factor * humidity_factor
    }
    
    /// Calculate sound level at distance (dB)
    /// SPL(d) = SPL(0) - 20*log10(d/d0) - α*d
    pub fn sound_level_at_distance(&self, spl_source: f64, distance: f64) -> f64 {
        let geometric_spreading = 20.0 * (distance / 1.0).log10();
        let atmospheric_loss = self.absorption_coefficient() * distance;
        
        spl_source - geometric_spreading - atmospheric_loss
    }
}

// ============================================================================
// ULTRASOUND
// ============================================================================

/// Ultrasound transducer
#[derive(Debug, Clone)]
pub struct Ultrasound {
    /// Frequency (Hz)
    pub frequency: f64,
    /// Transducer diameter (m)
    pub diameter: f64,
    /// Medium
    pub medium: Medium,
}

impl Ultrasound {
    /// Create ultrasound transducer
    pub fn new(frequency: f64, diameter: f64, medium: Medium) -> Self {
        Self {
            frequency,
            diameter,
            medium,
        }
    }
    
    /// Calculate wavelength (m)
    pub fn wavelength(&self) -> f64 {
        self.medium.speed_of_sound / self.frequency
    }
    
    /// Calculate near field length (m)
    /// N = D²/(4λ)
    pub fn near_field_length(&self) -> f64 {
        let lambda = self.wavelength();
        (self.diameter * self.diameter) / (4.0 * lambda)
    }
    
    /// Calculate beam divergence angle (degrees)
    /// sin(θ) ≈ 1.22λ/D (for circular transducer)
    pub fn beam_divergence(&self) -> f64 {
        let lambda = self.wavelength();
        let sin_theta = 1.22 * lambda / self.diameter;
        sin_theta.asin().to_degrees()
    }
    
    /// Calculate penetration depth for medical ultrasound (cm)
    /// Empirical: depth ≈ 40/f(MHz)
    pub fn penetration_depth_medical(&self) -> f64 {
        let f_mhz = self.frequency / 1e6;
        40.0 / f_mhz
    }
    
    /// Calculate Doppler shift for blood flow (Hz)
    /// Δf = 2*f*v*cos(θ)/c
    pub fn doppler_shift_blood_flow(&self, velocity: f64, angle: f64) -> f64 {
        let cos_angle = angle.to_radians().cos();
        2.0 * self.frequency * velocity * cos_angle / self.medium.speed_of_sound
    }
}

// ============================================================================
// RESONANCE & VIBRATIONS
// ============================================================================

/// Resonator system
#[derive(Debug, Clone)]
pub struct Resonator {
    /// Natural frequency (Hz)
    pub natural_frequency: f64,
    /// Quality factor Q (dimensionless)
    pub quality_factor: f64,
    /// Mass (kg)
    pub mass: f64,
}

impl Resonator {
    /// Create resonator
    pub fn new(natural_frequency: f64, quality_factor: f64, mass: f64) -> Self {
        Self {
            natural_frequency,
            quality_factor,
            mass,
        }
    }
    
    /// Calculate damping ratio ζ
    /// ζ = 1/(2Q)
    pub fn damping_ratio(&self) -> f64 {
        1.0 / (2.0 * self.quality_factor)
    }
    
    /// Calculate bandwidth (Hz)
    /// BW = f₀/Q
    pub fn bandwidth(&self) -> f64 {
        self.natural_frequency / self.quality_factor
    }
    
    /// Calculate spring constant (N/m)
    /// k = m*ω₀²
    pub fn spring_constant(&self) -> f64 {
        let omega = 2.0 * PI * self.natural_frequency;
        self.mass * omega * omega
    }
    
    /// Calculate amplitude response at frequency f
    /// A(f) = 1/√((1-(f/f₀)²)² + (1/Q * f/f₀)²)
    pub fn amplitude_response(&self, frequency: f64) -> f64 {
        let r = frequency / self.natural_frequency;
        let term1 = (1.0 - r * r).powi(2);
        let term2 = (r / self.quality_factor).powi(2);
        1.0 / (term1 + term2).sqrt()
    }
    
    /// Check if at resonance (within 3dB bandwidth)
    pub fn is_at_resonance(&self, frequency: f64) -> bool {
        let bw = self.bandwidth();
        (frequency - self.natural_frequency).abs() < bw / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sound_wave() {
        let medium = Medium::air_standard();
        let wave = SoundWave::new(440.0, 0.02, medium);  // A4 note
        
        // Wavelength should be c/f
        let lambda = wave.wavelength();
        assert!((lambda - 0.78).abs() < 0.01);
        
        // SPL should be reasonable
        let spl = wave.sound_pressure_level();
        assert!(spl > 0.0 && spl < 200.0);
        
        // Not ultrasound
        assert!(!wave.is_ultrasound());
    }
    
    #[test]
    fn test_doppler_effect() {
        let doppler = DopplerEffect::new(1000.0, 343.0, 0.0, 20.0);  // Source moving at 20 m/s
        
        // Frequency should increase (source approaching)
        let f_obs = doppler.observed_frequency();
        assert!(f_obs > 1000.0);
        
        // Approximately 6% increase
        let shift = doppler.relative_shift();
        assert!((shift - 0.062).abs() < 0.01);
    }
    
    #[test]
    fn test_room_acoustics() {
        let room = Room::rectangular(10.0, 8.0, 3.0, 0.2);  // Typical room
        
        // RT60 should be reasonable
        let rt60 = room.reverberation_time();
        assert!(rt60 > 0.0 && rt60 < 5.0);
        
        // Should be good for speech/music
        let quality = room.acoustics_quality();
        assert!(quality.contains("Good") || quality.contains("Reverberant"));
    }
    
    #[test]
    fn test_attenuation() {
        let atten = Attenuation::new(1000.0, 20.0, 50.0, 101.325);
        
        // Absorption should be positive
        let alpha = atten.absorption_coefficient();
        assert!(alpha > 0.0);
        
        // Sound should decrease with distance
        let spl_100m = atten.sound_level_at_distance(100.0, 100.0);
        let spl_10m = atten.sound_level_at_distance(100.0, 10.0);
        assert!(spl_100m < spl_10m);
    }
    
    #[test]
    fn test_ultrasound() {
        let medium = Medium::water_standard();
        let us = Ultrasound::new(5e6, 0.01, medium);  // 5 MHz, 1cm transducer
        
        // Near field should be positive
        let nf = us.near_field_length();
        assert!(nf > 0.0);
        
        // Beam divergence should be small
        let div = us.beam_divergence();
        assert!(div > 0.0 && div < 10.0);
        
        // Medical penetration
        let depth = us.penetration_depth_medical();
        assert!(depth > 0.0 && depth < 20.0);
    }
    
    #[test]
    fn test_resonator() {
        let res = Resonator::new(100.0, 50.0, 1.0);
        
        // At resonance, amplitude should be maximum
        let amp_resonance = res.amplitude_response(100.0);
        let amp_off = res.amplitude_response(150.0);
        assert!(amp_resonance > amp_off);
        
        // Q factor of 50 is good
        assert!(res.quality_factor > 10.0);
        
        // Should detect resonance
        assert!(res.is_at_resonance(100.0));
        assert!(!res.is_at_resonance(200.0));
    }
}
