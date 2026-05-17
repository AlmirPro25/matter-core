#![no_std]
//! Tiny Matter MBC1 loader and execution core for kernel targets.
//!
//! This crate intentionally starts small. It validates real Matter bytecode
//! without `std`, then executes a safe integer subset that is useful for the
//! first Sentinel L3 boot tests.

extern crate alloc;

use alloc::vec::Vec;

pub const MBC1_MAGIC: [u8; 4] = *b"MBC1";
pub const MBC1_VERSION_MAJOR: u8 = 0;
pub const MBC1_VERSION_MINOR: u8 = 1;
pub const MAX_BYTECODE_BYTES: usize = 128 * 1024;
pub const MAX_CONSTANTS: u32 = 4096;
pub const MAX_FUNCTIONS: u32 = 1024;
pub const MAX_EVENTS: u32 = 1024;
pub const MAX_INSTRUCTIONS: u32 = 65536;
pub const MAX_STRING_BYTES: u32 = 4096;
pub const MAX_STACK: usize = 1024;
pub const MAX_FUNCTION_PARAMS: u32 = 64;
pub const MAX_CALL_DEPTH: u32 = 32;
pub const MAX_SYSCALLS: usize = 256;

const SECTION_CONSTANTS: u8 = 0x01;
const SECTION_FUNCTIONS: u8 = 0x02;
const SECTION_EVENTS: u8 = 0x03;
const SECTION_MAIN: u8 = 0x04;
const SECTION_END: u8 = 0xFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelVmError {
    TooSmall,
    TooLarge,
    UnknownMagic,
    UnsupportedVersion,
    InvalidSection,
    Truncated,
    LimitExceeded,
    InvalidUtf8,
    UnknownConstant,
    UnknownOpcode,
    ConstantOutOfBounds,
    StackOverflow,
    StackUnderflow,
    TypeError,
    DivisionByZero,
    UnsupportedInstruction,
    InstructionBudgetExceeded,
    ProgramCounterOutOfBounds,
    UndefinedVariable,
    UndefinedFunction,
    ArgumentCountMismatch,
    CallDepthExceeded,
    SyscallLimitExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantRef<'a> {
    Int(i64),
    Float(u64),
    Bool(bool),
    String(&'a [u8]),
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionRef<'a> {
    LoadConst(u32),
    LoadGlobal(&'a [u8]),
    StoreGlobal(&'a [u8]),
    LoadLocal(&'a [u8]),
    StoreLocal(&'a [u8]),
    PushScope,
    PopScope,
    StoreExisting(&'a [u8]),
    LoadParam(u32),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    And,
    Or,
    Not,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Jump(u32),
    JumpIfFalse(u32),
    Call(u32),
    Return,
    SpawnEvent(&'a [u8]),
    CallNamed {
        name: &'a [u8],
        arg_count: u32,
    },
    Print,
    BackendCall {
        backend: &'a [u8],
        method: &'a [u8],
        arg_count: u32,
    },
    NewList(u32),
    LoadIndex,
    StoreIndex,
    ListPush,
    ListPop,
    ListLen,
    StoreIndexVar(&'a [u8]),
    ListPushVar(&'a [u8]),
    ListPopVar(&'a [u8]),
    NewMap(u32),
    MapHas,
    MapKeys,
    MapValues,
    NewStruct {
        type_name: &'a [u8],
        size: u32,
    },
    LoadField(&'a [u8]),
    StoreFieldVar {
        target: &'a [u8],
        field: &'a [u8],
    },
    Pop,
    Halt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MbcInfo {
    pub version_major: u8,
    pub version_minor: u8,
    pub flags: u16,
    pub declared_total_instructions: u32,
    pub constants: u32,
    pub functions: u32,
    pub events: u32,
    pub main_instructions: u32,
    pub decoded_total_instructions: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelValue {
    Int(i64),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSyscall<'a> {
    pub backend: &'a [u8],
    pub method: &'a [u8],
    pub args: Vec<KernelValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRunResult<'a> {
    pub last_value: KernelValue,
    pub prints: Vec<KernelValue>,
    pub syscalls: Vec<KernelSyscall<'a>>,
    pub instructions_executed: u32,
    pub max_call_depth: u32,
    pub halted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionRef<'a> {
    pub name: &'a [u8],
    pub param_count: u32,
    pub instructions: Vec<InstructionRef<'a>>,
}

pub fn inspect_mbc1(bytes: &[u8]) -> Result<MbcInfo, KernelVmError> {
    let mut cursor = MbcCursor::new(bytes)?;
    cursor.read_header()
}

pub fn run_mbc1_main(
    bytes: &[u8],
    instruction_budget: u32,
) -> Result<KernelRunResult<'_>, KernelVmError> {
    let mut cursor = MbcCursor::new(bytes)?;
    let layout = cursor.read_layout()?;
    let constants = cursor.decode_constants(layout.constants_start, layout.constants)?;
    let functions = cursor.decode_functions(layout.functions_start, layout.functions)?;
    let instructions = cursor.decode_instructions(layout.main_start)?;
    run_main(&constants, &functions, &instructions, instruction_budget)
}

fn run_main<'a>(
    constants: &[ConstantRef<'a>],
    functions: &[FunctionRef<'a>],
    instructions: &[InstructionRef<'a>],
    instruction_budget: u32,
) -> Result<KernelRunResult<'a>, KernelVmError> {
    let mut stack = Vec::new();
    let mut globals = Vec::new();
    let mut prints = Vec::new();
    let mut syscalls = Vec::new();
    let mut pc = 0usize;
    let mut executed = 0u32;
    let mut max_call_depth = 0u32;
    let mut last_value = KernelValue::Unit;
    let mut halted = false;

    while pc < instructions.len() {
        if executed >= instruction_budget {
            return Err(KernelVmError::InstructionBudgetExceeded);
        }
        executed += 1;

        match instructions[pc] {
            InstructionRef::LoadConst(index) => {
                let constant = constants
                    .get(index as usize)
                    .ok_or(KernelVmError::ConstantOutOfBounds)?;
                push_value(&mut stack, constant_to_kernel_value(*constant)?)?;
            }
            InstructionRef::LoadGlobal(name) | InstructionRef::LoadLocal(name) => {
                let value = load_global(&globals, name)?;
                push_value(&mut stack, value)?;
            }
            InstructionRef::StoreGlobal(name)
            | InstructionRef::StoreLocal(name)
            | InstructionRef::StoreExisting(name) => {
                let value = pop_value(&mut stack)?;
                store_global(&mut globals, name, value)?;
                last_value = value;
            }
            InstructionRef::Add => binary_int(&mut stack, |a, b| a.wrapping_add(b))?,
            InstructionRef::Sub => binary_int(&mut stack, |a, b| a.wrapping_sub(b))?,
            InstructionRef::Mul => binary_int(&mut stack, |a, b| a.wrapping_mul(b))?,
            InstructionRef::Div => binary_checked(&mut stack, |a, b| {
                if b == 0 {
                    Err(KernelVmError::DivisionByZero)
                } else {
                    Ok(a / b)
                }
            })?,
            InstructionRef::Mod => binary_checked(&mut stack, |a, b| {
                if b == 0 {
                    Err(KernelVmError::DivisionByZero)
                } else {
                    Ok(a % b)
                }
            })?,
            InstructionRef::Neg => {
                let value = pop_int(&mut stack)?;
                push_value(&mut stack, KernelValue::Int(value.wrapping_neg()))?;
            }
            InstructionRef::Eq => compare_values(&mut stack, |equal| equal)?,
            InstructionRef::NotEq => compare_values(&mut stack, |equal| !equal)?,
            InstructionRef::Lt => compare_int(&mut stack, |a, b| a < b)?,
            InstructionRef::Gt => compare_int(&mut stack, |a, b| a > b)?,
            InstructionRef::LtEq => compare_int(&mut stack, |a, b| a <= b)?,
            InstructionRef::GtEq => compare_int(&mut stack, |a, b| a >= b)?,
            InstructionRef::And => binary_bool(&mut stack, |a, b| a && b)?,
            InstructionRef::Or => binary_bool(&mut stack, |a, b| a || b)?,
            InstructionRef::Not => {
                let value = pop_bool(&mut stack)?;
                push_value(&mut stack, KernelValue::Bool(!value))?;
            }
            InstructionRef::Jump(target) => {
                pc = target as usize;
                if pc >= instructions.len() {
                    return Err(KernelVmError::ProgramCounterOutOfBounds);
                }
                continue;
            }
            InstructionRef::JumpIfFalse(target) => {
                if !truthy(pop_value(&mut stack)?) {
                    pc = target as usize;
                    if pc >= instructions.len() {
                        return Err(KernelVmError::ProgramCounterOutOfBounds);
                    }
                    continue;
                }
            }
            InstructionRef::CallNamed { name, arg_count } => {
                let args = pop_args(&mut stack, arg_count)?;
                let value = run_function(constants, functions, name, &args, instruction_budget, 0)?;
                executed = executed.saturating_add(value.instructions_executed);
                if executed > instruction_budget {
                    return Err(KernelVmError::InstructionBudgetExceeded);
                }
                max_call_depth = max_call_depth.max(value.max_call_depth);
                prints.extend(value.prints);
                append_syscalls(&mut syscalls, value.syscalls)?;
                push_value(&mut stack, value.value)?;
                last_value = value.value;
            }
            InstructionRef::BackendCall {
                backend,
                method,
                arg_count,
            } => {
                let args = pop_args(&mut stack, arg_count)?;
                record_syscall(&mut syscalls, backend, method, args)?;
                push_value(&mut stack, KernelValue::Unit)?;
                last_value = KernelValue::Unit;
            }
            InstructionRef::Print => {
                let value = pop_value(&mut stack)?;
                last_value = value;
                prints.push(value);
            }
            InstructionRef::Pop => {
                last_value = pop_value(&mut stack)?;
            }
            InstructionRef::Halt | InstructionRef::Return => {
                halted = true;
                break;
            }
            InstructionRef::PushScope | InstructionRef::PopScope => {}
            _ => return Err(KernelVmError::UnsupportedInstruction),
        }

        pc += 1;
    }

    if let Some(value) = stack.last().copied() {
        last_value = value;
    }

    Ok(KernelRunResult {
        last_value,
        prints,
        syscalls,
        instructions_executed: executed,
        max_call_depth,
        halted,
    })
}

#[derive(Debug, Clone)]
struct FunctionRunResult<'a> {
    value: KernelValue,
    prints: Vec<KernelValue>,
    syscalls: Vec<KernelSyscall<'a>>,
    instructions_executed: u32,
    max_call_depth: u32,
}

fn run_function<'a>(
    constants: &[ConstantRef<'a>],
    functions: &[FunctionRef<'a>],
    name: &[u8],
    args: &[KernelValue],
    instruction_budget: u32,
    call_depth: u32,
) -> Result<FunctionRunResult<'a>, KernelVmError> {
    if call_depth >= MAX_CALL_DEPTH {
        return Err(KernelVmError::CallDepthExceeded);
    }
    let function = functions
        .iter()
        .find(|function| function.name == name)
        .ok_or(KernelVmError::UndefinedFunction)?;
    if function.param_count != args.len() as u32 {
        return Err(KernelVmError::ArgumentCountMismatch);
    }

    let mut stack = Vec::new();
    let mut locals = Vec::new();
    let mut prints = Vec::new();
    let mut syscalls = Vec::new();
    let mut executed = 0u32;
    let mut max_call_depth = call_depth + 1;
    let mut pc = 0usize;
    let mut last_value = KernelValue::Unit;

    while pc < function.instructions.len() {
        if executed >= instruction_budget {
            return Err(KernelVmError::InstructionBudgetExceeded);
        }
        executed += 1;

        match function.instructions[pc] {
            InstructionRef::LoadConst(index) => {
                let constant = constants
                    .get(index as usize)
                    .ok_or(KernelVmError::ConstantOutOfBounds)?;
                push_value(&mut stack, constant_to_kernel_value(*constant)?)?;
            }
            InstructionRef::LoadParam(index) => {
                let value = args
                    .get(index as usize)
                    .copied()
                    .ok_or(KernelVmError::ArgumentCountMismatch)?;
                push_value(&mut stack, value)?;
            }
            InstructionRef::LoadGlobal(name) | InstructionRef::LoadLocal(name) => {
                let value = load_global(&locals, name)?;
                push_value(&mut stack, value)?;
            }
            InstructionRef::StoreGlobal(name)
            | InstructionRef::StoreLocal(name)
            | InstructionRef::StoreExisting(name) => {
                let value = pop_value(&mut stack)?;
                store_global(&mut locals, name, value)?;
                last_value = value;
            }
            InstructionRef::Add => binary_int(&mut stack, |a, b| a.wrapping_add(b))?,
            InstructionRef::Sub => binary_int(&mut stack, |a, b| a.wrapping_sub(b))?,
            InstructionRef::Mul => binary_int(&mut stack, |a, b| a.wrapping_mul(b))?,
            InstructionRef::Div => binary_checked(&mut stack, |a, b| {
                if b == 0 {
                    Err(KernelVmError::DivisionByZero)
                } else {
                    Ok(a / b)
                }
            })?,
            InstructionRef::Mod => binary_checked(&mut stack, |a, b| {
                if b == 0 {
                    Err(KernelVmError::DivisionByZero)
                } else {
                    Ok(a % b)
                }
            })?,
            InstructionRef::Neg => {
                let value = pop_int(&mut stack)?;
                push_value(&mut stack, KernelValue::Int(value.wrapping_neg()))?;
            }
            InstructionRef::And => binary_bool(&mut stack, |a, b| a && b)?,
            InstructionRef::Or => binary_bool(&mut stack, |a, b| a || b)?,
            InstructionRef::Not => {
                let value = pop_bool(&mut stack)?;
                push_value(&mut stack, KernelValue::Bool(!value))?;
            }
            InstructionRef::Eq => compare_values(&mut stack, |equal| equal)?,
            InstructionRef::NotEq => compare_values(&mut stack, |equal| !equal)?,
            InstructionRef::Lt => compare_int(&mut stack, |a, b| a < b)?,
            InstructionRef::Gt => compare_int(&mut stack, |a, b| a > b)?,
            InstructionRef::LtEq => compare_int(&mut stack, |a, b| a <= b)?,
            InstructionRef::GtEq => compare_int(&mut stack, |a, b| a >= b)?,
            InstructionRef::Jump(target) => {
                pc = target as usize;
                if pc >= function.instructions.len() {
                    return Err(KernelVmError::ProgramCounterOutOfBounds);
                }
                continue;
            }
            InstructionRef::JumpIfFalse(target) => {
                if !truthy(pop_value(&mut stack)?) {
                    pc = target as usize;
                    if pc >= function.instructions.len() {
                        return Err(KernelVmError::ProgramCounterOutOfBounds);
                    }
                    continue;
                }
            }
            InstructionRef::CallNamed { name, arg_count } => {
                let nested_args = pop_args(&mut stack, arg_count)?;
                let value = run_function(
                    constants,
                    functions,
                    name,
                    &nested_args,
                    instruction_budget.saturating_sub(executed),
                    call_depth + 1,
                )?;
                executed = executed.saturating_add(value.instructions_executed);
                if executed > instruction_budget {
                    return Err(KernelVmError::InstructionBudgetExceeded);
                }
                max_call_depth = max_call_depth.max(value.max_call_depth);
                prints.extend(value.prints);
                append_syscalls(&mut syscalls, value.syscalls)?;
                push_value(&mut stack, value.value)?;
                last_value = value.value;
            }
            InstructionRef::BackendCall {
                backend,
                method,
                arg_count,
            } => {
                let args = pop_args(&mut stack, arg_count)?;
                record_syscall(&mut syscalls, backend, method, args)?;
                push_value(&mut stack, KernelValue::Unit)?;
                last_value = KernelValue::Unit;
            }
            InstructionRef::Print => {
                let value = pop_value(&mut stack)?;
                last_value = value;
                prints.push(value);
            }
            InstructionRef::Pop => {
                last_value = pop_value(&mut stack)?;
            }
            InstructionRef::Return => {
                let value = stack.pop().unwrap_or(KernelValue::Unit);
                return Ok(FunctionRunResult {
                    value,
                    prints,
                    syscalls,
                    instructions_executed: executed,
                    max_call_depth,
                });
            }
            InstructionRef::Halt => {
                break;
            }
            InstructionRef::PushScope | InstructionRef::PopScope => {}
            _ => return Err(KernelVmError::UnsupportedInstruction),
        }

        if let Some(value) = stack.last().copied() {
            last_value = value;
        }
        pc += 1;
    }

    Ok(FunctionRunResult {
        value: last_value,
        prints,
        syscalls,
        instructions_executed: executed,
        max_call_depth,
    })
}

fn record_syscall<'a>(
    syscalls: &mut Vec<KernelSyscall<'a>>,
    backend: &'a [u8],
    method: &'a [u8],
    args: Vec<KernelValue>,
) -> Result<(), KernelVmError> {
    if syscalls.len() >= MAX_SYSCALLS {
        return Err(KernelVmError::SyscallLimitExceeded);
    }
    syscalls.push(KernelSyscall {
        backend,
        method,
        args,
    });
    Ok(())
}

fn append_syscalls<'a>(
    syscalls: &mut Vec<KernelSyscall<'a>>,
    mut nested: Vec<KernelSyscall<'a>>,
) -> Result<(), KernelVmError> {
    if syscalls.len().saturating_add(nested.len()) > MAX_SYSCALLS {
        return Err(KernelVmError::SyscallLimitExceeded);
    }
    syscalls.append(&mut nested);
    Ok(())
}

fn load_global(
    globals: &[(&[u8], KernelValue)],
    name: &[u8],
) -> Result<KernelValue, KernelVmError> {
    globals
        .iter()
        .rev()
        .find_map(|(key, value)| if *key == name { Some(*value) } else { None })
        .ok_or(KernelVmError::UndefinedVariable)
}

fn store_global<'a>(
    globals: &mut Vec<(&'a [u8], KernelValue)>,
    name: &'a [u8],
    value: KernelValue,
) -> Result<(), KernelVmError> {
    if let Some((_, existing)) = globals.iter_mut().rev().find(|(key, _)| *key == name) {
        *existing = value;
        return Ok(());
    }
    globals.push((name, value));
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct MbcLayout {
    constants_start: usize,
    constants: u32,
    functions_start: usize,
    functions: u32,
    main_start: usize,
}

struct MbcCursor<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> MbcCursor<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, KernelVmError> {
        if bytes.len() > MAX_BYTECODE_BYTES {
            return Err(KernelVmError::TooLarge);
        }
        if bytes.len() < 12 {
            return Err(KernelVmError::TooSmall);
        }
        Ok(Self { bytes, index: 0 })
    }

    fn read_header(&mut self) -> Result<MbcInfo, KernelVmError> {
        let (version_major, version_minor, flags, declared_total_instructions) =
            self.read_raw_header()?;
        let mut info = MbcInfo {
            version_major,
            version_minor,
            flags,
            declared_total_instructions,
            constants: 0,
            functions: 0,
            events: 0,
            main_instructions: 0,
            decoded_total_instructions: 0,
        };

        loop {
            let section = self.read_u8()?;
            match section {
                SECTION_CONSTANTS => {
                    info.constants = self.read_count(MAX_CONSTANTS)?;
                    for _ in 0..info.constants {
                        self.skip_constant()?;
                    }
                }
                SECTION_FUNCTIONS => {
                    info.functions = self.read_count(MAX_FUNCTIONS)?;
                    for _ in 0..info.functions {
                        self.skip_string()?;
                        let _param_count = self.read_u32()?;
                        info.decoded_total_instructions = info
                            .decoded_total_instructions
                            .checked_add(self.skip_instructions()?)
                            .ok_or(KernelVmError::LimitExceeded)?;
                    }
                }
                SECTION_EVENTS => {
                    info.events = self.read_count(MAX_EVENTS)?;
                    for _ in 0..info.events {
                        self.skip_string()?;
                        info.decoded_total_instructions = info
                            .decoded_total_instructions
                            .checked_add(self.skip_instructions()?)
                            .ok_or(KernelVmError::LimitExceeded)?;
                    }
                }
                SECTION_MAIN => {
                    info.main_instructions = self.skip_instructions()?;
                    info.decoded_total_instructions = info
                        .decoded_total_instructions
                        .checked_add(info.main_instructions)
                        .ok_or(KernelVmError::LimitExceeded)?;
                }
                SECTION_END => break,
                _ => return Err(KernelVmError::InvalidSection),
            }
        }

        if self.index != self.bytes.len()
            || info.decoded_total_instructions != declared_total_instructions
        {
            return Err(KernelVmError::LimitExceeded);
        }

        Ok(info)
    }

    fn read_layout(&mut self) -> Result<MbcLayout, KernelVmError> {
        self.read_raw_header()?;
        let mut layout = MbcLayout {
            constants_start: 0,
            constants: 0,
            functions_start: 0,
            functions: 0,
            main_start: 0,
        };

        loop {
            let section = self.read_u8()?;
            match section {
                SECTION_CONSTANTS => {
                    layout.constants_start = self.index;
                    layout.constants = self.read_count(MAX_CONSTANTS)?;
                    for _ in 0..layout.constants {
                        self.skip_constant()?;
                    }
                }
                SECTION_FUNCTIONS => {
                    layout.functions_start = self.index;
                    layout.functions = self.read_count(MAX_FUNCTIONS)?;
                    for _ in 0..layout.functions {
                        self.skip_string()?;
                        let _param_count = self.read_u32()?;
                        self.skip_instructions()?;
                    }
                }
                SECTION_EVENTS => {
                    let count = self.read_count(MAX_EVENTS)?;
                    for _ in 0..count {
                        self.skip_string()?;
                        self.skip_instructions()?;
                    }
                }
                SECTION_MAIN => {
                    layout.main_start = self.index;
                    self.skip_instructions()?;
                }
                SECTION_END => break,
                _ => return Err(KernelVmError::InvalidSection),
            }
        }

        Ok(layout)
    }

    fn decode_constants(
        &self,
        start: usize,
        count: u32,
    ) -> Result<Vec<ConstantRef<'a>>, KernelVmError> {
        let mut cursor = Self {
            bytes: self.bytes,
            index: start,
        };
        let encoded_count = cursor.read_count(MAX_CONSTANTS)?;
        if encoded_count != count {
            return Err(KernelVmError::LimitExceeded);
        }
        let mut constants = Vec::new();
        for _ in 0..count {
            constants.push(cursor.read_constant()?);
        }
        Ok(constants)
    }

    fn decode_functions(
        &self,
        start: usize,
        count: u32,
    ) -> Result<Vec<FunctionRef<'a>>, KernelVmError> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let mut cursor = Self {
            bytes: self.bytes,
            index: start,
        };
        let encoded_count = cursor.read_count(MAX_FUNCTIONS)?;
        if encoded_count != count {
            return Err(KernelVmError::LimitExceeded);
        }
        let mut functions = Vec::new();
        for _ in 0..count {
            let name = cursor.read_string()?;
            let param_count = cursor.read_count(MAX_FUNCTION_PARAMS)?;
            let instructions = cursor.decode_instructions_at_cursor()?;
            functions.push(FunctionRef {
                name,
                param_count,
                instructions,
            });
        }
        Ok(functions)
    }

    fn decode_instructions(&self, start: usize) -> Result<Vec<InstructionRef<'a>>, KernelVmError> {
        let mut cursor = Self {
            bytes: self.bytes,
            index: start,
        };
        cursor.decode_instructions_at_cursor()
    }

    fn decode_instructions_at_cursor(&mut self) -> Result<Vec<InstructionRef<'a>>, KernelVmError> {
        let count = self.read_count(MAX_INSTRUCTIONS)?;
        let mut instructions = Vec::new();
        for _ in 0..count {
            instructions.push(self.read_instruction()?);
        }
        Ok(instructions)
    }

    fn read_raw_header(&mut self) -> Result<(u8, u8, u16, u32), KernelVmError> {
        if self.read_bytes(4)? != MBC1_MAGIC {
            return Err(KernelVmError::UnknownMagic);
        }
        let version = self.read_u16()?;
        let major = (version >> 8) as u8;
        let minor = (version & 0xFF) as u8;
        if major != MBC1_VERSION_MAJOR || minor > MBC1_VERSION_MINOR {
            return Err(KernelVmError::UnsupportedVersion);
        }
        let flags = self.read_u16()?;
        let total = self.read_u32()?;
        if total > MAX_INSTRUCTIONS {
            return Err(KernelVmError::LimitExceeded);
        }
        Ok((major, minor, flags, total))
    }

    fn read_constant(&mut self) -> Result<ConstantRef<'a>, KernelVmError> {
        match self.read_u8()? {
            0x01 => Ok(ConstantRef::Int(self.read_i64()?)),
            0x02 => Ok(ConstantRef::Bool(self.read_u8()? != 0)),
            0x03 => Ok(ConstantRef::String(self.read_string()?)),
            0x04 => Ok(ConstantRef::Unit),
            0x05 => Ok(ConstantRef::Float(self.read_u64()?)),
            _ => Err(KernelVmError::UnknownConstant),
        }
    }

    fn read_instruction(&mut self) -> Result<InstructionRef<'a>, KernelVmError> {
        Ok(match self.read_u8()? {
            0x01 => InstructionRef::LoadConst(self.read_u32()?),
            0x02 => InstructionRef::LoadGlobal(self.read_string()?),
            0x03 => InstructionRef::StoreGlobal(self.read_string()?),
            0x04 => InstructionRef::LoadLocal(self.read_string()?),
            0x05 => InstructionRef::StoreLocal(self.read_string()?),
            0x06 => InstructionRef::PushScope,
            0x07 => InstructionRef::PopScope,
            0x08 => InstructionRef::StoreExisting(self.read_string()?),
            0x09 => InstructionRef::LoadParam(self.read_u32()?),
            0x10 => InstructionRef::Add,
            0x11 => InstructionRef::Sub,
            0x12 => InstructionRef::Mul,
            0x13 => InstructionRef::Div,
            0x14 => InstructionRef::Mod,
            0x15 => InstructionRef::Neg,
            0x16 => InstructionRef::And,
            0x17 => InstructionRef::Or,
            0x18 => InstructionRef::Not,
            0x20 => InstructionRef::Eq,
            0x21 => InstructionRef::NotEq,
            0x22 => InstructionRef::Lt,
            0x23 => InstructionRef::Gt,
            0x24 => InstructionRef::LtEq,
            0x25 => InstructionRef::GtEq,
            0x30 => InstructionRef::Jump(self.read_u32()?),
            0x31 => InstructionRef::JumpIfFalse(self.read_u32()?),
            0x40 => InstructionRef::Call(self.read_u32()?),
            0x41 => InstructionRef::Return,
            0x42 => InstructionRef::SpawnEvent(self.read_string()?),
            0x43 => {
                let name = self.read_string()?;
                let arg_count = self.read_u32()?;
                InstructionRef::CallNamed { name, arg_count }
            }
            0x50 => InstructionRef::Print,
            0x60 => {
                let backend = self.read_string()?;
                let method = self.read_string()?;
                let arg_count = self.read_u32()?;
                InstructionRef::BackendCall {
                    backend,
                    method,
                    arg_count,
                }
            }
            0x70 => InstructionRef::Pop,
            0x80 => InstructionRef::NewList(self.read_u32()?),
            0x81 => InstructionRef::LoadIndex,
            0x82 => InstructionRef::StoreIndex,
            0x83 => InstructionRef::ListPush,
            0x84 => InstructionRef::ListPop,
            0x85 => InstructionRef::ListLen,
            0x86 => InstructionRef::StoreIndexVar(self.read_string()?),
            0x87 => InstructionRef::ListPushVar(self.read_string()?),
            0x88 => InstructionRef::ListPopVar(self.read_string()?),
            0x90 => InstructionRef::NewMap(self.read_u32()?),
            0x91 => InstructionRef::MapHas,
            0x92 => InstructionRef::MapKeys,
            0x93 => InstructionRef::MapValues,
            0xA0 => {
                let type_name = self.read_string()?;
                let size = self.read_u32()?;
                InstructionRef::NewStruct { type_name, size }
            }
            0xA1 => InstructionRef::LoadField(self.read_string()?),
            0xA2 => {
                let target = self.read_string()?;
                let field = self.read_string()?;
                InstructionRef::StoreFieldVar { target, field }
            }
            0xFF => InstructionRef::Halt,
            _ => return Err(KernelVmError::UnknownOpcode),
        })
    }

    fn skip_constant(&mut self) -> Result<(), KernelVmError> {
        let _ = self.read_constant()?;
        Ok(())
    }

    fn skip_instructions(&mut self) -> Result<u32, KernelVmError> {
        let count = self.read_count(MAX_INSTRUCTIONS)?;
        for _ in 0..count {
            let _ = self.read_instruction()?;
        }
        Ok(count)
    }

    fn skip_string(&mut self) -> Result<(), KernelVmError> {
        let _ = self.read_string()?;
        Ok(())
    }

    fn read_count(&mut self, max: u32) -> Result<u32, KernelVmError> {
        let count = self.read_u32()?;
        if count > max {
            return Err(KernelVmError::LimitExceeded);
        }
        Ok(count)
    }

    fn read_string(&mut self) -> Result<&'a [u8], KernelVmError> {
        let len = self.read_u32()?;
        if len > MAX_STRING_BYTES {
            return Err(KernelVmError::LimitExceeded);
        }
        let bytes = self.read_bytes(len as usize)?;
        if core::str::from_utf8(bytes).is_err() {
            return Err(KernelVmError::InvalidUtf8);
        }
        Ok(bytes)
    }

    fn read_u8(&mut self) -> Result<u8, KernelVmError> {
        Ok(self.read_bytes(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, KernelVmError> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, KernelVmError> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, KernelVmError> {
        let bytes = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_i64(&mut self) -> Result<i64, KernelVmError> {
        let bytes = self.read_bytes(8)?;
        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], KernelVmError> {
        let end = self
            .index
            .checked_add(len)
            .ok_or(KernelVmError::Truncated)?;
        if end > self.bytes.len() {
            return Err(KernelVmError::Truncated);
        }
        let bytes = &self.bytes[self.index..end];
        self.index = end;
        Ok(bytes)
    }
}

fn constant_to_kernel_value(value: ConstantRef<'_>) -> Result<KernelValue, KernelVmError> {
    match value {
        ConstantRef::Int(value) => Ok(KernelValue::Int(value)),
        ConstantRef::Bool(value) => Ok(KernelValue::Bool(value)),
        ConstantRef::Unit => Ok(KernelValue::Unit),
        ConstantRef::Float(_) | ConstantRef::String(_) => Err(KernelVmError::TypeError),
    }
}

fn push_value(stack: &mut Vec<KernelValue>, value: KernelValue) -> Result<(), KernelVmError> {
    if stack.len() >= MAX_STACK {
        return Err(KernelVmError::StackOverflow);
    }
    stack.push(value);
    Ok(())
}

fn pop_value(stack: &mut Vec<KernelValue>) -> Result<KernelValue, KernelVmError> {
    stack.pop().ok_or(KernelVmError::StackUnderflow)
}

fn pop_args(
    stack: &mut Vec<KernelValue>,
    arg_count: u32,
) -> Result<Vec<KernelValue>, KernelVmError> {
    let mut args = Vec::new();
    for _ in 0..arg_count {
        args.push(pop_value(stack)?);
    }
    args.reverse();
    Ok(args)
}

fn pop_int(stack: &mut Vec<KernelValue>) -> Result<i64, KernelVmError> {
    match pop_value(stack)? {
        KernelValue::Int(value) => Ok(value),
        _ => Err(KernelVmError::TypeError),
    }
}

fn pop_bool(stack: &mut Vec<KernelValue>) -> Result<bool, KernelVmError> {
    match pop_value(stack)? {
        KernelValue::Bool(value) => Ok(value),
        _ => Err(KernelVmError::TypeError),
    }
}

fn binary_int<F>(stack: &mut Vec<KernelValue>, op: F) -> Result<(), KernelVmError>
where
    F: Fn(i64, i64) -> i64,
{
    binary_checked(stack, |a, b| Ok(op(a, b)))
}

fn binary_checked<F>(stack: &mut Vec<KernelValue>, op: F) -> Result<(), KernelVmError>
where
    F: Fn(i64, i64) -> Result<i64, KernelVmError>,
{
    let rhs = pop_int(stack)?;
    let lhs = pop_int(stack)?;
    push_value(stack, KernelValue::Int(op(lhs, rhs)?))
}

fn binary_bool<F>(stack: &mut Vec<KernelValue>, op: F) -> Result<(), KernelVmError>
where
    F: Fn(bool, bool) -> bool,
{
    let rhs = pop_bool(stack)?;
    let lhs = pop_bool(stack)?;
    push_value(stack, KernelValue::Bool(op(lhs, rhs)))
}

fn compare_int<F>(stack: &mut Vec<KernelValue>, op: F) -> Result<(), KernelVmError>
where
    F: Fn(i64, i64) -> bool,
{
    let rhs = pop_int(stack)?;
    let lhs = pop_int(stack)?;
    push_value(stack, KernelValue::Bool(op(lhs, rhs)))
}

fn compare_values<F>(stack: &mut Vec<KernelValue>, op: F) -> Result<(), KernelVmError>
where
    F: Fn(bool) -> bool,
{
    let rhs = pop_value(stack)?;
    let lhs = pop_value(stack)?;
    push_value(stack, KernelValue::Bool(op(lhs == rhs)))
}

fn truthy(value: KernelValue) -> bool {
    match value {
        KernelValue::Bool(value) => value,
        KernelValue::Int(value) => value != 0,
        KernelValue::Unit => false,
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn inspect_empty_mbc1_reports_sections() {
        let bytes = empty_program(&[0xFF]);
        let info = inspect_mbc1(&bytes).unwrap();

        assert_eq!(info.version_major, 0);
        assert_eq!(info.version_minor, 1);
        assert_eq!(info.constants, 0);
        assert_eq!(info.main_instructions, 1);
        assert_eq!(info.declared_total_instructions, 1);
        assert_eq!(info.decoded_total_instructions, 1);
    }

    #[test]
    fn run_integer_arithmetic_main() {
        let mut bytes = Vec::new();
        push_header(&mut bytes, 5);
        push_constants(&mut bytes, &[ConstantRef::Int(40), ConstantRef::Int(2)]);
        push_empty_functions(&mut bytes);
        push_empty_events(&mut bytes);
        bytes.push(SECTION_MAIN);
        push_u32(&mut bytes, 5);
        bytes.push(0x01);
        push_u32(&mut bytes, 0);
        bytes.push(0x01);
        push_u32(&mut bytes, 1);
        bytes.push(0x10);
        bytes.push(0x50);
        bytes.push(0xFF);
        bytes.push(SECTION_END);

        let result = run_mbc1_main(&bytes, 64).unwrap();

        assert_eq!(result.prints, vec![KernelValue::Int(42)]);
        assert_eq!(result.last_value, KernelValue::Int(42));
        assert_eq!(result.max_call_depth, 0);
        assert!(result.halted);
    }

    #[test]
    fn run_global_variables_in_main() {
        let mut bytes = Vec::new();
        push_header(&mut bytes, 9);
        push_constants(&mut bytes, &[ConstantRef::Int(40), ConstantRef::Int(2)]);
        push_empty_functions(&mut bytes);
        push_empty_events(&mut bytes);
        bytes.push(SECTION_MAIN);
        push_u32(&mut bytes, 9);
        bytes.push(0x01);
        push_u32(&mut bytes, 0);
        push_store_global(&mut bytes, b"base");
        bytes.push(0x01);
        push_u32(&mut bytes, 1);
        push_store_global(&mut bytes, b"delta");
        push_load_global(&mut bytes, b"base");
        push_load_global(&mut bytes, b"delta");
        bytes.push(0x10);
        bytes.push(0x50);
        bytes.push(0xFF);
        bytes.push(SECTION_END);

        let result = run_mbc1_main(&bytes, 64).unwrap();

        assert_eq!(result.prints, vec![KernelValue::Int(42)]);
        assert_eq!(result.last_value, KernelValue::Int(42));
        assert_eq!(result.max_call_depth, 0);
        assert!(result.halted);
    }

    #[test]
    fn run_loop_and_branch_in_main() {
        let bytes = loop_branch_program();
        let result = run_mbc1_main(&bytes, 128).unwrap();

        assert_eq!(result.prints, vec![KernelValue::Int(15)]);
        assert_eq!(result.last_value, KernelValue::Int(15));
        assert_eq!(result.max_call_depth, 0);
        assert!(result.halted);
    }

    #[test]
    fn run_named_function_call_in_main() {
        let mut bytes = Vec::new();
        push_header(&mut bytes, 11);
        push_constants(&mut bytes, &[ConstantRef::Int(7), ConstantRef::Int(8)]);
        bytes.push(SECTION_FUNCTIONS);
        push_u32(&mut bytes, 1);
        push_function_add(&mut bytes);
        push_empty_events(&mut bytes);
        bytes.push(SECTION_MAIN);
        push_u32(&mut bytes, 7);
        bytes.push(0x01);
        push_u32(&mut bytes, 0);
        bytes.push(0x01);
        push_u32(&mut bytes, 1);
        push_call_named(&mut bytes, b"add", 2);
        push_store_global(&mut bytes, b"total");
        push_load_global(&mut bytes, b"total");
        bytes.push(0x50);
        bytes.push(0xFF);
        bytes.push(SECTION_END);

        let result = run_mbc1_main(&bytes, 64).unwrap();

        assert_eq!(result.prints, vec![KernelValue::Int(15)]);
        assert_eq!(result.last_value, KernelValue::Int(15));
        assert_eq!(result.max_call_depth, 1);
        assert!(result.halted);
    }

    #[test]
    fn function_prints_are_propagated_to_main_result() {
        let mut bytes = Vec::new();
        push_header(&mut bytes, 7);
        push_constants(&mut bytes, &[ConstantRef::Int(40), ConstantRef::Int(2)]);
        bytes.push(SECTION_FUNCTIONS);
        push_u32(&mut bytes, 1);
        push_function_print_then_return(&mut bytes);
        push_empty_events(&mut bytes);
        bytes.push(SECTION_MAIN);
        push_u32(&mut bytes, 3);
        push_call_named(&mut bytes, b"emit", 0);
        bytes.push(0x50);
        bytes.push(0xFF);
        bytes.push(SECTION_END);

        let result = run_mbc1_main(&bytes, 64).unwrap();

        assert_eq!(
            result.prints,
            vec![KernelValue::Int(40), KernelValue::Int(2)]
        );
        assert_eq!(result.last_value, KernelValue::Int(2));
        assert_eq!(result.max_call_depth, 1);
        assert!(result.halted);
    }

    #[test]
    fn scalar_bool_ops_work_inside_functions() {
        let mut bytes = Vec::new();
        push_header(&mut bytes, 10);
        push_constants(
            &mut bytes,
            &[ConstantRef::Bool(true), ConstantRef::Bool(false)],
        );
        bytes.push(SECTION_FUNCTIONS);
        push_u32(&mut bytes, 1);
        push_function_bool_ops(&mut bytes);
        push_empty_events(&mut bytes);
        bytes.push(SECTION_MAIN);
        push_u32(&mut bytes, 3);
        push_call_named(&mut bytes, b"bool_ops", 0);
        bytes.push(0x50);
        bytes.push(0xFF);
        bytes.push(SECTION_END);

        let result = run_mbc1_main(&bytes, 64).unwrap();

        assert_eq!(result.prints, vec![KernelValue::Bool(true)]);
        assert_eq!(result.last_value, KernelValue::Bool(true));
        assert_eq!(result.max_call_depth, 1);
        assert!(result.halted);
    }

    #[test]
    fn run_recursive_factorial_with_depth_guard() {
        let bytes = recursive_factorial_program();
        let result = run_mbc1_main(&bytes, 256).unwrap();

        assert_eq!(result.prints, vec![KernelValue::Int(120)]);
        assert_eq!(result.last_value, KernelValue::Int(120));
        assert_eq!(result.max_call_depth, 5);
        assert!(result.halted);
    }

    #[test]
    fn unsupported_string_print_is_rejected_cleanly() {
        let mut bytes = Vec::new();
        push_header(&mut bytes, 3);
        push_constants(&mut bytes, &[ConstantRef::String(b"sentinel")]);
        push_empty_functions(&mut bytes);
        push_empty_events(&mut bytes);
        bytes.push(SECTION_MAIN);
        push_u32(&mut bytes, 3);
        bytes.push(0x01);
        push_u32(&mut bytes, 0);
        bytes.push(0x50);
        bytes.push(0xFF);
        bytes.push(SECTION_END);

        assert_eq!(run_mbc1_main(&bytes, 64), Err(KernelVmError::TypeError));
    }

    #[test]
    fn backend_call_is_recorded_as_controlled_syscall() {
        let mut bytes = Vec::new();
        push_header(&mut bytes, 6);
        push_constants(&mut bytes, &[ConstantRef::Int(120), ConstantRef::Int(1)]);
        push_empty_functions(&mut bytes);
        push_empty_events(&mut bytes);
        bytes.push(SECTION_MAIN);
        push_u32(&mut bytes, 6);
        bytes.push(0x01);
        push_u32(&mut bytes, 0);
        push_backend_call(&mut bytes, b"sentinel", b"telemetry", 1);
        bytes.push(0x70);
        bytes.push(0x01);
        push_u32(&mut bytes, 1);
        bytes.push(0x50);
        bytes.push(0xFF);
        bytes.push(SECTION_END);

        let result = run_mbc1_main(&bytes, 64).unwrap();

        assert_eq!(result.prints, vec![KernelValue::Int(1)]);
        assert_eq!(result.syscalls.len(), 1);
        assert_eq!(result.syscalls[0].backend, b"sentinel");
        assert_eq!(result.syscalls[0].method, b"telemetry");
        assert_eq!(result.syscalls[0].args, vec![KernelValue::Int(120)]);
    }

    fn empty_program(main: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_header(&mut bytes, 1);
        push_constants(&mut bytes, &[]);
        push_empty_functions(&mut bytes);
        push_empty_events(&mut bytes);
        bytes.push(SECTION_MAIN);
        push_u32(&mut bytes, 1);
        bytes.extend_from_slice(main);
        bytes.push(SECTION_END);
        bytes
    }

    fn push_header(bytes: &mut Vec<u8>, instruction_count: u32) {
        bytes.extend_from_slice(&MBC1_MAGIC);
        let version = ((MBC1_VERSION_MAJOR as u16) << 8) | MBC1_VERSION_MINOR as u16;
        push_u16(bytes, version);
        push_u16(bytes, 0);
        push_u32(bytes, instruction_count);
    }

    fn push_constants(bytes: &mut Vec<u8>, constants: &[ConstantRef<'_>]) {
        bytes.push(SECTION_CONSTANTS);
        push_u32(bytes, constants.len() as u32);
        for constant in constants {
            match constant {
                ConstantRef::Int(value) => {
                    bytes.push(0x01);
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                ConstantRef::Bool(value) => {
                    bytes.push(0x02);
                    bytes.push(*value as u8);
                }
                ConstantRef::String(value) => {
                    bytes.push(0x03);
                    push_u32(bytes, value.len() as u32);
                    bytes.extend_from_slice(value);
                }
                ConstantRef::Unit => bytes.push(0x04),
                ConstantRef::Float(value) => {
                    bytes.push(0x05);
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
    }

    fn push_empty_functions(bytes: &mut Vec<u8>) {
        bytes.push(SECTION_FUNCTIONS);
        push_u32(bytes, 0);
    }

    fn push_function_add(bytes: &mut Vec<u8>) {
        push_u32(bytes, 3);
        bytes.extend_from_slice(b"add");
        push_u32(bytes, 2);
        push_u32(bytes, 4);
        bytes.push(0x09);
        push_u32(bytes, 0);
        bytes.push(0x09);
        push_u32(bytes, 1);
        bytes.push(0x10);
        bytes.push(0x41);
    }

    fn push_function_print_then_return(bytes: &mut Vec<u8>) {
        push_u32(bytes, 4);
        bytes.extend_from_slice(b"emit");
        push_u32(bytes, 0);
        push_u32(bytes, 4);
        bytes.push(0x01);
        push_u32(bytes, 0);
        bytes.push(0x50);
        bytes.push(0x01);
        push_u32(bytes, 1);
        bytes.push(0x41);
    }

    fn push_function_bool_ops(bytes: &mut Vec<u8>) {
        push_u32(bytes, 8);
        bytes.extend_from_slice(b"bool_ops");
        push_u32(bytes, 0);
        push_u32(bytes, 7);
        bytes.push(0x01);
        push_u32(bytes, 0);
        bytes.push(0x01);
        push_u32(bytes, 1);
        bytes.push(0x16);
        bytes.push(0x18);
        bytes.push(0x01);
        push_u32(bytes, 1);
        bytes.push(0x17);
        bytes.push(0x41);
    }

    fn push_empty_events(bytes: &mut Vec<u8>) {
        bytes.push(SECTION_EVENTS);
        push_u32(bytes, 0);
    }

    fn push_load_global(bytes: &mut Vec<u8>, name: &[u8]) {
        bytes.push(0x02);
        push_u32(bytes, name.len() as u32);
        bytes.extend_from_slice(name);
    }

    fn push_store_global(bytes: &mut Vec<u8>, name: &[u8]) {
        bytes.push(0x03);
        push_u32(bytes, name.len() as u32);
        bytes.extend_from_slice(name);
    }

    fn push_store_local(bytes: &mut Vec<u8>, name: &[u8]) {
        bytes.push(0x05);
        push_u32(bytes, name.len() as u32);
        bytes.extend_from_slice(name);
    }

    fn push_store_existing(bytes: &mut Vec<u8>, name: &[u8]) {
        bytes.push(0x08);
        push_u32(bytes, name.len() as u32);
        bytes.extend_from_slice(name);
    }

    fn push_call_named(bytes: &mut Vec<u8>, name: &[u8], arg_count: u32) {
        bytes.push(0x43);
        push_u32(bytes, name.len() as u32);
        bytes.extend_from_slice(name);
        push_u32(bytes, arg_count);
    }

    fn push_backend_call(bytes: &mut Vec<u8>, backend: &[u8], method: &[u8], arg_count: u32) {
        bytes.push(0x60);
        push_u32(bytes, backend.len() as u32);
        bytes.extend_from_slice(backend);
        push_u32(bytes, method.len() as u32);
        bytes.extend_from_slice(method);
        push_u32(bytes, arg_count);
    }

    fn loop_branch_program() -> Vec<u8> {
        let mut bytes = Vec::new();
        push_header(&mut bytes, 28);
        push_constants(
            &mut bytes,
            &[
                ConstantRef::Int(1),
                ConstantRef::Int(0),
                ConstantRef::Int(5),
                ConstantRef::Int(15),
            ],
        );
        push_empty_functions(&mut bytes);
        push_empty_events(&mut bytes);
        bytes.push(SECTION_MAIN);
        push_u32(&mut bytes, 28);

        bytes.push(0x01);
        push_u32(&mut bytes, 0);
        push_store_global(&mut bytes, b"i");

        bytes.push(0x01);
        push_u32(&mut bytes, 1);
        push_store_global(&mut bytes, b"total");

        let loop_start = 4u32;
        push_load_global(&mut bytes, b"i");
        bytes.push(0x01);
        push_u32(&mut bytes, 2);
        bytes.push(0x24);
        bytes.push(0x31);
        push_u32(&mut bytes, 19);
        bytes.push(0x06);

        push_load_global(&mut bytes, b"total");
        push_load_global(&mut bytes, b"i");
        bytes.push(0x10);
        push_store_existing(&mut bytes, b"total");

        push_load_global(&mut bytes, b"i");
        bytes.push(0x01);
        push_u32(&mut bytes, 0);
        bytes.push(0x10);
        push_store_existing(&mut bytes, b"i");

        bytes.push(0x07);
        bytes.push(0x30);
        push_u32(&mut bytes, loop_start);

        push_load_global(&mut bytes, b"total");
        bytes.push(0x01);
        push_u32(&mut bytes, 3);
        bytes.push(0x20);
        bytes.push(0x31);
        push_u32(&mut bytes, 27);
        bytes.push(0x06);
        push_load_global(&mut bytes, b"total");
        bytes.push(0x50);
        bytes.push(0x07);
        bytes.push(0xFF);
        bytes.push(SECTION_END);

        bytes
    }

    fn recursive_factorial_program() -> Vec<u8> {
        let mut bytes = Vec::new();
        push_header(&mut bytes, 25);
        push_constants(&mut bytes, &[ConstantRef::Int(1), ConstantRef::Int(5)]);
        bytes.push(SECTION_FUNCTIONS);
        push_u32(&mut bytes, 1);
        push_function_fact(&mut bytes);
        push_empty_events(&mut bytes);
        bytes.push(SECTION_MAIN);
        push_u32(&mut bytes, 6);
        bytes.push(0x01);
        push_u32(&mut bytes, 1);
        push_call_named(&mut bytes, b"fact", 1);
        push_store_global(&mut bytes, b"result");
        push_load_global(&mut bytes, b"result");
        bytes.push(0x50);
        bytes.push(0xFF);
        bytes.push(SECTION_END);
        bytes
    }

    fn push_function_fact(bytes: &mut Vec<u8>) {
        push_u32(bytes, 4);
        bytes.extend_from_slice(b"fact");
        push_u32(bytes, 1);
        push_u32(bytes, 19);

        bytes.push(0x09);
        push_u32(bytes, 0);
        bytes.push(0x01);
        push_u32(bytes, 0);
        bytes.push(0x24);
        bytes.push(0x31);
        push_u32(bytes, 8);
        bytes.push(0x06);
        bytes.push(0x01);
        push_u32(bytes, 0);
        bytes.push(0x41);
        bytes.push(0x07);

        bytes.push(0x09);
        push_u32(bytes, 0);
        bytes.push(0x01);
        push_u32(bytes, 0);
        bytes.push(0x11);
        push_call_named(bytes, b"fact", 1);
        push_store_local(bytes, b"previous");

        bytes.push(0x09);
        push_u32(bytes, 0);
        push_load_global(bytes, b"previous");
        bytes.push(0x12);
        push_store_local(bytes, b"current");
        push_load_global(bytes, b"current");
        bytes.push(0x41);
    }

    #[test]
    fn run_recursive_factorial_with_function_locals() {
        let bytes = recursive_factorial_program();
        let result = run_mbc1_main(&bytes, 256).unwrap();

        assert_eq!(result.prints, vec![KernelValue::Int(120)]);
        assert_eq!(result.last_value, KernelValue::Int(120));
        assert_eq!(result.max_call_depth, 5);
        assert!(result.halted);
    }

    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}
