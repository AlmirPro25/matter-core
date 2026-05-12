//! Mach-O linker for macOS
//!
//! Generates macOS executables from machine code.

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Link machine code into a macOS Mach-O executable
pub fn link_macho<P: AsRef<Path>>(machine_code: &[u8], output_path: P) -> Result<(), String> {
    let mut macho = Vec::new();

    // Mach-O Header (32 bytes for 64-bit)
    write_macho_header(&mut macho);

    // Load Commands
    write_load_commands(&mut macho, machine_code.len());

    // Align to page boundary (4096 bytes)
    align_to(&mut macho, 4096);

    // Code section
    macho.extend_from_slice(machine_code);

    // Write to file
    let mut file =
        File::create(output_path).map_err(|e| format!("Failed to create output file: {}", e))?;

    file.write_all(&macho)
        .map_err(|e| format!("Failed to write output file: {}", e))?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file
            .metadata()
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(output_path.as_ref(), perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    Ok(())
}

/// Write Mach-O header (64-bit)
fn write_macho_header(macho: &mut Vec<u8>) {
    // magic (MH_MAGIC_64)
    macho.extend_from_slice(&[0xCF, 0xFA, 0xED, 0xFE]);

    // cputype (CPU_TYPE_X86_64)
    macho.extend_from_slice(&0x01000007u32.to_le_bytes());

    // cpusubtype (CPU_SUBTYPE_X86_64_ALL)
    macho.extend_from_slice(&0x00000003u32.to_le_bytes());

    // filetype (MH_EXECUTE)
    macho.extend_from_slice(&0x00000002u32.to_le_bytes());

    // ncmds (number of load commands = 2)
    macho.extend_from_slice(&0x00000002u32.to_le_bytes());

    // sizeofcmds (size of load commands)
    macho.extend_from_slice(&0x00000150u32.to_le_bytes());

    // flags (MH_NOUNDEFS | MH_DYLDLINK | MH_TWOLEVEL)
    macho.extend_from_slice(&0x00000085u32.to_le_bytes());

    // reserved (64-bit only)
    macho.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
}

/// Write load commands
fn write_load_commands(macho: &mut Vec<u8>, code_size: usize) {
    // LC_SEGMENT_64 command
    write_segment_command(macho, code_size);

    // LC_UNIXTHREAD command (entry point)
    write_thread_command(macho);
}

/// Write LC_SEGMENT_64 command
fn write_segment_command(macho: &mut Vec<u8>, code_size: usize) {
    // cmd (LC_SEGMENT_64)
    macho.extend_from_slice(&0x00000019u32.to_le_bytes());

    // cmdsize (size of this command = 152 bytes)
    macho.extend_from_slice(&0x00000098u32.to_le_bytes());

    // segname ("__TEXT")
    let mut segname = [0u8; 16];
    segname[..6].copy_from_slice(b"__TEXT");
    macho.extend_from_slice(&segname);

    // vmaddr (virtual memory address = 0x100000000)
    macho.extend_from_slice(&0x100000000u64.to_le_bytes());

    // vmsize (virtual memory size)
    let vmsize = align_value(code_size, 4096);
    macho.extend_from_slice(&(vmsize as u64).to_le_bytes());

    // fileoff (file offset = 0)
    macho.extend_from_slice(&0u64.to_le_bytes());

    // filesize (file size)
    macho.extend_from_slice(&(code_size as u64).to_le_bytes());

    // maxprot (VM_PROT_READ | VM_PROT_WRITE | VM_PROT_EXECUTE)
    macho.extend_from_slice(&0x00000007u32.to_le_bytes());

    // initprot (VM_PROT_READ | VM_PROT_EXECUTE)
    macho.extend_from_slice(&0x00000005u32.to_le_bytes());

    // nsects (number of sections = 1)
    macho.extend_from_slice(&0x00000001u32.to_le_bytes());

    // flags
    macho.extend_from_slice(&0x00000000u32.to_le_bytes());

    // Section header (80 bytes)
    write_section_header(macho, code_size);
}

/// Write section header
fn write_section_header(macho: &mut Vec<u8>, code_size: usize) {
    // sectname ("__text")
    let mut sectname = [0u8; 16];
    sectname[..6].copy_from_slice(b"__text");
    macho.extend_from_slice(&sectname);

    // segname ("__TEXT")
    let mut segname = [0u8; 16];
    segname[..6].copy_from_slice(b"__TEXT");
    macho.extend_from_slice(&segname);

    // addr (address = 0x100001000)
    macho.extend_from_slice(&0x100001000u64.to_le_bytes());

    // size
    macho.extend_from_slice(&(code_size as u64).to_le_bytes());

    // offset (file offset = 4096)
    macho.extend_from_slice(&0x00001000u32.to_le_bytes());

    // align (2^0 = 1 byte alignment)
    macho.extend_from_slice(&0x00000000u32.to_le_bytes());

    // reloff (no relocations)
    macho.extend_from_slice(&0x00000000u32.to_le_bytes());

    // nreloc (no relocations)
    macho.extend_from_slice(&0x00000000u32.to_le_bytes());

    // flags (S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS)
    macho.extend_from_slice(&0x80000400u32.to_le_bytes());

    // reserved1
    macho.extend_from_slice(&0x00000000u32.to_le_bytes());

    // reserved2
    macho.extend_from_slice(&0x00000000u32.to_le_bytes());

    // reserved3 (64-bit only)
    macho.extend_from_slice(&0x00000000u32.to_le_bytes());
}

/// Write LC_UNIXTHREAD command
fn write_thread_command(macho: &mut Vec<u8>) {
    // cmd (LC_UNIXTHREAD)
    macho.extend_from_slice(&0x00000005u32.to_le_bytes());

    // cmdsize (size of this command = 184 bytes)
    macho.extend_from_slice(&0x000000B8u32.to_le_bytes());

    // flavor (x86_THREAD_STATE64)
    macho.extend_from_slice(&0x00000004u32.to_le_bytes());

    // count (number of u32s in thread state = 42)
    macho.extend_from_slice(&0x0000002Au32.to_le_bytes());

    // Thread state (168 bytes)
    // RAX through R15, RIP, RFLAGS, CS, FS, GS
    let mut state = vec![0u8; 168];

    // Set RIP (instruction pointer) to entry point (0x100001000)
    let rip_offset = 16 * 8; // RIP is at offset 128
    state[rip_offset..rip_offset + 8].copy_from_slice(&0x100001000u64.to_le_bytes());

    macho.extend_from_slice(&state);
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
        std::env::temp_dir().join(format!("matter_native_{}_{}_macho", name, stamp))
    }

    #[test]
    fn test_align_value() {
        assert_eq!(align_value(100, 4096), 4096);
        assert_eq!(align_value(4096, 4096), 4096);
        assert_eq!(align_value(4097, 4096), 8192);
    }

    #[test]
    fn test_link_macho_basic() {
        let code = vec![0x48, 0xC7, 0xC0, 0x00, 0x00, 0x00, 0x00, 0xC3]; // mov rax, 0; ret
        let output = temp_output("basic");

        link_macho(&code, &output).expect("Mach-O linker should generate output");

        let bytes = fs::read(&output).expect("linked Mach-O should be readable");
        assert!(
            bytes.len() >= 4096 + code.len(),
            "Mach-O should include aligned header and code"
        );
        assert_eq!(
            &bytes[0..4],
            &[0xCF, 0xFA, 0xED, 0xFE],
            "Mach-O magic mismatch"
        );
        assert_eq!(
            &bytes[4096..4096 + code.len()],
            code.as_slice(),
            "code payload should be embedded at text offset"
        );

        let _ = fs::remove_file(output);
    }

    #[test]
    fn test_link_macho_invalid_output_path() {
        let code = vec![0xC3]; // ret
        let invalid = std::env::temp_dir()
            .join("matter_native_missing_dir")
            .join("nested")
            .join("out.macho");

        let result = link_macho(&code, &invalid);
        assert!(
            result.is_err(),
            "linker should fail for non-existent parent dir"
        );
    }

    #[test]
    fn test_link_macho_large_payload_alignment() {
        let code = vec![0x90; 11_111]; // NOP sled
        let output = temp_output("large");

        link_macho(&code, &output).expect("Mach-O linker should handle large payload");
        let bytes = fs::read(&output).expect("linked Mach-O should be readable");

        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]);
        assert_eq!(&bytes[4096..4096 + code.len()], code.as_slice());
        assert!(bytes.len() >= 4096 + code.len());

        let _ = fs::remove_file(output);
    }
}
