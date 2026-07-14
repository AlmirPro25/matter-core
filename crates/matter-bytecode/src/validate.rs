//! Structural validation for loaded MBC1 bytecode (Phase 2 hardening).
//!
//! Validation runs after binary decode and before any VM execution.
//! Wire format is unchanged; this only rejects malformed / oversized programs.

use crate::{Bytecode, Constant, Instruction};
use std::io::{Error, ErrorKind, Result};

/// Configurable structural limits for MBC1 load/validation.
#[derive(Debug, Clone)]
pub struct BytecodeLimits {
    pub max_file_bytes: usize,
    pub max_constants: usize,
    pub max_functions: usize,
    pub max_event_handlers: usize,
    pub max_instructions_per_block: usize,
    pub max_instructions_total: usize,
    pub max_string_bytes: usize,
    pub max_captures: usize,
    pub max_list_or_map_elems: usize,
}

impl Default for BytecodeLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 32 * 1024 * 1024, // 32 MiB
            max_constants: 100_000,
            max_functions: 50_000,
            max_event_handlers: 50_000,
            max_instructions_per_block: 500_000,
            max_instructions_total: 2_000_000,
            max_string_bytes: 1_048_576, // 1 MiB
            max_captures: 1_024,
            max_list_or_map_elems: 1_000_000,
        }
    }
}

impl BytecodeLimits {
    /// Read optional overrides from environment (positive integers only).
    ///
    /// - `MATTER_MBC_MAX_FILE_BYTES`
    /// - `MATTER_MBC_MAX_CONSTANTS`
    /// - `MATTER_MBC_MAX_FUNCTIONS`
    /// - `MATTER_MBC_MAX_EVENT_HANDLERS`
    /// - `MATTER_MBC_MAX_INSTRUCTIONS_PER_BLOCK`
    /// - `MATTER_MBC_MAX_INSTRUCTIONS_TOTAL`
    /// - `MATTER_MBC_MAX_STRING_BYTES`
    pub fn from_env() -> Self {
        let mut limits = Self::default();
        apply_env_usize("MATTER_MBC_MAX_FILE_BYTES", &mut limits.max_file_bytes);
        apply_env_usize("MATTER_MBC_MAX_CONSTANTS", &mut limits.max_constants);
        apply_env_usize("MATTER_MBC_MAX_FUNCTIONS", &mut limits.max_functions);
        apply_env_usize(
            "MATTER_MBC_MAX_EVENT_HANDLERS",
            &mut limits.max_event_handlers,
        );
        apply_env_usize(
            "MATTER_MBC_MAX_INSTRUCTIONS_PER_BLOCK",
            &mut limits.max_instructions_per_block,
        );
        apply_env_usize(
            "MATTER_MBC_MAX_INSTRUCTIONS_TOTAL",
            &mut limits.max_instructions_total,
        );
        apply_env_usize("MATTER_MBC_MAX_STRING_BYTES", &mut limits.max_string_bytes);
        limits
    }
}

fn apply_env_usize(key: &str, target: &mut usize) {
    if let Ok(v) = std::env::var(key) {
        if let Ok(n) = v.parse::<usize>() {
            if n > 0 {
                *target = n;
            }
        }
    }
}

fn invalid(msg: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidData, msg.into())
}

impl Bytecode {
    /// Validate structural integrity of this bytecode against default limits.
    pub fn validate(&self) -> Result<()> {
        self.validate_with_limits(&BytecodeLimits::from_env())
    }

