use std::collections::HashMap;
use std::fmt;

const REGISTER_COUNT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Nop,
    LoadConst { reg: usize, value: i64 },
    Add { dst: usize, a: usize, b: usize },
    Sub { dst: usize, a: usize, b: usize },
    Mul { dst: usize, a: usize, b: usize },
    Div { dst: usize, a: usize, b: usize },
    Store { addr: usize, reg: usize },
    LoadMem { reg: usize, addr: usize },
    Jump { target: usize },
    JumpIfZero { reg: usize, target: usize },
    Print { reg: usize },
    Halt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtualCpuError {
    InvalidRegisterIndex { reg: usize },
    InvalidMemoryAddress { addr: usize, len: usize },
    InvalidJumpTarget { target: usize, len: usize },
    ProgramCounterOutOfBounds { pc: usize, len: usize },
    DivisionByZero,
    ArithmeticOverflow,
    CycleBudgetExceeded { limit: u64, attempted: u64 },
    EnergyBudgetExceeded { limit: u64, attempted: u64 },
    ParseError { line: usize, message: String },
    InvalidRegister { line: usize, register: String },
    InvalidNumber { line: usize, value: String },
    UnknownInstruction { line: usize, instruction: String },
}

impl fmt::Display for VirtualCpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegisterIndex { reg } => write!(f, "invalid register index: {reg}"),
            Self::InvalidMemoryAddress { addr, len } => {
                write!(f, "invalid memory address {addr} (memory len: {len})")
            }
            Self::InvalidJumpTarget { target, len } => {
                write!(f, "invalid jump target {target} (program len: {len})")
            }
            Self::ProgramCounterOutOfBounds { pc, len } => {
                write!(f, "program counter {pc} out of bounds (program len: {len})")
            }
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::ArithmeticOverflow => write!(f, "arithmetic overflow"),
            Self::CycleBudgetExceeded { limit, attempted } => {
                write!(
                    f,
                    "cycle budget exceeded (limit: {limit}, attempted: {attempted})"
                )
            }
            Self::EnergyBudgetExceeded { limit, attempted } => {
                write!(
                    f,
                    "energy budget exceeded (limit: {limit}, attempted: {attempted})"
                )
            }
            Self::ParseError { line, message } => {
                write!(f, "parse error at line {line}: {message}")
            }
            Self::InvalidRegister { line, register } => {
                write!(f, "invalid register at line {line}: {register}")
            }
            Self::InvalidNumber { line, value } => {
                write!(f, "invalid number at line {line}: {value}")
            }
            Self::UnknownInstruction { line, instruction } => {
                write!(f, "unknown instruction at line {line}: {instruction}")
            }
        }
    }
}

impl std::error::Error for VirtualCpuError {}

