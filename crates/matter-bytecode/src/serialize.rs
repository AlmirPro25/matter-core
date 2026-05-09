/// Serialization for Matter Bytecode (MBC1)
/// 
/// Format:
/// - Magic: [u8; 4] = "MBC1"
/// - Version: u16 (major.minor encoded as major << 8 | minor)
/// - Flags: u16 (reserved for future use)
/// - Sections: Constants, Functions, Events, Main

use std::io::{Write, Result};
use crate::{Bytecode, Constant, Instruction, Function, EventHandler};

/// MBC1 Format Version
pub const MBC_VERSION_MAJOR: u8 = 0;
pub const MBC_VERSION_MINOR: u8 = 1;

/// Encode version as u16
fn encode_version(major: u8, minor: u8) -> u16 {
    ((major as u16) << 8) | (minor as u16)
}

/// Section type tags
const SECTION_CONSTANTS: u8 = 0x01;
const SECTION_FUNCTIONS: u8 = 0x02;
const SECTION_EVENTS: u8 = 0x03;
const SECTION_MAIN: u8 = 0x04;
const SECTION_END: u8 = 0xFF;

impl Bytecode {
    /// Serialize bytecode to writer
    pub fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Magic number
        writer.write_all(&self.magic)?;
        
        // Version (u16)
        let version = encode_version(MBC_VERSION_MAJOR, MBC_VERSION_MINOR);
        writer.write_all(&version.to_le_bytes())?;
        
        // Flags (u16) - reserved for future use
        let flags: u16 = 0;
        writer.write_all(&flags.to_le_bytes())?;
        
        // Total instruction count (u32) - for quick validation
        let total_instructions = self.count_total_instructions();
        writer.write_all(&(total_instructions as u32).to_le_bytes())?;
        
        // Sections
        self.serialize_constants_section(writer)?;
        self.serialize_functions_section(writer)?;
        self.serialize_events_section(writer)?;
        self.serialize_main_section(writer)?;
        
        // End marker
        writer.write_all(&[SECTION_END])?;
        
        Ok(())
    }
    
    /// Count total instructions for validation
    fn count_total_instructions(&self) -> usize {
        let mut count = self.main_instructions.len();
        
        for func in self.functions.values() {
            count += func.instructions.len();
        }
        
        for event in self.event_handlers.values() {
            count += event.instructions.len();
        }
        
        count
    }
    
    /// Serialize constants section
    fn serialize_constants_section<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&[SECTION_CONSTANTS])?;
        
        // Count
        writer.write_all(&(self.constants.len() as u32).to_le_bytes())?;
        
        // Each constant
        for constant in &self.constants {
            serialize_constant(constant, writer)?;
        }
        
        Ok(())
    }
    
    /// Serialize functions section
    fn serialize_functions_section<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&[SECTION_FUNCTIONS])?;
        
        // Count
        writer.write_all(&(self.functions.len() as u32).to_le_bytes())?;
        
        // Each function
        for (name, func) in &self.functions {
            serialize_function(name, func, writer)?;
        }
        
        Ok(())
    }
    
    /// Serialize event handlers section
    fn serialize_events_section<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&[SECTION_EVENTS])?;
        
        // Count
        writer.write_all(&(self.event_handlers.len() as u32).to_le_bytes())?;
        
        // Each event handler
        for (event, handler) in &self.event_handlers {
            serialize_event_handler(event, handler, writer)?;
        }
        
        Ok(())
    }
    
    /// Serialize main instructions section
    fn serialize_main_section<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&[SECTION_MAIN])?;
        
        serialize_instructions(&self.main_instructions, writer)?;
        
        Ok(())
    }
}

