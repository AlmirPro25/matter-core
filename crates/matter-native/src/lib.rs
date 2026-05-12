//! Matter Native Compiler (MNC)
//!
//! A from-scratch native code generator for Matter Core.
//! Zero external dependencies, pure Rust implementation.
//!
//! ## Architecture
//!
//! ```text
//! Matter Bytecode → Code Generator → Machine Code → Linker → Executable
//! ```
//!
//! ## Features
//!
//! - ✅ Zero dependencies (no LLVM, no GCC)
//! - ✅ Direct x86-64 code generation
//! - ✅ Optimized for Matter specifically
//! - ✅ Fast compilation
//! - ✅ Small binaries
//! - ✅ ARM64 support
//! - ✅ RISC-V support
//! - ✅ SIMD vectorization (SSE/AVX/NEON/RVV)

pub mod autopgo;
pub mod codegen;
pub mod linker;
pub mod lto;
pub mod optimizer;
pub mod profiler;
pub mod runtime;
pub mod simd;

use matter_bytecode::Bytecode;
use std::path::Path;

/// Compilation target architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    ARM64,
    RISCV64,
}

/// Compilation target operating system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    Windows,
    Linux,
    MacOS,
}

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    O0, // No optimization
    O1, // Basic optimization
    O2, // Moderate optimization
    O3, // Aggressive optimization
}

/// Compilation configuration
#[derive(Debug, Clone)]
pub struct CompileConfig {
    pub arch: Architecture,
    pub os: OperatingSystem,
    pub opt_level: OptLevel,
    pub debug_info: bool,
    pub enable_lto: bool,
}

impl Default for CompileConfig {
    fn default() -> Self {
        Self {
            arch: Architecture::X86_64,
            os: if cfg!(target_os = "windows") {
                OperatingSystem::Windows
            } else if cfg!(target_os = "linux") {
                OperatingSystem::Linux
            } else {
                OperatingSystem::MacOS
            },
            opt_level: OptLevel::O2,
            debug_info: false,
            enable_lto: true, // LTO enabled by default
        }
    }
}

/// Main compiler interface
pub struct NativeCompiler {
    config: CompileConfig,
}

impl NativeCompiler {
    /// Create a new native compiler with default configuration
    pub fn new() -> Self {
        Self {
            config: CompileConfig::default(),
        }
    }

    /// Create a new native compiler with custom configuration
    pub fn with_config(config: CompileConfig) -> Self {
        Self { config }
    }

    /// Compile Matter bytecode to native machine code
    pub fn compile(&self, bytecode: &Bytecode) -> Result<Vec<u8>, String> {
        // Step 0: Apply LTO if enabled
        let bytecode_to_compile = if self.config.enable_lto && self.config.opt_level != OptLevel::O0
        {
            let mut lto_optimizer = lto::LtoOptimizer::new();
            lto_optimizer.analyze(bytecode)?;
            lto_optimizer.optimize(bytecode)?
        } else {
            bytecode.clone()
        };

        // Step 1: Generate machine code
        let machine_code = match self.config.arch {
            Architecture::X86_64 => {
                let mut codegen = codegen::x86_64::X86CodeGen::new();
                codegen.compile(&bytecode_to_compile)?
            }
            Architecture::ARM64 => {
                let mut codegen = codegen::arm64::Arm64CodeGen::new();
                codegen.compile(&bytecode_to_compile)?
            }
            Architecture::RISCV64 => {
                let mut codegen = codegen::riscv64::RiscVCodeGen::new();
                codegen.compile(&bytecode_to_compile)?
            }
        };

        // Step 2: Optimize if requested
        let optimized_code = if self.config.opt_level != OptLevel::O0 {
            optimizer::optimize(&machine_code, self.config.opt_level)?
        } else {
            machine_code
        };

        Ok(optimized_code)
    }

    /// Compile Matter bytecode to native executable file
    pub fn compile_to_executable<P: AsRef<Path>>(
        &self,
        bytecode: &Bytecode,
        output_path: P,
    ) -> Result<(), String> {
        // Step 1: Compile to machine code
        let machine_code = self.compile(bytecode)?;

        // Step 2: Link into executable
        match self.config.os {
            OperatingSystem::Windows => {
                linker::pe::link_pe(&machine_code, output_path)?;
            }
            OperatingSystem::Linux => {
                linker::elf::link_elf(&machine_code, output_path)?;
            }
            OperatingSystem::MacOS => {
                linker::macho::link_macho(&machine_code, output_path)?;
            }
        }

        Ok(())
    }
}