pub fn parse_program(source: &str) -> Result<Vec<Instruction>, VirtualCpuError> {
    #[derive(Clone)]
    enum ParsedInstruction {
        Nop,
        LoadConst {
            reg: usize,
            value: i64,
        },
        Add {
            dst: usize,
            a: usize,
            b: usize,
        },
        Sub {
            dst: usize,
            a: usize,
            b: usize,
        },
        Mul {
            dst: usize,
            a: usize,
            b: usize,
        },
        Div {
            dst: usize,
            a: usize,
            b: usize,
        },
        Store {
            addr: usize,
            reg: usize,
        },
        LoadMem {
            reg: usize,
            addr: usize,
        },
        Jump {
            target: JumpTargetRefGlobal,
        },
        JumpIfZero {
            reg: usize,
            target: JumpTargetRefGlobal,
        },
        Print {
            reg: usize,
        },
        Halt,
    }

    let mut labels: HashMap<String, usize> = HashMap::new();
    let mut parsed: Vec<(usize, ParsedInstruction)> = Vec::new();
    let mut instruction_index: usize = 0;

    for (idx, raw_line) in source.lines().enumerate() {
        let line_number = idx + 1;
        let line = strip_comments(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        let mut rest = line;
        if let Some((first, remaining)) = line.split_once(':') {
            let label = first.trim();
            if !label.is_empty() && !label.contains(char::is_whitespace) {
                if !is_valid_label(label) {
                    return Err(VirtualCpuError::ParseError {
                        line: line_number,
                        message: format!("invalid label: {label}"),
                    });
                }
                if labels
                    .insert(label.to_string(), instruction_index)
                    .is_some()
                {
                    return Err(VirtualCpuError::ParseError {
                        line: line_number,
                        message: format!("duplicate label: {label}"),
                    });
                }
                rest = remaining.trim();
                if rest.is_empty() {
                    continue;
                }
            }
        }

        let parts: Vec<&str> = rest.split_whitespace().collect();
        let op = parts[0];
        let parsed_instruction = match op {
            "NOP" => {
                expect_arity(&parts, 1, line_number)?;
                ParsedInstruction::Nop
            }
            "LOAD_CONST" => {
                expect_arity(&parts, 3, line_number)?;
                let reg = parse_register(parts[1], line_number)?;
                let value = parse_i64(parts[2], line_number)?;
                ParsedInstruction::LoadConst { reg, value }
            }
            "ADD" => {
                expect_arity(&parts, 4, line_number)?;
                ParsedInstruction::Add {
                    dst: parse_register(parts[1], line_number)?,
                    a: parse_register(parts[2], line_number)?,
                    b: parse_register(parts[3], line_number)?,
                }
            }
            "SUB" => {
                expect_arity(&parts, 4, line_number)?;
                ParsedInstruction::Sub {
                    dst: parse_register(parts[1], line_number)?,
                    a: parse_register(parts[2], line_number)?,
                    b: parse_register(parts[3], line_number)?,
                }
            }
            "MUL" => {
                expect_arity(&parts, 4, line_number)?;
                ParsedInstruction::Mul {
                    dst: parse_register(parts[1], line_number)?,
                    a: parse_register(parts[2], line_number)?,
                    b: parse_register(parts[3], line_number)?,
                }
            }
            "DIV" => {
                expect_arity(&parts, 4, line_number)?;
                ParsedInstruction::Div {
                    dst: parse_register(parts[1], line_number)?,
                    a: parse_register(parts[2], line_number)?,
                    b: parse_register(parts[3], line_number)?,
                }
            }
            "STORE" => {
                expect_arity(&parts, 3, line_number)?;
                ParsedInstruction::Store {
                    addr: parse_usize(parts[1], line_number)?,
                    reg: parse_register(parts[2], line_number)?,
                }
            }
            "LOAD_MEM" => {
                expect_arity(&parts, 3, line_number)?;
                ParsedInstruction::LoadMem {
                    reg: parse_register(parts[1], line_number)?,
                    addr: parse_usize(parts[2], line_number)?,
                }
            }
            "JUMP" => {
                expect_arity(&parts, 2, line_number)?;
                ParsedInstruction::Jump {
                    target: parse_jump_target(parts[1], line_number)?,
                }
            }
            "JUMP_IF_ZERO" => {
                expect_arity(&parts, 3, line_number)?;
                ParsedInstruction::JumpIfZero {
                    reg: parse_register(parts[1], line_number)?,
                    target: parse_jump_target(parts[2], line_number)?,
                }
            }
            "PRINT" => {
                expect_arity(&parts, 2, line_number)?;
                ParsedInstruction::Print {
                    reg: parse_register(parts[1], line_number)?,
                }
            }
            "HALT" => {
                expect_arity(&parts, 1, line_number)?;
                ParsedInstruction::Halt
            }
            _ => {
                return Err(VirtualCpuError::UnknownInstruction {
                    line: line_number,
                    instruction: op.to_string(),
                });
            }
        };

        parsed.push((line_number, parsed_instruction));
        instruction_index += 1;
    }

    let mut program = Vec::with_capacity(parsed.len());
    for (line, instruction) in parsed {
        let resolved = match instruction {
            ParsedInstruction::Nop => Instruction::Nop,
            ParsedInstruction::LoadConst { reg, value } => Instruction::LoadConst { reg, value },
            ParsedInstruction::Add { dst, a, b } => Instruction::Add { dst, a, b },
            ParsedInstruction::Sub { dst, a, b } => Instruction::Sub { dst, a, b },
            ParsedInstruction::Mul { dst, a, b } => Instruction::Mul { dst, a, b },
            ParsedInstruction::Div { dst, a, b } => Instruction::Div { dst, a, b },
            ParsedInstruction::Store { addr, reg } => Instruction::Store { addr, reg },
            ParsedInstruction::LoadMem { reg, addr } => Instruction::LoadMem { reg, addr },
            ParsedInstruction::Jump { target } => Instruction::Jump {
                target: resolve_jump_target(target, &labels, line)?,
            },
            ParsedInstruction::JumpIfZero { reg, target } => Instruction::JumpIfZero {
                reg,
                target: resolve_jump_target(target, &labels, line)?,
            },
            ParsedInstruction::Print { reg } => Instruction::Print { reg },
            ParsedInstruction::Halt => Instruction::Halt,
        };
        program.push(resolved);
    }

    Ok(program)
}

pub fn run_program_text(source: &str, memory_size: usize) -> Result<VirtualCpu, VirtualCpuError> {
    let program = parse_program(source)?;
    let mut cpu = VirtualCpu::new(memory_size);
    cpu.load_program(program)?;
    cpu.run()?;
    Ok(cpu)
}

fn expect_arity(parts: &[&str], expected: usize, line: usize) -> Result<(), VirtualCpuError> {
    if parts.len() != expected {
        return Err(VirtualCpuError::ParseError {
            line,
            message: format!("expected {expected} tokens, got {}", parts.len()),
        });
    }
    Ok(())
}

fn parse_register(token: &str, line: usize) -> Result<usize, VirtualCpuError> {
    if token.len() != 2 || !token.starts_with('r') {
        return Err(VirtualCpuError::InvalidRegister {
            line,
            register: token.to_string(),
        });
    }

    let idx = token[1..]
        .parse::<usize>()
        .map_err(|_| VirtualCpuError::InvalidRegister {
            line,
            register: token.to_string(),
        })?;

    if idx >= REGISTER_COUNT {
        return Err(VirtualCpuError::InvalidRegister {
            line,
            register: token.to_string(),
        });
    }

    Ok(idx)
}

fn parse_i64(token: &str, line: usize) -> Result<i64, VirtualCpuError> {
    if let Some(hex) = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        return i64::from_str_radix(hex, 16).map_err(|_| VirtualCpuError::InvalidNumber {
            line,
            value: token.to_string(),
        });
    }
    token
        .parse::<i64>()
        .map_err(|_| VirtualCpuError::InvalidNumber {
            line,
            value: token.to_string(),
        })
}

