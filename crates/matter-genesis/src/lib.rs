//! # Matter Genesis
//!
//! O DNA da Realidade: Integração fundamental de Relatividade (espaço-tempo curvo/Schwarzschild),
//! Mecânica Quântica (função de onda, evolução temporal, colapso de medição) e Química (overlap orbital).

pub mod backend;

use std::collections::HashMap;

/// Estrutura para representar números complexos de forma nativa.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn zero() -> Self {
        Self { re: 0.0, im: 0.0 }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, other: Self) -> Self {
        Self {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    pub fn mul_real(self, scalar: f64) -> Self {
        Self {
            re: self.re * scalar,
            im: self.im * scalar,
        }
    }

    pub fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    pub fn norm(self) -> f64 {
        self.norm_sq().sqrt()
    }

    pub fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    pub fn exp(self) -> Self {
        let r = self.re.exp();
        Self {
            re: r * self.im.cos(),
            im: r * self.im.sin(),
        }
    }
}

/// Grid unidimensional para simulação quântica.
pub const GRID_SIZE: usize = 64;
pub const X_MIN: f64 = -8.0;
pub const X_MAX: f64 = 8.0;

/// Partícula fundamental quântico-relativística.
#[derive(Debug, Clone)]
pub struct GenesisParticle {
    pub id: String,
    pub rest_mass: f64, // m0
    pub charge: f64,    // q
    // 4-vetor posição (t, x, y, z)
    pub pos_4: [f64; 4],
    // 4-vetor momentum (E, px, py, pz)
    pub mom_4: [f64; 4],
    // Função de onda quântica complexa discretizada
    pub wavefunction: Vec<Complex>,
}

impl GenesisParticle {
    /// Constrói um pacote de ondas gaussiano quântico com um momentum inicial e posição média.
    pub fn new(id: String, rest_mass: f64, charge: f64, x0: f64, p0: f64) -> Self {
        let mut wavefunction = vec![Complex::zero(); GRID_SIZE];
        let dx = (X_MAX - X_MIN) / (GRID_SIZE - 1) as f64;
        let sigma = 0.8; // largura do pacote

        // Preenche com o pacote de onda gaussiano: psi(x) = exp(-(x-x0)^2 / (4*sigma^2)) * exp(i * p0 * x)
        for (i, psi) in wavefunction.iter_mut().enumerate().take(GRID_SIZE) {
            let x = X_MIN + i as f64 * dx;
            let exponent = -((x - x0) * (x - x0)) / (4.0 * sigma * sigma);
            let phase = Complex::new(0.0, p0 * x).exp();
            *psi = Complex::new(exponent.exp(), 0.0).mul(phase);
        }

        let mut particle = Self {
            id,
            rest_mass,
            charge,
            pos_4: [0.0, x0, 0.0, 0.0],
            mom_4: [rest_mass, p0, 0.0, 0.0], // E = m0 inicialmente se p for pequeno
            wavefunction,
        };

        particle.normalize();
        particle.update_energy();
        particle
    }

    /// Normaliza a função de onda para que a integral de probabilidade total seja igual a 1.
    pub fn normalize(&mut self) {
        let dx = (X_MAX - X_MIN) / (GRID_SIZE - 1) as f64;
        let mut total_prob = 0.0;
        for psi in &self.wavefunction {
            total_prob += psi.norm_sq() * dx;
        }

        if total_prob > 1e-15 {
            let norm_factor = total_prob.sqrt();
            for psi in &mut self.wavefunction {
                *psi = psi.mul_real(1.0 / norm_factor);
            }
        }
    }

    /// Atualiza a energia E no 4-vetor momentum a partir da relação de dispersão relativística E^2 = p^2 + m0^2 (c=1).
    pub fn update_energy(&mut self) {
        let px = self.mom_4[1];
        let py = self.mom_4[2];
        let pz = self.mom_4[3];
        let p_sq = px * px + py * py + pz * pz;
        let m_sq = self.rest_mass * self.rest_mass;
        self.mom_4[0] = (p_sq + m_sq).sqrt();
    }

    /// Retorna o valor médio da posição <x> = integral( x * |psi(x)|^2 dx )
    pub fn expectation_position(&self) -> f64 {
        let dx = (X_MAX - X_MIN) / (GRID_SIZE - 1) as f64;
        let mut exp_x = 0.0;
        for i in 0..GRID_SIZE {
            let x = X_MIN + i as f64 * dx;
            exp_x += x * self.wavefunction[i].norm_sq() * dx;
        }
        exp_x
    }

    /// Evolui a função de onda um passo temporal no espaço-tempo relativístico curvo.
    /// Incorporamos a dilatação temporal gravitacional (Schwarzschild) retardando localmente o fluxo temporal.
    #[allow(clippy::needless_range_loop)]
    pub fn evolve_step(&mut self, dt: f64, gravity_source_mass: f64) {
        let dx = (X_MAX - X_MIN) / (GRID_SIZE - 1) as f64;
        let hbar = 1.0; // Unidades atômicas
        let m = self.rest_mass.max(0.01);

        let mut next_wf = self.wavefunction.clone();

        for i in 1..(GRID_SIZE - 1) {
            let x = X_MIN + i as f64 * dx;

            // 1. Potencial Gravitacional sob a métrica de Schwarzschild (aproximação fraca no fator de tempo)
            // g00 = 1 - 2GM / (c^2 * r)
            // Vamos assumir que a fonte gravitacional está localizada na origem x = 0
            let r = x.abs().max(0.1);
            let g00 = (1.0 - (2.0 * 0.5 * gravity_source_mass) / r).max(0.01);
            let time_dilation = g00.sqrt(); // Fator multiplicativo de tempo local

            // O tempo passa mais devagar perto da massa gravitacional
            let local_dt = dt * time_dilation;

            // 2. Operador de Energia Cinética Quântica (Discretização da segunda derivada)
            let psi_curr = self.wavefunction[i];
            let psi_prev = self.wavefunction[i - 1];
            let psi_next = self.wavefunction[i + 1];
            let laplacian = Complex::new(
                (psi_next.re - 2.0 * psi_curr.re + psi_prev.re) / (dx * dx),
                (psi_next.im - 2.0 * psi_curr.im + psi_prev.im) / (dx * dx),
            );
            // Kinetic energy = - (hbar^2 / (2m)) * d2/dx2
            let kinetic = laplacian.mul_real(-(hbar * hbar) / (2.0 * m));

            // 3. Operador de Energia Potencial (Coulomb atrativo/repulsivo se houver carga)
            // V(x) = - q * Q / |x|
            // Vamos modelar um núcleo atômico leve na origem (carga positiva Q = 1)
            let q_target = 1.0;
            let coulomb_potential = -(self.charge * q_target) / (x.abs() + 0.1);
            let potential = psi_curr.mul_real(coulomb_potential);

            // Hamiltonian total: H = T + V
            let hamiltonian = kinetic.add(potential);

            // Evolução temporal infinitesimal de Schrödinger: d_psi = -i * (H * psi) * dt / hbar
            // Multiplicação por -i: (-i.re * H.re - -i.im * H.im) -> (0 * H.re - (-1) * H.im) = H.im, (-1) * H.re
            let minus_i_h = Complex::new(hamiltonian.im, -hamiltonian.re);
            let d_psi = minus_i_h.mul_real(local_dt / hbar);

            next_wf[i] = self.wavefunction[i].add(d_psi);
        }

        self.wavefunction = next_wf;
        self.normalize();

        // Atualiza a posição clássica e o tempo próprio no 4-vetor
        let new_x = self.expectation_position();
        self.pos_4[0] += dt; // tempo coordenado
        self.pos_4[1] = new_x;

        // Atualiza momentum baseando-se no desvio médio do momentum na função de onda
        // p_avg = integral( psi* * (-i hbar d/dx) psi dx )
        let mut p_avg = 0.0;
        for i in 1..(GRID_SIZE - 1) {
            let psi_conj = self.wavefunction[i].conj();
            // Derivada primeira central
            let d_re = (self.wavefunction[i + 1].re - self.wavefunction[i - 1].re) / (2.0 * dx);
            let d_im = (self.wavefunction[i + 1].im - self.wavefunction[i - 1].im) / (2.0 * dx);
            let derivative = Complex::new(d_re, d_im);
            // -i * derivative = (derivative.im, -derivative.re)
            let minus_i_d = Complex::new(derivative.im, -derivative.re);
            let term = psi_conj.mul(minus_i_d);
            p_avg += term.re * dx; // momentum médio é real
        }
        self.mom_4[1] = p_avg;
        self.update_energy();
    }

    /// Medição quântica: colapsa a função de onda para uma única posição discreta com base nas probabilidades.
    pub fn measure(&mut self) -> f64 {
        let dx = (X_MAX - X_MIN) / (GRID_SIZE - 1) as f64;

        // Simulação de RNG determinístico baseado na soma das fases para mantê-lo reprodutível na demo
        let mut phase_sum = 0.0;
        for psi in &self.wavefunction {
            phase_sum += psi.re.abs() + psi.im.abs();
        }
        let rand_val = (phase_sum * 12345.6789) % 1.0;

        let mut cumulative = 0.0;
        let mut selected_idx = 0;

        for i in 0..GRID_SIZE {
            cumulative += self.wavefunction[i].norm_sq() * dx;
            if rand_val <= cumulative {
                selected_idx = i;
                break;
            }
        }

        let measured_x = X_MIN + selected_idx as f64 * dx;

        // Colapso: Função de onda vira uma gaussiana extremamente estreita no ponto medido
        let sigma = 0.1;
        for i in 0..GRID_SIZE {
            let x = X_MIN + i as f64 * dx;
            let exponent = -((x - measured_x) * (x - measured_x)) / (4.0 * sigma * sigma);
            self.wavefunction[i] = Complex::new(exponent.exp(), 0.0);
        }

        self.normalize();
        self.pos_4[1] = measured_x;
        self.mom_4[1] = 0.0; // Momentum colapsado
        self.update_energy();

        measured_x
    }

    /// Medição sob o referencial de um observador em movimento (Lorentz & Quantum Back-action).
    pub fn measure_relative(&mut self, v_obs: f64) -> f64 {
        let measured_x = self.measure(); // Colapso no referencial de repouso
        let v_clamp = v_obs.clamp(-0.999, 0.999);
        let gamma = 1.0 / (1.0 - v_clamp * v_clamp).sqrt();
        let t = self.pos_4[0];

        // Transformação de Lorentz para posição coordenada: x' = gamma * (x - vt)
        let observed_x = gamma * (measured_x - v_clamp * t);

        // Retroação quântica (Quantum Back-action): perturbação do momentum
        let mut phase_sum = 0.0;
        for psi in &self.wavefunction {
            phase_sum += psi.re.abs() + psi.im.abs();
        }
        let noise = ((phase_sum * 9876.54321) % 1.0) - 0.5;
        let p_kick = v_clamp * noise * 5.0;

        self.mom_4[1] += p_kick;
        self.update_energy();

        observed_x
    }

    /// Medição quântica em estado emaranhado. Colapsa mutuamente as duas partículas
    /// conservando a simetria de momento linear.
    pub fn measure_entangled(&mut self, other: &mut GenesisParticle, v_obs: f64) -> (f64, f64) {
        let x_obs_a = self.measure_relative(v_obs);
        let rest_x_a = self.pos_4[1];
        let rest_x_b = -rest_x_a;

        let dx = (X_MAX - X_MIN) / (GRID_SIZE - 1) as f64;
        let sigma = 0.1;
        for i in 0..GRID_SIZE {
            let x = X_MIN + i as f64 * dx;
            let exponent = -((x - rest_x_b) * (x - rest_x_b)) / (4.0 * sigma * sigma);
            other.wavefunction[i] = Complex::new(exponent.exp(), 0.0);
        }
        other.normalize();
        other.pos_4[1] = rest_x_b;

        other.mom_4[1] = -self.mom_4[1];
        other.update_energy();

        let v_clamp = v_obs.clamp(-0.999, 0.999);
        let gamma = 1.0 / (1.0 - v_clamp * v_clamp).sqrt();
        let t_b = other.pos_4[0];
        let x_obs_b = gamma * (rest_x_b - v_clamp * t_b);

        (x_obs_a, x_obs_b)
    }

    /// Calcula o overlap orbital quântico entre duas funções de onda (predição de ligação).
    pub fn overlap(&self, other: &Self) -> f64 {
        let dx = (X_MAX - X_MIN) / (GRID_SIZE - 1) as f64;
        let mut sum = Complex::zero();

        for i in 0..GRID_SIZE {
            // integral ( psi_A * conj(psi_B) dx )
            let term = self.wavefunction[i].mul(other.wavefunction[i].conj());
            sum = sum.add(term);
        }

        sum.norm() * dx
    }
}

pub struct GenesisEngine {
    pub particles: HashMap<String, GenesisParticle>,
    pub entanglement: HashMap<String, String>,
}

impl GenesisEngine {
    pub fn new() -> Self {
        Self {
            particles: HashMap::new(),
            entanglement: HashMap::new(),
        }
    }

    pub fn entangle(&mut self, id_a: String, id_b: String) {
        self.entanglement.insert(id_a.clone(), id_b.clone());
        self.entanglement.insert(id_b, id_a);
    }

    pub fn create_particle(&mut self, id: String, mass: f64, charge: f64, x0: f64, p0: f64) {
        let particle = GenesisParticle::new(id.clone(), mass, charge, x0, p0);
        self.particles.insert(id, particle);
    }
}

impl Default for GenesisEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_math() {
        let c1 = Complex::new(1.0, 2.0);
        let c2 = Complex::new(3.0, -1.0);
        let sum = c1.add(c2);
        assert_eq!(sum.re, 4.0);
        assert_eq!(sum.im, 1.0);

        let prod = c1.mul(c2);
        assert_eq!(prod.re, 5.0);
        assert_eq!(prod.im, 5.0);
    }

    #[test]
    fn test_wavepacket_normalization() {
        let p = GenesisParticle::new("p1".to_string(), 1.0, -1.0, 0.0, 0.0);
        let dx = (X_MAX - X_MIN) / (GRID_SIZE - 1) as f64;
        let mut sum_prob = 0.0;
        for psi in &p.wavefunction {
            sum_prob += psi.norm_sq() * dx;
        }
        assert!((sum_prob - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_particle_evolution() {
        let mut p = GenesisParticle::new("p1".to_string(), 1.0, -1.0, -2.0, 1.0);
        let exp_x_initial = p.expectation_position();

        // Evolui no tempo sob gravidade
        p.evolve_step(0.1, 0.5);
        let exp_x_after = p.expectation_position();

        // O pacote de onda deve ter se propagado devido ao momentum inicial
        assert!(exp_x_after != exp_x_initial);
    }

    #[test]
    fn test_overlap() {
        let p1 = GenesisParticle::new("p1".to_string(), 1.0, -1.0, 0.0, 0.0);
        let p2 = GenesisParticle::new("p2".to_string(), 1.0, -1.0, 0.0, 0.0);
        let p3 = GenesisParticle::new("p3".to_string(), 1.0, -1.0, 4.0, 0.0);

        let overlap_same = p1.overlap(&p2);
        let overlap_far = p1.overlap(&p3);

        assert!((overlap_same - 1.0).abs() < 1e-4);
        assert!(overlap_far < 0.1);
    }

    #[test]
    fn test_measure_relative() {
        let mut p = GenesisParticle::new("p1".to_string(), 1.0, -1.0, 0.0, 0.0);
        let observed_x = p.measure_relative(0.5);
        let gamma = 1.0 / (1.0 - 0.25f64).sqrt();
        let rest_x = p.pos_4[1];
        let expected_x = gamma * rest_x;
        assert!((observed_x - expected_x).abs() < 1e-5);
        assert!(p.mom_4[1] != 0.0);
    }

    #[test]
    fn test_measure_entangled() {
        let mut p1 = GenesisParticle::new("p1".to_string(), 1.0, -1.0, 0.0, 0.0);
        let mut p2 = GenesisParticle::new("p2".to_string(), 1.0, -1.0, 0.0, 0.0);
        let (_obs_a, obs_b) = p1.measure_entangled(&mut p2, 0.5);

        let gamma = 1.0 / (1.0 - 0.25f64).sqrt();
        let expected_b = gamma * p2.pos_4[1];
        assert!((obs_b - expected_b).abs() < 1e-5);

        // Conservation of momentum: p2.mom_4[1] should be exactly negative p1.mom_4[1]
        assert_eq!(p2.mom_4[1], -p1.mom_4[1]);
    }
}