impl Default for NativeCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matter_bytecode::{Bytecode, Constant, Instruction};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_output(ext: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "matter_native_compile_{}_{}.{}",
            std::process::id(),
            stamp,
            ext
        ))
    }

    #[test]
    fn test_compiler_creation() {
        let compiler = NativeCompiler::new();
        assert_eq!(compiler.config.arch, Architecture::X86_64);
    }

    #[test]
    fn test_simple_program() {
        let mut bytecode = Bytecode::new();
        let const_id = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(const_id),
            Instruction::Print,
            Instruction::Halt,
        ];

        let compiler = NativeCompiler::new();
        let result = compiler.compile(&bytecode);

        // Should compile without errors
        assert!(result.is_ok());

        // Should produce non-empty machine code
        let machine_code = result.unwrap();
        assert!(!machine_code.is_empty());
    }

    #[test]
    fn test_compile_to_executable_writes_platform_binary() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::Halt];

        let config = CompileConfig::default();
        let ext = match config.os {
            OperatingSystem::Windows => "exe",
            OperatingSystem::Linux => "elf",
            OperatingSystem::MacOS => "macho",
        };
        let output = temp_output(ext);

        let compiler = NativeCompiler::with_config(config);
        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("native compiler should write executable");

        let bytes = fs::read(&output).expect("output executable should be readable");
        assert!(!bytes.is_empty(), "linked binary should not be empty");

        match compiler.config.os {
            OperatingSystem::Windows => assert_eq!(&bytes[0..2], b"MZ"),
            OperatingSystem::Linux => assert_eq!(&bytes[0..4], &[0x7F, b'E', b'L', b'F']),
            OperatingSystem::MacOS => assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]),
        }

        let _ = fs::remove_file(output);
    }

    #[test]
    fn test_compile_supports_arm64() {
        let mut bytecode = Bytecode::new();
        let c42 = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![Instruction::LoadConst(c42), Instruction::Halt];

        let compiler = NativeCompiler::with_config(CompileConfig {
            arch: Architecture::ARM64,
            ..CompileConfig::default()
        });

        let result = compiler.compile(&bytecode);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_compile_supports_riscv64() {
        let mut bytecode = Bytecode::new();
        let c42 = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![Instruction::LoadConst(c42), Instruction::Halt];

        let compiler = NativeCompiler::with_config(CompileConfig {
            arch: Architecture::RISCV64,
            ..CompileConfig::default()
        });

        let result = compiler.compile(&bytecode);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    fn lcg_next(state: &mut u64) -> u64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *state
    }

    fn fuzz_program(seed: u64, steps: usize) -> Bytecode {
        let mut bytecode = Bytecode::new();
        let mut state = seed;
        let mut stack_depth = 0usize;

        for _ in 0..steps {
            let choice = (lcg_next(&mut state) % 8) as usize;
            match choice {
                0..=2 => {
                    let val = ((lcg_next(&mut state) % 200) as i64) - 100;
                    let c = bytecode.add_constant(Constant::Int(val));
                    bytecode.main_instructions.push(Instruction::LoadConst(c));
                    stack_depth += 1;
                }
                3 => {
                    let c = bytecode.add_constant(Constant::Bool((lcg_next(&mut state) & 1) == 1));
                    bytecode.main_instructions.push(Instruction::LoadConst(c));
                    stack_depth += 1;
                }
                4 if stack_depth >= 2 => {
                    bytecode.main_instructions.push(Instruction::Add);
                    stack_depth -= 1;
                }
                5 if stack_depth >= 2 => {
                    bytecode.main_instructions.push(Instruction::Sub);
                    stack_depth -= 1;
                }
                6 if stack_depth >= 2 => {
                    bytecode.main_instructions.push(Instruction::Eq);
                    stack_depth -= 1;
                }
                7 if stack_depth >= 1 => {
                    bytecode.main_instructions.push(Instruction::Pop);
                    stack_depth -= 1;
                }
                _ => {}
            }
        }

        while stack_depth > 0 {
            bytecode.main_instructions.push(Instruction::Pop);
            stack_depth -= 1;
        }
        bytecode.main_instructions.push(Instruction::Halt);
        bytecode
    }

    #[test]
    fn test_fuzz_light_codegen_stability() {
        let compiler = NativeCompiler::with_config(CompileConfig {
            arch: Architecture::X86_64,
            opt_level: OptLevel::O0,
            ..CompileConfig::default()
        });

        for seed in 1u64..=100 {
            let program = fuzz_program(seed, 40);
            let result = compiler.compile(&program);
            assert!(
                result.is_ok(),
                "seed {} should compile: {:?}",
                seed,
                result.err()
            );
            let code = result.unwrap();
            assert!(
                !code.is_empty(),
                "seed {} produced empty machine code",
                seed
            );
        }
    }
}
