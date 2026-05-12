//! x86-64 code generator
//!
//! Generates native x86-64 machine code from Matter bytecode.
//! Uses System V AMD64 ABI calling convention.

// Error format convention (keep consistent across backends):
// - Prefix: "<BACKEND> backend: <summary>"
// - Optional context: "[context:key=value,...]"
// - Quantitative diagnostics when relevant: "needed N, available M"
use super::Register;
use matter_bytecode::{Bytecode, Constant, Instruction};
use std::collections::HashMap;

/// x86-64 code generator
pub struct X86CodeGen {
    /// Generated machine code
    code: Vec<u8>,

    /// Data section (strings, constants)
    data: Vec<u8>,

    /// Variable stack offsets (name -> rbp-relative offset)
    variables: HashMap<String, i32>,

    /// Current stack frame offset for locals
    stack_offset: i32,

    /// Current stack depth (for validation)
    stack_depth: i32,

    /// Jump targets (bytecode IP -> code offset)
    jump_targets: HashMap<usize, usize>,

    /// Pending jumps to patch (code offset -> (bytecode target IP, instruction length))
    pending_jumps: Vec<(usize, usize, usize)>,

    /// Pending data relative patches (code offset -> data offset)
    pending_data_patches: Vec<(usize, usize)>,

    /// Function addresses (name -> code offset)
    function_addresses: HashMap<String, usize>,
}

impl X86CodeGen {
    fn ctx_arg_count(arg_count: usize) -> String {
        format!("arg_count={}", arg_count)
    }

    fn ctx_backend_call(backend: &str, method: &str, arg_count: usize) -> String {
        format!("backend={}.{},arg_count={}", backend, method, arg_count)
    }

    fn ctx_operands(operands: &str) -> String {
        format!("operands={}", operands)
    }

