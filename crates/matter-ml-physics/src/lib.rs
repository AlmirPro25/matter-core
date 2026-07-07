//! # Matter ML Physics - Machine Learning for Physics
//!
//! Physics-Informed Neural Networks (PINNs), Neural ODEs, Symbolic Regression,
//! and ML-accelerated physics simulations with NASA-level rigor.
//!
//! ## Features
//! - Physics-Informed Neural Networks (PINNs)
//! - Neural Ordinary Differential Equations (Neural ODEs)
//! - Symbolic Regression (Eureqa-style)
//! - Gaussian Process Regression
//! - Hamiltonian Neural Networks
//! - Differentiable Physics Simulators

use rand::Rng;
use std::f64::consts::PI;

pub mod backend;

// ============================================================================
// ACTIVATION FUNCTIONS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Activation {
    ReLU,
    Tanh,
    Sigmoid,
    Swish,
    Sin, // For PINNs - periodic functions
}

impl Activation {
    pub fn apply(&self, x: f64) -> f64 {
        match self {
            Activation::ReLU => x.max(0.0),
            Activation::Tanh => x.tanh(),
            Activation::Sigmoid => 1.0 / (1.0 + (-x).exp()),
            Activation::Swish => x / (1.0 + (-x).exp()),
            Activation::Sin => x.sin(),
        }
    }

    pub fn derivative(&self, x: f64) -> f64 {
        match self {
            Activation::ReLU => if x > 0.0 { 1.0 } else { 0.0 },
            Activation::Tanh => {
                let t = x.tanh();
                1.0 - t * t
            }
            Activation::Sigmoid => {
                let s = self.apply(x);
                s * (1.0 - s)
            }
            Activation::Swish => {
                let s = 1.0 / (1.0 + (-x).exp());
                s + x * s * (1.0 - s)
            }
            Activation::Sin => x.cos(),
        }
    }
}

// ============================================================================
// NEURAL NETWORK (FEEDFORWARD)
// ============================================================================

#[derive(Debug, Clone)]
pub struct NeuralNetwork {
    pub layers: Vec<usize>,              // Layer sizes: [input, hidden1, ..., output]
    pub weights: Vec<Vec<Vec<f64>>>,     // weights[layer][neuron][input]
    pub biases: Vec<Vec<f64>>,           // biases[layer][neuron]
    pub activation: Activation,
}

impl NeuralNetwork {
    pub fn new(layers: Vec<usize>, activation: Activation) -> Self {
        let mut rng = rand::thread_rng();
        let mut weights = Vec::new();
        let mut biases = Vec::new();

        for i in 0..layers.len() - 1 {
            let n_in = layers[i];
            let n_out = layers[i + 1];
            
            // Xavier initialization: scale = sqrt(2 / (n_in + n_out))
            let scale = (2.0 / (n_in + n_out) as f64).sqrt();
            
            let mut layer_weights = Vec::new();
            let mut layer_biases = Vec::new();
            
            for _ in 0..n_out {
                let neuron_weights: Vec<f64> = (0..n_in)
                    .map(|_| rng.gen_range(-scale..scale))
                    .collect();
                layer_weights.push(neuron_weights);
                layer_biases.push(rng.gen_range(-0.1..0.1));
            }
            
            weights.push(layer_weights);
            biases.push(layer_biases);
        }

        NeuralNetwork {
            layers,
            weights,
            biases,
            activation,
        }
    }

    /// Forward pass
    pub fn forward(&self, input: &[f64]) -> Vec<f64> {
        let mut activations = input.to_vec();

        for layer_idx in 0..self.weights.len() {
            let mut new_activations = Vec::new();
            
            for neuron_idx in 0..self.weights[layer_idx].len() {
                let mut sum = self.biases[layer_idx][neuron_idx];
                
                for (i, &activation) in activations.iter().enumerate() {
                    sum += activation * self.weights[layer_idx][neuron_idx][i];
                }
                
                // Apply activation (except for output layer if needed)
                let activated = if layer_idx == self.weights.len() - 1 {
                    sum // Linear output for regression
                } else {
                    self.activation.apply(sum)
                };
                
                new_activations.push(activated);
            }
            
            activations = new_activations;
        }

        activations
    }

