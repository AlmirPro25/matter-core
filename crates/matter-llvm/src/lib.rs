//! Matter Core - LLVM Backend
//!
//! The real LLVM backend is behind the `llvm-sys` feature because it requires a
//! local LLVM 17 development installation. The default build keeps the wider
//! workspace testable on machines that only need the bytecode VM and native
//! experimental backend.

use matter_bytecode::Bytecode;

#[cfg(feature = "llvm-sys")]
mod real;

#[cfg(feature = "llvm-sys")]
pub use real::*;

#[cfg(not(feature = "llvm-sys"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    None,
    Less,
    Default,
    Aggressive,
}

#[cfg(not(feature = "llvm-sys"))]
pub fn parse_opt_level(level: &str) -> Result<OptLevel, String> {
    match level {
        "0" | "O0" | "none" => Ok(OptLevel::None),
        "1" | "O1" | "less" => Ok(OptLevel::Less),
        "2" | "O2" | "default" => Ok(OptLevel::Default),
        "3" | "O3" | "aggressive" => Ok(OptLevel::Aggressive),
        other => Err(format!("Invalid LLVM optimization level: {}", other)),
    }
}

#[cfg(not(feature = "llvm-sys"))]
pub fn get_llvm_ir(_bytecode: &Bytecode) -> Result<String, String> {
    Err(llvm_unavailable())
}

#[cfg(not(feature = "llvm-sys"))]
pub fn compile_to_native(_bytecode: &Bytecode, _output: &str) -> Result<(), String> {
    Err(llvm_unavailable())
}

#[cfg(not(feature = "llvm-sys"))]
pub fn compile_to_native_with_opt(
    _bytecode: &Bytecode,
    _output: &str,
    _opt_level: OptLevel,
) -> Result<(), String> {
    Err(llvm_unavailable())
}

#[cfg(not(feature = "llvm-sys"))]
fn llvm_unavailable() -> String {
    "LLVM backend unavailable: rebuild matter-llvm with feature `llvm-sys` and LLVM 17 installed"
        .to_string()
}
