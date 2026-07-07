//! # Matter Multiscale - Multiscale & Multiphysics Simulation
//!
//! Coupled simulations across spatial scales (atomistic → continuum),
//! temporal scales (femtoseconds → years), and physical domains
//! (mechanical + thermal + electromagnetic + chemical).
//!
//! ## Features
//! - Scale Bridging (QM/MM, MD/Continuum)
//! - Heterogeneous Multiscale Methods (HMM)
//! - Domain Decomposition
//! - Operator Splitting
//! - Staggered Coupling
//! - Adaptive Mesh Refinement (AMR)

// ============================================================================
// SPATIAL SCALES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpatialScale {
    Quantum,      // Å (angstroms) - electrons, orbitals
    Atomistic,    // nm (nanometers) - atoms, molecules
    Mesoscale,    // μm (micrometers) - grains, particles
    Continuum,    // mm-m - bulk material
    Macroscale,   // m-km - structures, systems
}

impl SpatialScale {
    /// Typical length scale (m)
    pub fn typical_length(&self) -> f64 {
        match self {
            SpatialScale::Quantum => 1e-10,      // 1 Å
            SpatialScale::Atomistic => 1e-9,     // 1 nm
            SpatialScale::Mesoscale => 1e-6,     // 1 μm
            SpatialScale::Continuum => 1e-3,     // 1 mm
            SpatialScale::Macroscale => 1.0,     // 1 m
        }
    }

    /// Typical time scale (s)
    pub fn typical_time(&self) -> f64 {
        match self {
            SpatialScale::Quantum => 1e-15,      // 1 fs (vibrations)
            SpatialScale::Atomistic => 1e-12,    // 1 ps (MD timestep)
            SpatialScale::Mesoscale => 1e-9,     // 1 ns (diffusion)
            SpatialScale::Continuum => 1e-6,     // 1 μs (deformation)
            SpatialScale::Macroscale => 1.0,         // 1 s (large scale)
        }
    }

    /// Number of particles typically simulated
    pub fn particle_count(&self) -> usize {
        match self {
            SpatialScale::Quantum => 100,           // Few atoms (DFT)
            SpatialScale::Atomistic => 1_000_000,   // MD simulation
            SpatialScale::Mesoscale => 10_000,      // Coarse-grained
            SpatialScale::Continuum => 100_000,     // FEM mesh
            SpatialScale::Macroscale => 10_000,     // Structural elements
        }
    }
}

// ============================================================================
// PHYSICAL DOMAINS
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalDomain {
    Mechanical,       // Stress, strain, deformation
    Thermal,          // Temperature, heat flow
    Electromagnetic,  // E-field, B-field, currents
    Chemical,         // Reactions, diffusion
    Fluid,            // Velocity, pressure
}

// ============================================================================
// SCALE BRIDGING
// ============================================================================

/// QM/MM: Quantum Mechanics / Molecular Mechanics
/// Partition system into QM (reactive) and MM (environment) regions
#[derive(Debug, Clone)]
pub struct QmMmCoupling {
    pub qm_atoms: Vec<usize>,     // Atom indices in QM region
    pub mm_atoms: Vec<usize>,     // Atom indices in MM region
    pub boundary_atoms: Vec<usize>, // Boundary between QM/MM
    pub embedding_energy: f64,    // QM-MM interaction energy (eV)
}

impl QmMmCoupling {
    pub fn new(qm_atoms: Vec<usize>, mm_atoms: Vec<usize>) -> Self {
        // Identify boundary atoms (QM atoms with MM neighbors)
        let mut boundary_atoms = Vec::new();
        for &qm_idx in &qm_atoms {
            // Simplified: assume adjacent indices are neighbors
            if mm_atoms.contains(&(qm_idx + 1)) || mm_atoms.contains(&(qm_idx.saturating_sub(1))) {
                boundary_atoms.push(qm_idx);
            }
        }

        QmMmCoupling {
            qm_atoms,
            mm_atoms,
            boundary_atoms,
            embedding_energy: 0.0,
        }
    }

    /// Total number of atoms
    pub fn total_atoms(&self) -> usize {
        self.qm_atoms.len() + self.mm_atoms.len()
    }

    /// QM fraction
    pub fn qm_fraction(&self) -> f64 {
        self.qm_atoms.len() as f64 / self.total_atoms() as f64
    }

