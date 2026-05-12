//! PE (Portable Executable) linker for Windows
//!
//! Generates Windows .exe files from machine code.

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Link machine code into a Windows PE executable
pub fn link_pe<P: AsRef<Path>>(machine_code: &[u8], output_path: P) -> Result<(), String> {
    let mut exe = Vec::new();

    // DOS Header (64 bytes)
    write_dos_header(&mut exe);

    // DOS Stub (64 bytes)
    write_dos_stub(&mut exe);

    // PE Signature (4 bytes)
    exe.extend_from_slice(b"PE\0\0");

    // COFF Header (20 bytes)
    write_coff_header(&mut exe, machine_code.len());

    // Optional Header (224 bytes for PE32+)
    write_optional_header(&mut exe, machine_code.len());

    // Section Headers
    write_section_headers(&mut exe, machine_code.len());

    // Align to section alignment (4096 bytes)
    align_to(&mut exe, 4096);

    // .text section (code)
    exe.extend_from_slice(machine_code);

    // Align to file alignment (512 bytes)
    align_to(&mut exe, 512);

    // Write to file
    let mut file =
        File::create(output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    file.write_all(&exe)
        .map_err(|e| format!("Failed to write output file: {}", e))?;

    Ok(())
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
fn write_coff_header(exe: &mut Vec<u8>, _code_size: usize) {
    exe.extend_from_slice(&[0x64, 0x86]); // Machine (x86-64)
    exe.extend_from_slice(&[0x01, 0x00]); // NumberOfSections
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // TimeDateStamp
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // PointerToSymbolTable
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // NumberOfSymbols
    exe.extend_from_slice(&[0xF0, 0x00]); // SizeOfOptionalHeader (240 bytes)
    exe.extend_from_slice(&[0x22, 0x00]); // Characteristics (executable, large address aware)
}

/// Write Optional Header (PE32+)
fn write_optional_header(exe: &mut Vec<u8>, code_size: usize) {
    // Standard fields
    exe.extend_from_slice(&[0x0B, 0x02]); // Magic (PE32+)
    exe.extend_from_slice(&[0x0E, 0x00]); // Linker version
    exe.extend_from_slice(&(code_size as u32).to_le_bytes()); // SizeOfCode
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // SizeOfInitializedData
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // SizeOfUninitializedData
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00]); // AddressOfEntryPoint (0x1000)
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00]); // BaseOfCode

    // NT additional fields
    exe.extend_from_slice(&[0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00]); // ImageBase
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00]); // SectionAlignment (4096)
    exe.extend_from_slice(&[0x00, 0x02, 0x00, 0x00]); // FileAlignment (512)
    exe.extend_from_slice(&[0x06, 0x00]); // MajorOperatingSystemVersion
    exe.extend_from_slice(&[0x00, 0x00]); // MinorOperatingSystemVersion
    exe.extend_from_slice(&[0x00, 0x00]); // MajorImageVersion
    exe.extend_from_slice(&[0x00, 0x00]); // MinorImageVersion
    exe.extend_from_slice(&[0x06, 0x00]); // MajorSubsystemVersion
    exe.extend_from_slice(&[0x00, 0x00]); // MinorSubsystemVersion
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Win32VersionValue

    let image_size = align_value(0x1000 + code_size, 4096);
    exe.extend_from_slice(&(image_size as u32).to_le_bytes()); // SizeOfImage
    exe.extend_from_slice(&[0x00, 0x04, 0x00, 0x00]); // SizeOfHeaders (1024)
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // CheckSum
    exe.extend_from_slice(&[0x03, 0x00]); // Subsystem (console)
    exe.extend_from_slice(&[0x60, 0x81]); // DllCharacteristics
    exe.extend_from_slice(&[0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00]); // SizeOfStackReserve
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // SizeOfStackCommit
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // SizeOfHeapReserve
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // SizeOfHeapCommit
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // LoaderFlags
    exe.extend_from_slice(&[0x10, 0x00, 0x00, 0x00]); // NumberOfRvaAndSizes

    // Data directories (16 entries, 8 bytes each = 128 bytes)
    exe.extend_from_slice(&[0; 128]);
}

/// Write section headers
fn write_section_headers(exe: &mut Vec<u8>, code_size: usize) {
    // .text section
    exe.extend_from_slice(b".text\0\0\0"); // Name
    exe.extend_from_slice(&(code_size as u32).to_le_bytes()); // VirtualSize
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00]); // VirtualAddress

    let raw_size = align_value(code_size, 512);
    exe.extend_from_slice(&(raw_size as u32).to_le_bytes()); // SizeOfRawData
    exe.extend_from_slice(&[0x00, 0x10, 0x00, 0x00]); // PointerToRawData (4096)
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // PointerToRelocations
    exe.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // PointerToLinenumbers
    exe.extend_from_slice(&[0x00, 0x00]); // NumberOfRelocations
    exe.extend_from_slice(&[0x00, 0x00]); // NumberOfLinenumbers
    exe.extend_from_slice(&[0x20, 0x00, 0x00, 0x60]); // Characteristics (code, executable, readable)
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
