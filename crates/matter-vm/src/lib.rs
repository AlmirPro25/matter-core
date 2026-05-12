//! Matter Virtual Machine
//! Stack-based VM para executar bytecode MBC
use matter_backend::{Backend, Value};
use matter_bytecode::*;
use matter_energy::EnergyEstimator;
use matter_jit::{HotPathDetector, JitCompiler, Profiler};
use matter_memory::{CycleDetector, MemoryPool};
use matter_native::runtime::NativeRuntime;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::ptr;

#[derive(Debug)]
pub enum VmError {
    StackUnderflow,
    TypeError(String),
    UndefinedVariable(String),
    UndefinedFunction(String),
    InvalidInstruction,
    BackendError(String),
    DivisionByZero,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::StackUnderflow => write!(f, "stack underflow"),
            VmError::TypeError(message) => write!(f, "type error: {}", message),
            VmError::UndefinedVariable(name) => write!(f, "undefined variable '{}'", name),
            VmError::UndefinedFunction(name) => write!(f, "undefined function '{}'", name),
            VmError::InvalidInstruction => write!(f, "invalid instruction"),
            VmError::BackendError(message) => write!(f, "backend error: {}", message),
            VmError::DivisionByZero => write!(f, "division by zero"),
        }
    }
}

/// Tipo de escopo
#[derive(Debug, Clone, PartialEq)]
enum ScopeType {
    Event,
    Function,
    Block,
}

/// Frame de escopo para lookup hierárquico
#[derive(Debug, Clone)]
struct ScopeFrame {
    _scope_type: ScopeType,
    variables: HashMap<String, Value>,
    params: Vec<Value>,
}

impl ScopeFrame {
    fn new(scope_type: ScopeType) -> Self {
        Self {
            _scope_type: scope_type,
            variables: HashMap::new(),
            params: Vec::new(),
        }
    }

    fn function(args: Vec<Value>) -> Self {
        Self {
            _scope_type: ScopeType::Function,
            variables: HashMap::new(),
            params: args,
        }
    }
}

/// Call frame para execução de funções (mantido para compatibilidade)
#[derive(Debug, Clone)]
struct CallFrame;

pub struct Vm {
    stack: Vec<Value>,
    call_stack: Vec<CallFrame>,
    scope_stack: Vec<ScopeFrame>,
    globals: HashMap<String, Value>,
    bytecode: Bytecode,
    backends: HashMap<String, Box<dyn Backend>>,
    stdout_enabled: bool,
    output: Vec<String>,
    event_queue: VecDeque<String>,

    // Sprint 24 Phase 2: Memory Pool Integration
    memory_pool: MemoryPool,

    // Sprint 24 Phase 3: Cycle Detection Integration
    cycle_detector: CycleDetector,
    gc_threshold: usize,

    // Sprint 26: JIT Integration
    jit_compiler: JitCompiler,
    hot_path_detector: HotPathDetector,
    jit_failed_functions: HashSet<String>,
    native_runtime: NativeRuntime,
    estimated_instruction_cost: f64,
    estimated_backend_cost: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcpuExecutionResult {
    pub registers: [i64; 8],
    pub memory: Vec<i64>,
    pub cycles: u64,
    pub energy_consumed: u64,
    pub program_counter: usize,
    pub running: bool,
}

fn vm_exec_type_error(op: &str, detail: &str) -> VmError {
    VmError::TypeError(format!(
        "VM execution failed [context:op={}]: {}",
        op, detail
    ))
}

impl Vm {
    pub fn new(bytecode: Bytecode) -> Self {
        Self {
            stack: Vec::new(),
            call_stack: Vec::new(),
            scope_stack: Vec::new(),
            globals: HashMap::new(),
            bytecode,
            backends: HashMap::new(),
            stdout_enabled: true,
            output: Vec::new(),
            event_queue: VecDeque::new(),
            memory_pool: MemoryPool::new(),
            cycle_detector: CycleDetector::with_threshold(1000),
            gc_threshold: 1000,
            jit_compiler: JitCompiler::new(),
            hot_path_detector: HotPathDetector::new(Profiler::new()),
            jit_failed_functions: HashSet::new(),
            native_runtime: NativeRuntime {
                vm_ptr: ptr::null_mut(), // Will be set during execution if needed
                lookup_fn: lookup_native_func,
                get_global_fn: get_vm_global,
                set_global_fn: set_vm_global,
            },
            estimated_instruction_cost: 0.0,
            estimated_backend_cost: 0.0,
        }
    }

    pub fn register_backend(&mut self, name: String, backend: Box<dyn Backend>) {
        self.backends.insert(name, backend);
    }

    pub fn call_backend(
        &mut self,
        backend: &str,
        method: &str,
        args: Vec<Value>,
    ) -> Result<Value, VmError> {
        let backend_impl = self.backends.get_mut(backend).ok_or_else(|| {
            VmError::BackendError(format!(
                "Backend call failed [context:backend={},method={}]: backend not found",
                backend, method
            ))
        })?;
        backend_impl
            .call(method, args)
            .map_err(VmError::BackendError)
    }