    /// Compute embedding energy (electrostatic QM-MM interaction)
    /// E_embed = Σ_i Σ_j q_i q_j / r_ij where i∈QM, j∈MM
    pub fn compute_embedding(&mut self, charges: &[f64], positions: &[(f64, f64, f64)]) -> f64 {
        let mut energy = 0.0;
        let ke = 8.9875517923e9; // Coulomb constant (N·m²/C²)

        for &qm_idx in &self.qm_atoms {
            for &mm_idx in &self.mm_atoms {
                let q_qm = charges[qm_idx];
                let q_mm = charges[mm_idx];

                let (x1, y1, z1) = positions[qm_idx];
                let (x2, y2, z2) = positions[mm_idx];

                let dx = x2 - x1;
                let dy = y2 - y1;
                let dz = z2 - z1;
                let r = (dx * dx + dy * dy + dz * dz).sqrt();

                if r > 1e-10 {
                    energy += ke * q_qm * q_mm / r;
                }
            }
        }

        self.embedding_energy = energy;
        energy
    }
}

// ============================================================================
// HETEROGENEOUS MULTISCALE METHOD (HMM)
// ============================================================================

/// HMM: Coarse model + fine model with periodic refinement
#[derive(Debug, Clone)]
pub struct HeterogeneousMultiscaleMethod {
    pub coarse_scale: SpatialScale,
    pub fine_scale: SpatialScale,
    pub refinement_interval: f64, // Time between fine-scale updates (s)
    pub last_refinement: f64,     // Time of last refinement (s)
}

impl HeterogeneousMultiscaleMethod {
    pub fn new(coarse: SpatialScale, fine: SpatialScale) -> Self {
        let refinement_interval = coarse.typical_time() / 10.0;
        HeterogeneousMultiscaleMethod {
            coarse_scale: coarse,
            fine_scale: fine,
            refinement_interval,
            last_refinement: 0.0,
        }
    }

    /// Check if refinement is needed
    pub fn needs_refinement(&self, current_time: f64) -> bool {
        current_time - self.last_refinement >= self.refinement_interval
    }

    /// Perform refinement (extract fine-scale data)
    pub fn refine(&mut self, current_time: f64) {
        self.last_refinement = current_time;
    }

    /// Scale separation ratio
    pub fn scale_ratio(&self) -> f64 {
        self.coarse_scale.typical_length() / self.fine_scale.typical_length()
    }
}

// ============================================================================
// DOMAIN DECOMPOSITION
// ============================================================================

/// Spatial domain decomposition for parallel computing
#[derive(Debug, Clone)]
pub struct DomainDecomposition {
    pub domains: Vec<Domain>,
    pub overlap_width: f64, // Overlap between domains (m)
}

#[derive(Debug, Clone)]
pub struct Domain {
    pub id: usize,
    pub bounds: (f64, f64, f64, f64, f64, f64), // (xmin, xmax, ymin, ymax, zmin, zmax)
    pub neighbors: Vec<usize>,                    // Neighboring domain IDs
}

impl DomainDecomposition {
    pub fn cartesian_grid(nx: usize, ny: usize, nz: usize, size: (f64, f64, f64), overlap: f64) -> Self {
        let (lx, ly, lz) = size;
        let dx = lx / nx as f64;
        let dy = ly / ny as f64;
        let dz = lz / nz as f64;

        let mut domains = Vec::new();

        for i in 0..nx {
            for j in 0..ny {
                for k in 0..nz {
                    let id = i * ny * nz + j * nz + k;
                    let xmin = i as f64 * dx - overlap;
                    let xmax = (i + 1) as f64 * dx + overlap;
                    let ymin = j as f64 * dy - overlap;
                    let ymax = (j + 1) as f64 * dy + overlap;
                    let zmin = k as f64 * dz - overlap;
                    let zmax = (k + 1) as f64 * dz + overlap;

                    let mut neighbors = Vec::new();
                    // Add adjacent domains (6-connectivity)
                    if i > 0 { neighbors.push((i - 1) * ny * nz + j * nz + k); }
                    if i < nx - 1 { neighbors.push((i + 1) * ny * nz + j * nz + k); }
                    if j > 0 { neighbors.push(i * ny * nz + (j - 1) * nz + k); }
                    if j < ny - 1 { neighbors.push(i * ny * nz + (j + 1) * nz + k); }
                    if k > 0 { neighbors.push(i * ny * nz + j * nz + (k - 1)); }
                    if k < nz - 1 { neighbors.push(i * ny * nz + j * nz + (k + 1)); }

                    domains.push(Domain {
                        id,
                        bounds: (xmin, xmax, ymin, ymax, zmin, zmax),
                        neighbors,
                    });
                }
            }
        }

        DomainDecomposition {
            domains,
            overlap_width: overlap,
        }
    }

    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }
}

// ============================================================================
// OPERATOR SPLITTING (for multiphysics)
// ============================================================================

