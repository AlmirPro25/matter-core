//! # Matter Relativity
//!
//! Emulação física de relatividade restrita e geral (Schwarzschild),
//! acoplada com interface de decoerência quântico-gravitacional.

use std::collections::HashMap;

/// Constante da velocidade da luz em unidades naturais
pub const C: f64 = 1.0;
/// Constante gravitacional em unidades naturais
pub const G: f64 = 1.0;

/// Estrutura para computações da Relatividade Restrita
pub struct SpecialRelativity;

impl SpecialRelativity {
    /// Calcula o fator Lorentz gama (γ) para uma dada velocidade v (como fração de c)
    pub fn lorentz_factor(v: f64) -> Result<f64, String> {
        let beta = v / C;
        if beta.abs() >= 1.0 {
            return Err(
                "Velocidade não pode atingir ou superar a velocidade da luz (c)".to_string(),
            );
        }
        Ok(1.0 / (1.0 - beta * beta).sqrt())
    }

    /// Executa um Lorentz Boost para coordenadas (x, t) com velocidade v
    pub fn lorentz_boost(v: f64, x: f64, t: f64) -> Result<(f64, f64, f64), String> {
        let gamma = Self::lorentz_factor(v)?;
        let x_prime = gamma * (x - v * t);
        let t_prime = gamma * (t - v * x / (C * C));
        Ok((x_prime, t_prime, gamma))
    }

    /// Calcula a dilatação do tempo (tempo medido por observador estacionário)
    pub fn time_dilation(v: f64, proper_time: f64) -> Result<f64, String> {
        let gamma = Self::lorentz_factor(v)?;
        Ok(proper_time * gamma)
    }

    /// Calcula a contração de comprimento
    pub fn length_contraction(v: f64, proper_length: f64) -> Result<f64, String> {
        let gamma = Self::lorentz_factor(v)?;
        Ok(proper_length / gamma)
    }

    /// Calcula massa relativística e energias (E=mc^2)
    pub fn mass_energy(m0: f64, v: f64) -> Result<HashMap<String, f64>, String> {
        let gamma = Self::lorentz_factor(v)?;
        let rel_mass = m0 * gamma;
        let rest_energy = m0 * C * C;
        let total_energy = rel_mass * C * C;
        let kinetic_energy = total_energy - rest_energy;
        let momentum = rel_mass * v;

        let mut res = HashMap::new();
        res.insert("gamma".to_string(), gamma);
        res.insert("relativistic_mass".to_string(), rel_mass);
        res.insert("rest_energy".to_string(), rest_energy);
        res.insert("total_energy".to_string(), total_energy);
        res.insert("kinetic_energy".to_string(), kinetic_energy);
        res.insert("momentum".to_string(), momentum);

        Ok(res)
    }
}

/// Estrutura para computações da Relatividade Geral (Métrica de Schwarzschild)
pub struct GeneralRelativity;

impl GeneralRelativity {
    /// Calcula o Raio de Schwarzschild para uma determinada massa M
    pub fn schwarzschild_radius(mass: f64) -> f64 {
        2.0 * G * mass / (C * C)
    }

    /// Calcula o fator de dilatação temporal gravitacional sob a métrica de Schwarzschild
    pub fn gravitational_time_dilation(
        mass: f64,
        radius: f64,
        coordinate_time: f64,
    ) -> Result<f64, String> {
        let rs = Self::schwarzschild_radius(mass);
        if radius <= rs {
            return Err("Raio está dentro ou no horizonte de eventos de Schwarzschild (singularidade temporal)".to_string());
        }
        let factor = (1.0 - rs / radius).sqrt();
        Ok(coordinate_time * factor)
    }

