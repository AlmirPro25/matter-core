//! Fluid Dynamics: Navier-Stokes, turbulence, CFD, aerodynamics

use std::f64::consts::PI;
use thiserror::Error;

pub mod backend;

#[derive(Error, Debug)]
pub enum FluidError {
    #[error("Invalid flow: {0}")]
    InvalidFlow(String),
    #[error("Calculation failed: {0}")]
    CalculationFailed(String),
}

pub type Result<T> = std::result::Result<T, FluidError>;

/// Fluid properties
#[derive(Clone)]
pub struct Fluid {
    pub density: f64,
    pub viscosity: f64,
    pub temperature: f64,
}

impl Fluid {
    pub fn water() -> Self {
        Self {
            density: 1000.0,
            viscosity: 0.001,
            temperature: 293.0,
        }
    }

    pub fn air() -> Self {
        Self {
            density: 1.225,
            viscosity: 1.81e-5,
            temperature: 293.0,
        }
    }

    /// Kinematic viscosity: ν = μ/ρ
    pub fn kinematic_viscosity(&self) -> f64 {
        self.viscosity / self.density
    }

    /// Speed of sound
    pub fn speed_of_sound(&self) -> f64 {
        if self.density > 100.0 {
            1481.0 // Water
        } else {
            343.0 // Air
        }
    }
}

/// Flow regime
pub struct Flow {
    pub fluid: Fluid,
    pub velocity: f64,
    pub length_scale: f64,
}

impl Flow {
    pub fn new(fluid: Fluid, v: f64, l: f64) -> Self {
        Self {
            fluid,
            velocity: v,
            length_scale: l,
        }
    }

    /// Reynolds number: Re = ρvL/μ = vL/ν
    pub fn reynolds_number(&self) -> f64 {
        self.velocity * self.length_scale / self.fluid.kinematic_viscosity()
    }

    /// Mach number: Ma = v/c
    pub fn mach_number(&self) -> f64 {
        self.velocity / self.fluid.speed_of_sound()
    }

    /// Is laminar or turbulent?
    pub fn is_laminar(&self) -> bool {
        self.reynolds_number() < 2300.0
    }

    /// Is compressible?
    pub fn is_compressible(&self) -> bool {
        self.mach_number() > 0.3
    }

    /// Froude number: Fr = v/√(gL)
    pub fn froude_number(&self) -> f64 {
        let g = 9.81;
        self.velocity / (g * self.length_scale).sqrt()
    }

    /// Dynamic pressure: q = ½ρv²
    pub fn dynamic_pressure(&self) -> f64 {
        0.5 * self.fluid.density * self.velocity * self.velocity
    }
}

/// Pipe flow
pub struct PipeFlow {
    pub flow: Flow,
    pub diameter: f64,
    pub length: f64,
    pub roughness: f64,
}

impl PipeFlow {
    pub fn new(fluid: Fluid, v: f64, d: f64, l: f64) -> Self {
        Self {
            flow: Flow::new(fluid, v, d),
            diameter: d,
            length: l,
            roughness: 0.0,
        }
    }

    /// Friction factor (Darcy-Weisbach)
    pub fn friction_factor(&self) -> f64 {
        let re = self.flow.reynolds_number();

        if re < 2300.0 {
            // Laminar: f = 64/Re
            64.0 / re
        } else {
            // Turbulent (Colebrook approximation)
            let e_d = self.roughness / self.diameter;
            let a = (e_d / 3.7 + 5.74 / re.powf(0.9)).log10();
            0.25 / (a * a)
        }
    }

    /// Pressure drop: ΔP = f(L/D)(ρv²/2)
    pub fn pressure_drop(&self) -> f64 {
        let f = self.friction_factor();
        f * (self.length / self.diameter) * self.flow.dynamic_pressure()
    }

    /// Volumetric flow rate: Q = vA
    pub fn flow_rate(&self) -> f64 {
        let area = PI * self.diameter * self.diameter / 4.0;
        self.flow.velocity * area
    }
}

/// Boundary layer
pub struct BoundaryLayer {
    pub flow: Flow,
    pub distance: f64,
}

impl BoundaryLayer {
    pub fn new(fluid: Fluid, v: f64, x: f64) -> Self {
        Self {
            flow: Flow::new(fluid, v, x),
            distance: x,
        }
    }

