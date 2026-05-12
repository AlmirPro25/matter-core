#![no_std]
//! Shared ABI between Matter Core and Sentinel OS.
//!
//! This crate is intentionally small and `no_std`: it can be compiled into the
//! Sentinel kernel while Matter keeps the full compiler, CLI, and desktop tools
//! in the host workspace.

pub const PVM1_MAGIC: [u8; 4] = *b"PVM1";
pub const PVM2_MAGIC: [u8; 4] = *b"PVM2";
pub const PVM2_FORMAT_VERSION: u16 = 2;
pub const PVM_MAX_NAME_BYTES: usize = 64;
pub const PVM_MAX_BYTES: usize = 64 * 1024;
pub const PVM_MAX_OPCODES: u32 = 4096;
pub const PVM_MAX_DIMENSION: u32 = 8192;

pub const MATTER_L3_MAGIC: [u8; 4] = *b"MTL3";
pub const MATTER_L3_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiError {
    TooSmall,
    TooLarge,
    UnknownMagic,
    UnsupportedVersion,
    NameTooLong,
    NameOutOfBounds,
    InvalidDimension,
    TooManyOpcodes,
    TrailingBytes,
    TruncatedOpcode,
    UnknownOpcode,
    OpcodeCountMismatch,
    PayloadOutOfBounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PvmFormat {
    Pvm1,
    Pvm2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PvmOpcodeTag {
    Clear = 0,
    FillRect = 1,
    Pulse = 2,
    SetBehavior = 3,
    Frame = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pvm2Header {
    pub format_version: u16,
    pub name_len: u16,
    pub package_version: u32,
    pub permissions: u64,
    pub entrypoint: u32,
    pub width: u32,
    pub height: u32,
    pub opcode_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PvmbcInfo<'a> {
    pub format: PvmFormat,
    pub name: &'a [u8],
    pub format_version: u16,
    pub package_version: u32,
    pub permissions: u64,
    pub entrypoint: u32,
    pub width: u32,
    pub height: u32,
    pub declared_opcodes: u32,
    pub decoded_opcodes: u32,
    pub opcode_counts: [u32; 5],
    pub frame_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatterL3Command {
    LoadPvmbc = 1,
    InspectPvmbc = 2,
    RunVisualApp = 3,
    LoadMatterBytecode = 4,
    RunMatterBytecode = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatterL3Request<'a> {
    pub version: u16,
    pub command: MatterL3Command,
    pub flags: u32,
    pub payload: &'a [u8],
}

pub fn inspect_pvmbc(bytes: &[u8]) -> Result<PvmbcInfo<'_>, AbiError> {
    if bytes.len() > PVM_MAX_BYTES {
        return Err(AbiError::TooLarge);
    }
    if bytes.len() < 4 {
        return Err(AbiError::TooSmall);
    }

    match magic(bytes) {
        PVM1_MAGIC => inspect_pvm1(bytes),
        PVM2_MAGIC => inspect_pvm2(bytes),
        _ => Err(AbiError::UnknownMagic),
    }
}

pub fn read_pvm2_header(bytes: &[u8]) -> Result<Pvm2Header, AbiError> {
    if bytes.len() < 36 {
        return Err(AbiError::TooSmall);
    }
    let format_version = read_u16(bytes, 4)?;
    if format_version != PVM2_FORMAT_VERSION {
        return Err(AbiError::UnsupportedVersion);
    }
    let name_len = read_u16(bytes, 6)?;
    if name_len as usize > PVM_MAX_NAME_BYTES {
        return Err(AbiError::NameTooLong);
    }
    let header = Pvm2Header {
        format_version,
        name_len,
        package_version: read_u32(bytes, 8)?,
        permissions: read_u64(bytes, 12)?,
        entrypoint: read_u32(bytes, 20)?,
        width: read_u32(bytes, 24)?,
        height: read_u32(bytes, 28)?,
        opcode_count: read_u32(bytes, 32)?,
    };
    validate_pvm_limits(header.width, header.height, header.opcode_count)?;
    Ok(header)
}

pub fn parse_l3_request(bytes: &[u8]) -> Result<MatterL3Request<'_>, AbiError> {
    if bytes.len() < 16 {
        return Err(AbiError::TooSmall);
    }
    if magic(bytes) != MATTER_L3_MAGIC {
        return Err(AbiError::UnknownMagic);
    }
    let version = read_u16(bytes, 4)?;
    if version != MATTER_L3_VERSION {
        return Err(AbiError::UnsupportedVersion);
    }
    let command = match read_u16(bytes, 6)? {
        1 => MatterL3Command::LoadPvmbc,
        2 => MatterL3Command::InspectPvmbc,
        3 => MatterL3Command::RunVisualApp,
        4 => MatterL3Command::LoadMatterBytecode,
        5 => MatterL3Command::RunMatterBytecode,
        _ => return Err(AbiError::UnknownOpcode),
    };
    let flags = read_u32(bytes, 8)?;
    let payload_len = read_u32(bytes, 12)? as usize;
    let payload_end = 16usize
        .checked_add(payload_len)
        .ok_or(AbiError::PayloadOutOfBounds)?;
    if payload_end > bytes.len() {
        return Err(AbiError::PayloadOutOfBounds);
    }
    if payload_end != bytes.len() {
        return Err(AbiError::TrailingBytes);
    }
    Ok(MatterL3Request {
        version,
        command,
        flags,
        payload: &bytes[16..payload_end],
    })
}

pub fn write_l3_header(
    out: &mut [u8],
    command: MatterL3Command,
    flags: u32,
    payload_len: u32,
) -> Result<usize, AbiError> {
    if out.len() < 16 {
        return Err(AbiError::TooSmall);
    }
    out[0..4].copy_from_slice(&MATTER_L3_MAGIC);
    write_u16(out, 4, MATTER_L3_VERSION)?;
    write_u16(out, 6, command as u16)?;
    write_u32(out, 8, flags)?;
    write_u32(out, 12, payload_len)?;
    Ok(16)
}

fn inspect_pvm1(bytes: &[u8]) -> Result<PvmbcInfo<'_>, AbiError> {
    if bytes.len() < 16 {
        return Err(AbiError::TooSmall);
    }
    let width = read_u32(bytes, 4)?;
    let height = read_u32(bytes, 8)?;
    let opcode_count = read_u32(bytes, 12)?;
    validate_pvm_limits(width, height, opcode_count)?;
    let (decoded_opcodes, opcode_counts, frame_count) =
        inspect_opcode_stream(&bytes[16..], opcode_count)?;
    Ok(PvmbcInfo {
        format: PvmFormat::Pvm1,
        name: &[],
        format_version: 1,
        package_version: 0,
        permissions: 0,
        entrypoint: 0,
        width,
        height,
        declared_opcodes: opcode_count,
        decoded_opcodes,
        opcode_counts,
        frame_count,
    })
}

fn inspect_pvm2(bytes: &[u8]) -> Result<PvmbcInfo<'_>, AbiError> {
    let header = read_pvm2_header(bytes)?;
    let name_start = 36;
    let opcode_start = name_start + header.name_len as usize;
    if opcode_start > bytes.len() {
        return Err(AbiError::NameOutOfBounds);
    }
    let (decoded_opcodes, opcode_counts, frame_count) =
        inspect_opcode_stream(&bytes[opcode_start..], header.opcode_count)?;
    Ok(PvmbcInfo {
        format: PvmFormat::Pvm2,
        name: &bytes[name_start..opcode_start],
        format_version: header.format_version,
        package_version: header.package_version,
        permissions: header.permissions,
        entrypoint: header.entrypoint,
        width: header.width,
        height: header.height,
        declared_opcodes: header.opcode_count,
        decoded_opcodes,
        opcode_counts,
        frame_count,
    })
}

