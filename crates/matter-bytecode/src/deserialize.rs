//! Deserialization for Matter Bytecode (MBC1)

use crate::{Bytecode, BytecodeLimits, Constant, EventHandler, Function, Instruction};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read, Result};

/// Decode version from u16
fn decode_version(version: u16) -> (u8, u8) {
    let major = (version >> 8) as u8;
    let minor = (version & 0xFF) as u8;
    (major, minor)
}

/// Section type tags
const SECTION_CONSTANTS: u8 = 0x01;
const SECTION_FUNCTIONS: u8 = 0x02;
const SECTION_EVENTS: u8 = 0x03;
const SECTION_MAIN: u8 = 0x04;
const SECTION_END: u8 = 0xFF;

fn invalid(msg: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidData, msg.into())
}

fn read_bounded_count(reader: &mut impl Read, limit: usize, label: &str) -> Result<usize> {
    let mut count_bytes = [0u8; 4];
    reader.read_exact(&mut count_bytes)?;
    let count = u32::from_le_bytes(count_bytes) as usize;
    if count > limit {
        return Err(invalid(format!(
            "{} count {} exceeds limit {}",
            label, count, limit
        )));
    }
    Ok(count)
}

impl Bytecode {
    /// Deserialize bytecode from reader (binary decode only).
    /// Prefer [`Bytecode::load_from_file`] / [`Bytecode::load_from_bytes`] which also validate.
    pub fn deserialize<R: Read>(reader: &mut R) -> Result<Self> {
        let limits = BytecodeLimits::from_env();
        Self::deserialize_with_limits(reader, &limits)
    }

    /// Deserialize with explicit size bounds (still does not run full structural validate).
    pub fn deserialize_with_limits<R: Read>(
        reader: &mut R,
        limits: &BytecodeLimits,
    ) -> Result<Self> {
        // Read and verify magic
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"MBC1" {
            return Err(invalid("Invalid MBC1 magic number"));
        }

        // Read version
        let mut version_bytes = [0u8; 2];
        reader.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        let (major, minor) = decode_version(version);

        // Check version compatibility
        if major != 0 {
            return Err(invalid(format!(
                "Unsupported MBC version: {}.{}",
                major, minor
            )));
        }

        // Read flags (reserved)
        let mut flags_bytes = [0u8; 2];
        reader.read_exact(&mut flags_bytes)?;
        let _flags = u16::from_le_bytes(flags_bytes);

        // Read total instruction count (advisory header field; still bounded)
        let mut count_bytes = [0u8; 4];
        reader.read_exact(&mut count_bytes)?;
        let total_instructions = u32::from_le_bytes(count_bytes) as usize;
        if total_instructions > limits.max_instructions_total {
            return Err(invalid(format!(
                "header total_instructions {} exceeds limit {}",
                total_instructions, limits.max_instructions_total
            )));
        }

        // Initialize bytecode
        let mut bytecode = Bytecode {
            magic,
            constants: Vec::new(),
            functions: HashMap::new(),
            event_handlers: HashMap::new(),
            main_instructions: Vec::new(),
        };

        // Read sections
        loop {
            let mut section_tag = [0u8; 1];
            reader.read_exact(&mut section_tag)?;

            match section_tag[0] {
                SECTION_CONSTANTS => {
                    bytecode.constants = deserialize_constants_section(reader, limits)?;
                }
                SECTION_FUNCTIONS => {
                    bytecode.functions = deserialize_functions_section(reader, limits)?;
                }
                SECTION_EVENTS => {
                    bytecode.event_handlers = deserialize_events_section(reader, limits)?;
                }
                SECTION_MAIN => {
                    bytecode.main_instructions = deserialize_instructions(reader, limits)?;
                }
                SECTION_END => break,
                _ => {
                    return Err(invalid(format!(
                        "Unknown section tag: 0x{:02X}",
                        section_tag[0]
                    )));
                }
            }
        }

        Ok(bytecode)
    }
}

/// Deserialize constants section
fn deserialize_constants_section<R: Read>(
    reader: &mut R,
    limits: &BytecodeLimits,
) -> Result<Vec<Constant>> {
    let count = read_bounded_count(reader, limits.max_constants, "constants")?;

    let mut constants = Vec::with_capacity(count);
    for _ in 0..count {
        constants.push(deserialize_constant(reader, limits)?);
    }

    Ok(constants)
}

