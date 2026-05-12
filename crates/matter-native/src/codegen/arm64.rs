//! ARM64 (AArch64) code generator
//!
//! Basic ARM64 code generation for Matter bytecode.
//! Implements essential instructions for Turing-complete execution.

// Error format convention (keep consistent across backends):
// - Prefix: "<BACKEND> backend: <summary>"
// - Optional context: "[context:key=value,...]"
// - Quantitative diagnostics when relevant: "needed N, available M"
use matter_bytecode::{Bytecode, Constant, Instruction};

/// ARM64 registers
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Arm64Register {
    X0 = 0,   // Return value / argument 1
    X1 = 1,   // Argument 2
    X2 = 2,   // Argument 3
    X3 = 3,   // Argument 4
    X4 = 4,   // Argument 5
    X5 = 5,   // Argument 6
    X6 = 6,   // Argument 7
    X7 = 7,   // Argument 8
    X8 = 8,   // Indirect result
    X9 = 9,   // Temporary
    X10 = 10, // Temporary
    X11 = 11, // Temporary
    X12 = 12, // Temporary
    X13 = 13, // Temporary
    X14 = 14, // Temporary
    X15 = 15, // Temporary
    X16 = 16, // IP0
    X17 = 17, // IP1
    X18 = 18, // Platform register
    X19 = 19, // Callee-saved
    X20 = 20, // Callee-saved
    X21 = 21, // Callee-saved
    X22 = 22, // Callee-saved
    X23 = 23, // Callee-saved
    X24 = 24, // Callee-saved
    X25 = 25, // Callee-saved
    X26 = 26, // Callee-saved
    X27 = 27, // Callee-saved
    X28 = 28, // Callee-saved
    X29 = 29, // Frame pointer
    X30 = 30, // Link register
    SP = 31,  // Stack pointer
}

impl Arm64Register {
    fn encoding(&self) -> u32 {
        *self as u32
    }
}

/// ARM64 code generator
pub struct Arm64CodeGen {
    code: Vec<u8>,
}

impl Arm64CodeGen {
    fn context_block(context: &str) -> String {
        format!("[context:{}]", context)
    }

    fn err_unimplemented(instr: &Instruction) -> String {
        format!(
            "ARM64 backend: unimplemented instruction {}",
            Self::context_block(&format!("instr={:?}", instr))
        )
    }

    fn err_load_const_oob(id: usize, constants_len: usize) -> String {
        format!(
            "ARM64 backend: LoadConst index out of bounds {}",
            Self::context_block(&format!("id={},constants={}", id, constants_len))
        )
    }

    fn err_unsupported_const(id: usize, constant: &Constant) -> String {
        format!(
            "ARM64 backend: unsupported constant in LoadConst {}",
            Self::context_block(&format!("id={},constant={:?}", id, constant))
        )
    }

    fn err_int_out_of_immediate_range(id: usize, constant: i64) -> String {
        format!(
            "ARM64 backend: integer constant out of immediate range {}",
            Self::context_block(&format!("id={},constant={}", id, constant))
        )
    }

    pub fn new() -> Self {
        Self { code: Vec::new() }
    }

    pub fn compile(&mut self, bytecode: &Bytecode) -> Result<Vec<u8>, String> {
        // Emit prologue
        self.emit_prologue();

        // Compile main instructions
        for instr in &bytecode.main_instructions {
            self.compile_instruction(instr, &bytecode.constants)?;
        }

        // Emit epilogue
        self.emit_epilogue();

        Ok(self.code.clone())
    }

    fn compile_instruction(
        &mut self,
        instr: &Instruction,
        constants: &[Constant],
    ) -> Result<(), String> {
        match instr {
            Instruction::LoadConst(id) => {
                self.compile_load_const(*id, constants)?;
            }
            Instruction::Add => {
                self.compile_add()?;
            }
            Instruction::Sub => {
                self.compile_sub()?;
            }
            Instruction::Mul => {
                self.compile_mul()?;
            }
            Instruction::Div => {
                self.compile_div()?;
            }
            Instruction::Eq => {
                self.compile_eq()?;
            }
            Instruction::NotEq => {
                self.compile_not_eq()?;
            }
            Instruction::Lt => {
                self.compile_lt()?;
            }
            Instruction::Gt => {
                self.compile_gt()?;
            }
            Instruction::LtEq => {
                self.compile_lt_eq()?;
            }
            Instruction::GtEq => {
                self.compile_gt_eq()?;
            }
            Instruction::Jump(_target) => {
                self.compile_jump()?;
            }
            Instruction::JumpIfFalse(_target) => {
                self.compile_jump_if_false()?;
            }
            Instruction::LoadLocal(_name) => {
                self.compile_load_local()?;
            }
            Instruction::StoreLocal(_name) => {
                self.compile_store_local()?;
            }
            Instruction::Call(_) | Instruction::CallNamed { .. } => {
                self.compile_call()?;
            }
            Instruction::Print => {
                self.compile_print()?;
            }
            Instruction::Halt | Instruction::Return => {
                self.compile_return()?;
            }
            _ => {
                return Err(Self::err_unimplemented(instr));
            }
        }
        Ok(())
    }