fn inspect_opcode_stream(bytes: &[u8], declared: u32) -> Result<(u32, [u32; 5], u32), AbiError> {
    if declared > PVM_MAX_OPCODES {
        return Err(AbiError::TooManyOpcodes);
    }
    let mut index = 0;
    let mut decoded = 0;
    let mut counts = [0u32; 5];
    let mut frames = 0;
    while index < bytes.len() {
        if decoded >= declared {
            return Err(AbiError::TrailingBytes);
        }
        let tag = bytes[index];
        let size = match tag {
            0 => 1 + 4,
            1 => 1 + 16 + 4,
            2 => 1 + 16 + 1,
            3 => 1 + 16 + 2,
            4 => 1,
            _ => return Err(AbiError::UnknownOpcode),
        };
        if index + size > bytes.len() {
            return Err(AbiError::TruncatedOpcode);
        }
        counts[tag as usize] += 1;
        if tag == PvmOpcodeTag::Frame as u8 {
            frames += 1;
        }
        decoded += 1;
        index += size;
    }
    if decoded != declared {
        return Err(AbiError::OpcodeCountMismatch);
    }
    Ok((decoded, counts, frames))
}

fn validate_pvm_limits(width: u32, height: u32, opcode_count: u32) -> Result<(), AbiError> {
    if width == 0 || height == 0 || width > PVM_MAX_DIMENSION || height > PVM_MAX_DIMENSION {
        return Err(AbiError::InvalidDimension);
    }
    if opcode_count > PVM_MAX_OPCODES {
        return Err(AbiError::TooManyOpcodes);
    }
    Ok(())
}