    pub fn set_stdout_enabled(&mut self, enabled: bool) {
        self.stdout_enabled = enabled;
    }

    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output)
    }

    /// Extrai o estado global atual (para REPL)
    pub fn get_globals(&self) -> HashMap<String, Value> {
        self.globals.clone()
    }

    /// Injeta estado global (para REPL)
    pub fn set_globals(&mut self, globals: HashMap<String, Value>) {
        self.globals = globals;
    }

    /// Mescla funções de outro bytecode (para REPL)
    pub fn merge_functions(&mut self, other_bytecode: &Bytecode) {
        for (name, function) in &other_bytecode.functions {
            self.bytecode
                .functions
                .insert(name.clone(), function.clone());
        }
    }

    /// Get memory pool statistics (Sprint 24 Phase 2)
    pub fn memory_pool_stats(&self) -> matter_memory::PoolStats {
        self.memory_pool.stats()
    }

    /// Reset memory pool (Sprint 24 Phase 2)
    /// Reuses allocated chunks for new allocations
    pub fn reset_memory_pool(&self) {
        self.memory_pool.reset();
    }

    /// Clear memory pool (Sprint 24 Phase 2)
    /// Deallocates all chunks
    pub fn clear_memory_pool(&self) {
        self.memory_pool.clear();
    }

    /// Get cycle detector statistics (Sprint 24 Phase 3)
    pub fn cycle_detector_stats(&self) -> matter_memory::CycleDetectorStats {
        self.cycle_detector.stats()
    }

    /// Force garbage collection (Sprint 24 Phase 3)
    /// Runs cycle detection and collects unreachable objects
    pub fn force_gc(&self) -> matter_memory::CycleDetectionResult {
        self.cycle_detector.force_collect()
    }

    /// Set GC threshold (Sprint 24 Phase 3)
    /// GC will run automatically after this many allocations
    pub fn set_gc_threshold(&mut self, threshold: usize) {
        self.gc_threshold = threshold;
        self.cycle_detector.set_threshold(threshold);
    }

    /// Get current GC threshold (Sprint 24 Phase 3)
    pub fn gc_threshold(&self) -> usize {
        self.gc_threshold
    }

    pub fn estimated_instruction_cost(&self) -> f64 {
        self.estimated_instruction_cost
    }

    pub fn estimated_backend_cost(&self) -> f64 {
        self.estimated_backend_cost
    }

    fn track_instruction_cost(&mut self, bucket: &str) {
        self.estimated_instruction_cost += EnergyEstimator::estimate_instruction_cost(bucket);
    }

    /// Clear cycle detector (Sprint 24 Phase 3)
    /// Removes all tracked objects
    pub fn clear_cycle_detector(&self) {
        self.cycle_detector.clear();
    }

    /// Push novo escopo
    fn push_scope(&mut self, scope_type: ScopeType) {
        self.scope_stack.push(ScopeFrame::new(scope_type));
    }

    fn push_function_scope(&mut self, args: Vec<Value>) {
        self.scope_stack.push(ScopeFrame::function(args));
    }

    /// Pop escopo (cleanup automático)
    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn update_existing_variable(&mut self, name: &str, value: Value) -> Result<(), VmError> {
        for scope in self.scope_stack.iter_mut().rev() {
            if scope.variables.contains_key(name) {
                scope.variables.insert(name.to_string(), value);
                return Ok(());
            }
        }

        if self.globals.contains_key(name) {
            self.globals.insert(name.to_string(), value);
            Ok(())
        } else {
            Err(VmError::UndefinedVariable(name.to_string()))
        }
    }

    fn load_existing_variable(&self, name: &str) -> Result<Value, VmError> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(value) = scope.variables.get(name) {
                return Ok(value.clone());
            }
        }

        self.globals
            .get(name)
            .cloned()
            .ok_or_else(|| VmError::UndefinedVariable(name.to_string()))
    }

    pub fn run(&mut self) -> Result<(), VmError> {
        self.execute(&self.bytecode.main_instructions.clone())?;
        self.drain_event_queue()
    }

    pub fn emit_event(&mut self, event: &str) -> Result<(), VmError> {
        self.emit_event_now(event)?;
        self.drain_event_queue()
    }

    fn emit_event_now(&mut self, event: &str) -> Result<(), VmError> {
        if let Some(handler) = self.bytecode.event_handlers.get(event) {
            let instructions = handler.instructions.clone();

            // Event scope
            self.push_scope(ScopeType::Event);
            let result = self.execute(&instructions);
            self.pop_scope();

            result?;
        }
        Ok(())
    }

    fn drain_event_queue(&mut self) -> Result<(), VmError> {
        let mut executed = 0usize;
        while let Some(event) = self.event_queue.pop_front() {
            executed += 1;
            if executed > 10_000 {
                return Err(vm_exec_type_error(
                    "spawn_event_queue",
                    "exceeded 10000 events",
                ));
            }
            self.emit_event_now(&event)?;
        }
        Ok(())
    }

    fn execute_function_call(&mut self, func_name: &str, arg_count: usize) -> Result<(), VmError> {
        // Sprint 26: JIT Check
        let cached_native = if let Some(native_func) = self.jit_compiler.code_cache.get(func_name) {
            native_func.record_call();
            Some(native_func.clone())
        } else {
            None
        };
        if let Some(native_func) = cached_native {
            // Setup VM pointer for the call
            self.native_runtime.vm_ptr = self as *mut Vm as *mut std::ffi::c_void;

            // Execute native code
            unsafe {
                let result_val = native_func.execute(&mut self.native_runtime);
                self.stack.push(Value::Int(result_val));
            }

            return Ok(());
        }

        // Fallback to Bytecode
        let function = self
            .bytecode
            .functions
            .get(func_name)
            .ok_or_else(|| VmError::UndefinedFunction(func_name.to_string()))?;
        let param_count = function.param_count;
        let func_instructions = function.instructions.as_slice() as *const [Instruction];

        // Record call for JIT profiling
        let call_count = self
            .hot_path_detector
            .profiler_mut()
            .record_call_count(func_name);

        // Check if it's time to JIT compile
        if (call_count == 1_000 || call_count % 1_024 == 0)
            && !self.jit_compiler.is_compiled(func_name)
            && !self.jit_failed_functions.contains(func_name)
        {
            self.hot_path_detector.update();
            if self.hot_path_detector.is_hot_function(func_name)
                && self
                    .jit_compiler
                    .compile_function(func_name, unsafe { &*func_instructions })
                    .is_err()
            {
                self.jit_failed_functions.insert(func_name.to_string());
            }
        }

        // Verificar número de parâmetros
        if arg_count != param_count {
            return Err(vm_exec_type_error(
                "call_arity",
                &format!(
                    "function {} expects {} arguments, got {}",
                    func_name, param_count, arg_count
                ),
            ));
        }

        if self.stack.len() < arg_count {
            return Err(VmError::StackUnderflow);
        }
        let args = match arg_count {
            0 => Vec::new(),
            1 => vec![self.stack.pop().ok_or(VmError::StackUnderflow)?],
            2 => {
                let second = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                let first = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                vec![first, second]
            }
            _ => {
                let mut args = Vec::with_capacity(arg_count);
                for _ in 0..arg_count {
                    args.push(self.stack.pop().ok_or(VmError::StackUnderflow)?);
                }
                args.reverse();
                args
            }
        };

        // Push call frame
        self.call_stack.push(CallFrame);
        let scope_depth_before_call = self.scope_stack.len();

        // Push function scope with positional parameters. This keeps hot calls off
        // the string/hashmap path used by normal locals.
        self.push_function_scope(args);

        // Executar função sem clonar o corpo a cada chamada. O bytecode não é
        // mutado durante `execute`; `merge_functions` só acontece fora da execução.
        let result = self.execute(unsafe { &*func_instructions });

        // Pop function scope (cleanup automático)
        while self.scope_stack.len() > scope_depth_before_call {
            self.pop_scope();
        }

        // Pop call frame
        self.call_stack.pop();
        result
    }

    fn execute(&mut self, instructions: &[Instruction]) -> Result<(), VmError> {
        let mut ip = 0; // instruction pointer

        while ip < instructions.len() {
            match &instructions[ip] {
                Instruction::LoadConst(id) => {
                    self.track_instruction_cost("load_store");
                    let constant = &self.bytecode.constants[*id];
                    let value = match constant {
                        Constant::Int(n) => Value::Int(*n),
                        Constant::Float(f) => Value::Float(*f),
                        Constant::Bool(b) => Value::Bool(*b),
                        Constant::String(s) => Value::new_string(s.clone()),
                        Constant::Unit => Value::Unit,
                    };
                    self.stack.push(value);
                }

                Instruction::LoadGlobal(name) => {
                    self.track_instruction_cost("load_store");
                    // Lookup hierárquico - clonar para evitar borrow issues
                    let value = {
                        // Busca do escopo mais interno para o mais externo
                        let mut found = None;
                        for scope in self.scope_stack.iter().rev() {
                            if let Some(v) = scope.variables.get(name) {
                                found = Some(v.clone());
                                break;
                            }
                        }

                        // Fallback para global
                        found.or_else(|| self.globals.get(name).cloned())
                    };

                    if let Some(v) = value {
                        self.stack.push(v);
                    } else if self.bytecode.functions.contains_key(name) {
                        self.stack.push(Value::new_function(name.clone()));
                    } else {
                        return Err(VmError::UndefinedVariable(name.clone()));
                    }
                }

                Instruction::StoreGlobal(name) => {
                    self.track_instruction_cost("load_store");
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    // StoreGlobal SEMPRE armazena no escopo global
                    // Isso é necessário para que variáveis globais possam ser
                    // atualizadas de dentro de loops e blocos
                    self.globals.insert(name.clone(), value);
                }

                Instruction::LoadLocal(name) => {
                    // Lookup hierárquico - clonar para evitar borrow issues
                    let value = {
                        let mut found = None;
                        for scope in self.scope_stack.iter().rev() {
                            if let Some(v) = scope.variables.get(name) {
                                found = Some(v.clone());
                                break;
                            }
                        }
                        found.or_else(|| self.globals.get(name).cloned())
                    };

                    if let Some(v) = value {
                        self.stack.push(v);
                    } else {
                        return Err(VmError::UndefinedVariable(name.clone()));
                    }
                }

                Instruction::LoadParam(index) => {
                    let value = self
                        .scope_stack
                        .iter()
                        .rev()
                        .find(|scope| scope._scope_type == ScopeType::Function)
                        .and_then(|scope| scope.params.get(*index))
                        .cloned()
                        .ok_or_else(|| {
                            vm_exec_type_error(
                                "load_param",
                                &format!("missing function parameter {}", index),
                            )
                        })?;
                    self.stack.push(value);
                }

                Instruction::StoreLocal(name) => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if let Some(scope) = self.scope_stack.last_mut() {
                        scope.variables.insert(name.clone(), value);
                    } else {
                        self.globals.insert(name.clone(), value);
                    }
                }

                Instruction::StoreExisting(name) => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    self.update_existing_variable(name, value)?;
                }

                Instruction::Add => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match (&left, &right) {
                        (Value::Int(l), Value::Int(r)) => self.stack.push(Value::Int(l + r)),
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Float(l + r)),
                        (Value::Int(l), Value::Float(r)) => {
                            self.stack.push(Value::Float(*l as f64 + r))
                        }
                        (Value::Float(l), Value::Int(r)) => {
                            self.stack.push(Value::Float(l + *r as f64))
                        }
                        _ => return Err(vm_exec_type_error("add", "operands must be numbers")),
                    }
                }

                Instruction::Sub => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match (&left, &right) {
                        (Value::Int(l), Value::Int(r)) => self.stack.push(Value::Int(l - r)),
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Float(l - r)),
                        (Value::Int(l), Value::Float(r)) => {
                            self.stack.push(Value::Float(*l as f64 - r))
                        }
                        (Value::Float(l), Value::Int(r)) => {
                            self.stack.push(Value::Float(l - *r as f64))
                        }
                        _ => return Err(vm_exec_type_error("sub", "operands must be numbers")),
                    }
                }

                Instruction::Mul => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match (&left, &right) {
                        (Value::Int(l), Value::Int(r)) => self.stack.push(Value::Int(l * r)),
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Float(l * r)),
                        (Value::Int(l), Value::Float(r)) => {
                            self.stack.push(Value::Float(*l as f64 * r))
                        }
                        (Value::Float(l), Value::Int(r)) => {
                            self.stack.push(Value::Float(l * *r as f64))
                        }
                        _ => return Err(vm_exec_type_error("mul", "operands must be numbers")),
                    }
                }

                Instruction::Div => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match (&left, &right) {
                        (Value::Int(l), Value::Int(r)) => {
                            if *r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            self.stack.push(Value::Int(l / r));
                        }
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Float(l / r)),
                        (Value::Int(l), Value::Float(r)) => {
                            self.stack.push(Value::Float(*l as f64 / r))
                        }
                        (Value::Float(l), Value::Int(r)) => {
                            self.stack.push(Value::Float(l / *r as f64))
                        }
                        _ => return Err(vm_exec_type_error("div", "operands must be numbers")),
                    }
                }

                Instruction::Mod => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match (&left, &right) {
                        (Value::Int(l), Value::Int(r)) => {
                            if *r == 0 {
                                return Err(VmError::DivisionByZero);
                            }
                            self.stack.push(Value::Int(l % r));
                        }
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Float(l % r)),
                        _ => return Err(vm_exec_type_error("mod", "operands must be numbers")),
                    }
                }

                Instruction::Neg => {
                    let val = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match val {
                        Value::Int(n) => self.stack.push(Value::Int(-n)),
                        Value::Float(f) => self.stack.push(Value::Float(-f)),
                        _ => return Err(vm_exec_type_error("neg", "operand must be a number")),
                    }
                }

                Instruction::And => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = left.as_bool().map_err(VmError::TypeError)?
                        && right.as_bool().map_err(VmError::TypeError)?;
                    self.stack.push(Value::Bool(result));
                }

                Instruction::Or => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = left.as_bool().map_err(VmError::TypeError)?
                        || right.as_bool().map_err(VmError::TypeError)?;
                    self.stack.push(Value::Bool(result));
                }

                Instruction::Not => {
                    let val = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = !val.as_bool().map_err(VmError::TypeError)?;
                    self.stack.push(Value::Bool(result));
                }

                Instruction::Eq => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    self.stack.push(Value::Bool(left == right));
                }

                Instruction::NotEq => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    self.stack.push(Value::Bool(left != right));
                }

                Instruction::Lt => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match (&left, &right) {
                        (Value::Int(l), Value::Int(r)) => self.stack.push(Value::Bool(l < r)),
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Bool(l < r)),
                        (Value::Int(l), Value::Float(r)) => {
                            self.stack.push(Value::Bool((*l as f64) < *r))
                        }
                        (Value::Float(l), Value::Int(r)) => {
                            self.stack.push(Value::Bool(*l < (*r as f64)))
                        }
                        _ => return Err(vm_exec_type_error("lt", "operands must be numbers")),
                    }
                }

                Instruction::Gt => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match (&left, &right) {
                        (Value::Int(l), Value::Int(r)) => self.stack.push(Value::Bool(l > r)),
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Bool(l > r)),
                        (Value::Int(l), Value::Float(r)) => {
                            self.stack.push(Value::Bool((*l as f64) > *r))
                        }
                        (Value::Float(l), Value::Int(r)) => {
                            self.stack.push(Value::Bool(*l > (*r as f64)))
                        }
                        _ => return Err(vm_exec_type_error("gt", "operands must be numbers")),
                    }
                }

                Instruction::LtEq => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match (&left, &right) {
                        (Value::Int(l), Value::Int(r)) => self.stack.push(Value::Bool(l <= r)),
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Bool(l <= r)),
                        (Value::Int(l), Value::Float(r)) => {
                            self.stack.push(Value::Bool((*l as f64) <= *r))
                        }
                        (Value::Float(l), Value::Int(r)) => {
                            self.stack.push(Value::Bool(*l <= (*r as f64)))
                        }
                        _ => return Err(vm_exec_type_error("lteq", "operands must be numbers")),
                    }
                }

                Instruction::GtEq => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    match (&left, &right) {
                        (Value::Int(l), Value::Int(r)) => self.stack.push(Value::Bool(l >= r)),
                        (Value::Float(l), Value::Float(r)) => self.stack.push(Value::Bool(l >= r)),
                        (Value::Int(l), Value::Float(r)) => {
                            self.stack.push(Value::Bool((*l as f64) >= *r))
                        }
                        (Value::Float(l), Value::Int(r)) => {
                            self.stack.push(Value::Bool(*l >= (*r as f64)))
                        }
                        _ => return Err(vm_exec_type_error("gteq", "operands must be numbers")),
                    }
                }

                Instruction::Jump(target) => {
                    ip = *target;
                    continue;
                }

                Instruction::JumpIfFalse(target) => {
                    let condition = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if !condition.as_bool().map_err(VmError::TypeError)? {
                        ip = *target;
                        continue;
                    }
                }

                Instruction::Call(arg_count) => {
                    self.track_instruction_cost("call");
                    let func_value = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::Function(func_name) = func_value {
                        self.execute_function_call(func_name.as_ref(), *arg_count)?;
                    } else {
                        return Err(vm_exec_type_error("call", "expected function"));
                    }
                }

                Instruction::CallNamed { name, arg_count } => {
                    self.track_instruction_cost("call");
                    self.execute_function_call(name, *arg_count)?;
                }

                Instruction::Return => {
                    // Return value should be on top of stack
                    return Ok(());
                }

                Instruction::SpawnEvent(event) => {
                    self.event_queue.push_back(event.clone());
                }

                Instruction::Print => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let line = value.to_display_string();
                    self.output.push(line.clone());
                    if self.stdout_enabled {
                        println!("{}", line);
                    }
                }

                Instruction::BackendCall {
                    backend,
                    method,
                    arg_count,
                } => {
                    self.track_instruction_cost("backend_call");
                    let mut args = Vec::new();
                    for _ in 0..*arg_count {
                        args.push(self.stack.pop().ok_or(VmError::StackUnderflow)?);
                    }
                    args.reverse();

                    self.estimated_backend_cost += EnergyEstimator::estimate_backend_call(backend);
                    let result = self.call_backend(backend, method, args)?;
                    self.stack.push(result);
                }

                Instruction::Pop => {
                    self.stack.pop().ok_or(VmError::StackUnderflow)?;
                }

                Instruction::PushScope => {
                    self.push_scope(ScopeType::Block);
                }

                Instruction::PopScope => {
                    self.pop_scope();
                }

                // Sprint 4: Data Model - Lists
                Instruction::NewList(size) => {
                    // Pop N elements from stack and create list
                    let mut elements = Vec::with_capacity(*size);
                    for _ in 0..*size {
                        elements.push(self.stack.pop().ok_or(VmError::StackUnderflow)?);
                    }
                    elements.reverse(); // Stack is LIFO, so reverse to get correct order
                    self.stack.push(Value::new_list(elements));
                }

                Instruction::LoadIndex => {
                    // Pop index, pop list, push value
                    let index = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let list = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::List(elements) = list {
                        let idx = index.as_int().map_err(VmError::TypeError)? as usize;
                        if idx < elements.len() {
                            self.stack.push(elements[idx].clone());
                        } else {
                            return Err(vm_exec_type_error(
                                "load_index",
                                &format!("index {} out of bounds", idx),
                            ));
                        }
                    } else if let Value::Map(entries) = list {
                        let key = index.as_string().map_err(VmError::TypeError)?;
                        if let Some(value) = entries.get(&key) {
                            self.stack.push(value.clone());
                        } else {
                            return Err(vm_exec_type_error(
                                "load_index",
                                &format!("map key '{}' not found", key),
                            ));
                        }
                    } else {
                        return Err(vm_exec_type_error(
                            "load_index",
                            "expected list or map for indexing",
                        ));
                    }
                }

                Instruction::StoreIndex => {
                    // Pop value, pop index, pop list (creates new list with updated value)
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let index = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let list = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::List(elements) = list {
                        let idx = index.as_int().map_err(VmError::TypeError)? as usize;
                        if idx < elements.len() {
                            let mut new_elements = (*elements).clone();
                            new_elements[idx] = value;
                            self.stack.push(Value::new_list(new_elements));
                        } else {
                            return Err(vm_exec_type_error(
                                "store_index",
                                &format!("index {} out of bounds", idx),
                            ));
                        }
                    } else {
                        return Err(vm_exec_type_error(
                            "store_index",
                            "expected list for indexing",
                        ));
                    }
                }

                Instruction::ListPush => {
                    // Pop value, pop list, push list (with value appended)
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let list = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::List(elements) = list {
                        let mut new_elements = (*elements).clone();
                        new_elements.push(value);
                        self.stack.push(Value::new_list(new_elements));
                    } else {
                        return Err(vm_exec_type_error("list_push", "expected list for push"));
                    }
                }

                Instruction::ListPop => {
                    // Pop list, push value, push list (with last element removed)
                    let list = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::List(elements) = list {
                        let mut new_elements = (*elements).clone();
                        if let Some(value) = new_elements.pop() {
                            self.stack.push(value);
                            self.stack.push(Value::new_list(new_elements));
                        } else {
                            return Err(vm_exec_type_error(
                                "list_pop",
                                "cannot pop from empty list",
                            ));
                        }
                    } else {
                        return Err(vm_exec_type_error("list_pop", "expected list for pop"));
                    }
                }

                Instruction::ListLen => {
                    // Pop list, push length
                    let list = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::List(elements) = list {
                        self.stack.push(Value::Int(elements.len() as i64));
                    } else {
                        return Err(vm_exec_type_error("list_len", "expected list for len"));
                    }
                }

                Instruction::StoreIndexVar(name) => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let index = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let list = self.load_existing_variable(name)?;

                    if let Value::List(elements) = list {
                        let idx = index.as_int().map_err(VmError::TypeError)? as usize;
                        if idx < elements.len() {
                            let mut new_elements = (*elements).clone();
                            new_elements[idx] = value;
                            self.update_existing_variable(name, Value::new_list(new_elements))?;
                        } else {
                            return Err(vm_exec_type_error(
                                "store_index_var",
                                &format!("index {} out of bounds", idx),
                            ));
                        }
                    } else if let Value::Map(entries) = list {
                        let key = index.as_string().map_err(VmError::TypeError)?;
                        let mut new_entries = (*entries).clone();
                        new_entries.insert(key, value);
                        self.update_existing_variable(name, Value::new_map(new_entries))?;
                    } else {
                        return Err(vm_exec_type_error(
                            "store_index_var",
                            &format!("expected list or map variable '{}'", name),
                        ));
                    }
                }

                Instruction::ListPushVar(name) => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let list = self.load_existing_variable(name)?;

                    if let Value::List(elements) = list {
                        let mut new_elements = (*elements).clone();
                        new_elements.push(value);
                        self.update_existing_variable(name, Value::new_list(new_elements))?;
                        self.stack.push(Value::Unit);
                    } else {
                        return Err(vm_exec_type_error(
                            "list_push_var",
                            &format!("expected list variable '{}'", name),
                        ));
                    }
                }

                Instruction::ListPopVar(name) => {
                    let list = self.load_existing_variable(name)?;

                    if let Value::List(elements) = list {
                        let mut new_elements = (*elements).clone();
                        if let Some(value) = new_elements.pop() {
                            self.update_existing_variable(name, Value::new_list(new_elements))?;
                            self.stack.push(value);
                        } else {
                            return Err(vm_exec_type_error(
                                "list_pop_var",
                                "cannot pop from empty list",
                            ));
                        }
                    } else {
                        return Err(vm_exec_type_error(
                            "list_pop_var",
                            &format!("expected list variable '{}'", name),
                        ));
                    }
                }

                Instruction::NewMap(size) => {
                    let mut entries = HashMap::new();
                    for _ in 0..*size {
                        let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                        let key = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                        entries.insert(key.as_string().map_err(VmError::TypeError)?, value);
                    }
                    self.stack.push(Value::new_map(entries));
                }

                Instruction::MapHas => {
                    let key = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let map = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::Map(entries) = map {
                        let key = key.as_string().map_err(VmError::TypeError)?;
                        self.stack.push(Value::Bool(entries.contains_key(&key)));
                    } else {
                        return Err(vm_exec_type_error("map_has", "expected map for has"));
                    }
                }

                Instruction::MapKeys => {
                    let map = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::Map(entries) = map {
                        let mut keys: Vec<String> = entries.keys().cloned().collect();
                        keys.sort();
                        self.stack.push(Value::new_list(
                            keys.into_iter().map(Value::new_string).collect(),
                        ));
                    } else {
                        return Err(vm_exec_type_error("map_keys", "expected map for keys"));
                    }
                }

                Instruction::MapValues => {
                    let map = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::Map(entries) = map {
                        let mut pairs: Vec<(String, Value)> =
                            (*entries).clone().into_iter().collect();
                        pairs.sort_by(|left, right| left.0.cmp(&right.0));
                        self.stack.push(Value::new_list(
                            pairs.into_iter().map(|(_, value)| value).collect(),
                        ));
                    } else {
                        return Err(vm_exec_type_error("map_values", "expected map for values"));
                    }
                }

                Instruction::NewStruct(type_name, size) => {
                    let mut fields = HashMap::new();
                    for _ in 0..*size {
                        let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                        let field = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                        fields.insert(field.as_string().map_err(VmError::TypeError)?, value);
                    }
                    self.stack
                        .push(Value::new_struct(type_name.clone(), fields));
                }

                Instruction::LoadField(field) => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    match value {
                        Value::Struct { fields, .. } => {
                            if let Some(field_value) = fields.get(field) {
                                self.stack.push(field_value.clone());
                            } else {
                                return Err(vm_exec_type_error(
                                    "load_field",
                                    &format!("struct field '{}' not found", field),
                                ));
                            }
                        }
                        Value::Map(entries) => {
                            if let Some(field_value) = entries.get(field) {
                                self.stack.push(field_value.clone());
                            } else {
                                return Err(vm_exec_type_error(
                                    "load_field",
                                    &format!("map field '{}' not found", field),
                                ));
                            }
                        }
                        _ => {
                            return Err(vm_exec_type_error(
                                "load_field",
                                "expected struct or map for field access",
                            ));
                        }
                    }
                }

                Instruction::StoreFieldVar { target, field } => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let current = self.load_existing_variable(target)?;

                    match current {
                        Value::Struct { type_name, fields } => {
                            let mut new_fields = (*fields).clone();
                            new_fields.insert(field.clone(), value);
                            self.update_existing_variable(
                                target,
                                Value::new_struct((*type_name).clone(), new_fields),
                            )?;
                        }
                        Value::Map(entries) => {
                            let mut new_entries = (*entries).clone();
                            new_entries.insert(field.clone(), value);
                            self.update_existing_variable(target, Value::new_map(new_entries))?;
                        }
                        _ => {
                            return Err(vm_exec_type_error(
                                "store_field_var",
                                &format!("expected struct or map variable '{}'", target),
                            ));
                        }
                    }
                }

                Instruction::Halt => {
                    break;
                }
            }

            ip += 1;
        }

        Ok(())
    }
}

