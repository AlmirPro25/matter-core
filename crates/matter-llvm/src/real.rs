//! Matter Core - LLVM Backend
//!
//! This crate provides native code generation using LLVM,
//! enabling Matter programs to be compiled to native executables
//! with 10-100x performance improvement over bytecode interpretation.

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{BasicTypeEnum, IntType};
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;
use matter_bytecode::Bytecode;
use std::collections::HashMap;

/// Loop context for break/continue support
struct LoopContext<'ctx> {
    continue_block: inkwell::basic_block::BasicBlock<'ctx>,
    break_block: inkwell::basic_block::BasicBlock<'ctx>,
}

/// LLVM Code Generator for Matter Core
pub struct LLVMCodegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, PointerValue<'ctx>>,
    functions: HashMap<String, FunctionValue<'ctx>>,

    // Virtual stack for Matter values
    stack: Vec<IntValue<'ctx>>,

    // Basic blocks for control flow
    basic_blocks: HashMap<usize, inkwell::basic_block::BasicBlock<'ctx>>,

    // Loop context stack for break/continue
    loop_stack: Vec<LoopContext<'ctx>>,
}

impl<'ctx> LLVMCodegen<'ctx> {
    /// Create a new LLVM code generator
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        let mut codegen = Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            functions: HashMap::new(),
            stack: Vec::new(),
            basic_blocks: HashMap::new(),
            loop_stack: Vec::new(),
        };

        codegen.declare_builtins();
        codegen
    }

    /// Declare native builtin C functions from the matter runtime
    fn declare_builtins(&mut self) {
        let i64_type = self.i64_type();
        let i8_ptr_type = self.i8_type().ptr_type(inkwell::AddressSpace::default());

        // matter_list_new(capacity: i64) -> *mut MatterList (we treat pointers as i64 in Matter for now, or just use i8_ptr)
        let list_new_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
        let fn_list_new = self
            .module
            .add_function("matter_list_new", list_new_type, None);
        self.functions
            .insert("matter_list_new".to_string(), fn_list_new);

        // matter_list_push(list_ptr: *mut MatterList, value: i64) -> bool (i1 or i64)
        let list_push_type = self
            .i32_type()
            .fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
        let fn_list_push = self
            .module
            .add_function("matter_list_push", list_push_type, None);
        self.functions
            .insert("matter_list_push".to_string(), fn_list_push);

        // matter_list_pop(list_ptr: *mut MatterList) -> i64
        let list_pop_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
        let fn_list_pop = self
            .module
            .add_function("matter_list_pop", list_pop_type, None);
        self.functions
            .insert("matter_list_pop".to_string(), fn_list_pop);

        // matter_list_len(list_ptr: *mut MatterList) -> i64
        let list_len_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
        let fn_list_len = self
            .module
            .add_function("matter_list_len", list_len_type, None);
        self.functions
            .insert("matter_list_len".to_string(), fn_list_len);

        // matter_list_get(list_ptr: *mut MatterList, index: i64) -> i64
        let list_get_type = i64_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
        let fn_list_get = self
            .module
            .add_function("matter_list_get", list_get_type, None);
        self.functions
            .insert("matter_list_get".to_string(), fn_list_get);

        // matter_list_set(list_ptr: *mut MatterList, index: i64, value: i64) -> bool
        let list_set_type = self.i32_type().fn_type(
            &[i8_ptr_type.into(), i64_type.into(), i64_type.into()],
            false,
        );
        let fn_list_set = self
            .module
            .add_function("matter_list_set", list_set_type, None);
        self.functions
            .insert("matter_list_set".to_string(), fn_list_set);

        // Map functions
        let map_new_type = i8_ptr_type.fn_type(&[], false);
        let fn_map_new = self
            .module
            .add_function("matter_map_new", map_new_type, None);
        self.functions
            .insert("matter_map_new".to_string(), fn_map_new);

        let map_insert_type = self.i32_type().fn_type(
            &[i8_ptr_type.into(), i64_type.into(), i64_type.into()],
            false,
        );
        let fn_map_insert = self
            .module
            .add_function("matter_map_insert", map_insert_type, None);
        self.functions
            .insert("matter_map_insert".to_string(), fn_map_insert);

        let map_has_type = self
            .i32_type()
            .fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
        let fn_map_has = self
            .module
            .add_function("matter_map_has", map_has_type, None);
        self.functions
            .insert("matter_map_has".to_string(), fn_map_has);

        let map_keys_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        let fn_map_keys = self
            .module
            .add_function("matter_map_keys", map_keys_type, None);
        self.functions
            .insert("matter_map_keys".to_string(), fn_map_keys);

        let map_values_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        let fn_map_values = self
            .module
            .add_function("matter_map_values", map_values_type, None);
        self.functions
            .insert("matter_map_values".to_string(), fn_map_values);

        // Generic collection functions
        let col_get_type = i64_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
        let fn_col_get = self
            .module
            .add_function("matter_collection_get", col_get_type, None);
        self.functions
            .insert("matter_collection_get".to_string(), fn_col_get);

        let col_set_type = self.i32_type().fn_type(
            &[i8_ptr_type.into(), i64_type.into(), i64_type.into()],
            false,
        );
        let fn_col_set = self
            .module
            .add_function("matter_collection_set", col_set_type, None);
        self.functions
            .insert("matter_collection_set".to_string(), fn_col_set);
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

    /// Compile a user-defined function
    pub fn compile_function(
        &mut self,
        name: &str,
        param_count: usize,
        instructions: &[matter_bytecode::Instruction],
        constants: &[matter_bytecode::Constant],
    ) -> Result<(), String> {
        // Create function
        let function = self.create_function(name, param_count);

        // Create entry basic block
        let entry_bb = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_bb);

        // Allocate space for parameters and store them as local variables
        for (i, param) in function.get_param_iter().enumerate() {
            let param_name = format!("param_{}", i);
            let param_int = param.into_int_value();

            // Create alloca for parameter
            let alloca = self
                .builder
                .build_alloca(self.i64_type(), &param_name)
                .map_err(|e| format!("Failed to create alloca for parameter: {:?}", e))?;

            // Store parameter value
            self.builder
                .build_store(alloca, param_int)
                .map_err(|e| format!("Failed to store parameter: {:?}", e))?;

            // Register parameter as local variable
            self.variables.insert(param_name, alloca);
        }

        // Compile function body
        self.compile_instructions(instructions, constants)?;

        // If function doesn't end with return, add default return 0
        if let Some(last_instr) = instructions.last() {
            if !matches!(last_instr, matter_bytecode::Instruction::Return) {
                let zero = self.i64_type().const_int(0, false);
                self.builder
                    .build_return(Some(&zero))
                    .map_err(|e| format!("Failed to build default return: {:?}", e))?;
            }
        }

        Ok(())
    }

    /// Compile bytecode to LLVM IR
    pub fn compile_bytecode(&mut self, bytecode: &Bytecode) -> Result<(), String> {
        // Create main function
        let main_fn = self.create_main();

        // Compile main instructions
        self.compile_instructions(&bytecode.main_instructions, &bytecode.constants)?;

        // Return 0 from main
        let zero = self.i32_type().const_int(0, false);
        self.builder
            .build_return(Some(&zero))
            .map_err(|e| format!("Failed to build return: {:?}", e))?;

        Ok(())
    }

    /// Compile a sequence of instructions with control flow support
    fn compile_instructions(
        &mut self,
        instructions: &[matter_bytecode::Instruction],
        constants: &[matter_bytecode::Constant],
    ) -> Result<(), String> {
        use matter_bytecode::Instruction;

        // First pass: create basic blocks for all jump targets
        let mut jump_targets = std::collections::HashSet::new();
        for (ip, instruction) in instructions.iter().enumerate() {
            match instruction {
                Instruction::Jump(target) | Instruction::JumpIfFalse(target) => {
                    jump_targets.insert(*target);
                }
                _ => {}
            }
        }

        // Create basic blocks for jump targets
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_parent())
            .ok_or("No current function")?;

        for target in jump_targets.iter() {
            let bb = self
                .context
                .append_basic_block(current_fn, &format!("bb_{}", target));
            self.basic_blocks.insert(*target, bb);
        }

        // Second pass: compile instructions
        let mut ip = 0;
        while ip < instructions.len() {
            // If this is a jump target, position builder at the basic block
            if let Some(bb) = self.basic_blocks.get(&ip) {
                // Only switch to the block if we're not already there
                if let Some(current_bb) = self.builder.get_insert_block() {
                    if current_bb != *bb {
                        self.builder.position_at_end(*bb);
                    }
                } else {
                    self.builder.position_at_end(*bb);
                }
            }

            match &instructions[ip] {
                Instruction::LoadConst(id) => {
                    self.compile_load_const(*id, constants)?;
                }
                Instruction::LoadGlobal(name) => {
                    self.compile_load_global(name)?;
                }
                Instruction::StoreGlobal(name) => {
                    self.compile_store_global(name)?;
                }
                Instruction::LoadLocal(name) => {
                    self.compile_load_local(name)?;
                }
                Instruction::StoreLocal(name) => {
                    self.compile_store_local(name)?;
                }
                Instruction::StoreExisting(name) => {
                    self.compile_store_existing(name)?;
                }
                Instruction::Add => {
                    self.compile_add_instruction()?;
                }
                Instruction::Sub => {
                    self.compile_sub_instruction()?;
                }
                Instruction::Mul => {
                    self.compile_mul_instruction()?;
                }
                Instruction::Div => {
                    self.compile_div_instruction()?;
                }
                Instruction::Eq => {
                    self.compile_eq()?;
                }
                Instruction::NotEq => {
                    self.compile_not_eq()?;
                }
                Instruction::Lt => {
                    self.compile_lt()?;
                }
                Instruction::Gt => {
                    self.compile_gt()?;
                }
                Instruction::LtEq => {
                    self.compile_lt_eq()?;
                }
                Instruction::GtEq => {
                    self.compile_gt_eq()?;
                }
                Instruction::Jump(target) => {
                    self.compile_jump(*target)?;
                    // After unconditional jump, we need a new basic block
                    if ip + 1 < instructions.len() {
                        let next_bb = self
                            .context
                            .append_basic_block(current_fn, &format!("after_jump_{}", ip));
                        self.builder.position_at_end(next_bb);
                    }
                }
                Instruction::JumpIfFalse(target) => {
                    self.compile_jump_if_false(*target, ip + 1)?;
                }
                Instruction::Call(arg_count) => {
                    self.compile_call(*arg_count)?;
                }
                Instruction::Return => {
                    self.compile_return()?;
                }
                Instruction::Print => {
                    self.compile_print()?;
                }
                Instruction::Pop => {
                    self.compile_pop()?;
                }
                Instruction::PushScope => {
                    // Scope management is handled at compile time
                }
                Instruction::PopScope => {
                    // Scope management is handled at compile time
                }
                Instruction::Halt => {
                    // Halt is implicit at end of main
                }
                Instruction::NewList(size) => {
                    self.compile_new_list(*size)?;
                }
                Instruction::LoadIndex => {
                    self.compile_load_index()?;
                }
                Instruction::StoreIndex => {
                    self.compile_store_index()?;
                }
                Instruction::StoreIndexVar(name) => {
                    self.compile_store_index_var(name)?;
                }
                Instruction::ListPush => {
                    self.compile_list_push()?;
                }
                Instruction::ListPop => {
                    self.compile_list_pop()?;
                }
                Instruction::ListLen => {
                    self.compile_list_len()?;
                }
                Instruction::ListPushVar(name) => {
                    self.compile_list_push_var(name)?;
                }
                Instruction::ListPopVar(name) => {
                    self.compile_list_pop_var(name)?;
                }
                Instruction::NewMap(size) => {
                    self.compile_new_map(*size)?;
                }
                Instruction::MapHas => {
                    self.compile_map_has()?;
                }
                Instruction::MapKeys => {
                    self.compile_map_keys()?;
                }
                Instruction::MapValues => {
                    self.compile_map_values()?;
                }
                Instruction::NewStruct(type_name, size) => {
                    self.compile_new_struct(type_name, *size)?;
                }
                Instruction::LoadField(field) => {
                    self.compile_load_field(field)?;
                }
                Instruction::StoreFieldVar { target, field } => {
                    self.compile_store_field_var(target, field)?;
                }
                Instruction::SpawnEvent(event) => {
                    self.compile_spawn_event(event)?;
                }
                Instruction::BackendCall {
                    backend,
                    method,
                    arg_count,
                } => {
                    self.compile_backend_call(backend, method, *arg_count)?;
                }
                _ => {
                    return Err(format!(
                        "Instruction not yet implemented: {:?}",
                        instructions[ip]
                    ));
                }
            }

            ip += 1;
        }

        Ok(())
    }

    /// Compile LoadConst instruction
    fn compile_load_const(
        &mut self,
        id: usize,
        constants: &[matter_bytecode::Constant],
    ) -> Result<(), String> {
        use matter_bytecode::Constant;

        let constant = &constants[id];
        match constant {
            Constant::Int(n) => {
                let value = self.compile_int(*n);
                self.stack.push(value);
            }
            Constant::Bool(b) => {
                let value = self.i64_type().const_int(*b as u64, false);
                self.stack.push(value);
            }
            Constant::String(s) => {
                // For now, represent strings as 0 (TODO: proper string handling)
                let value = self.i64_type().const_int(0, false);
                self.stack.push(value);
            }
            Constant::Unit => {
                // Unit is represented as 0
                let value = self.i64_type().const_int(0, false);
                self.stack.push(value);
            }
        }

        Ok(())
    }

    /// Compile LoadGlobal instruction
    fn compile_load_global(&mut self, name: &str) -> Result<(), String> {
        if let Some(ptr) = self.variables.get(name) {
            let value = self
                .builder
                .build_load(self.i64_type(), *ptr, name)
                .map_err(|e| format!("Failed to load global: {:?}", e))?;
            if let inkwell::values::BasicValueEnum::IntValue(int_val) = value {
                self.stack.push(int_val);
            } else {
                return Err(format!("Expected int value for global '{}'", name));
            }
        } else {
            return Err(format!("Undefined global variable '{}'", name));
        }
        Ok(())
    }

    /// Compile StoreGlobal instruction
    fn compile_store_global(&mut self, name: &str) -> Result<(), String> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in StoreGlobal".to_string())?;

        // Create global variable if it doesn't exist
        if !self.variables.contains_key(name) {
            let global = self.module.add_global(self.i64_type(), None, name);
            global.set_initializer(&self.i64_type().const_int(0, false));
            self.variables
                .insert(name.to_string(), global.as_pointer_value());
        }

        let ptr = self
            .variables
            .get(name)
            .ok_or_else(|| format!("Internal error: global variable '{}' not registered", name))?;
        self.builder
            .build_store(*ptr, value)
            .map_err(|e| format!("Failed to store global: {:?}", e))?;

        Ok(())
    }

    /// Compile LoadLocal instruction
    fn compile_load_local(&mut self, name: &str) -> Result<(), String> {
        // For now, treat locals same as globals (TODO: proper scope management)
        self.compile_load_global(name)
    }

    /// Compile StoreLocal instruction
    fn compile_store_local(&mut self, name: &str) -> Result<(), String> {
        // For now, treat locals same as globals (TODO: proper scope management)
        self.compile_store_global(name)
    }

    /// Compile StoreExisting instruction
    fn compile_store_existing(&mut self, name: &str) -> Result<(), String> {
        // Same as StoreGlobal but requires variable to exist
        if !self.variables.contains_key(name) {
            return Err(format!("Cannot set undefined variable '{}'", name));
        }
        self.compile_store_global(name)
    }

    /// Compile Add instruction
    fn compile_add_instruction(&mut self) -> Result<(), String> {
        let right = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Add".to_string())?;
        let left = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Add".to_string())?;
        let result = self.compile_add(left, right)?;
        self.stack.push(result);
        Ok(())
    }

    /// Compile Sub instruction
    fn compile_sub_instruction(&mut self) -> Result<(), String> {
        let right = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Sub".to_string())?;
        let left = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Sub".to_string())?;
        let result = self.compile_sub(left, right)?;
        self.stack.push(result);
        Ok(())
    }

    /// Compile Mul instruction
    fn compile_mul_instruction(&mut self) -> Result<(), String> {
        let right = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Mul".to_string())?;
        let left = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Mul".to_string())?;
        let result = self.compile_mul(left, right)?;
        self.stack.push(result);
        Ok(())
    }

    /// Compile Div instruction
    fn compile_div_instruction(&mut self) -> Result<(), String> {
        let right = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Div".to_string())?;
        let left = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Div".to_string())?;
        let result = self.compile_div(left, right)?;
        self.stack.push(result);
        Ok(())
    }

    /// Compile Eq instruction
    fn compile_eq(&mut self) -> Result<(), String> {
        let right = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Eq".to_string())?;
        let left = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Eq".to_string())?;
        let result = self.compile_cmp(left, right, IntPredicate::EQ)?;
        // Extend i1 to i64
        let extended = self
            .builder
            .build_int_z_extend(result, self.i64_type(), "eq_ext")
            .map_err(|e| format!("Failed to extend: {:?}", e))?;
        self.stack.push(extended);
        Ok(())
    }

    /// Compile NotEq instruction
    fn compile_not_eq(&mut self) -> Result<(), String> {
        let right = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in NotEq".to_string())?;
        let left = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in NotEq".to_string())?;
        let result = self.compile_cmp(left, right, IntPredicate::NE)?;
        let extended = self
            .builder
            .build_int_z_extend(result, self.i64_type(), "ne_ext")
            .map_err(|e| format!("Failed to extend: {:?}", e))?;
        self.stack.push(extended);
        Ok(())
    }

    /// Compile Lt instruction
    fn compile_lt(&mut self) -> Result<(), String> {
        let right = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Lt".to_string())?;
        let left = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Lt".to_string())?;
        let result = self.compile_cmp(left, right, IntPredicate::SLT)?;
        let extended = self
            .builder
            .build_int_z_extend(result, self.i64_type(), "lt_ext")
            .map_err(|e| format!("Failed to extend: {:?}", e))?;
        self.stack.push(extended);
        Ok(())
    }

    /// Compile Gt instruction
    fn compile_gt(&mut self) -> Result<(), String> {
        let right = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Gt".to_string())?;
        let left = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Gt".to_string())?;
        let result = self.compile_cmp(left, right, IntPredicate::SGT)?;
        let extended = self
            .builder
            .build_int_z_extend(result, self.i64_type(), "gt_ext")
            .map_err(|e| format!("Failed to extend: {:?}", e))?;
        self.stack.push(extended);
        Ok(())
    }

    /// Compile LtEq instruction
    fn compile_lt_eq(&mut self) -> Result<(), String> {
        let right = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in LtEq".to_string())?;
        let left = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in LtEq".to_string())?;
        let result = self.compile_cmp(left, right, IntPredicate::SLE)?;
        let extended = self
            .builder
            .build_int_z_extend(result, self.i64_type(), "le_ext")
            .map_err(|e| format!("Failed to extend: {:?}", e))?;
        self.stack.push(extended);
        Ok(())
    }

    /// Compile GtEq instruction
    fn compile_gt_eq(&mut self) -> Result<(), String> {
        let right = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in GtEq".to_string())?;
        let left = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in GtEq".to_string())?;
        let result = self.compile_cmp(left, right, IntPredicate::SGE)?;
        let extended = self
            .builder
            .build_int_z_extend(result, self.i64_type(), "ge_ext")
            .map_err(|e| format!("Failed to extend: {:?}", e))?;
        self.stack.push(extended);
        Ok(())
    }

    /// Compile Jump instruction
    fn compile_jump(&mut self, target: usize) -> Result<(), String> {
        let target_bb = self
            .basic_blocks
            .get(&target)
            .ok_or_else(|| format!("Jump target {} not found", target))?;

        self.builder
            .build_unconditional_branch(*target_bb)
            .map_err(|e| format!("Failed to build jump: {:?}", e))?;

        Ok(())
    }

    /// Compile JumpIfFalse instruction
    fn compile_jump_if_false(&mut self, target: usize, next_ip: usize) -> Result<(), String> {
        let condition = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in JumpIfFalse".to_string())?;

        // Convert i64 to i1 (0 = false, non-zero = true)
        let zero = self.i64_type().const_int(0, false);
        let cond_bool = self
            .builder
            .build_int_compare(IntPredicate::NE, condition, zero, "cond_bool")
            .map_err(|e| format!("Failed to build condition: {:?}", e))?;

        // Get or create target basic block
        let false_bb = self
            .basic_blocks
            .get(&target)
            .ok_or_else(|| format!("Jump target {} not found", target))?;

        // Create basic block for the next instruction (true branch)
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_parent())
            .ok_or("No current function")?;

        let true_bb = if let Some(bb) = self.basic_blocks.get(&next_ip) {
            *bb
        } else {
            let bb = self
                .context
                .append_basic_block(current_fn, &format!("bb_{}", next_ip));
            self.basic_blocks.insert(next_ip, bb);
            bb
        };

        // Build conditional branch
        self.builder
            .build_conditional_branch(cond_bool, true_bb, *false_bb)
            .map_err(|e| format!("Failed to build conditional branch: {:?}", e))?;

        // Position builder at true branch for next instruction
        self.builder.position_at_end(true_bb);

        Ok(())
    }

    /// Compile Call instruction
    fn compile_call(&mut self, arg_count: usize) -> Result<(), String> {
        // Pop function identifier/name from stack
        // In Matter bytecode, the function to call should be on the stack
        let func_value = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Call (function)".to_string())?;

        // Pop arguments from stack
        let mut args = Vec::new();
        for _ in 0..arg_count {
            args.push(
                self.stack
                    .pop()
                    .ok_or_else(|| "Stack underflow in Call (arguments)".to_string())?,
            );
        }
        args.reverse(); // Stack is LIFO, so reverse to get correct order

        // For now, we need a way to map the function value to an actual function
        // This is a simplified implementation that assumes func_value is a function pointer
        // or we need to maintain a function registry

        // REAL IMPLEMENTATION: Try to find function by index/name
        // For now, we'll look up the first user-defined function if it exists
        let function = if let Some((_, func)) = self
            .functions
            .iter()
            .find(|(name, _)| !name.starts_with("printf") && *name != "main")
        {
            *func
        } else {
            // If no user function found, push placeholder and return
            let zero = self.i64_type().const_int(0, false);
            self.stack.push(zero);
            return Ok(());
        };

        // Build the call with arguments
        let call_result = self
            .builder
            .build_call(
                function,
                &args.iter().map(|v| (*v).into()).collect::<Vec<_>>(),
                "call",
            )
            .map_err(|e| format!("Failed to build call: {:?}", e))?;

        // Push return value onto stack
        if let Some(result) = call_result.try_as_basic_value().left() {
            if let inkwell::values::BasicValueEnum::IntValue(int_val) = result {
                self.stack.push(int_val);
            } else {
                return Err("Function returned non-integer value".to_string());
            }
        } else {
            // Function returned void, push 0
            let zero = self.i64_type().const_int(0, false);
            self.stack.push(zero);
        }

        Ok(())
    }

    /// Compile Return instruction
    fn compile_return(&mut self) -> Result<(), String> {
        // Pop return value from stack
        let return_value = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Return".to_string())?;

        // Build return instruction
        self.builder
            .build_return(Some(&return_value))
            .map_err(|e| format!("Failed to build return: {:?}", e))?;

        Ok(())
    }

    /// Compile Print instruction
    fn compile_print(&mut self) -> Result<(), String> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in Print".to_string())?;

        // Declare printf if not already declared
        if !self.functions.contains_key("printf") {
            let i8_ptr_type = self.i8_type().ptr_type(inkwell::AddressSpace::default());
            let printf_type = self.i32_type().fn_type(&[i8_ptr_type.into()], true);
            let printf_fn = self.module.add_function("printf", printf_type, None);
            self.functions.insert("printf".to_string(), printf_fn);
        }

        // Create format string "%lld\n" for i64
        let format_str = self
            .builder
            .build_global_string_ptr("%lld\n", "fmt")
            .map_err(|e| format!("Failed to create format string: {:?}", e))?;

        let printf_fn = self
            .functions
            .get("printf")
            .ok_or_else(|| "Internal error: printf function missing".to_string())?;
        self.builder
            .build_call(
                *printf_fn,
                &[format_str.as_pointer_value().into(), value.into()],
                "printf_call",
            )
            .map_err(|e| format!("Failed to call printf: {:?}", e))?;

        Ok(())
    }

    /// Compile Pop instruction
    fn compile_pop(&mut self) -> Result<(), String> {
        self.stack
            .pop()
            .ok_or_else(|| "Stack underflow in Pop".to_string())?;
        Ok(())
    }

    // ============================================================================
    // DATA STRUCTURES
    // ============================================================================
    // Note: For simplicity, we represent data structures as i64 handles/IDs
    // A full implementation would use LLVM structs and heap allocation

    /// Compile NewList instruction
    fn compile_new_list(&mut self, size: usize) -> Result<(), String> {
        // Collect N elements from stack
        let mut initial_elements = Vec::new();
        for _ in 0..size {
            let elem = self
                .stack
                .pop()
                .ok_or_else(|| "Stack underflow in NewList".to_string())?;
            initial_elements.push(elem);
        }
        // Elements are popped in reverse order
        initial_elements.reverse();

        // Call matter_list_new(size)
        let fn_list_new = self.functions.get("matter_list_new").unwrap();
        let cap = self
            .i64_type()
            .const_int(std::cmp::max(size, 4) as u64, false);
        let list_ptr_val = self
            .builder
            .build_call(*fn_list_new, &[cap.into()], "call_list_new")
            .map_err(|e| format!("Failed: {:?}", e))?
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Push initial elements
        let fn_list_push = self.functions.get("matter_list_push").unwrap();
        for elem in initial_elements {
            self.builder
                .build_call(
                    *fn_list_push,
                    &[list_ptr_val.into(), elem.into()],
                    "call_list_push",
                )
                .map_err(|e| format!("Failed: {:?}", e))?;
        }

        // We store the pointer casted as i64 on the Matter stack
        let list_handle = self
            .builder
            .build_ptr_to_int(list_ptr_val, self.i64_type(), "list_handle")
            .map_err(|e| format!("Failed: {:?}", e))?;
        self.stack.push(list_handle);

        Ok(())
    }

    /// Compile LoadIndex instruction
    fn compile_load_index(&mut self) -> Result<(), String> {
        let index = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in LoadIndex (index)".to_string())?;
        let collection = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in LoadIndex (collection)".to_string())?;

        let fn_col_get = self.functions.get("matter_collection_get").unwrap();
        let col_ptr_val = self
            .builder
            .build_int_to_ptr(
                collection,
                self.i8_type().ptr_type(inkwell::AddressSpace::default()),
                "col_ptr",
            )
            .map_err(|e| format!("Failed: {:?}", e))?;

        let value_val = self
            .builder
            .build_call(
                *fn_col_get,
                &[col_ptr_val.into(), index.into()],
                "call_col_get",
            )
            .map_err(|e| format!("Failed: {:?}", e))?
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        self.stack.push(value_val);
        Ok(())
    }

    /// Compile StoreIndex instruction
    fn compile_store_index(&mut self) -> Result<(), String> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in StoreIndex (value)".to_string())?;
        let index = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in StoreIndex (index)".to_string())?;
        let collection = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in StoreIndex (collection)".to_string())?;

        let fn_col_set = self.functions.get("matter_collection_set").unwrap();
        let col_ptr_val = self
            .builder
            .build_int_to_ptr(
                collection,
                self.i8_type().ptr_type(inkwell::AddressSpace::default()),
                "col_ptr",
            )
            .map_err(|e| format!("Failed: {:?}", e))?;

        self.builder
            .build_call(
                *fn_col_set,
                &[col_ptr_val.into(), index.into(), value.into()],
                "call_col_set",
            )
            .map_err(|e| format!("Failed: {:?}", e))?;

        // Matter's VM pushes the collection back after store index
        self.stack.push(collection);

        Ok(())
    }

    /// Compile StoreIndexVar instruction
    fn compile_store_index_var(&mut self, _name: &str) -> Result<(), String> {
        let _value = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in StoreIndexVar (value)".to_string())?;
        let _index = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in StoreIndexVar (index)".to_string())?;

        // TODO: Implement proper variable list/map index assignment

        Ok(())
    }

    /// Compile ListPush instruction
    fn compile_list_push(&mut self) -> Result<(), String> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in ListPush (value)".to_string())?;
        let list = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in ListPush (list)".to_string())?;

        let fn_list_push = self.functions.get("matter_list_push").unwrap();
        let list_ptr_val = self
            .builder
            .build_int_to_ptr(
                list,
                self.i8_type().ptr_type(inkwell::AddressSpace::default()),
                "list_ptr",
            )
            .map_err(|e| format!("Failed: {:?}", e))?;

        self.builder
            .build_call(
                *fn_list_push,
                &[list_ptr_val.into(), value.into()],
                "call_list_push",
            )
            .map_err(|e| format!("Failed: {:?}", e))?;

        // Push modified list back
        self.stack.push(list);

        Ok(())
    }

    /// Compile ListPop instruction
    fn compile_list_pop(&mut self) -> Result<(), String> {
        let list = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in ListPop".to_string())?;

        let fn_list_pop = self.functions.get("matter_list_pop").unwrap();
        let list_ptr_val = self
            .builder
            .build_int_to_ptr(
                list,
                self.i8_type().ptr_type(inkwell::AddressSpace::default()),
                "list_ptr",
            )
            .map_err(|e| format!("Failed: {:?}", e))?;

        let value_val = self
            .builder
            .build_call(*fn_list_pop, &[list_ptr_val.into()], "call_list_pop")
            .map_err(|e| format!("Failed: {:?}", e))?
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        self.stack.push(value_val);
        self.stack.push(list);

        Ok(())
    }

    /// Compile ListLen instruction
    fn compile_list_len(&mut self) -> Result<(), String> {
        let list = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in ListLen".to_string())?;

        let fn_list_len = self.functions.get("matter_list_len").unwrap();
        let list_ptr_val = self
            .builder
            .build_int_to_ptr(
                list,
                self.i8_type().ptr_type(inkwell::AddressSpace::default()),
                "list_ptr",
            )
            .map_err(|e| format!("Failed: {:?}", e))?;

        let len_val = self
            .builder
            .build_call(*fn_list_len, &[list_ptr_val.into()], "call_list_len")
            .map_err(|e| format!("Failed: {:?}", e))?
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        self.stack.push(len_val);

        Ok(())
    }

    /// Compile ListPushVar instruction
    fn compile_list_push_var(&mut self, _name: &str) -> Result<(), String> {
        let _value = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in ListPushVar".to_string())?;

        // TODO: Implement proper variable list push

        // Push unit
        let unit = self.i64_type().const_int(0, false);
        self.stack.push(unit);

        Ok(())
    }

    /// Compile ListPopVar instruction
    fn compile_list_pop_var(&mut self, _name: &str) -> Result<(), String> {
        // TODO: Implement proper variable list pop

        // Push popped value (placeholder)
        let value = self.i64_type().const_int(0, false);
        self.stack.push(value);

        Ok(())
    }

    /// Compile NewMap instruction
    fn compile_new_map(&mut self, size: usize) -> Result<(), String> {
        // Collect N key/value pairs from stack
        let mut initial_pairs = Vec::new();
        for _ in 0..size {
            let value = self
                .stack
                .pop()
                .ok_or_else(|| "Stack underflow in NewMap (value)".to_string())?;
            let key = self
                .stack
                .pop()
                .ok_or_else(|| "Stack underflow in NewMap (key)".to_string())?;
            initial_pairs.push((key, value));
        }
        initial_pairs.reverse();

        let fn_map_new = self.functions.get("matter_map_new").unwrap();
        let map_ptr_val = self
            .builder
            .build_call(*fn_map_new, &[], "call_map_new")
            .map_err(|e| format!("Failed: {:?}", e))?
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let fn_map_insert = self.functions.get("matter_map_insert").unwrap();
        for (key, value) in initial_pairs {
            self.builder
                .build_call(
                    *fn_map_insert,
                    &[map_ptr_val.into(), key.into(), value.into()],
                    "call_map_insert",
                )
                .map_err(|e| format!("Failed: {:?}", e))?;
        }

        let map_handle = self
            .builder
            .build_ptr_to_int(map_ptr_val, self.i64_type(), "map_handle")
            .map_err(|e| format!("Failed: {:?}", e))?;
        self.stack.push(map_handle);

        Ok(())
    }

    /// Compile MapHas instruction
    fn compile_map_has(&mut self) -> Result<(), String> {
        let key = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in MapHas (key)".to_string())?;
        let map = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in MapHas (map)".to_string())?;

        let fn_map_has = self.functions.get("matter_map_has").unwrap();
        let map_ptr_val = self
            .builder
            .build_int_to_ptr(
                map,
                self.i8_type().ptr_type(inkwell::AddressSpace::default()),
                "map_ptr",
            )
            .map_err(|e| format!("Failed: {:?}", e))?;

        let result_val = self
            .builder
            .build_call(
                *fn_map_has,
                &[map_ptr_val.into(), key.into()],
                "call_map_has",
            )
            .map_err(|e| format!("Failed: {:?}", e))?
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_int_value();

        // Convert i32 (bool) to i64
        let result_i64 = self
            .builder
            .build_int_z_extend(result_val, self.i64_type(), "has_ext")
            .map_err(|e| format!("Failed: {:?}", e))?;

        self.stack.push(result_i64);

        Ok(())
    }

    /// Compile MapKeys instruction
    fn compile_map_keys(&mut self) -> Result<(), String> {
        let map = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in MapKeys".to_string())?;

        let fn_map_keys = self.functions.get("matter_map_keys").unwrap();
        let map_ptr_val = self
            .builder
            .build_int_to_ptr(
                map,
                self.i8_type().ptr_type(inkwell::AddressSpace::default()),
                "map_ptr",
            )
            .map_err(|e| format!("Failed: {:?}", e))?;

        let list_ptr_val = self
            .builder
            .build_call(*fn_map_keys, &[map_ptr_val.into()], "call_map_keys")
            .map_err(|e| format!("Failed: {:?}", e))?
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let list_handle = self
            .builder
            .build_ptr_to_int(list_ptr_val, self.i64_type(), "keys_handle")
            .map_err(|e| format!("Failed: {:?}", e))?;

        self.stack.push(list_handle);

        Ok(())
    }

    /// Compile MapValues instruction
    fn compile_map_values(&mut self) -> Result<(), String> {
        let map = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in MapValues".to_string())?;

        let fn_map_values = self.functions.get("matter_map_values").unwrap();
        let map_ptr_val = self
            .builder
            .build_int_to_ptr(
                map,
                self.i8_type().ptr_type(inkwell::AddressSpace::default()),
                "map_ptr",
            )
            .map_err(|e| format!("Failed: {:?}", e))?;

        let list_ptr_val = self
            .builder
            .build_call(*fn_map_values, &[map_ptr_val.into()], "call_map_values")
            .map_err(|e| format!("Failed: {:?}", e))?
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        let list_handle = self
            .builder
            .build_ptr_to_int(list_ptr_val, self.i64_type(), "values_handle")
            .map_err(|e| format!("Failed: {:?}", e))?;

        self.stack.push(list_handle);

        Ok(())
    }

    /// Compile NewStruct instruction
    fn compile_new_struct(&mut self, _type_name: &str, size: usize) -> Result<(), String> {
        // Pop N field/value pairs from stack
        for _ in 0..size {
            let _value = self
                .stack
                .pop()
                .ok_or_else(|| "Stack underflow in NewStruct (value)".to_string())?;
            let _field = self
                .stack
                .pop()
                .ok_or_else(|| "Stack underflow in NewStruct (field)".to_string())?;
        }

        // Push struct handle (placeholder: 0)
        let struct_handle = self.i64_type().const_int(0, false);
        self.stack.push(struct_handle);

        Ok(())
    }

    /// Compile LoadField instruction
    fn compile_load_field(&mut self, _field: &str) -> Result<(), String> {
        let _struct_or_map = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in LoadField".to_string())?;

        // Push field value (placeholder: 0)
        let value = self.i64_type().const_int(0, false);
        self.stack.push(value);

        Ok(())
    }

    /// Compile StoreFieldVar instruction
    fn compile_store_field_var(&mut self, _target: &str, _field: &str) -> Result<(), String> {
        let _value = self
            .stack
            .pop()
            .ok_or_else(|| "Stack underflow in StoreFieldVar".to_string())?;

        // TODO: Implement proper struct/map field assignment

        Ok(())
    }

    /// Compile SpawnEvent instruction
    fn compile_spawn_event(&mut self, _event: &str) -> Result<(), String> {
        // TODO: Implement event spawning
        // For now, this is a no-op
        Ok(())
    }

    /// Compile BackendCall instruction
    fn compile_backend_call(
        &mut self,
        _backend: &str,
        _method: &str,
        arg_count: usize,
    ) -> Result<(), String> {
        // Pop arguments from stack
        for _ in 0..arg_count {
            self.stack
                .pop()
                .ok_or_else(|| "Stack underflow in BackendCall".to_string())?;
        }

        // TODO: Implement proper backend call mechanism

        // Push result (placeholder: 0)
        let result = self.i64_type().const_int(0, false);
        self.stack.push(result);

        Ok(())
    }

    // ============================================================================
    // PUBLIC API
    // ============================================================================

    /// Compile an integer constant
    pub fn compile_int(&self, value: i64) -> IntValue<'ctx> {
        self.i64_type().const_int(value as u64, false)
    }

    /// Compile an addition
    pub fn compile_add(
        &self,
        lhs: IntValue<'ctx>,
        rhs: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        self.builder
            .build_int_add(lhs, rhs, "add")
            .map_err(|e| format!("Failed to build add: {:?}", e))
    }

    /// Compile a subtraction
    pub fn compile_sub(
        &self,
        lhs: IntValue<'ctx>,
        rhs: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        self.builder
            .build_int_sub(lhs, rhs, "sub")
            .map_err(|e| format!("Failed to build sub: {:?}", e))
    }

    /// Compile a multiplication
    pub fn compile_mul(
        &self,
        lhs: IntValue<'ctx>,
        rhs: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        self.builder
            .build_int_mul(lhs, rhs, "mul")
            .map_err(|e| format!("Failed to build mul: {:?}", e))
    }

    /// Compile a division
    pub fn compile_div(
        &self,
        lhs: IntValue<'ctx>,
        rhs: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        self.builder
            .build_int_signed_div(lhs, rhs, "div")
            .map_err(|e| format!("Failed to build div: {:?}", e))
    }

    /// Compile a comparison
    pub fn compile_cmp(
        &self,
        lhs: IntValue<'ctx>,
        rhs: IntValue<'ctx>,
        op: IntPredicate,
    ) -> Result<IntValue<'ctx>, String> {
        self.builder
            .build_int_compare(op, lhs, rhs, "cmp")
            .map_err(|e| format!("Failed to build cmp: {:?}", e))
    }

    /// Get the LLVM IR as a string
    pub fn get_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }

    /// Verify the module
    pub fn verify(&self) -> Result<(), String> {
        self.module
            .verify()
            .map_err(|e| format!("Module verification failed: {}", e.to_string()))
    }

    /// Write object file with specified optimization level
    pub fn write_object_file(
        &self,
        path: &str,
        opt_level: OptimizationLevel,
    ) -> Result<(), String> {
        use inkwell::targets::{
            CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
        };

        Target::initialize_all(&InitializationConfig::default());

        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple)
            .map_err(|e| format!("Failed to get target: {}", e.to_string()))?;

        let target_machine = target
            .create_target_machine(
                &target_triple,
                "generic",
                "",
                opt_level,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or("Failed to create target machine")?;

        target_machine
            .write_to_file(&self.module, FileType::Object, path.as_ref())
            .map_err(|e| format!("Failed to write object file: {}", e.to_string()))?;

        Ok(())
    }

    /// Compile to executable with specified optimization level
    pub fn compile_to_executable(
        &self,
        output_path: &str,
        opt_level: OptimizationLevel,
    ) -> Result<(), String> {
        // Write object file
        let obj_path = format!("{}.o", output_path);
        self.write_object_file(&obj_path, opt_level)?;

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

/// Compile Matter bytecode to native code with specified optimization level
pub fn compile_to_native_with_opt(
    bytecode: &Bytecode,
    output_path: &str,
    opt_level: OptimizationLevel,
) -> Result<(), String> {
    let context = Context::create();
    let mut codegen = LLVMCodegen::new(&context, "matter_program");

    // Compile bytecode
    codegen.compile_bytecode(bytecode)?;

    // Verify module
    codegen.verify()?;

    // Compile to executable
    codegen.compile_to_executable(output_path, opt_level)?;

    Ok(())
}

/// Compile Matter bytecode to native code (default: aggressive optimization)
pub fn compile_to_native(bytecode: &Bytecode, output_path: &str) -> Result<(), String> {
    compile_to_native_with_opt(bytecode, output_path, OptimizationLevel::Aggressive)
}

/// Parse optimization level from string (-O0, -O1, -O2, -O3)
pub fn parse_opt_level(opt_str: &str) -> Result<OptimizationLevel, String> {
    match opt_str {
        "-O0" | "O0" | "0" => Ok(OptimizationLevel::None),
        "-O1" | "O1" | "1" => Ok(OptimizationLevel::Less),
        "-O2" | "O2" | "2" => Ok(OptimizationLevel::Default),
        "-O3" | "O3" | "3" => Ok(OptimizationLevel::Aggressive),
        _ => Err(format!(
            "Invalid optimization level: {}. Use -O0, -O1, -O2, or -O3",
            opt_str
        )),
    }
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
    use matter_bytecode::{Bytecode, Constant, Instruction};

    #[test]
    fn test_codegen_creation() {
        let context = Context::create();
        let codegen = LLVMCodegen::new(&context, "test");
        assert_eq!(codegen.variables.len(), 0);
        assert_eq!(codegen.functions.len(), 0);
        assert_eq!(codegen.stack.len(), 0);
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

    #[test]
    fn test_compile_simple_constant() {
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test");

        let mut bytecode = Bytecode::new();
        let const_id = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![Instruction::LoadConst(const_id), Instruction::Halt];

        let result = codegen.compile_bytecode(&bytecode);
        assert!(result.is_ok());
        assert_eq!(codegen.stack.len(), 1);
    }

    #[test]
    fn test_compile_arithmetic() {
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test");

        let mut bytecode = Bytecode::new();
        let const_10 = bytecode.add_constant(Constant::Int(10));
        let const_20 = bytecode.add_constant(Constant::Int(20));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(const_10),
            Instruction::LoadConst(const_20),
            Instruction::Add,
            Instruction::Halt,
        ];

        let result = codegen.compile_bytecode(&bytecode);
        assert!(result.is_ok());
        assert_eq!(codegen.stack.len(), 1);
    }

    #[test]
    fn test_compile_variable_store_load() {
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test");

        let mut bytecode = Bytecode::new();
        let const_42 = bytecode.add_constant(Constant::Int(42));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(const_42),
            Instruction::StoreGlobal("x".to_string()),
            Instruction::LoadGlobal("x".to_string()),
            Instruction::Halt,
        ];

        let result = codegen.compile_bytecode(&bytecode);
        assert!(result.is_ok());
        assert_eq!(codegen.stack.len(), 1);
        assert!(codegen.variables.contains_key("x"));
    }

    #[test]
    fn test_compile_comparison() {
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test");

        let mut bytecode = Bytecode::new();
        let const_10 = bytecode.add_constant(Constant::Int(10));
        let const_20 = bytecode.add_constant(Constant::Int(20));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(const_10),
            Instruction::LoadConst(const_20),
            Instruction::Lt,
            Instruction::Halt,
        ];

        let result = codegen.compile_bytecode(&bytecode);
        assert!(result.is_ok());
        assert_eq!(codegen.stack.len(), 1);
    }

    #[test]
    fn test_get_llvm_ir_simple() {
        let mut bytecode = Bytecode::new();
        let const_42 = bytecode.add_constant(Constant::Int(42));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(const_42),
            Instruction::Print,
            Instruction::Halt,
        ];

        let result = get_llvm_ir(&bytecode);
        assert!(result.is_ok());

        let ir = result.unwrap();
        assert!(ir.contains("define"));
        assert!(ir.contains("main"));
        assert!(ir.contains("printf"));
    }

    #[test]
    fn test_compile_if_statement() {
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test");

        let mut bytecode = Bytecode::new();
        let const_10 = bytecode.add_constant(Constant::Int(10));
        let const_20 = bytecode.add_constant(Constant::Int(20));

        // if (10 < 20) { print(1); } else { print(0); }
        bytecode.main_instructions = vec![
            Instruction::LoadConst(const_10), // 0
            Instruction::LoadConst(const_20), // 1
            Instruction::Lt,                  // 2
            Instruction::JumpIfFalse(7),      // 3 - jump to else
            // then branch
            Instruction::LoadConst(bytecode.add_constant(Constant::Int(1))), // 4
            Instruction::Print,                                              // 5
            Instruction::Jump(9),                                            // 6 - jump to end
            // else branch
            Instruction::LoadConst(bytecode.add_constant(Constant::Int(0))), // 7
            Instruction::Print,                                              // 8
            // end
            Instruction::Halt, // 9
        ];

        let result = codegen.compile_bytecode(&bytecode);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_while_loop() {
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test");

        let mut bytecode = Bytecode::new();
        let const_0 = bytecode.add_constant(Constant::Int(0));
        let const_1 = bytecode.add_constant(Constant::Int(1));
        let const_10 = bytecode.add_constant(Constant::Int(10));

        // let x = 0; while (x < 10) { x = x + 1; }
        bytecode.main_instructions = vec![
            Instruction::LoadConst(const_0),           // 0
            Instruction::StoreGlobal("x".to_string()), // 1
            // loop start
            Instruction::LoadGlobal("x".to_string()), // 2
            Instruction::LoadConst(const_10),         // 3
            Instruction::Lt,                          // 4
            Instruction::JumpIfFalse(11),             // 5 - jump to end
            // loop body
            Instruction::LoadGlobal("x".to_string()),  // 6
            Instruction::LoadConst(const_1),           // 7
            Instruction::Add,                          // 8
            Instruction::StoreGlobal("x".to_string()), // 9
            Instruction::Jump(2),                      // 10 - jump to loop start
            // end
            Instruction::Halt, // 11
        ];

        let result = codegen.compile_bytecode(&bytecode);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_unconditional_jump() {
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test");

        let mut bytecode = Bytecode::new();
        let const_42 = bytecode.add_constant(Constant::Int(42));

        bytecode.main_instructions = vec![
            Instruction::Jump(2),             // 0 - skip next instruction
            Instruction::LoadConst(const_42), // 1 - skipped
            Instruction::LoadConst(const_42), // 2 - executed
            Instruction::Print,               // 3
            Instruction::Halt,                // 4
        ];

        let result = codegen.compile_bytecode(&bytecode);
        assert!(result.is_ok());
    }
}