/// Serialize a constant
fn serialize_constant<W: Write>(constant: &Constant, writer: &mut W) -> Result<()> {
    match constant {
        Constant::Int(n) => {
            writer.write_all(&[0x01])?; // type tag
            writer.write_all(&n.to_le_bytes())?;
        }
        Constant::Bool(b) => {
            writer.write_all(&[0x02])?;
            writer.write_all(&[*b as u8])?;
        }
        Constant::String(s) => {
            writer.write_all(&[0x03])?;
            let bytes = s.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
        Constant::Unit => {
            writer.write_all(&[0x04])?;
        }
    }
    Ok(())
}

/// Serialize a function
fn serialize_function<W: Write>(name: &str, func: &Function, writer: &mut W) -> Result<()> {
    // Name
    let name_bytes = name.as_bytes();
    writer.write_all(&(name_bytes.len() as u32).to_le_bytes())?;
    writer.write_all(name_bytes)?;
    
    // Param count
    writer.write_all(&(func.param_count as u32).to_le_bytes())?;
    
    // Instructions
    serialize_instructions(&func.instructions, writer)?;
    
    Ok(())
}

/// Serialize an event handler
fn serialize_event_handler<W: Write>(event: &str, handler: &EventHandler, writer: &mut W) -> Result<()> {
    // Event name
    let event_bytes = event.as_bytes();
    writer.write_all(&(event_bytes.len() as u32).to_le_bytes())?;
    writer.write_all(event_bytes)?;
    
    // Instructions
    serialize_instructions(&handler.instructions, writer)?;
    
    Ok(())
}

/// Serialize instructions
fn serialize_instructions<W: Write>(instructions: &[Instruction], writer: &mut W) -> Result<()> {
    // Count
    writer.write_all(&(instructions.len() as u32).to_le_bytes())?;
    
    // Each instruction
    for instr in instructions {
        serialize_instruction(instr, writer)?;
    }
    
    Ok(())
}

/// Serialize a single instruction
fn serialize_instruction<W: Write>(instr: &Instruction, writer: &mut W) -> Result<()> {
    match instr {
        Instruction::LoadConst(id) => {
            writer.write_all(&[0x01])?;
            writer.write_all(&(*id as u32).to_le_bytes())?;
        }
        Instruction::LoadGlobal(name) => {
            writer.write_all(&[0x02])?;
            let bytes = name.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
        Instruction::StoreGlobal(name) => {
            writer.write_all(&[0x03])?;
            let bytes = name.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
        Instruction::LoadLocal(name) => {
            writer.write_all(&[0x04])?;
            let bytes = name.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
        Instruction::StoreLocal(name) => {
            writer.write_all(&[0x05])?;
            let bytes = name.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
        Instruction::StoreExisting(name) => {
            writer.write_all(&[0x08])?;
            let bytes = name.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
        Instruction::PushScope => writer.write_all(&[0x06])?,
        Instruction::PopScope => writer.write_all(&[0x07])?,
        Instruction::Add => writer.write_all(&[0x10])?,
        Instruction::Sub => writer.write_all(&[0x11])?,
        Instruction::Mul => writer.write_all(&[0x12])?,
        Instruction::Div => writer.write_all(&[0x13])?,
        Instruction::Eq => writer.write_all(&[0x20])?,
        Instruction::NotEq => writer.write_all(&[0x21])?,
        Instruction::Lt => writer.write_all(&[0x22])?,
        Instruction::Gt => writer.write_all(&[0x23])?,
        Instruction::LtEq => writer.write_all(&[0x24])?,
        Instruction::GtEq => writer.write_all(&[0x25])?,
        Instruction::Jump(addr) => {
            writer.write_all(&[0x30])?;
            writer.write_all(&(*addr as u32).to_le_bytes())?;
        }
        Instruction::JumpIfFalse(addr) => {
            writer.write_all(&[0x31])?;
            writer.write_all(&(*addr as u32).to_le_bytes())?;
        }
        Instruction::Call(arg_count) => {
            writer.write_all(&[0x40])?;
            writer.write_all(&(*arg_count as u32).to_le_bytes())?;
        }
        Instruction::Return => writer.write_all(&[0x41])?,
        Instruction::Print => writer.write_all(&[0x50])?,
        Instruction::BackendCall { backend, method, arg_count } => {
            writer.write_all(&[0x60])?;
            
            // Backend name
            let backend_bytes = backend.as_bytes();
            writer.write_all(&(backend_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(backend_bytes)?;
            
            // Method name
            let method_bytes = method.as_bytes();
            writer.write_all(&(method_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(method_bytes)?;
            
            // Arg count
            writer.write_all(&(*arg_count as u32).to_le_bytes())?;
        }
        
        // Sprint 4: Data Model - Lists (0x80-0x8F range)
        Instruction::NewList(size) => {
            writer.write_all(&[0x80])?;
            writer.write_all(&(*size as u32).to_le_bytes())?;
        }
        Instruction::LoadIndex => writer.write_all(&[0x81])?,
        Instruction::StoreIndex => writer.write_all(&[0x82])?,
        Instruction::ListPush => writer.write_all(&[0x83])?,
        Instruction::ListPop => writer.write_all(&[0x84])?,
        Instruction::ListLen => writer.write_all(&[0x85])?,
        Instruction::StoreIndexVar(name) => {
            writer.write_all(&[0x86])?;
            let bytes = name.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
        Instruction::ListPushVar(name) => {
            writer.write_all(&[0x87])?;
            let bytes = name.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
        Instruction::ListPopVar(name) => {
            writer.write_all(&[0x88])?;
            let bytes = name.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
        Instruction::NewMap(size) => {
            writer.write_all(&[0x90])?;
            writer.write_all(&(*size as u32).to_le_bytes())?;
        }
        Instruction::MapHas => writer.write_all(&[0x91])?,
        Instruction::MapKeys => writer.write_all(&[0x92])?,
        Instruction::MapValues => writer.write_all(&[0x93])?,
        Instruction::NewStruct(type_name, size) => {
            writer.write_all(&[0xA0])?;
            let bytes = type_name.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
            writer.write_all(&(*size as u32).to_le_bytes())?;
        }
        Instruction::LoadField(field) => {
            writer.write_all(&[0xA1])?;
            let bytes = field.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }
        Instruction::StoreFieldVar { target, field } => {
            writer.write_all(&[0xA2])?;
            let target_bytes = target.as_bytes();
            writer.write_all(&(target_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(target_bytes)?;
            let field_bytes = field.as_bytes();
            writer.write_all(&(field_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(field_bytes)?;
        }

        Instruction::Pop => writer.write_all(&[0x70])?,
        Instruction::Halt => writer.write_all(&[0xFF])?,
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_serialize_empty_bytecode() {
        let bytecode = Bytecode::new();
        let mut buffer = Vec::new();
        
        bytecode.serialize(&mut buffer).unwrap();
        
        // Check magic
        assert_eq!(&buffer[0..4], b"MBC1");
        
        // Check version
        let version = u16::from_le_bytes([buffer[4], buffer[5]]);
        assert_eq!(version, encode_version(0, 1));
    }
    
    #[test]
    fn test_serialize_with_constants() {
        let mut bytecode = Bytecode::new();
        bytecode.add_constant(Constant::Int(42));
        bytecode.add_constant(Constant::Bool(true));
        bytecode.add_constant(Constant::String("hello".to_string()));
        
        let mut buffer = Vec::new();
        bytecode.serialize(&mut buffer).unwrap();
        
        assert!(buffer.len() > 10);
    }
}