    /// Validate structural integrity against explicit limits.
    pub fn validate_with_limits(&self, limits: &BytecodeLimits) -> Result<()> {
        if &self.magic != b"MBC1" {
            return Err(invalid(format!(
                "invalid MBC1 magic: {:?}",
                self.magic
            )));
        }

        if self.constants.len() > limits.max_constants {
            return Err(invalid(format!(
                "too many constants: {} (limit {})",
                self.constants.len(),
                limits.max_constants
            )));
        }
        if self.functions.len() > limits.max_functions {
            return Err(invalid(format!(
                "too many functions: {} (limit {})",
                self.functions.len(),
                limits.max_functions
            )));
        }
        if self.event_handlers.len() > limits.max_event_handlers {
            return Err(invalid(format!(
                "too many event handlers: {} (limit {})",
                self.event_handlers.len(),
                limits.max_event_handlers
            )));
        }

        for (i, c) in self.constants.iter().enumerate() {
            if let Constant::String(s) = c {
                if s.len() > limits.max_string_bytes {
                    return Err(invalid(format!(
                        "constant[{}] string too large: {} bytes (limit {})",
                        i,
                        s.len(),
                        limits.max_string_bytes
                    )));
                }
            }
        }

        let mut total_instr = 0usize;

        total_instr = total_instr
            .checked_add(self.main_instructions.len())
            .ok_or_else(|| invalid("instruction count overflow"))?;
        validate_instruction_block(
            "main",
            &self.main_instructions,
            self.constants.len(),
            &self.functions,
            limits,
        )?;

        for (name, func) in &self.functions {
            if name.len() > limits.max_string_bytes {
                return Err(invalid(format!(
                    "function name too long: {} bytes",
                    name.len()
                )));
            }
            total_instr = total_instr
                .checked_add(func.instructions.len())
                .ok_or_else(|| invalid("instruction count overflow"))?;
            validate_instruction_block(
                &format!("function '{}'", name),
                &func.instructions,
                self.constants.len(),
                &self.functions,
                limits,
            )?;
        }

        for (event, handler) in &self.event_handlers {
            if event.len() > limits.max_string_bytes {
                return Err(invalid(format!(
                    "event name too long: {} bytes",
                    event.len()
                )));
            }
            total_instr = total_instr
                .checked_add(handler.instructions.len())
                .ok_or_else(|| invalid("instruction count overflow"))?;
            validate_instruction_block(
                &format!("event '{}'", event),
                &handler.instructions,
                self.constants.len(),
                &self.functions,
                limits,
            )?;
        }

        if total_instr > limits.max_instructions_total {
            return Err(invalid(format!(
                "too many instructions total: {} (limit {})",
                total_instr, limits.max_instructions_total
            )));
        }

        Ok(())
    }
}