fn parse_usize(token: &str, line: usize) -> Result<usize, VirtualCpuError> {
    if let Some(hex) = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        return usize::from_str_radix(hex, 16).map_err(|_| VirtualCpuError::InvalidNumber {
            line,
            value: token.to_string(),
        });
    }
    token
        .parse::<usize>()
        .map_err(|_| VirtualCpuError::InvalidNumber {
            line,
            value: token.to_string(),
        })
}

fn parse_jump_target(token: &str, line: usize) -> Result<JumpTargetRefGlobal, VirtualCpuError> {
    if let Ok(index) = token.parse::<usize>() {
        return Ok(JumpTargetRefGlobal::Index(index));
    }

    if token.is_empty() || token.contains(char::is_whitespace) {
        return Err(VirtualCpuError::ParseError {
            line,
            message: format!("invalid jump target: {token}"),
        });
    }

    Ok(JumpTargetRefGlobal::Label(token.to_string()))
}

#[derive(Clone)]
enum JumpTargetRefGlobal {
    Index(usize),
    Label(String),
}

fn resolve_jump_target(
    target: JumpTargetRefGlobal,
    labels: &HashMap<String, usize>,
    line: usize,
) -> Result<usize, VirtualCpuError> {
    match target {
        JumpTargetRefGlobal::Index(i) => Ok(i),
        JumpTargetRefGlobal::Label(name) => {
            labels
                .get(&name)
                .copied()
                .ok_or(VirtualCpuError::ParseError {
                    line,
                    message: format!("unknown label: {name}"),
                })
        }
    }
}

fn is_valid_label(label: &str) -> bool {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn strip_comments(line: &str) -> &str {
    let hash = line.find('#');
    let semicolon = line.find(';');
    let end = match (hash, semicolon) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => line.len(),
    };
    &line[..end]
}

