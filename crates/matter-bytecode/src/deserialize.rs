//! Deserialization for Matter Bytecode (MBC1)

use crate::{Bytecode, Constant, EventHandler, Function, Instruction};
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

impl Bytecode {
    /// Deserialize bytecode from reader
    pub fn deserialize<R: Read>(reader: &mut R) -> Result<Self> {
        // Read and verify magic
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"MBC1" {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Invalid MBC1 magic number",
            ));
        }

        // Read version
        let mut version_bytes = [0u8; 2];
        reader.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        let (major, minor) = decode_version(version);

        // Check version compatibility
        if major != 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Unsupported MBC version: {}.{}", major, minor),
            ));
        }

        // Read flags (reserved)
        let mut flags_bytes = [0u8; 2];
        reader.read_exact(&mut flags_bytes)?;
        let _flags = u16::from_le_bytes(flags_bytes);

        // Read total instruction count
        let mut count_bytes = [0u8; 4];
        reader.read_exact(&mut count_bytes)?;
        let _total_instructions = u32::from_le_bytes(count_bytes);

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
                    bytecode.constants = deserialize_constants_section(reader)?;
                }
                SECTION_FUNCTIONS => {
                    bytecode.functions = deserialize_functions_section(reader)?;
                }
                SECTION_EVENTS => {
                    bytecode.event_handlers = deserialize_events_section(reader)?;
                }
                SECTION_MAIN => {
                    bytecode.main_instructions = deserialize_instructions(reader)?;
                }
                SECTION_END => break,
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("Unknown section tag: 0x{:02X}", section_tag[0]),
                    ));
                }
            }
        }

        Ok(bytecode)
    }
}

/// Deserialize constants section
fn deserialize_constants_section<R: Read>(reader: &mut R) -> Result<Vec<Constant>> {
    let mut count_bytes = [0u8; 4];
    reader.read_exact(&mut count_bytes)?;
    let count = u32::from_le_bytes(count_bytes) as usize;

    let mut constants = Vec::with_capacity(count);
    for _ in 0..count {
        constants.push(deserialize_constant(reader)?);
    }

    Ok(constants)
}

/// Deserialize a constant
fn deserialize_constant<R: Read>(reader: &mut R) -> Result<Constant> {
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
            let mut len_bytes = [0u8; 4];
            reader.read_exact(&mut len_bytes)?;
            let len = u32::from_le_bytes(len_bytes) as usize;

            let mut bytes = vec![0u8; len];
            reader.read_exact(&mut bytes)?;

            let s = String::from_utf8(bytes)
                .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e)))?;

            Ok(Constant::String(s))
        }
        0x04 => {
            // Unit
            Ok(Constant::Unit)
        }
        0x05 => {
            // Float
            let mut bytes = [0u8; 8];
            reader.read_exact(&mut bytes)?;
            Ok(Constant::Float(f64::from_le_bytes(bytes)))
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Unknown constant type: 0x{:02X}", type_tag[0]),
        )),
    }
}

/// Deserialize functions section
fn deserialize_functions_section<R: Read>(reader: &mut R) -> Result<HashMap<String, Function>> {
    let mut count_bytes = [0u8; 4];
    reader.read_exact(&mut count_bytes)?;
    let count = u32::from_le_bytes(count_bytes) as usize;

    let mut functions = HashMap::new();
    for _ in 0..count {
        let (name, func) = deserialize_function(reader)?;
        functions.insert(name, func);
    }

    Ok(functions)
}

/// Deserialize a function
fn deserialize_function<R: Read>(reader: &mut R) -> Result<(String, Function)> {
    // Name
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;

    let mut name_bytes = vec![0u8; len];
    reader.read_exact(&mut name_bytes)?;
    let name = String::from_utf8(name_bytes)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e)))?;

    // Param count
    let mut param_bytes = [0u8; 4];
    reader.read_exact(&mut param_bytes)?;
    let param_count = u32::from_le_bytes(param_bytes) as usize;

    // Instructions
    let instructions = deserialize_instructions(reader)?;

    let func = Function {
        name: name.clone(),
        param_count,
        instructions,
    };

    Ok((name, func))
}