    fn compile_load_const(&mut self, id: usize, constants: &[Constant]) -> Result<(), String> {
        if id >= constants.len() {
            return Err(Self::err_load_const_oob(id, constants.len()));
        }

        match &constants[id] {
            Constant::Int(n) => {
                // MOV X0, #n (simplified - only works for small immediates)
                let imm =
                    u32::try_from(*n).map_err(|_| Self::err_int_out_of_immediate_range(id, *n))?;
                self.emit_mov_imm(Arm64Register::X0, imm);
            }
            other => {
                return Err(Self::err_unsupported_const(id, other));
            }
        }

        Ok(())
    }

    fn compile_add(&mut self) -> Result<(), String> {
        // ADD X0, X0, X1
        self.emit_add(Arm64Register::X0, Arm64Register::X0, Arm64Register::X1);
        Ok(())
    }

    fn compile_sub(&mut self) -> Result<(), String> {
        // SUB X0, X0, X1
        self.emit_sub(Arm64Register::X0, Arm64Register::X0, Arm64Register::X1);
        Ok(())
    }

    fn compile_mul(&mut self) -> Result<(), String> {
        // MUL X0, X0, X1
        self.emit_mul(Arm64Register::X0, Arm64Register::X0, Arm64Register::X1);
        Ok(())
    }

    fn compile_print(&mut self) -> Result<(), String> {
        // Stub - would call runtime function
        self.emit_nop();
        Ok(())
    }

    fn compile_return(&mut self) -> Result<(), String> {
        // RET
        self.emit_ret();
        Ok(())
    }

    fn compile_div(&mut self) -> Result<(), String> {
        // SDIV X0, X0, X1
        self.emit_sdiv(Arm64Register::X0, Arm64Register::X0, Arm64Register::X1);
        Ok(())
    }

    fn compile_eq(&mut self) -> Result<(), String> {
        // CMP X0, X1
        self.emit_cmp(Arm64Register::X0, Arm64Register::X1);
        // CSET X0, EQ
        self.emit_cset(Arm64Register::X0, 0); // EQ = 0
        Ok(())
    }

    fn compile_not_eq(&mut self) -> Result<(), String> {
        // CMP X0, X1
        self.emit_cmp(Arm64Register::X0, Arm64Register::X1);
        // CSET X0, NE
        self.emit_cset(Arm64Register::X0, 1); // NE = 1
        Ok(())
    }

    fn compile_lt(&mut self) -> Result<(), String> {
        // CMP X0, X1
        self.emit_cmp(Arm64Register::X0, Arm64Register::X1);
        // CSET X0, LT
        self.emit_cset(Arm64Register::X0, 11); // LT = 11
        Ok(())
    }

    fn compile_gt(&mut self) -> Result<(), String> {
        // CMP X0, X1
        self.emit_cmp(Arm64Register::X0, Arm64Register::X1);
        // CSET X0, GT
        self.emit_cset(Arm64Register::X0, 12); // GT = 12
        Ok(())
    }

    fn compile_lt_eq(&mut self) -> Result<(), String> {
        // CMP X0, X1
        self.emit_cmp(Arm64Register::X0, Arm64Register::X1);
        // CSET X0, LE
        self.emit_cset(Arm64Register::X0, 13); // LE = 13
        Ok(())
    }

    fn compile_gt_eq(&mut self) -> Result<(), String> {
        // CMP X0, X1
        self.emit_cmp(Arm64Register::X0, Arm64Register::X1);
        // CSET X0, GE
        self.emit_cset(Arm64Register::X0, 10); // GE = 10
        Ok(())
    }

    fn compile_jump(&mut self) -> Result<(), String> {
        // B (unconditional branch) - placeholder
        self.emit_b(0);
        Ok(())
    }

    fn compile_jump_if_false(&mut self) -> Result<(), String> {
        // CBZ X0, target (compare and branch if zero)
        self.emit_cbz(Arm64Register::X0, 0);
        Ok(())
    }