/// Deserialize a constant
fn deserialize_constant<R: Read>(reader: &mut R, limits: &BytecodeLimits) -> Result<Constant> {
    let mut type_tag = [0u8; 1];
    reader.read_exact(&mut type_tag)?;

    match type_tag[0] {
        0x01 => {
            // Int
            let mut bytes = [0u8; 8];
            reader.read_exact(&mut bytes)?;
            Ok(Constant::Int(i64::from_le_bytes(bytes)))
        }
        0x02 => {
            // Bool
            let mut byte = [0u8; 1];
            reader.read_exact(&mut byte)?;
            Ok(Constant::Bool(byte[0] != 0))
        }
        0x03 => {
            // String
            Ok(Constant::String(read_string(reader, limits)?))
        }
        0x04 => {
            // Unit
            Ok(Constant::Unit)
        }
        0x06 => {
            // Null
            Ok(Constant::Null)
        }
        0x05 => {
            // Float
            let mut bytes = [0u8; 8];
            reader.read_exact(&mut bytes)?;
            Ok(Constant::Float(f64::from_le_bytes(bytes)))
        }
        _ => Err(invalid(format!(
            "Unknown constant type: 0x{:02X}",
            type_tag[0]
        ))),
    }
}

/// Deserialize functions section
fn deserialize_functions_section<R: Read>(
    reader: &mut R,
    limits: &BytecodeLimits,
) -> Result<HashMap<String, Function>> {
    let count = read_bounded_count(reader, limits.max_functions, "functions")?;

    let mut functions = HashMap::new();
    for _ in 0..count {
        let (name, func) = deserialize_function(reader, limits)?;
        functions.insert(name, func);
    }

    Ok(functions)
}

/// Deserialize a function
fn deserialize_function<R: Read>(
    reader: &mut R,
    limits: &BytecodeLimits,
) -> Result<(String, Function)> {
    let name = read_string(reader, limits)?;

    // Param count
    let mut param_bytes = [0u8; 4];
    reader.read_exact(&mut param_bytes)?;
    let param_count = u32::from_le_bytes(param_bytes) as usize;

    // Instructions
    let instructions = deserialize_instructions(reader, limits)?;

    let func = Function {
        name: name.clone(),
        param_count,
        instructions,
    };

    Ok((name, func))
}

/// Deserialize event handlers section
fn deserialize_events_section<R: Read>(
    reader: &mut R,
    limits: &BytecodeLimits,
) -> Result<HashMap<String, EventHandler>> {
    let count = read_bounded_count(reader, limits.max_event_handlers, "event_handlers")?;

    let mut events = HashMap::new();
    for _ in 0..count {
        let (event, handler) = deserialize_event_handler(reader, limits)?;
        events.insert(event, handler);
    }

    Ok(events)
}

/// Deserialize an event handler
fn deserialize_event_handler<R: Read>(
    reader: &mut R,
    limits: &BytecodeLimits,
) -> Result<(String, EventHandler)> {
    let event = read_string(reader, limits)?;

    // Instructions
    let instructions = deserialize_instructions(reader, limits)?;

    let handler = EventHandler {
        event: event.clone(),
        instructions,
    };

    Ok((event, handler))
}

/// Deserialize instructions
fn deserialize_instructions<R: Read>(
    reader: &mut R,
    limits: &BytecodeLimits,
) -> Result<Vec<Instruction>> {
    let count = read_bounded_count(reader, limits.max_instructions_per_block, "instructions")?;

    let mut instructions = Vec::with_capacity(count);
    for _ in 0..count {
        instructions.push(deserialize_instruction(reader, limits)?);
    }

    Ok(instructions)
}

