//! RISC-V SIMD code generation (Vector Extension)
//!
//! Generates RISC-V Vector Extension instructions for vectorized operations.

use super::{SimdInstruction, SimdOp, SimdType, VectorSize};

/// RISC-V SIMD code generator
pub struct RiscvSimdCodeGen {
    /// Generated machine code
    code: Vec<u8>,
}

impl RiscvSimdCodeGen {
    /// Create a new RISC-V SIMD code generator
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }

    /// Emit SIMD instruction
    pub fn emit(&mut self, instr: &SimdInstruction) -> Result<(), String> {
        // RISC-V Vector Extension supports variable-length vectors
        // For simplicity, we'll use 128-bit vectors
        match instr.vector_size {
            VectorSize::V128 => self.emit_vector(instr),
            VectorSize::V256 | VectorSize::V512 => {
                Err("RISC-V vector length is configurable at runtime".to_string())
            }
        }
    }

    /// Emit RISC-V Vector instruction
    fn emit_vector(&mut self, instr: &SimdInstruction) -> Result<(), String> {
        match (instr.op, instr.data_type) {
            // VFADD.VV v0, v0, v1 (add f32 vectors)
            (SimdOp::Add, SimdType::F32) => {
                self.code.extend_from_slice(&[0x57, 0x10, 0x00, 0x02]);
                Ok(())
            }
            // VFADD.VV v0, v0, v1 (add f64 vectors)
            (SimdOp::Add, SimdType::F64) => {
                self.code.extend_from_slice(&[0x57, 0x10, 0x00, 0x06]);
                Ok(())
            }
            // VFSUB.VV v0, v0, v1 (subtract f32 vectors)
            (SimdOp::Sub, SimdType::F32) => {
                self.code.extend_from_slice(&[0x57, 0x10, 0x00, 0x0A]);
                Ok(())
            }
            // VFSUB.VV v0, v0, v1 (subtract f64 vectors)
            (SimdOp::Sub, SimdType::F64) => {
                self.code.extend_from_slice(&[0x57, 0x10, 0x00, 0x0E]);
                Ok(())
            }
            // VFMUL.VV v0, v0, v1 (multiply f32 vectors)
            (SimdOp::Mul, SimdType::F32) => {
                self.code.extend_from_slice(&[0x57, 0x10, 0x00, 0x92]);
                Ok(())
            }
            // VFMUL.VV v0, v0, v1 (multiply f64 vectors)
            (SimdOp::Mul, SimdType::F64) => {
                self.code.extend_from_slice(&[0x57, 0x10, 0x00, 0x96]);
                Ok(())
            }
            // VFDIV.VV v0, v0, v1 (divide f32 vectors)
            (SimdOp::Div, SimdType::F32) => {
                self.code.extend_from_slice(&[0x57, 0x10, 0x00, 0x82]);
                Ok(())
            }
            // VFDIV.VV v0, v0, v1 (divide f64 vectors)
            (SimdOp::Div, SimdType::F64) => {
                self.code.extend_from_slice(&[0x57, 0x10, 0x00, 0x86]);
                Ok(())
            }
            // VLE32.V v0, (a0) (load f32 vector)
            (SimdOp::Load, SimdType::F32) => {
                self.code.extend_from_slice(&[0x07, 0x60, 0x05, 0x02]);
                Ok(())
            }
            // VLE64.V v0, (a0) (load f64 vector)
            (SimdOp::Load, SimdType::F64) => {
                self.code.extend_from_slice(&[0x07, 0x70, 0x05, 0x02]);
                Ok(())
            }
            // VSE32.V v0, (a0) (store f32 vector)
            (SimdOp::Store, SimdType::F32) => {
                self.code.extend_from_slice(&[0x27, 0x60, 0x05, 0x02]);
                Ok(())
            }
            // VSE64.V v0, (a0) (store f64 vector)
            (SimdOp::Store, SimdType::F64) => {
                self.code.extend_from_slice(&[0x27, 0x70, 0x05, 0x02]);
                Ok(())
            }
            // VFMV.V.F v0, fa0 (broadcast f32)
            (SimdOp::Broadcast, SimdType::F32) => {
                self.code.extend_from_slice(&[0x57, 0x50, 0x05, 0x5E]);
                Ok(())
            }
            // VFMV.V.F v0, fa0 (broadcast f64)
            (SimdOp::Broadcast, SimdType::F64) => {
                self.code.extend_from_slice(&[0x57, 0x50, 0x05, 0x5E]);
                Ok(())
            }
            // VFREDSUM.VS v0, v0, v1 (horizontal sum f32)
            (SimdOp::HorizontalSum, SimdType::F32) => {
                self.code.extend_from_slice(&[0x57, 0x10, 0x00, 0x06]);
                Ok(())
            }
            _ => Err(format!(
                "Unsupported RISC-V vector operation: {:?} with type {:?}",
                instr.op, instr.data_type
            )),
        }
    }

    /// Get generated code
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Take generated code
    pub fn take_code(self) -> Vec<u8> {
        self.code
    }
}

impl Default for RiscvSimdCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_riscv_add_f32() {
        let mut gen = RiscvSimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Add, VectorSize::V128, SimdType::F32);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0x57, 0x10, 0x00, 0x02]); // VFADD.VV
    }

    #[test]
    fn test_riscv_mul_f32() {
        let mut gen = RiscvSimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Mul, VectorSize::V128, SimdType::F32);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0x57, 0x10, 0x00, 0x92]); // VFMUL.VV
    }

    #[test]
    fn test_riscv_load_store() {
        let mut gen = RiscvSimdCodeGen::new();

        // Load
        let load = SimdInstruction::new(SimdOp::Load, VectorSize::V128, SimdType::F32);
        gen.emit(&load).unwrap();
        assert_eq!(gen.code()[0..4], [0x07, 0x60, 0x05, 0x02]); // VLE32.V

        // Store
        let store = SimdInstruction::new(SimdOp::Store, VectorSize::V128, SimdType::F32);
        gen.emit(&store).unwrap();
        assert_eq!(gen.code()[4..8], [0x27, 0x60, 0x05, 0x02]); // VSE32.V
    }

    #[test]
    fn test_riscv_broadcast() {
        let mut gen = RiscvSimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Broadcast, VectorSize::V128, SimdType::F32);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0x57, 0x50, 0x05, 0x5E]); // VFMV.V.F
    }

    #[test]
    fn test_v256_not_supported() {
        let mut gen = RiscvSimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Add, VectorSize::V256, SimdType::F32);
        let result = gen.emit(&instr);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("configurable at runtime"));
    }
}