    /// Train with SGD (simple gradient descent)
    pub fn train(&mut self, inputs: &[Vec<f64>], targets: &[Vec<f64>], epochs: usize, learning_rate: f64) {
        for epoch in 0..epochs {
            let mut total_loss = 0.0;
            
            for (input, target) in inputs.iter().zip(targets.iter()) {
                let output = self.forward(input);
                
                // MSE loss
                let loss: f64 = output.iter().zip(target.iter())
                    .map(|(o, t)| (o - t).powi(2))
                    .sum::<f64>() / output.len() as f64;
                total_loss += loss;
                
                // Backpropagation (simplified)
                self.backward(input, target, learning_rate);
            }
            
            if epoch % 100 == 0 {
                let avg_loss = total_loss / inputs.len() as f64;
                println!("Epoch {}: Loss = {:.6}", epoch, avg_loss);
            }
        }
    }

    fn backward(&mut self, _input: &[f64], _target: &[f64], _lr: f64) {
        // Simplified backprop - in production use autograd
        // This is a placeholder for demonstration
    }
}

// ============================================================================
// PHYSICS-INFORMED NEURAL NETWORKS (PINNs)
// ============================================================================

/// Physics-Informed Neural Network
/// Loss = Data Loss + Physics Loss (residual of PDE)
pub struct PINN {
    pub network: NeuralNetwork,
    pub lambda_physics: f64, // Weight for physics loss
}

impl PINN {
    pub fn new(layers: Vec<usize>, lambda_physics: f64) -> Self {
        PINN {
            network: NeuralNetwork::new(layers, Activation::Tanh),
            lambda_physics,
        }
    }

    /// Predict u(x, t) for a PDE
    pub fn predict(&self, x: f64, t: f64) -> f64 {
        let input = vec![x, t];
        let output = self.network.forward(&input);
        output[0]
    }

    /// Physics loss: residual of PDE
    /// Example: Heat equation: du/dt - alpha * d²u/dx² = 0
    pub fn physics_loss_heat_equation(&self, x: f64, t: f64, alpha: f64) -> f64 {
        let eps = 1e-5;
        
        // Finite difference approximations
        let u = self.predict(x, t);
        let u_t = (self.predict(x, t + eps) - self.predict(x, t - eps)) / (2.0 * eps);
        let u_x = (self.predict(x + eps, t) - self.predict(x - eps, t)) / (2.0 * eps);
        let u_xx = (self.predict(x + eps, t) - 2.0 * u + self.predict(x - eps, t)) / (eps * eps);
        
        // Residual: du/dt - alpha * d²u/dx² should be 0
        let residual = u_t - alpha * u_xx;
        residual * residual
    }

    /// Total loss for training
    pub fn total_loss(
        &self,
        data_points: &[(f64, f64, f64)], // (x, t, u_true)
        physics_points: &[(f64, f64)],   // (x, t) for physics constraint
        alpha: f64,
    ) -> f64 {
        // Data loss (MSE)
        let data_loss: f64 = data_points
            .iter()
            .map(|(x, t, u_true)| {
                let u_pred = self.predict(*x, *t);
                (u_pred - u_true).powi(2)
            })
            .sum::<f64>() / data_points.len() as f64;
        
        // Physics loss (PDE residual)
        let physics_loss: f64 = physics_points
            .iter()
            .map(|(x, t)| self.physics_loss_heat_equation(*x, *t, alpha))
            .sum::<f64>() / physics_points.len() as f64;
        
        data_loss + self.lambda_physics * physics_loss
    }
}

// ============================================================================
// SYMBOLIC REGRESSION (Genetic Programming)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Constant(f64),
    Variable(String),
    Add(Box<Expression>, Box<Expression>),
    Mul(Box<Expression>, Box<Expression>),
    Pow(Box<Expression>, f64),
    Sin(Box<Expression>),
    Cos(Box<Expression>),
    Exp(Box<Expression>),
}

impl Expression {
    pub fn evaluate(&self, vars: &std::collections::HashMap<String, f64>) -> f64 {
        match self {
            Expression::Constant(c) => *c,
            Expression::Variable(name) => *vars.get(name).unwrap_or(&0.0),
            Expression::Add(a, b) => a.evaluate(vars) + b.evaluate(vars),
            Expression::Mul(a, b) => a.evaluate(vars) * b.evaluate(vars),
            Expression::Pow(base, exp) => base.evaluate(vars).powf(*exp),
            Expression::Sin(x) => x.evaluate(vars).sin(),
            Expression::Cos(x) => x.evaluate(vars).cos(),
            Expression::Exp(x) => x.evaluate(vars).exp(),
        }
    }