/// Deserialize a single instruction
fn deserialize_instruction<R: Read>(
    reader: &mut R,
    limits: &BytecodeLimits,
) -> Result<Instruction> {
    let mut opcode = [0u8; 1];
    reader.read_exact(&mut opcode)?;

    match opcode[0] {
        0x01 => {
            // LoadConst
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            Ok(Instruction::LoadConst(u32::from_le_bytes(bytes) as usize))
        }
        0x02 => {
            // LoadGlobal
            Ok(Instruction::LoadGlobal(read_string(reader, limits)?))
        }
        0x03 => {
            // StoreGlobal
            Ok(Instruction::StoreGlobal(read_string(reader, limits)?))
        }
        0x04 => {
            // LoadLocal
            Ok(Instruction::LoadLocal(read_string(reader, limits)?))
        }
        0x05 => {
            // StoreLocal
            Ok(Instruction::StoreLocal(read_string(reader, limits)?))
        }
        0x06 => Ok(Instruction::PushScope),
        0x07 => Ok(Instruction::PopScope),
        0x08 => {
            // StoreExisting
            Ok(Instruction::StoreExisting(read_string(reader, limits)?))
        }
        0x09 => {
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            Ok(Instruction::LoadParam(u32::from_le_bytes(bytes) as usize))
        }
        0x10 => Ok(Instruction::Add),
        0x11 => Ok(Instruction::Sub),
        0x12 => Ok(Instruction::Mul),
        0x13 => Ok(Instruction::Div),
        0x14 => Ok(Instruction::Mod),
        0x15 => Ok(Instruction::Neg),
        0x16 => Ok(Instruction::And),
        0x17 => Ok(Instruction::Or),
        0x18 => Ok(Instruction::Not),
        0x20 => Ok(Instruction::Eq),
        0x21 => Ok(Instruction::NotEq),
        0x22 => Ok(Instruction::Lt),
        0x23 => Ok(Instruction::Gt),
        0x24 => Ok(Instruction::LtEq),
        0x25 => Ok(Instruction::GtEq),
        0x30 => {
            // Jump
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            Ok(Instruction::Jump(u32::from_le_bytes(bytes) as usize))
        }
        0x31 => {
            // JumpIfFalse
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            Ok(Instruction::JumpIfFalse(u32::from_le_bytes(bytes) as usize))
        }
        0x40 => {
            // Call
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            Ok(Instruction::Call(u32::from_le_bytes(bytes) as usize))
        }
        0x41 => Ok(Instruction::Return),
        0x42 => Ok(Instruction::SpawnEvent(read_string(reader, limits)?)),
        0x43 => {
            let name = read_string(reader, limits)?;
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            Ok(Instruction::CallNamed {
                name,
                arg_count: u32::from_le_bytes(bytes) as usize,
            })
        }
        0x50 => Ok(Instruction::Print),
        0x60 => {
            // BackendCall
            let backend = read_string(reader, limits)?;
            let method = read_string(reader, limits)?;

            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            let arg_count = u32::from_le_bytes(bytes) as usize;

            Ok(Instruction::BackendCall {
                backend,
                method,
                arg_count,
            })
        }

        // Sprint 4: Data Model - Lists (0x80-0x8F range)
        0x80 => {
            // NewList
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            let n = u32::from_le_bytes(bytes) as usize;
            if n > limits.max_list_or_map_elems {
                return Err(invalid(format!(
                    "NewList size {} exceeds limit {}",
                    n, limits.max_list_or_map_elems
                )));
            }
            Ok(Instruction::NewList(n))
        }
        0x81 => Ok(Instruction::LoadIndex),
        0x82 => Ok(Instruction::StoreIndex),
        0x83 => Ok(Instruction::ListPush),
        0x84 => Ok(Instruction::ListPop),
        0x85 => Ok(Instruction::ListLen),
        0x86 => Ok(Instruction::StoreIndexVar(read_string(reader, limits)?)),
        0x87 => Ok(Instruction::ListPushVar(read_string(reader, limits)?)),
        0x88 => Ok(Instruction::ListPopVar(read_string(reader, limits)?)),
        0x90 => {
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            let n = u32::from_le_bytes(bytes) as usize;
            if n > limits.max_list_or_map_elems {
                return Err(invalid(format!(
                    "NewMap size {} exceeds limit {}",
                    n, limits.max_list_or_map_elems
                )));
            }
            Ok(Instruction::NewMap(n))
        }
        0x91 => Ok(Instruction::MapHas),
        0x92 => Ok(Instruction::MapKeys),
        0x93 => Ok(Instruction::MapValues),
        0xA0 => {
            let type_name = read_string(reader, limits)?;
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            let n = u32::from_le_bytes(bytes) as usize;
            if n > limits.max_list_or_map_elems {
                return Err(invalid(format!(
                    "NewStruct fields {} exceeds limit {}",
                    n, limits.max_list_or_map_elems
                )));
            }
            Ok(Instruction::NewStruct(type_name, n))
        }
        0xA1 => Ok(Instruction::LoadField(read_string(reader, limits)?)),
        0xA2 => {
            let target = read_string(reader, limits)?;
            let field = read_string(reader, limits)?;
            Ok(Instruction::StoreFieldVar { target, field })
        }

        0x70 => Ok(Instruction::Pop),
        0xB0 => {
            let func_name = read_string(reader, limits)?;
            let count = {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                u32::from_le_bytes(buf) as usize
            };
            if count > limits.max_captures {
                return Err(invalid(format!(
                    "MakeClosure captures {} exceeds limit {}",
                    count, limits.max_captures
                )));
            }
            let mut capture_names = Vec::with_capacity(count);
            for _ in 0..count {
                capture_names.push(read_string(reader, limits)?);
            }
            Ok(Instruction::MakeClosure {
                func_name,
                capture_names,
            })
        }
        0xFF => Ok(Instruction::Halt),
        _ => Err(invalid(format!("Unknown opcode: 0x{:02X}", opcode[0]))),
    }
}