    /// Boundary layer thickness: δ ≈ 5x/√Re_x
    pub fn thickness(&self) -> f64 {
        let re_x = self.flow.velocity * self.distance / self.flow.fluid.kinematic_viscosity();
        5.0 * self.distance / re_x.sqrt()
    }

    /// Displacement thickness: δ* ≈ 1.72x/√Re_x
    pub fn displacement_thickness(&self) -> f64 {
        let re_x = self.flow.velocity * self.distance / self.flow.fluid.kinematic_viscosity();
        1.72 * self.distance / re_x.sqrt()
    }

    /// Skin friction coefficient: C_f = 0.664/√Re_x
    pub fn skin_friction(&self) -> f64 {
        let re_x = self.flow.velocity * self.distance / self.flow.fluid.kinematic_viscosity();
        0.664 / re_x.sqrt()
    }
}

/// Airfoil aerodynamics
pub struct Airfoil {
    pub chord: f64,
    pub angle_of_attack: f64,
    pub flow: Flow,
}

impl Airfoil {
    pub fn new(fluid: Fluid, v: f64, chord: f64, alpha: f64) -> Self {
        Self {
            chord,
            angle_of_attack: alpha,
            flow: Flow::new(fluid, v, chord),
        }
    }

    /// Lift coefficient (thin airfoil theory): C_L ≈ 2πα
    pub fn lift_coefficient(&self) -> f64 {
        2.0 * PI * self.angle_of_attack
    }

    /// Drag coefficient (simplified)
    pub fn drag_coefficient(&self) -> f64 {
        let cl = self.lift_coefficient();
        0.01 + cl * cl / (PI * 10.0) // Induced drag
    }

    /// Lift force: L = ½ρv²SC_L
    pub fn lift_force(&self, span: f64) -> f64 {
        let area = self.chord * span;
        self.flow.dynamic_pressure() * area * self.lift_coefficient()
    }

    /// Drag force: D = ½ρv²SC_D
    pub fn drag_force(&self, span: f64) -> f64 {
        let area = self.chord * span;
        self.flow.dynamic_pressure() * area * self.drag_coefficient()
    }

    /// Lift-to-drag ratio
    pub fn ld_ratio(&self) -> f64 {
        self.lift_coefficient() / self.drag_coefficient()
    }
}

/// Turbulence model
pub struct Turbulence {
    pub kinetic_energy: f64,
    pub dissipation_rate: f64,
}

impl Turbulence {
    pub fn new(k: f64, epsilon: f64) -> Self {
        Self {
            kinetic_energy: k,
            dissipation_rate: epsilon,
        }
    }

    /// Turbulent viscosity (k-ε model): μ_t = ρC_μk²/ε
    pub fn turbulent_viscosity(&self, density: f64) -> f64 {
        let c_mu = 0.09;
        density * c_mu * self.kinetic_energy * self.kinetic_energy / self.dissipation_rate
    }

    /// Turbulent length scale: l = k^(3/2)/ε
    pub fn length_scale(&self) -> f64 {
        self.kinetic_energy.powf(1.5) / self.dissipation_rate
    }

    /// Turbulent time scale: τ = k/ε
    pub fn time_scale(&self) -> f64 {
        self.kinetic_energy / self.dissipation_rate
    }

    /// Turbulent intensity: I = √(2k/3)/U
    pub fn intensity(&self, mean_velocity: f64) -> f64 {
        (2.0 * self.kinetic_energy / 3.0).sqrt() / mean_velocity
    }
}

/// Shock wave (supersonic flow)
pub struct ShockWave {
    pub mach_upstream: f64,
    pub gamma: f64,
}

impl ShockWave {
    pub fn new(m1: f64) -> Self {
        Self {
            mach_upstream: m1,
            gamma: 1.4, // Air
        }
    }

    /// Downstream Mach number (normal shock)
    pub fn mach_downstream(&self) -> f64 {
        let m1 = self.mach_upstream;
        let g = self.gamma;

        ((1.0 + (g - 1.0) / 2.0 * m1 * m1) / (g * m1 * m1 - (g - 1.0) / 2.0)).sqrt()
    }

    /// Pressure ratio: P2/P1
    pub fn pressure_ratio(&self) -> f64 {
        let m1 = self.mach_upstream;
        let g = self.gamma;

        1.0 + 2.0 * g / (g + 1.0) * (m1 * m1 - 1.0)
    }

