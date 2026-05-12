//! Matter Core - WebAssembly Target
//!
//! This crate provides WebAssembly bindings for Matter Core,
//! allowing Matter programs to run in web browsers.

use matter_bytecode::BytecodeBuilder;
use matter_parser::Parser;
use matter_runtime::Runtime;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Initialize panic hook for better error messages in browser console
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Result of executing Matter code
#[derive(Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// Matter Core WASM Runtime
#[wasm_bindgen]
pub struct MatterWasm {
    output_buffer: Vec<String>,
}

#[wasm_bindgen]
impl MatterWasm {
    /// Create a new Matter WASM runtime
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            output_buffer: Vec::new(),
        }
    }

    /// Execute Matter source code
    #[wasm_bindgen]
    pub fn execute(&mut self, source: &str) -> JsValue {
        self.output_buffer.clear();

        let result = self.execute_internal(source);

        Self::result_to_js_value(&result)
    }

    /// Compile Matter source to bytecode (JSON format)
    #[wasm_bindgen]
    pub fn compile(&self, source: &str) -> JsValue {
        match self.compile_internal(source) {
            Ok(bytecode_json) => {
                let result = ExecutionResult {
                    success: true,
                    output: bytecode_json,
                    error: None,
                };
                Self::result_to_js_value(&result)
            }
            Err(error) => {
                let result = ExecutionResult {
                    success: false,
                    output: String::new(),
                    error: Some(error),
                };
                Self::result_to_js_value(&result)
            }
        }
    }

    /// Get the output buffer
    #[wasm_bindgen]
    pub fn get_output(&self) -> String {
        self.output_buffer.join("\n")
    }

    /// Clear the output buffer
    #[wasm_bindgen]
    pub fn clear_output(&mut self) {
        self.output_buffer.clear();
    }

    /// Get version information
    #[wasm_bindgen]
    pub fn version() -> String {
        "Matter Core v0.9.0 (WASM)".to_string()
    }
}

impl MatterWasm {
    fn result_to_js_value(result: &ExecutionResult) -> JsValue {
        serde_wasm_bindgen::to_value(result).unwrap_or_else(|error| {
            JsValue::from_str(
                format!(
                    "{{\"success\":false,\"output\":\"\",\"error\":\"WASM serialization error: {}\"}}",
                    error
                )
                .as_str(),
            )
        })
    }

    fn execute_internal(&mut self, source: &str) -> ExecutionResult {
        // Parse
        let mut parser = Parser::from_source(source);
        let program = match parser.parse() {
            Ok(prog) => prog,
            Err(e) => {
                return ExecutionResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Parse error: {}", e)),
                };
            }
        };

        // Compile
        let builder = BytecodeBuilder::new();
        let bytecode = match builder.build_checked(&program) {
            Ok(bc) => bc,
            Err(e) => {
                return ExecutionResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Compile error: {}", e)),
                };
            }
        };

        // Execute
        let mut runtime = Runtime::new(bytecode);
        runtime.set_stdout_enabled(false);

        match runtime.run() {
            Ok(_) => {
                // Capture print output
                let output_lines = runtime.take_output();
                let output = output_lines.join("\n");
                self.output_buffer.extend(output_lines);

                ExecutionResult {
                    success: true,
                    output,
                    error: None,
                }
            }
            Err(e) => ExecutionResult {
                success: false,
                output: String::new(),
                error: Some(format!("Runtime error: {}", e)),
            },
        }
    }

    fn compile_internal(&self, source: &str) -> Result<String, String> {
        // Parse
        let mut parser = Parser::from_source(source);
        let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

        // Compile
        let builder = BytecodeBuilder::new();
        let bytecode = builder
            .build_checked(&program)
            .map_err(|e| format!("Compile error: {}", e))?;

        // Serialize to JSON
        serde_json::to_string_pretty(&bytecode).map_err(|e| format!("Serialization error: {}", e))
    }
}

impl Default for MatterWasm {
    fn default() -> Self {
        Self::new()
    }
}

/// Standalone function to execute Matter code
#[wasm_bindgen]
pub fn execute_matter(source: &str) -> JsValue {
    let mut runtime = MatterWasm::new();
    runtime.execute(source)
}

/// Standalone function to compile Matter code
#[wasm_bindgen]
pub fn compile_matter(source: &str) -> JsValue {
    let runtime = MatterWasm::new();
    runtime.compile(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_runtime_creation() {
        let runtime = MatterWasm::new();
        assert_eq!(runtime.output_buffer.len(), 0);
    }

    #[test]
    fn test_version() {
        let version = MatterWasm::version();
        assert!(version.contains("Matter Core"));
        assert!(version.contains("WASM"));
    }

    #[test]
    fn test_simple_execution() {
        let mut runtime = MatterWasm::new();
        let source = "let x = 10 + 20";
        let result = runtime.execute_internal(source);
        assert!(result.success);
    }

    #[test]
    fn test_compilation() {
        let runtime = MatterWasm::new();
        let source = "let x = 42";
        let result = runtime.compile_internal(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_buffer() {
        let mut runtime = MatterWasm::new();
        assert_eq!(runtime.get_output(), "");
        runtime.clear_output();
        assert_eq!(runtime.get_output(), "");
    }
}