fn validate_instruction_block(
    block_name: &str,
    instructions: &[Instruction],
    constant_count: usize,
    functions: &std::collections::HashMap<String, crate::Function>,
    limits: &BytecodeLimits,
) -> Result<()> {
    if instructions.len() > limits.max_instructions_per_block {
        return Err(invalid(format!(
            "{}: too many instructions: {} (limit {})",
            block_name,
            instructions.len(),
            limits.max_instructions_per_block
        )));
    }

    let len = instructions.len();
    for (ip, instr) in instructions.iter().enumerate() {
        match instr {
            Instruction::LoadConst(id) => {
                if *id >= constant_count {
                    return Err(invalid(format!(
                        "{}[{}]: LoadConst index {} out of range (constants={})",
                        block_name, ip, id, constant_count
                    )));
                }
            }
            Instruction::Jump(target) | Instruction::JumpIfFalse(target) => {
                // Jump targets are absolute instruction indices; equal to `len` means "past end".
                if *target > len {
                    return Err(invalid(format!(
                        "{}[{}]: jump target {} out of range (block_len={})",
                        block_name, ip, target, len
                    )));
                }
            }
            Instruction::MakeClosure {
                func_name,
                capture_names,
            } => {
                if !functions.contains_key(func_name) {
                    return Err(invalid(format!(
                        "{}[{}]: MakeClosure references unknown function '{}'",
                        block_name, ip, func_name
                    )));
                }
                if capture_names.len() > limits.max_captures {
                    return Err(invalid(format!(
                        "{}[{}]: too many captures: {} (limit {})",
                        block_name,
                        ip,
                        capture_names.len(),
                        limits.max_captures
                    )));
                }
                for cap in capture_names {
                    if cap.len() > limits.max_string_bytes {
                        return Err(invalid(format!(
                            "{}[{}]: capture name too long",
                            block_name, ip
                        )));
                    }
                }
            }
            Instruction::CallNamed { name, .. } => {
                if name.len() > limits.max_string_bytes {
                    return Err(invalid(format!(
                        "{}[{}]: CallNamed name too long",
                        block_name, ip
                    )));
                }
            }
            Instruction::NewList(n) | Instruction::NewMap(n) | Instruction::NewStruct(_, n) => {
                if *n > limits.max_list_or_map_elems {
                    return Err(invalid(format!(
                        "{}[{}]: collection size {} exceeds limit {}",
                        block_name, ip, n, limits.max_list_or_map_elems
                    )));
                }
            }
            Instruction::LoadGlobal(s)
            | Instruction::StoreGlobal(s)
            | Instruction::LoadLocal(s)
            | Instruction::StoreLocal(s)
            | Instruction::StoreExisting(s)
            | Instruction::SpawnEvent(s)
            | Instruction::StoreIndexVar(s)
            | Instruction::ListPushVar(s)
            | Instruction::ListPopVar(s)
            | Instruction::LoadField(s) => {
                if s.len() > limits.max_string_bytes {
                    return Err(invalid(format!(
                        "{}[{}]: identifier/string operand too long",
                        block_name, ip
                    )));
                }
            }
            Instruction::BackendCall {
                backend,
                method,
                arg_count: _,
            } => {
                if backend.len() > limits.max_string_bytes || method.len() > limits.max_string_bytes
                {
                    return Err(invalid(format!(
                        "{}[{}]: BackendCall name too long",
                        block_name, ip
                    )));
                }
            }
            Instruction::StoreFieldVar { target, field } => {
                if target.len() > limits.max_string_bytes || field.len() > limits.max_string_bytes {
                    return Err(invalid(format!(
                        "{}[{}]: StoreFieldVar name too long",
                        block_name, ip
                    )));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Constant, Function, Instruction};
    use std::io::Cursor;

    #[test]
    fn valid_empty_program_passes() {
        let mut bc = Bytecode::new();
        bc.main_instructions.push(Instruction::Halt);
        bc.validate().unwrap();
    }

    #[test]
    fn bad_magic_rejected() {
        let mut bc = Bytecode::new();
        bc.magic = *b"XXXX";
        bc.main_instructions.push(Instruction::Halt);
        assert!(bc.validate().is_err());
    }

    #[test]
    fn load_const_oob_rejected() {
        let mut bc = Bytecode::new();
        bc.main_instructions
            .push(Instruction::LoadConst(5));
        bc.main_instructions.push(Instruction::Halt);
        assert!(bc.validate().is_err());
    }

    #[test]
    fn jump_oob_rejected() {
        let mut bc = Bytecode::new();
        bc.main_instructions.push(Instruction::Jump(99));
        assert!(bc.validate().is_err());
    }

    #[test]
    fn random_bytes_fail_deserialize() {
        let junk = b"NOT_MBC1_GARBAGE_DATA!!!!!!!!!!!!";
        let mut cursor = Cursor::new(junk.as_slice());
        assert!(Bytecode::deserialize(&mut cursor).is_err());
    }

    #[test]
    fn truncated_header_fails() {
        let mut cursor = Cursor::new(b"MBC1\x00".as_slice());
        assert!(Bytecode::deserialize(&mut cursor).is_err());
    }

    #[test]
    fn roundtrip_still_validates() {
        let mut bc = Bytecode::new();
        bc.add_constant(Constant::Int(42));
        bc.add_constant(Constant::String("hi".into()));
        bc.functions.insert(
            "f".into(),
            Function {
                name: "f".into(),
                param_count: 0,
                instructions: vec![Instruction::LoadConst(0), Instruction::Return],
            },
        );
        bc.main_instructions
            .push(Instruction::CallNamed {
                name: "f".into(),
                arg_count: 0,
            });
        bc.main_instructions.push(Instruction::Halt);

        let mut buf = Vec::new();
        bc.serialize(&mut buf).unwrap();
        let mut cursor = Cursor::new(buf);
        let loaded = Bytecode::deserialize(&mut cursor).unwrap();
        loaded.validate().unwrap();
    }

    #[test]
    fn make_closure_unknown_fn_rejected() {
        let mut bc = Bytecode::new();
        bc.main_instructions.push(Instruction::MakeClosure {
            func_name: "missing".into(),
            capture_names: vec![],
        });
        assert!(bc.validate().is_err());
    }

    #[test]
    fn limits_reject_too_many_constants() {
        let mut bc = Bytecode::new();
        for i in 0..10 {
            bc.add_constant(Constant::Int(i));
        }
        bc.main_instructions.push(Instruction::Halt);
        let limits = BytecodeLimits {
            max_constants: 5,
            ..BytecodeLimits::default()
        };
        assert!(bc.validate_with_limits(&limits).is_err());
    }
}
