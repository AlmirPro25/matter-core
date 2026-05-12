//! x86-64 SIMD code generation (SSE/AVX)
//!
//! Generates SSE and AVX instructions for vectorized operations.

use super::{SimdInstruction, SimdOp, SimdType, VectorSize};

/// x86-64 SIMD code generator
pub struct X86SimdCodeGen {
    /// Generated machine code
    code: Vec<u8>,
}

impl X86SimdCodeGen {
    /// Create a new x86-64 SIMD code generator
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }

    /// Emit SIMD instruction
    pub fn emit(&mut self, instr: &SimdInstruction) -> Result<(), String> {
        match instr.vector_size {
            VectorSize::V128 => self.emit_sse(instr),
            VectorSize::V256 => self.emit_avx(instr),
            VectorSize::V512 => self.emit_avx512(instr),
        }
    }

    /// Emit SSE instruction (128-bit)
    fn emit_sse(&mut self, instr: &SimdInstruction) -> Result<(), String> {
        match (instr.op, instr.data_type) {
            // ADDPS xmm0, xmm1 (add packed single-precision)
            (SimdOp::Add, SimdType::F32) => {
                self.code.extend_from_slice(&[0x0F, 0x58, 0xC1]);
                Ok(())
            }
            // ADDPD xmm0, xmm1 (add packed double-precision)
            (SimdOp::Add, SimdType::F64) => {
                self.code.extend_from_slice(&[0x66, 0x0F, 0x58, 0xC1]);
                Ok(())
            }
            // SUBPS xmm0, xmm1 (subtract packed single-precision)
            (SimdOp::Sub, SimdType::F32) => {
                self.code.extend_from_slice(&[0x0F, 0x5C, 0xC1]);
                Ok(())
            }
            // SUBPD xmm0, xmm1 (subtract packed double-precision)
            (SimdOp::Sub, SimdType::F64) => {
                self.code.extend_from_slice(&[0x66, 0x0F, 0x5C, 0xC1]);
                Ok(())
            }
            // MULPS xmm0, xmm1 (multiply packed single-precision)
            (SimdOp::Mul, SimdType::F32) => {
                self.code.extend_from_slice(&[0x0F, 0x59, 0xC1]);
                Ok(())
            }
            // MULPD xmm0, xmm1 (multiply packed double-precision)
            (SimdOp::Mul, SimdType::F64) => {
                self.code.extend_from_slice(&[0x66, 0x0F, 0x59, 0xC1]);
                Ok(())
            }
            // DIVPS xmm0, xmm1 (divide packed single-precision)
            (SimdOp::Div, SimdType::F32) => {
                self.code.extend_from_slice(&[0x0F, 0x5E, 0xC1]);
                Ok(())
            }
            // DIVPD xmm0, xmm1 (divide packed double-precision)
            (SimdOp::Div, SimdType::F64) => {
                self.code.extend_from_slice(&[0x66, 0x0F, 0x5E, 0xC1]);
                Ok(())
            }
            // MOVAPS xmm0, [rax] (load aligned packed single-precision)
            (SimdOp::Load, SimdType::F32) => {
                self.code.extend_from_slice(&[0x0F, 0x28, 0x00]);
                Ok(())
            }
            // MOVAPD xmm0, [rax] (load aligned packed double-precision)
            (SimdOp::Load, SimdType::F64) => {
                self.code.extend_from_slice(&[0x66, 0x0F, 0x28, 0x00]);
                Ok(())
            }
            // MOVAPS [rax], xmm0 (store aligned packed single-precision)
            (SimdOp::Store, SimdType::F32) => {
                self.code.extend_from_slice(&[0x0F, 0x29, 0x00]);
                Ok(())
            }
            // MOVAPD [rax], xmm0 (store aligned packed double-precision)
            (SimdOp::Store, SimdType::F64) => {
                self.code.extend_from_slice(&[0x66, 0x0F, 0x29, 0x00]);
                Ok(())
            }
            // SHUFPS xmm0, xmm0, 0 (broadcast single-precision)
            (SimdOp::Broadcast, SimdType::F32) => {
                self.code.extend_from_slice(&[0x0F, 0xC6, 0xC0, 0x00]);
                Ok(())
            }
            // HADDPS xmm0, xmm0 (horizontal add single-precision)
            (SimdOp::HorizontalSum, SimdType::F32) => {
                self.code.extend_from_slice(&[0xF2, 0x0F, 0x7C, 0xC0]);
                Ok(())
            }
            _ => Err(format!(
                "Unsupported SSE operation: {:?} with type {:?}",
                instr.op, instr.data_type
            )),
        }
    }

    /// Emit AVX instruction (256-bit)
    fn emit_avx(&mut self, instr: &SimdInstruction) -> Result<(), String> {
        match (instr.op, instr.data_type) {
            // VADDPS ymm0, ymm0, ymm1 (add packed single-precision)
            (SimdOp::Add, SimdType::F32) => {
                self.code.extend_from_slice(&[0xC5, 0xFC, 0x58, 0xC1]);
                Ok(())
            }
            // VADDPD ymm0, ymm0, ymm1 (add packed double-precision)
            (SimdOp::Add, SimdType::F64) => {
                self.code.extend_from_slice(&[0xC5, 0xFD, 0x58, 0xC1]);
                Ok(())
            }
            // VSUBPS ymm0, ymm0, ymm1 (subtract packed single-precision)
            (SimdOp::Sub, SimdType::F32) => {
                self.code.extend_from_slice(&[0xC5, 0xFC, 0x5C, 0xC1]);
                Ok(())
            }
            // VSUBPD ymm0, ymm0, ymm1 (subtract packed double-precision)
            (SimdOp::Sub, SimdType::F64) => {
                self.code.extend_from_slice(&[0xC5, 0xFD, 0x5C, 0xC1]);
                Ok(())
            }
            // VMULPS ymm0, ymm0, ymm1 (multiply packed single-precision)
            (SimdOp::Mul, SimdType::F32) => {
                self.code.extend_from_slice(&[0xC5, 0xFC, 0x59, 0xC1]);
                Ok(())
            }
            // VMULPD ymm0, ymm0, ymm1 (multiply packed double-precision)
            (SimdOp::Mul, SimdType::F64) => {
                self.code.extend_from_slice(&[0xC5, 0xFD, 0x59, 0xC1]);
                Ok(())
            }
            // VDIVPS ymm0, ymm0, ymm1 (divide packed single-precision)
            (SimdOp::Div, SimdType::F32) => {
                self.code.extend_from_slice(&[0xC5, 0xFC, 0x5E, 0xC1]);
                Ok(())
            }
            // VDIVPD ymm0, ymm0, ymm1 (divide packed double-precision)
            (SimdOp::Div, SimdType::F64) => {
                self.code.extend_from_slice(&[0xC5, 0xFD, 0x5E, 0xC1]);
                Ok(())
            }
            // VMOVAPS ymm0, [rax] (load aligned packed single-precision)
            (SimdOp::Load, SimdType::F32) => {
                self.code.extend_from_slice(&[0xC5, 0xFC, 0x28, 0x00]);
                Ok(())
            }
            // VMOVAPD ymm0, [rax] (load aligned packed double-precision)
            (SimdOp::Load, SimdType::F64) => {
                self.code.extend_from_slice(&[0xC5, 0xFD, 0x28, 0x00]);
                Ok(())
            }
            // VMOVAPS [rax], ymm0 (store aligned packed single-precision)
            (SimdOp::Store, SimdType::F32) => {
                self.code.extend_from_slice(&[0xC5, 0xFC, 0x29, 0x00]);
                Ok(())
            }
            // VMOVAPD [rax], ymm0 (store aligned packed double-precision)
            (SimdOp::Store, SimdType::F64) => {
                self.code.extend_from_slice(&[0xC5, 0xFD, 0x29, 0x00]);
                Ok(())
            }
            // VBROADCASTSS ymm0, xmm0 (broadcast single-precision)
            (SimdOp::Broadcast, SimdType::F32) => {
                self.code.extend_from_slice(&[0xC4, 0xE2, 0x7D, 0x18, 0xC0]);
                Ok(())
            }
            _ => Err(format!(
                "Unsupported AVX operation: {:?} with type {:?}",
                instr.op, instr.data_type
            )),
        }
    }

    /// Emit AVX-512 instruction (512-bit)
    fn emit_avx512(&mut self, instr: &SimdInstruction) -> Result<(), String> {
        match (instr.op, instr.data_type) {
            // VADDPS zmm0, zmm0, zmm1 (add packed single-precision)
            (SimdOp::Add, SimdType::F32) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0x7C, 0x48, 0x58, 0xC1]);
                Ok(())
            }
            // VADDPD zmm0, zmm0, zmm1 (add packed double-precision)
            (SimdOp::Add, SimdType::F64) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0xFD, 0x48, 0x58, 0xC1]);
                Ok(())
            }
            // VSUBPS zmm0, zmm0, zmm1 (subtract packed single-precision)
            (SimdOp::Sub, SimdType::F32) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0x7C, 0x48, 0x5C, 0xC1]);
                Ok(())
            }
            // VSUBPD zmm0, zmm0, zmm1 (subtract packed double-precision)
            (SimdOp::Sub, SimdType::F64) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0xFD, 0x48, 0x5C, 0xC1]);
                Ok(())
            }
            // VMULPS zmm0, zmm0, zmm1 (multiply packed single-precision)
            (SimdOp::Mul, SimdType::F32) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0x7C, 0x48, 0x59, 0xC1]);
                Ok(())
            }
            // VMULPD zmm0, zmm0, zmm1 (multiply packed double-precision)
            (SimdOp::Mul, SimdType::F64) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0xFD, 0x48, 0x59, 0xC1]);
                Ok(())
            }
            // VDIVPS zmm0, zmm0, zmm1 (divide packed single-precision)
            (SimdOp::Div, SimdType::F32) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0x7C, 0x48, 0x5E, 0xC1]);
                Ok(())
            }
            // VDIVPD zmm0, zmm0, zmm1 (divide packed double-precision)
            (SimdOp::Div, SimdType::F64) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0xFD, 0x48, 0x5E, 0xC1]);
                Ok(())
            }
            // VMOVAPS zmm0, [rax] (load aligned packed single-precision)
            (SimdOp::Load, SimdType::F32) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0x7C, 0x48, 0x28, 0x00]);
                Ok(())
            }
            // VMOVAPD zmm0, [rax] (load aligned packed double-precision)
            (SimdOp::Load, SimdType::F64) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0xFD, 0x48, 0x28, 0x00]);
                Ok(())
            }
            // VMOVAPS [rax], zmm0 (store aligned packed single-precision)
            (SimdOp::Store, SimdType::F32) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0x7C, 0x48, 0x29, 0x00]);
                Ok(())
            }
            // VMOVAPD [rax], zmm0 (store aligned packed double-precision)
            (SimdOp::Store, SimdType::F64) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF1, 0xFD, 0x48, 0x29, 0x00]);
                Ok(())
            }
            // VBROADCASTSS zmm0, xmm0 (broadcast single-precision)
            (SimdOp::Broadcast, SimdType::F32) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF2, 0x7D, 0x48, 0x18, 0xC0]);
                Ok(())
            }
            // VBROADCASTSD zmm0, xmm0 (broadcast double-precision)
            (SimdOp::Broadcast, SimdType::F64) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF2, 0xFD, 0x48, 0x19, 0xC0]);
                Ok(())
            }
            // VFMADD213PS zmm0, zmm1, zmm2 (fused multiply-add single-precision)
            (SimdOp::Fma, SimdType::F32) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF2, 0x75, 0x48, 0xA8, 0xC2]);
                Ok(())
            }
            // VFMADD213PD zmm0, zmm1, zmm2 (fused multiply-add double-precision)
            (SimdOp::Fma, SimdType::F64) => {
                self.code
                    .extend_from_slice(&[0x62, 0xF2, 0xF5, 0x48, 0xA8, 0xC2]);
                Ok(())
            }
            _ => Err(format!(
                "Unsupported AVX-512 operation: {:?} with type {:?}",
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

impl Default for X86SimdCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_add_f32() {
        let mut gen = X86SimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Add, VectorSize::V128, SimdType::F32);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0x0F, 0x58, 0xC1]); // ADDPS xmm0, xmm1
    }

    #[test]
    fn test_sse_add_f64() {
        let mut gen = X86SimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Add, VectorSize::V128, SimdType::F64);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0x66, 0x0F, 0x58, 0xC1]); // ADDPD xmm0, xmm1
    }

    #[test]
    fn test_sse_mul_f32() {
        let mut gen = X86SimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Mul, VectorSize::V128, SimdType::F32);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0x0F, 0x59, 0xC1]); // MULPS xmm0, xmm1
    }

    #[test]
    fn test_avx_add_f32() {
        let mut gen = X86SimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Add, VectorSize::V256, SimdType::F32);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0xC5, 0xFC, 0x58, 0xC1]); // VADDPS ymm0, ymm0, ymm1
    }

    #[test]
    fn test_avx_mul_f64() {
        let mut gen = X86SimdCodeGen::new();
        let instr = SimdInstruction::new(SimdOp::Mul, VectorSize::V256, SimdType::F64);
        gen.emit(&instr).unwrap();
        assert_eq!(gen.code(), &[0xC5, 0xFD, 0x59, 0xC1]); // VMULPD ymm0, ymm0, ymm1
    }

    #[test]
    fn test_sse_load_store() {
        let mut gen = X86SimdCodeGen::new();

        // Load
        let load = SimdInstruction::new(SimdOp::Load, VectorSize::V128, SimdType::F32);
        gen.emit(&load).unwrap();
        assert_eq!(gen.code()[0..3], [0x0F, 0x28, 0x00]); // MOVAPS xmm0, [rax]

        // Store
        let store = SimdInstruction::new(SimdOp::Store, VectorSize::V128, SimdType::F32);
        gen.emit(&store).unwrap();
        assert_eq!(gen.code()[3..6], [0x0F, 0x29, 0x00]); // MOVAPS [rax], xmm0
    }

    #[test]
    fn test_multiple_instructions() {
        let mut gen = X86SimdCodeGen::new();

        // Add
        let add = SimdInstruction::new(SimdOp::Add, VectorSize::V128, SimdType::F32);
        gen.emit(&add).unwrap();

        // Multiply
        let mul = SimdInstruction::new(SimdOp::Mul, VectorSize::V128, SimdType::F32);
        gen.emit(&mul).unwrap();

        // Should have both instructions
        assert_eq!(gen.code().len(), 6); // 3 bytes each
    }
}