/// Operator splitting: solve coupled physics sequentially
/// du/dt = L1(u) + L2(u) → u^(n+1) = S2(dt) ∘ S1(dt) u^n
#[derive(Debug, Clone)]
pub struct OperatorSplitting {
    pub domains: Vec<PhysicalDomain>,
    pub time_step: f64,
    pub splitting_order: usize, // 1 (Lie) or 2 (Strang)
}

impl OperatorSplitting {
    pub fn new(domains: Vec<PhysicalDomain>, dt: f64) -> Self {
        OperatorSplitting {
            domains,
            time_step: dt,
            splitting_order: 2, // Strang splitting (2nd order)
        }
    }

    /// Lie splitting (1st order): S = S1 ∘ S2
    pub fn lie_splitting_sequence(&self) -> Vec<PhysicalDomain> {
        self.domains.clone()
    }

    /// Strang splitting (2nd order): S = S1(dt/2) ∘ S2(dt) ∘ S1(dt/2)
    pub fn strang_splitting_sequence(&self) -> Vec<(PhysicalDomain, f64)> {
        let mut sequence = Vec::new();
        let n = self.domains.len();

        // Forward half-steps
        for i in 0..n {
            sequence.push((self.domains[i], self.time_step / 2.0));
        }

        // Backward half-steps
        for i in (0..n).rev() {
            sequence.push((self.domains[i], self.time_step / 2.0));
        }

        sequence
    }
}

// ============================================================================
// ADAPTIVE MESH REFINEMENT (AMR)
// ============================================================================

/// AMR: Refine mesh in regions with high gradients
#[derive(Debug, Clone)]
pub struct AdaptiveMeshRefinement {
    pub cells: Vec<MeshCell>,
    pub max_level: usize,
    pub refinement_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct MeshCell {
    pub id: usize,
    pub level: usize,              // Refinement level (0 = coarsest)
    pub center: (f64, f64, f64),   // Cell center
    pub size: f64,                 // Cell size (assuming cubic)
    pub value: f64,                // Field value
    pub gradient: f64,             // Field gradient magnitude
    pub refined: bool,             // Has this cell been refined?
}

impl AdaptiveMeshRefinement {
    pub fn new(initial_cells: Vec<MeshCell>, threshold: f64) -> Self {
        AdaptiveMeshRefinement {
            cells: initial_cells,
            max_level: 5,
            refinement_threshold: threshold,
        }
    }

    /// Identify cells needing refinement
    pub fn mark_for_refinement(&self) -> Vec<usize> {
        self.cells
            .iter()
            .filter(|cell| {
                !cell.refined
                    && cell.level < self.max_level
                    && cell.gradient > self.refinement_threshold
            })
            .map(|cell| cell.id)
            .collect()
    }

    /// Refine a cell into 8 subcells (octree)
    pub fn refine_cell(&mut self, cell_id: usize) {
        let cell_center = self.cells[cell_id].center;
        let cell_size = self.cells[cell_id].size;
        let cell_level = self.cells[cell_id].level;
        let cell_value = self.cells[cell_id].value;
        
        let h = cell_size / 2.0;
        let new_level = cell_level + 1;
        let (cx, cy, cz) = cell_center;

        // Create 8 subcells
        let offsets = [
            (-h / 2.0, -h / 2.0, -h / 2.0),
            (h / 2.0, -h / 2.0, -h / 2.0),
            (-h / 2.0, h / 2.0, -h / 2.0),
            (h / 2.0, h / 2.0, -h / 2.0),
            (-h / 2.0, -h / 2.0, h / 2.0),
            (h / 2.0, -h / 2.0, h / 2.0),
            (-h / 2.0, h / 2.0, h / 2.0),
            (h / 2.0, h / 2.0, h / 2.0),
        ];

        for (dx, dy, dz) in offsets.iter() {
            let new_cell = MeshCell {
                id: self.cells.len(),
                level: new_level,
                center: (cx + dx, cy + dy, cz + dz),
                size: h,
                value: cell_value, // Inherit value
                gradient: 0.0,
                refined: false,
            };
            self.cells.push(new_cell);
        }

        // Mark original cell as refined
        self.cells[cell_id].refined = true;
    }

    /// Total number of active (non-refined) cells
    pub fn active_cell_count(&self) -> usize {
        self.cells.iter().filter(|c| !c.refined).count()
    }
}

// ============================================================================
// STAGGERED COUPLING
// ============================================================================

/// Staggered coupling: alternate between physics solves
#[derive(Debug, Clone)]
pub struct StaggeredCoupling {
    pub physics_a: PhysicalDomain,
    pub physics_b: PhysicalDomain,
    pub convergence_tolerance: f64,
    pub max_iterations: usize,
}

impl StaggeredCoupling {
    pub fn new(a: PhysicalDomain, b: PhysicalDomain) -> Self {
        StaggeredCoupling {
            physics_a: a,
            physics_b: b,
            convergence_tolerance: 1e-6,
            max_iterations: 100,
        }
    }

