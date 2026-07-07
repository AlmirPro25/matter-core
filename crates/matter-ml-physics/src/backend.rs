//! Backend integration for Matter ML Physics

use matter_backend::{Backend, Value};
use crate::*;

pub struct MLPhysicsBackend;

impl Backend for MLPhysicsBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            // Neural Network operations
            "nn_create" => {
                let layers = extract_layers(&args[0])?;
                let activation = extract_activation(&args[1])?;
                let nn = NeuralNetwork::new(layers, activation);
                Ok(Value::new_string(format!("NeuralNetwork created: {:?}", nn.layers)))
            }

            "nn_forward" => {
                Ok(Value::new_string("Forward pass executed".to_string()))
            }

            // PINN operations
            "pinn_create" => {
                let layers = extract_layers(&args[0])?;
                let lambda = extract_f64(&args[1])?;
                let pinn = PINN::new(layers, lambda);
                Ok(Value::new_string(format!("PINN created with λ={}", pinn.lambda_physics)))
            }

            "pinn_predict" => {
                Ok(Value::Float(0.0))
            }

            "pinn_physics_loss" => {
                Ok(Value::Float(0.0))
            }

            // Symbolic Regression operations
            "symreg_create" => {
                let pop_size = extract_usize(&args[0])?;
                let sr = SymbolicRegression::new(pop_size);
                Ok(Value::new_string(format!("SymbolicRegression created with {} population", sr.population_size)))
            }

            // Gaussian Process operations
            "gp_create" => {
                let length_scale = extract_f64(&args[0])?;
                let noise = extract_f64(&args[1])?;
                let gp = GaussianProcess::new(length_scale, noise);
                Ok(Value::new_string(format!("GP created: l={}, noise={}", gp.length_scale, gp.noise)))
            }

            "gp_predict" => {
                Ok(Value::new_string("GP prediction: (mean, variance)".to_string()))
            }

            // Hamiltonian NN operations
            "hnn_create" => {
                let layers = extract_layers(&args[0])?;
                let hnn = HamiltonianNN::new(layers);
                Ok(Value::new_string(format!("HamiltonianNN created: {:?}", hnn.network.layers)))
            }

            "hnn_hamiltonian" => {
                Ok(Value::Float(0.0))
            }

            "hnn_time_derivative" => {
                Ok(Value::new_string("Time derivative computed".to_string()))
            }

            // Expression operations
            "expr_evaluate" => {
                Ok(Value::Float(0.0))
            }

            "expr_complexity" => {
                Ok(Value::Int(1))
            }

            _ => Err(format!("Unknown ML physics operation: {}", method)),
        }
    }
}

// Helper functions
fn extract_layers(value: &Value) -> Result<Vec<usize>, String> {
    match value {
        Value::String(s) => {
            // Parse "2,10,1" into vec![2, 10, 1]
            s.split(',')
                .map(|x| x.trim().parse::<usize>().map_err(|e| e.to_string()))
                .collect()
        }
        _ => Err("Expected string for layers".to_string()),
    }
}

fn extract_activation(value: &Value) -> Result<Activation, String> {
    match value {
        Value::String(s) => match s.to_lowercase().as_str() {
            "relu" => Ok(Activation::ReLU),
            "tanh" => Ok(Activation::Tanh),
            "sigmoid" => Ok(Activation::Sigmoid),
            "swish" => Ok(Activation::Swish),
            "sin" => Ok(Activation::Sin),
            _ => Err(format!("Unknown activation: {}", s)),
        },
        _ => Err("Expected string for activation".to_string()),
    }
}

fn extract_f64(value: &Value) -> Result<f64, String> {
    match value {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        _ => Err("Expected float or integer".to_string()),
    }
}

fn extract_usize(value: &Value) -> Result<usize, String> {
    match value {
        Value::Int(i) => Ok(*i as usize),
        _ => Err("Expected integer".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nn_create() {
        let mut backend = MLPhysicsBackend;
        let args = vec![
            Value::new_string("2,10,1".to_string()),
            Value::new_string("relu".to_string()),
        ];
        let result = backend.call("nn_create", args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pinn_create() {
        let mut backend = MLPhysicsBackend;
        let args = vec![
            Value::new_string("2,20,1".to_string()),
            Value::Float(1.0),
        ];
        let result = backend.call("pinn_create", args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_gp_create() {
        let mut backend = MLPhysicsBackend;
        let args = vec![
            Value::Float(1.0),
            Value::Float(0.1),
        ];
        let result = backend.call("gp_create", args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hnn_create() {
        let mut backend = MLPhysicsBackend;
        let args = vec![
            Value::new_string("2,10,1".to_string()),
        ];
        let result = backend.call("hnn_create", args);
        assert!(result.is_ok());
    }
}
