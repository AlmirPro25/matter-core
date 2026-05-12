// Matter Automatic Parallelization
// Analyzes code and automatically parallelizes safe operations

use matter_ast::{Expression, Statement};
use matter_backend::Value;
use matter_error::MatterError;
use petgraph::graph::{DiGraph, NodeIndex};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

type Result<T> = std::result::Result<T, MatterError>;

/// Automatic parallelization engine
pub struct AutoParallel {
    /// Dependency graph
    dependencies: DiGraph<Operation, Dependency>,
    /// Operation to node mapping
    operations: HashMap<String, NodeIndex>,
    /// Parallelizable groups
    parallel_groups: Vec<Vec<NodeIndex>>,
}

/// Operation in the dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub kind: OperationKind,
    pub reads: HashSet<String>,
    pub writes: HashSet<String>,
    pub cost: f64,
    pub parallelizable: bool,
}

/// Kind of operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationKind {
    /// Pure computation
    Pure,
    /// I/O operation
    IO,
    /// FFI call
    FFI { language: String },
    /// Memory allocation
    Allocation,
    /// Function call
    FunctionCall { name: String },
}

/// Dependency between operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Dependency {
    /// Data dependency (read-after-write)
    Data,
    /// Control dependency
    Control,
    /// Anti-dependency (write-after-read)
    Anti,
    /// Output dependency (write-after-write)
    Output,
}

impl AutoParallel {
    /// Create a new auto-parallelization engine
    pub fn new() -> Self {
        Self {
            dependencies: DiGraph::new(),
            operations: HashMap::new(),
            parallel_groups: Vec::new(),
        }
    }

    /// Analyze code and build dependency graph
    pub fn analyze(&mut self, stmts: &[Statement]) -> Result<()> {
        for (i, stmt) in stmts.iter().enumerate() {
            self.analyze_stmt(stmt, i)?;
        }
        Ok(())
    }

    /// Analyze a statement
    fn analyze_stmt(&mut self, stmt: &Statement, index: usize) -> Result<()> {
        let op_id = format!("stmt_{}", index);

        let operation = match stmt {
            Statement::Let { name, value, .. } => {
                let mut reads = HashSet::new();
                self.collect_reads(value, &mut reads);

                let mut writes = HashSet::new();
                writes.insert(name.clone());

                Operation {
                    id: op_id.clone(),
                    kind: OperationKind::Pure,
                    reads,
                    writes,
                    cost: self.estimate_cost(value),
                    parallelizable: true,
                }
            }
            Statement::Expression(expr) => {
                let mut reads = HashSet::new();
                self.collect_reads(expr, &mut reads);

                Operation {
                    id: op_id.clone(),
                    kind: self.classify_expr(expr),
                    reads,
                    writes: HashSet::new(),
                    cost: self.estimate_cost(expr),
                    parallelizable: self.is_parallelizable(expr),
                }
            }
            _ => Operation {
                id: op_id.clone(),
                kind: OperationKind::Pure,
                reads: HashSet::new(),
                writes: HashSet::new(),
                cost: 1.0,
                parallelizable: false,
            },
        };

        let idx = self.dependencies.add_node(operation);
        self.operations.insert(op_id, idx);

        Ok(())
    }

    /// Collect variables read by an expression
    fn collect_reads(&self, expr: &Expression, reads: &mut HashSet<String>) {
        match expr {
            Expression::Identifier(name) => {
                reads.insert(name.clone());
            }
            Expression::Binary { left, right, .. } => {
                self.collect_reads(left, reads);
                self.collect_reads(right, reads);
            }
            Expression::Unary { operand, .. } => self.collect_reads(operand, reads),
            Expression::Call { callee, args, .. } => {
                self.collect_reads(callee, reads);
                for arg in args {
                    self.collect_reads(arg, reads);
                }
            }
            Expression::List(elements) => {
                for elem in elements {
                    self.collect_reads(elem, reads);
                }
            }
            Expression::Map(entries)
            | Expression::StructLiteral {
                fields: entries, ..
            } => {
                for (_, value) in entries {
                    self.collect_reads(value, reads);
                }
            }
            Expression::Field { target, .. } => self.collect_reads(target, reads),
            Expression::Index { target, index } => {
                self.collect_reads(target, reads);
                self.collect_reads(index, reads);
            }
            Expression::MethodCall { target, args, .. } => {
                self.collect_reads(target, reads);
                for arg in args {
                    self.collect_reads(arg, reads);
                }
            }
            Expression::BackendCall { args, .. } => {
                for arg in args {
                    self.collect_reads(arg, reads);
                }
            }
            _ => {}
        }
    }

    /// Classify expression kind
    fn classify_expr(&self, expr: &Expression) -> OperationKind {
        match expr {
            Expression::Call { callee, .. } => {
                if let Expression::Identifier(name) = &**callee {
                    OperationKind::FunctionCall { name: name.clone() }
                } else {
                    OperationKind::Pure
                }
            }
            _ => OperationKind::Pure,
        }
    }

    /// Estimate operation cost
    fn estimate_cost(&self, expr: &Expression) -> f64 {
        match expr {
            Expression::Int(_)
            | Expression::Float(_)
            | Expression::Bool(_)
            | Expression::String(_)
            | Expression::Unit => 1.0,
            Expression::Identifier(_) => 1.0,
            Expression::Binary { .. } | Expression::Unary { .. } => 2.0,
            Expression::Call { args, .. } => 10.0 + args.len() as f64,
            Expression::List(elements) => 5.0 + elements.len() as f64,
            _ => 5.0,
        }
    }