    fn compile_load_local(&mut self) -> Result<(), String> {
        // LDR X0, [X29, #offset] - stub
        self.emit_ldr(Arm64Register::X0, Arm64Register::X29, -16);
        Ok(())
    }

    fn compile_store_local(&mut self) -> Result<(), String> {
        // STR X0, [X29, #offset] - stub
        self.emit_str(Arm64Register::X0, Arm64Register::X29, -16);
        Ok(())
    }

    fn compile_call(&mut self) -> Result<(), String> {
        // BL (branch with link) - stub
        self.emit_bl(0);
        Ok(())
    }

    // ============================================================================
    // INSTRUCTION EMISSION
    // ============================================================================

    fn emit_prologue(&mut self) {
        // STP X29, X30, [SP, #-16]!
        self.emit_stp(
            Arm64Register::X29,
            Arm64Register::X30,
            Arm64Register::SP,
            -16,
        );
        // MOV X29, SP
        self.emit_mov_reg(Arm64Register::X29, Arm64Register::SP);
    }

    fn emit_epilogue(&mut self) {
        // MOV SP, X29
        self.emit_mov_reg(Arm64Register::SP, Arm64Register::X29);
        // LDP X29, X30, [SP], #16
        self.emit_ldp(
            Arm64Register::X29,
            Arm64Register::X30,
            Arm64Register::SP,
            16,
        );
        // RET
        self.emit_ret();
    }