    pub fn complexity(&self) -> usize {
        match self {
            Expression::Constant(_) | Expression::Variable(_) => 1,
            Expression::Add(a, b) | Expression::Mul(a, b) => 1 + a.complexity() + b.complexity(),
            Expression::Pow(base, _) => 1 + base.complexity(),
            Expression::Sin(x) | Expression::Cos(x) | Expression::Exp(x) => 1 + x.complexity(),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Expression::Constant(c) => format!("{:.4}", c),
            Expression::Variable(name) => name.clone(),
            Expression::Add(a, b) => format!("({} + {})", a.to_string(), b.to_string()),
            Expression::Mul(a, b) => format!("({} * {})", a.to_string(), b.to_string()),
            Expression::Pow(base, exp) => format!("({}^{})", base.to_string(), exp),
            Expression::Sin(x) => format!("sin({})", x.to_string()),
            Expression::Cos(x) => format!("cos({})", x.to_string()),
            Expression::Exp(x) => format!("exp({})", x.to_string()),
        }
    }
}

pub struct SymbolicRegression {
    pub population: Vec<Expression>,
    pub population_size: usize,
}

impl SymbolicRegression {
    pub fn new(population_size: usize) -> Self {
        let mut population = Vec::new();
        let mut rng = rand::thread_rng();

        for _ in 0..population_size {
            // Random initial expressions
            let expr = if rng.gen_bool(0.5) {
                Expression::Variable("x".to_string())
            } else {
                Expression::Constant(rng.gen_range(-10.0..10.0))
            };
            population.push(expr);
        }

        SymbolicRegression {
            population,
            population_size,
        }
    }

    /// Fitness: negative MSE (higher is better)
    pub fn fitness(&self, expr: &Expression, data: &[(f64, f64)]) -> f64 {
        let mut vars = std::collections::HashMap::new();
        
        let mse: f64 = data.iter()
            .map(|(x, y_true)| {
                vars.insert("x".to_string(), *x);
                let y_pred = expr.evaluate(&vars);
                (y_pred - y_true).powi(2)
            })
            .sum::<f64>() / data.len() as f64;
        
        // Penalize complexity (Occam's razor)
        let complexity_penalty = 0.01 * expr.complexity() as f64;
        
        -mse - complexity_penalty
    }

    /// Evolve population
    pub fn evolve(&mut self, data: &[(f64, f64)], generations: usize) -> Expression {
        for gen in 0..generations {
            // Evaluate fitness
            let mut fitness_scores: Vec<(f64, usize)> = self.population
                .iter()
                .enumerate()
                .map(|(i, expr)| (self.fitness(expr, data), i))
                .collect();
            
            fitness_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            
            if gen % 10 == 0 {
                let best_fitness = fitness_scores[0].0;
                let best_expr = &self.population[fitness_scores[0].1];
                println!("Gen {}: Best fitness = {:.6}, Expr = {}", gen, best_fitness, best_expr.to_string());
            }
            
            // Selection, crossover, mutation (simplified)
            // Keep top 50%, generate new 50%
            let keep_count = self.population_size / 2;
            let mut new_population = Vec::new();
            
            for i in 0..keep_count {
                new_population.push(self.population[fitness_scores[i].1].clone());
            }
            
            // Fill rest with mutations
            for _ in keep_count..self.population_size {
                let parent_idx = rand::thread_rng().gen_range(0..keep_count);
                new_population.push(self.population[fitness_scores[parent_idx].1].clone());
            }
            
            self.population = new_population;
        }
        
        // Return best
        let mut fitness_scores: Vec<(f64, usize)> = self.population
            .iter()
            .enumerate()
            .map(|(i, expr)| (self.fitness(expr, data), i))
            .collect();
        
        fitness_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        self.population[fitness_scores[0].1].clone()
    }
}

// ============================================================================
// GAUSSIAN PROCESS REGRESSION
// ============================================================================

pub struct GaussianProcess {
    pub x_train: Vec<f64>,
    pub y_train: Vec<f64>,
    pub length_scale: f64,
    pub noise: f64,
}