    /// Check if expression is parallelizable
    fn is_parallelizable(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Int(_)
            | Expression::Float(_)
            | Expression::Bool(_)
            | Expression::String(_)
            | Expression::Unit => true,
            Expression::Identifier(_) => true,
            Expression::Unary { operand, .. } => self.is_parallelizable(operand),
            Expression::Binary { left, right, .. } => {
                self.is_parallelizable(left) && self.is_parallelizable(right)
            }
            Expression::List(elements) => elements.iter().all(|e| self.is_parallelizable(e)),
            _ => false,
        }
    }

    /// Build dependency edges
    pub fn build_dependencies(&mut self) -> Result<()> {
        let nodes: Vec<_> = self.dependencies.node_indices().collect();

        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let op_i = &self.dependencies[nodes[i]];
                let op_j = &self.dependencies[nodes[j]];

                // Check for dependencies
                if self.has_dependency(op_i, op_j) {
                    let dep_type = self.classify_dependency(op_i, op_j);
                    self.dependencies.add_edge(nodes[i], nodes[j], dep_type);
                }
            }
        }

        Ok(())
    }

    /// Check if there's a dependency between operations
    fn has_dependency(&self, op_i: &Operation, op_j: &Operation) -> bool {
        // Read-after-write
        if !op_i.writes.is_disjoint(&op_j.reads) {
            return true;
        }

        // Write-after-read
        if !op_i.reads.is_disjoint(&op_j.writes) {
            return true;
        }

        // Write-after-write
        if !op_i.writes.is_disjoint(&op_j.writes) {
            return true;
        }

        false
    }

    /// Classify dependency type
    fn classify_dependency(&self, op_i: &Operation, op_j: &Operation) -> Dependency {
        if !op_i.writes.is_disjoint(&op_j.reads) {
            Dependency::Data
        } else if !op_i.reads.is_disjoint(&op_j.writes) {
            Dependency::Anti
        } else if !op_i.writes.is_disjoint(&op_j.writes) {
            Dependency::Output
        } else {
            Dependency::Control
        }
    }

    /// Find parallelizable groups
    pub fn find_parallel_groups(&mut self) -> Result<Vec<Vec<String>>> {
        self.parallel_groups.clear();

        let nodes: Vec<_> = self.dependencies.node_indices().collect();
        let mut visited = HashSet::new();

        for &node in &nodes {
            if visited.contains(&node) {
                continue;
            }

            let mut group = vec![node];
            visited.insert(node);

            // Find independent operations
            for &other in &nodes {
                if visited.contains(&other) {
                    continue;
                }

                if self.can_parallelize(node, other) {
                    group.push(other);
                    visited.insert(other);
                }
            }

            if group.len() > 1 {
                self.parallel_groups.push(group);
            }
        }

        // Convert to operation IDs
        let result = self
            .parallel_groups
            .iter()
            .map(|group| {
                group
                    .iter()
                    .map(|&idx| self.dependencies[idx].id.clone())
                    .collect()
            })
            .collect();

        Ok(result)
    }

    /// Check if two operations can be parallelized
    fn can_parallelize(&self, a: NodeIndex, b: NodeIndex) -> bool {
        let op_a = &self.dependencies[a];
        let op_b = &self.dependencies[b];

        // Both must be parallelizable
        if !op_a.parallelizable || !op_b.parallelizable {
            return false;
        }

        // No dependencies between them
        if self.has_dependency(op_a, op_b) || self.has_dependency(op_b, op_a) {
            return false;
        }

        true
    }

    /// Execute operations in parallel
    pub fn execute_parallel<F>(&self, group: &[String], executor: F) -> Result<Vec<Value>>
    where
        F: Fn(&str) -> Result<Value> + Send + Sync,
    {
        let results: Result<Vec<_>> = group.par_iter().map(|op_id| executor(op_id)).collect();

        results
    }

    /// Get parallelization statistics
    pub fn get_stats(&self) -> ParallelStats {
        let total_ops = self.dependencies.node_count();
        let parallel_ops: usize = self.parallel_groups.iter().map(|g| g.len()).sum();
        let sequential_ops = total_ops - parallel_ops;

        let total_cost: f64 = self
            .dependencies
            .node_indices()
            .map(|idx| self.dependencies[idx].cost)
            .sum();

        let parallel_cost: f64 = self
            .parallel_groups
            .iter()
            .map(|group| {
                group
                    .iter()
                    .map(|&idx| self.dependencies[idx].cost)
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0)
            })
            .sum();

        let sequential_cost = total_cost - parallel_cost;
        let speedup = if parallel_cost > 0.0 {
            total_cost / (sequential_cost + parallel_cost)
        } else {
            1.0
        };

        ParallelStats {
            total_operations: total_ops,
            parallel_operations: parallel_ops,
            sequential_operations: sequential_ops,
            parallel_groups: self.parallel_groups.len(),
            estimated_speedup: speedup,
        }
    }
}

/// Parallelization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelStats {
    pub total_operations: usize,
    pub parallel_operations: usize,
    pub sequential_operations: usize,
    pub parallel_groups: usize,
    pub estimated_speedup: f64,
}

impl Default for AutoParallel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_parallel_creation() {
        let parallel = AutoParallel::new();
        assert_eq!(parallel.operations.len(), 0);
    }

    #[test]
    fn test_operation_cost() {
        let parallel = AutoParallel::new();
        let expr = Expression::Float(42.0);
        assert_eq!(parallel.estimate_cost(&expr), 1.0);
    }
}
