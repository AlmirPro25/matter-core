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
        self.compile_internal(bytecode, false)
    }

    fn compile_internal(
        &self,
        bytecode: &Bytecode,
        standalone_executable: bool,
    ) -> Result<Vec<u8>, String> {
        // Step 0: Apply LTO if enabled
        let bytecode_to_compile = if self.config.enable_lto
            && self.config.opt_level != OptLevel::O0
            && !standalone_executable
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
                let mut codegen = if standalone_executable {
                    codegen::x86_64::X86CodeGen::new_standalone_executable()
                } else {
                    codegen::x86_64::X86CodeGen::new()
                };
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
        let machine_code = self.compile_internal(bytecode, true)?;

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
    use matter_bytecode::{Bytecode, Constant, Function, Instruction};
    use std::fs;
    #[cfg(windows)]
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_OUTPUT_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_output(ext: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let counter = TEMP_OUTPUT_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "matter_native_compile_{}_{}_{}.{}",
            std::process::id(),
            stamp,
            counter,
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

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_prints_integer_output() {
        let mut bytecode = Bytecode::new();
        let c0 = bytecode.add_constant(Constant::Int(0));
        let c7 = bytecode.add_constant(Constant::Int(7));
        let c17 = bytecode.add_constant(Constant::Int(17));
        let c5 = bytecode.add_constant(Constant::Int(5));
        let c42 = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c0),
            Instruction::LoadConst(c7),
            Instruction::Sub,
            Instruction::Print,
            Instruction::LoadConst(c17),
            Instruction::LoadConst(c5),
            Instruction::Mod,
            Instruction::Print,
            Instruction::LoadConst(c42),
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });
        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("native compiler should write executable with print support");

        let run = Command::new(&output)
            .output()
            .expect("generated executable should run");
        assert!(
            run.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run.status.code(),
            String::from_utf8_lossy(&run.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run.stdout), "-7\n2\n42\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_empty_struct_allocation() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![
            Instruction::NewStruct("Point".to_string(), 0),
            Instruction::StoreGlobal("p".to_string()),
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support empty struct allocation");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_prints_string_output() {
        let mut bytecode = Bytecode::new();
        let c = bytecode.add_constant(Constant::String("hello".to_string()));
        bytecode.main_instructions = vec![Instruction::LoadConst(c), Instruction::Print];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should compile string constants");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "hello\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_uses_named_main_when_top_level_is_empty() {
        let mut bytecode = Bytecode::new();
        let c = bytecode.add_constant(Constant::String("named main".to_string()));
        bytecode.main_instructions = vec![Instruction::Halt];
        bytecode.functions.insert(
            "main".to_string(),
            Function {
                name: "main".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::LoadConst(c),
                    Instruction::Print,
                    Instruction::Return,
                ],
            },
        );

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("native compiler should use named main as executable entry");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "named main\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_compares_string_contents() {
        let mut bytecode = Bytecode::new();
        let left = bytecode.add_constant(Constant::String("matter".to_string()));
        let right_same = bytecode.add_constant(Constant::String("matter".to_string()));
        let right_diff = bytecode.add_constant(Constant::String("core".to_string()));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(left),
            Instruction::LoadConst(right_same),
            Instruction::Eq,
            Instruction::Print,
            Instruction::LoadConst(left),
            Instruction::LoadConst(right_diff),
            Instruction::Eq,
            Instruction::Print,
            Instruction::LoadConst(left),
            Instruction::StoreGlobal("name".to_string()),
            Instruction::LoadGlobal("name".to_string()),
            Instruction::LoadConst(right_same),
            Instruction::NotEq,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should compare string contents");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "1\n0\n0\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_preserves_string_function_return_kind() {
        let mut bytecode = Bytecode::new();
        let returned = bytecode.add_constant(Constant::String("native label".to_string()));
        let expected = bytecode.add_constant(Constant::String("native label".to_string()));
        bytecode.main_instructions = vec![Instruction::Halt];
        bytecode.functions.insert(
            "label".to_string(),
            Function {
                name: "label".to_string(),
                param_count: 0,
                instructions: vec![Instruction::LoadConst(returned), Instruction::Return],
            },
        );
        bytecode.functions.insert(
            "main".to_string(),
            Function {
                name: "main".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::CallNamed {
                        name: "label".to_string(),
                        arg_count: 0,
                    },
                    Instruction::Print,
                    Instruction::CallNamed {
                        name: "label".to_string(),
                        arg_count: 0,
                    },
                    Instruction::LoadConst(expected),
                    Instruction::Eq,
                    Instruction::Print,
                    Instruction::Return,
                ],
            },
        );

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should preserve string return kinds");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run_output.stdout),
            "native label\n1\n"
        );

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_preserves_string_parameter_kind() {
        let mut bytecode = Bytecode::new();
        let arg = bytecode.add_constant(Constant::String("param text".to_string()));
        let expected = bytecode.add_constant(Constant::String("param text".to_string()));
        bytecode.main_instructions = vec![Instruction::Halt];
        bytecode.functions.insert(
            "echo".to_string(),
            Function {
                name: "echo".to_string(),
                param_count: 1,
                instructions: vec![
                    Instruction::LoadParam(0),
                    Instruction::Print,
                    Instruction::LoadParam(0),
                    Instruction::Return,
                ],
            },
        );
        bytecode.functions.insert(
            "main".to_string(),
            Function {
                name: "main".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::LoadConst(arg),
                    Instruction::CallNamed {
                        name: "echo".to_string(),
                        arg_count: 1,
                    },
                    Instruction::LoadConst(expected),
                    Instruction::Eq,
                    Instruction::Print,
                    Instruction::Return,
                ],
            },
        );

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should preserve string parameter kinds");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run_output.stdout),
            "param text\n1\n"
        );

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_propagates_nested_string_function_kinds() {
        let mut bytecode = Bytecode::new();
        let source_value = bytecode.add_constant(Constant::String("nested text".to_string()));
        let expected = bytecode.add_constant(Constant::String("nested text".to_string()));
        bytecode.main_instructions = vec![Instruction::Halt];
        bytecode.functions.insert(
            "source".to_string(),
            Function {
                name: "source".to_string(),
                param_count: 0,
                instructions: vec![Instruction::LoadConst(source_value), Instruction::Return],
            },
        );
        bytecode.functions.insert(
            "id".to_string(),
            Function {
                name: "id".to_string(),
                param_count: 1,
                instructions: vec![Instruction::LoadParam(0), Instruction::Return],
            },
        );
        bytecode.functions.insert(
            "main".to_string(),
            Function {
                name: "main".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::CallNamed {
                        name: "source".to_string(),
                        arg_count: 0,
                    },
                    Instruction::CallNamed {
                        name: "id".to_string(),
                        arg_count: 1,
                    },
                    Instruction::Print,
                    Instruction::CallNamed {
                        name: "source".to_string(),
                        arg_count: 0,
                    },
                    Instruction::CallNamed {
                        name: "id".to_string(),
                        arg_count: 1,
                    },
                    Instruction::LoadConst(expected),
                    Instruction::Eq,
                    Instruction::Print,
                    Instruction::Return,
                ],
            },
        );

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should propagate nested string function kinds");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run_output.stdout),
            "nested text\n1\n"
        );

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_list_len_and_index() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        let c30 = bytecode.add_constant(Constant::Int(30));
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::LoadConst(c30),
            Instruction::NewList(3),
            Instruction::StoreGlobal("xs".to_string()),
            Instruction::LoadGlobal("xs".to_string()),
            Instruction::ListLen,
            Instruction::Print,
            Instruction::LoadGlobal("xs".to_string()),
            Instruction::LoadConst(c1),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support list len and index");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "3\n20\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_string_list_index() {
        let mut bytecode = Bytecode::new();
        let ca = bytecode.add_constant(Constant::String("alpha".to_string()));
        let cb = bytecode.add_constant(Constant::String("beta".to_string()));
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ca),
            Instruction::LoadConst(cb),
            Instruction::NewList(2),
            Instruction::LoadConst(c1),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support string list indexing");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "beta\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_list_index_mutation() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        let c30 = bytecode.add_constant(Constant::Int(30));
        let c99 = bytecode.add_constant(Constant::Int(99));
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::LoadConst(c30),
            Instruction::NewList(3),
            Instruction::StoreGlobal("xs".to_string()),
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c99),
            Instruction::StoreIndexVar("xs".to_string()),
            Instruction::LoadGlobal("xs".to_string()),
            Instruction::LoadConst(c1),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support list index mutation");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "99\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_string_list_index_mutation() {
        let mut bytecode = Bytecode::new();
        let ca = bytecode.add_constant(Constant::String("alpha".to_string()));
        let cb = bytecode.add_constant(Constant::String("beta".to_string()));
        let cg = bytecode.add_constant(Constant::String("gamma".to_string()));
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ca),
            Instruction::LoadConst(cb),
            Instruction::NewList(2),
            Instruction::StoreGlobal("names".to_string()),
            Instruction::LoadConst(c1),
            Instruction::LoadConst(cg),
            Instruction::StoreIndexVar("names".to_string()),
            Instruction::LoadGlobal("names".to_string()),
            Instruction::LoadConst(c1),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support string list index mutation");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "gamma\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_list_push() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::NewList(1),
            Instruction::LoadConst(c20),
            Instruction::ListPush,
            Instruction::StoreGlobal("xs".to_string()),
            Instruction::LoadGlobal("xs".to_string()),
            Instruction::ListLen,
            Instruction::Print,
            Instruction::LoadGlobal("xs".to_string()),
            Instruction::LoadConst(c1),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support list push");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "2\n20\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_string_list_push_var() {
        let mut bytecode = Bytecode::new();
        let ca = bytecode.add_constant(Constant::String("alpha".to_string()));
        let cb = bytecode.add_constant(Constant::String("beta".to_string()));
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ca),
            Instruction::NewList(1),
            Instruction::StoreGlobal("names".to_string()),
            Instruction::LoadConst(cb),
            Instruction::ListPushVar("names".to_string()),
            Instruction::Pop,
            Instruction::LoadGlobal("names".to_string()),
            Instruction::ListLen,
            Instruction::Print,
            Instruction::LoadGlobal("names".to_string()),
            Instruction::LoadConst(c1),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support string list push var");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "2\nbeta\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_empty_list_string_push() {
        let mut bytecode = Bytecode::new();
        let value = bytecode.add_constant(Constant::String("ready".to_string()));
        let c0 = bytecode.add_constant(Constant::Int(0));
        bytecode.main_instructions = vec![
            Instruction::NewList(0),
            Instruction::LoadConst(value),
            Instruction::ListPush,
            Instruction::StoreGlobal("names".to_string()),
            Instruction::LoadGlobal("names".to_string()),
            Instruction::LoadConst(c0),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should infer string type after empty list push");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "ready\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_empty_list_string_push_var() {
        let mut bytecode = Bytecode::new();
        let value = bytecode.add_constant(Constant::String("ready".to_string()));
        let c0 = bytecode.add_constant(Constant::Int(0));
        bytecode.main_instructions = vec![
            Instruction::NewList(0),
            Instruction::StoreGlobal("names".to_string()),
            Instruction::LoadConst(value),
            Instruction::ListPushVar("names".to_string()),
            Instruction::Pop,
            Instruction::LoadGlobal("names".to_string()),
            Instruction::LoadConst(c0),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should infer string type after ListPushVar");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "ready\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_list_pop() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::NewList(2),
            Instruction::ListPop,
            Instruction::ListLen,
            Instruction::Print,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support list pop");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "1\n20\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_list_pop_var() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::NewList(2),
            Instruction::StoreGlobal("xs".to_string()),
            Instruction::ListPopVar("xs".to_string()),
            Instruction::Print,
            Instruction::LoadGlobal("xs".to_string()),
            Instruction::ListLen,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support list pop var");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "20\n1\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_string_list_pop_var() {
        let mut bytecode = Bytecode::new();
        let ca = bytecode.add_constant(Constant::String("alpha".to_string()));
        let cb = bytecode.add_constant(Constant::String("beta".to_string()));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ca),
            Instruction::LoadConst(cb),
            Instruction::NewList(2),
            Instruction::StoreGlobal("names".to_string()),
            Instruction::ListPopVar("names".to_string()),
            Instruction::Print,
            Instruction::LoadGlobal("names".to_string()),
            Instruction::ListLen,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support string list pop var");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "beta\n1\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_int_map_has() {
        let mut bytecode = Bytecode::new();
        let k1 = bytecode.add_constant(Constant::Int(1));
        let v100 = bytecode.add_constant(Constant::Int(100));
        let k2 = bytecode.add_constant(Constant::Int(2));
        let v200 = bytecode.add_constant(Constant::Int(200));
        let k3 = bytecode.add_constant(Constant::Int(3));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(k1),
            Instruction::LoadConst(v100),
            Instruction::LoadConst(k2),
            Instruction::LoadConst(v200),
            Instruction::NewMap(2),
            Instruction::StoreGlobal("m".to_string()),
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(k1),
            Instruction::MapHas,
            Instruction::Print,
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(k3),
            Instruction::MapHas,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support integer map has");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "1\n0\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_string_map_has() {
        let mut bytecode = Bytecode::new();
        let ka = bytecode.add_constant(Constant::String("alpha".to_string()));
        let v100 = bytecode.add_constant(Constant::Int(100));
        let kb = bytecode.add_constant(Constant::String("beta".to_string()));
        let v200 = bytecode.add_constant(Constant::Int(200));
        let kg = bytecode.add_constant(Constant::String("gamma".to_string()));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ka),
            Instruction::LoadConst(v100),
            Instruction::LoadConst(kb),
            Instruction::LoadConst(v200),
            Instruction::NewMap(2),
            Instruction::StoreGlobal("m".to_string()),
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(ka),
            Instruction::MapHas,
            Instruction::Print,
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(kg),
            Instruction::MapHas,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support string map has");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "1\n0\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_map_load_index() {
        let mut bytecode = Bytecode::new();
        let ka = bytecode.add_constant(Constant::String("alpha".to_string()));
        let v100 = bytecode.add_constant(Constant::Int(100));
        let v999 = bytecode.add_constant(Constant::Int(999));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ka),
            Instruction::LoadConst(v100),
            Instruction::NewMap(1),
            Instruction::StoreGlobal("m".to_string()),
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(ka),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::LoadConst(ka),
            Instruction::LoadConst(v999),
            Instruction::StoreIndexVar("m".to_string()),
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(ka),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support map LoadIndex");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "100\n999\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_string_map_load_index() {
        let mut bytecode = Bytecode::new();
        let ka = bytecode.add_constant(Constant::String("alpha".to_string()));
        let va = bytecode.add_constant(Constant::String("first".to_string()));
        let vb = bytecode.add_constant(Constant::String("second".to_string()));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ka),
            Instruction::LoadConst(va),
            Instruction::NewMap(1),
            Instruction::StoreGlobal("m".to_string()),
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(ka),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::LoadConst(ka),
            Instruction::LoadConst(vb),
            Instruction::StoreIndexVar("m".to_string()),
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(ka),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support string map LoadIndex");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run_output.stdout),
            "first\nsecond\n"
        );

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_map_keys_and_values() {
        let mut bytecode = Bytecode::new();
        let k1 = bytecode.add_constant(Constant::Int(1));
        let v100 = bytecode.add_constant(Constant::Int(100));
        let k2 = bytecode.add_constant(Constant::Int(2));
        let v200 = bytecode.add_constant(Constant::Int(200));
        let c0 = bytecode.add_constant(Constant::Int(0));
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(k1),
            Instruction::LoadConst(v100),
            Instruction::LoadConst(k2),
            Instruction::LoadConst(v200),
            Instruction::NewMap(2),
            Instruction::StoreGlobal("m".to_string()),
            Instruction::LoadGlobal("m".to_string()),
            Instruction::MapKeys,
            Instruction::StoreGlobal("keys".to_string()),
            Instruction::LoadGlobal("keys".to_string()),
            Instruction::ListLen,
            Instruction::Print,
            Instruction::LoadGlobal("keys".to_string()),
            Instruction::LoadConst(c0),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::LoadGlobal("m".to_string()),
            Instruction::MapValues,
            Instruction::StoreGlobal("values".to_string()),
            Instruction::LoadGlobal("values".to_string()),
            Instruction::ListLen,
            Instruction::Print,
            Instruction::LoadGlobal("values".to_string()),
            Instruction::LoadConst(c1),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support map keys and values");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run_output.stdout),
            "2\n1\n2\n200\n"
        );

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_string_map_keys() {
        let mut bytecode = Bytecode::new();
        let key = bytecode.add_constant(Constant::String("name".to_string()));
        let value = bytecode.add_constant(Constant::Int(7));
        let c0 = bytecode.add_constant(Constant::Int(0));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(key),
            Instruction::LoadConst(value),
            Instruction::NewMap(1),
            Instruction::MapKeys,
            Instruction::LoadConst(c0),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should preserve string map keys");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "name\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_string_map_values() {
        let mut bytecode = Bytecode::new();
        let key = bytecode.add_constant(Constant::Int(42));
        let value = bytecode.add_constant(Constant::String("answer".to_string()));
        let c0 = bytecode.add_constant(Constant::Int(0));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(key),
            Instruction::LoadConst(value),
            Instruction::NewMap(1),
            Instruction::MapValues,
            Instruction::LoadConst(c0),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should preserve string map values");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "answer\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_map_insert_mutation() {
        let mut bytecode = Bytecode::new();
        let ka = bytecode.add_constant(Constant::String("alpha".to_string()));
        let v100 = bytecode.add_constant(Constant::Int(100));
        let kb = bytecode.add_constant(Constant::String("beta".to_string()));
        let v200 = bytecode.add_constant(Constant::Int(200));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ka),
            Instruction::LoadConst(v100),
            Instruction::NewMap(1),
            Instruction::StoreGlobal("m".to_string()),
            Instruction::LoadConst(kb),
            Instruction::LoadConst(v200),
            Instruction::StoreIndexVar("m".to_string()),
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(kb),
            Instruction::MapHas,
            Instruction::Print,
            Instruction::LoadGlobal("m".to_string()),
            Instruction::MapKeys,
            Instruction::ListLen,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support map insert mutation");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "1\n2\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_map_update_mutation() {
        let mut bytecode = Bytecode::new();
        let ka = bytecode.add_constant(Constant::String("alpha".to_string()));
        let v100 = bytecode.add_constant(Constant::Int(100));
        let v999 = bytecode.add_constant(Constant::Int(999));
        let c0 = bytecode.add_constant(Constant::Int(0));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ka),
            Instruction::LoadConst(v100),
            Instruction::NewMap(1),
            Instruction::StoreGlobal("m".to_string()),
            Instruction::LoadConst(ka),
            Instruction::LoadConst(v999),
            Instruction::StoreIndexVar("m".to_string()),
            Instruction::LoadGlobal("m".to_string()),
            Instruction::MapKeys,
            Instruction::ListLen,
            Instruction::Print,
            Instruction::LoadGlobal("m".to_string()),
            Instruction::MapValues,
            Instruction::LoadConst(c0),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support map update mutation");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "1\n999\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_map_store_index_mutation() {
        let mut bytecode = Bytecode::new();
        let ka = bytecode.add_constant(Constant::String("alpha".to_string()));
        let v100 = bytecode.add_constant(Constant::Int(100));
        let kb = bytecode.add_constant(Constant::String("beta".to_string()));
        let v200 = bytecode.add_constant(Constant::Int(200));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ka),
            Instruction::LoadConst(v100),
            Instruction::NewMap(1),
            Instruction::StoreGlobal("m".to_string()),
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(kb),
            Instruction::LoadConst(v200),
            Instruction::StoreIndex,
            Instruction::LoadGlobal("m".to_string()),
            Instruction::LoadConst(kb),
            Instruction::MapHas,
            Instruction::Print,
            Instruction::LoadGlobal("m".to_string()),
            Instruction::MapKeys,
            Instruction::ListLen,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support map StoreIndex mutation");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "1\n2\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_struct_field_access() {
        let mut bytecode = Bytecode::new();
        let kx = bytecode.add_constant(Constant::String("x".to_string()));
        let vx = bytecode.add_constant(Constant::Int(10));
        let ky = bytecode.add_constant(Constant::String("y".to_string()));
        let vy = bytecode.add_constant(Constant::Int(20));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(kx),
            Instruction::LoadConst(vx),
            Instruction::LoadConst(ky),
            Instruction::LoadConst(vy),
            Instruction::NewStruct("Point".to_string(), 2),
            Instruction::LoadField("y".to_string()),
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support struct field access");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "20\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_struct_field_mutation() {
        let mut bytecode = Bytecode::new();
        let kx = bytecode.add_constant(Constant::String("x".to_string()));
        let vx = bytecode.add_constant(Constant::Int(10));
        let ky = bytecode.add_constant(Constant::String("y".to_string()));
        let vy = bytecode.add_constant(Constant::Int(20));
        let v99 = bytecode.add_constant(Constant::Int(99));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(kx),
            Instruction::LoadConst(vx),
            Instruction::LoadConst(ky),
            Instruction::LoadConst(vy),
            Instruction::NewStruct("Point".to_string(), 2),
            Instruction::StoreGlobal("p".to_string()),
            Instruction::LoadConst(v99),
            Instruction::StoreFieldVar {
                target: "p".to_string(),
                field: "y".to_string(),
            },
            Instruction::LoadGlobal("p".to_string()),
            Instruction::LoadField("y".to_string()),
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support struct field mutation");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "99\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_standalone_string_struct_field_access_and_mutation() {
        let mut bytecode = Bytecode::new();
        let kname = bytecode.add_constant(Constant::String("name".to_string()));
        let alice = bytecode.add_constant(Constant::String("Alice".to_string()));
        let bob = bytecode.add_constant(Constant::String("Bob".to_string()));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(kname),
            Instruction::LoadConst(alice),
            Instruction::NewStruct("User".to_string(), 1),
            Instruction::StoreGlobal("user".to_string()),
            Instruction::LoadGlobal("user".to_string()),
            Instruction::LoadField("name".to_string()),
            Instruction::Print,
            Instruction::LoadConst(bob),
            Instruction::StoreFieldVar {
                target: "user".to_string(),
                field: "name".to_string(),
            },
            Instruction::LoadGlobal("user".to_string()),
            Instruction::LoadField("name".to_string()),
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support string struct fields");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "Alice\nBob\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_preserves_string_field_shape_from_function_return() {
        let mut bytecode = Bytecode::new();
        let kname = bytecode.add_constant(Constant::String("name".to_string()));
        let alice = bytecode.add_constant(Constant::String("Alice".to_string()));
        bytecode.functions.insert(
            "make_user".to_string(),
            Function {
                name: "make_user".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::LoadConst(kname),
                    Instruction::LoadConst(alice),
                    Instruction::NewStruct("User".to_string(), 1),
                    Instruction::Return,
                ],
            },
        );
        bytecode.main_instructions = vec![
            Instruction::CallNamed {
                name: "make_user".to_string(),
                arg_count: 0,
            },
            Instruction::LoadField("name".to_string()),
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should preserve returned struct field shape");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "Alice\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_preserves_string_map_shape_from_function_return() {
        let mut bytecode = Bytecode::new();
        let key = bytecode.add_constant(Constant::String("status".to_string()));
        let ready = bytecode.add_constant(Constant::String("ready".to_string()));
        bytecode.functions.insert(
            "make_status".to_string(),
            Function {
                name: "make_status".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::LoadConst(key),
                    Instruction::LoadConst(ready),
                    Instruction::NewMap(1),
                    Instruction::Return,
                ],
            },
        );
        bytecode.main_instructions = vec![
            Instruction::CallNamed {
                name: "make_status".to_string(),
                arg_count: 0,
            },
            Instruction::LoadConst(key),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should preserve returned map shape");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "ready\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_preserves_nested_struct_shape_from_function_return() {
        let mut bytecode = Bytecode::new();
        let kname = bytecode.add_constant(Constant::String("name".to_string()));
        let alice = bytecode.add_constant(Constant::String("Alice".to_string()));
        let kuser = bytecode.add_constant(Constant::String("user".to_string()));
        bytecode.functions.insert(
            "make_profile".to_string(),
            Function {
                name: "make_profile".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::LoadConst(kuser),
                    Instruction::LoadConst(kname),
                    Instruction::LoadConst(alice),
                    Instruction::NewStruct("User".to_string(), 1),
                    Instruction::NewStruct("Profile".to_string(), 1),
                    Instruction::Return,
                ],
            },
        );
        bytecode.main_instructions = vec![
            Instruction::CallNamed {
                name: "make_profile".to_string(),
                arg_count: 0,
            },
            Instruction::LoadField("user".to_string()),
            Instruction::LoadField("name".to_string()),
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should preserve nested returned struct shape");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "Alice\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_preserves_nested_map_shape_from_function_return() {
        let mut bytecode = Bytecode::new();
        let kuser = bytecode.add_constant(Constant::String("user".to_string()));
        let kname = bytecode.add_constant(Constant::String("name".to_string()));
        let alice = bytecode.add_constant(Constant::String("Alice".to_string()));
        bytecode.functions.insert(
            "make_payload".to_string(),
            Function {
                name: "make_payload".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::LoadConst(kuser),
                    Instruction::LoadConst(kname),
                    Instruction::LoadConst(alice),
                    Instruction::NewMap(1),
                    Instruction::NewMap(1),
                    Instruction::Return,
                ],
            },
        );
        bytecode.main_instructions = vec![
            Instruction::CallNamed {
                name: "make_payload".to_string(),
                arg_count: 0,
            },
            Instruction::LoadConst(kuser),
            Instruction::LoadIndex,
            Instruction::LoadConst(kname),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should preserve nested returned map shape");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "Alice\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_named_main_mixes_nested_struct_and_map_returns() {
        let mut bytecode = Bytecode::new();
        let kuser = bytecode.add_constant(Constant::String("user".to_string()));
        let kname = bytecode.add_constant(Constant::String("name".to_string()));
        let alice = bytecode.add_constant(Constant::String("Alice".to_string()));
        let bob = bytecode.add_constant(Constant::String("Bob".to_string()));
        bytecode.functions.insert(
            "make_profile".to_string(),
            Function {
                name: "make_profile".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::LoadConst(kuser),
                    Instruction::LoadConst(kname),
                    Instruction::LoadConst(alice),
                    Instruction::NewStruct("User".to_string(), 1),
                    Instruction::NewStruct("Profile".to_string(), 1),
                    Instruction::Return,
                ],
            },
        );
        bytecode.functions.insert(
            "make_payload".to_string(),
            Function {
                name: "make_payload".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::LoadConst(kuser),
                    Instruction::LoadConst(kname),
                    Instruction::LoadConst(bob),
                    Instruction::NewMap(1),
                    Instruction::NewMap(1),
                    Instruction::Return,
                ],
            },
        );
        bytecode.functions.insert(
            "main".to_string(),
            Function {
                name: "main".to_string(),
                param_count: 0,
                instructions: vec![
                    Instruction::CallNamed {
                        name: "make_profile".to_string(),
                        arg_count: 0,
                    },
                    Instruction::StoreLocal("profile".to_string()),
                    Instruction::LoadGlobal("profile".to_string()),
                    Instruction::LoadField("user".to_string()),
                    Instruction::LoadField("name".to_string()),
                    Instruction::Print,
                    Instruction::CallNamed {
                        name: "make_payload".to_string(),
                        arg_count: 0,
                    },
                    Instruction::StoreLocal("payload".to_string()),
                    Instruction::LoadGlobal("payload".to_string()),
                    Instruction::LoadConst(kuser),
                    Instruction::LoadIndex,
                    Instruction::LoadConst(kname),
                    Instruction::LoadIndex,
                    Instruction::Print,
                    Instruction::Return,
                ],
            },
        );
        bytecode.main_instructions = vec![Instruction::Halt];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support named main with mixed returns");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "Alice\nBob\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_extended_math_backend_calls() {
        let mut bytecode = Bytecode::new();
        let neg7 = bytecode.add_constant(Constant::Int(-7));
        let nine = bytecode.add_constant(Constant::Int(9));
        let neg3 = bytecode.add_constant(Constant::Int(-3));
        let ten = bytecode.add_constant(Constant::Int(10));
        let three = bytecode.add_constant(Constant::Int(3));
        let twelve = bytecode.add_constant(Constant::Int(12));
        let zero = bytecode.add_constant(Constant::Int(0));
        let five = bytecode.add_constant(Constant::Int(5));
        let two = bytecode.add_constant(Constant::Int(2));
        let ten_exp = bytecode.add_constant(Constant::Int(10));
        let sqrt_input = bytecode.add_constant(Constant::Int(144));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(neg7),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "abs".to_string(),
                arg_count: 1,
            },
            Instruction::Print,
            Instruction::LoadConst(nine),
            Instruction::LoadConst(neg3),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "min".to_string(),
                arg_count: 2,
            },
            Instruction::Print,
            Instruction::LoadConst(nine),
            Instruction::LoadConst(neg3),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "max".to_string(),
                arg_count: 2,
            },
            Instruction::Print,
            Instruction::LoadConst(ten),
            Instruction::LoadConst(three),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "mod".to_string(),
                arg_count: 2,
            },
            Instruction::Print,
            Instruction::LoadConst(twelve),
            Instruction::LoadConst(zero),
            Instruction::LoadConst(five),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "clamp".to_string(),
                arg_count: 3,
            },
            Instruction::Print,
            Instruction::LoadConst(two),
            Instruction::LoadConst(ten_exp),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "pow".to_string(),
                arg_count: 2,
            },
            Instruction::Print,
            Instruction::LoadConst(sqrt_input),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "sqrt".to_string(),
                arg_count: 1,
            },
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support extended math backend calls");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&run_output.stdout),
            "7\n-3\n9\n1\n5\n1024\n12\n"
        );

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_string_len_backend_call() {
        let mut bytecode = Bytecode::new();
        let matter = bytecode.add_constant(Constant::String("Matter".to_string()));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(matter),
            Instruction::BackendCall {
                backend: "string".to_string(),
                method: "len".to_string(),
                arg_count: 1,
            },
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support string.len backend call");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "6\n");

        let _ = fs::remove_file(output);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_executable_supports_string_contains_backend_call() {
        let mut bytecode = Bytecode::new();
        let matter_core = bytecode.add_constant(Constant::String("Matter Core".to_string()));
        let core = bytecode.add_constant(Constant::String("Core".to_string()));
        let runtime = bytecode.add_constant(Constant::String("runtime".to_string()));
        let empty = bytecode.add_constant(Constant::String(String::new()));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(matter_core),
            Instruction::LoadConst(core),
            Instruction::BackendCall {
                backend: "string".to_string(),
                method: "contains".to_string(),
                arg_count: 2,
            },
            Instruction::Print,
            Instruction::LoadConst(matter_core),
            Instruction::LoadConst(runtime),
            Instruction::BackendCall {
                backend: "string".to_string(),
                method: "contains".to_string(),
                arg_count: 2,
            },
            Instruction::Print,
            Instruction::LoadConst(matter_core),
            Instruction::LoadConst(empty),
            Instruction::BackendCall {
                backend: "string".to_string(),
                method: "contains".to_string(),
                arg_count: 2,
            },
            Instruction::Print,
            Instruction::Halt,
        ];

        let output = temp_output("exe");
        let compiler = NativeCompiler::with_config(CompileConfig {
            os: OperatingSystem::Windows,
            arch: Architecture::X86_64,
            opt_level: OptLevel::O3,
            debug_info: false,
            enable_lto: true,
        });

        compiler
            .compile_to_executable(&bytecode, &output)
            .expect("standalone native executable should support string.contains backend call");

        let run_output = Command::new(&output)
            .output()
            .expect("failed to run generated executable");
        assert!(
            run_output.status.success(),
            "generated executable failed: status={:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&run_output.stdout), "1\n0\n1\n");

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

#[cfg(test)]
mod make_closure_native_surface {
    use matter_bytecode::Instruction;

    #[test]
    fn make_closure_instruction_variant_exists() {
        let i = Instruction::MakeClosure {
            func_name: "f".into(),
            capture_names: vec![],
        };
        assert!(matches!(i, Instruction::MakeClosure { .. }));
    }
}
