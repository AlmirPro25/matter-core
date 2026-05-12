//! SIMD (Single Instruction Multiple Data) support
//!
//! Provides vectorization capabilities for parallel data processing:
//! - SSE/AVX instructions (x86-64)
//! - NEON instructions (ARM64)
//! - Vector instructions (RISC-V)
//!
//! Enables 2-4x speedup for numeric operations through auto-vectorization.

pub mod arm64;
pub mod riscv64;
pub mod x86_64;

/// SIMD vector size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorSize {
    /// 128-bit vectors (4x f32 or 2x f64)
    V128,
    /// 256-bit vectors (8x f32 or 4x f64)
    V256,
    /// 512-bit vectors (16x f32 or 8x f64)
    V512,
}

/// SIMD operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdOp {
    /// Vector addition
    Add,
    /// Vector subtraction
    Sub,
    /// Vector multiplication
    Mul,
    /// Vector division
    Div,
    /// Vector load from memory
    Load,
    /// Vector store to memory
    Store,
    /// Vector broadcast (replicate scalar)
    Broadcast,
    /// Vector horizontal sum
    HorizontalSum,
    /// Fused multiply-add (a * b + c)
    Fma,
}

/// SIMD data type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdType {
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,
    /// 32-bit integer
    I32,
    /// 64-bit integer
    I64,
}

/// SIMD instruction
#[derive(Debug, Clone)]
pub struct SimdInstruction {
    pub op: SimdOp,
    pub vector_size: VectorSize,
    pub data_type: SimdType,
}

impl SimdInstruction {
    /// Create a new SIMD instruction
    pub fn new(op: SimdOp, vector_size: VectorSize, data_type: SimdType) -> Self {
        Self {
            op,
            vector_size,
            data_type,
        }
    }

    /// Get the number of elements in the vector
    pub fn element_count(&self) -> usize {
        let bits = match self.vector_size {
            VectorSize::V128 => 128,
            VectorSize::V256 => 256,
            VectorSize::V512 => 512,
        };

        let element_bits = match self.data_type {
            SimdType::F32 | SimdType::I32 => 32,
            SimdType::F64 | SimdType::I64 => 64,
        };

        bits / element_bits
    }
}

/// Auto-vectorization analyzer
pub struct Vectorizer {
    /// Minimum loop iterations for vectorization
    min_iterations: usize,
    /// Target vector size
    vector_size: VectorSize,
}

impl Vectorizer {
    /// Create a new vectorizer
    pub fn new(vector_size: VectorSize) -> Self {
        Self {
            min_iterations: 4,
            vector_size,
        }
    }

    /// Check if a loop can be vectorized
    pub fn can_vectorize(&self, iterations: usize, has_dependencies: bool) -> bool {
        iterations >= self.min_iterations && !has_dependencies
    }

    /// Get vectorization factor (how many iterations per vector operation)
    pub fn vectorization_factor(&self, data_type: SimdType) -> usize {
        let instr = SimdInstruction::new(SimdOp::Add, self.vector_size, data_type);
        instr.element_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_element_count() {
        // 128-bit vectors
        let v128_f32 = SimdInstruction::new(SimdOp::Add, VectorSize::V128, SimdType::F32);
        assert_eq!(v128_f32.element_count(), 4); // 128 / 32 = 4

        let v128_f64 = SimdInstruction::new(SimdOp::Add, VectorSize::V128, SimdType::F64);
        assert_eq!(v128_f64.element_count(), 2); // 128 / 64 = 2

        // 256-bit vectors
        let v256_f32 = SimdInstruction::new(SimdOp::Add, VectorSize::V256, SimdType::F32);
        assert_eq!(v256_f32.element_count(), 8); // 256 / 32 = 8

        let v256_f64 = SimdInstruction::new(SimdOp::Add, VectorSize::V256, SimdType::F64);
        assert_eq!(v256_f64.element_count(), 4); // 256 / 64 = 4
    }

    #[test]
    fn test_vectorizer_can_vectorize() {
        let vectorizer = Vectorizer::new(VectorSize::V128);

        // Should vectorize: enough iterations, no dependencies
        assert!(vectorizer.can_vectorize(8, false));

        // Should NOT vectorize: too few iterations
        assert!(!vectorizer.can_vectorize(2, false));

        // Should NOT vectorize: has dependencies
        assert!(!vectorizer.can_vectorize(8, true));
    }

    #[test]
    fn test_vectorization_factor() {
        let vectorizer = Vectorizer::new(VectorSize::V128);

        // F32: 4 elements per vector
        assert_eq!(vectorizer.vectorization_factor(SimdType::F32), 4);

        // F64: 2 elements per vector
        assert_eq!(vectorizer.vectorization_factor(SimdType::F64), 2);
    }

    #[test]
    fn test_simd_instruction_creation() {
        let instr = SimdInstruction::new(SimdOp::Mul, VectorSize::V256, SimdType::F32);
        assert_eq!(instr.op, SimdOp::Mul);
        assert_eq!(instr.vector_size, VectorSize::V256);
        assert_eq!(instr.data_type, SimdType::F32);
    }
}
