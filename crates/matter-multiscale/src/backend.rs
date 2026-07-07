//! Backend integration for Matter Multiscale

use matter_backend::{Backend, Value};
use crate::*;

pub struct MultiscaleBackend;

impl Backend for MultiscaleBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        match method {
            // Spatial Scales
            "scale_length" => {
                let scale_str = args[0].as_string()?;
                let scale = parse_scale(&scale_str)?;
                Ok(Value::Float(scale.typical_length()))
            }

            "scale_time" => {
                let scale_str = args[0].as_string()?;
                let scale = parse_scale(&scale_str)?;
                Ok(Value::Float(scale.typical_time()))
            }

            "scale_particles" => {
                let scale_str = args[0].as_string()?;
                let scale = parse_scale(&scale_str)?;
                Ok(Value::Int(scale.particle_count() as i64))
            }

            // QM/MM
            "qmmm_create" => {
                let qm_count = args[0].as_int()? as usize;
                let mm_count = args[1].as_int()? as usize;
                let qm = (0..qm_count).collect();
                let mm = (qm_count..qm_count + mm_count).collect();
                let qmmm = QmMmCoupling::new(qm, mm);
                Ok(Value::new_string(format!(
                    "QM/MM: {} QM atoms, {} MM atoms",
                    qmmm.qm_atoms.len(),
                    qmmm.mm_atoms.len()
                )))
            }

            // HMM
            "hmm_create" => {
                let coarse_str = args[0].as_string()?;
                let fine_str = args[1].as_string()?;
                let coarse = parse_scale(&coarse_str)?;
                let fine = parse_scale(&fine_str)?;
                let hmm = HeterogeneousMultiscaleMethod::new(coarse, fine);
                Ok(Value::new_string(format!("HMM: scale ratio = {:.1e}", hmm.scale_ratio())))
            }

            // Domain Decomposition
            "domain_decomp" => {
                let nx = args[0].as_int()? as usize;
                let ny = args[1].as_int()? as usize;
                let nz = args[2].as_int()? as usize;
                let dd = DomainDecomposition::cartesian_grid(nx, ny, nz, (1.0, 1.0, 1.0), 0.1);
                Ok(Value::new_string(format!("{} domains created", dd.domain_count())))
            }

            // Operator Splitting
            "splitting_create" => {
                let domains = vec![PhysicalDomain::Mechanical, PhysicalDomain::Thermal];
                let dt = args[0].as_float()?;
                let splitting = OperatorSplitting::new(domains, dt);
                Ok(Value::new_string(format!("Operator splitting: order {}", splitting.splitting_order)))
            }

            // AMR
            "amr_create" => {
                let threshold = args[0].as_float()?;
                let cells = vec![
                    MeshCell {
                        id: 0,
                        level: 0,
                        center: (0.5, 0.5, 0.5),
                        size: 1.0,
                        value: 1.0,
                        gradient: 0.0,
                        refined: false,
                    }
                ];
                let amr = AdaptiveMeshRefinement::new(cells, threshold);
                Ok(Value::new_string(format!("AMR: {} active cells", amr.active_cell_count())))
            }

            _ => Err(format!("Unknown multiscale operation: {}", method)),
        }
    }
}

fn parse_scale(s: &str) -> Result<SpatialScale, String> {
    match s.to_lowercase().as_str() {
        "quantum" => Ok(SpatialScale::Quantum),
        "atomistic" => Ok(SpatialScale::Atomistic),
        "mesoscale" => Ok(SpatialScale::Mesoscale),
        "continuum" => Ok(SpatialScale::Continuum),
        "macroscale" => Ok(SpatialScale::Macroscale),
        _ => Err(format!("Unknown scale: {}", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_length() {
        let mut backend = MultiscaleBackend;
        let args = vec![Value::new_string("atomistic".to_string())];
        let result = backend.call("scale_length", args);
        assert!(result.is_ok());
        if let Ok(Value::Float(f)) = result {
            assert_eq!(f, 1e-9);
        }
    }

    #[test]
    fn test_qmmm_create() {
        let mut backend = MultiscaleBackend;
        let args = vec![Value::Int(10), Value::Int(100)];
        let result = backend.call("qmmm_create", args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hmm_create() {
        let mut backend = MultiscaleBackend;
        let args = vec![
            Value::new_string("continuum".to_string()),
            Value::new_string("atomistic".to_string()),
        ];
        let result = backend.call("hmm_create", args);
        assert!(result.is_ok());
    }
}
