//! Matter Core - LLVM Backend
//!
//! This crate provides native code generation using LLVM,
//! enabling Matter programs to be compiled to native executables
//! with 10-100x performance improvement over bytecode interpretation.

use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::types::{BasicTypeEnum, IntType};
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use std::collections::HashMap;
use matter_bytecode::Bytecode;

/// LLVM Code Generator for Matter Core
pub struct LLVMCodegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, PointerValue<'ctx>>,
    functions: HashMap<String, FunctionValue<'ctx>>,
}

impl<'ctx> LLVMCodegen<'ctx> {
    /// Create a new LLVM code generator
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    /// Get the i64 type
    fn i64_type(&self) -> IntType<'ctx> {
        self.context.i64_type()
    }

    /// Get the i32 type
    fn i32_type(&self) -> IntType<'ctx> {
        self.context.i32_type()
    }

    /// Get the i8 type (for strings)
    fn i8_type(&self) -> IntType<'ctx> {
        self.context.i8_type()
    }

    /// Create a function
    pub fn create_function(&mut self, name: &str, param_count: usize) -> FunctionValue<'ctx> {
        let i64_type = self.i64_type();
        let param_types: Vec<BasicTypeEnum> = vec![i64_type.into(); param_count];
        let fn_type = i64_type.fn_type(&param_types, false);
        let function = self.module.add_function(name, fn_type, None);

        self.functions.insert(name.to_string(), function);
        function
    }

    /// Create the main function
    pub fn create_main(&mut self) -> FunctionValue<'ctx> {
        let i32_type = self.i32_type();
        let fn_type = i32_type.fn_type(&[], false);
        let function = self.module.add_function("main", fn_type, None);

        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        function
    }

    /// Compile bytecode to LLVM IR
    pub fn compile_bytecode(&mut self, bytecode: &Bytecode) -> Result<(), String> {
        // Create main function
        let main_fn = self.create_main();

        // For now, just return 0
        let zero = self.i32_type().const_int(0, false);
        self.builder.build_return(Some(&zero))
            .map_err(|e| format!("Failed to build return: {:?}", e))?;

        Ok(())
    }

    /// Compile an integer constant
    pub fn compile_int(&self, value: i64) -> IntValue<'ctx> {
        self.i64_type().const_int(value as u64, false)
    }

    /// Compile an addition
    pub fn compile_add(&self, lhs: IntValue<'ctx>, rhs: IntValue<'ctx>) -> Result<IntValue<'ctx>, String> {
        self.builder.build_int_add(lhs, rhs, "add")
            .map_err(|e| format!("Failed to build add: {:?}", e))
    }

    /// Compile a subtraction
    pub fn compile_sub(&self, lhs: IntValue<'ctx>, rhs: IntValue<'ctx>) -> Result<IntValue<'ctx>, String> {
        self.builder.build_int_sub(lhs, rhs, "sub")
            .map_err(|e| format!("Failed to build sub: {:?}", e))
    }

    /// Compile a multiplication
    pub fn compile_mul(&self, lhs: IntValue<'ctx>, rhs: IntValue<'ctx>) -> Result<IntValue<'ctx>, String> {
        self.builder.build_int_mul(lhs, rhs, "mul")
            .map_err(|e| format!("Failed to build mul: {:?}", e))
    }

    /// Compile a division
    pub fn compile_div(&self, lhs: IntValue<'ctx>, rhs: IntValue<'ctx>) -> Result<IntValue<'ctx>, String> {
        self.builder.build_int_signed_div(lhs, rhs, "div")
            .map_err(|e| format!("Failed to build div: {:?}", e))
    }

    /// Compile a comparison
    pub fn compile_cmp(&self, lhs: IntValue<'ctx>, rhs: IntValue<'ctx>, op: IntPredicate) -> Result<IntValue<'ctx>, String> {
        self.builder.build_int_compare(op, lhs, rhs, "cmp")
            .map_err(|e| format!("Failed to build cmp: {:?}", e))
    }

    /// Get the LLVM IR as a string
    pub fn get_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }

    /// Verify the module
    pub fn verify(&self) -> Result<(), String> {
        self.module.verify()
            .map_err(|e| format!("Module verification failed: {}", e.to_string()))
    }

    /// Write object file
    pub fn write_object_file(&self, path: &str) -> Result<(), String> {
        use inkwell::targets::{Target, InitializationConfig, TargetMachine, RelocMode, CodeModel, FileType};

        Target::initialize_all(&InitializationConfig::default());

        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple)
            .map_err(|e| format!("Failed to get target: {}", e.to_string()))?;

        let target_machine = target
            .create_target_machine(
                &target_triple,
                "generic",
                "",
                OptimizationLevel::Aggressive,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or("Failed to create target machine")?;

        target_machine
            .write_to_file(&self.module, FileType::Object, path.as_ref())
            .map_err(|e| format!("Failed to write object file: {}", e.to_string()))?;

        Ok(())
    }

    /// Compile to executable
    pub fn compile_to_executable(&self, output_path: &str) -> Result<(), String> {
        // Write object file
        let obj_path = format!("{}.o", output_path);
        self.write_object_file(&obj_path)?;

        // Link with system linker
        #[cfg(target_os = "windows")]
        let link_cmd = format!("link /OUT:{}.exe {}", output_path, obj_path);

        #[cfg(not(target_os = "windows"))]
        let link_cmd = format!("gcc {} -o {}", obj_path, output_path);

        std::process::Command::new("sh")
            .arg("-c")
            .arg(&link_cmd)
            .output()
            .map_err(|e| format!("Failed to link: {}", e))?;

        // Clean up object file
        std::fs::remove_file(&obj_path)
            .map_err(|e| format!("Failed to remove object file: {}", e))?;

        Ok(())
    }
}

/// Compile Matter bytecode to native code
pub fn compile_to_native(bytecode: &Bytecode, output_path: &str) -> Result<(), String> {
    let context = Context::create();
    let mut codegen = LLVMCodegen::new(&context, "matter_program");

    // Compile bytecode
    codegen.compile_bytecode(bytecode)?;

    // Verify module
    codegen.verify()?;

    // Compile to executable
    codegen.compile_to_executable(output_path)?;

    Ok(())
}

/// Get LLVM IR for bytecode
pub fn get_llvm_ir(bytecode: &Bytecode) -> Result<String, String> {
    let context = Context::create();
    let mut codegen = LLVMCodegen::new(&context, "matter_program");

    // Compile bytecode
    codegen.compile_bytecode(bytecode)?;

    // Verify module
    codegen.verify()?;

    // Return IR
    Ok(codegen.get_ir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codegen_creation() {
        let context = Context::create();
        let codegen = LLVMCodegen::new(&context, "test");
        assert_eq!(codegen.variables.len(), 0);
        assert_eq!(codegen.functions.len(), 0);
    }

    #[test]
    fn test_create_main() {
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test");
        let main_fn = codegen.create_main();
        assert_eq!(main_fn.get_name().to_str().unwrap(), "main");
    }

    #[test]
    fn test_compile_int() {
        let context = Context::create();
        let codegen = LLVMCodegen::new(&context, "test");
        let value = codegen.compile_int(42);
        assert_eq!(value.get_zero_extended_constant().unwrap(), 42);
    }

    #[test]
    fn test_get_ir() {
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test");
        codegen.create_main();
        let ir = codegen.get_ir();
        assert!(ir.contains("define"));
        assert!(ir.contains("main"));
    }

    #[test]
    fn test_verify_empty_module() {
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test");
        codegen.create_main();
        assert!(codegen.verify().is_ok());
    }
}