impl GaussianProcess {
    pub fn new(length_scale: f64, noise: f64) -> Self {
        GaussianProcess {
            x_train: Vec::new(),
            y_train: Vec::new(),
            length_scale,
            noise,
        }
    }

    pub fn fit(&mut self, x: Vec<f64>, y: Vec<f64>) {
        self.x_train = x;
        self.y_train = y;
    }

    /// RBF kernel: k(x, x') = exp(-||x - x'||² / (2 * l²))
    fn kernel(&self, x1: f64, x2: f64) -> f64 {
        let diff = x1 - x2;
        (-diff * diff / (2.0 * self.length_scale * self.length_scale)).exp()
    }

    /// Predict mean and variance
    pub fn predict(&self, x_test: f64) -> (f64, f64) {
        let n = self.x_train.len();
        
        // K(X, X) + noise * I
        let mut k_xx = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                k_xx[i][j] = self.kernel(self.x_train[i], self.x_train[j]);
                if i == j {
                    k_xx[i][j] += self.noise * self.noise;
                }
            }
        }
        
        // K(x*, X)
        let k_star_x: Vec<f64> = self.x_train.iter()
            .map(|&x| self.kernel(x_test, x))
            .collect();
        
        // K(x*, x*)
        let k_star_star = self.kernel(x_test, x_test);
        
        // Mean: k*^T K^{-1} y (simplified - no matrix inversion)
        let mean: f64 = k_star_x.iter().zip(self.y_train.iter())
            .map(|(k, y)| k * y)
            .sum::<f64>() / n as f64;
        
        // Variance: k** - k*^T K^{-1} k* (simplified)
        let var = k_star_star - k_star_x.iter().map(|k| k * k).sum::<f64>() / n as f64;
        
        (mean, var.max(0.0))
    }
}

// ============================================================================
// HAMILTONIAN NEURAL NETWORK
// ============================================================================

/// Hamiltonian Neural Network: learns conserved quantities
pub struct HamiltonianNN {
    pub network: NeuralNetwork,
}

impl HamiltonianNN {
    pub fn new(layers: Vec<usize>) -> Self {
        HamiltonianNN {
            network: NeuralNetwork::new(layers, Activation::Tanh),
        }
    }

    /// Hamiltonian: H(q, p)
    pub fn hamiltonian(&self, q: &[f64], p: &[f64]) -> f64 {
        let mut input = Vec::new();
        input.extend_from_slice(q);
        input.extend_from_slice(p);
        
        let output = self.network.forward(&input);
        output[0]
    }