    /// Density ratio: ρ2/ρ1
    pub fn density_ratio(&self) -> f64 {
        let m1 = self.mach_upstream;
        let g = self.gamma;

        (g + 1.0) * m1 * m1 / ((g - 1.0) * m1 * m1 + 2.0)
    }

    /// Temperature ratio: T2/T1
    pub fn temperature_ratio(&self) -> f64 {
        self.pressure_ratio() / self.density_ratio()
    }
}

/// Fluid simulator
pub struct FluidSimulator {
    pub fluid: Option<Fluid>,
    pub flow: Option<Flow>,
    pub pipe: Option<PipeFlow>,
    pub airfoil: Option<Airfoil>,
    pub shock: Option<ShockWave>,
}

impl FluidSimulator {
    pub fn new() -> Self {
        Self {
            fluid: None,
            flow: None,
            pipe: None,
            airfoil: None,
            shock: None,
        }
    }

    pub fn create_water(&mut self) {
        self.fluid = Some(Fluid::water());
    }

    pub fn create_air(&mut self) {
        self.fluid = Some(Fluid::air());
    }

    pub fn create_flow(&mut self, v: f64, l: f64) {
        if let Some(fluid) = self.fluid.clone() {
            self.flow = Some(Flow::new(fluid, v, l));
        }
    }

    pub fn create_pipe(&mut self, v: f64, d: f64, l: f64) {
        if let Some(fluid) = self.fluid.clone() {
            self.pipe = Some(PipeFlow::new(fluid, v, d, l));
        }
    }

    pub fn create_airfoil(&mut self, v: f64, chord: f64, alpha: f64) {
        if let Some(fluid) = self.fluid.clone() {
            self.airfoil = Some(Airfoil::new(fluid, v, chord, alpha));
        }
    }

    pub fn create_shock(&mut self, mach: f64) {
        self.shock = Some(ShockWave::new(mach));
    }

    pub fn reynolds_number(&self) -> Result<f64> {
        self.flow
            .as_ref()
            .map(|f| f.reynolds_number())
            .ok_or_else(|| FluidError::InvalidFlow("No flow".to_string()))
    }

    pub fn pipe_pressure_drop(&self) -> Result<f64> {
        self.pipe
            .as_ref()
            .map(|p| p.pressure_drop())
            .ok_or_else(|| FluidError::InvalidFlow("No pipe".to_string()))
    }

    pub fn airfoil_lift(&self, span: f64) -> Result<f64> {
        self.airfoil
            .as_ref()
            .map(|a| a.lift_force(span))
            .ok_or_else(|| FluidError::InvalidFlow("No airfoil".to_string()))
    }

    pub fn shock_pressure_ratio(&self) -> Result<f64> {
        self.shock
            .as_ref()
            .map(|s| s.pressure_ratio())
            .ok_or_else(|| FluidError::InvalidFlow("No shock".to_string()))
    }
}

impl Default for FluidSimulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fluid() {
        let water = Fluid::water();
        assert!(water.density > 900.0);

        let air = Fluid::air();
        assert!(air.density < 2.0);
    }

    #[test]
    fn test_flow() {
        let water = Fluid::water();
        let flow = Flow::new(water, 1.0, 0.1);
        let re = flow.reynolds_number();
        assert!(re > 0.0);
    }

    #[test]
    fn test_pipe() {
        let water = Fluid::water();
        let pipe = PipeFlow::new(water, 2.0, 0.1, 10.0);
        let dp = pipe.pressure_drop();
        assert!(dp > 0.0);
    }

    #[test]
    fn test_airfoil() {
        let air = Fluid::air();
        let airfoil = Airfoil::new(air, 50.0, 1.0, 0.1);
        let cl = airfoil.lift_coefficient();
        assert!(cl > 0.0);
    }

    #[test]
    fn test_shock() {
        let shock = ShockWave::new(2.0);
        let p_ratio = shock.pressure_ratio();
        assert!(p_ratio > 1.0);
    }

    #[test]
    fn test_simulator() {
        let mut sim = FluidSimulator::new();

        sim.create_water();
        sim.create_flow(1.0, 0.1);
        assert!(sim.reynolds_number().is_ok());

        sim.create_pipe(2.0, 0.1, 10.0);
        assert!(sim.pipe_pressure_drop().is_ok());

        sim.create_air();
        sim.create_airfoil(50.0, 1.0, 0.1);
        assert!(sim.airfoil_lift(10.0).is_ok());

        sim.create_shock(2.0);
        assert!(sim.shock_pressure_ratio().is_ok());
    }
}
