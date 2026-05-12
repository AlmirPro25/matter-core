//! Code generation modules for different architectures

pub mod arm64;
pub mod riscv64;
pub mod x86_64;

/// Register representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    RAX,
    RBX,
    RCX,
    RDX,
    RSI,
    RDI,
    RBP,
    RSP,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

impl Register {
    /// Get the register encoding for x86-64
    pub fn encoding(&self) -> u8 {
        match self {
            Register::RAX => 0,
            Register::RCX => 1,
            Register::RDX => 2,
            Register::RBX => 3,
            Register::RSP => 4,
            Register::RBP => 5,
            Register::RSI => 6,
            Register::RDI => 7,
            Register::R8 => 8,
            Register::R9 => 9,
            Register::R10 => 10,
            Register::R11 => 11,
            Register::R12 => 12,
            Register::R13 => 13,
            Register::R14 => 14,
            Register::R15 => 15,
        }
    }
}

/// Machine code instruction
#[derive(Debug, Clone)]
pub enum MachineInstr {
    /// Move immediate value to register
    MovImm { dest: Register, value: i64 },

    /// Move register to register
    MovReg { dest: Register, src: Register },

    /// Add two registers
    AddReg { dest: Register, src: Register },

    /// Subtract two registers
    SubReg { dest: Register, src: Register },

    /// Multiply two registers
    MulReg { dest: Register, src: Register },

    /// Divide (RDX:RAX by src, quotient in RAX, remainder in RDX)
    DivReg { src: Register },

    /// Compare two registers
    CmpReg { left: Register, right: Register },

    /// Push register to stack
    Push { reg: Register },

    /// Pop from stack to register
    Pop { reg: Register },

    /// Call function at address
    Call { offset: i32 },

    /// Return from function
    Ret,

    /// Unconditional jump
    Jmp { offset: i32 },

    /// Jump if equal (ZF=1)
    Je { offset: i32 },

    /// Jump if not equal (ZF=0)
    Jne { offset: i32 },

    /// Jump if less (SF≠OF)
    Jl { offset: i32 },

    /// Jump if greater (ZF=0 and SF=OF)
    Jg { offset: i32 },

    /// Jump if less or equal (ZF=1 or SF≠OF)
    Jle { offset: i32 },

    /// Jump if greater or equal (SF=OF)
    Jge { offset: i32 },

    /// System call
    Syscall,

    /// No operation
    Nop,
}
