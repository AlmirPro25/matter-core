//! RISC-V 64-bit code generator
//!
//! Basic RISC-V code generation for Matter bytecode.
//! Implements essential instructions for Turing-complete execution.

// Error format convention (keep consistent across backends):
// - Prefix: "<BACKEND> backend: <summary>"
// - Optional context: "[context:key=value,...]"
// - Quantitative diagnostics when relevant: "needed N, available M"
use matter_bytecode::{Bytecode, Constant, Instruction};

/// RISC-V registers
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum RiscVRegister {
    Zero = 0, // x0 - Hard-wired zero
    Ra = 1,   // x1 - Return address
    Sp = 2,   // x2 - Stack pointer
    Gp = 3,   // x3 - Global pointer
    Tp = 4,   // x4 - Thread pointer
    T0 = 5,   // x5 - Temporary
    T1 = 6,   // x6 - Temporary
    T2 = 7,   // x7 - Temporary
    S0 = 8,   // x8 - Saved register / Frame pointer
    S1 = 9,   // x9 - Saved register
    A0 = 10,  // x10 - Function argument / return value
    A1 = 11,  // x11 - Function argument / return value
    A2 = 12,  // x12 - Function argument
    A3 = 13,  // x13 - Function argument
    A4 = 14,  // x14 - Function argument
    A5 = 15,  // x15 - Function argument
    A6 = 16,  // x16 - Function argument
    A7 = 17,  // x17 - Function argument
    S2 = 18,  // x18 - Saved register
    S3 = 19,  // x19 - Saved register
    S4 = 20,  // x20 - Saved register
    S5 = 21,  // x21 - Saved register
    S6 = 22,  // x22 - Saved register
    S7 = 23,  // x23 - Saved register
    S8 = 24,  // x24 - Saved register
    S9 = 25,  // x25 - Saved register
    S10 = 26, // x26 - Saved register
    S11 = 27, // x27 - Saved register
    T3 = 28,  // x28 - Temporary
    T4 = 29,  // x29 - Temporary
    T5 = 30,  // x30 - Temporary
    T6 = 31,  // x31 - Temporary
}

impl RiscVRegister {
    fn encoding(&self) -> u32 {
        *self as u32
    }
}

/// RISC-V code generator
pub struct RiscVCodeGen {
    code: Vec<u8>,
}

impl RiscVCodeGen {
    fn context_block(context: &str) -> String {
        format!("[context:{}]", context)
    }

    fn err_unimplemented(instr: &Instruction) -> String {
        format!(
            "RISC-V backend: unimplemented instruction {}",
            Self::context_block(&format!("instr={:?}", instr))
        )
    }

    fn err_load_const_oob(id: usize, constants_len: usize) -> String {
        format!(
            "RISC-V backend: LoadConst index out of bounds {}",
            Self::context_block(&format!("id={},constants={}", id, constants_len))
        )
    }

    fn err_unsupported_const(id: usize, constant: &Constant) -> String {
        format!(
            "RISC-V backend: unsupported constant in LoadConst {}",
            Self::context_block(&format!("id={},constant={:?}", id, constant))
        )
    }

    fn err_int_out_of_immediate_range(id: usize, constant: i64) -> String {
        format!(
            "RISC-V backend: integer constant out of immediate range {}",
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
                // LI a0, n (load immediate)
                let imm =
                    i32::try_from(*n).map_err(|_| Self::err_int_out_of_immediate_range(id, *n))?;
                self.emit_li(RiscVRegister::A0, imm);
            }
            other => {
                return Err(Self::err_unsupported_const(id, other));
            }
        }