#[test]
fn test_avx512_add_f32() {
    let mut gen = X86SimdCodeGen::new();
    let instr = SimdInstruction::new(SimdOp::Add, VectorSize::V512, SimdType::F32);
    gen.emit(&instr).unwrap();
    assert_eq!(gen.code(), &[0x62, 0xF1, 0x7C, 0x48, 0x58, 0xC1]); // VADDPS zmm0, zmm0, zmm1
}

#[test]
fn test_avx512_add_f64() {
    let mut gen = X86SimdCodeGen::new();
    let instr = SimdInstruction::new(SimdOp::Add, VectorSize::V512, SimdType::F64);
    gen.emit(&instr).unwrap();
    assert_eq!(gen.code(), &[0x62, 0xF1, 0xFD, 0x48, 0x58, 0xC1]); // VADDPD zmm0, zmm0, zmm1
}

#[test]
fn test_avx512_mul_f32() {
    let mut gen = X86SimdCodeGen::new();
    let instr = SimdInstruction::new(SimdOp::Mul, VectorSize::V512, SimdType::F32);
    gen.emit(&instr).unwrap();
    assert_eq!(gen.code(), &[0x62, 0xF1, 0x7C, 0x48, 0x59, 0xC1]); // VMULPS zmm0, zmm0, zmm1
}