    /// Create a new x86-64 code generator
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            data: Vec::new(),
            variables: HashMap::new(),
            stack_offset: 0,
            stack_depth: 0,
            jump_targets: HashMap::new(),
            pending_jumps: Vec::new(),
            pending_data_patches: Vec::new(),
            function_addresses: HashMap::new(),
        }
    }

    /// Compile Matter bytecode to x86-64 machine code
    pub fn compile(&mut self, bytecode: &Bytecode) -> Result<Vec<u8>, String> {
        // First pass: compile all functions
        for (name, function) in &bytecode.functions {
            self.compile_function(name, function, &bytecode.constants)?;
        }

        // Record main function start
        let _main_start = self.code.len();

        // Emit main function prologue
        self.emit_prologue();

        // Second pass: identify jump targets in main
        for instr in &bytecode.main_instructions {
            match instr {
                Instruction::Jump(target) | Instruction::JumpIfFalse(target) => {
                    self.jump_targets.insert(*target, 0); // Will be patched in third pass
                }
                _ => {}
            }
        }

        // Third pass: compile main instructions
        for (ip, instr) in bytecode.main_instructions.iter().enumerate() {
            // Mark jump target
            if self.jump_targets.contains_key(&ip) {
                self.jump_targets.insert(ip, self.code.len());
            }

            self.compile_instruction(instr, &bytecode.constants)?;
        }

        // Emit main function epilogue
        self.emit_epilogue();

        // Fourth pass: patch jumps
        self.patch_jumps()?;

        // Capture code length for data patching
        let code_len = self.code.len();

        // Append data section at the end
        self.code.extend_from_slice(&self.data);

        // Fifth pass: patch data relative addresses
        self.patch_data_offsets(code_len)?;

        Ok(self.code.clone())
    }

    /// Compile a user-defined function
    fn compile_function(
        &mut self,
        name: &str,
        function: &matter_bytecode::Function,
        constants: &[Constant],
    ) -> Result<(), String> {
        // Record function start address
        let func_start = self.code.len();
        self.function_addresses.insert(name.to_string(), func_start);

        // Save current state
        let saved_variables = self.variables.clone();
        let saved_stack_offset = self.stack_offset;
        let saved_jump_targets = self.jump_targets.clone();
        let saved_pending_jumps = self.pending_jumps.clone();

        // Reset for function compilation
        self.variables.clear();
        self.jump_targets.clear();
        self.pending_jumps.clear();

        // Emit function prologue
        self.emit_prologue();

        // Parameters are passed via registers (System V AMD64 ABI):
        // RDI, RSI, RDX, RCX, R8, R9 (Linux/macOS)
        // RCX, RDX, R8, R9 (Windows)
        // We'll store them as locals on the stack

        #[cfg(not(windows))]
        let param_regs = [
            Register::RDI,
            Register::RSI,
            Register::RDX,
            Register::RCX,
            Register::R8,
            Register::R9,
        ];

        #[cfg(windows)]
        let param_regs = [Register::RCX, Register::RDX, Register::R8, Register::R9];

        // Note: First parameter is runtime pointer, so we skip it
        // Actual function parameters start from second register
        let max_register_args = param_regs.len() - 1;
        for i in 0..function.param_count {
            let param_name = format!("__param_{}", i);
            self.stack_offset -= 8;
            self.variables.insert(param_name.clone(), self.stack_offset);

            // Store parameter from register or load from caller stack.
            let reg_idx = i + 1; // Skip first register (runtime pointer)
            if i < max_register_args {
                self.emit_mov_to_stack(self.stack_offset, param_regs[reg_idx]);
            } else {
                let stack_arg_idx = i - max_register_args;
                #[cfg(not(windows))]
                let caller_stack_offset = 16 + (stack_arg_idx as i32 * 8);
                #[cfg(windows)]
                let caller_stack_offset = 48 + (stack_arg_idx as i32 * 8);
                self.emit_mov_from_stack(Register::RAX, caller_stack_offset);
                self.emit_mov_to_stack(self.stack_offset, Register::RAX);
            }
        }

        // First pass: identify jump targets
        for instr in &function.instructions {
            match instr {
                Instruction::Jump(target) | Instruction::JumpIfFalse(target) => {
                    self.jump_targets.insert(*target, 0);
                }
                _ => {}
            }
        }

        // Second pass: compile instructions
        for (ip, instr) in function.instructions.iter().enumerate() {
            if self.jump_targets.contains_key(&ip) {
                self.jump_targets.insert(ip, self.code.len());
            }

            self.compile_instruction(instr, constants)?;
        }

        // Ensure function returns
        if !matches!(function.instructions.last(), Some(Instruction::Return)) {
            self.emit_mov_imm(Register::RAX, 0); // Return Unit (0)
            self.emit_epilogue();
        }

        // Patch jumps within function
        self.patch_jumps()?;

        // Restore state
        self.variables = saved_variables;
        self.stack_offset = saved_stack_offset;
        self.jump_targets = saved_jump_targets;
        self.pending_jumps = saved_pending_jumps;

        Ok(())
    }

    /// Compile a single instruction
    fn compile_instruction(
        &mut self,
        instr: &Instruction,
        constants: &[Constant],
    ) -> Result<(), String> {
        match instr {
            Instruction::LoadConst(id) => {
                self.compile_load_const(*id, constants)?;
            }
            Instruction::Add => {
                self.compile_add()?;
            }
            Instruction::Sub => {
                self.compile_sub()?;
            }
            Instruction::Mul => {
                self.compile_mul()?;
            }
            Instruction::Div => {
                self.compile_div()?;
            }
            Instruction::Mod => {
                return Err("Mod not supported in native yet".to_string());
            }
            Instruction::Neg => {
                return Err("Neg not supported in native yet".to_string());
            }
            Instruction::And => {
                return Err("And not supported in native yet".to_string());
            }
            Instruction::Or => {
                return Err("Or not supported in native yet".to_string());
            }
            Instruction::Not => {
                return Err("Not not supported in native yet".to_string());
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
            Instruction::LoadLocal(name) => {
                self.compile_load_local(name)?;
            }
            Instruction::LoadParam(index) => {
                self.compile_load_local(&format!("__param_{}", index))?;
            }
            Instruction::LoadGlobal(name) => {
                self.compile_load_global(name)?;
            }
            Instruction::StoreLocal(name) => {
                self.compile_store_local(name)?;
            }
            Instruction::StoreGlobal(name) => {
                self.compile_store_global(name)?;
            }
            Instruction::StoreExisting(name) => {
                self.compile_store_local(name)?;
            }
            Instruction::Jump(target) => {
                self.compile_jump(*target)?;
            }
            Instruction::JumpIfFalse(target) => {
                self.compile_jump_if_false(*target)?;
            }
            Instruction::Print => {
                self.compile_print()?;
            }
            Instruction::Pop => {
                self.compile_pop()?;
            }
            Instruction::Call(arg_count) => {
                // The function name is on the stack in the VM,
                // but in JIT we might need a different approach.
                // For now, we assume the call is to a name we can lookup.
                self.compile_call(*arg_count)?;
            }
            Instruction::CallNamed { arg_count, .. } => {
                self.compile_call(*arg_count)?;
            }
            Instruction::Halt | Instruction::Return => {
                self.compile_return()?;
            }
            Instruction::PushScope | Instruction::PopScope => {
                // Scope management is compile-time only
            }
            // Sprint 26 Phase 4: Data Structures
            Instruction::NewList(count) => {
                self.compile_new_list(*count)?;
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
            Instruction::NewMap(count) => {
                self.compile_new_map(*count)?;
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
            Instruction::NewStruct(type_name, field_count) => {
                self.compile_new_struct(type_name, *field_count)?;
            }
            Instruction::LoadField(field) => {
                self.compile_load_field(field)?;
            }
            Instruction::StoreFieldVar { target, field } => {
                self.compile_store_field_var(target, field)?;
            }
            Instruction::SpawnEvent(_event) => {
                // VM behavior: enqueue side-effect only; no stack value produced.
                // Native backend currently has no event queue integration, so this is a no-op.
            }
            Instruction::BackendCall {
                backend,
                method,
                arg_count,
            } => {
                self.compile_backend_call(backend, method, *arg_count)?;
            }
        }

        Ok(())
    }

    /// Compile LoadConst instruction
    fn compile_load_const(&mut self, id: usize, constants: &[Constant]) -> Result<(), String> {
        if id >= constants.len() {
            return Err(format!(
                "LoadConst index out of bounds: id={}, constants={}",
                id,
                constants.len()
            ));
        }
        let constant = &constants[id];

        match constant {
            Constant::Int(n) => {
                // mov rax, n
                self.emit_mov_imm(Register::RAX, *n);
                self.emit_push(Register::RAX);
            }
            Constant::Float(f) => {
                // Warning: Native compiler currently treats all as i64
                self.emit_mov_imm(Register::RAX, f.to_bits() as i64);
                self.emit_push(Register::RAX);
            }
            Constant::Bool(b) => {
                // mov rax, 0/1
                self.emit_mov_imm(Register::RAX, *b as i64);
                self.emit_push(Register::RAX);
            }
            Constant::String(s) => {
                // Native path currently represents values as i64.
                // Use a stable hash for string constants so map keys are not all zero.
                let hash = self.hash_type_name(s);
                self.emit_mov_imm(Register::RAX, hash);
                self.emit_push(Register::RAX);
            }
            Constant::Unit => {
                // mov rax, 0
                self.emit_mov_imm(Register::RAX, 0);
                self.emit_push(Register::RAX);
            }
        }

        Ok(())
    }

    /// Compile Add instruction
    fn compile_add(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RBX, "Add")?;
        self.emit_pop_checked(Register::RAX, "Add")?;

        // add rax, rbx
        self.emit_add_reg(Register::RAX, Register::RBX);
        self.emit_push(Register::RAX);

        Ok(())
    }

    /// Compile Sub instruction
    fn compile_sub(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RBX, "Sub")?; // Right operand
        self.emit_pop_checked(Register::RAX, "Sub")?; // Left operand

        // sub rax, rbx
        self.emit_sub_reg(Register::RAX, Register::RBX);
        self.emit_push(Register::RAX);

        Ok(())
    }

    /// Compile Mul instruction
    fn compile_mul(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RBX, "Mul")?;
        self.emit_pop_checked(Register::RAX, "Mul")?;

        // imul rax, rbx
        self.emit_mul_reg(Register::RAX, Register::RBX);
        self.emit_push(Register::RAX);

        Ok(())
    }

    /// Compile Div instruction
    fn compile_div(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RBX, "Div")?; // Divisor
        self.emit_pop_checked(Register::RAX, "Div")?; // Dividend

        // Sign extend RAX to RDX:RAX
        self.emit_cqo();

        // idiv rbx (quotient in RAX, remainder in RDX)
        self.emit_div_reg(Register::RBX);

        self.emit_push(Register::RAX);

        Ok(())
    }

    /// Compile comparison instruction
    fn compile_comparison(&mut self, predicate: &str) -> Result<(), String> {
        self.emit_pop_checked(Register::RBX, predicate)?; // Right
        self.emit_pop_checked(Register::RAX, predicate)?; // Left

        // cmp rax, rbx
        self.emit_cmp_reg(Register::RAX, Register::RBX);

        // setCC al
        self.emit_setcc(predicate);

        // movzx rax, al
        self.emit_movzx();

        self.emit_push(Register::RAX);

        Ok(())
    }

    fn compile_eq(&mut self) -> Result<(), String> {
        self.compile_comparison("e")
    }

    fn compile_not_eq(&mut self) -> Result<(), String> {
        self.compile_comparison("ne")
    }

    fn compile_lt(&mut self) -> Result<(), String> {
        self.compile_comparison("l")
    }

    fn compile_gt(&mut self) -> Result<(), String> {
        self.compile_comparison("g")
    }

    fn compile_lt_eq(&mut self) -> Result<(), String> {
        self.compile_comparison("le")
    }

    fn compile_gt_eq(&mut self) -> Result<(), String> {
        self.compile_comparison("ge")
    }

    /// Compile StoreLocal instruction
    fn compile_store_local(&mut self, name: &str) -> Result<(), String> {
        self.emit_pop_checked(Register::RAX, "StoreLocal")?;

        // Allocate stack space if variable doesn't exist
        if !self.variables.contains_key(name) {
            self.stack_offset -= 8;
            self.variables.insert(name.to_string(), self.stack_offset);
        }

        let offset = self.variables[name];

        // mov [rbp + offset], rax
        self.emit_mov_to_stack(offset, Register::RAX);

        Ok(())
    }

    /// Compile LoadLocal instruction
    fn compile_load_local(&mut self, name: &str) -> Result<(), String> {
        let offset = self
            .variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;

        // mov rax, [rbp + offset]
        self.emit_mov_from_stack(Register::RAX, *offset);
        self.emit_push(Register::RAX);

        Ok(())
    }

    /// Compile LoadGlobal instruction
    fn compile_load_global(&mut self, name: &str) -> Result<(), String> {
        // Check if this is a function name
        if let Some(&func_addr) = self.function_addresses.get(name) {
            // Load function address directly
            self.emit_mov_imm(Register::RAX, func_addr as i64);
            self.emit_push(Register::RAX);
            return Ok(());
        }

        // Otherwise, load from runtime globals
        let name_offset = self.add_data_string(name);

        // 1. Load runtime pointer from [rbp - 8] into RCX/RDI
        #[cfg(windows)]
        let arg1_reg = Register::RCX;
        #[cfg(not(windows))]
        let arg1_reg = Register::RDI;

        self.emit_mov_from_stack(arg1_reg, -8);

        // 2. Load name address into RDX/RSI using LEA RIP-relative
        #[cfg(windows)]
        let arg2_reg = Register::RDX;
        #[cfg(not(windows))]
        let arg2_reg = Register::RSI;

        let patch_pos = self.code.len() + 3; // Offset in lea reg, [rip + disp32]
        self.emit_lea_rip(arg2_reg, 0);
        self.pending_data_patches.push((patch_pos, name_offset));

        // 3. Call get_global_fn (at offset 16 in NativeRuntime)
        // [runtime_ptr + 16] is get_global_fn
        self.emit_mov_from_reg_offset(Register::R10, arg1_reg, 16);
        self.emit_call_reg(Register::R10);

        // 4. Result is in RAX, push it
        self.emit_push(Register::RAX);

        Ok(())
    }

    /// Compile StoreGlobal instruction
    fn compile_store_global(&mut self, name: &str) -> Result<(), String> {
        let name_offset = self.add_data_string(name);

        // 1. Pop value to store into R8/RDX (3rd arg)
        self.emit_pop_checked(Register::RAX, "StoreGlobal")?;
        #[cfg(windows)]
        let arg3_reg = Register::R8;
        #[cfg(not(windows))]
        let arg3_reg = Register::RDX;
        self.emit_mov_reg(arg3_reg, Register::RAX);

        // 2. Load runtime pointer from [rbp - 8] into RCX/RDI
        #[cfg(windows)]
        let arg1_reg = Register::RCX;
        #[cfg(not(windows))]
        let arg1_reg = Register::RDI;
        self.emit_mov_from_stack(arg1_reg, -8);

        // 3. Load name address into RDX/RSI (2nd arg)
        #[cfg(windows)]
        let arg2_reg = Register::RDX;
        #[cfg(not(windows))]
        let arg2_reg = Register::RSI;

        let patch_pos = self.code.len() + 3;
        self.emit_lea_rip(arg2_reg, 0);
        self.pending_data_patches.push((patch_pos, name_offset));

        // 4. Call set_global_fn (at offset 24 in NativeRuntime)
        self.emit_mov_from_reg_offset(Register::R10, arg1_reg, 24);
        self.emit_call_reg(Register::R10);

        Ok(())
    }

    /// Add a string to the data section and return its future absolute address
    /// (Note: For JIT, we'll need to patch this address later)
    fn add_data_string(&mut self, s: &str) -> usize {
        let offset = self.data.len();
        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0); // Null terminator
        offset
    }

    /// Compile Jump instruction
    fn compile_jump(&mut self, target: usize) -> Result<(), String> {
        // jmp target (will be patched later)
        let jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder offset
        self.pending_jumps.push((jump_pos, target, 5)); // 5 bytes for jmp rel32

        Ok(())
    }

    /// Compile JumpIfFalse instruction
    fn compile_jump_if_false(&mut self, target: usize) -> Result<(), String> {
        self.emit_pop_checked(Register::RAX, "JumpIfFalse")?;

        // test rax, rax
        self.emit_test_reg(Register::RAX);

        // je target (jump if zero)
        let jump_pos = self.code.len();
        self.emit_je(0); // Placeholder offset
        self.pending_jumps.push((jump_pos, target, 6)); // 6 bytes for je rel32

        Ok(())
    }

    /// Compile Print instruction
    fn compile_print(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RAX, "Print")?; // Value to print

        // Move value to first argument register (Windows: RCX, Linux: RDI)
        #[cfg(windows)]
        self.emit_mov_reg(Register::RCX, Register::RAX);
        #[cfg(not(windows))]
        self.emit_mov_reg(Register::RDI, Register::RAX);

        // Call print_int
        // We use a fixed address for now, or we'd need to pass it in
        let print_addr = crate::runtime::NativeRuntime::print_int as *const () as usize as i64;
        self.emit_mov_imm(Register::R10, print_addr);
        self.emit_call_reg(Register::R10);

        Ok(())
    }

    /// Compile Call instruction
    fn compile_call(&mut self, arg_count: usize) -> Result<(), String> {
        // System V AMD64 ABI calling convention:
        // Arguments in: RDI, RSI, RDX, RCX, R8, R9, then stack (Linux/macOS)
        // Windows x64: RCX, RDX, R8, R9, then stack
        // First argument is always runtime pointer
        // Return value in RAX

        #[cfg(not(windows))]
        let arg_regs = [
            Register::RDI,
            Register::RSI,
            Register::RDX,
            Register::RCX,
            Register::R8,
            Register::R9,
        ];

        #[cfg(windows)]
        let arg_regs = [Register::RCX, Register::RDX, Register::R8, Register::R9];

        let max_register_args = arg_regs.len().saturating_sub(1);

        // Need all call arguments plus callee on stack before popping.
        self.ensure_stack_items(arg_count + 1, "Call", &Self::ctx_arg_count(arg_count))?;

        // Pop function name/address from stack
        self.emit_pop_checked(Register::R11, "Call")?; // Function address or name

        // Pop arguments from stack (in reverse order) and place in registers
        // We need to pop them into temporary storage first
        let mut temp_stack_offset = self.stack_offset;
        for _ in 0..arg_count {
            temp_stack_offset -= 8;
            self.emit_pop_checked(Register::RAX, "Call")?;
            self.emit_mov_to_stack(temp_stack_offset, Register::RAX);
        }

        // Load runtime pointer into first argument register
        self.emit_mov_from_stack(arg_regs[0], -8);

        // Load arguments from temporary storage into registers
        let mut current_offset = temp_stack_offset;
        for i in 0..arg_count.min(arg_regs.len() - 1) {
            self.emit_mov_from_stack(arg_regs[i + 1], current_offset);
            current_offset += 8;
        }

        // SysV only: pass extra arguments on stack, right-to-left.
        #[cfg(not(windows))]
        let stack_arg_count = arg_count.saturating_sub(max_register_args);
        #[cfg(windows)]
        let stack_arg_count = arg_count.saturating_sub(max_register_args);

        #[cfg(not(windows))]
        for arg_idx in (max_register_args..arg_count).rev() {
            let offset = temp_stack_offset + (arg_idx as i32 * 8);
            self.emit_mov_from_stack(Register::RAX, offset);
            self.emit_push(Register::RAX);
        }

        #[cfg(windows)]
        for arg_idx in (max_register_args..arg_count).rev() {
            let offset = temp_stack_offset + (arg_idx as i32 * 8);
            self.emit_mov_from_stack(Register::RAX, offset);
            self.emit_push(Register::RAX);
        }

        // Windows x64 requires 32-byte shadow space reserved by caller.
        #[cfg(windows)]
        self.emit_sub_imm(Register::RSP, 32);

        // Call the function
        // For now, we assume R11 contains the function address
        self.emit_call_reg(Register::R11);

        #[cfg(not(windows))]
        {
            if stack_arg_count > 0 {
                self.emit_add_imm(Register::RSP, (stack_arg_count * 8) as i32);
            }
        }

        #[cfg(windows)]
        {
            let cleanup = 32 + (stack_arg_count * 8) as i32;
            self.emit_add_imm(Register::RSP, cleanup);
        }

        // Push return value
        self.emit_push(Register::RAX);

        Ok(())
    }

    /// Compile Return instruction
    fn compile_return(&mut self) -> Result<(), String> {
        // Pop return value from stack if present
        if self.stack_depth > 0 {
            self.emit_pop(Register::RAX);
        } else {
            // Default return 0 (Unit)
            self.emit_mov_imm(Register::RAX, 0);
        }

        // Emit function epilogue (restores RBP and returns)
        // Note: Don't call emit_epilogue here as it's already called at the end of main
        // Instead, just emit the return instruction
        self.code.push(0xC3); // ret

        Ok(())
    }

    /// Compile Pop instruction
    fn compile_pop(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RAX, "Pop")?;
        Ok(())
    }

    /// Compile BackendCall instruction
    /// Current native backend does not support external backend bridge yet.
    /// We still lower the instruction and fail explicitly at runtime.
    fn compile_backend_call(
        &mut self,
        backend: &str,
        method: &str,
        arg_count: usize,
    ) -> Result<(), String> {
        self.ensure_stack_items(
            arg_count,
            "BackendCall",
            &Self::ctx_backend_call(backend, method, arg_count),
        )?;

        let expected_math_arity = match (backend, method) {
            ("math", "add" | "sub" | "mul" | "div") => Some(2),
            ("math", "neg") => Some(1),
            _ => None,
        };
        if let Some(expected_arity) = expected_math_arity {
            if arg_count != expected_arity {
                self.pop_n_for_backend_call(arg_count, backend, method)?;
                let panic_msg = format!(
                    "Invalid arity for backend call {}.{}: expected {}, got {}",
                    backend, method, expected_arity, arg_count
                );
                self.emit_panic_message(&panic_msg);
                return Ok(());
            }
        }

        if backend == "math" {
            match (method, arg_count) {
                ("add", 2) => {
                    self.pop_for_backend_call(Register::RBX, backend, method)?; // rhs
                    self.pop_for_backend_call(Register::RAX, backend, method)?; // lhs
                    self.emit_add_reg(Register::RAX, Register::RBX);
                    let add_overflow_jump_pos = self.code.len();
                    self.emit_jo(0); // Placeholder to panic path
                    self.emit_push(Register::RAX);

                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0); // Placeholder

                    let panic_pos = self.code.len();
                    self.patch_jcc_rel32(add_overflow_jump_pos, panic_pos);
                    self.emit_panic_message("Overflow in math.add");

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(end_jump_pos, end_pos);
                    return Ok(());
                }
                ("sub", 2) => {
                    self.pop_for_backend_call(Register::RBX, backend, method)?; // rhs
                    self.pop_for_backend_call(Register::RAX, backend, method)?; // lhs
                    self.emit_sub_reg(Register::RAX, Register::RBX);
                    let sub_overflow_jump_pos = self.code.len();
                    self.emit_jo(0); // Placeholder to panic path
                    self.emit_push(Register::RAX);

                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0); // Placeholder

                    let panic_pos = self.code.len();
                    self.patch_jcc_rel32(sub_overflow_jump_pos, panic_pos);
                    self.emit_panic_message("Overflow in math.sub");

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(end_jump_pos, end_pos);
                    return Ok(());
                }
                ("mul", 2) => {
                    self.pop_for_backend_call(Register::RBX, backend, method)?; // rhs
                    self.pop_for_backend_call(Register::RAX, backend, method)?; // lhs
                    self.emit_mul_reg(Register::RAX, Register::RBX);
                    let mul_overflow_jump_pos = self.code.len();
                    self.emit_jo(0); // Placeholder to panic path
                    self.emit_push(Register::RAX);

                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0); // Placeholder

                    let panic_pos = self.code.len();
                    self.patch_jcc_rel32(mul_overflow_jump_pos, panic_pos);
                    self.emit_panic_message("Overflow in math.mul");

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(end_jump_pos, end_pos);
                    return Ok(());
                }
                ("div", 2) => {
                    self.pop_for_backend_call(Register::RBX, backend, method)?; // rhs
                    self.pop_for_backend_call(Register::RAX, backend, method)?; // lhs

                    // Division by zero guard.
                    self.emit_test_reg(Register::RBX);
                    let div_zero_jump_pos = self.code.len();
                    self.emit_je(0); // Placeholder to panic path

                    // Signed division overflow guard: i64::MIN / -1.
                    self.emit_mov_imm(Register::RCX, i64::MIN);
                    self.emit_cmp_reg(Register::RAX, Register::RCX);
                    let maybe_overflow_jump_pos = self.code.len();
                    self.emit_jne(0); // Skip overflow check when lhs != i64::MIN
                    self.emit_cmp_imm(Register::RBX, -1);
                    let div_overflow_jump_pos = self.code.len();
                    self.emit_je(0); // Placeholder to overflow panic path

                    let continue_div_pos = self.code.len();
                    self.emit_cqo();
                    self.emit_div_reg(Register::RBX);
                    self.emit_push(Register::RAX);

                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0); // Placeholder

                    let div_zero_panic_pos = self.code.len();
                    self.patch_jcc_rel32(div_zero_jump_pos, div_zero_panic_pos);
                    self.patch_jcc_rel32(maybe_overflow_jump_pos, continue_div_pos);
                    self.emit_panic_message("Division by zero in math.div");

                    let div_overflow_panic_pos = self.code.len();
                    self.patch_jcc_rel32(div_overflow_jump_pos, div_overflow_panic_pos);
                    self.emit_panic_message("Overflow in math.div");

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(end_jump_pos, end_pos);
                    return Ok(());
                }
                ("neg", 1) => {
                    self.pop_for_backend_call(Register::RAX, backend, method)?;

                    // Negating i64::MIN overflows in two's complement.
                    self.emit_mov_imm(Register::RCX, i64::MIN);
                    self.emit_cmp_reg(Register::RAX, Register::RCX);
                    let neg_overflow_jump_pos = self.code.len();
                    self.emit_je(0); // Placeholder to panic path

                    // neg rax
                    self.code.extend_from_slice(&[0x48, 0xF7, 0xD8]);
                    self.emit_push(Register::RAX);

                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0); // Placeholder

                    let panic_pos = self.code.len();
                    self.patch_jcc_rel32(neg_overflow_jump_pos, panic_pos);
                    self.emit_panic_message("Overflow in math.neg");

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(end_jump_pos, end_pos);
                    return Ok(());
                }
                _ => {}
            }
        }

        // Pop backend call args to keep stack shape deterministic on unsupported paths.
        self.pop_n_for_backend_call(arg_count, backend, method)?;

        let panic_msg = format!(
            "BackendCall not supported in native runtime: {}.{}",
            backend, method
        );
        self.emit_panic_message(&panic_msg);

        Ok(())
    }

    fn pop_for_backend_call(
        &mut self,
        reg: Register,
        backend: &str,
        method: &str,
    ) -> Result<(), String> {
        let context = format!("backend={}.{}", backend, method);
        self.ensure_stack_items(1, "BackendCall", &context)?;
        self.emit_pop(reg);
        Ok(())
    }

    fn pop_n_for_backend_call(
        &mut self,
        count: usize,
        backend: &str,
        method: &str,
    ) -> Result<(), String> {
        for _ in 0..count {
            self.pop_for_backend_call(Register::RAX, backend, method)?;
        }
        Ok(())
    }

    fn emit_panic_message(&mut self, message: &str) {
        let msg_offset = self.add_data_string(message);
        let patch_pos = self.code.len() + 3;
        self.emit_lea_rip(Register::RDI, 0);
        self.pending_data_patches.push((patch_pos, msg_offset));
        self.emit_mov_imm(Register::RSI, message.len() as i64);
        self.emit_call_runtime("matter_panic");
    }

    fn patch_jcc_rel32(&mut self, jump_pos: usize, target_pos: usize) {
        let offset = (target_pos as i32) - (jump_pos as i32) - 6;
        self.code[jump_pos + 2..jump_pos + 6].copy_from_slice(&offset.to_le_bytes());
    }

    fn patch_jmp_rel32(&mut self, jump_pos: usize, target_pos: usize) {
        let offset = (target_pos as i32) - (jump_pos as i32) - 5;
        self.code[jump_pos + 1..jump_pos + 5].copy_from_slice(&offset.to_le_bytes());
    }

    // ============================================================================
    // REGISTER MANAGEMENT
    // ============================================================================

    /// Emit: push reg
    fn emit_push(&mut self, reg: Register) {
        // Opcode: 0x50 + reg
        // REX.B if needed
        if reg.encoding() >= 8 {
            self.code.push(0x41);
        }
        self.code.push(0x50 + (reg.encoding() & 7));
        self.stack_depth += 1;
    }

    /// Emit: pop reg
    fn emit_pop(&mut self, reg: Register) {
        // Opcode: 0x58 + reg
        // REX.B if needed
        if reg.encoding() >= 8 {
            self.code.push(0x41);
        }
        self.code.push(0x58 + (reg.encoding() & 7));
        self.stack_depth -= 1;
    }

    fn emit_pop_checked(&mut self, reg: Register, instruction: &str) -> Result<(), String> {
        if self.stack_depth <= 0 {
            return Err(format!(
                "Stack underflow while compiling instruction {}",
                instruction
            ));
        }
        self.emit_pop(reg);
        Ok(())
    }

    fn ensure_stack_items(
        &self,
        needed: usize,
        instruction: &str,
        context: &str,
    ) -> Result<(), String> {
        let available = self.stack_depth.max(0) as usize;
        if available < needed {
            return Err(format!(
                "Stack underflow while compiling instruction {} [context:{}]: needed {}, available {}",
                instruction, context, needed, available
            ));
        }
        Ok(())
    }

    // ============================================================================
    // CODE EMISSION
    // ============================================================================

    /// Emit function prologue
    fn emit_prologue(&mut self) {
        // push rbp
        self.code.push(0x55);

        // mov rbp, rsp
        self.code.extend_from_slice(&[0x48, 0x89, 0xE5]);

        // sub rsp, 32 (shadow space for Windows + saving runtime ptr)
        self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x20]);

        // Save NativeRuntime pointer (1st arg) to stack at [rbp - 8]
        #[cfg(windows)]
        self.emit_mov_to_stack(-8, Register::RCX);
        #[cfg(not(windows))]
        self.emit_mov_to_stack(-8, Register::RDI);

        self.stack_offset = -8; // Start locals after runtime ptr
    }

    /// Emit function epilogue
    fn emit_epilogue(&mut self) {
        // mov rsp, rbp
        self.code.extend_from_slice(&[0x48, 0x89, 0xEC]);

        // pop rbp
        self.code.push(0x5D);

        // ret
        self.code.push(0xC3);
    }

    /// Emit: mov reg, imm64
    fn emit_mov_imm(&mut self, dest: Register, value: i64) {
        // REX.W + B (if needed)
        let rex = 0x48 | if dest.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0xB8 + reg
        self.code.push(0xB8 + (dest.encoding() & 7));

        // Immediate value (8 bytes, little-endian)
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    /// Emit: mov dest, src
    fn emit_mov_reg(&mut self, dest: Register, src: Register) {
        // REX.W + R + B
        let rex = 0x48
            | if src.encoding() >= 8 { 4 } else { 0 }
            | if dest.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x89 (mov r/m64, r64)
        self.code.push(0x89);

        // ModR/M: 11 (register mode) + src + dest
        let modrm = 0xC0 | ((src.encoding() & 7) << 3) | (dest.encoding() & 7);
        self.code.push(modrm);
    }

    /// Emit: add dest, src
    fn emit_add_reg(&mut self, dest: Register, src: Register) {
        // REX.W + R + B
        let rex = 0x48
            | if src.encoding() >= 8 { 4 } else { 0 }
            | if dest.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x01 (add r/m64, r64)
        self.code.push(0x01);

        // ModR/M
        let modrm = 0xC0 | ((src.encoding() & 7) << 3) | (dest.encoding() & 7);
        self.code.push(modrm);
    }

    /// Emit: sub dest, src
    fn emit_sub_reg(&mut self, dest: Register, src: Register) {
        // REX.W + R + B
        let rex = 0x48
            | if src.encoding() >= 8 { 4 } else { 0 }
            | if dest.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x29 (sub r/m64, r64)
        self.code.push(0x29);

        // ModR/M
        let modrm = 0xC0 | ((src.encoding() & 7) << 3) | (dest.encoding() & 7);
        self.code.push(modrm);
    }

    /// Emit: imul dest, src
    fn emit_mul_reg(&mut self, dest: Register, src: Register) {
        // REX.W + R + B
        let rex = 0x48
            | if dest.encoding() >= 8 { 4 } else { 0 }
            | if src.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x0F 0xAF (imul r64, r/m64)
        self.code.extend_from_slice(&[0x0F, 0xAF]);

        // ModR/M
        let modrm = 0xC0 | ((dest.encoding() & 7) << 3) | (src.encoding() & 7);
        self.code.push(modrm);
    }

    /// Emit: cqo (sign extend RAX to RDX:RAX)
    fn emit_cqo(&mut self) {
        self.code.extend_from_slice(&[0x48, 0x99]);
    }

    /// Emit: idiv src
    fn emit_div_reg(&mut self, src: Register) {
        // REX.W + B
        let rex = 0x48 | if src.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0xF7 (idiv r/m64)
        self.code.push(0xF7);

        // ModR/M: /7 (idiv) + src
        let modrm = 0xF8 | (src.encoding() & 7);
        self.code.push(modrm);
    }

    /// Emit: cmp left, right
    fn emit_cmp_reg(&mut self, left: Register, right: Register) {
        // REX.W + R + B
        let rex = 0x48
            | if right.encoding() >= 8 { 4 } else { 0 }
            | if left.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x39 (cmp r/m64, r64)
        self.code.push(0x39);

        // ModR/M
        let modrm = 0xC0 | ((right.encoding() & 7) << 3) | (left.encoding() & 7);
        self.code.push(modrm);
    }

    /// Emit: cmp reg, imm
    fn emit_cmp_imm(&mut self, reg: Register, value: i64) {
        // REX.W + B
        let rex = 0x48 | if reg.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        if (-128..=127).contains(&value) {
            // Opcode: 0x83 /7 (cmp r/m64, imm8)
            self.code.push(0x83);
            let modrm = 0xF8 | (reg.encoding() & 7);
            self.code.push(modrm);
            self.code.push(value as u8);
        } else {
            // Opcode: 0x81 /7 (cmp r/m64, imm32)
            self.code.push(0x81);
            let modrm = 0xF8 | (reg.encoding() & 7);
            self.code.push(modrm);
            self.code.extend_from_slice(&(value as i32).to_le_bytes());
        }
    }

    /// Emit: setCC al
    fn emit_setcc(&mut self, condition: &str) {
        let opcode = match condition {
            "e" => 0x94,  // sete
            "ne" => 0x95, // setne
            "l" => 0x9C,  // setl
            "g" => 0x9F,  // setg
            "le" => 0x9E, // setle
            "ge" => 0x9D, // setge
            _ => 0x94,    // default to sete
        };

        // Opcode: 0x0F 0x9X (setCC r/m8)
        self.code.extend_from_slice(&[0x0F, opcode]);

        // ModR/M: AL (register 0)
        self.code.push(0xC0);
    }

    /// Emit: movzx rax, al
    fn emit_movzx(&mut self) {
        // REX.W
        self.code.push(0x48);

        // Opcode: 0x0F 0xB6 (movzx r64, r/m8)
        self.code.extend_from_slice(&[0x0F, 0xB6]);

        // ModR/M: RAX, AL
        self.code.push(0xC0);
    }

    /// Emit: test reg, reg
    fn emit_test_reg(&mut self, reg: Register) {
        // REX.W + R + B
        let rex = 0x48
            | if reg.encoding() >= 8 { 4 } else { 0 }
            | if reg.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x85 (test r/m64, r64)
        self.code.push(0x85);

        // ModR/M: reg, reg
        let modrm = 0xC0 | ((reg.encoding() & 7) << 3) | (reg.encoding() & 7);
        self.code.push(modrm);
    }

    /// Emit: mov [rbp + offset], reg
    fn emit_mov_to_stack(&mut self, offset: i32, src: Register) {
        // REX.W + R
        let rex = 0x48 | if src.encoding() >= 8 { 4 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x89 (mov r/m64, r64)
        self.code.push(0x89);

        // ModR/M: [rbp + disp32] + src
        let modrm = 0x85 | ((src.encoding() & 7) << 3);
        self.code.push(modrm);

        // Displacement (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: mov reg, [rbp + offset]
    fn emit_mov_from_stack(&mut self, dest: Register, offset: i32) {
        // REX.W + R
        let rex = 0x48 | if dest.encoding() >= 8 { 4 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x8B (mov r64, r/m64)
        self.code.push(0x8B);

        // ModR/M: [rbp + disp32] + dest
        let modrm = 0x85 | ((dest.encoding() & 7) << 3);
        self.code.push(modrm);

        // Displacement (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: mov reg, [other_reg + offset]
    fn emit_mov_from_reg_offset(&mut self, dest: Register, src: Register, offset: i32) {
        // REX.W + R + B
        let rex = 0x48
            | if dest.encoding() >= 8 { 4 } else { 0 }
            | if src.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x8B (mov r64, r/m64)
        self.code.push(0x8B);

        // ModR/M: [reg + disp32] + dest
        let modrm = 0x80 | ((dest.encoding() & 7) << 3) | (src.encoding() & 7);
        self.code.push(modrm);

        // Displacement (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: jmp offset
    fn emit_jmp(&mut self, offset: i32) {
        // Opcode: 0xE9 (jmp rel32)
        self.code.push(0xE9);

        // Offset (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: je offset
    fn emit_je(&mut self, offset: i32) {
        // Opcode: 0x0F 0x84 (je rel32)
        self.code.extend_from_slice(&[0x0F, 0x84]);

        // Offset (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: jne offset
    fn emit_jne(&mut self, offset: i32) {
        // Opcode: 0x0F 0x85 (jne rel32)
        self.code.extend_from_slice(&[0x0F, 0x85]);

        // Offset (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: jl offset
    fn emit_jl(&mut self, offset: i32) {
        // Opcode: 0x0F 0x8C (jl rel32)
        self.code.extend_from_slice(&[0x0F, 0x8C]);

        // Offset (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: jo offset
    fn emit_jo(&mut self, offset: i32) {
        // Opcode: 0x0F 0x80 (jo rel32)
        self.code.extend_from_slice(&[0x0F, 0x80]);

        // Offset (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: jge offset (jump if greater or equal)
    fn emit_jge(&mut self, offset: i32) {
        // Opcode: 0x0F 0x8D (jge rel32)
        self.code.extend_from_slice(&[0x0F, 0x8D]);

        // Offset (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: call reg
    fn emit_call_reg(&mut self, reg: Register) {
        // Opcode: 0xFF /2 (call r/m64)
        // REX.B if needed
        if reg.encoding() >= 8 {
            self.code.push(0x41);
        }
        self.code.push(0xFF);
        self.code.push(0xD0 + (reg.encoding() & 7));
    }

    /// Emit: lea reg, [rip + offset]
    fn emit_lea_rip(&mut self, dest: Register, offset: i32) {
        // REX.W + R
        let rex = 0x48 | if dest.encoding() >= 8 { 4 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x8D
        self.code.push(0x8D);

        // ModR/M: [RIP + disp32] + dest
        let modrm = 0x05 | ((dest.encoding() & 7) << 3);
        self.code.push(modrm);

        // Offset (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Patch all pending jumps
    fn patch_jumps(&mut self) -> Result<(), String> {
        let pending = self.pending_jumps.clone();
        for (jump_pos, target_ip, instr_len) in pending {
            let target_offset = self
                .jump_targets
                .get(&target_ip)
                .ok_or_else(|| format!("Jump target {} not found", target_ip))?;

            // Calculate relative offset
            // offset = target - (jump_pos + instr_len)
            let relative_offset = (*target_offset as i32) - (jump_pos as i32) - (instr_len as i32);

            // Patch the offset in the code (it starts after the opcode)
            let offset_start = if instr_len == 5 {
                jump_pos + 1
            } else {
                jump_pos + 2
            };
            let offset_bytes = relative_offset.to_le_bytes();
            self.code[offset_start] = offset_bytes[0];
            self.code[offset_start + 1] = offset_bytes[1];
            self.code[offset_start + 2] = offset_bytes[2];
            self.code[offset_start + 3] = offset_bytes[3];
        }

        Ok(())
    }

    /// Patch all pending data offsets
    fn patch_data_offsets(&mut self, code_len: usize) -> Result<(), String> {
        for (patch_pos, data_offset) in &self.pending_data_patches {
            // Absolute address of data = base + code_len + data_offset
            // Instruction ends at patch_pos + 4
            // Relative offset = (code_len + data_offset) - (patch_pos + 4)
            let relative_offset = (code_len as i32 + *data_offset as i32) - (*patch_pos as i32 + 4);

            let offset_bytes = relative_offset.to_le_bytes();
            self.code[*patch_pos] = offset_bytes[0];
            self.code[*patch_pos + 1] = offset_bytes[1];
            self.code[*patch_pos + 2] = offset_bytes[2];
            self.code[*patch_pos + 3] = offset_bytes[3];
        }

        Ok(())
    }

    // ============================================================================
    // SPRINT 26 PHASE 4: DATA STRUCTURES
    // ============================================================================

    /// Compile NewList instruction
    /// Creates a list with N elements from stack
    /// Memory layout: [type_tag(8) | length(8) | capacity(8) | data_ptr(8)]
    fn compile_new_list(&mut self, count: usize) -> Result<(), String> {
        self.ensure_stack_items(count, "NewList", &format!("element_count={}", count))?;

        // 1. Allocate list structure (32 bytes)
        self.emit_mov_imm(Register::RDI, 32);
        self.emit_call_runtime("matter_alloc");

        // Allocation must succeed.
        self.emit_test_reg(Register::RAX);
        let list_alloc_fail_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder to panic path

        // Save list pointer in R15
        self.emit_mov_reg(Register::R15, Register::RAX);

        // 2. Set type tag (0x01 = List)
        self.emit_mov_imm(Register::RBX, 0x01);
        self.emit_mov_to_mem(Register::R15, 0, Register::RBX);

        // 3. Set length
        self.emit_mov_imm(Register::RBX, count as i64);
        self.emit_mov_to_mem(Register::R15, 8, Register::RBX);

        // 4. Set capacity
        self.emit_mov_imm(Register::RBX, count as i64);
        self.emit_mov_to_mem(Register::R15, 16, Register::RBX);

        // 5. Allocate data array
        self.emit_mov_imm(Register::RDI, (count * 8) as i64);
        self.emit_call_runtime("matter_alloc");

        // Allocation must succeed.
        self.emit_test_reg(Register::RAX);
        let data_alloc_fail_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder to panic path

        // Store data pointer
        self.emit_mov_to_mem(Register::R15, 24, Register::RAX);

        // 6. Pop elements from stack and store in reverse order
        for i in (0..count).rev() {
            self.emit_pop_checked(Register::RBX, "NewList")?;
            // Load data pointer
            self.emit_mov_from_mem(Register::RCX, Register::R15, 24);
            // Calculate offset
            let offset = (i * 8) as i32;
            // Store element
            self.emit_mov_to_mem_offset(Register::RCX, offset, Register::RBX);
        }

        // 7. Push list pointer
        self.emit_push(Register::R15);

        // Jump over panic paths
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        // list alloc fail panic
        let list_fail_pos = self.code.len();
        self.patch_jcc_rel32(list_alloc_fail_jump_pos, list_fail_pos);
        self.emit_panic_message("List allocation failed");

        // data alloc fail panic
        let data_fail_pos = self.code.len();
        self.patch_jcc_rel32(data_alloc_fail_jump_pos, data_fail_pos);
        self.emit_panic_message("List data allocation failed");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile LoadIndex instruction
    /// Pop index, pop list, push value
    /// Includes bounds checking with panic on out-of-bounds access
    fn compile_load_index(&mut self) -> Result<(), String> {
        self.ensure_stack_items(2, "LoadIndex", &Self::ctx_operands("list,index"))?;

        // 1. Pop index
        self.emit_pop_checked(Register::RBX, "LoadIndex")?;

        // 2. Pop list
        self.emit_pop_checked(Register::RAX, "LoadIndex")?;

        // 2a. Type check: must be List (0x01)
        self.emit_mov_from_mem(Register::RDX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RDX, 0x01);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder

        // 3. Bounds check: load length
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 8);

        // 4a. Negative index is invalid.
        self.emit_cmp_imm(Register::RBX, 0);
        let panic_negative_jump_pos = self.code.len();
        self.emit_jl(0); // Placeholder

        // 4b. Compare index < length (if index >= length, panic)
        self.emit_cmp_reg(Register::RBX, Register::RCX);

        // 5. jge .panic (jump if index >= length)
        let panic_jump_pos = self.code.len();
        self.emit_jge(0); // Placeholder

        // === NORMAL PATH: Load value ===
        // 6. Load data pointer
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 24);

        // 7. Calculate offset (index * 8)
        self.emit_shl_imm(Register::RBX, 3);

        // 8. Add offset to data pointer
        self.emit_add_reg(Register::RCX, Register::RBX);

        // 9. Load value
        self.emit_mov_from_mem(Register::RAX, Register::RCX, 0);

        // 10. Push value
        self.emit_push(Register::RAX);

        // Jump over panic path
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        // === PANIC PATH ===
        let panic_pos = self.code.len();

        // Patch invalid-type jump
        self.patch_jcc_rel32(type_panic_jump_pos, panic_pos);

        // Patch negative-index jump
        self.patch_jcc_rel32(panic_negative_jump_pos, panic_pos);

        // Patch bounds jump
        self.patch_jcc_rel32(panic_jump_pos, panic_pos);

        // Call matter_panic with explicit message.
        self.emit_panic_message("Index out of bounds or non-list");

        // Patch end jump
        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile StoreIndex instruction
    /// Pop value, pop index, pop list
    fn compile_store_index(&mut self) -> Result<(), String> {
        self.ensure_stack_items(3, "StoreIndex", &Self::ctx_operands("list,index,value"))?;

        // 1. Pop value
        self.emit_pop_checked(Register::R8, "StoreIndex")?;

        // 2. Pop index
        self.emit_pop_checked(Register::RBX, "StoreIndex")?;

        // 3. Pop list
        self.emit_pop_checked(Register::RAX, "StoreIndex")?;

        // 3a. Type check: must be List (0x01)
        self.emit_mov_from_mem(Register::RDX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RDX, 0x01);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder

        // 4a. Negative index is invalid.
        self.emit_cmp_imm(Register::RBX, 0);
        let panic_negative_jump_pos = self.code.len();
        self.emit_jl(0); // Placeholder

        // 4b. Bounds check (if index >= length, panic)
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 8);
        self.emit_cmp_reg(Register::RBX, Register::RCX);
        let panic_jump_pos = self.code.len();
        self.emit_jge(0); // Placeholder

        // 5. Load data pointer
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 24);

        // 6. Calculate offset (index * 8)
        self.emit_shl_imm(Register::RBX, 3);

        // 7. Add offset
        self.emit_add_reg(Register::RCX, Register::RBX);

        // 8. Store value
        self.emit_mov_to_mem(Register::RCX, 0, Register::R8);

        // Jump over panic path
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        // === PANIC PATH ===
        let panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, panic_pos);
        self.patch_jcc_rel32(panic_negative_jump_pos, panic_pos);
        self.patch_jcc_rel32(panic_jump_pos, panic_pos);
        self.emit_panic_message("Index out of bounds (store) or non-list");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile StoreIndexVar instruction
    /// Pop value, pop index, mutate variable collection[index]
    fn compile_store_index_var(&mut self, name: &str) -> Result<(), String> {
        self.ensure_stack_items(
            2,
            "StoreIndexVar",
            &format!("target={},operands=index,value", name),
        )?;

        // Pop value and index
        self.emit_pop_checked(Register::R8, "StoreIndexVar")?;
        self.emit_pop_checked(Register::RBX, "StoreIndexVar")?;

        // Load target collection from variable
        let offset = self
            .variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;
        self.emit_mov_from_stack(Register::RAX, *offset);

        // Dispatch by type: List(0x01) or Map(0x02)
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RCX, 0x01);
        let list_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder
        self.emit_cmp_imm(Register::RCX, 0x02);
        let map_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder

        // Invalid type panic
        let type_panic_msg = format!("Expected list or map variable '{}'", name);
        self.emit_panic_message(&type_panic_msg);

        // Patch list jump
        let list_pos = self.code.len();
        self.patch_jcc_rel32(list_jump_pos, list_pos);

        // List path bounds checks
        self.emit_cmp_imm(Register::RBX, 0);
        let list_oob_jump_pos = self.code.len();
        self.emit_jl(0); // Placeholder
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 8); // len
        self.emit_cmp_reg(Register::RBX, Register::RCX);
        let list_oob2_jump_pos = self.code.len();
        self.emit_jge(0); // Placeholder
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 24); // data ptr
        self.emit_shl_imm(Register::RBX, 3);
        self.emit_add_reg(Register::RCX, Register::RBX);
        self.emit_mov_to_mem(Register::RCX, 0, Register::R8);
        let list_end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        // List OOB panic
        let list_oob_pos = self.code.len();
        self.patch_jcc_rel32(list_oob_jump_pos, list_oob_pos);
        self.patch_jcc_rel32(list_oob2_jump_pos, list_oob_pos);
        self.emit_panic_message("Index out of bounds (store) or non-list");

        // Patch map jump
        let map_pos = self.code.len();
        self.patch_jcc_rel32(map_jump_pos, map_pos);

        // Map path: key is index i64
        self.emit_mov_reg(Register::RDI, Register::RAX); // map
        self.emit_mov_reg(Register::RSI, Register::RBX); // key
        self.emit_mov_reg(Register::RDX, Register::R8); // value
        self.emit_call_runtime("matter_map_insert");

        // Patch list end jump to here
        let end_pos = self.code.len();
        self.patch_jmp_rel32(list_end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile ListPush instruction
    /// Pop value, pop list, push list (mutated)
    fn compile_list_push(&mut self) -> Result<(), String> {
        self.ensure_stack_items(2, "ListPush", &Self::ctx_operands("list,value"))?;

        // 1. Pop value
        self.emit_pop_checked(Register::R8, "ListPush")?;

        // 2. Pop list
        self.emit_pop_checked(Register::RAX, "ListPush")?;

        // 2a. Type check: must be List (0x01)
        self.emit_mov_from_mem(Register::RDX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RDX, 0x01);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder

        // 3. Load length
        self.emit_mov_from_mem(Register::RBX, Register::RAX, 8);

        // 4. Load capacity
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 16);

        // 5. Check if resize needed (length >= capacity)
        self.emit_cmp_reg(Register::RBX, Register::RCX);
        let resize_jump_pos = self.code.len();
        self.emit_jge(0); // Placeholder to resize path

        let skip_resize_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder to skip resize path

        // Resize path
        let resize_path_pos = self.code.len();
        self.patch_jcc_rel32(resize_jump_pos, resize_path_pos);

        // new_capacity = capacity + 1 (simple growth, also handles capacity=0)
        self.emit_mov_reg(Register::RSI, Register::RCX);
        self.emit_add_imm(Register::RSI, 1);
        self.emit_mov_reg(Register::RDI, Register::RAX); // list_ptr
        self.emit_call_runtime("matter_list_resize");

        // If resize fails, panic explicitly instead of continuing with invalid pointers.
        self.emit_test_reg(Register::RAX);
        let resize_fail_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder to panic path

        // Continue path
        let continue_pos = self.code.len();
        self.patch_jmp_rel32(skip_resize_jump_pos, continue_pos);

        let after_check_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder over panic path

        // Resize-failure panic path
        let panic_pos = self.code.len();
        self.patch_jcc_rel32(resize_fail_jump_pos, panic_pos);
        self.emit_panic_message("List resize failed");

        let after_check_pos = self.code.len();
        self.patch_jmp_rel32(after_check_jump_pos, after_check_pos);

        // Non-list panic path
        let type_panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, type_panic_pos);
        self.emit_panic_message("Expected list for push");

        // 6. Load data pointer
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 24);

        // 7. Calculate offset (length * 8)
        self.emit_mov_reg(Register::RDX, Register::RBX);
        self.emit_shl_imm(Register::RDX, 3);

        // 8. Add offset
        self.emit_add_reg(Register::RCX, Register::RDX);

        // 9. Store value
        self.emit_mov_to_mem(Register::RCX, 0, Register::R8);

        // 10. Increment length
        self.emit_add_imm(Register::RBX, 1);
        self.emit_mov_to_mem(Register::RAX, 8, Register::RBX);

        // 11. Push list
        self.emit_push(Register::RAX);

        Ok(())
    }

    /// Compile ListPushVar instruction
    /// Pop value, mutate variable list, push unit
    fn compile_list_push_var(&mut self, name: &str) -> Result<(), String> {
        self.ensure_stack_items(1, "ListPushVar", &format!("target={},operands=value", name))?;

        // Pop value
        self.emit_pop_checked(Register::R8, "ListPushVar")?;

        // Load list from variable
        let offset = self
            .variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;
        self.emit_mov_from_stack(Register::RAX, *offset);

        // Type check: list
        self.emit_mov_from_mem(Register::RDX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RDX, 0x01);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder

        // length/capacity
        self.emit_mov_from_mem(Register::RBX, Register::RAX, 8);
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 16);
        self.emit_cmp_reg(Register::RBX, Register::RCX);
        let resize_jump_pos = self.code.len();
        self.emit_jge(0); // Placeholder
        let skip_resize_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        // Resize path
        let resize_pos = self.code.len();
        self.patch_jcc_rel32(resize_jump_pos, resize_pos);
        self.emit_mov_reg(Register::RSI, Register::RCX);
        self.emit_add_imm(Register::RSI, 1);
        self.emit_mov_reg(Register::RDI, Register::RAX);
        self.emit_call_runtime("matter_list_resize");
        self.emit_test_reg(Register::RAX);
        let resize_fail_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder
        let cont_pos = self.code.len();
        self.patch_jmp_rel32(skip_resize_jump_pos, cont_pos);

        // Store append value
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 24);
        self.emit_mov_reg(Register::RDX, Register::RBX);
        self.emit_shl_imm(Register::RDX, 3);
        self.emit_add_reg(Register::RCX, Register::RDX);
        self.emit_mov_to_mem(Register::RCX, 0, Register::R8);
        self.emit_add_imm(Register::RBX, 1);
        self.emit_mov_to_mem(Register::RAX, 8, Register::RBX);

        // Return unit
        self.emit_mov_imm(Register::RAX, 0);
        self.emit_push(Register::RAX);
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        // Type panic
        let type_panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, type_panic_pos);
        let type_msg = format!("Expected list variable '{}'", name);
        self.emit_panic_message(&type_msg);

        // Resize fail panic
        let resize_fail_pos = self.code.len();
        self.patch_jcc_rel32(resize_fail_jump_pos, resize_fail_pos);
        self.emit_panic_message("List resize failed");

        // Patch function end jump
        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile ListPop instruction
    /// Pop list, push value, push list (mutated)
    fn compile_list_pop(&mut self) -> Result<(), String> {
        self.ensure_stack_items(1, "ListPop", &Self::ctx_operands("list"))?;

        // 1. Pop list
        self.emit_pop_checked(Register::RAX, "ListPop")?;

        // 1a. Type check: must be List (0x01)
        self.emit_mov_from_mem(Register::RDX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RDX, 0x01);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder

        // 2. Load length
        self.emit_mov_from_mem(Register::RBX, Register::RAX, 8);

        // 3. Check if empty (length == 0)
        self.emit_test_reg(Register::RBX);
        let empty_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder to empty path

        // 4. Decrement length
        self.emit_sub_imm(Register::RBX, 1);
        self.emit_mov_to_mem(Register::RAX, 8, Register::RBX);

        // 5. Load data pointer
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 24);

        // 6. Calculate offset (length * 8)
        self.emit_mov_reg(Register::RDX, Register::RBX);
        self.emit_shl_imm(Register::RDX, 3);

        // 7. Add offset
        self.emit_add_reg(Register::RCX, Register::RDX);

        // 8. Load value
        self.emit_mov_from_mem(Register::R8, Register::RCX, 0);

        // 9. Push value
        self.emit_push(Register::R8);

        // 10. Push list
        self.emit_push(Register::RAX);

        // Jump over empty path
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        // Non-list panic path
        let type_panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, type_panic_pos);
        self.emit_panic_message("Expected list for pop");

        // Empty path: VM parity -> error on pop from empty list.
        let empty_path_pos = self.code.len();
        self.patch_jcc_rel32(empty_jump_pos, empty_path_pos);
        self.emit_panic_message("Cannot pop from empty list");

        // Patch end jump
        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile ListPopVar instruction
    /// Mutate variable list, push popped value
    fn compile_list_pop_var(&mut self, name: &str) -> Result<(), String> {
        // Load list from variable
        let offset = self
            .variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;
        self.emit_mov_from_stack(Register::RAX, *offset);

        // Type check: list
        self.emit_mov_from_mem(Register::RDX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RDX, 0x01);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder

        // Empty check
        self.emit_mov_from_mem(Register::RBX, Register::RAX, 8);
        self.emit_test_reg(Register::RBX);
        let empty_panic_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder

        // Decrement len and load popped value
        self.emit_sub_imm(Register::RBX, 1);
        self.emit_mov_to_mem(Register::RAX, 8, Register::RBX);
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 24);
        self.emit_mov_reg(Register::RDX, Register::RBX);
        self.emit_shl_imm(Register::RDX, 3);
        self.emit_add_reg(Register::RCX, Register::RDX);
        self.emit_mov_from_mem(Register::R8, Register::RCX, 0);
        self.emit_push(Register::R8);
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        // Type panic
        let type_panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, type_panic_pos);
        let type_msg = format!("Expected list variable '{}'", name);
        self.emit_panic_message(&type_msg);

        // Empty panic
        let empty_panic_pos = self.code.len();
        self.patch_jcc_rel32(empty_panic_jump_pos, empty_panic_pos);
        self.emit_panic_message("Cannot pop from empty list");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile ListLen instruction
    /// Pop list, push length
    fn compile_list_len(&mut self) -> Result<(), String> {
        self.ensure_stack_items(1, "ListLen", &Self::ctx_operands("list"))?;

        // 1. Pop list
        self.emit_pop_checked(Register::RAX, "ListLen")?;

        // 1a. Type check: must be List (0x01)
        self.emit_mov_from_mem(Register::RDX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RDX, 0x01);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder

        // 2. Load length
        self.emit_mov_from_mem(Register::RBX, Register::RAX, 8);

        // 3. Push length
        self.emit_push(Register::RBX);

        // Jump over panic path
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        // Non-list panic path
        let panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, panic_pos);
        self.emit_panic_message("Expected list for len");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile NewMap instruction
    /// Creates a map with N key/value pairs from stack
    /// Memory layout: [type_tag(8) | size(8) | buckets_ptr(8)]
    fn compile_new_map(&mut self, count: usize) -> Result<(), String> {
        self.ensure_stack_items(count * 2, "NewMap", &format!("pair_count={}", count))?;

        // 1. Call matter_map_new() to create empty map
        self.emit_call_runtime("matter_map_new");

        // Allocation must succeed.
        self.emit_test_reg(Register::RAX);
        let map_alloc_fail_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder to panic path

        // Save map pointer in R15
        self.emit_mov_reg(Register::R15, Register::RAX);

        // 2. Pop key-value pairs and insert using matter_map_insert
        for _ in 0..count {
            // Pop value
            self.emit_pop_checked(Register::RBX, "NewMap")?;
            // Pop key
            self.emit_pop_checked(Register::RCX, "NewMap")?;

            // Call matter_map_insert(map, key, value)
            // System V AMD64 ABI: RDI, RSI, RDX
            self.emit_mov_reg(Register::RDI, Register::R15); // map
            self.emit_mov_reg(Register::RSI, Register::RCX); // key
            self.emit_mov_reg(Register::RDX, Register::RBX); // value
            self.emit_call_runtime("matter_map_insert");
        }

        // 3. Push map pointer
        self.emit_push(Register::R15);

        // Jump over panic path
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        let panic_pos = self.code.len();
        self.patch_jcc_rel32(map_alloc_fail_jump_pos, panic_pos);
        self.emit_panic_message("Map allocation failed");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile MapHas instruction
    /// Pop key, pop map, push bool
    fn compile_map_has(&mut self) -> Result<(), String> {
        self.ensure_stack_items(2, "MapHas", &Self::ctx_operands("map,key"))?;

        // 1. Pop key
        self.emit_pop_checked(Register::RBX, "MapHas")?;

        // 2. Pop map
        self.emit_pop_checked(Register::RAX, "MapHas")?;

        // 2a. Type check: must be Map (0x02)
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RCX, 0x02);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder

        // 3. Call matter_map_has(map, key)
        // System V AMD64 ABI: RDI, RSI
        self.emit_mov_reg(Register::RDI, Register::RAX); // map
        self.emit_mov_reg(Register::RSI, Register::RBX); // key
        self.emit_call_runtime("matter_map_has");

        // 4. Push result (RAX contains bool)
        self.emit_push(Register::RAX);

        // Jump over panic path
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        let panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, panic_pos);
        self.emit_panic_message("Expected map for has");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile MapKeys instruction
    /// Pop map, push list of keys
    fn compile_map_keys(&mut self) -> Result<(), String> {
        self.ensure_stack_items(1, "MapKeys", &Self::ctx_operands("map"))?;

        // 1. Pop map
        self.emit_pop_checked(Register::RAX, "MapKeys")?;

        // 1a. Type check: must be Map (0x02)
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RCX, 0x02);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder

        // 2. Call matter_map_keys(map)
        self.emit_mov_reg(Register::RDI, Register::RAX);
        self.emit_call_runtime("matter_map_keys");

        // 3. Push resulting list pointer
        self.emit_push(Register::RAX);

        // Jump over panic path
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        let panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, panic_pos);
        self.emit_panic_message("Expected map for keys");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile MapValues instruction
    /// Pop map, push list of values
    fn compile_map_values(&mut self) -> Result<(), String> {
        self.ensure_stack_items(1, "MapValues", &Self::ctx_operands("map"))?;

        // 1. Pop map
        self.emit_pop_checked(Register::RAX, "MapValues")?;

        // 1a. Type check: must be Map (0x02)
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RCX, 0x02);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder

        // 2. Call matter_map_values(map)
        self.emit_mov_reg(Register::RDI, Register::RAX);
        self.emit_call_runtime("matter_map_values");

        // 3. Push resulting list pointer
        self.emit_push(Register::RAX);

        // Jump over panic path
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        let panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, panic_pos);
        self.emit_panic_message("Expected map for values");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);
        Ok(())
    }

    /// Compile NewStruct instruction
    /// Creates a struct by materializing a map-like keyed layout.
    /// Stack input is pairs: field_name, value (same order used by bytecode compiler/VM).
    fn compile_new_struct(&mut self, _type_name: &str, field_count: usize) -> Result<(), String> {
        self.ensure_stack_items(
            field_count * 2,
            "NewStruct",
            &format!("field_count={}", field_count),
        )?;

        // 1. Create backing map
        self.emit_call_runtime("matter_map_new");
        self.emit_test_reg(Register::RAX);
        let struct_alloc_fail_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder to panic path
        self.emit_mov_reg(Register::R15, Register::RAX);

        // 2. Insert N pairs by hashed field name
        for _ in 0..field_count {
            // Pop value then field name key
            self.emit_pop_checked(Register::RBX, "NewStruct")?; // value
            self.emit_pop_checked(Register::RCX, "NewStruct")?; // field name hash

            self.emit_mov_reg(Register::RDI, Register::R15); // map
            self.emit_mov_reg(Register::RSI, Register::RCX); // key
            self.emit_mov_reg(Register::RDX, Register::RBX); // value
            self.emit_call_runtime("matter_map_insert");
        }

        // 3. Push struct/map pointer
        self.emit_push(Register::R15);

        // Jump over panic path
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        let panic_pos = self.code.len();
        self.patch_jcc_rel32(struct_alloc_fail_jump_pos, panic_pos);
        self.emit_panic_message("Struct allocation failed");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile LoadField instruction
    /// Pop struct/map, push field value
    /// Supports both Structs (direct offset) and Maps (hash lookup)
    fn compile_load_field(&mut self, field: &str) -> Result<(), String> {
        self.ensure_stack_items(1, "LoadField", &format!("field={}", field))?;

        // 1. Pop struct/map
        self.emit_pop_checked(Register::RAX, "LoadField")?;

        // 2. Load type tag
        self.emit_mov_from_mem(Register::RBX, Register::RAX, 0);

        // 3. Check type: 0x02 = Map, 0x03 = Struct
        self.emit_cmp_imm(Register::RBX, 0x02);

        // 4. Jump if Map (je .map_path)
        let map_jump_pos = self.code.len();
        self.emit_je(0); // Placeholder

        // Non-map path must be Struct; otherwise panic invalid type.
        self.emit_cmp_imm(Register::RBX, 0x03);
        let invalid_type_jump_pos = self.code.len();
        self.emit_jne(0); // Placeholder to invalid-type panic path

        // === STRUCT PATH ===
        // For structs, we use direct offset lookup
        // Field "x" at offset 16, "y" at offset 24, etc.
        if let Some(field_offset) = self.struct_field_offset(field) {
            self.emit_mov_from_mem(Register::RDX, Register::RAX, field_offset);
            self.emit_push(Register::RDX);
        } else {
            let panic_msg = format!("Unknown struct field '{}'", field);
            self.emit_panic_message(&panic_msg);
        }

        // Jump over map path
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // Placeholder

        // === MAP PATH ===
        let map_pos = self.code.len();

        // Patch map jump
        self.patch_jcc_rel32(map_jump_pos, map_pos);

        // Hash field name to i64 key (FNV-1a)
        let field_hash = self.hash_type_name(field);
        self.emit_mov_imm(Register::RBX, field_hash);
        self.emit_mov_reg(Register::R14, Register::RAX); // preserve map pointer

        // First validate key exists (VM parity): missing field is an error path.
        self.emit_mov_reg(Register::RDI, Register::R14); // map
        self.emit_mov_reg(Register::RSI, Register::RBX); // key
        self.emit_call_runtime("matter_map_has");
        self.emit_test_reg(Register::RAX);
        let missing_field_jump_pos = self.code.len();
        self.emit_je(0); // placeholder to panic path

        // Call matter_map_lookup(map, key)
        self.emit_mov_reg(Register::RDI, Register::R14); // map
        self.emit_mov_reg(Register::RSI, Register::RBX); // key
        self.emit_call_runtime("matter_map_lookup");

        self.emit_push(Register::RAX); // found value

        // Jump over panic path
        let end_map_jump_pos = self.code.len();
        self.emit_jmp(0); // placeholder

        // Patch end jump
        let invalid_type_pos = self.code.len();
        self.patch_jcc_rel32(invalid_type_jump_pos, invalid_type_pos);
        self.emit_panic_message("Expected struct or map for field access");

        let panic_path_pos = self.code.len();
        self.patch_jcc_rel32(missing_field_jump_pos, panic_path_pos);

        // Panic: field not found
        let panic_msg = format!("Field '{}' not found", field);
        self.emit_panic_message(&panic_msg);

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);
        self.patch_jmp_rel32(end_map_jump_pos, end_pos);

        Ok(())
    }

    /// Compile StoreFieldVar instruction
    /// Pop value, mutate variable field
    fn compile_store_field_var(&mut self, target: &str, field: &str) -> Result<(), String> {
        self.ensure_stack_items(
            1,
            "StoreFieldVar",
            &format!("target={},field={}", target, field),
        )?;

        // 1. Pop value
        self.emit_pop_checked(Register::RBX, "StoreFieldVar")?;

        // 2. Load struct from variable
        let offset = self
            .variables
            .get(target)
            .ok_or_else(|| format!("Undefined variable: {}", target))?;
        self.emit_mov_from_stack(Register::RAX, *offset);

        // 3. Branch by runtime tag: map-backed struct path or legacy positional struct path.
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0); // type_tag
        self.emit_cmp_imm(Register::RCX, 0x02); // Map
        let map_jump_pos = self.code.len();
        self.emit_je(0); // placeholder

        // Must be Struct for legacy positional write; otherwise panic invalid type.
        self.emit_cmp_imm(Register::RCX, 0x03); // Struct
        let invalid_type_jump_pos = self.code.len();
        self.emit_jne(0); // placeholder

        // Legacy positional struct write
        if let Some(field_offset) = self.struct_field_offset(field) {
            self.emit_mov_to_mem(Register::RAX, field_offset, Register::RBX);
        } else {
            let panic_msg = format!("Unknown struct field '{}'", field);
            self.emit_panic_message(&panic_msg);
        }
        let end_jump_pos = self.code.len();
        self.emit_jmp(0); // placeholder

        // Map-backed write via map_insert(map, hash(field), value)
        let map_pos = self.code.len();
        self.patch_jcc_rel32(map_jump_pos, map_pos);

        let field_hash = self.hash_type_name(field);
        self.emit_mov_imm(Register::RSI, field_hash); // key
        self.emit_mov_reg(Register::RDI, Register::RAX); // map
        self.emit_mov_reg(Register::RDX, Register::RBX); // value
        self.emit_call_runtime("matter_map_insert");

        // Invalid type panic path
        let invalid_type_pos = self.code.len();
        self.patch_jcc_rel32(invalid_type_jump_pos, invalid_type_pos);
        self.emit_panic_message("Expected struct or map variable for field store");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Resolve struct field offset in bytes for legacy positional structs.
    fn struct_field_offset(&self, field: &str) -> Option<i32> {
        match field {
            "x" => Some(16),
            "y" => Some(24),
            "z" => Some(32),
            "w" => Some(40),
            _ => None,
        }
    }

    /// Hash a type name to a unique ID
    fn hash_type_name(&self, name: &str) -> i64 {
        // Simple hash function (FNV-1a)
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash as i64
    }

    /// Emit: call runtime function
    fn emit_call_runtime(&mut self, name: &str) {
        // Get function address from runtime
        let func_addr = match name {
            "matter_alloc" => crate::runtime::builtins::matter_alloc as *const () as usize as i64,
            "matter_list_new" => {
                crate::runtime::builtins::matter_list_new as *const () as usize as i64
            }
            "matter_list_resize" => {
                crate::runtime::builtins::matter_list_resize as *const () as usize as i64
            }
            "matter_map_new" => {
                crate::runtime::builtins::matter_map_new as *const () as usize as i64
            }
            "matter_map_insert" => {
                crate::runtime::builtins::matter_map_insert as *const () as usize as i64
            }
            "matter_map_lookup" => {
                crate::runtime::builtins::matter_map_lookup as *const () as usize as i64
            }
            "matter_map_has" => {
                crate::runtime::builtins::matter_map_has as *const () as usize as i64
            }
            "matter_map_keys" => {
                crate::runtime::builtins::matter_map_keys as *const () as usize as i64
            }
            "matter_map_values" => {
                crate::runtime::builtins::matter_map_values as *const () as usize as i64
            }
            "matter_struct_new" => {
                crate::runtime::builtins::matter_struct_new as *const () as usize as i64
            }
            "matter_panic" => crate::runtime::builtins::matter_panic as *const () as usize as i64,
            _ => {
                // Unknown function, emit NOP
                self.code.push(0x90);
                return;
            }
        };

        // mov r10, func_addr
        self.emit_mov_imm(Register::R10, func_addr);

        // call r10
        self.emit_call_reg(Register::R10);
    }

    /// Emit: mov [reg + offset], value_reg
    fn emit_mov_to_mem(&mut self, base: Register, offset: i32, value: Register) {
        // REX.W + R + B
        let rex = 0x48
            | if value.encoding() >= 8 { 4 } else { 0 }
            | if base.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x89 (mov r/m64, r64)
        self.code.push(0x89);

        // ModR/M: [base + disp32] + value
        let modrm = 0x80 | ((value.encoding() & 7) << 3) | (base.encoding() & 7);
        self.code.push(modrm);

        // Displacement (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: mov [reg + offset], value_reg (with offset calculation)
    fn emit_mov_to_mem_offset(&mut self, base: Register, offset: i32, value: Register) {
        self.emit_mov_to_mem(base, offset, value);
    }

    /// Emit: mov dest, [reg + offset]
    fn emit_mov_from_mem(&mut self, dest: Register, base: Register, offset: i32) {
        // REX.W + R + B
        let rex = 0x48
            | if dest.encoding() >= 8 { 4 } else { 0 }
            | if base.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0x8B (mov r64, r/m64)
        self.code.push(0x8B);

        // ModR/M: [base + disp32] + dest
        let modrm = 0x80 | ((dest.encoding() & 7) << 3) | (base.encoding() & 7);
        self.code.push(modrm);

        // Displacement (4 bytes)
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    /// Emit: shl reg, imm
    fn emit_shl_imm(&mut self, reg: Register, shift: u8) {
        // REX.W + B
        let rex = 0x48 | if reg.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        // Opcode: 0xC1 (shl r/m64, imm8)
        self.code.push(0xC1);

        // ModR/M: /4 (shl) + reg
        let modrm = 0xE0 | (reg.encoding() & 7);
        self.code.push(modrm);

        // Immediate shift amount
        self.code.push(shift);
    }

    /// Emit: add reg, imm
    fn emit_add_imm(&mut self, reg: Register, value: i32) {
        // REX.W + B
        let rex = 0x48 | if reg.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        if (-128..=127).contains(&value) {
            // Opcode: 0x83 (add r/m64, imm8)
            self.code.push(0x83);

            // ModR/M: /0 (add) + reg
            let modrm = 0xC0 | (reg.encoding() & 7);
            self.code.push(modrm);

            // Immediate value (1 byte)
            self.code.push(value as u8);
        } else {
            // Opcode: 0x81 (add r/m64, imm32)
            self.code.push(0x81);

            // ModR/M: /0 (add) + reg
            let modrm = 0xC0 | (reg.encoding() & 7);
            self.code.push(modrm);

            // Immediate value (4 bytes)
            self.code.extend_from_slice(&value.to_le_bytes());
        }
    }

    /// Emit: sub reg, imm
    fn emit_sub_imm(&mut self, reg: Register, value: i32) {
        // REX.W + B
        let rex = 0x48 | if reg.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        if (-128..=127).contains(&value) {
            // Opcode: 0x83 (sub r/m64, imm8)
            self.code.push(0x83);

            // ModR/M: /5 (sub) + reg
            let modrm = 0xE8 | (reg.encoding() & 7);
            self.code.push(modrm);

            // Immediate value (1 byte)
            self.code.push(value as u8);
        } else {
            // Opcode: 0x81 (sub r/m64, imm32)
            self.code.push(0x81);

            // ModR/M: /5 (sub) + reg
            let modrm = 0xE8 | (reg.encoding() & 7);
            self.code.push(modrm);

            // Immediate value (4 bytes)
            self.code.extend_from_slice(&value.to_le_bytes());
        }
    }
}

impl Default for X86CodeGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_opcode(code: &[u8], opcode: &[u8]) -> Option<usize> {
        code.windows(opcode.len()).position(|w| w == opcode)
    }

    fn read_rel32(code: &[u8], start: usize) -> i32 {
        let bytes = [
            code[start],
            code[start + 1],
            code[start + 2],
            code[start + 3],
        ];
        i32::from_le_bytes(bytes)
    }

    fn lcg_next(state: &mut u64) -> u64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *state
    }

    #[test]
    fn test_codegen_creation() {
        let codegen = X86CodeGen::new();
        assert_eq!(codegen.code.len(), 0);
    }

    #[test]
    fn test_simple_arithmetic() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(10));
        let c2 = bytecode.add_constant(Constant::Int(20));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::Add,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_add_with_missing_operands_returns_compile_error() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::Add, Instruction::Halt];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("Add without operands should fail compile");
        assert!(err.contains("Stack underflow while compiling instruction Add"));
    }

    #[test]
    fn test_jump_if_false_without_condition_returns_compile_error() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::JumpIfFalse(1), Instruction::Halt];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("JumpIfFalse without condition should fail compile");
        assert!(err.contains("Stack underflow while compiling instruction JumpIfFalse"));
    }

    #[test]
    fn test_call_without_function_on_stack_returns_compile_error() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::Call(0), Instruction::Halt];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("Call without function should fail compile");
        assert!(err.contains("Stack underflow while compiling instruction Call"));
    }

    #[test]
    fn test_call_with_partial_arguments_returns_compile_error_with_counts() {
        let mut bytecode = Bytecode::new();
        let c_fn = bytecode.add_constant(Constant::Int(123)); // fake callee placeholder
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c_fn),
            Instruction::Call(2),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("Call with partial arguments should fail compile");
        assert!(err.contains("Stack underflow while compiling instruction Call"));
        assert!(err.contains("[context:arg_count=2]"));
        assert!(err.contains("needed 3, available 1"));
    }

    #[test]
    fn test_x86_stack_underflow_error_format_uses_context_block() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::Call(2), Instruction::Halt];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("expected stack underflow on call precheck");

        assert!(err.starts_with("Stack underflow while compiling instruction"));
        assert!(err.contains("[context:arg_count=2]"));
    }

    #[test]
    fn test_new_list_with_missing_elements_returns_compile_error() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::NewList(2),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("NewList should fail compile when elements are missing");
        assert!(err.contains("Stack underflow while compiling instruction NewList"));
        assert!(err.contains("needed 2, available 1"));
    }

    #[test]
    fn test_new_map_with_missing_pair_returns_compile_error() {
        let mut bytecode = Bytecode::new();
        let k = bytecode.add_constant(Constant::Int(10));
        let v = bytecode.add_constant(Constant::Int(20));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(k),
            Instruction::LoadConst(v),
            Instruction::NewMap(2),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("NewMap should fail compile when key/value pairs are missing");
        assert!(err.contains("Stack underflow while compiling instruction NewMap"));
        assert!(err.contains("[context:pair_count=2]"));
        assert!(err.contains("needed 4, available 2"));
    }

    #[test]
    fn test_load_index_with_missing_operands_returns_compile_error() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::LoadIndex, Instruction::Halt];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("LoadIndex should fail compile when operands are missing");
        assert!(err.contains("Stack underflow while compiling instruction LoadIndex"));
    }

    #[test]
    fn test_store_index_with_partial_operands_returns_compile_error() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::StoreIndex,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("StoreIndex with missing list operand should fail compile");
        assert!(err.contains("Stack underflow while compiling instruction StoreIndex"));
    }

    #[test]
    fn test_map_has_with_partial_operands_returns_compile_error() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::MapHas,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("MapHas with missing map operand should fail compile");
        assert!(err.contains("Stack underflow while compiling instruction MapHas"));
        assert!(err.contains("needed 2, available 1"));
    }

    #[test]
    fn test_jump_if_false_and_jump_offsets_are_patched() {
        let mut bytecode = Bytecode::new();
        let c_false = bytecode.add_constant(Constant::Bool(false));

        // if false { print 1 } else { print 2 }
        // Pattern exercises both JumpIfFalse (JE rel32) and Jump (JMP rel32).
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c_false),
            Instruction::JumpIfFalse(5),
            Instruction::LoadConst(bytecode.add_constant(Constant::Int(1))),
            Instruction::Print,
            Instruction::Jump(7),
            Instruction::LoadConst(bytecode.add_constant(Constant::Int(2))),
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");

        let je_pos = find_opcode(&code, &[0x0F, 0x84]).expect("expected JE opcode");
        let je_rel = read_rel32(&code, je_pos + 2);
        assert_ne!(je_rel, 0, "JE rel32 must be patched");

        let jmp_pos = find_opcode(&code, &[0xE9]).expect("expected JMP opcode");
        let jmp_rel = read_rel32(&code, jmp_pos + 1);
        assert_ne!(jmp_rel, 0, "JMP rel32 must be patched");
    }

    #[test]
    fn test_loop_contains_backward_jump() {
        let mut bytecode = Bytecode::new();
        let c_zero = bytecode.add_constant(Constant::Int(0));
        let c_three = bytecode.add_constant(Constant::Int(3));
        let c_one = bytecode.add_constant(Constant::Int(1));

        // while i < 3 { i = i + 1 }
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c_zero),
            Instruction::StoreLocal("i".to_string()),
            Instruction::LoadLocal("i".to_string()),
            Instruction::LoadConst(c_three),
            Instruction::Lt,
            Instruction::JumpIfFalse(11),
            Instruction::LoadLocal("i".to_string()),
            Instruction::LoadConst(c_one),
            Instruction::Add,
            Instruction::StoreExisting("i".to_string()),
            Instruction::Jump(2),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("loop bytecode should compile");

        // Find all unconditional jumps and ensure at least one is backward (negative rel32).
        let mut found_backward = false;
        let mut idx = 0;
        while idx < code.len() {
            if code[idx] == 0xE9 && idx + 4 < code.len() {
                let rel = read_rel32(&code, idx + 1);
                if rel < 0 {
                    found_backward = true;
                    break;
                }
                idx += 5;
            } else {
                idx += 1;
            }
        }

        assert!(
            found_backward,
            "expected at least one backward jump for loop"
        );
    }

    #[test]
    fn test_fuzz_cfg_jump_patch_stability() {
        // Build many small programs with valid control-flow targets.
        for seed in 1u64..=120 {
            let mut state = seed;
            let mut bytecode = Bytecode::new();

            let c_start = bytecode.add_constant(Constant::Int((lcg_next(&mut state) % 5) as i64));
            let c_limit =
                bytecode.add_constant(Constant::Int(6 + (lcg_next(&mut state) % 5) as i64));
            let c_step =
                bytecode.add_constant(Constant::Int(1 + (lcg_next(&mut state) % 2) as i64));
            let c_flag = bytecode.add_constant(Constant::Bool((lcg_next(&mut state) & 1) == 1));

            // Template:
            // i = start
            // if flag == false jump into loop-check
            // optional arithmetic
            // loop-check: if i < limit else halt
            // body: i = i + step
            // jump back to loop-check
            bytecode.main_instructions = vec![
                Instruction::LoadConst(c_start),             // 0
                Instruction::StoreLocal("i".to_string()),    // 1
                Instruction::LoadConst(c_flag),              // 2
                Instruction::JumpIfFalse(7),                 // 3
                Instruction::LoadLocal("i".to_string()),     // 4
                Instruction::LoadConst(c_step),              // 5
                Instruction::Add,                            // 6
                Instruction::StoreExisting("i".to_string()), // 7
                Instruction::LoadLocal("i".to_string()),     // 8
                Instruction::LoadConst(c_limit),             // 9
                Instruction::Lt,                             // 10
                Instruction::JumpIfFalse(16),                // 11
                Instruction::LoadLocal("i".to_string()),     // 12
                Instruction::LoadConst(c_step),              // 13
                Instruction::Add,                            // 14
                Instruction::StoreExisting("i".to_string()), // 15
                Instruction::Jump(8),                        // 16 (backward jump)
                Instruction::Halt,                           // 17
            ];

            let mut codegen = X86CodeGen::new();
            let code = match codegen.compile(&bytecode) {
                Ok(code) => code,
                Err(e) => {
                    panic!("seed {} failed: {}", seed, e);
                }
            };
            assert!(!code.is_empty(), "seed {} produced empty code", seed);

            // Collect rel32 jumps that were emitted.
            let mut jmp_count = 0usize;
            let mut has_backward = false;
            let mut idx = 0usize;
            while idx < code.len() {
                if code[idx] == 0xE9 && idx + 4 < code.len() {
                    let rel = read_rel32(&code, idx + 1);
                    assert_ne!(rel, 0, "seed {} has unpatched JMP rel32", seed);
                    if rel < 0 {
                        has_backward = true;
                    }
                    jmp_count += 1;
                    idx += 5;
                } else if idx + 5 < code.len() && code[idx] == 0x0F && code[idx + 1] == 0x84 {
                    let rel = read_rel32(&code, idx + 2);
                    assert_ne!(rel, 0, "seed {} has unpatched JE rel32", seed);
                    jmp_count += 1;
                    idx += 6;
                } else {
                    idx += 1;
                }
            }

            assert!(
                jmp_count >= 3,
                "seed {} should emit multiple control-flow jumps",
                seed
            );
            assert!(
                has_backward,
                "seed {} should include backward loop jump",
                seed
            );
        }
    }

    #[test]
    fn test_multifunction_call_graph_stability() {
        let mut bytecode = Bytecode::new();

        // leaf0(x) = x + 1
        let c1 = bytecode.add_constant(Constant::Int(1));
        let leaf0 = matter_bytecode::Function {
            name: "leaf0".to_string(),
            param_count: 1,
            instructions: vec![
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadConst(c1),
                Instruction::Add,
                Instruction::Return,
            ],
        };
        bytecode.functions.insert("leaf0".to_string(), leaf0);

        // leaf1(x) = x + 2
        let c2 = bytecode.add_constant(Constant::Int(2));
        let leaf1 = matter_bytecode::Function {
            name: "leaf1".to_string(),
            param_count: 1,
            instructions: vec![
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadConst(c2),
                Instruction::Add,
                Instruction::Return,
            ],
        };
        bytecode.functions.insert("leaf1".to_string(), leaf1);

        // mid(a, b) = leaf0(a) + leaf1(b)
        let mid = matter_bytecode::Function {
            name: "mid".to_string(),
            param_count: 2,
            instructions: vec![
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadGlobal("leaf0".to_string()),
                Instruction::Call(1),
                Instruction::LoadLocal("__param_1".to_string()),
                Instruction::LoadGlobal("leaf1".to_string()),
                Instruction::Call(1),
                Instruction::Add,
                Instruction::Return,
            ],
        };
        bytecode.functions.insert("mid".to_string(), mid);

        // root(n) = mid(n, n)
        let root = matter_bytecode::Function {
            name: "root".to_string(),
            param_count: 1,
            instructions: vec![
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadGlobal("mid".to_string()),
                Instruction::Call(2),
                Instruction::Return,
            ],
        };
        bytecode.functions.insert("root".to_string(), root);

        // main: root(10)
        let c10 = bytecode.add_constant(Constant::Int(10));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadGlobal("root".to_string()),
            Instruction::Call(1),
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("multi-function program should compile");
        assert!(!code.is_empty(), "generated code should not be empty");

        // Ensure function map has all expected symbols.
        for name in ["leaf0", "leaf1", "mid", "root"] {
            assert!(
                codegen.function_addresses.contains_key(name),
                "missing function address for {}",
                name
            );
        }

        // There should be multiple indirect calls (FF D3 for call r11 in current encoder).
        let call_count = code.windows(2).filter(|w| *w == [0x41, 0xFF]).count()
            + code.windows(2).filter(|w| *w == [0xFF, 0xD3]).count();
        assert!(
            call_count >= 3,
            "expected multiple call sites in generated code"
        );
    }

    #[test]
    fn test_deep_call_chain_stability() {
        let mut bytecode = Bytecode::new();
        let depth = 12usize;
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c_start = bytecode.add_constant(Constant::Int(5));

        // f0(x) = x + 1
        bytecode.functions.insert(
            "f0".to_string(),
            matter_bytecode::Function {
                name: "f0".to_string(),
                param_count: 1,
                instructions: vec![
                    Instruction::LoadLocal("__param_0".to_string()),
                    Instruction::LoadConst(c1),
                    Instruction::Add,
                    Instruction::Return,
                ],
            },
        );

        // fi(x) = f(i-1)(x) for i in 1..depth
        for i in 1..depth {
            let prev = format!("f{}", i - 1);
            let curr = format!("f{}", i);
            bytecode.functions.insert(
                curr.clone(),
                matter_bytecode::Function {
                    name: curr,
                    param_count: 1,
                    instructions: vec![
                        Instruction::LoadLocal("__param_0".to_string()),
                        Instruction::LoadGlobal(prev),
                        Instruction::Call(1),
                        Instruction::Return,
                    ],
                },
            );
        }

        // main: call deepest function
        let root = format!("f{}", depth - 1);
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c_start),
            Instruction::LoadGlobal(root),
            Instruction::Call(1),
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("deep call chain should compile");
        assert!(!code.is_empty(), "generated code should not be empty");

        for i in 0..depth {
            let fname = format!("f{}", i);
            assert!(
                codegen.function_addresses.contains_key(&fname),
                "missing function address for {}",
                fname
            );
        }

        // Expect many call opcodes for deep chain.
        let call_count = code.windows(2).filter(|w| *w == [0xFF, 0xD3]).count();
        assert!(
            call_count >= depth,
            "expected at least {} call instructions",
            depth
        );
    }

    #[test]
    fn test_function_definition() {
        let mut bytecode = Bytecode::new();

        // Define function: fn double(x) { return x * 2 }
        let c2 = bytecode.add_constant(Constant::Int(2));
        let function = matter_bytecode::Function {
            name: "double".to_string(),
            param_count: 1,
            instructions: vec![
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadConst(c2),
                Instruction::Mul,
                Instruction::Return,
            ],
        };
        bytecode.functions.insert("double".to_string(), function);

        // Main: just halt
        bytecode.main_instructions = vec![Instruction::Halt];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());

        // Verify function was compiled
        assert!(codegen.function_addresses.contains_key("double"));
    }

    #[test]
    fn test_function_call() {
        let mut bytecode = Bytecode::new();

        // Define function: fn add(a, b) { return a + b }
        let function = matter_bytecode::Function {
            name: "add".to_string(),
            param_count: 2,
            instructions: vec![
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadLocal("__param_1".to_string()),
                Instruction::Add,
                Instruction::Return,
            ],
        };
        bytecode.functions.insert("add".to_string(), function);

        // Main: call add(10, 20)
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::LoadGlobal("add".to_string()),
            Instruction::Call(2),
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_recursive_function() {
        let mut bytecode = Bytecode::new();

        // Define function: fn fib(n) { if n <= 1 { return n } return fib(n-1) + fib(n-2) }
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));

        let function = matter_bytecode::Function {
            name: "fib".to_string(),
            param_count: 1,
            instructions: vec![
                // if n <= 1
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadConst(c1),
                Instruction::LtEq,
                Instruction::JumpIfFalse(6), // Jump to recursive case
                // return n
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::Return,
                // fib(n-1)
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadConst(c1),
                Instruction::Sub,
                Instruction::LoadGlobal("fib".to_string()),
                Instruction::Call(1),
                // fib(n-2)
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadConst(c2),
                Instruction::Sub,
                Instruction::LoadGlobal("fib".to_string()),
                Instruction::Call(1),
                // add results
                Instruction::Add,
                Instruction::Return,
            ],
        };
        bytecode.functions.insert("fib".to_string(), function);

        // Main: call fib(5)
        let c5 = bytecode.add_constant(Constant::Int(5));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c5),
            Instruction::LoadGlobal("fib".to_string()),
            Instruction::Call(1),
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    // ============================================================================
    // SPRINT 26 PHASE 4: DATA STRUCTURES TESTS
    // ============================================================================

    #[test]
    fn test_new_list() {
        let mut bytecode = Bytecode::new();

        // Create list [1, 2, 3]
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));
        let c3 = bytecode.add_constant(Constant::Int(3));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::LoadConst(c3),
            Instruction::NewList(3),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_load_index() {
        let mut bytecode = Bytecode::new();

        // Create list [10, 20, 30] and access index 1
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        let c30 = bytecode.add_constant(Constant::Int(30));
        let c1 = bytecode.add_constant(Constant::Int(1));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::LoadConst(c30),
            Instruction::NewList(3),
            Instruction::LoadConst(c1),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_load_index_emits_negative_index_check_branch() {
        let mut bytecode = Bytecode::new();
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c_idx = bytecode.add_constant(Constant::Int(0));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::NewList(1),
            Instruction::LoadConst(c_idx),
            Instruction::LoadIndex,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("load-index should compile");

        // jl rel32 (0F 8C) is used for signed negative-index guard.
        assert!(
            find_opcode(&code, &[0x0F, 0x8C]).is_some(),
            "expected JL opcode for negative index guard in LoadIndex"
        );
    }

    #[test]
    fn test_list_len() {
        let mut bytecode = Bytecode::new();

        // Create list [1, 2, 3, 4, 5] and get length
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));
        let c3 = bytecode.add_constant(Constant::Int(3));
        let c4 = bytecode.add_constant(Constant::Int(4));
        let c5 = bytecode.add_constant(Constant::Int(5));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::LoadConst(c3),
            Instruction::LoadConst(c4),
            Instruction::LoadConst(c5),
            Instruction::NewList(5),
            Instruction::ListLen,
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_list_push() {
        let mut bytecode = Bytecode::new();

        // Create list [1, 2] and push 3
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));
        let c3 = bytecode.add_constant(Constant::Int(3));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::NewList(2),
            Instruction::LoadConst(c3),
            Instruction::ListPush,
            Instruction::ListLen,
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_list_pop() {
        let mut bytecode = Bytecode::new();

        // Create list [10, 20, 30] and pop
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        let c30 = bytecode.add_constant(Constant::Int(30));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::LoadConst(c30),
            Instruction::NewList(3),
            Instruction::ListPop,
            Instruction::Print, // Print popped value
            Instruction::Pop,   // Pop list
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_empty_list() {
        let mut bytecode = Bytecode::new();

        // Create empty list []
        bytecode.main_instructions = vec![
            Instruction::NewList(0),
            Instruction::ListLen,
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_new_map() {
        let mut bytecode = Bytecode::new();

        // Create map {"a": 1, "b": 2}
        let c_a = bytecode.add_constant(Constant::String("a".to_string()));
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c_b = bytecode.add_constant(Constant::String("b".to_string()));
        let c2 = bytecode.add_constant(Constant::Int(2));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c_a),
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c_b),
            Instruction::LoadConst(c2),
            Instruction::NewMap(2),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_new_struct() {
        let mut bytecode = Bytecode::new();

        // Create struct Point { x: 10, y: 20 }
        let cx = bytecode.add_constant(Constant::String("x".to_string()));
        let c10 = bytecode.add_constant(Constant::Int(10));
        let cy = bytecode.add_constant(Constant::String("y".to_string()));
        let c20 = bytecode.add_constant(Constant::Int(20));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(cx),
            Instruction::LoadConst(c10),
            Instruction::LoadConst(cy),
            Instruction::LoadConst(c20),
            Instruction::NewStruct("Point".to_string(), 2),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_map_keys_and_values_codegen() {
        let mut bytecode = Bytecode::new();

        // Create map {"a": 1, "b": 2}, then request keys and values.
        let c_a = bytecode.add_constant(Constant::String("a".to_string()));
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c_b = bytecode.add_constant(Constant::String("b".to_string()));
        let c2 = bytecode.add_constant(Constant::Int(2));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c_a),
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c_b),
            Instruction::LoadConst(c2),
            Instruction::NewMap(2),
            Instruction::StoreLocal("m".to_string()),
            Instruction::LoadLocal("m".to_string()),
            Instruction::MapKeys,
            Instruction::Pop,
            Instruction::LoadLocal("m".to_string()),
            Instruction::MapValues,
            Instruction::Pop,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_store_index() {
        let mut bytecode = Bytecode::new();

        // Create list [1, 2, 3] and set index 1 to 99
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));
        let c3 = bytecode.add_constant(Constant::Int(3));
        let c_idx = bytecode.add_constant(Constant::Int(1));
        let c99 = bytecode.add_constant(Constant::Int(99));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::LoadConst(c3),
            Instruction::NewList(3),
            Instruction::StoreLocal("list".to_string()),
            Instruction::LoadLocal("list".to_string()),
            Instruction::LoadConst(c_idx),
            Instruction::LoadConst(c99),
            Instruction::StoreIndex,
            Instruction::LoadLocal("list".to_string()),
            Instruction::LoadConst(c_idx),
            Instruction::LoadIndex,
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_store_index_var_codegen() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));
        let c_idx = bytecode.add_constant(Constant::Int(0));
        let c99 = bytecode.add_constant(Constant::Int(99));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::NewList(2),
            Instruction::StoreLocal("list".to_string()),
            Instruction::LoadConst(c_idx),
            Instruction::LoadConst(c99),
            Instruction::StoreIndexVar("list".to_string()),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_list_push_var_codegen() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::NewList(1),
            Instruction::StoreLocal("list".to_string()),
            Instruction::LoadConst(c2),
            Instruction::ListPushVar("list".to_string()),
            Instruction::Pop,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_list_pop_var_codegen() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::NewList(2),
            Instruction::StoreLocal("list".to_string()),
            Instruction::ListPopVar("list".to_string()),
            Instruction::Pop,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_store_index_emits_negative_index_check_branch() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c_idx = bytecode.add_constant(Constant::Int(0));
        let c99 = bytecode.add_constant(Constant::Int(99));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::NewList(1),
            Instruction::LoadConst(c_idx),
            Instruction::LoadConst(c99),
            Instruction::StoreIndex,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("store-index should compile");

        // jl rel32 (0F 8C) is used for signed negative-index guard.
        assert!(
            find_opcode(&code, &[0x0F, 0x8C]).is_some(),
            "expected JL opcode for negative index guard in StoreIndex"
        );
    }

    #[test]
    fn test_list_in_function() {
        let mut bytecode = Bytecode::new();

        // Define function: fn sum_list(list) { return list[0] + list[1] + list[2] }
        let c0 = bytecode.add_constant(Constant::Int(0));
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));

        let function = matter_bytecode::Function {
            name: "sum_list".to_string(),
            param_count: 1,
            instructions: vec![
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadConst(c0),
                Instruction::LoadIndex,
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadConst(c1),
                Instruction::LoadIndex,
                Instruction::Add,
                Instruction::LoadLocal("__param_0".to_string()),
                Instruction::LoadConst(c2),
                Instruction::LoadIndex,
                Instruction::Add,
                Instruction::Return,
            ],
        };
        bytecode.functions.insert("sum_list".to_string(), function);

        // Main: create list [10, 20, 30] and call sum_list
        let c10 = bytecode.add_constant(Constant::Int(10));
        let c20 = bytecode.add_constant(Constant::Int(20));
        let c30 = bytecode.add_constant(Constant::Int(30));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c10),
            Instruction::LoadConst(c20),
            Instruction::LoadConst(c30),
            Instruction::NewList(3),
            Instruction::LoadGlobal("sum_list".to_string()),
            Instruction::Call(1),
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_load_field_from_struct_codegen() {
        let mut bytecode = Bytecode::new();
        let cx = bytecode.add_constant(Constant::String("x".to_string()));
        let c10 = bytecode.add_constant(Constant::Int(10));
        let cy = bytecode.add_constant(Constant::String("y".to_string()));
        let c20 = bytecode.add_constant(Constant::Int(20));

        // point = Point { x: 10, y: 20 }; print(point.y)
        bytecode.main_instructions = vec![
            Instruction::LoadConst(cx),
            Instruction::LoadConst(c10),
            Instruction::LoadConst(cy),
            Instruction::LoadConst(c20),
            Instruction::NewStruct("Point".to_string(), 2),
            Instruction::LoadField("y".to_string()),
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_store_field_var_codegen() {
        let mut bytecode = Bytecode::new();
        let cx = bytecode.add_constant(Constant::String("x".to_string()));
        let c10 = bytecode.add_constant(Constant::Int(10));
        let cy = bytecode.add_constant(Constant::String("y".to_string()));
        let c20 = bytecode.add_constant(Constant::Int(20));
        let c99 = bytecode.add_constant(Constant::Int(99));

        // point = Point { x: 10, y: 20 }; point.y = 99; print(point.y)
        bytecode.main_instructions = vec![
            Instruction::LoadConst(cx),
            Instruction::LoadConst(c10),
            Instruction::LoadConst(cy),
            Instruction::LoadConst(c20),
            Instruction::NewStruct("Point".to_string(), 2),
            Instruction::StoreLocal("point".to_string()),
            Instruction::LoadConst(c99),
            Instruction::StoreFieldVar {
                target: "point".to_string(),
                field: "y".to_string(),
            },
            Instruction::LoadLocal("point".to_string()),
            Instruction::LoadField("y".to_string()),
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_codegen_embeds_missing_field_panic_message() {
        let mut bytecode = Bytecode::new();
        let cx = bytecode.add_constant(Constant::String("x".to_string()));
        let c10 = bytecode.add_constant(Constant::Int(10));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(cx),
            Instruction::LoadConst(c10),
            Instruction::NewStruct("Point".to_string(), 1),
            Instruction::LoadField("missing".to_string()),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("Field 'missing' not found\0".len())
                .any(|w| w == b"Field 'missing' not found\0"),
            "generated code should embed missing-field panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_invalid_field_access_type_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1), // non-struct/map value
            Instruction::LoadField("x".to_string()),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("Expected struct or map for field access\0".len())
                .any(|w| w == b"Expected struct or map for field access\0"),
            "generated code should embed invalid-type panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_store_index_oob_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c_idx = bytecode.add_constant(Constant::Int(9));
        let c_val = bytecode.add_constant(Constant::Int(42));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::NewList(1),
            Instruction::LoadConst(c_idx),
            Instruction::LoadConst(c_val),
            Instruction::StoreIndex,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("Index out of bounds (store) or non-list\0".len())
                .any(|w| w == b"Index out of bounds (store) or non-list\0"),
            "generated code should embed store-index panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_load_index_oob_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c_idx = bytecode.add_constant(Constant::Int(9));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::NewList(1),
            Instruction::LoadConst(c_idx),
            Instruction::LoadIndex,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("Index out of bounds or non-list\0".len())
                .any(|w| w == b"Index out of bounds or non-list\0"),
            "generated code should embed load-index panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_store_field_var_invalid_type_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c99 = bytecode.add_constant(Constant::Int(99));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::StoreLocal("target".to_string()),
            Instruction::LoadConst(c99),
            Instruction::StoreFieldVar {
                target: "target".to_string(),
                field: "x".to_string(),
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("Expected struct or map variable for field store\0".len())
                .any(|w| w == b"Expected struct or map variable for field store\0"),
            "generated code should embed store-field invalid-type panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_unknown_struct_field_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadField("unknown_field".to_string()),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("Unknown struct field 'unknown_field'\0".len())
                .any(|w| w == b"Unknown struct field 'unknown_field'\0"),
            "generated code should embed unknown-struct-field panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_list_pop_empty_panic_message() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![
            Instruction::NewList(0),
            Instruction::ListPop,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("Cannot pop from empty list\0".len())
                .any(|w| w == b"Cannot pop from empty list\0"),
            "generated code should embed list-pop-empty panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_list_resize_failed_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));

        // Build a push path that can trigger resize logic in codegen.
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::NewList(1),
            Instruction::LoadConst(c2),
            Instruction::ListPush,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("List resize failed\0".len())
                .any(|w| w == b"List resize failed\0"),
            "generated code should embed list-resize-failed panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_newlist_alloc_panic_messages() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::NewList(1),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("List allocation failed\0".len())
                .any(|w| w == b"List allocation failed\0"),
            "generated code should embed list-allocation-failed panic message"
        );
        assert!(
            code.windows("List data allocation failed\0".len())
                .any(|w| w == b"List data allocation failed\0"),
            "generated code should embed list-data-allocation-failed panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_newmap_alloc_panic_message() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![Instruction::NewMap(0), Instruction::Halt];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("Map allocation failed\0".len())
                .any(|w| w == b"Map allocation failed\0"),
            "generated code should embed map-allocation-failed panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_newstruct_alloc_panic_message() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![
            Instruction::NewStruct("Point".to_string(), 0),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(
            code.windows("Struct allocation failed\0".len())
                .any(|w| w == b"Struct allocation failed\0"),
            "generated code should embed struct-allocation-failed panic message"
        );
    }

    #[test]
    fn test_codegen_embeds_non_list_type_panic_messages() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::LoadIndex,
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::StoreIndex,
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::ListPush,
            Instruction::LoadConst(c1),
            Instruction::ListPop,
            Instruction::LoadConst(c1),
            Instruction::ListLen,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");

        assert!(code
            .windows("Index out of bounds or non-list\0".len())
            .any(|w| w == b"Index out of bounds or non-list\0"));
        assert!(code
            .windows("Index out of bounds (store) or non-list\0".len())
            .any(|w| w == b"Index out of bounds (store) or non-list\0"));
        assert!(code
            .windows("Expected list for push\0".len())
            .any(|w| w == b"Expected list for push\0"));
        assert!(code
            .windows("Expected list for pop\0".len())
            .any(|w| w == b"Expected list for pop\0"));
        assert!(code
            .windows("Expected list for len\0".len())
            .any(|w| w == b"Expected list for len\0"));
    }

    #[test]
    fn test_codegen_embeds_non_map_type_panic_messages() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::MapHas,
            Instruction::LoadConst(c1),
            Instruction::MapKeys,
            Instruction::LoadConst(c1),
            Instruction::MapValues,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");

        assert!(code
            .windows("Expected map for has\0".len())
            .any(|w| w == b"Expected map for has\0"));
        assert!(code
            .windows("Expected map for keys\0".len())
            .any(|w| w == b"Expected map for keys\0"));
        assert!(code
            .windows("Expected map for values\0".len())
            .any(|w| w == b"Expected map for values\0"));
    }

    #[test]
    fn test_codegen_embeds_var_collection_type_panic_messages() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::StoreLocal("v".to_string()),
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::StoreIndexVar("v".to_string()),
            Instruction::LoadConst(c1),
            Instruction::ListPushVar("v".to_string()),
            Instruction::Pop,
            Instruction::ListPopVar("v".to_string()),
            Instruction::Pop,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen.compile(&bytecode).expect("codegen should succeed");
        assert!(code
            .windows("Expected list or map variable 'v'\0".len())
            .any(|w| w == b"Expected list or map variable 'v'\0"));
        assert!(code
            .windows("Expected list variable 'v'\0".len())
            .any(|w| w == b"Expected list variable 'v'\0"));
    }

    #[test]
    fn test_spawn_event_codegen_no_stack_side_effect() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![
            Instruction::SpawnEvent("tick".to_string()),
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);
        assert!(
            result.is_ok(),
            "SpawnEvent should compile in native backend"
        );
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_backend_call_codegen_embeds_runtime_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::BackendCall {
                backend: "http".to_string(),
                method: "get".to_string(),
                arg_count: 1,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("BackendCall should compile and panic at runtime");
        assert!(code
            .windows("BackendCall not supported in native runtime: http.get\0".len())
            .any(|w| w == b"BackendCall not supported in native runtime: http.get\0"));
    }

    #[test]
    fn test_backend_call_math_add_is_natively_supported() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "add".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.add BackendCall should compile natively");
        assert!(!code
            .windows("BackendCall not supported in native runtime: math.add\0".len())
            .any(|w| w == b"BackendCall not supported in native runtime: math.add\0"));
    }

    #[test]
    fn test_backend_call_math_add_embeds_overflow_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(i64::MAX));
        let c2 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "add".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.add BackendCall should compile with overflow guard");
        assert!(code
            .windows("Overflow in math.add\0".len())
            .any(|w| w == b"Overflow in math.add\0"));
    }

    #[test]
    fn test_backend_call_math_neg_is_natively_supported() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "neg".to_string(),
                arg_count: 1,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.neg BackendCall should compile natively");
        assert!(!code
            .windows("BackendCall not supported in native runtime: math.neg\0".len())
            .any(|w| w == b"BackendCall not supported in native runtime: math.neg\0"));
    }

    #[test]
    fn test_backend_call_math_neg_embeds_overflow_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(i64::MIN));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "neg".to_string(),
                arg_count: 1,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.neg BackendCall should compile with overflow guard");
        assert!(code
            .windows("Overflow in math.neg\0".len())
            .any(|w| w == b"Overflow in math.neg\0"));
    }

    #[test]
    fn test_backend_call_math_sub_is_natively_supported() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(5));
        let c2 = bytecode.add_constant(Constant::Int(3));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "sub".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.sub BackendCall should compile natively");
        assert!(!code
            .windows("BackendCall not supported in native runtime: math.sub\0".len())
            .any(|w| w == b"BackendCall not supported in native runtime: math.sub\0"));
    }

    #[test]
    fn test_backend_call_math_sub_embeds_overflow_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(i64::MIN));
        let c2 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "sub".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.sub BackendCall should compile with overflow guard");
        assert!(code
            .windows("Overflow in math.sub\0".len())
            .any(|w| w == b"Overflow in math.sub\0"));
    }

    #[test]
    fn test_backend_call_math_mul_is_natively_supported() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(6));
        let c2 = bytecode.add_constant(Constant::Int(7));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "mul".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.mul BackendCall should compile natively");
        assert!(!code
            .windows("BackendCall not supported in native runtime: math.mul\0".len())
            .any(|w| w == b"BackendCall not supported in native runtime: math.mul\0"));
    }

    #[test]
    fn test_backend_call_math_mul_embeds_overflow_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(i64::MAX));
        let c2 = bytecode.add_constant(Constant::Int(2));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "mul".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.mul BackendCall should compile with overflow guard");
        assert!(code
            .windows("Overflow in math.mul\0".len())
            .any(|w| w == b"Overflow in math.mul\0"));
    }

    #[test]
    fn test_backend_call_math_div_is_natively_supported() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(8));
        let c2 = bytecode.add_constant(Constant::Int(2));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "div".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.div BackendCall should compile natively");
        assert!(!code
            .windows("BackendCall not supported in native runtime: math.div\0".len())
            .any(|w| w == b"BackendCall not supported in native runtime: math.div\0"));
    }

    #[test]
    fn test_backend_call_math_div_embeds_division_by_zero_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(8));
        let c2 = bytecode.add_constant(Constant::Int(0));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "div".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.div BackendCall should compile with div-by-zero guard");
        assert!(code
            .windows("Division by zero in math.div\0".len())
            .any(|w| w == b"Division by zero in math.div\0"));
    }

    #[test]
    fn test_backend_call_math_div_embeds_overflow_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(i64::MIN));
        let c2 = bytecode.add_constant(Constant::Int(-1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "div".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.div BackendCall should compile with overflow guard");
        assert!(code
            .windows("Overflow in math.div\0".len())
            .any(|w| w == b"Overflow in math.div\0"));
    }

    #[test]
    fn test_backend_call_math_add_invalid_arity_embeds_specific_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "add".to_string(),
                arg_count: 1,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.add with invalid arity should compile with runtime panic");
        assert!(code
            .windows("Invalid arity for backend call math.add: expected 2, got 1\0".len())
            .any(|w| w == b"Invalid arity for backend call math.add: expected 2, got 1\0"));
    }

    #[test]
    fn test_backend_call_math_neg_invalid_arity_embeds_specific_panic_message() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "neg".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("math.neg with invalid arity should compile with runtime panic");
        assert!(code
            .windows("Invalid arity for backend call math.neg: expected 1, got 2\0".len())
            .any(|w| w == b"Invalid arity for backend call math.neg: expected 1, got 2\0"));
    }

    #[test]
    fn test_backend_call_math_add_missing_argument_returns_compile_error() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::BackendCall {
                backend: "math".to_string(),
                method: "add".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("math.add should fail compile when stack is missing arguments");
        assert!(err.contains("Stack underflow while compiling instruction BackendCall"));
        assert!(err.contains("[context:backend=math.add,arg_count=2]"));
        assert!(err.contains("needed 2, available 1"));
    }

    #[test]
    fn test_backend_call_unsupported_missing_argument_returns_compile_error() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::BackendCall {
                backend: "http".to_string(),
                method: "get".to_string(),
                arg_count: 2,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("unsupported backend call should fail when arg pops underflow");
        assert!(err.contains("Stack underflow while compiling instruction BackendCall"));
        assert!(err.contains("[context:backend=http.get,arg_count=2]"));
        assert!(err.contains("needed 2, available 1"));
    }

    #[test]
    fn test_x86_backend_call_error_format_uses_context_block() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![
            Instruction::BackendCall {
                backend: "http".to_string(),
                method: "get".to_string(),
                arg_count: 1,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("expected backend call stack underflow");

        assert!(err.starts_with("Stack underflow while compiling instruction"));
        assert!(err.contains("[context:backend=http.get,arg_count=1]"));
    }

    #[test]
    fn test_x86_backend_call_error_preserves_nested_method_name_in_context() {
        let mut bytecode = Bytecode::new();
        bytecode.main_instructions = vec![
            Instruction::BackendCall {
                backend: "cloud.storage".to_string(),
                method: "object.get".to_string(),
                arg_count: 1,
            },
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let err = codegen
            .compile(&bytecode)
            .expect_err("expected backend call stack underflow");

        assert!(err.contains("Stack underflow while compiling instruction BackendCall"));
        assert!(err.contains("[context:backend=cloud.storage.object.get,arg_count=1]"));
    }

    #[test]
    fn test_backend_call_pop_path_underflow_uses_standard_context_format() {
        let mut codegen = X86CodeGen::new();

        let err = codegen
            .pop_for_backend_call(Register::RAX, "cloud.storage", "object.get")
            .expect_err("expected backend-call pop underflow error");

        assert!(err.starts_with("Stack underflow while compiling instruction BackendCall"));
        assert!(err.contains("[context:backend=cloud.storage.object.get]"));
        assert!(err.contains("needed 1, available 0"));
    }

    fn build_many_args_program(arg_count: usize) -> Bytecode {
        let mut bytecode = Bytecode::new();
        let zero = bytecode.add_constant(Constant::Int(0));

        let function = matter_bytecode::Function {
            name: "many".to_string(),
            param_count: arg_count,
            instructions: vec![Instruction::LoadConst(zero), Instruction::Return],
        };
        bytecode.functions.insert("many".to_string(), function);

        let mut main = Vec::new();
        for i in 0..arg_count {
            let c = bytecode.add_constant(Constant::Int(i as i64));
            main.push(Instruction::LoadConst(c));
        }
        main.push(Instruction::LoadGlobal("many".to_string()));
        main.push(Instruction::Call(arg_count));
        main.push(Instruction::Halt);
        bytecode.main_instructions = main;
        bytecode
    }

    #[test]
    fn test_call_supports_stack_args() {
        #[cfg(windows)]
        let arg_count = 6;
        #[cfg(not(windows))]
        let arg_count = 8;
        let bytecode = build_many_args_program(arg_count);

        let mut codegen = X86CodeGen::new();
        let result = codegen.compile(&bytecode);
        assert!(result.is_ok(), "target path should support stack arguments");
    }
}