    /// Emit: MOV Xd, #imm
    fn emit_mov_imm(&mut self, dest: Arm64Register, imm: u32) {
        // MOVZ Xd, #imm16, LSL #0
        // Encoding: 1101 0010 100 imm16 000000 Rd
        let instr = 0xD2800000 | ((imm & 0xFFFF) << 5) | dest.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: MOV Xd, Xn
    fn emit_mov_reg(&mut self, dest: Arm64Register, src: Arm64Register) {
        // ORR Xd, XZR, Xn (MOV is alias)
        // Encoding: 1010 1010 000 Rm 000000 Rn Rd
        let instr = 0xAA0003E0 | (src.encoding() << 16) | dest.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: ADD Xd, Xn, Xm
    fn emit_add(&mut self, dest: Arm64Register, src1: Arm64Register, src2: Arm64Register) {
        // ADD Xd, Xn, Xm
        // Encoding: 1000 1011 000 Rm 000000 Rn Rd
        let instr = 0x8B000000 | (src2.encoding() << 16) | (src1.encoding() << 5) | dest.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: SUB Xd, Xn, Xm
    fn emit_sub(&mut self, dest: Arm64Register, src1: Arm64Register, src2: Arm64Register) {
        // SUB Xd, Xn, Xm
        // Encoding: 1100 1011 000 Rm 000000 Rn Rd
        let instr = 0xCB000000 | (src2.encoding() << 16) | (src1.encoding() << 5) | dest.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: MUL Xd, Xn, Xm
    fn emit_mul(&mut self, dest: Arm64Register, src1: Arm64Register, src2: Arm64Register) {
        // MADD Xd, Xn, Xm, XZR (MUL is alias)
        // Encoding: 1001 1011 000 Rm 011111 Rn Rd
        let instr = 0x9B007C00 | (src2.encoding() << 16) | (src1.encoding() << 5) | dest.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: NOP
    fn emit_nop(&mut self) {
        // NOP
        let instr = 0xD503201Fu32;
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: STP Xt1, Xt2, [Xn, #imm]!
    fn emit_stp(
        &mut self,
        reg1: Arm64Register,
        reg2: Arm64Register,
        base: Arm64Register,
        offset: i32,
    ) {
        // STP Xt1, Xt2, [Xn, #imm]! (pre-indexed)
        let imm7 = ((offset / 8) & 0x7F) as u32;
        let instr = 0xA9800000
            | (imm7 << 15)
            | (reg2.encoding() << 10)
            | (base.encoding() << 5)
            | reg1.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: LDP Xt1, Xt2, [Xn], #imm
    fn emit_ldp(
        &mut self,
        reg1: Arm64Register,
        reg2: Arm64Register,
        base: Arm64Register,
        offset: i32,
    ) {
        // LDP Xt1, Xt2, [Xn], #imm (post-indexed)
        let imm7 = ((offset / 8) & 0x7F) as u32;
        let instr = 0xA8C00000
            | (imm7 << 15)
            | (reg2.encoding() << 10)
            | (base.encoding() << 5)
            | reg1.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: RET
    fn emit_ret(&mut self) {
        // RET (return to X30)
        // Encoding: 1101 0110 0101 1111 0000 00 11110 00000
        let instr = 0xD65F03C0u32;
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: SDIV Xd, Xn, Xm (signed division)
    fn emit_sdiv(&mut self, dest: Arm64Register, src1: Arm64Register, src2: Arm64Register) {
        // SDIV Xd, Xn, Xm
        // Encoding: 1001 1010 110 Rm 000011 Rn Rd
        let instr = 0x9AC00C00 | (src2.encoding() << 16) | (src1.encoding() << 5) | dest.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: CMP Xn, Xm (compare)
    fn emit_cmp(&mut self, src1: Arm64Register, src2: Arm64Register) {
        // CMP is alias for SUBS XZR, Xn, Xm
        // Encoding: 1110 1011 000 Rm 000000 Rn 11111
        let instr = 0xEB00001F | (src2.encoding() << 16) | (src1.encoding() << 5);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: CSET Xd, cond (conditional set)
    fn emit_cset(&mut self, dest: Arm64Register, cond: u32) {
        // CSET is alias for CSINC Xd, XZR, XZR, invert(cond)
        // Encoding: 1001 1010 100 11111 cond 01 11111 Rd
        let instr = 0x9A9F07E0 | ((cond ^ 1) << 12) | dest.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: B offset (unconditional branch)
    fn emit_b(&mut self, offset: i32) {
        // B imm26
        // Encoding: 0001 01 imm26
        let imm26 = ((offset / 4) & 0x3FFFFFF) as u32;
        let instr = 0x14000000 | imm26;
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: CBZ Xt, offset (compare and branch if zero)
    fn emit_cbz(&mut self, reg: Arm64Register, offset: i32) {
        // CBZ Xt, imm19
        // Encoding: 1011 0100 imm19 Rt
        let imm19 = ((offset / 4) & 0x7FFFF) as u32;
        let instr = 0xB4000000 | (imm19 << 5) | reg.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: LDR Xt, [Xn, #offset]
    fn emit_ldr(&mut self, dest: Arm64Register, base: Arm64Register, offset: i32) {
        // LDR Xt, [Xn, #imm12]
        // Encoding: 1111 1000 01 imm12 Rn Rt
        let imm12 = ((offset / 8) & 0xFFF) as u32;
        let instr = 0xF9400000 | (imm12 << 10) | (base.encoding() << 5) | dest.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: STR Xt, [Xn, #offset]
    fn emit_str(&mut self, src: Arm64Register, base: Arm64Register, offset: i32) {
        // STR Xt, [Xn, #imm12]
        // Encoding: 1111 1000 00 imm12 Rn Rt
        let imm12 = ((offset / 8) & 0xFFF) as u32;
        let instr = 0xF9000000 | (imm12 << 10) | (base.encoding() << 5) | src.encoding();
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: BL offset (branch with link)
    fn emit_bl(&mut self, offset: i32) {
        // BL imm26
        // Encoding: 1001 01 imm26
        let imm26 = ((offset / 4) & 0x3FFFFFF) as u32;
        let instr = 0x94000000 | imm26;
        self.code.extend_from_slice(&instr.to_le_bytes());
    }
}

impl Default for Arm64CodeGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arm64_codegen_creation() {
        let codegen = Arm64CodeGen::new();
        assert_eq!(codegen.code.len(), 0);
    }

    #[test]
    fn test_arm64_simple_program() {
        let mut bytecode = Bytecode::new();
        let c42 = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![Instruction::LoadConst(c42), Instruction::Halt];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_arm64_arithmetic() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::Add,
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_arm64_subtraction() {
        let mut bytecode = Bytecode::new();
        let c30 = bytecode.add_constant(Constant::Int(30));
        let c10 = bytecode.add_constant(Constant::Int(10));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c30),
            Instruction::LoadConst(c10),
            Instruction::Sub,
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_arm64_multiplication() {
        let mut bytecode = Bytecode::new();
        let c5 = bytecode.add_constant(Constant::Int(5));
        let c6 = bytecode.add_constant(Constant::Int(6));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c5),
            Instruction::LoadConst(c6),
            Instruction::Mul,
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_arm64_division() {
        let mut bytecode = Bytecode::new();
        let c20 = bytecode.add_constant(Constant::Int(20));
        let c4 = bytecode.add_constant(Constant::Int(4));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c20),
            Instruction::LoadConst(c4),
            Instruction::Div,
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_arm64_comparisons() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::Lt,
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_arm64_control_flow() {
        let mut bytecode = Bytecode::new();
        let c0 = bytecode.add_constant(Constant::Int(0));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c0),
            Instruction::JumpIfFalse(3),
            Instruction::LoadConst(c0),
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_arm64_function_call() {
        let mut bytecode = Bytecode::new();
        let c42 = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c42),
            Instruction::Call(1),
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_arm64_locals() {
        let mut bytecode = Bytecode::new();
        let c100 = bytecode.add_constant(Constant::Int(100));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c100),
            Instruction::StoreLocal("x".to_string()),
            Instruction::LoadLocal("x".to_string()),
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_arm64_unimplemented_instruction_returns_error() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::NewList(0), Instruction::Halt];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("ARM64 backend: unimplemented instruction"));
        assert!(err.contains("[context:instr=NewList(0)]"));
    }

    #[test]
    fn test_arm64_unimplemented_instruction_preserves_payload_context() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![
            Instruction::BackendCall {
                backend: "http".to_string(),
                method: "get".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("ARM64 backend: unimplemented instruction"));
        assert!(err.contains("backend: \"http\""));
        assert!(err.contains("method: \"get\""));
        assert!(err.contains("arg_count: 2"));
    }

    #[test]
    fn test_arm64_unsupported_constant_returns_error() {
        let mut bytecode = Bytecode::new();
        let cbool = bytecode.add_constant(Constant::Bool(true));
        bytecode.main_instructions = vec![Instruction::LoadConst(cbool), Instruction::Halt];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("ARM64 backend: unsupported constant in LoadConst"));
        assert!(err.contains("[context:id=0,constant=Bool(true)]"));
    }

    #[test]
    fn test_arm64_unsupported_string_constant_preserves_context_payload() {
        let mut bytecode = Bytecode::new();
        let cstr = bytecode.add_constant(Constant::String("alpha".to_string()));
        bytecode.main_instructions = vec![Instruction::LoadConst(cstr), Instruction::Halt];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("ARM64 backend: unsupported constant in LoadConst"));
        assert!(err.contains("constant=String(\"alpha\")"));
    }

    #[test]
    fn test_arm64_int_out_of_immediate_range_returns_error() {
        let mut bytecode = Bytecode::new();
        let c = bytecode.add_constant(Constant::Int(-1));
        bytecode.main_instructions = vec![Instruction::LoadConst(c), Instruction::Halt];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("ARM64 backend: integer constant out of immediate range"));
    }

    #[test]
    fn test_arm64_int_u32_max_is_accepted() {
        let mut bytecode = Bytecode::new();
        let c = bytecode.add_constant(Constant::Int(u32::MAX as i64));
        bytecode.main_instructions = vec![Instruction::LoadConst(c), Instruction::Halt];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_arm64_int_u32_max_plus_one_is_rejected() {
        let mut bytecode = Bytecode::new();
        let c = bytecode.add_constant(Constant::Int(u32::MAX as i64 + 1));
        bytecode.main_instructions = vec![Instruction::LoadConst(c), Instruction::Halt];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("ARM64 backend: integer constant out of immediate range"));
    }

    #[test]
    fn test_arm64_constant_index_out_of_bounds_returns_error() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::LoadConst(999), Instruction::Halt];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("ARM64 backend: LoadConst index out of bounds"));
        assert!(err.contains("[context:id=999,constants=0]"));
    }

    #[test]
    fn test_arm64_constant_index_oob_context_reports_nonzero_constants_len() {
        let mut bytecode = Bytecode::new();
        let _c0 = bytecode.add_constant(Constant::Int(7));
        bytecode.main_instructions = vec![Instruction::LoadConst(2), Instruction::Halt];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("ARM64 backend: LoadConst index out of bounds"));
        assert!(err.contains("[context:id=2,constants=1]"));
    }

    #[test]
    fn test_arm64_unsupported_constant_error_includes_id() {
        let mut bytecode = Bytecode::new();
        let cint = bytecode.add_constant(Constant::Int(1));
        let cbool = bytecode.add_constant(Constant::Bool(true));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(cint),
            Instruction::LoadConst(cbool),
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("[context:id=1,constant=Bool(true)]"));
    }

    #[test]
    fn test_arm64_range_error_includes_id() {
        let mut bytecode = Bytecode::new();
        let cint = bytecode.add_constant(Constant::Int(1));
        let cout = bytecode.add_constant(Constant::Int(u32::MAX as i64 + 1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(cint),
            Instruction::LoadConst(cout),
            Instruction::Halt,
        ];

        let mut codegen = Arm64CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("[context:id=1,constant=4294967296]"));
    }

    #[test]
    fn test_arm64_error_format_uses_context_block() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::LoadConst(999), Instruction::Halt];

        let mut codegen = Arm64CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("expected LoadConst oob error");

        assert!(err.starts_with("ARM64 backend:"));
        assert!(err.contains("[context:"));
    }
}