#[derive(Debug, Clone, PartialEq)]
pub struct CpuStats {
    pub cycles: u64,
    pub energy_consumed: u64,
    pub program_counter: usize,
    pub running: bool,
}

#[derive(Debug, Clone)]
pub struct VirtualCpu {
    pub registers: [i64; REGISTER_COUNT],
    pub program_counter: usize,
    pub running: bool,
    pub memory: Vec<i64>,
    pub cycles: u64,
    pub energy_consumed: u64,
    cycle_budget: Option<u64>,
    energy_budget: Option<u64>,
    program: Vec<Instruction>,
}

impl VirtualCpu {
    pub fn new(memory_size: usize) -> Self {
        Self {
            registers: [0; REGISTER_COUNT],
            program_counter: 0,
            running: false,
            memory: vec![0; memory_size],
            cycles: 0,
            energy_consumed: 0,
            cycle_budget: None,
            energy_budget: None,
            program: Vec::new(),
        }
    }

    pub fn set_cycle_budget(&mut self, budget: Option<u64>) {
        self.cycle_budget = budget;
    }

    pub fn set_energy_budget(&mut self, budget: Option<u64>) {
        self.energy_budget = budget;
    }

    pub fn load_program(&mut self, program: Vec<Instruction>) -> Result<(), VirtualCpuError> {
        self.program = program;
        self.program_counter = 0;
        self.running = !self.program.is_empty();
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), VirtualCpuError> {
        if !self.running {
            return Ok(());
        }

        let instr = self.program.get(self.program_counter).cloned().ok_or(
            VirtualCpuError::ProgramCounterOutOfBounds {
                pc: self.program_counter,
                len: self.program.len(),
            },
        )?;

        let mut next_pc = self.program_counter + 1;
        let cycles = match instr {
            Instruction::Nop => 1,
            Instruction::LoadConst { reg, value } => {
                self.set_reg(reg, value)?;
                1
            }
            Instruction::Add { dst, a, b } => {
                let v = self
                    .reg(a)?
                    .checked_add(self.reg(b)?)
                    .ok_or(VirtualCpuError::ArithmeticOverflow)?;
                self.set_reg(dst, v)?;
                1
            }
            Instruction::Sub { dst, a, b } => {
                let v = self
                    .reg(a)?
                    .checked_sub(self.reg(b)?)
                    .ok_or(VirtualCpuError::ArithmeticOverflow)?;
                self.set_reg(dst, v)?;
                1
            }
            Instruction::Mul { dst, a, b } => {
                let v = self
                    .reg(a)?
                    .checked_mul(self.reg(b)?)
                    .ok_or(VirtualCpuError::ArithmeticOverflow)?;
                self.set_reg(dst, v)?;
                3
            }
            Instruction::Div { dst, a, b } => {
                let denom = self.reg(b)?;
                if denom == 0 {
                    return Err(VirtualCpuError::DivisionByZero);
                }
                let v = self
                    .reg(a)?
                    .checked_div(denom)
                    .ok_or(VirtualCpuError::ArithmeticOverflow)?;
                self.set_reg(dst, v)?;
                8
            }
            Instruction::Store { addr, reg } => {
                let value = self.reg(reg)?;
                self.set_mem(addr, value)?;
                2
            }
            Instruction::LoadMem { reg, addr } => {
                let value = self.mem(addr)?;
                self.set_reg(reg, value)?;
                2
            }
            Instruction::Jump { target } => {
                self.validate_jump_target(target)?;
                next_pc = target;
                1
            }
            Instruction::JumpIfZero { reg, target } => {
                if self.reg(reg)? == 0 {
                    self.validate_jump_target(target)?;
                    next_pc = target;
                }
                1
            }
            Instruction::Print { reg } => {
                println!("{}", self.reg(reg)?);
                1
            }
            Instruction::Halt => {
                self.running = false;
                1
            }
        };

        let next_cycles = self
            .cycles
            .checked_add(cycles)
            .ok_or(VirtualCpuError::ArithmeticOverflow)?;
        let next_energy = self
            .energy_consumed
            .checked_add(cycles)
            .ok_or(VirtualCpuError::ArithmeticOverflow)?;

        if let Some(limit) = self.cycle_budget {
            if next_cycles > limit {
                return Err(VirtualCpuError::CycleBudgetExceeded {
                    limit,
                    attempted: next_cycles,
                });
            }
        }

        if let Some(limit) = self.energy_budget {
            if next_energy > limit {
                return Err(VirtualCpuError::EnergyBudgetExceeded {
                    limit,
                    attempted: next_energy,
                });
            }
        }

        self.program_counter = next_pc;
        self.cycles = next_cycles;
        self.energy_consumed = next_energy;
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), VirtualCpuError> {
        while self.running {
            self.step()?;
        }
        Ok(())
    }

