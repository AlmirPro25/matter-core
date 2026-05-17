//! PE (Portable Executable) linker for Windows
//!
//! Generates Windows .exe files from machine code.

use std::fs::File;
use std::io::Write;
use std::path::Path;

pub const IMAGE_BASE: u64 = 0x400000;
pub const TEXT_RVA: u32 = 0x1000;

const SECTION_ALIGNMENT: usize = 4096;
const FILE_ALIGNMENT: usize = 512;
const TEXT_RAW_POINTER: usize = 0x1000;
const IMPORT_FUNCTIONS: [&[u8]; 5] = [
    b"ExitProcess\0",
    b"GetStdHandle\0",
    b"WriteFile\0",
    b"GetProcessHeap\0",
    b"HeapAlloc\0",
];
const EXIT_PROCESS_IAT_INDEX: usize = 0;
const GET_STD_HANDLE_IAT_INDEX: usize = 1;
const WRITE_FILE_IAT_INDEX: usize = 2;
const GET_PROCESS_HEAP_IAT_INDEX: usize = 3;
const HEAP_ALLOC_IAT_INDEX: usize = 4;

/// Link machine code into a Windows PE executable
pub fn link_pe<P: AsRef<Path>>(machine_code: &[u8], output_path: P) -> Result<(), String> {
    let mut exe = Vec::new();
    let layout = PeLayout::new(machine_code.len())?;
    let idata = build_import_table(layout.idata_rva);

    // DOS Header (64 bytes)
    write_dos_header(&mut exe);

    // DOS Stub (64 bytes)
    write_dos_stub(&mut exe);

    // PE Signature (4 bytes)
    exe.extend_from_slice(b"PE\0\0");

    // COFF Header (20 bytes)
    write_coff_header(&mut exe);

    // Optional Header (224 bytes for PE32+)
    write_optional_header(&mut exe, &layout, idata.len());

    // Section Headers
    write_section_headers(&mut exe, &layout, idata.len());

    // Align to section alignment (4096 bytes)
    align_to(&mut exe, TEXT_RAW_POINTER);

    // .text section (code)
    exe.extend_from_slice(machine_code);

    // Align to file alignment (512 bytes)
    align_to(&mut exe, FILE_ALIGNMENT);

    // .idata section (imports)
    exe.extend_from_slice(&idata);

    // Align to file alignment (512 bytes)
    align_to(&mut exe, FILE_ALIGNMENT);

    // Write to file
    let mut file =
        File::create(output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    file.write_all(&exe)
        .map_err(|e| format!("Failed to write output file: {}", e))?;

    Ok(())
}

pub fn exit_process_iat_rva(code_size: usize) -> u32 {
    import_iat_rva(code_size, EXIT_PROCESS_IAT_INDEX)
}

pub fn get_std_handle_iat_rva(code_size: usize) -> u32 {
    import_iat_rva(code_size, GET_STD_HANDLE_IAT_INDEX)
}

pub fn write_file_iat_rva(code_size: usize) -> u32 {
    import_iat_rva(code_size, WRITE_FILE_IAT_INDEX)
}

pub fn get_process_heap_iat_rva(code_size: usize) -> u32 {
    import_iat_rva(code_size, GET_PROCESS_HEAP_IAT_INDEX)
}

pub fn heap_alloc_iat_rva(code_size: usize) -> u32 {
    import_iat_rva(code_size, HEAP_ALLOC_IAT_INDEX)
}

fn import_iat_rva(code_size: usize, index: usize) -> u32 {
    PeLayout::new(code_size)
        .expect("code size should fit in PE layout")
        .idata_rva
        + IMPORT_IAT_OFFSET as u32
        + (index * 8) as u32
}

#[derive(Debug, Clone, Copy)]
struct PeLayout {
    text_raw_size: usize,
    idata_rva: u32,
    idata_raw_pointer: u32,
}

impl PeLayout {
    fn new(code_size: usize) -> Result<Self, String> {
        let text_raw_size = align_value(code_size, FILE_ALIGNMENT);
        let idata_rva = align_value(TEXT_RVA as usize + code_size, SECTION_ALIGNMENT) as u32;
        let idata_raw_pointer = (TEXT_RAW_POINTER + text_raw_size) as u32;

        Ok(Self {
            text_raw_size,
            idata_rva,
            idata_raw_pointer,
        })
    }
}

const IMPORT_DESCRIPTOR_SIZE: usize = 20;
const IMPORT_NULL_DESCRIPTOR_SIZE: usize = 20;
const IMPORT_ILT_OFFSET: usize = IMPORT_DESCRIPTOR_SIZE + IMPORT_NULL_DESCRIPTOR_SIZE;
const IMPORT_THUNK_BYTES: usize = (IMPORT_FUNCTIONS.len() + 1) * 8;
const IMPORT_IAT_OFFSET: usize = IMPORT_ILT_OFFSET + IMPORT_THUNK_BYTES;
const IMPORT_HINT_NAME_OFFSET: usize = IMPORT_IAT_OFFSET + IMPORT_THUNK_BYTES;

fn build_import_table(idata_rva: u32) -> Vec<u8> {
    let mut idata = Vec::new();

    idata.extend_from_slice(&(idata_rva + IMPORT_ILT_OFFSET as u32).to_le_bytes());
    idata.extend_from_slice(&0u32.to_le_bytes());
    idata.extend_from_slice(&0u32.to_le_bytes());

    let dll_name_offset = IMPORT_HINT_NAME_OFFSET
        + IMPORT_FUNCTIONS
            .iter()
            .map(|name| 2 + name.len())
            .sum::<usize>();
    let dll_name_rva = idata_rva + dll_name_offset as u32;
    idata.extend_from_slice(&dll_name_rva.to_le_bytes());
    idata.extend_from_slice(&(idata_rva + IMPORT_IAT_OFFSET as u32).to_le_bytes());

    idata.extend_from_slice(&[0; IMPORT_NULL_DESCRIPTOR_SIZE]);

    let mut hint_name_rvas = Vec::with_capacity(IMPORT_FUNCTIONS.len());
    let mut hint_name_offset = IMPORT_HINT_NAME_OFFSET;
    for function_name in IMPORT_FUNCTIONS {
        hint_name_rvas.push(idata_rva + hint_name_offset as u32);
        hint_name_offset += 2 + function_name.len();
    }

    for hint_name_rva in &hint_name_rvas {
        idata.extend_from_slice(&(*hint_name_rva as u64).to_le_bytes());
    }
    idata.extend_from_slice(&0u64.to_le_bytes());

    for hint_name_rva in &hint_name_rvas {
        idata.extend_from_slice(&(*hint_name_rva as u64).to_le_bytes());
    }
    idata.extend_from_slice(&0u64.to_le_bytes());

    for function_name in IMPORT_FUNCTIONS {
        idata.extend_from_slice(&0u16.to_le_bytes());
        idata.extend_from_slice(function_name);
    }
    idata.extend_from_slice(b"KERNEL32.dll\0");
    idata
}

/// Write DOS header (MZ header)
fn write_dos_header(exe: &mut Vec<u8>) {
    exe.extend_from_slice(b"MZ"); // e_magic
    exe.extend_from_slice(&[0x90, 0x00]); // e_cblp
    exe.extend_from_slice(&[0x03, 0x00]); // e_cp
    exe.extend_from_slice(&[0x00, 0x00]); // e_crlc
    exe.extend_from_slice(&[0x04, 0x00]); // e_cparhdr
    exe.extend_from_slice(&[0x00, 0x00]); // e_minalloc
    exe.extend_from_slice(&[0xFF, 0xFF]); // e_maxalloc
    exe.extend_from_slice(&[0x00, 0x00]); // e_ss
    exe.extend_from_slice(&[0xB8, 0x00]); // e_sp
    exe.extend_from_slice(&[0x00, 0x00]); // e_csum
    exe.extend_from_slice(&[0x00, 0x00]); // e_ip
    exe.extend_from_slice(&[0x00, 0x00]); // e_cs
    exe.extend_from_slice(&[0x40, 0x00]); // e_lfarlc
    exe.extend_from_slice(&[0x00, 0x00]); // e_ovno
    exe.extend_from_slice(&[0; 8]); // e_res
    exe.extend_from_slice(&[0x00, 0x00]); // e_oemid
    exe.extend_from_slice(&[0x00, 0x00]); // e_oeminfo
    exe.extend_from_slice(&[0; 20]); // e_res2
    exe.extend_from_slice(&[0x80, 0x00, 0x00, 0x00]); // e_lfanew (PE header at 0x80)
}

/// Write DOS stub program
fn write_dos_stub(exe: &mut Vec<u8>) {
    // Simple DOS stub that prints "This program cannot be run in DOS mode."
    exe.extend_from_slice(&[
        0x0E, 0x1F, 0xBA, 0x0E, 0x00, 0xB4, 0x09, 0xCD, 0x21, 0xB8, 0x01, 0x4C, 0xCD, 0x21, 0x54,
        0x68, 0x69, 0x73, 0x20, 0x70, 0x72, 0x6F, 0x67, 0x72, 0x61, 0x6D, 0x20, 0x63, 0x61, 0x6E,
        0x6E, 0x6F, 0x74, 0x20, 0x62, 0x65, 0x20, 0x72, 0x75, 0x6E, 0x20, 0x69, 0x6E, 0x20, 0x44,
        0x4F, 0x53, 0x20, 0x6D, 0x6F, 0x64, 0x65, 0x2E, 0x0D, 0x0D, 0x0A, 0x24, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ]);
}

/// Write COFF header
fn write_coff_header(exe: &mut Vec<u8>) {
    exe.extend_from_slice(&[0x64, 0x86]); // Machine (x86-64)
    exe.extend_from_slice(&[0x02, 0x00]); // NumberOfSections
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // TimeDateStamp
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // PointerToSymbolTable
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // NumberOfSymbols
    exe.extend_from_slice(&[0xF0, 0x00]); // SizeOfOptionalHeader (240 bytes)
    exe.extend_from_slice(&[0x22, 0x00]); // Characteristics (executable, large address aware)
}

/// Write Optional Header (PE32+)
fn write_optional_header(exe: &mut Vec<u8>, layout: &PeLayout, idata_size: usize) {
    // Standard fields
    exe.extend_from_slice(&[0x0B, 0x02]); // Magic (PE32+)
    exe.extend_from_slice(&[0x0E, 0x00]); // Linker version
    exe.extend_from_slice(&(layout.text_raw_size as u32).to_le_bytes()); // SizeOfCode
    exe.extend_from_slice(&(align_value(idata_size, FILE_ALIGNMENT) as u32).to_le_bytes()); // SizeOfInitializedData
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // SizeOfUninitializedData
    exe.extend_from_slice(&TEXT_RVA.to_le_bytes()); // AddressOfEntryPoint
    exe.extend_from_slice(&TEXT_RVA.to_le_bytes()); // BaseOfCode

    // NT additional fields
    exe.extend_from_slice(&IMAGE_BASE.to_le_bytes()); // ImageBase
    exe.extend_from_slice(&(SECTION_ALIGNMENT as u32).to_le_bytes()); // SectionAlignment
    exe.extend_from_slice(&(FILE_ALIGNMENT as u32).to_le_bytes()); // FileAlignment
    exe.extend_from_slice(&[0x06, 0x00]); // MajorOperatingSystemVersion
    exe.extend_from_slice(&[0x00, 0x00]); // MinorOperatingSystemVersion
    exe.extend_from_slice(&[0x00, 0x00]); // MajorImageVersion
    exe.extend_from_slice(&[0x00, 0x00]); // MinorImageVersion
    exe.extend_from_slice(&[0x06, 0x00]); // MajorSubsystemVersion
    exe.extend_from_slice(&[0x00, 0x00]); // MinorSubsystemVersion
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Win32VersionValue

    let image_size = align_value(layout.idata_rva as usize + idata_size, SECTION_ALIGNMENT);
    exe.extend_from_slice(&(image_size as u32).to_le_bytes()); // SizeOfImage
    exe.extend_from_slice(&[0x00, 0x04, 0x00, 0x00]); // SizeOfHeaders (1024)
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // CheckSum
    exe.extend_from_slice(&[0x03, 0x00]); // Subsystem (console)
    exe.extend_from_slice(&[0x00, 0x01]); // DllCharacteristics (NX compatible, fixed image base)
    exe.extend_from_slice(&[0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00]); // SizeOfStackReserve
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // SizeOfStackCommit
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // SizeOfHeapReserve
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // SizeOfHeapCommit
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // LoaderFlags
    exe.extend_from_slice(&[0x10, 0x00, 0x00, 0x00]); // NumberOfRvaAndSizes

    // Data directories (16 entries, 8 bytes each = 128 bytes)
    exe.extend_from_slice(&[0; 8]); // Export table
    exe.extend_from_slice(&layout.idata_rva.to_le_bytes()); // Import table RVA
    exe.extend_from_slice(&(idata_size as u32).to_le_bytes()); // Import table size
    exe.extend_from_slice(&[0; 112]);
}

/// Write section headers
fn write_section_headers(exe: &mut Vec<u8>, layout: &PeLayout, idata_size: usize) {
    // .text section
    exe.extend_from_slice(b".text\0\0\0"); // Name
    exe.extend_from_slice(&(layout.text_raw_size as u32).to_le_bytes()); // VirtualSize
    exe.extend_from_slice(&TEXT_RVA.to_le_bytes()); // VirtualAddress

    exe.extend_from_slice(&(layout.text_raw_size as u32).to_le_bytes()); // SizeOfRawData
    exe.extend_from_slice(&(TEXT_RAW_POINTER as u32).to_le_bytes()); // PointerToRawData
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // PointerToRelocations
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // PointerToLinenumbers
    exe.extend_from_slice(&[0x00, 0x00]); // NumberOfRelocations
    exe.extend_from_slice(&[0x00, 0x00]); // NumberOfLinenumbers
    exe.extend_from_slice(&[0x20, 0x00, 0x00, 0x60]); // Characteristics (code, executable, readable)

    // .idata section
    exe.extend_from_slice(b".idata\0\0"); // Name
    exe.extend_from_slice(&(idata_size as u32).to_le_bytes()); // VirtualSize
    exe.extend_from_slice(&layout.idata_rva.to_le_bytes()); // VirtualAddress

    let raw_size = align_value(idata_size, FILE_ALIGNMENT);
    exe.extend_from_slice(&(raw_size as u32).to_le_bytes()); // SizeOfRawData
    exe.extend_from_slice(&layout.idata_raw_pointer.to_le_bytes()); // PointerToRawData
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // PointerToRelocations
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // PointerToLinenumbers
    exe.extend_from_slice(&[0x00, 0x00]); // NumberOfRelocations
    exe.extend_from_slice(&[0x00, 0x00]); // NumberOfLinenumbers
    exe.extend_from_slice(&[0x40, 0x00, 0x00, 0xC0]); // Characteristics (initialized data, readable, writable)
}

/// Align vector to specified alignment
fn align_to(data: &mut Vec<u8>, alignment: usize) {
    let current = data.len();
    let aligned = align_value(current, alignment);
    let padding = aligned - current;
    data.extend_from_slice(&vec![0; padding]);
}

/// Calculate aligned value
fn align_value(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_output(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "matter_native_{}_{}_{}.exe",
            name,
            std::process::id(),
            stamp
        ))
    }

    #[test]
    fn test_align_value() {
        assert_eq!(align_value(100, 512), 512);
        assert_eq!(align_value(512, 512), 512);
        assert_eq!(align_value(513, 512), 1024);
    }

    #[test]
    fn test_link_pe_basic() {
        let code = vec![0x48, 0xC7, 0xC0, 0x00, 0x00, 0x00, 0x00, 0xC3]; // mov rax, 0; ret
        let output = temp_output("basic");

        link_pe(&code, &output).expect("PE linker should generate output");

        let bytes = fs::read(&output).expect("linked PE should be readable");
        assert!(
            bytes.len() >= 4096 + code.len(),
            "PE should include aligned header and code"
        );
        assert_eq!(&bytes[0..2], b"MZ", "DOS signature mismatch");
        assert_eq!(&bytes[0x80..0x84], b"PE\0\0", "PE signature mismatch");
        assert_eq!(
            &bytes[4096..4096 + code.len()],
            code.as_slice(),
            "code payload should be embedded at .text"
        );

        let _ = fs::remove_file(output);
    }

    #[test]
    fn test_link_pe_invalid_output_path() {
        let code = vec![0xC3]; // ret
        let invalid = std::env::temp_dir()
            .join("matter_native_missing_dir")
            .join("nested")
            .join("out.exe");

        let result = link_pe(&code, &invalid);
        assert!(
            result.is_err(),
            "linker should fail for non-existent parent dir"
        );
    }

    #[test]
    fn test_link_pe_large_payload_alignment() {
        let code = vec![0x90; 10_000]; // NOP sled
        let output = temp_output("large");

        link_pe(&code, &output).expect("PE linker should handle large payload");
        let bytes = fs::read(&output).expect("linked PE should be readable");

        assert_eq!(&bytes[0..2], b"MZ");
        assert_eq!(&bytes[0x80..0x84], b"PE\0\0");
        assert_eq!(&bytes[4096..4096 + code.len()], code.as_slice());
        assert_eq!(
            bytes.len() % 512,
            0,
            "PE file should be padded to file alignment"
        );

        let _ = fs::remove_file(output);
    }
}