/// Deserialize event handlers section
fn deserialize_events_section<R: Read>(reader: &mut R) -> Result<HashMap<String, EventHandler>> {
    let mut count_bytes = [0u8; 4];
    reader.read_exact(&mut count_bytes)?;
    let count = u32::from_le_bytes(count_bytes) as usize;

    let mut events = HashMap::new();
    for _ in 0..count {
        let (event, handler) = deserialize_event_handler(reader)?;
        events.insert(event, handler);
    }

    Ok(events)
}

/// Deserialize an event handler
fn deserialize_event_handler<R: Read>(reader: &mut R) -> Result<(String, EventHandler)> {
    // Event name
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;

    let mut event_bytes = vec![0u8; len];
    reader.read_exact(&mut event_bytes)?;
    let event = String::from_utf8(event_bytes)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e)))?;

    // Instructions
    let instructions = deserialize_instructions(reader)?;

    let handler = EventHandler {
        event: event.clone(),
        instructions,
    };

    Ok((event, handler))
}

/// Deserialize instructions
fn deserialize_instructions<R: Read>(reader: &mut R) -> Result<Vec<Instruction>> {
    let mut count_bytes = [0u8; 4];
    reader.read_exact(&mut count_bytes)?;
    let count = u32::from_le_bytes(count_bytes) as usize;

    let mut instructions = Vec::with_capacity(count);
    for _ in 0..count {
        instructions.push(deserialize_instruction(reader)?);
    }

    Ok(instructions)
}

/// Deserialize a single instruction
fn deserialize_instruction<R: Read>(reader: &mut R) -> Result<Instruction> {
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
            Ok(Instruction::LoadGlobal(read_string(reader)?))
        }
        0x03 => {
            // StoreGlobal
            Ok(Instruction::StoreGlobal(read_string(reader)?))
        }
        0x04 => {
            // LoadLocal
            Ok(Instruction::LoadLocal(read_string(reader)?))
        }
        0x05 => {
            // StoreLocal
            Ok(Instruction::StoreLocal(read_string(reader)?))
        }
        0x06 => Ok(Instruction::PushScope),
        0x07 => Ok(Instruction::PopScope),
        0x08 => {
            // StoreExisting
            Ok(Instruction::StoreExisting(read_string(reader)?))
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
        0x42 => Ok(Instruction::SpawnEvent(read_string(reader)?)),
        0x43 => {
            let name = read_string(reader)?;
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
            let backend = read_string(reader)?;
            let method = read_string(reader)?;

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
            Ok(Instruction::NewList(u32::from_le_bytes(bytes) as usize))
        }
        0x81 => Ok(Instruction::LoadIndex),
        0x82 => Ok(Instruction::StoreIndex),
        0x83 => Ok(Instruction::ListPush),
        0x84 => Ok(Instruction::ListPop),
        0x85 => Ok(Instruction::ListLen),
        0x86 => Ok(Instruction::StoreIndexVar(read_string(reader)?)),
        0x87 => Ok(Instruction::ListPushVar(read_string(reader)?)),
        0x88 => Ok(Instruction::ListPopVar(read_string(reader)?)),
        0x90 => {
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            Ok(Instruction::NewMap(u32::from_le_bytes(bytes) as usize))
        }
        0x91 => Ok(Instruction::MapHas),
        0x92 => Ok(Instruction::MapKeys),
        0x93 => Ok(Instruction::MapValues),
        0xA0 => {
            let type_name = read_string(reader)?;
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            Ok(Instruction::NewStruct(
                type_name,
                u32::from_le_bytes(bytes) as usize,
            ))
        }
        0xA1 => Ok(Instruction::LoadField(read_string(reader)?)),
        0xA2 => {
            let target = read_string(reader)?;
            let field = read_string(reader)?;
            Ok(Instruction::StoreFieldVar { target, field })
        }

        0x70 => Ok(Instruction::Pop),
        0xFF => Ok(Instruction::Halt),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Unknown opcode: 0x{:02X}", opcode[0]),
        )),
    }
}

/// Helper to read a string
fn read_string<R: Read>(reader: &mut R) -> Result<String> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes)?;
    let len = u32::from_le_bytes(len_bytes) as usize;

    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;

    String::from_utf8(bytes)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Invalid UTF-8: {}", e)))
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
