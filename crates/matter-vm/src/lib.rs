/// Matter Virtual Machine
/// Stack-based VM para executar bytecode MBC

use matter_backend::{Backend, Value};
use matter_bytecode::*;
use std::collections::{HashMap, VecDeque};
use std::fmt;

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
}

impl ScopeFrame {
    fn new(scope_type: ScopeType) -> Self {
        Self {
            _scope_type: scope_type,
            variables: HashMap::new(),
        }
    }
}

/// Call frame para execução de funções (mantido para compatibilidade)
#[derive(Debug, Clone)]
struct CallFrame {
    _function_name: String,
    _return_address: usize,
}

impl CallFrame {
    fn new(function_name: String, return_address: usize) -> Self {
        Self {
            _function_name: function_name,
            _return_address: return_address,
        }
    }
}

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
        }
    }
    
    pub fn register_backend(&mut self, name: String, backend: Box<dyn Backend>) {
        self.backends.insert(name, backend);
    }

    pub fn set_stdout_enabled(&mut self, enabled: bool) {
        self.stdout_enabled = enabled;
    }

    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output)
    }
    
    /// Push novo escopo
    fn push_scope(&mut self, scope_type: ScopeType) {
        self.scope_stack.push(ScopeFrame::new(scope_type));
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
                return Err(VmError::TypeError(
                    "spawn event queue exceeded 10000 events".to_string(),
                ));
            }
            self.emit_event_now(&event)?;
        }
        Ok(())
    }
    
    fn execute(&mut self, instructions: &[Instruction]) -> Result<(), VmError> {
        let mut ip = 0; // instruction pointer
        
        while ip < instructions.len() {
            match &instructions[ip] {
                Instruction::LoadConst(id) => {
                    let constant = &self.bytecode.constants[*id];
                    let value = match constant {
                        Constant::Int(n) => Value::Int(*n),
                        Constant::Bool(b) => Value::Bool(*b),
                        Constant::String(s) => Value::String(s.clone()),
                        Constant::Unit => Value::Unit,
                    };
                    self.stack.push(value);
                }
                
                Instruction::LoadGlobal(name) => {
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
                        self.stack.push(Value::Function(name.clone()));
                    } else {
                        return Err(VmError::UndefinedVariable(name.clone()));
                    }
                }
                
                Instruction::StoreGlobal(name) => {
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
                    let result = Value::Int(
                        left.as_int().map_err(|e| VmError::TypeError(e))? + 
                        right.as_int().map_err(|e| VmError::TypeError(e))?
                    );
                    self.stack.push(result);
                }
                
                Instruction::Sub => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = Value::Int(
                        left.as_int().map_err(|e| VmError::TypeError(e))? - 
                        right.as_int().map_err(|e| VmError::TypeError(e))?
                    );
                    self.stack.push(result);
                }
                
                Instruction::Mul => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = Value::Int(
                        left.as_int().map_err(|e| VmError::TypeError(e))? * 
                        right.as_int().map_err(|e| VmError::TypeError(e))?
                    );
                    self.stack.push(result);
                }
                
                Instruction::Div => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let right = right.as_int().map_err(|e| VmError::TypeError(e))?;
                    if right == 0 {
                        return Err(VmError::DivisionByZero);
                    }
                    let result = Value::Int(left.as_int().map_err(|e| VmError::TypeError(e))? / right);
                    self.stack.push(result);
                }
                
                Instruction::Eq => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = Value::Bool(left == right);
                    self.stack.push(result);
                }
                
                Instruction::NotEq => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = Value::Bool(left != right);
                    self.stack.push(result);
                }
                
                Instruction::Lt => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = Value::Bool(
                        left.as_int().map_err(|e| VmError::TypeError(e))? < 
                        right.as_int().map_err(|e| VmError::TypeError(e))?
                    );
                    self.stack.push(result);
                }
                
                Instruction::Gt => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = Value::Bool(
                        left.as_int().map_err(|e| VmError::TypeError(e))? > 
                        right.as_int().map_err(|e| VmError::TypeError(e))?
                    );
                    self.stack.push(result);
                }
                
                Instruction::LtEq => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = Value::Bool(
                        left.as_int().map_err(|e| VmError::TypeError(e))? <= 
                        right.as_int().map_err(|e| VmError::TypeError(e))?
                    );
                    self.stack.push(result);
                }
                
                Instruction::GtEq => {
                    let right = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let left = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let result = Value::Bool(
                        left.as_int().map_err(|e| VmError::TypeError(e))? >= 
                        right.as_int().map_err(|e| VmError::TypeError(e))?
                    );
                    self.stack.push(result);
                }
                
                Instruction::Jump(target) => {
                    ip = *target;
                    continue;
                }
                
                Instruction::JumpIfFalse(target) => {
                    let condition = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    if !condition.as_bool().map_err(|e| VmError::TypeError(e))? {
                        ip = *target;
                        continue;
                    }
                }
                
                Instruction::Call(arg_count) => {
                    let func_value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    
                    if let Value::Function(func_name) = func_value {
                        // Clonar função para evitar borrow issues
                        let function = self.bytecode.functions.get(&func_name)
                            .ok_or_else(|| VmError::UndefinedFunction(func_name.clone()))?
                            .clone();
                        
                        // Criar call frame
                        let frame = CallFrame::new(func_name.clone(), ip + 1);
                        
                        // Coletar argumentos da stack
                        let mut args = Vec::new();
                        for _ in 0..*arg_count {
                            args.push(self.stack.pop().ok_or(VmError::StackUnderflow)?);
                        }
                        args.reverse();
                        
                        // Verificar número de parâmetros
                        if args.len() != function.param_count {
                            return Err(VmError::TypeError(format!(
                                "Function {} expects {} arguments, got {}",
                                func_name, function.param_count, args.len()
                            )));
                        }
                        
                        // Push call frame
                        self.call_stack.push(frame);
                        let scope_depth_before_call = self.scope_stack.len();

                        // Push function scope
                        self.push_scope(ScopeType::Function);
                        
                        // Bind argumentos aos parâmetros no scope
                        for (i, arg) in args.into_iter().enumerate() {
                            let param_name = format!("__param_{}", i);
                            if let Some(scope) = self.scope_stack.last_mut() {
                                scope.variables.insert(param_name, arg);
                            }
                        }
                        
                        // Executar função
                        let func_instructions = function.instructions;
                        let result = self.execute(&func_instructions);
                        
                        // Pop function scope (cleanup automático)
                        while self.scope_stack.len() > scope_depth_before_call {
                            self.pop_scope();
                        }
                        
                        // Pop call frame
                        self.call_stack.pop();
                        result?;
                        
                        // Return value já está na stack
                    } else {
                        return Err(VmError::TypeError("Not a function".to_string()));
                    }
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
                
                Instruction::BackendCall { backend, method, arg_count } => {
                    let mut args = Vec::new();
                    for _ in 0..*arg_count {
                        args.push(self.stack.pop().ok_or(VmError::StackUnderflow)?);
                    }
                    args.reverse();
                    
                    if let Some(backend_impl) = self.backends.get_mut(backend) {
                        let result = backend_impl.call(method, args)
                            .map_err(|e| VmError::BackendError(e))?;
                        self.stack.push(result);
                    } else {
                        return Err(VmError::BackendError(format!("Backend '{}' not found", backend)));
                    }
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
                    self.stack.push(Value::List(elements));
                }
                
                Instruction::LoadIndex => {
                    // Pop index, pop list, push value
                    let index = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let list = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    
                    if let Value::List(elements) = list {
                        let idx = index.as_int().map_err(|e| VmError::TypeError(e))? as usize;
                        if idx < elements.len() {
                            self.stack.push(elements[idx].clone());
                        } else {
                            return Err(VmError::TypeError(format!("Index {} out of bounds", idx)));
                        }
                    } else if let Value::Map(entries) = list {
                        let key = index.as_string().map_err(VmError::TypeError)?;
                        if let Some(value) = entries.get(&key) {
                            self.stack.push(value.clone());
                        } else {
                            return Err(VmError::TypeError(format!("Map key '{}' not found", key)));
                        }
                    } else {
                        return Err(VmError::TypeError("Expected list or map for indexing".to_string()));
                    }
                }
                
                Instruction::StoreIndex => {
                    // Pop value, pop index, pop list (mutates list in place)
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let index = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let list = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    
                    if let Value::List(mut elements) = list {
                        let idx = index.as_int().map_err(|e| VmError::TypeError(e))? as usize;
                        if idx < elements.len() {
                            elements[idx] = value;
                            // Note: This doesn't actually mutate the original list
                            // We need to update the variable that holds the list
                            // For now, this is a limitation
                        } else {
                            return Err(VmError::TypeError(format!("Index {} out of bounds", idx)));
                        }
                    } else {
                        return Err(VmError::TypeError("Expected list for indexing".to_string()));
                    }
                }
                
                Instruction::ListPush => {
                    // Pop value, pop list, push list (with value appended)
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let list = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    
                    if let Value::List(mut elements) = list {
                        elements.push(value);
                        self.stack.push(Value::List(elements));
                    } else {
                        return Err(VmError::TypeError("Expected list for push".to_string()));
                    }
                }
                
                Instruction::ListPop => {
                    // Pop list, push value, push list (with last element removed)
                    let list = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    
                    if let Value::List(mut elements) = list {
                        if let Some(value) = elements.pop() {
                            self.stack.push(value);
                            self.stack.push(Value::List(elements));
                        } else {
                            return Err(VmError::TypeError("Cannot pop from empty list".to_string()));
                        }
                    } else {
                        return Err(VmError::TypeError("Expected list for pop".to_string()));
                    }
                }
                
                Instruction::ListLen => {
                    // Pop list, push length
                    let list = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    
                    if let Value::List(elements) = list {
                        self.stack.push(Value::Int(elements.len() as i64));
                    } else {
                        return Err(VmError::TypeError("Expected list for len".to_string()));
                    }
                }

                Instruction::StoreIndexVar(name) => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let index = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let list = self.load_existing_variable(name)?;

                    if let Value::List(mut elements) = list {
                        let idx = index.as_int().map_err(VmError::TypeError)? as usize;
                        if idx < elements.len() {
                            elements[idx] = value;
                            self.update_existing_variable(name, Value::List(elements))?;
                        } else {
                            return Err(VmError::TypeError(format!("Index {} out of bounds", idx)));
                        }
                    } else if let Value::Map(mut entries) = list {
                        let key = index.as_string().map_err(VmError::TypeError)?;
                        entries.insert(key, value);
                        self.update_existing_variable(name, Value::Map(entries))?;
                    } else {
                        return Err(VmError::TypeError(format!("Expected list or map variable '{}'", name)));
                    }
                }

                Instruction::ListPushVar(name) => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let list = self.load_existing_variable(name)?;

                    if let Value::List(mut elements) = list {
                        elements.push(value);
                        self.update_existing_variable(name, Value::List(elements))?;
                        self.stack.push(Value::Unit);
                    } else {
                        return Err(VmError::TypeError(format!("Expected list variable '{}'", name)));
                    }
                }

                Instruction::ListPopVar(name) => {
                    let list = self.load_existing_variable(name)?;

                    if let Value::List(mut elements) = list {
                        if let Some(value) = elements.pop() {
                            self.update_existing_variable(name, Value::List(elements))?;
                            self.stack.push(value);
                        } else {
                            return Err(VmError::TypeError("Cannot pop from empty list".to_string()));
                        }
                    } else {
                        return Err(VmError::TypeError(format!("Expected list variable '{}'", name)));
                    }
                }

                Instruction::NewMap(size) => {
                    let mut entries = HashMap::new();
                    for _ in 0..*size {
                        let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                        let key = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                        entries.insert(key.as_string().map_err(VmError::TypeError)?, value);
                    }
                    self.stack.push(Value::Map(entries));
                }

                Instruction::MapHas => {
                    let key = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let map = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::Map(entries) = map {
                        let key = key.as_string().map_err(VmError::TypeError)?;
                        self.stack.push(Value::Bool(entries.contains_key(&key)));
                    } else {
                        return Err(VmError::TypeError("Expected map for has".to_string()));
                    }
                }

                Instruction::MapKeys => {
                    let map = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::Map(entries) = map {
                        let mut keys: Vec<String> = entries.keys().cloned().collect();
                        keys.sort();
                        self.stack.push(Value::List(
                            keys.into_iter().map(Value::String).collect(),
                        ));
                    } else {
                        return Err(VmError::TypeError("Expected map for keys".to_string()));
                    }
                }

                Instruction::MapValues => {
                    let map = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    if let Value::Map(entries) = map {
                        let mut pairs: Vec<(String, Value)> = entries.into_iter().collect();
                        pairs.sort_by(|left, right| left.0.cmp(&right.0));
                        self.stack.push(Value::List(
                            pairs.into_iter().map(|(_, value)| value).collect(),
                        ));
                    } else {
                        return Err(VmError::TypeError("Expected map for values".to_string()));
                    }
                }

                Instruction::NewStruct(type_name, size) => {
                    let mut fields = HashMap::new();
                    for _ in 0..*size {
                        let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                        let field = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                        fields.insert(field.as_string().map_err(VmError::TypeError)?, value);
                    }
                    self.stack.push(Value::Struct {
                        type_name: type_name.clone(),
                        fields,
                    });
                }

                Instruction::LoadField(field) => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;

                    match value {
                        Value::Struct { fields, .. } => {
                            if let Some(field_value) = fields.get(field) {
                                self.stack.push(field_value.clone());
                            } else {
                                return Err(VmError::TypeError(format!(
                                    "Struct field '{}' not found",
                                    field
                                )));
                            }
                        }
                        Value::Map(entries) => {
                            if let Some(field_value) = entries.get(field) {
                                self.stack.push(field_value.clone());
                            } else {
                                return Err(VmError::TypeError(format!(
                                    "Map field '{}' not found",
                                    field
                                )));
                            }
                        }
                        _ => {
                            return Err(VmError::TypeError(
                                "Expected struct or map for field access".to_string(),
                            ));
                        }
                    }
                }

                Instruction::StoreFieldVar { target, field } => {
                    let value = self.stack.pop().ok_or(VmError::StackUnderflow)?;
                    let current = self.load_existing_variable(target)?;

                    match current {
                        Value::Struct {
                            type_name,
                            mut fields,
                        } => {
                            fields.insert(field.clone(), value);
                            self.update_existing_variable(
                                target,
                                Value::Struct { type_name, fields },
                            )?;
                        }
                        Value::Map(mut entries) => {
                            entries.insert(field.clone(), value);
                            self.update_existing_variable(target, Value::Map(entries))?;
                        }
                        _ => {
                            return Err(VmError::TypeError(format!(
                                "Expected struct or map variable '{}'",
                                target
                            )));
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

#[cfg(test)]
mod tests {
    use super::*;
    
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
}