/// Helper to read a length-prefixed UTF-8 string with size bound.
fn read_string<R: Read>(reader: &mut R, limits: &BytecodeLimits) -> Result<String> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    if len > limits.max_string_bytes {
        return Err(invalid(format!(
            "string length {} exceeds limit {}",
            len, limits.max_string_bytes
        )));
    }

    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;

    String::from_utf8(bytes).map_err(|e| invalid(format!("Invalid UTF-8: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_deserialize_empty_bytecode() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions.push(Instruction::Halt);

        let mut buffer = Vec::new();
        bytecode.serialize(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let deserialized = Bytecode::deserialize(&mut cursor).unwrap();

        assert_eq!(deserialized.magic, *b"MBC1");
        assert_eq!(deserialized.main_instructions.len(), 1);
    }

    #[test]
    fn test_round_trip() {
        let mut bytecode = Bytecode::new();
        bytecode.add_constant(Constant::Int(42));
        bytecode.add_constant(Constant::String("test".to_string()));
        bytecode.main_instructions.push(Instruction::LoadConst(0));
        bytecode.main_instructions.push(Instruction::Print);
        bytecode
            .main_instructions
            .push(Instruction::ListPushVar("items".to_string()));
        bytecode
            .main_instructions
            .push(Instruction::ListPopVar("items".to_string()));
        bytecode
            .main_instructions
            .push(Instruction::StoreIndexVar("items".to_string()));
        bytecode
            .main_instructions
            .push(Instruction::NewStruct("User".to_string(), 1));
        bytecode
            .main_instructions
            .push(Instruction::LoadField("name".to_string()));
        bytecode.main_instructions.push(Instruction::StoreFieldVar {
            target: "user".to_string(),
            field: "name".to_string(),
        });
        bytecode.main_instructions.push(Instruction::LoadParam(1));
        bytecode.main_instructions.push(Instruction::CallNamed {
            name: "compute".to_string(),
            arg_count: 2,
        });
        bytecode.main_instructions.push(Instruction::Halt);

        let mut buffer = Vec::new();
        bytecode.serialize(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let deserialized = Bytecode::deserialize(&mut cursor).unwrap();

        assert_eq!(deserialized.constants.len(), 2);
        assert_eq!(deserialized.main_instructions.len(), 11);
        assert!(matches!(
            &deserialized.main_instructions[2],
            Instruction::ListPushVar(name) if name == "items"
        ));
        assert!(matches!(
            &deserialized.main_instructions[5],
            Instruction::NewStruct(type_name, 1) if type_name == "User"
        ));
        assert!(matches!(
            &deserialized.main_instructions[8],
            Instruction::LoadParam(1)
        ));
        assert!(matches!(
            &deserialized.main_instructions[9],
            Instruction::CallNamed { name, arg_count: 2 } if name == "compute"
        ));
    }
}
