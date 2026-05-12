//! Linker modules for different executable formats

pub mod elf; // Linux ELF format
pub mod macho;
pub mod pe; // Windows PE format // macOS Mach-O format

/// Executable section
#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub data: Vec<u8>,
    pub virtual_address: u64,
    pub virtual_size: u64,
    pub flags: SectionFlags,
}

/// Section flags
#[derive(Debug, Clone, Copy)]
pub struct SectionFlags {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

impl SectionFlags {
    pub fn code() -> Self {
        Self {
            readable: true,
            writable: false,
            executable: true,
        }
    }

    pub fn data() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
        }
    }

    pub fn rodata() -> Self {
        Self {
            readable: true,
            writable: false,
            executable: false,
        }
    }
}