        Ok(())
    }

    fn compile_add(&mut self) -> Result<(), String> {
        // ADD a0, a0, a1
        self.emit_add(RiscVRegister::A0, RiscVRegister::A0, RiscVRegister::A1);
        Ok(())
    }

    fn compile_sub(&mut self) -> Result<(), String> {
        // SUB a0, a0, a1
        self.emit_sub(RiscVRegister::A0, RiscVRegister::A0, RiscVRegister::A1);
        Ok(())
    }

    fn compile_mul(&mut self) -> Result<(), String> {
        // MUL a0, a0, a1
        self.emit_mul(RiscVRegister::A0, RiscVRegister::A0, RiscVRegister::A1);
        Ok(())
    }

    fn compile_div(&mut self) -> Result<(), String> {
        // DIV a0, a0, a1
        self.emit_div(RiscVRegister::A0, RiscVRegister::A0, RiscVRegister::A1);
        Ok(())
    }

    fn compile_eq(&mut self) -> Result<(), String> {
        // SUB a0, a0, a1
        self.emit_sub(RiscVRegister::A0, RiscVRegister::A0, RiscVRegister::A1);
        // SEQZ a0, a0 (set if equal to zero)
        self.emit_seqz(RiscVRegister::A0, RiscVRegister::A0);
        Ok(())
    }

    fn compile_not_eq(&mut self) -> Result<(), String> {
        // SUB a0, a0, a1
        self.emit_sub(RiscVRegister::A0, RiscVRegister::A0, RiscVRegister::A1);
        // SNEZ a0, a0 (set if not equal to zero)
        self.emit_snez(RiscVRegister::A0, RiscVRegister::A0);
        Ok(())
    }

    fn compile_lt(&mut self) -> Result<(), String> {
        // SLT a0, a0, a1 (set if less than)
        self.emit_slt(RiscVRegister::A0, RiscVRegister::A0, RiscVRegister::A1);
        Ok(())
    }

    fn compile_gt(&mut self) -> Result<(), String> {
        // SLT a0, a1, a0 (set if a1 < a0, i.e., a0 > a1)
        self.emit_slt(RiscVRegister::A0, RiscVRegister::A1, RiscVRegister::A0);
        Ok(())
    }

    fn compile_lt_eq(&mut self) -> Result<(), String> {
        // SLT a0, a1, a0
        self.emit_slt(RiscVRegister::A0, RiscVRegister::A1, RiscVRegister::A0);
        // XORI a0, a0, 1 (invert)
        self.emit_xori(RiscVRegister::A0, RiscVRegister::A0, 1);
        Ok(())
    }

    fn compile_gt_eq(&mut self) -> Result<(), String> {
        // SLT a0, a0, a1
        self.emit_slt(RiscVRegister::A0, RiscVRegister::A0, RiscVRegister::A1);
        // XORI a0, a0, 1 (invert)
        self.emit_xori(RiscVRegister::A0, RiscVRegister::A0, 1);
        Ok(())
    }

    fn compile_jump(&mut self) -> Result<(), String> {
        // JAL zero, offset (unconditional jump)
        self.emit_jal(RiscVRegister::Zero, 0);
        Ok(())
    }

    fn compile_jump_if_false(&mut self) -> Result<(), String> {
        // BEQZ a0, offset (branch if equal to zero)
        self.emit_beqz(RiscVRegister::A0, 0);
        Ok(())
    }

    fn compile_load_local(&mut self) -> Result<(), String> {
        // LD a0, -16(s0) - stub
        self.emit_ld(RiscVRegister::A0, RiscVRegister::S0, -16);
        Ok(())
    }

    fn compile_store_local(&mut self) -> Result<(), String> {
        // SD a0, -16(s0) - stub
        self.emit_sd(RiscVRegister::A0, RiscVRegister::S0, -16);
        Ok(())
    }

    fn compile_call(&mut self) -> Result<(), String> {
        // JALR ra, 0(a0) - stub
        self.emit_jalr(RiscVRegister::Ra, RiscVRegister::A0, 0);
        Ok(())
    }

    fn compile_print(&mut self) -> Result<(), String> {
        // Stub - would call runtime function
        self.emit_nop();
        Ok(())
    }

    fn compile_return(&mut self) -> Result<(), String> {
        // RET (pseudo-instruction for JALR zero, 0(ra))
        self.emit_ret();
        Ok(())
    }

    // ============================================================================
    // INSTRUCTION EMISSION
    // ============================================================================

    fn emit_prologue(&mut self) {
        // ADDI sp, sp, -16
        self.emit_addi(RiscVRegister::Sp, RiscVRegister::Sp, -16);
        // SD ra, 8(sp)
        self.emit_sd(RiscVRegister::Ra, RiscVRegister::Sp, 8);
        // SD s0, 0(sp)
        self.emit_sd(RiscVRegister::S0, RiscVRegister::Sp, 0);
        // ADDI s0, sp, 16
        self.emit_addi(RiscVRegister::S0, RiscVRegister::Sp, 16);
    }

    fn emit_epilogue(&mut self) {
        // LD ra, 8(sp)
        self.emit_ld(RiscVRegister::Ra, RiscVRegister::Sp, 8);
        // LD s0, 0(sp)
        self.emit_ld(RiscVRegister::S0, RiscVRegister::Sp, 0);
        // ADDI sp, sp, 16
        self.emit_addi(RiscVRegister::Sp, RiscVRegister::Sp, 16);
        // RET
        self.emit_ret();
    }

    /// Emit: LI rd, imm (load immediate - pseudo-instruction)
    fn emit_li(&mut self, dest: RiscVRegister, imm: i32) {
        // LUI rd, imm[31:12]
        let upper = (imm >> 12) & 0xFFFFF;
        self.emit_lui(dest, upper as u32);
        // ADDI rd, rd, imm[11:0]
        let lower = imm & 0xFFF;
        self.emit_addi(dest, dest, lower);
    }

    /// Emit: LUI rd, imm
    fn emit_lui(&mut self, dest: RiscVRegister, imm: u32) {
        // LUI: opcode=0110111, rd, imm[31:12]
        let instr = 0x37 | (dest.encoding() << 7) | ((imm & 0xFFFFF) << 12);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: ADDI rd, rs1, imm
    fn emit_addi(&mut self, dest: RiscVRegister, src: RiscVRegister, imm: i32) {
        // ADDI: opcode=0010011, funct3=000
        let instr =
            0x13 | (dest.encoding() << 7) | (src.encoding() << 15) | (((imm as u32) & 0xFFF) << 20);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: ADD rd, rs1, rs2
    fn emit_add(&mut self, dest: RiscVRegister, src1: RiscVRegister, src2: RiscVRegister) {
        // ADD: opcode=0110011, funct3=000, funct7=0000000
        let instr =
            0x33 | (dest.encoding() << 7) | (src1.encoding() << 15) | (src2.encoding() << 20);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: SUB rd, rs1, rs2
    fn emit_sub(&mut self, dest: RiscVRegister, src1: RiscVRegister, src2: RiscVRegister) {
        // SUB: opcode=0110011, funct3=000, funct7=0100000
        let instr = 0x33
            | (dest.encoding() << 7)
            | (src1.encoding() << 15)
            | (src2.encoding() << 20)
            | (0x20 << 25);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: MUL rd, rs1, rs2
    fn emit_mul(&mut self, dest: RiscVRegister, src1: RiscVRegister, src2: RiscVRegister) {
        // MUL: opcode=0110011, funct3=000, funct7=0000001
        let instr = 0x33
            | (dest.encoding() << 7)
            | (src1.encoding() << 15)
            | (src2.encoding() << 20)
            | (0x1 << 25);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: DIV rd, rs1, rs2
    fn emit_div(&mut self, dest: RiscVRegister, src1: RiscVRegister, src2: RiscVRegister) {
        // DIV: opcode=0110011, funct3=100, funct7=0000001
        let instr = 0x33
            | (dest.encoding() << 7)
            | (0x4 << 12)
            | (src1.encoding() << 15)
            | (src2.encoding() << 20)
            | (0x1 << 25);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: SLT rd, rs1, rs2 (set if less than)
    fn emit_slt(&mut self, dest: RiscVRegister, src1: RiscVRegister, src2: RiscVRegister) {
        // SLT: opcode=0110011, funct3=010, funct7=0000000
        let instr = 0x33
            | (dest.encoding() << 7)
            | (0x2 << 12)
            | (src1.encoding() << 15)
            | (src2.encoding() << 20);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: SEQZ rd, rs (set if equal to zero - pseudo)
    fn emit_seqz(&mut self, dest: RiscVRegister, src: RiscVRegister) {
        // SLTIU rd, rs, 1
        let instr =
            0x13 | (dest.encoding() << 7) | (0x3 << 12) | (src.encoding() << 15) | (1 << 20);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: SNEZ rd, rs (set if not equal to zero - pseudo)
    fn emit_snez(&mut self, dest: RiscVRegister, src: RiscVRegister) {
        // SLTU rd, zero, rs
        let instr = 0x33
            | (dest.encoding() << 7)
            | (0x3 << 12)
            | (RiscVRegister::Zero.encoding() << 15)
            | (src.encoding() << 20);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: XORI rd, rs, imm
    fn emit_xori(&mut self, dest: RiscVRegister, src: RiscVRegister, imm: i32) {
        // XORI: opcode=0010011, funct3=100
        let instr = 0x13
            | (dest.encoding() << 7)
            | (0x4 << 12)
            | (src.encoding() << 15)
            | (((imm as u32) & 0xFFF) << 20);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: LD rd, offset(rs)
    fn emit_ld(&mut self, dest: RiscVRegister, base: RiscVRegister, offset: i32) {
        // LD: opcode=0000011, funct3=011
        let instr = 0x3
            | (dest.encoding() << 7)
            | (0x3 << 12)
            | (base.encoding() << 15)
            | (((offset as u32) & 0xFFF) << 20);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: SD rs, offset(base)
    fn emit_sd(&mut self, src: RiscVRegister, base: RiscVRegister, offset: i32) {
        // SD: opcode=0100011, funct3=011
        let imm_low = (offset as u32) & 0x1F;
        let imm_high = ((offset as u32) >> 5) & 0x7F;
        let instr = 0x23
            | (imm_low << 7)
            | (0x3 << 12)
            | (base.encoding() << 15)
            | (src.encoding() << 20)
            | (imm_high << 25);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: JAL rd, offset
    fn emit_jal(&mut self, dest: RiscVRegister, offset: i32) {
        // JAL: opcode=1101111
        let imm = offset as u32;
        let imm_20 = (imm >> 20) & 0x1;
        let imm_10_1 = (imm >> 1) & 0x3FF;
        let imm_11 = (imm >> 11) & 0x1;
        let imm_19_12 = (imm >> 12) & 0xFF;
        let instr = 0x6F
            | (dest.encoding() << 7)
            | (imm_19_12 << 12)
            | (imm_11 << 20)
            | (imm_10_1 << 21)
            | (imm_20 << 31);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: JALR rd, offset(rs)
    fn emit_jalr(&mut self, dest: RiscVRegister, base: RiscVRegister, offset: i32) {
        // JALR: opcode=1100111, funct3=000
        let instr = 0x67
            | (dest.encoding() << 7)
            | (base.encoding() << 15)
            | (((offset as u32) & 0xFFF) << 20);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: BEQZ rs, offset (branch if equal to zero - pseudo)
    fn emit_beqz(&mut self, src: RiscVRegister, offset: i32) {
        // BEQ rs, zero, offset
        let imm = offset as u32;
        let imm_12 = (imm >> 12) & 0x1;
        let imm_10_5 = (imm >> 5) & 0x3F;
        let imm_4_1 = (imm >> 1) & 0xF;
        let imm_11 = (imm >> 11) & 0x1;
        let instr = 0x63
            | (imm_11 << 7)
            | (imm_4_1 << 8)
            | (src.encoding() << 15)
            | (RiscVRegister::Zero.encoding() << 20)
            | (imm_10_5 << 25)
            | (imm_12 << 31);
        self.code.extend_from_slice(&instr.to_le_bytes());
    }

    /// Emit: RET (pseudo-instruction for JALR zero, 0(ra))
    fn emit_ret(&mut self) {
        self.emit_jalr(RiscVRegister::Zero, RiscVRegister::Ra, 0);
    }

    /// Emit: NOP (pseudo-instruction for ADDI zero, zero, 0)
    fn emit_nop(&mut self) {
        self.emit_addi(RiscVRegister::Zero, RiscVRegister::Zero, 0);
    }
}

impl Default for RiscVCodeGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_riscv_codegen_creation() {
        let codegen = RiscVCodeGen::new();
        assert_eq!(codegen.code.len(), 0);
    }

    #[test]
    fn test_riscv_simple_program() {
        let mut bytecode = Bytecode::new();
        let c42 = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![Instruction::LoadConst(c42), Instruction::Halt];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_riscv_arithmetic() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::Add,
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_riscv_subtraction() {
        let mut bytecode = Bytecode::new();
        let c30 = bytecode.add_constant(Constant::Int(30));
        let c10 = bytecode.add_constant(Constant::Int(10));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c30),
            Instruction::LoadConst(c10),
            Instruction::Sub,
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_riscv_multiplication() {
        let mut bytecode = Bytecode::new();
        let c5 = bytecode.add_constant(Constant::Int(5));
        let c6 = bytecode.add_constant(Constant::Int(6));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c5),
            Instruction::LoadConst(c6),
            Instruction::Mul,
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_riscv_division() {
        let mut bytecode = Bytecode::new();
        let c20 = bytecode.add_constant(Constant::Int(20));
        let c4 = bytecode.add_constant(Constant::Int(4));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c20),
            Instruction::LoadConst(c4),
            Instruction::Div,
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_riscv_comparisons() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::Lt,
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_riscv_control_flow() {
        let mut bytecode = Bytecode::new();
        let c0 = bytecode.add_constant(Constant::Int(0));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c0),
            Instruction::JumpIfFalse(3),
            Instruction::LoadConst(c0),
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_riscv_function_call() {
        let mut bytecode = Bytecode::new();
        let c42 = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c42),
            Instruction::Call(1),
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_riscv_locals() {
        let mut bytecode = Bytecode::new();
        let c100 = bytecode.add_constant(Constant::Int(100));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c100),
            Instruction::StoreLocal("x".to_string()),
            Instruction::LoadLocal("x".to_string()),
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_riscv_unimplemented_instruction_returns_error() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::NewList(0), Instruction::Halt];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("RISC-V backend: unimplemented instruction"));
        assert!(err.contains("[context:instr=NewList(0)]"));
    }

    #[test]
    fn test_riscv_unimplemented_instruction_preserves_payload_context() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![
            Instruction::BackendCall {
                backend: "http".to_string(),
                method: "get".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("RISC-V backend: unimplemented instruction"));
        assert!(err.contains("backend: \"http\""));
        assert!(err.contains("method: \"get\""));
        assert!(err.contains("arg_count: 2"));
    }

    #[test]
    fn test_riscv_unsupported_constant_returns_error() {
        let mut bytecode = Bytecode::new();
        let cbool = bytecode.add_constant(Constant::Bool(true));
        bytecode.main_instructions = vec![Instruction::LoadConst(cbool), Instruction::Halt];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("RISC-V backend: unsupported constant in LoadConst"));
        assert!(err.contains("[context:id=0,constant=Bool(true)]"));
    }

    #[test]
    fn test_riscv_unsupported_string_constant_preserves_context_payload() {
        let mut bytecode = Bytecode::new();
        let cstr = bytecode.add_constant(Constant::String("alpha".to_string()));
        bytecode.main_instructions = vec![Instruction::LoadConst(cstr), Instruction::Halt];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("RISC-V backend: unsupported constant in LoadConst"));
        assert!(err.contains("constant=String(\"alpha\")"));
    }

    #[test]
    fn test_riscv_int_out_of_immediate_range_returns_error() {
        let mut bytecode = Bytecode::new();
        let c = bytecode.add_constant(Constant::Int(i64::MAX));
        bytecode.main_instructions = vec![Instruction::LoadConst(c), Instruction::Halt];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("RISC-V backend: integer constant out of immediate range"));
    }

    #[test]
    fn test_riscv_int_i32_bounds_are_accepted() {
        let mut bytecode = Bytecode::new();
        let c_min = bytecode.add_constant(Constant::Int(i32::MIN as i64));
        let c_max = bytecode.add_constant(Constant::Int(i32::MAX as i64));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c_min),
            Instruction::LoadConst(c_max),
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
    }

    #[test]
    fn test_riscv_int_i32_max_plus_one_is_rejected() {
        let mut bytecode = Bytecode::new();
        let c = bytecode.add_constant(Constant::Int(i32::MAX as i64 + 1));
        bytecode.main_instructions = vec![Instruction::LoadConst(c), Instruction::Halt];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("RISC-V backend: integer constant out of immediate range"));
    }

    #[test]
    fn test_riscv_constant_index_out_of_bounds_returns_error() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::LoadConst(999), Instruction::Halt];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("RISC-V backend: LoadConst index out of bounds"));
        assert!(err.contains("[context:id=999,constants=0]"));
    }

    #[test]
    fn test_riscv_constant_index_oob_context_reports_nonzero_constants_len() {
        let mut bytecode = Bytecode::new();
        let _c0 = bytecode.add_constant(Constant::Int(7));
        bytecode.main_instructions = vec![Instruction::LoadConst(2), Instruction::Halt];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("RISC-V backend: LoadConst index out of bounds"));
        assert!(err.contains("[context:id=2,constants=1]"));
    }

    #[test]
    fn test_riscv_unsupported_constant_error_includes_id() {
        let mut bytecode = Bytecode::new();
        let cint = bytecode.add_constant(Constant::Int(1));
        let cbool = bytecode.add_constant(Constant::Bool(true));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(cint),
            Instruction::LoadConst(cbool),
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("[context:id=1,constant=Bool(true)]"));
    }

    #[test]
    fn test_riscv_range_error_includes_id() {
        let mut bytecode = Bytecode::new();
        let cint = bytecode.add_constant(Constant::Int(1));
        let cout = bytecode.add_constant(Constant::Int(i32::MAX as i64 + 1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(cint),
            Instruction::LoadConst(cout),
            Instruction::Halt,
        ];

        let mut codegen = RiscVCodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("[context:id=1,constant=2147483648]"));
    }

    #[test]
    fn test_riscv_error_format_uses_context_block() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::LoadConst(999), Instruction::Halt];

        let mut codegen = RiscVCodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("expected LoadConst oob error");

        assert!(err.starts_with("RISC-V backend:"));
        assert!(err.contains("[context:"));
    }
}
