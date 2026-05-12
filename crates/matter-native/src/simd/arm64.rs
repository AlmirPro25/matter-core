//! ARM64 SIMD code generation (NEON)
//!
//! Generates NEON instructions for vectorized operations on ARM64.

use super::{SimdInstruction, SimdOp, SimdType, VectorSize};

/// ARM64 SIMD code generator
pub struct Arm64SimdCodeGen {
    /// Generated machine code
    code: Vec<u8>,
}

impl Arm64SimdCodeGen {
    /// Create a new ARM64 SIMD code generator
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }

    /// Emit SIMD instruction
    pub fn emit(&mut self, instr: &SimdInstruction) -> Result<(), String> {
        match instr.vector_size {
            VectorSize::V128 => self.emit_neon(instr),
            VectorSize::V256 | VectorSize::V512 => {
                Err("ARM64 only supports 128-bit NEON vectors".to_string())
            }
        }
    }

    /// Emit NEON instruction (128-bit)
    fn emit_neon(&mut self, instr: &SimdInstruction) -> Result<(), String> {
        match (instr.op, instr.data_type) {
            // FADD v0.4s, v0.4s, v1.4s (add 4x f32)
            (SimdOp::Add, SimdType::F32) => {
                self.code.extend_from_slice(&[0x20, 0xD4, 0x21, 0x4E]);
                Ok(())
            }
            // FADD v0.2d, v0.2d, v1.2d (add 2x f64)
            (SimdOp::Add, SimdType::F64) => {
                self.code.extend_from_slice(&[0x20, 0xD4, 0x61, 0x4E]);
                Ok(())
            }
            // FSUB v0.4s, v0.4s, v1.4s (subtract 4x f32)
            (SimdOp::Sub, SimdType::F32) => {
                self.code.extend_from_slice(&[0x20, 0xD4, 0xA1, 0x4E]);
                Ok(())
            }
            // FSUB v0.2d, v0.2d, v1.2d (subtract 2x f64)
            (SimdOp::Sub, SimdType::F64) => {
                self.code.extend_from_slice(&[0x20, 0xD4, 0xE1, 0x4E]);
                Ok(())
            }
            // FMUL v0.4s, v0.4s, v1.4s (multiply 4x f32)
            (SimdOp::Mul, SimdType::F32) => {
                self.code.extend_from_slice(&[0x20, 0xDC, 0x21, 0x6E]);
                Ok(())
            }
            // FMUL v0.2d, v0.2d, v1.2d (multiply 2x f64)
            (SimdOp::Mul, SimdType::F64) => {
                self.code.extend_from_slice(&[0x20, 0xDC, 0x61, 0x6E]);
                Ok(())
            }
            // FDIV v0.4s, v0.4s, v1.4s (divide 4x f32)
            (SimdOp::Div, SimdType::F32) => {
                self.code.extend_from_slice(&[0x20, 0xFC, 0x21, 0x6E]);
                Ok(())
            }
            // FDIV v0.2d, v0.2d, v1.2d (divide 2x f64)
            (SimdOp::Div, SimdType::F64) => {
                self.code.extend_from_slice(&[0x20, 0xFC, 0x61, 0x6E]);
                Ok(())
            }
            // LDR q0, [x0] (load 128-bit)
            (SimdOp::Load, SimdType::F32) | (SimdOp::Load, SimdType::F64) => {
                self.code.extend_from_slice(&[0x00, 0x00, 0xC0, 0x3D]);
                Ok(())
            }
            // STR q0, [x0] (store 128-bit)
            (SimdOp::Store, SimdType::F32) | (SimdOp::Store, SimdType::F64) => {
                self.code.extend_from_slice(&[0x00, 0x00, 0x80, 0x3D]);
                Ok(())
            }
            // DUP v0.4s, v0.s[0] (broadcast f32)
            (SimdOp::Broadcast, SimdType::F32) => {
                self.code.extend_from_slice(&[0x00, 0x04, 0x04, 0x4E]);
                Ok(())
            }
            // DUP v0.2d, v0.d[0] (broadcast f64)
            (SimdOp::Broadcast, SimdType::F64) => {
                self.code.extend_from_slice(&[0x00, 0x04, 0x08, 0x4E]);
                Ok(())
            }
            // FADDP v0.4s, v0.4s, v0.4s (horizontal add f32)
            (SimdOp::HorizontalSum, SimdType::F32) => {
                self.code.extend_from_slice(&[0x00, 0xD4, 0x20, 0x6E]);
                Ok(())
            }
            _ => Err(format!(
                "Unsupported NEON operation: {:?} with type {:?}",
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

impl Default for Arm64SimdCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neon_add_f32() {
        let mut gen = Arm64SimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Add, VectorSize::V128, SimdType::F32);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0x20, 0xD4, 0x21, 0x4E]); // FADD v0.4s, v0.4s, v1.4s
    }

    #[test]
    fn test_neon_add_f64() {
        let mut gen = Arm64SimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Add, VectorSize::V128, SimdType::F64);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0x20, 0xD4, 0x61, 0x4E]); // FADD v0.2d, v0.2d, v1.2d
    }

    #[test]
    fn test_neon_mul_f32() {
        let mut gen = Arm64SimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Mul, VectorSize::V128, SimdType::F32);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0x20, 0xDC, 0x21, 0x6E]); // FMUL v0.4s, v0.4s, v1.4s
    }

    #[test]
    fn test_neon_load_store() {
        let mut gen = Arm64SimdCodeGen::new();

        // Load
        let load = SimdInstruction::new(SimdOp::Load, VectorSize::V128, SimdType::F32);
        gen.emit(&load).unwrap();
        assert_eq!(gen.code()[0..4], [0x00, 0x00, 0xC0, 0x3D]); // LDR q0, [x0]

        // Store
        let store = SimdInstruction::new(SimdOp::Store, VectorSize::V128, SimdType::F32);
        gen.emit(&store).unwrap();
        assert_eq!(gen.code()[4..8], [0x00, 0x00, 0x80, 0x3D]); // STR q0, [x0]
    }

    #[test]
    fn test_neon_broadcast() {
        let mut gen = Arm64SimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Broadcast, VectorSize::V128, SimdType::F32);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0x00, 0x04, 0x04, 0x4E]); // DUP v0.4s, v0.s[0]
    }

    #[test]
    fn test_v256_not_supported() {
        let mut gen = Arm64SimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Add, VectorSize::V256, SimdType::F32);
        let result = gen.emit(&instr);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ARM64 only supports 128-bit"));
    }
}