fn magic(bytes: &[u8]) -> [u8; 4] {
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, AbiError> {
    let slice = bytes.get(offset..offset + 2).ok_or(AbiError::TooSmall)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, AbiError> {
    let slice = bytes.get(offset..offset + 4).ok_or(AbiError::TooSmall)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, AbiError> {
    let slice = bytes.get(offset..offset + 8).ok_or(AbiError::TooSmall)?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) -> Result<(), AbiError> {
    let out = bytes
        .get_mut(offset..offset + 2)
        .ok_or(AbiError::TooSmall)?;
    out.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), AbiError> {
    let out = bytes
        .get_mut(offset..offset + 4)
        .ok_or(AbiError::TooSmall)?;
    out.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    fn push_u16(bytes: &mut std::vec::Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut std::vec::Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u64(bytes: &mut std::vec::Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn sample_pvm2() -> std::vec::Vec<u8> {
        let mut bytes = std::vec::Vec::new();
        bytes.extend_from_slice(b"PVM2");
        push_u16(&mut bytes, 2);
        push_u16(&mut bytes, 11);
        push_u32(&mut bytes, 1);
        push_u64(&mut bytes, 0);
        push_u32(&mut bytes, 0);
        push_u32(&mut bytes, 1280);
        push_u32(&mut bytes, 720);
        push_u32(&mut bytes, 2);
        bytes.extend_from_slice(b"matter-core");
        bytes.push(PvmOpcodeTag::Clear as u8);
        push_u32(&mut bytes, 0xff07111e);
        bytes.push(PvmOpcodeTag::Frame as u8);
        bytes
    }

    #[test]
    fn inspect_pvm2_reports_manifest_and_opcodes() {
        let bytes = sample_pvm2();

        let info = inspect_pvmbc(&bytes).unwrap();

        assert_eq!(info.format, PvmFormat::Pvm2);
        assert_eq!(info.name, b"matter-core");
        assert_eq!(info.width, 1280);
        assert_eq!(info.height, 720);
        assert_eq!(info.declared_opcodes, 2);
        assert_eq!(info.decoded_opcodes, 2);
        assert_eq!(info.opcode_counts[PvmOpcodeTag::Clear as usize], 1);
        assert_eq!(info.opcode_counts[PvmOpcodeTag::Frame as usize], 1);
        assert_eq!(info.frame_count, 1);
    }

    #[test]
    fn parse_l3_request_reads_payload_without_allocation() {
        let payload = [1u8, 2, 3, 4];
        let mut bytes = [0u8; 20];
        write_l3_header(
            &mut bytes,
            MatterL3Command::RunVisualApp,
            7,
            payload.len() as u32,
        )
        .unwrap();
        bytes[16..].copy_from_slice(&payload);

        let request = parse_l3_request(&bytes).unwrap();

        assert_eq!(request.version, MATTER_L3_VERSION);
        assert_eq!(request.command, MatterL3Command::RunVisualApp);
        assert_eq!(request.flags, 7);
        assert_eq!(request.payload, payload);
    }

    #[test]
    fn invalid_opcode_is_rejected() {
        let mut bytes = sample_pvm2();
        let opcode_start = 36 + 11;
        bytes[opcode_start] = 99;

        let error = inspect_pvmbc(&bytes).unwrap_err();

        assert_eq!(error, AbiError::UnknownOpcode);
    }
}