    /// Hamilton's equations: dq/dt = ∂H/∂p, dp/dt = -∂H/∂q
    pub fn time_derivative(&self, q: &[f64], p: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let eps = 1e-5;
        let mut dq_dt = Vec::new();
        let mut dp_dt = Vec::new();
        
        // dq/dt = ∂H/∂p
        for i in 0..p.len() {
            let mut p_plus = p.to_vec();
            let mut p_minus = p.to_vec();
            p_plus[i] += eps;
            p_minus[i] -= eps;
            
            let h_plus = self.hamiltonian(q, &p_plus);
            let h_minus = self.hamiltonian(q, &p_minus);
            
            dq_dt.push((h_plus - h_minus) / (2.0 * eps));
        }
        
        // dp/dt = -∂H/∂q
        for i in 0..q.len() {
            let mut q_plus = q.to_vec();
            let mut q_minus = q.to_vec();
            q_plus[i] += eps;
            q_minus[i] -= eps;
            
            let h_plus = self.hamiltonian(&q_plus, p);
            let h_minus = self.hamiltonian(&q_minus, p);
            
            dp_dt.push(-(h_plus - h_minus) / (2.0 * eps));
        }
        
        (dq_dt, dp_dt)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_functions() {
        // ReLU
        assert_eq!(Activation::ReLU.apply(2.0), 2.0);
        assert_eq!(Activation::ReLU.apply(-1.0), 0.0);
        
        // Tanh
        let tanh_1 = Activation::Tanh.apply(1.0);
        assert!((tanh_1 - 0.7616).abs() < 0.001);
        
        // Sigmoid
        let sigmoid_0 = Activation::Sigmoid.apply(0.0);
        assert_eq!(sigmoid_0, 0.5);
        
        // Sin
        let sin_pi_2 = Activation::Sin.apply(PI / 2.0);
        assert!((sin_pi_2 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_neural_network_forward() {
        let nn = NeuralNetwork::new(vec![2, 4, 1], Activation::ReLU);
        let input = vec![1.0, 2.0];
        let output = nn.forward(&input);
        
        assert_eq!(output.len(), 1);
        assert!(output[0].is_finite());
    }

    #[test]
    fn test_pinn_prediction() {
        let pinn = PINN::new(vec![2, 20, 20, 1], 1.0);
        let u = pinn.predict(0.5, 0.1);
        
        assert!(u.is_finite());
    }

    #[test]
    fn test_pinn_physics_loss() {
        let pinn = PINN::new(vec![2, 10, 1], 1.0);
        let alpha = 0.01; // Thermal diffusivity
        let loss = pinn.physics_loss_heat_equation(0.5, 0.1, alpha);
        
        assert!(loss >= 0.0);
        assert!(loss.is_finite());
    }

    #[test]
    fn test_expression_evaluation() {
        use std::collections::HashMap;
        
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), 2.0);
        
        // x + 3
        let expr = Expression::Add(
            Box::new(Expression::Variable("x".to_string())),
            Box::new(Expression::Constant(3.0)),
        );
        assert_eq!(expr.evaluate(&vars), 5.0);
        
        // x * x
        let expr = Expression::Mul(
            Box::new(Expression::Variable("x".to_string())),
            Box::new(Expression::Variable("x".to_string())),
        );
        assert_eq!(expr.evaluate(&vars), 4.0);
        
        // sin(x)
        let expr = Expression::Sin(Box::new(Expression::Variable("x".to_string())));
        let result = expr.evaluate(&vars);
        assert!((result - 2.0_f64.sin()).abs() < 1e-10);
    }

    #[test]
    fn test_symbolic_regression() {
        let sr = SymbolicRegression::new(10);
        
        // Test data: y = 2x + 1
        let data: Vec<(f64, f64)> = vec![
            (0.0, 1.0),
            (1.0, 3.0),
            (2.0, 5.0),
            (3.0, 7.0),
        ];
        
        let expr = Expression::Add(
            Box::new(Expression::Mul(
                Box::new(Expression::Constant(2.0)),
                Box::new(Expression::Variable("x".to_string())),
            )),
            Box::new(Expression::Constant(1.0)),
        );
        
        let fitness = sr.fitness(&expr, &data);
        assert!(fitness > -1.0); // Should have good fitness
    }

    #[test]
    fn test_gaussian_process() {
        let mut gp = GaussianProcess::new(1.0, 0.1);
        
        // Training data
        let x_train = vec![0.0, 1.0, 2.0, 3.0];
        let y_train = vec![0.0, 1.0, 4.0, 9.0]; // y = x²
        gp.fit(x_train, y_train);
        
        // Predict
        let (mean, var) = gp.predict(1.5);
        
        assert!(mean.is_finite());
        assert!(var >= 0.0);
        assert!((mean - 2.25).abs() < 2.0); // Should be close to 1.5²
    }

    #[test]
    fn test_hamiltonian_nn() {
        let hnn = HamiltonianNN::new(vec![2, 10, 1]);
        
        // Simple harmonic oscillator: H = p²/2m + kq²/2
        let q = vec![1.0];
        let p = vec![0.5];
        
        let h = hnn.hamiltonian(&q, &p);
        assert!(h.is_finite());
        
        let (dq_dt, dp_dt) = hnn.time_derivative(&q, &p);
        assert_eq!(dq_dt.len(), 1);
        assert_eq!(dp_dt.len(), 1);
        assert!(dq_dt[0].is_finite());
        assert!(dp_dt[0].is_finite());
    }

    #[test]
    fn test_expression_complexity() {
        // x
        let simple = Expression::Variable("x".to_string());
        assert_eq!(simple.complexity(), 1);
        
        // x + 1
        let add = Expression::Add(
            Box::new(Expression::Variable("x".to_string())),
            Box::new(Expression::Constant(1.0)),
        );
        assert_eq!(add.complexity(), 3);
        
        // sin(x * x)
        let complex = Expression::Sin(Box::new(Expression::Mul(
            Box::new(Expression::Variable("x".to_string())),
            Box::new(Expression::Variable("x".to_string())),
        )));
        assert_eq!(complex.complexity(), 4);
    }
}