    /// Dá um passo num integrador geodésico usando o potencial efetivo de Einstein (Correção Post-Newtoniana)
    /// Retorna a nova posição (x, y) e velocidade (vx, vy)
    pub fn geodesic_step(
        mass: f64,
        x: f64,
        y: f64,
        vx: f64,
        vy: f64,
        dt: f64,
    ) -> Result<(f64, f64, f64, f64), String> {
        let r_sqr = x * x + y * y;
        let r = r_sqr.sqrt();
        let rs = Self::schwarzschild_radius(mass);

        if r <= rs {
            // Capturado pelo horizonte de eventos
            return Ok((0.0, 0.0, 0.0, 0.0));
        }

        // Momento angular específico L = r x v = x*vy - y*vx
        let l = x * vy - y * vx;
        let l_sqr = l * l;

        // Aceleração clássica com correção relativística (3L^2 / r^2 c^2)
        // a = -GM/r^3 * vec(r) * (1 + 3L^2 / (r^2 * c^2))
        let correction = 1.0 + (3.0 * l_sqr) / (r_sqr * C * C);
        let acc_mag = -(G * mass * correction) / (r_sqr * r);

        let ax = acc_mag * x;
        let ay = acc_mag * y;

        // Integração simples de Euler-Cromer
        let new_vx = vx + ax * dt;
        let new_vy = vy + ay * dt;
        let new_x = x + new_vx * dt;
        let new_y = y + new_vy * dt;

        Ok((new_x, new_y, new_vx, new_vy))
    }
}

/// Interface Quântico-Gravitacional
pub struct QuantumGravityBridge;

impl QuantumGravityBridge {
    /// Determina o tempo de coerência efetivo de um Qubit sob efeitos gravitacionais.
    /// A aceleração gravitacional própria causa decoerência adicional (efeito análogo a Unruh/Hawking térmico).
    pub fn gravitational_decoherence(
        coherence_time: f64,
        mass: f64,
        radius: f64,
        alpha: f64,
    ) -> Result<f64, String> {
        let rs = GeneralRelativity::schwarzschild_radius(mass);
        if radius <= rs {
            return Ok(0.0); // Coerência destruída instantaneamente no horizonte
        }

        // Fator de dilatação temporal gravitacional (redshift)
        let red_factor = (1.0 - rs / radius).sqrt();

        // Aceleração própria para se manter estacionário a um raio r
        // a = GM / (r^2 * sqrt(1 - Rs/r))
        let proper_acceleration = (G * mass) / (radius * radius * red_factor);

        // O tempo de coerência diminui exponencialmente com a aceleração própria do campo
        let unruh_thermal_decay = (-alpha * proper_acceleration).exp();

        // O tempo na perspectiva do laboratório também sofre dilatação
        let final_coherence = coherence_time * red_factor * unruh_thermal_decay;

        Ok(final_coherence)
    }
}

pub mod backend;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lorentz_factor() {
        let lf = SpecialRelativity::lorentz_factor(0.6).unwrap();
        assert!((lf - 1.25).abs() < 1e-6); // Gamma at 0.6c is exactly 1.25
    }

    #[test]
    fn test_lorentz_boost() {
        let (x_p, t_p, gamma) = SpecialRelativity::lorentz_boost(0.6, 1.0, 0.0).unwrap();
        assert!((gamma - 1.25).abs() < 1e-6);
        assert!((x_p - 1.25).abs() < 1e-6);
        assert!((t_p - (-0.75)).abs() < 1e-6);
    }

    #[test]
    fn test_mass_energy() {
        let me = SpecialRelativity::mass_energy(10.0, 0.8).unwrap();
        let rest = me.get("rest_energy").unwrap();
        let total = me.get("total_energy").unwrap();
        assert!((rest - 10.0).abs() < 1e-6);
        assert!((total - 16.6666666).abs() < 1e-3); // gamma = 1.6666...
    }

    #[test]
    fn test_schwarzschild_radius() {
        let rs = GeneralRelativity::schwarzschild_radius(5.0);
        assert_eq!(rs, 10.0);
    }

    #[test]
    fn test_gravitational_time_dilation() {
        let t_dilated = GeneralRelativity::gravitational_time_dilation(5.0, 20.0, 100.0).unwrap();
        // Rs = 10, radius = 20, factor = sqrt(1 - 10/20) = sqrt(0.5) = 0.7071
        assert!((t_dilated - 70.710678).abs() < 1e-3);
    }

    #[test]
    fn test_quantum_decoherence() {
        // Rs = 2, radius = 10, factor = sqrt(1 - 0.2) = 0.8944
        // Acceleration = 1 / (100 * 0.8944) = 0.01118
        let coh = QuantumGravityBridge::gravitational_decoherence(100.0, 1.0, 10.0, 0.1).unwrap();
        assert!(coh < 100.0);
        assert!(coh > 80.0);
    }
}