#[test]
fn test_avx512_fma_f32() {
    let mut gen = X86SimdCodeGen::new();
    let instr = SimdInstruction::new(SimdOp::Fma, VectorSize::V512, SimdType::F32);
    gen.emit(&instr).unwrap();
    assert_eq!(gen.code(), &[0x62, 0xF2, 0x75, 0x48, 0xA8, 0xC2]); // VFMADD213PS zmm0, zmm1, zmm2
}

#[test]
fn test_avx512_fma_f64() {
    let mut gen = X86SimdCodeGen::new();
    let instr = SimdInstruction::new(SimdOp::Fma, VectorSize::V512, SimdType::F64);
    gen.emit(&instr).unwrap();
    assert_eq!(gen.code(), &[0x62, 0xF2, 0xF5, 0x48, 0xA8, 0xC2]); // VFMADD213PD zmm0, zmm1, zmm2
}

#[test]
fn test_avx512_load_store() {
    let mut gen = X86SimdCodeGen::new();

    // Load
    let load = SimdInstruction::new(SimdOp::Load, VectorSize::V512, SimdType::F32);
    gen.emit(&load).unwrap();
    assert_eq!(gen.code()[0..6], [0x62, 0xF1, 0x7C, 0x48, 0x28, 0x00]); // VMOVAPS zmm0, [rax]

    // Store
    let store = SimdInstruction::new(SimdOp::Store, VectorSize::V512, SimdType::F32);
    gen.emit(&store).unwrap();
    assert_eq!(gen.code()[6..12], [0x62, 0xF1, 0x7C, 0x48, 0x29, 0x00]); // VMOVAPS [rax], zmm0
}

#[test]
fn test_avx512_broadcast() {
    let mut gen = X86SimdCodeGen::new();
    let instr = SimdInstruction::new(SimdOp::Broadcast, VectorSize::V512, SimdType::F32);
    gen.emit(&instr).unwrap();
    assert_eq!(gen.code(), &[0x62, 0xF2, 0x7D, 0x48, 0x18, 0xC0]); // VBROADCASTSS zmm0, xmm0
}