    /// Iterate until convergence
    pub fn iterate(&self, initial_a: f64, initial_b: f64) -> (f64, f64, usize) {
        let mut a = initial_a;
        let mut b = initial_b;
        let mut iter = 0;

        for i in 0..self.max_iterations {
            iter = i + 1;

            // Solve A with B fixed
            let a_new = self.solve_a(a, b);

            // Solve B with A fixed
            let b_new = self.solve_b(a_new, b);

            // Check convergence
            let residual_a = (a_new - a).abs();
            let residual_b = (b_new - b).abs();

            if residual_a < self.convergence_tolerance && residual_b < self.convergence_tolerance {
                return (a_new, b_new, iter);
            }

            a = a_new;
            b = b_new;
        }

        (a, b, iter)
    }

    fn solve_a(&self, _a: f64, b: f64) -> f64 {
        // Placeholder: A depends on B
        b * 0.9 + 0.1
    }

    fn solve_b(&self, a: f64, _b: f64) -> f64 {
        // Placeholder: B depends on A
        a * 1.1 - 0.05
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_scales() {
        assert_eq!(SpatialScale::Quantum.typical_length(), 1e-10);
        assert_eq!(SpatialScale::Atomistic.typical_length(), 1e-9);
        assert!(SpatialScale::Quantum.particle_count() < SpatialScale::Atomistic.particle_count());
    }

    #[test]
    fn test_qm_mm_coupling() {
        let qm = vec![0, 1, 2];
        let mm = vec![3, 4, 5, 6];
        let qmmm = QmMmCoupling::new(qm, mm);

        assert_eq!(qmmm.total_atoms(), 7);
        assert!((qmmm.qm_fraction() - 3.0 / 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_qm_mm_embedding() {
        let qm = vec![0, 1];
        let mm = vec![2, 3];
        let mut qmmm = QmMmCoupling::new(qm, mm);

        let charges = vec![1.0, -1.0, 0.5, -0.5];
        let positions = vec![
            (0.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (2.0, 0.0, 0.0),
            (3.0, 0.0, 0.0),
        ];

        let energy = qmmm.compute_embedding(&charges, &positions);
        assert!(energy.is_finite());
    }

    #[test]
    fn test_hmm() {
        let hmm = HeterogeneousMultiscaleMethod::new(
            SpatialScale::Continuum,
            SpatialScale::Atomistic,
        );

        assert!(hmm.scale_ratio() > 1e5); // ~6 orders of magnitude
        assert!(hmm.needs_refinement(1e-5)); // After typical interval
    }

    #[test]
    fn test_domain_decomposition() {
        let dd = DomainDecomposition::cartesian_grid(2, 2, 2, (1.0, 1.0, 1.0), 0.1);

        assert_eq!(dd.domain_count(), 8); // 2x2x2
        assert!(dd.domains[0].neighbors.len() <= 3); // Corner has 3 neighbors
    }

    #[test]
    fn test_operator_splitting() {
        let domains = vec![PhysicalDomain::Mechanical, PhysicalDomain::Thermal];
        let splitting = OperatorSplitting::new(domains, 0.01);

        let lie = splitting.lie_splitting_sequence();
        assert_eq!(lie.len(), 2);

        let strang = splitting.strang_splitting_sequence();
        assert_eq!(strang.len(), 4); // 2 forward + 2 backward
    }

    #[test]
    fn test_amr() {
        let cells = vec![
            MeshCell {
                id: 0,
                level: 0,
                center: (0.5, 0.5, 0.5),
                size: 1.0,
                value: 1.0,
                gradient: 10.0, // High gradient
                refined: false,
            },
        ];

        let mut amr = AdaptiveMeshRefinement::new(cells, 5.0);
        assert_eq!(amr.active_cell_count(), 1);

        let to_refine = amr.mark_for_refinement();
        assert_eq!(to_refine.len(), 1); // Cell 0 needs refinement

        amr.refine_cell(0);
        assert_eq!(amr.cells.len(), 9); // 1 parent + 8 children
        assert_eq!(amr.active_cell_count(), 8); // Only children are active
    }

    #[test]
    fn test_staggered_coupling() {
        let coupling = StaggeredCoupling::new(
            PhysicalDomain::Mechanical,
            PhysicalDomain::Thermal,
        );

        let (a, b, iters) = coupling.iterate(1.0, 1.0);

        assert!(a.is_finite());
        assert!(b.is_finite());
        assert!(iters > 0);
        assert!(iters <= coupling.max_iterations);
    }
}

pub mod backend;