pub fn run_vcpu_program(
    program: Vec<matter_vcpu::Instruction>,
    memory_size: usize,
) -> Result<VcpuExecutionResult, matter_vcpu::VirtualCpuError> {
    run_vcpu_program_with_limits(program, memory_size, None, None)
}

pub fn run_vcpu_program_with_limits(
    program: Vec<matter_vcpu::Instruction>,
    memory_size: usize,
    cycle_budget: Option<u64>,
    energy_budget: Option<u64>,
) -> Result<VcpuExecutionResult, matter_vcpu::VirtualCpuError> {
    let mut cpu = matter_vcpu::VirtualCpu::new(memory_size);
    cpu.set_cycle_budget(cycle_budget);
    cpu.set_energy_budget(energy_budget);
    cpu.load_program(program)?;
    cpu.run()?;

    let stats = cpu.stats();
    Ok(VcpuExecutionResult {
        registers: cpu.registers,
        memory: cpu.memory,
        cycles: stats.cycles,
        energy_consumed: stats.energy_consumed,
        program_counter: stats.program_counter,
        running: stats.running,
    })
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use matter_jit::cache::NativeFunction;
    use matter_native::runtime::ExecutableMemory;
    use matter_vcpu::Instruction as VcpuInstruction;
    use std::ffi::CString;

    #[test]
    fn test_vcpu_bridge_executes_sum_program() {
        let result = run_vcpu_program(
            vec![
                VcpuInstruction::LoadConst { reg: 0, value: 10 },
                VcpuInstruction::LoadConst { reg: 1, value: 20 },
                VcpuInstruction::Add { dst: 2, a: 0, b: 1 },
                VcpuInstruction::Halt,
            ],
            16,
        )
        .unwrap();

        assert_eq!(result.registers[2], 30);
        assert_eq!(result.cycles, 4);
        assert_eq!(result.energy_consumed, 4);
        assert!(!result.running);
    }

    #[test]
    fn test_vcpu_bridge_respects_cycle_budget() {
        let err = run_vcpu_program_with_limits(
            vec![
                VcpuInstruction::Nop,
                VcpuInstruction::Nop,
                VcpuInstruction::Nop,
                VcpuInstruction::Halt,
            ],
            8,
            Some(2),
            None,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            matter_vcpu::VirtualCpuError::CycleBudgetExceeded {
                limit: 2,
                attempted: 3
            }
        ));
    }

    #[test]
    fn test_vm_basic() {
        let mut bytecode = Bytecode::new();
        let const_id = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(const_id),
            Instruction::StoreGlobal("x".to_string()),
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        vm.run().unwrap();

        assert_eq!(vm.globals.get("x"), Some(&Value::Int(42)));
    }

    #[test]
    fn test_vm_division_by_zero_returns_error() {
        let mut bytecode = Bytecode::new();
        let ten = bytecode.add_constant(Constant::Int(10));
        let zero = bytecode.add_constant(Constant::Int(0));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(ten),
            Instruction::LoadConst(zero),
            Instruction::Div,
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        let error = vm.run().unwrap_err();

        assert!(matches!(error, VmError::DivisionByZero));
        assert_eq!(error.to_string(), "division by zero");
    }

    #[test]
    fn test_vm_captures_print_output() {
        let mut bytecode = Bytecode::new();
        let value = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(value),
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        vm.set_stdout_enabled(false);
        vm.run().unwrap();

        assert_eq!(vm.take_output(), vec!["42".to_string()]);
    }

    #[test]
    fn test_vm_call_type_error_uses_context_contract() {
        let mut bytecode = Bytecode::new();
        let value = bytecode.add_constant(Constant::Int(42));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(value),
            Instruction::Call(0),
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        let error = vm.run().unwrap_err();

        assert!(matches!(error, VmError::TypeError(_)));
        let msg = error.to_string();
        assert!(msg.contains("VM execution failed [context:op=call]: expected function"));
    }

    #[test]
    fn test_vm_call_arity_error_uses_context_contract() {
        let mut bytecode = Bytecode::new();
        bytecode.functions.insert(
            "f".to_string(),
            Function {
                name: "f".to_string(),
                param_count: 1,
                instructions: vec![Instruction::Return],
            },
        );
        bytecode.main_instructions = vec![
            Instruction::LoadGlobal("f".to_string()),
            Instruction::Call(0),
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        let error = vm.run().unwrap_err();
        let msg = error.to_string();
        assert!(msg.contains("VM execution failed [context:op=call_arity]:"));
        assert!(msg.contains("function f expects 1 arguments, got 0"));
    }

    #[test]
    fn test_vm_load_index_type_error_uses_context_contract() {
        let mut bytecode = Bytecode::new();
        let list_candidate = bytecode.add_constant(Constant::Int(7));
        let index = bytecode.add_constant(Constant::Int(0));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(list_candidate),
            Instruction::LoadConst(index),
            Instruction::LoadIndex,
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        let error = vm.run().unwrap_err();

        assert!(matches!(error, VmError::TypeError(_)));
        let msg = error.to_string();
        assert!(msg.contains(
            "VM execution failed [context:op=load_index]: expected list or map for indexing"
        ));
    }

    #[test]
    fn test_vm_list_pop_type_error_uses_context_contract() {
        let mut bytecode = Bytecode::new();
        let value = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(value),
            Instruction::ListPop,
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        let error = vm.run().unwrap_err();
        let msg = error.to_string();
        assert!(msg.contains("VM execution failed [context:op=list_pop]: expected list for pop"));
    }

    #[test]
    fn test_vm_map_has_type_error_uses_context_contract() {
        let mut bytecode = Bytecode::new();
        let not_a_map = bytecode.add_constant(Constant::Int(10));
        let key = bytecode.add_constant(Constant::String("k".to_string()));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(not_a_map),
            Instruction::LoadConst(key),
            Instruction::MapHas,
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        let error = vm.run().unwrap_err();
        let msg = error.to_string();
        assert!(msg.contains("VM execution failed [context:op=map_has]: expected map for has"));
    }

    #[test]
    fn test_vm_load_field_type_error_uses_context_contract() {
        let mut bytecode = Bytecode::new();
        let not_struct = bytecode.add_constant(Constant::Int(10));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(not_struct),
            Instruction::LoadField("name".to_string()),
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        let error = vm.run().unwrap_err();
        let msg = error.to_string();
        assert!(msg.contains(
            "VM execution failed [context:op=load_field]: expected struct or map for field access"
        ));
    }

    #[test]
    fn test_vm_store_field_var_type_error_uses_context_contract() {
        let mut bytecode = Bytecode::new();
        let value = bytecode.add_constant(Constant::Int(1));
        let not_struct = bytecode.add_constant(Constant::Int(7));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(not_struct),
            Instruction::StoreGlobal("x".to_string()),
            Instruction::LoadConst(value),
            Instruction::StoreFieldVar {
                target: "x".to_string(),
                field: "name".to_string(),
            },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        let error = vm.run().unwrap_err();
        let msg = error.to_string();
        assert!(msg.contains(
            "VM execution failed [context:op=store_field_var]: expected struct or map variable 'x'"
        ));
    }

    #[test]
    fn test_vm_list_push_var_type_error_uses_context_contract() {
        let mut bytecode = Bytecode::new();
        let not_list = bytecode.add_constant(Constant::Int(7));
        let value = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(not_list),
            Instruction::StoreGlobal("x".to_string()),
            Instruction::LoadConst(value),
            Instruction::ListPushVar("x".to_string()),
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        let error = vm.run().unwrap_err();
        let msg = error.to_string();
        assert!(msg.contains(
            "VM execution failed [context:op=list_push_var]: expected list variable 'x'"
        ));
    }

    #[test]
    fn test_vm_store_index_var_type_error_uses_context_contract() {
        let mut bytecode = Bytecode::new();
        let not_list_or_map = bytecode.add_constant(Constant::Int(7));
        let index = bytecode.add_constant(Constant::Int(0));
        let value = bytecode.add_constant(Constant::Int(1));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(not_list_or_map),
            Instruction::StoreGlobal("x".to_string()),
            Instruction::LoadConst(index),
            Instruction::LoadConst(value),
            Instruction::StoreIndexVar("x".to_string()),
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        let error = vm.run().unwrap_err();
        let msg = error.to_string();
        assert!(msg.contains(
            "VM execution failed [context:op=store_index_var]: expected list or map variable 'x'"
        ));
    }

    #[test]
    fn test_vm_backend_not_found_uses_context_contract() {
        let mut vm = Vm::new(Bytecode::new());
        let err = vm.call_backend("missing", "ping", vec![]).unwrap_err();

        match err {
            VmError::BackendError(message) => {
                assert!(message.starts_with("Backend call failed"));
                assert!(message.contains("[context:backend=missing,method=ping]"));
                assert!(message.contains("backend not found"));
            }
            other => panic!("expected backend error, got {}", other),
        }
    }

    // Sprint 24 Phase 2: Memory Pool Tests

    #[test]
    fn test_memory_pool_initialization() {
        let bytecode = Bytecode::new();
        let vm = Vm::new(bytecode);

        let stats = vm.memory_pool_stats();
        assert_eq!(stats.chunk_count, 0);
        assert_eq!(stats.total_allocated, 0);
        assert_eq!(stats.total_used, 0);
        assert_eq!(stats.allocation_count, 0);
    }

    #[test]
    fn test_memory_pool_reset() {
        let bytecode = Bytecode::new();
        let vm = Vm::new(bytecode);

        // Allocate some memory
        vm.memory_pool.allocate(100).unwrap();
        vm.memory_pool.allocate(200).unwrap();

        let stats_before = vm.memory_pool_stats();
        assert_eq!(stats_before.total_used, 300);
        assert_eq!(stats_before.allocation_count, 2);

        // Reset pool
        vm.reset_memory_pool();

        let stats_after = vm.memory_pool_stats();
        assert_eq!(stats_after.chunk_count, 1); // Chunks still exist
        assert_eq!(stats_after.total_used, 0); // But usage is reset
        assert_eq!(stats_after.allocation_count, 0);
    }

    #[test]
    fn test_memory_pool_clear() {
        let bytecode = Bytecode::new();
        let vm = Vm::new(bytecode);

        // Allocate some memory
        vm.memory_pool.allocate(100).unwrap();
        vm.memory_pool.allocate(200).unwrap();

        // Clear pool
        vm.clear_memory_pool();

        let stats = vm.memory_pool_stats();
        assert_eq!(stats.chunk_count, 0);
        assert_eq!(stats.total_allocated, 0);
        assert_eq!(stats.total_used, 0);
        assert_eq!(stats.allocation_count, 0);
    }

    #[test]
    fn test_memory_pool_stats_display() {
        let bytecode = Bytecode::new();
        let vm = Vm::new(bytecode);

        vm.memory_pool.allocate(100).unwrap();

        let stats = vm.memory_pool_stats();
        let display = format!("{}", stats);
        assert!(display.contains("Memory Pool Statistics"));
        assert!(display.contains("Chunks"));
        assert!(display.contains("Total allocated"));
        assert!(display.contains("Total used"));
    }

    // Sprint 24 Phase 3: Cycle Detector Tests

    #[test]
    fn test_cycle_detector_initialization() {
        let bytecode = Bytecode::new();
        let vm = Vm::new(bytecode);

        let stats = vm.cycle_detector_stats();
        assert_eq!(stats.tracked_objects, 0);
        assert_eq!(stats.collections_run, 0);
        assert_eq!(stats.cycles_detected, 0);
        assert_eq!(stats.threshold, 1000);
    }

    #[test]
    fn test_gc_threshold() {
        let bytecode = Bytecode::new();
        let mut vm = Vm::new(bytecode);

        assert_eq!(vm.gc_threshold(), 1000);

        vm.set_gc_threshold(500);
        assert_eq!(vm.gc_threshold(), 500);

        let stats = vm.cycle_detector_stats();
        assert_eq!(stats.threshold, 500);
    }

    #[test]
    fn test_force_gc() {
        let bytecode = Bytecode::new();
        let vm = Vm::new(bytecode);

        // Force GC should run without errors
        let result = vm.force_gc();
        assert_eq!(result.cycles_found, 0);
        assert_eq!(result.objects_collected, 0);

        let stats = vm.cycle_detector_stats();
        assert_eq!(stats.collections_run, 1);
    }

    #[test]
    fn test_clear_cycle_detector() {
        let bytecode = Bytecode::new();
        let vm = Vm::new(bytecode);

        // Clear should work without errors
        vm.clear_cycle_detector();

        let stats = vm.cycle_detector_stats();
        assert_eq!(stats.tracked_objects, 0);
    }

    #[test]
    fn test_cycle_detector_stats_display() {
        let bytecode = Bytecode::new();
        let vm = Vm::new(bytecode);

        let stats = vm.cycle_detector_stats();
        let display = format!("{}", stats);
        assert!(display.contains("Cycle Detector Statistics"));
        assert!(display.contains("Tracked objects"));
        assert!(display.contains("Collections run"));
        assert!(display.contains("Cycles detected"));
    }

    #[test]
    fn test_lookup_native_func_returns_null_for_missing_function() {
        let bytecode = Bytecode::new();
        let mut vm = Vm::new(bytecode);
        let name = CString::new("missing_fn").unwrap();

        let ptr = lookup_native_func(&mut vm as *mut Vm as *mut std::ffi::c_void, name.as_ptr());
        assert!(ptr.is_null());
    }

    #[test]
    fn test_lookup_native_func_returns_cached_code_pointer() {
        let bytecode = Bytecode::new();
        let mut vm = Vm::new(bytecode);
        let name = CString::new("cached_fn").unwrap();

        let memory = ExecutableMemory::new(&[0xC3]).unwrap();
        let func = NativeFunction::new("cached_fn".to_string(), 1, memory);
        vm.jit_compiler
            .code_cache
            .insert("cached_fn".to_string(), func)
            .unwrap();

        let ptr = lookup_native_func(&mut vm as *mut Vm as *mut std::ffi::c_void, name.as_ptr());
        assert!(!ptr.is_null());
    }

    #[test]
    fn test_backend_call_has_higher_estimated_cost_than_local_op() {
        let mut bytecode = Bytecode::new();
        let c1 = bytecode.add_constant(Constant::Int(1));
        let c2 = bytecode.add_constant(Constant::Int(2));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(c1),
            Instruction::LoadConst(c2),
            Instruction::Add,
            Instruction::BackendCall {
                backend: "agent".to_string(),
                method: "say".to_string(),
                arg_count: 1,
            },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        vm.register_backend(
            "agent".to_string(),
            Box::new(matter_backend::AgentBackend::new()),
        );
        vm.run().unwrap();

        assert!(vm.estimated_backend_cost() > 0.0);
        assert!(vm.estimated_instruction_cost() > 0.0);
        assert!(vm.estimated_backend_cost() > 1.0);
    }

    #[test]
    fn test_tool_backend_call_is_estimated_as_expensive() {
        let mut bytecode = Bytecode::new();
        let tool_name = bytecode.add_constant(Constant::String("agent.backend".to_string()));
        bytecode.main_instructions = vec![
            Instruction::LoadConst(tool_name),
            Instruction::BackendCall {
                backend: "tool".to_string(),
                method: "call".to_string(),
                arg_count: 1,
            },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(bytecode);
        vm.register_backend(
            "tool".to_string(),
            Box::new(matter_backend::ToolBackend::new()),
        );
        vm.run().unwrap();

        assert!(vm.estimated_backend_cost() >= 12.0);
    }
}

// ============================================================================
// NATIVE RUNTIME HELPERS (C ABI)
// ============================================================================

extern "C" fn lookup_native_func(_vm_ptr: *mut std::ffi::c_void, _name: *const i8) -> *const u8 {
    if _vm_ptr.is_null() || _name.is_null() {
        return std::ptr::null();
    }

    unsafe {
        let vm: &mut Vm = &mut *(_vm_ptr as *mut Vm);
        let name = std::ffi::CStr::from_ptr(_name).to_string_lossy();
        vm.jit_compiler
            .code_cache
            .get_code_ptr(name.as_ref())
            .unwrap_or(std::ptr::null())
    }
}

extern "C" fn get_vm_global(vm_ptr: *mut std::ffi::c_void, name_ptr: *const i8) -> i64 {
    if vm_ptr.is_null() || name_ptr.is_null() {
        return 0;
    }

    unsafe {
        let vm: &Vm = &*(vm_ptr as *const Vm);
        let name = std::ffi::CStr::from_ptr(name_ptr).to_string_lossy();

        if let Some(Value::Int(val)) = vm.globals.get(name.as_ref()) {
            *val
        } else {
            0
        }
    }
}

extern "C" fn set_vm_global(vm_ptr: *mut std::ffi::c_void, name_ptr: *const i8, value: i64) {
    if vm_ptr.is_null() || name_ptr.is_null() {
        return;
    }

    unsafe {
        let vm: &mut Vm = &mut *(vm_ptr as *mut Vm);
        let name = std::ffi::CStr::from_ptr(name_ptr).to_string_lossy();
        vm.globals.insert(name.to_string(), Value::Int(value));
    }
}
