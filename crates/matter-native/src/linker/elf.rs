//! ELF (Executable and Linkable Format) linker for Linux
//!
//! Generates Linux executables from machine code.

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Link machine code into a Linux ELF executable
pub fn link_elf<P: AsRef<Path>>(machine_code: &[u8], output_path: P) -> Result<(), String> {
    let mut elf = Vec::new();

    // ELF Header (64 bytes)
    write_elf_header(&mut elf, machine_code.len());

    // Program Headers
    write_program_headers(&mut elf, machine_code.len());

    // Code section
    elf.extend_from_slice(machine_code);

    // Write to file
    let mut file =
        File::create(output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    file.write_all(&elf)
        .map_err(|e| format!("Failed to write output file: {}", e))?;

    // Make executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file
            .metadata()
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(output_path, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    Ok(())
}

/// Write ELF header
fn write_elf_header(elf: &mut Vec<u8>, _code_size: usize) {
    // ELF identification
    elf.extend_from_slice(&[0x7F, b'E', b'L', b'F']); // Magic number
    elf.push(2); // 64-bit
    elf.push(1); // Little endian
    elf.push(1); // ELF version
    elf.push(0); // System V ABI
    elf.extend_from_slice(&[0; 8]); // Padding

    // ELF header fields
    elf.extend_from_slice(&[0x02, 0x00]); // e_type: ET_EXEC
    elf.extend_from_slice(&[0x3E, 0x00]); // e_machine: x86-64
    elf.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // e_version
    elf.extend_from_slice(&[0x78, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00]); // e_entry: 0x400078
    elf.extend_from_slice(&[0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // e_phoff: 64
    elf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // e_shoff: 0
    elf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // e_flags
    elf.extend_from_slice(&[0x40, 0x00]); // e_ehsize: 64
    elf.extend_from_slice(&[0x38, 0x00]); // e_phentsize: 56
    elf.extend_from_slice(&[0x01, 0x00]); // e_phnum: 1
    elf.extend_from_slice(&[0x00, 0x00]); // e_shentsize: 0
    elf.extend_from_slice(&[0x00, 0x00]); // e_shnum: 0
    elf.extend_from_slice(&[0x00, 0x00]); // e_shstrndx: 0
}

/// Write program headers
fn write_program_headers(elf: &mut Vec<u8>, code_size: usize) {
    // Program header for code segment
    elf.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // p_type: PT_LOAD
    elf.extend_from_slice(&[0x05, 0x00, 0x00, 0x00]); // p_flags: PF_R | PF_X
    elf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // p_offset: 0
    elf.extend_from_slice(&[0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00]); // p_vaddr: 0x400000
    elf.extend_from_slice(&[0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00]); // p_paddr: 0x400000

    let file_size = 64 + 56 + code_size;
    elf.extend_from_slice(&(file_size as u64).to_le_bytes()); // p_filesz
    elf.extend_from_slice(&(file_size as u64).to_le_bytes()); // p_memsz
    elf.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // p_align: 4096
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
        std::env::temp_dir().join(format!("matter_native_{}_{}_elf", name, stamp))
    }

    #[test]
    fn test_link_elf_basic() {
        let code = vec![0x48, 0xC7, 0xC0, 0x00, 0x00, 0x00, 0x00, 0xC3]; // mov rax, 0; ret
        let output = temp_output("basic");

        link_elf(&code, &output).expect("ELF linker should generate output");

        let bytes = fs::read(&output).expect("linked ELF should be readable");
        assert!(
            bytes.len() >= 64 + 56 + code.len(),
            "ELF should include header, program header and code"
        );
        assert_eq!(
            &bytes[0..4],
            &[0x7F, b'E', b'L', b'F'],
            "ELF magic mismatch"
        );
        assert_eq!(bytes[4], 2, "ELF class must be 64-bit");
        assert_eq!(bytes[5], 1, "ELF must be little-endian");
        assert_eq!(
            &bytes[120..120 + code.len()],
            code.as_slice(),
            "code payload should start after ELF+PHDR"
        );

        let _ = fs::remove_file(output);
    }

    #[test]
    fn test_link_elf_invalid_output_path() {
        let code = vec![0xC3]; // ret
        let invalid = std::env::temp_dir()
            .join("matter_native_missing_dir")
            .join("nested")
            .join("out.elf");

        let result = link_elf(&code, &invalid);
        assert!(
            result.is_err(),
            "linker should fail for non-existent parent dir"
        );
    }

    #[test]
    fn test_link_elf_large_payload_layout() {
        let code = vec![0x90; 12_345]; // NOP sled
        let output = temp_output("large");

        link_elf(&code, &output).expect("ELF linker should handle large payload");
        let bytes = fs::read(&output).expect("linked ELF should be readable");

        assert_eq!(&bytes[0..4], &[0x7F, b'E', b'L', b'F']);
        assert_eq!(&bytes[120..120 + code.len()], code.as_slice());
        assert_eq!(
            bytes.len(),
            64 + 56 + code.len(),
            "ELF should keep compact layout"
        );

        let _ = fs::remove_file(output);
    }
}