    pub fn stats(&self) -> CpuStats {
        CpuStats {
            cycles: self.cycles,
            energy_consumed: self.energy_consumed,
            program_counter: self.program_counter,
            running: self.running,
        }
    }

    fn reg(&self, reg: usize) -> Result<i64, VirtualCpuError> {
        self.registers
            .get(reg)
            .copied()
            .ok_or(VirtualCpuError::InvalidRegisterIndex { reg })
    }

    fn set_reg(&mut self, reg: usize, value: i64) -> Result<(), VirtualCpuError> {
        let slot = self
            .registers
            .get_mut(reg)
            .ok_or(VirtualCpuError::InvalidRegisterIndex { reg })?;
        *slot = value;
        Ok(())
    }

    fn mem(&self, addr: usize) -> Result<i64, VirtualCpuError> {
        self.memory
            .get(addr)
            .copied()
            .ok_or(VirtualCpuError::InvalidMemoryAddress {
                addr,
                len: self.memory.len(),
            })
    }

    fn set_mem(&mut self, addr: usize, value: i64) -> Result<(), VirtualCpuError> {
        let len = self.memory.len();
        let slot = self
            .memory
            .get_mut(addr)
            .ok_or(VirtualCpuError::InvalidMemoryAddress { addr, len })?;
        *slot = value;
        Ok(())
    }

    fn validate_jump_target(&self, target: usize) -> Result<(), VirtualCpuError> {
        if target >= self.program.len() {
            return Err(VirtualCpuError::InvalidJumpTarget {
                target,
                len: self.program.len(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soma_10_mais_20_igual_30() {
        let mut cpu = VirtualCpu::new(16);
        cpu.load_program(vec![
            Instruction::LoadConst { reg: 0, value: 10 },
            Instruction::LoadConst { reg: 1, value: 20 },
            Instruction::Add { dst: 2, a: 0, b: 1 },
            Instruction::Halt,
        ])
        .unwrap();
        cpu.run().unwrap();
        assert_eq!(cpu.registers[2], 30);
    }

    #[test]
    fn store_load_memoria() {
        let mut cpu = VirtualCpu::new(8);
        cpu.load_program(vec![
            Instruction::LoadConst { reg: 0, value: 42 },
            Instruction::Store { addr: 3, reg: 0 },
            Instruction::LoadMem { reg: 1, addr: 3 },
            Instruction::Halt,
        ])
        .unwrap();
        cpu.run().unwrap();
        assert_eq!(cpu.registers[1], 42);
    }

    #[test]
    fn jump_funciona() {
        let mut cpu = VirtualCpu::new(8);
        cpu.load_program(vec![
            Instruction::Jump { target: 2 },
            Instruction::LoadConst { reg: 0, value: 999 },
            Instruction::LoadConst { reg: 0, value: 7 },
            Instruction::Halt,
        ])
        .unwrap();
        cpu.run().unwrap();
        assert_eq!(cpu.registers[0], 7);
    }

    #[test]
    fn divisao_por_zero_retorna_erro() {
        let mut cpu = VirtualCpu::new(8);
        cpu.load_program(vec![
            Instruction::LoadConst { reg: 0, value: 10 },
            Instruction::LoadConst { reg: 1, value: 0 },
            Instruction::Div { dst: 2, a: 0, b: 1 },
            Instruction::Halt,
        ])
        .unwrap();

        let err = cpu.run().unwrap_err();
        assert_eq!(err, VirtualCpuError::DivisionByZero);
    }

    #[test]
    fn endereco_invalido_retorna_erro() {
        let mut cpu = VirtualCpu::new(2);
        cpu.load_program(vec![
            Instruction::LoadConst { reg: 0, value: 1 },
            Instruction::Store { addr: 9, reg: 0 },
            Instruction::Halt,
        ])
        .unwrap();

        let err = cpu.run().unwrap_err();
        assert!(matches!(err, VirtualCpuError::InvalidMemoryAddress { .. }));
    }

    #[test]
    fn programa_termina_com_halt() {
        let mut cpu = VirtualCpu::new(4);
        cpu.load_program(vec![Instruction::Nop, Instruction::Halt])
            .unwrap();
        cpu.run().unwrap();

        assert!(!cpu.running);
        assert_eq!(cpu.program_counter, 2);
    }

    #[test]
    fn energia_e_ciclos_aumentam_corretamente() {
        let mut cpu = VirtualCpu::new(8);
        cpu.load_program(vec![
            Instruction::Nop,
            Instruction::LoadConst { reg: 0, value: 6 },
            Instruction::LoadConst { reg: 1, value: 3 },
            Instruction::Div { dst: 2, a: 0, b: 1 },
            Instruction::Store { addr: 0, reg: 2 },
            Instruction::Halt,
        ])
        .unwrap();
        cpu.run().unwrap();

        // 1 + 1 + 1 + 8 + 2 + 1 = 14
        assert_eq!(cpu.cycles, 14);
        assert_eq!(cpu.energy_consumed, 14);

        let stats = cpu.stats();
        assert_eq!(stats.cycles, 14);
        assert_eq!(stats.energy_consumed, 14);
    }

    #[test]
    fn overflow_retorna_erro_sem_panic() {
        let mut cpu = VirtualCpu::new(8);
        cpu.load_program(vec![
            Instruction::LoadConst {
                reg: 0,
                value: i64::MAX,
            },
            Instruction::LoadConst { reg: 1, value: 1 },
            Instruction::Add { dst: 2, a: 0, b: 1 },
            Instruction::Halt,
        ])
        .unwrap();

        let err = cpu.run().unwrap_err();
        assert_eq!(err, VirtualCpuError::ArithmeticOverflow);
    }

    #[test]
    fn limite_de_ciclos_retorna_erro() {
        let mut cpu = VirtualCpu::new(8);
        cpu.set_cycle_budget(Some(2));
        cpu.load_program(vec![
            Instruction::Nop,
            Instruction::Nop,
            Instruction::Nop,
            Instruction::Halt,
        ])
        .unwrap();

        let err = cpu.run().unwrap_err();
        assert_eq!(
            err,
            VirtualCpuError::CycleBudgetExceeded {
                limit: 2,
                attempted: 3
            }
        );
    }

    #[test]
    fn limite_de_energia_retorna_erro() {
        let mut cpu = VirtualCpu::new(8);
        cpu.set_energy_budget(Some(1));
        cpu.load_program(vec![Instruction::Nop, Instruction::Nop, Instruction::Halt])
            .unwrap();

        let err = cpu.run().unwrap_err();
        assert_eq!(
            err,
            VirtualCpuError::EnergyBudgetExceeded {
                limit: 1,
                attempted: 2
            }
        );
    }

    #[test]
    fn parse_load_const() {
        let program = parse_program("LOAD_CONST r0 10").expect("parse should succeed");
        assert_eq!(program, vec![Instruction::LoadConst { reg: 0, value: 10 }]);
    }

    #[test]
    fn parse_add() {
        let program = parse_program("ADD r2 r0 r1").expect("parse should succeed");
        assert_eq!(program, vec![Instruction::Add { dst: 2, a: 0, b: 1 }]);
    }

    #[test]
    fn parse_ignora_comentarios_e_linhas_vazias() {
        let source = "\n# comentario\nLOAD_CONST r0 10\n\n; outro\nHALT\n";
        let program = parse_program(source).expect("parse should succeed");
        assert_eq!(
            program,
            vec![
                Instruction::LoadConst { reg: 0, value: 10 },
                Instruction::Halt
            ]
        );
    }

    #[test]
    fn parse_registrador_invalido_retorna_erro() {
        let err = parse_program("LOAD_CONST r8 10").expect_err("should fail");
        assert_eq!(
            err,
            VirtualCpuError::InvalidRegister {
                line: 1,
                register: "r8".to_string()
            }
        );
    }

    #[test]
    fn parse_instrucao_desconhecida_retorna_erro() {
        let err = parse_program("FOO r0").expect_err("should fail");
        assert_eq!(
            err,
            VirtualCpuError::UnknownInstruction {
                line: 1,
                instruction: "FOO".to_string()
            }
        );
    }

    #[test]
    fn parse_numero_invalido_retorna_erro() {
        let err = parse_program("LOAD_CONST r0 abc").expect_err("should fail");
        assert_eq!(
            err,
            VirtualCpuError::InvalidNumber {
                line: 1,
                value: "abc".to_string()
            }
        );
    }

    #[test]
    fn programa_textual_completo_executa_e_imprime_30() {
        let source = "\
LOAD_CONST r0 10
LOAD_CONST r1 20
ADD r2 r0 r1
PRINT r2
HALT
";
        let program = parse_program(source).expect("parse should succeed");
        let mut cpu = VirtualCpu::new(16);
        cpu.load_program(program).expect("load should succeed");
        cpu.run().expect("run should succeed");
        assert_eq!(cpu.registers[2], 30);
        assert_eq!(cpu.stats().cycles, 5);
        assert_eq!(cpu.stats().energy_consumed, 5);
        assert_eq!(cpu.stats().program_counter, 5);
    }

    #[test]
    fn run_program_text_executa_com_sucesso() {
        let source = "\
LOAD_CONST r0 10
LOAD_CONST r1 20
ADD r2 r0 r1
HALT
";
        let cpu = run_program_text(source, 8).expect("run_program_text should succeed");
        assert_eq!(cpu.registers[2], 30);
        assert_eq!(cpu.stats().cycles, 4);
    }

    #[test]
    fn run_program_text_retorna_erro_de_parse() {
        let source = "LOAD_CONST r9 10";
        let err = run_program_text(source, 8).expect_err("run_program_text should fail");
        assert_eq!(
            err,
            VirtualCpuError::InvalidRegister {
                line: 1,
                register: "r9".to_string()
            }
        );
    }

    #[test]
    fn parse_label_com_jump() {
        let source = "\
JUMP fim
LOAD_CONST r0 99
fim: HALT
";
        let program = parse_program(source).expect("parse should succeed");
        assert_eq!(program[0], Instruction::Jump { target: 2 });
        assert_eq!(program[2], Instruction::Halt);
    }

    #[test]
    fn parse_label_desconhecido_retorna_erro() {
        let source = "JUMP inexistente";
        let err = parse_program(source).expect_err("should fail");
        assert_eq!(
            err,
            VirtualCpuError::ParseError {
                line: 1,
                message: "unknown label: inexistente".to_string()
            }
        );
    }

    #[test]
    fn parse_label_com_traco_e_underscore() {
        let source = "\
loop_main-1: NOP
JUMP loop_main-1
";
        let program = parse_program(source).expect("parse should succeed");
        assert_eq!(program[0], Instruction::Nop);
        assert_eq!(program[1], Instruction::Jump { target: 0 });
    }

    #[test]
    fn parse_label_invalido_retorna_erro() {
        let source = "1loop: HALT";
        let err = parse_program(source).expect_err("should fail");
        assert_eq!(
            err,
            VirtualCpuError::ParseError {
                line: 1,
                message: "invalid label: 1loop".to_string()
            }
        );
    }

    #[test]
    fn parse_ignora_comentario_inline_ponto_virgula() {
        let source = "\
LOAD_CONST r0 10 ; carrega dez
HALT ; fim
";
        let program = parse_program(source).expect("parse should succeed");
        assert_eq!(
            program,
            vec![
                Instruction::LoadConst { reg: 0, value: 10 },
                Instruction::Halt
            ]
        );
    }

    #[test]
    fn parse_load_const_hexadecimal() {
        let program = parse_program("LOAD_CONST r0 0x1e").expect("parse should succeed");
        assert_eq!(program, vec![Instruction::LoadConst { reg: 0, value: 30 }]);
    }

    #[test]
    fn parse_endereco_hexadecimal() {
        let program = parse_program("STORE 0x0a r2").expect("parse should succeed");
        assert_eq!(program, vec![Instruction::Store { addr: 10, reg: 2 }]);
    }
}
