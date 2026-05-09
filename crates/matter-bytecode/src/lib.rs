/// Bytecode generation and serialization for Matter
/// Formato MBC1 (Matter Bytecode v1)

use matter_ast::*;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::{Result, BufWriter, BufReader};
use std::path::Path;

mod serialize;
mod deserialize;

pub use serialize::*;

/// Instruções da VM Matter
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // Stack operations
    LoadConst(ConstantId),
    LoadGlobal(String),
    StoreGlobal(String),
    LoadLocal(String),
    StoreLocal(String),
    StoreExisting(String),
    
    // Scope management
    PushScope,
    PopScope,
    
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    
    // Comparison
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    
    // Control flow
    Jump(usize),
    JumpIfFalse(usize),
    Call(usize), // número de argumentos
    Return,
    SpawnEvent(String),
    
    // Built-ins
    Print,
    
    // Backend
    BackendCall {
        backend: String,
        method: String,
        arg_count: usize,
    },
    
    // Sprint 4: Data Model - Lists
    NewList(usize),    // Create list with N elements from stack
    LoadIndex,         // Pop index, pop collection, push value
    StoreIndex,        // Pop value, pop index, pop collection
    StoreIndexVar(String), // Pop value, pop index, mutate variable list[index]
    ListPush,          // Pop value, pop list, push list (mutated)
    ListPop,           // Pop list, push value, push list (mutated)
    ListLen,           // Pop list, push length
    ListPushVar(String), // Pop value, mutate variable list, push unit
    ListPopVar(String),  // Mutate variable list, push popped value
    NewMap(usize),     // Create map with N key/value pairs from stack
    MapHas,            // Pop key, pop map, push bool
    MapKeys,           // Pop map, push list of keys
    MapValues,         // Pop map, push list of values
    NewStruct(String, usize), // Create struct with N field/value pairs from stack
    LoadField(String),        // Pop struct/map, push field value
    StoreFieldVar {
        target: String,
        field: String,
    },
    
    // Special
    Pop,
    Halt,
}

pub type ConstantId = usize;

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Int(i64),
    Bool(bool),
    String(String),
    Unit,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub param_count: usize,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct EventHandler {
    pub event: String,
    pub instructions: Vec<Instruction>,
}

/// Bytecode completo de um programa Matter
#[derive(Debug, Clone)]
pub struct Bytecode {
    pub magic: [u8; 4], // "MBC1"
    pub constants: Vec<Constant>,
    pub functions: HashMap<String, Function>,
    pub event_handlers: HashMap<String, EventHandler>,
    pub main_instructions: Vec<Instruction>,
}

impl Bytecode {
    pub fn new() -> Self {
        Self {
            magic: *b"MBC1",
            constants: Vec::new(),
            functions: HashMap::new(),
            event_handlers: HashMap::new(),
            main_instructions: Vec::new(),
        }
    }
    
    pub fn add_constant(&mut self, constant: Constant) -> ConstantId {
        // Reuse existing constants
        if let Some(pos) = self.constants.iter().position(|c| c == &constant) {
            return pos;
        }
        
        let id = self.constants.len();
        self.constants.push(constant);
        id
    }
    
    /// Save bytecode to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        self.serialize(&mut writer)
    }
    
    /// Load bytecode from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        Self::deserialize(&mut reader)
    }
}

impl Default for Bytecode {
    fn default() -> Self {
        Self::new()
    }
}

/// Compilador de AST para Bytecode
pub struct BytecodeBuilder {
    bytecode: Bytecode,
    loop_stack: Vec<LoopContext>,
    temp_counter: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub message: String,
}

impl SemanticError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone, Default)]
struct ValidationContext {
    loop_depth: usize,
    function_depth: usize,
    scopes: Vec<HashSet<String>>,
    ambient_globals: HashSet<String>,
}

impl ValidationContext {
    fn new(ambient_globals: HashSet<String>) -> Self {
        Self {
            loop_depth: 0,
            function_depth: 0,
            scopes: vec![HashSet::new()],
            ambient_globals,
        }
    }

    fn define(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn contains_variable(&self, name: &str, functions: &HashMap<String, usize>) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
            || self.ambient_globals.contains(name)
            || functions.contains_key(name)
    }
}

/// Contexto de loop para break/continue
#[derive(Debug, Clone)]
struct LoopContext {
    break_jumps: Vec<usize>,
    continue_jumps: Vec<usize>,
}

fn validate_program(program: &Program) -> std::result::Result<(), SemanticError> {
    let mut functions = HashMap::new();
    let mut events = HashSet::new();
    let mut structs = HashMap::new();
    let mut global_names = HashSet::new();

    for stmt in &program.statements {
        match stmt {
            Statement::Let { name, .. } => {
                global_names.insert(name.clone());
            }
            Statement::FunctionDef { name, params, .. } => {
                if functions.contains_key(name) {
                    return Err(SemanticError::new(format!(
                        "duplicate function definition '{}'",
                        name
                    )));
                }
                validate_unique_names(params, "function parameter")?;
                functions.insert(name.clone(), params.len());
            }
            Statement::OnEvent { event, .. } => {
                if !events.insert(event.clone()) {
                    return Err(SemanticError::new(format!(
                        "duplicate event handler '{}'",
                        event
                    )));
                }
            }
            Statement::StructDef { name, fields } => {
                if structs.contains_key(name) {
                    return Err(SemanticError::new(format!(
                        "duplicate struct definition '{}'",
                        name
                    )));
                }

                let field_names: Vec<String> = fields.iter().map(|(field, _)| field.clone()).collect();
                validate_unique_names(&field_names, "struct field")?;
                structs.insert(name.clone(), fields.clone());
            }
            _ => {}
        }
    }

    let mut context = ValidationContext::new(HashSet::new());
    for stmt in &program.statements {
        validate_statement(stmt, &mut context, &structs, &functions)?;
    }

    let mut ambient_context = ValidationContext::new(global_names);
    for stmt in &program.statements {
        match stmt {
            Statement::FunctionDef { body, params, .. } => {
                ambient_context.function_depth = 1;
                ambient_context.scopes = vec![params.iter().cloned().collect()];
                validate_block(body, &mut ambient_context, &structs, &functions)?;
            }
            Statement::OnEvent { body, .. } => {
                ambient_context.function_depth = 0;
                ambient_context.scopes = vec![HashSet::new()];
                validate_block(body, &mut ambient_context, &structs, &functions)?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn validate_unique_names(names: &[String], label: &str) -> std::result::Result<(), SemanticError> {
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name) {
            return Err(SemanticError::new(format!(
                "duplicate {} '{}'",
                label, name
            )));
        }
    }
    Ok(())
}

fn validate_statement(
    stmt: &Statement,
    context: &mut ValidationContext,
    structs: &HashMap<String, Vec<(String, String)>>,
    functions: &HashMap<String, usize>,
) -> std::result::Result<(), SemanticError> {
    match stmt {
        Statement::Let { name, value } => {
            validate_expression(value, structs, functions, context)?;
            context.define(name);
        }
        Statement::Set { name, value } => {
            if !context.contains_variable(name, functions) {
                return Err(SemanticError::new(format!(
                    "cannot set undefined variable '{}'",
                    name
                )));
            }
            validate_expression(value, structs, functions, context)?;
        }
        Statement::SetField { target, value, .. } => {
            if !context.contains_variable(target, functions) {
                return Err(SemanticError::new(format!(
                    "cannot set field on undefined variable '{}'",
                    target
                )));
            }
            validate_expression(value, structs, functions, context)?;
        }
        Statement::Return(value) | Statement::Print(value) | Statement::Expression(value) => {
            validate_expression(value, structs, functions, context)?
        }
        Statement::SetIndex { target, index, value } => {
            validate_expression(target, structs, functions, context)?;
            validate_expression(index, structs, functions, context)?;
            validate_expression(value, structs, functions, context)?;
        }
        Statement::FunctionDef { .. } => {}
        Statement::StructDef { .. } => {}
        Statement::OnEvent { .. } => {}
        Statement::Spawn { .. } => {}
        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            validate_expression(condition, structs, functions, context)?;
            context.push_scope();
            validate_block(then_body, context, structs, functions)?;
            context.pop_scope();
            if let Some(else_body) = else_body {
                context.push_scope();
                validate_block(else_body, context, structs, functions)?;
                context.pop_scope();
            }
        }
        Statement::While { condition, body } => {
            validate_expression(condition, structs, functions, context)?;
            context.loop_depth += 1;
            context.push_scope();
            validate_block(body, context, structs, functions)?;
            context.pop_scope();
            context.loop_depth -= 1;
        }
        Statement::For {
            item,
            iterable,
            body,
        } => {
            validate_expression(iterable, structs, functions, context)?;
            context.loop_depth += 1;
            context.push_scope();
            context.define(item);
            validate_block(body, context, structs, functions)?;
            context.pop_scope();
            context.loop_depth -= 1;
        }
        Statement::Loop { body } => {
            context.loop_depth += 1;
            context.push_scope();
            validate_block(body, context, structs, functions)?;
            context.pop_scope();
            context.loop_depth -= 1;
        }
        Statement::Break => {
            if context.loop_depth == 0 {
                return Err(SemanticError::new("'break' used outside of a loop"));
            }
        }
        Statement::Continue => {
            if context.loop_depth == 0 {
                return Err(SemanticError::new("'continue' used outside of a loop"));
            }
        }
    }

    if matches!(stmt, Statement::Return(_)) && context.function_depth == 0 {
        return Err(SemanticError::new("'return' used outside of a function"));
    }

    Ok(())
}

fn validate_block(
    body: &[Statement],
    context: &mut ValidationContext,
    structs: &HashMap<String, Vec<(String, String)>>,
    functions: &HashMap<String, usize>,
) -> std::result::Result<(), SemanticError> {
    for stmt in body {
        validate_statement(stmt, context, structs, functions)?;
    }
    Ok(())
}

fn validate_expression(
    expr: &Expression,
    structs: &HashMap<String, Vec<(String, String)>>,
    functions: &HashMap<String, usize>,
    context: &ValidationContext,
) -> std::result::Result<(), SemanticError> {
    match expr {
        Expression::Int(_)
        | Expression::Bool(_)
        | Expression::String(_)
        | Expression::Unit => {}
        Expression::Identifier(name) => {
            if !context.contains_variable(name, functions) {
                return Err(SemanticError::new(format!(
                    "undefined variable '{}'",
                    name
                )));
            }
        }
        Expression::Binary { left, right, .. } => {
            validate_expression(left, structs, functions, context)?;
            validate_expression(right, structs, functions, context)?;
        }
        Expression::Call { callee, args } => {
            if let Expression::Identifier(name) = callee.as_ref() {
                if let Some(expected) = functions.get(name) {
                    if args.len() != *expected {
                        return Err(SemanticError::new(format!(
                            "function '{}' expects {} argument(s), got {}",
                            name,
                            expected,
                            args.len()
                        )));
                    }
                } else {
                    return Err(SemanticError::new(format!("unknown function '{}'", name)));
                }
            }

            validate_expression(callee, structs, functions, context)?;
            for arg in args {
                validate_expression(arg, structs, functions, context)?;
            }
        }
        Expression::BackendCall { args, .. } => {
            for arg in args {
                validate_expression(arg, structs, functions, context)?;
            }
        }
        Expression::List(elements) => {
            for element in elements {
                validate_expression(element, structs, functions, context)?;
            }
        }
        Expression::Map(entries) => {
            validate_unique_names(
                &entries.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>(),
                "map key",
            )?;
            for (_, value) in entries {
                validate_expression(value, structs, functions, context)?;
            }
        }
        Expression::StructLiteral { type_name, fields } => {
            let declared_fields = structs.get(type_name).ok_or_else(|| {
                SemanticError::new(format!("unknown struct '{}'", type_name))
            })?;
            let field_names: Vec<String> = fields.iter().map(|(field, _)| field.clone()).collect();
            validate_unique_names(&field_names, "struct literal field")?;

            for field in &field_names {
                if !declared_fields
                    .iter()
                    .any(|(declared_field, _)| declared_field == field)
                {
                    return Err(SemanticError::new(format!(
                        "unknown field '{}' for struct '{}'",
                        field, type_name
                    )));
                }
            }

            for (field, field_type) in declared_fields {
                if !field_names.contains(field) {
                    return Err(SemanticError::new(format!(
                        "missing field '{}' for struct '{}'",
                        field, type_name
                    )));
                }

                if let Some((_, value)) = fields.iter().find(|(name, _)| name == field) {
                    if let Some(actual_type) = infer_static_type(value) {
                        if &actual_type != field_type {
                            return Err(SemanticError::new(format!(
                                "field '{}' for struct '{}' expects {}, got {}",
                                field, type_name, field_type, actual_type
                            )));
                        }
                    }
                }
            }

            for (_, value) in fields {
                validate_expression(value, structs, functions, context)?;
            }
        }
        Expression::Index { target, index } => {
            validate_expression(target, structs, functions, context)?;
            validate_expression(index, structs, functions, context)?;
        }
        Expression::Field { target, .. } => {
            validate_expression(target, structs, functions, context)?;
        }
        Expression::MethodCall { target, method, args } => {
            if is_backend_call(target, method) {
                for arg in args {
                    validate_expression(arg, structs, functions, context)?;
                }
            } else {
                validate_method_arity(method, args.len())?;
                validate_expression(target, structs, functions, context)?;
                for arg in args {
                    validate_expression(arg, structs, functions, context)?;
                }
            }
        }
    }

    Ok(())
}

fn validate_method_arity(method: &str, arg_count: usize) -> std::result::Result<(), SemanticError> {
    let expected = match method {
        "push" => Some(1),
        "pop" | "len" | "keys" | "values" => Some(0),
        "has" => Some(1),
        _ => None,
    };

    if let Some(expected) = expected {
        if arg_count != expected {
            return Err(SemanticError::new(format!(
                "method '{}' expects {} argument(s), got {}",
                method, expected, arg_count
            )));
        }
    }

    Ok(())
}

fn is_backend_call(target: &Expression, method: &str) -> bool {
    if let Expression::Identifier(name) = target {
        if is_builtin_backend(name) {
            return true;
        }
    }

    matches!(target, Expression::Identifier(_)) && !matches!(
        method,
        "push" | "pop" | "len" | "has" | "keys" | "values"
    )
}

fn is_builtin_backend(name: &str) -> bool {
    matches!(
        name,
        "agent" | "visual" | "store" | "net" | "math" | "string" | "list"
    )
}

fn infer_static_type(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Int(_) => Some("int".to_string()),
        Expression::Bool(_) => Some("bool".to_string()),
        Expression::String(_) => Some("string".to_string()),
        Expression::Unit => Some("unit".to_string()),
        Expression::List(_) => Some("list".to_string()),
        Expression::Map(_) => Some("map".to_string()),
        Expression::StructLiteral { type_name, .. } => Some(type_name.clone()),
        Expression::Binary { op, .. } => match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => Some("int".to_string()),
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::Lt
            | BinaryOp::Gt
            | BinaryOp::LtEq
            | BinaryOp::GtEq => Some("bool".to_string()),
        },
        Expression::MethodCall { method, .. } => match method.as_str() {
            "len" => Some("int".to_string()),
            "has" => Some("bool".to_string()),
            "keys" | "values" => Some("list".to_string()),
            "push" => Some("unit".to_string()),
            _ => None,
        },
        Expression::Identifier(_)
        | Expression::Call { .. }
        | Expression::BackendCall { .. }
        | Expression::Index { .. }
        | Expression::Field { .. } => None,
    }
}

impl BytecodeBuilder {
    pub fn new() -> Self {
        Self {
            bytecode: Bytecode::new(),
            loop_stack: Vec::new(),
            temp_counter: 0,
        }
    }

    pub fn build_checked(self, program: &Program) -> std::result::Result<Bytecode, SemanticError> {
        validate_program(program)?;
        Ok(self.build(program))
    }
    
    pub fn build(mut self, program: &Program) -> Bytecode {
        let mut main_instructions = Vec::new();
        
        for statement in &program.statements {
            self.compile_statement(statement, &mut main_instructions);
        }
        
        main_instructions.push(Instruction::Halt);
        self.bytecode.main_instructions = main_instructions;
        self.bytecode
    }

    fn next_temp_name(&mut self, prefix: &str) -> String {
        let name = format!("__matter_{}_{}", prefix, self.temp_counter);
        self.temp_counter += 1;
        name
    }
    
    fn compile_statement(&mut self, stmt: &Statement, instructions: &mut Vec<Instruction>) {
        self.compile_statement_with_scope(stmt, instructions, false);
    }

    fn compile_statement_with_scope(
        &mut self,
        stmt: &Statement,
        instructions: &mut Vec<Instruction>,
        local_declarations: bool,
    ) {
        match stmt {
            Statement::Let { name, value } => {
                self.compile_expression(value, instructions);
                if local_declarations {
                    instructions.push(Instruction::StoreLocal(name.clone()));
                } else {
                    instructions.push(Instruction::StoreGlobal(name.clone()));
                }
            }
            
            Statement::Set { name, value } => {
                self.compile_expression(value, instructions);
                instructions.push(Instruction::StoreExisting(name.clone()));
            }
            
            Statement::SetIndex { target, index, value } => {
                // Compile: target[index] = value
                if let Expression::Identifier(name) = target {
                    self.compile_expression(index, instructions);
                    self.compile_expression(value, instructions);
                    instructions.push(Instruction::StoreIndexVar(name.clone()));
                } else {
                    self.compile_expression(target, instructions);
                    self.compile_expression(index, instructions);
                    self.compile_expression(value, instructions);
                    instructions.push(Instruction::StoreIndex);
                }
            }
            
            Statement::SetField { target, field, value } => {
                self.compile_expression(value, instructions);
                instructions.push(Instruction::StoreFieldVar {
                    target: target.clone(),
                    field: field.clone(),
                });
            }
            
            Statement::Print(expr) => {
                self.compile_expression(expr, instructions);
                instructions.push(Instruction::Print);
            }
            
            Statement::FunctionDef { name, params, body } => {
                let mut func_instructions = Vec::new();
                
                // Compilar corpo da função
                for stmt in body {
                    self.compile_function_statement(stmt, &mut func_instructions, params);
                }
                
                // Garantir que função retorna
                if !matches!(func_instructions.last(), Some(Instruction::Return)) {
                    let unit_id = self.bytecode.add_constant(Constant::Unit);
                    func_instructions.push(Instruction::LoadConst(unit_id));
                    func_instructions.push(Instruction::Return);
                }
                
                let function = Function {
                    name: name.clone(),
                    param_count: params.len(),
                    instructions: func_instructions,
                };
                
                self.bytecode.functions.insert(name.clone(), function);
            }
            
            Statement::StructDef { .. } => {
                // Struct definitions are declarations for now; struct literals carry runtime shape.
            }
            
            Statement::OnEvent { event, body } => {
                let mut event_instructions = Vec::new();
                
                for stmt in body {
                    self.compile_statement_with_scope(stmt, &mut event_instructions, true);
                }
                
                let handler = EventHandler {
                    event: event.clone(),
                    instructions: event_instructions,
                };
                
                self.bytecode.event_handlers.insert(event.clone(), handler);
            }

            Statement::Spawn { event } => {
                instructions.push(Instruction::SpawnEvent(event.clone()));
            }
            
            Statement::If { condition, then_body, else_body } => {
                self.compile_expression(condition, instructions);
                
                let jump_if_false_pos = instructions.len();
                instructions.push(Instruction::JumpIfFalse(0)); // placeholder
                
                // Then block com scope
                instructions.push(Instruction::PushScope);
                for stmt in then_body {
                    self.compile_statement_with_scope(stmt, instructions, true);
                }
                instructions.push(Instruction::PopScope);
                
                if let Some(else_stmts) = else_body {
                    let jump_pos = instructions.len();
                    instructions.push(Instruction::Jump(0)); // placeholder
                    
                    let else_start = instructions.len();
                    instructions[jump_if_false_pos] = Instruction::JumpIfFalse(else_start);
                    
                    // Else block com scope
                    instructions.push(Instruction::PushScope);
                    for stmt in else_stmts {
                        self.compile_statement_with_scope(stmt, instructions, true);
                    }
                    instructions.push(Instruction::PopScope);
                    
                    let end_pos = instructions.len();
                    instructions[jump_pos] = Instruction::Jump(end_pos);
                } else {
                    let end_pos = instructions.len();
                    instructions[jump_if_false_pos] = Instruction::JumpIfFalse(end_pos);
                }
            }
            
            Statement::Return(expr) => {
                self.compile_expression(expr, instructions);
                instructions.push(Instruction::Return);
            }
            
            Statement::While { condition, body } => {
                let loop_start = instructions.len();
                
                // Push loop context
                self.loop_stack.push(LoopContext {
                    break_jumps: Vec::new(),
                    continue_jumps: Vec::new(),
                });
                
                // Condition check
                self.compile_expression(condition, instructions);
                
                let jump_if_false_pos = instructions.len();
                instructions.push(Instruction::JumpIfFalse(0)); // placeholder
                
                // Body com scope
                instructions.push(Instruction::PushScope);
                for stmt in body {
                    self.compile_statement_with_scope(stmt, instructions, true);
                }
                instructions.push(Instruction::PopScope);
                
                // Jump back to condition
                instructions.push(Instruction::Jump(loop_start));
                
                // Patch break jumps
                let loop_end = instructions.len();
                instructions[jump_if_false_pos] = Instruction::JumpIfFalse(loop_end);
                
                if let Some(loop_ctx) = self.loop_stack.pop() {
                    for break_pos in loop_ctx.break_jumps {
                        instructions[break_pos] = Instruction::Jump(loop_end);
                    }
                    for continue_pos in loop_ctx.continue_jumps {
                        instructions[continue_pos] = Instruction::Jump(loop_start);
                    }
                }
            }

            Statement::For {
                item,
                iterable,
                body,
            } => {
                self.compile_for_loop(item, iterable, body, instructions, false, &[]);
            }
            
            Statement::Loop { body } => {
                let loop_start = instructions.len();
                
                // Push loop context
                self.loop_stack.push(LoopContext {
                    break_jumps: Vec::new(),
                    continue_jumps: Vec::new(),
                });
                
                // Body com scope
                instructions.push(Instruction::PushScope);
                for stmt in body {
                    self.compile_statement_with_scope(stmt, instructions, true);
                }
                instructions.push(Instruction::PopScope);
                
                // Jump back to start
                instructions.push(Instruction::Jump(loop_start));
                
                // Patch break jumps
                let loop_end = instructions.len();
                
                if let Some(loop_ctx) = self.loop_stack.pop() {
                    for break_pos in loop_ctx.break_jumps {
                        instructions[break_pos] = Instruction::Jump(loop_end);
                    }
                    for continue_pos in loop_ctx.continue_jumps {
                        instructions[continue_pos] = Instruction::Jump(loop_start);
                    }
                }
            }
            
            Statement::Break => {
                if let Some(loop_ctx) = self.loop_stack.last_mut() {
                    loop_ctx.break_jumps.push(instructions.len());
                    instructions.push(Instruction::Jump(0)); // placeholder
                }
            }
            
            Statement::Continue => {
                if let Some(loop_ctx) = self.loop_stack.last_mut() {
                    loop_ctx.continue_jumps.push(instructions.len());
                    instructions.push(Instruction::Jump(0)); // placeholder
                }
            }
            
            Statement::Expression(expr) => {
                self.compile_expression(expr, instructions);
                instructions.push(Instruction::Pop);
            }
        }
    }
    
    fn compile_function_statement(&mut self, stmt: &Statement, instructions: &mut Vec<Instruction>, params: &[String]) {
        match stmt {
            Statement::Let { name, value } => {
                self.compile_function_expression(value, instructions, params);
                instructions.push(Instruction::StoreLocal(name.clone()));
            }
            
            Statement::Set { name, value } => {
                self.compile_function_expression(value, instructions, params);
                instructions.push(Instruction::StoreExisting(name.clone()));
            }
            
            Statement::SetIndex { target, index, value } => {
                self.compile_expression(target, instructions);
                self.compile_expression(index, instructions);
                self.compile_expression(value, instructions);
                instructions.push(Instruction::StoreIndex);
            }
            
            Statement::SetField { target, field, value } => {
                self.compile_function_expression(value, instructions, params);
                instructions.push(Instruction::StoreFieldVar {
                    target: target.clone(),
                    field: field.clone(),
                });
            }
            
            Statement::Print(expr) => {
                self.compile_function_expression(expr, instructions, params);
                instructions.push(Instruction::Print);
            }

            Statement::Spawn { event } => {
                instructions.push(Instruction::SpawnEvent(event.clone()));
            }
            
            Statement::If { condition, then_body, else_body } => {
                self.compile_function_expression(condition, instructions, params);
                
                let jump_if_false_pos = instructions.len();
                instructions.push(Instruction::JumpIfFalse(0));
                
                // Then block com scope
                instructions.push(Instruction::PushScope);
                for stmt in then_body {
                    self.compile_function_statement(stmt, instructions, params);
                }
                instructions.push(Instruction::PopScope);
                
                if let Some(else_stmts) = else_body {
                    let jump_pos = instructions.len();
                    instructions.push(Instruction::Jump(0));
                    
                    let else_start = instructions.len();
                    instructions[jump_if_false_pos] = Instruction::JumpIfFalse(else_start);
                    
                    // Else block com scope
                    instructions.push(Instruction::PushScope);
                    for stmt in else_stmts {
                        self.compile_function_statement(stmt, instructions, params);
                    }
                    instructions.push(Instruction::PopScope);
                    
                    let end_pos = instructions.len();
                    instructions[jump_pos] = Instruction::Jump(end_pos);
                } else {
                    let end_pos = instructions.len();
                    instructions[jump_if_false_pos] = Instruction::JumpIfFalse(end_pos);
                }
            }
            
            Statement::Return(expr) => {
                self.compile_function_expression(expr, instructions, params);
                instructions.push(Instruction::Return);
            }
            
            Statement::While { condition, body } => {
                let loop_start = instructions.len();
                
                // Push loop context
                self.loop_stack.push(LoopContext {
                    break_jumps: Vec::new(),
                    continue_jumps: Vec::new(),
                });
                
                // Condition check
                self.compile_function_expression(condition, instructions, params);
                
                let jump_if_false_pos = instructions.len();
                instructions.push(Instruction::JumpIfFalse(0));
                
                // Body com scope
                instructions.push(Instruction::PushScope);
                for stmt in body {
                    self.compile_function_statement(stmt, instructions, params);
                }
                instructions.push(Instruction::PopScope);
                
                // Jump back to condition
                instructions.push(Instruction::Jump(loop_start));
                
                // Patch jumps
                let loop_end = instructions.len();
                instructions[jump_if_false_pos] = Instruction::JumpIfFalse(loop_end);
                
                if let Some(loop_ctx) = self.loop_stack.pop() {
                    for break_pos in loop_ctx.break_jumps {
                        instructions[break_pos] = Instruction::Jump(loop_end);
                    }
                    for continue_pos in loop_ctx.continue_jumps {
                        instructions[continue_pos] = Instruction::Jump(loop_start);
                    }
                }
            }

            Statement::For {
                item,
                iterable,
                body,
            } => {
                self.compile_for_loop(item, iterable, body, instructions, true, params);
            }
            
            Statement::Loop { body } => {
                let loop_start = instructions.len();
                
                // Push loop context
                self.loop_stack.push(LoopContext {
                    break_jumps: Vec::new(),
                    continue_jumps: Vec::new(),
                });
                
                // Body com scope
                instructions.push(Instruction::PushScope);
                for stmt in body {
                    self.compile_function_statement(stmt, instructions, params);
                }
                instructions.push(Instruction::PopScope);
                
                // Jump back to start
                instructions.push(Instruction::Jump(loop_start));
                
                // Patch jumps
                let loop_end = instructions.len();
                
                if let Some(loop_ctx) = self.loop_stack.pop() {
                    for break_pos in loop_ctx.break_jumps {
                        instructions[break_pos] = Instruction::Jump(loop_end);
                    }
                    for continue_pos in loop_ctx.continue_jumps {
                        instructions[continue_pos] = Instruction::Jump(loop_start);
                    }
                }
            }
            
            Statement::Break => {
                if let Some(loop_ctx) = self.loop_stack.last_mut() {
                    loop_ctx.break_jumps.push(instructions.len());
                    instructions.push(Instruction::Jump(0));
                }
            }
            
            Statement::Continue => {
                if let Some(loop_ctx) = self.loop_stack.last_mut() {
                    loop_ctx.continue_jumps.push(instructions.len());
                    instructions.push(Instruction::Jump(0));
                }
            }
            
            Statement::Expression(expr) => {
                self.compile_function_expression(expr, instructions, params);
                instructions.push(Instruction::Pop);
            }
            
            _ => {
                // Outros statements não suportados dentro de funções por enquanto
            }
        }
    }
    
    fn compile_function_expression(&mut self, expr: &Expression, instructions: &mut Vec<Instruction>, params: &[String]) {
        match expr {
            Expression::Identifier(name) => {
                // Verificar se é parâmetro
                if let Some(idx) = params.iter().position(|p| p == name) {
                    let param_name = format!("__param_{}", idx);
                    instructions.push(Instruction::LoadLocal(param_name));
                } else {
                    // Tentar local, depois global
                    instructions.push(Instruction::LoadGlobal(name.clone()));
                }
            }
            
            Expression::Binary { left, op, right } => {
                self.compile_function_expression(left, instructions, params);
                self.compile_function_expression(right, instructions, params);
                
                let instr = match op {
                    BinaryOp::Add => Instruction::Add,
                    BinaryOp::Sub => Instruction::Sub,
                    BinaryOp::Mul => Instruction::Mul,
                    BinaryOp::Div => Instruction::Div,
                    BinaryOp::Eq => Instruction::Eq,
                    BinaryOp::NotEq => Instruction::NotEq,
                    BinaryOp::Lt => Instruction::Lt,
                    BinaryOp::Gt => Instruction::Gt,
                    BinaryOp::LtEq => Instruction::LtEq,
                    BinaryOp::GtEq => Instruction::GtEq,
                };
                
                instructions.push(instr);
            }
            
            Expression::Call { callee, args } => {
                for arg in args {
                    self.compile_function_expression(arg, instructions, params);
                }
                
                self.compile_function_expression(callee, instructions, params);
                
                instructions.push(Instruction::Call(args.len()));
            }
            
            _ => {
                // Outros casos usam compilação normal
                self.compile_expression(expr, instructions);
            }
        }
    }

    fn compile_for_loop(
        &mut self,
        item: &str,
        iterable: &Expression,
        body: &[Statement],
        instructions: &mut Vec<Instruction>,
        in_function: bool,
        params: &[String],
    ) {
        let list_temp = self.next_temp_name("for_list");
        let index_temp = self.next_temp_name("for_index");

        if in_function {
            self.compile_function_expression(iterable, instructions, params);
            instructions.push(Instruction::StoreLocal(list_temp.clone()));
        } else {
            self.compile_expression(iterable, instructions);
            instructions.push(Instruction::StoreGlobal(list_temp.clone()));
        }

        let zero_id = self.bytecode.add_constant(Constant::Int(0));
        instructions.push(Instruction::LoadConst(zero_id));
        if in_function {
            instructions.push(Instruction::StoreLocal(index_temp.clone()));
        } else {
            instructions.push(Instruction::StoreGlobal(index_temp.clone()));
        }

        let loop_start = instructions.len();

        if in_function {
            instructions.push(Instruction::LoadLocal(index_temp.clone()));
            instructions.push(Instruction::LoadLocal(list_temp.clone()));
        } else {
            instructions.push(Instruction::LoadGlobal(index_temp.clone()));
            instructions.push(Instruction::LoadGlobal(list_temp.clone()));
        }
        instructions.push(Instruction::ListLen);
        instructions.push(Instruction::Lt);

        let jump_if_false_pos = instructions.len();
        instructions.push(Instruction::JumpIfFalse(0));

        self.loop_stack.push(LoopContext {
            break_jumps: Vec::new(),
            continue_jumps: Vec::new(),
        });

        instructions.push(Instruction::PushScope);
        if in_function {
            instructions.push(Instruction::LoadLocal(list_temp.clone()));
            instructions.push(Instruction::LoadLocal(index_temp.clone()));
            instructions.push(Instruction::LoadIndex);
            instructions.push(Instruction::StoreLocal(item.to_string()));
            for stmt in body {
                self.compile_function_statement(stmt, instructions, params);
            }
        } else {
            instructions.push(Instruction::LoadGlobal(list_temp.clone()));
            instructions.push(Instruction::LoadGlobal(index_temp.clone()));
            instructions.push(Instruction::LoadIndex);
            instructions.push(Instruction::StoreLocal(item.to_string()));
            for stmt in body {
                self.compile_statement_with_scope(stmt, instructions, true);
            }
        }
        instructions.push(Instruction::PopScope);

        let increment_start = instructions.len();
        if in_function {
            instructions.push(Instruction::LoadLocal(index_temp.clone()));
        } else {
            instructions.push(Instruction::LoadGlobal(index_temp.clone()));
        }
        let one_id = self.bytecode.add_constant(Constant::Int(1));
        instructions.push(Instruction::LoadConst(one_id));
        instructions.push(Instruction::Add);
        if in_function {
            instructions.push(Instruction::StoreLocal(index_temp));
        } else {
            instructions.push(Instruction::StoreGlobal(index_temp));
        }
        instructions.push(Instruction::Jump(loop_start));

        let loop_end = instructions.len();
        instructions[jump_if_false_pos] = Instruction::JumpIfFalse(loop_end);

        if let Some(loop_ctx) = self.loop_stack.pop() {
            for break_pos in loop_ctx.break_jumps {
                instructions[break_pos] = Instruction::Jump(loop_end);
            }
            for continue_pos in loop_ctx.continue_jumps {
                instructions[continue_pos] = Instruction::Jump(increment_start);
            }
        }
    }
    
    fn compile_expression(&mut self, expr: &Expression, instructions: &mut Vec<Instruction>) {
        match expr {
            Expression::Int(n) => {
                let id = self.bytecode.add_constant(Constant::Int(*n));
                instructions.push(Instruction::LoadConst(id));
            }
            
            Expression::Bool(b) => {
                let id = self.bytecode.add_constant(Constant::Bool(*b));
                instructions.push(Instruction::LoadConst(id));
            }
            
            Expression::String(s) => {
                let id = self.bytecode.add_constant(Constant::String(s.clone()));
                instructions.push(Instruction::LoadConst(id));
            }
            
            Expression::Unit => {
                let id = self.bytecode.add_constant(Constant::Unit);
                instructions.push(Instruction::LoadConst(id));
            }
            
            Expression::Identifier(name) => {
                instructions.push(Instruction::LoadGlobal(name.clone()));
            }
            
            Expression::Binary { left, op, right } => {
                self.compile_expression(left, instructions);
                self.compile_expression(right, instructions);
                
                let instr = match op {
                    BinaryOp::Add => Instruction::Add,
                    BinaryOp::Sub => Instruction::Sub,
                    BinaryOp::Mul => Instruction::Mul,
                    BinaryOp::Div => Instruction::Div,
                    BinaryOp::Eq => Instruction::Eq,
                    BinaryOp::NotEq => Instruction::NotEq,
                    BinaryOp::Lt => Instruction::Lt,
                    BinaryOp::Gt => Instruction::Gt,
                    BinaryOp::LtEq => Instruction::LtEq,
                    BinaryOp::GtEq => Instruction::GtEq,
                };
                
                instructions.push(instr);
            }
            
            Expression::Call { callee, args } => {
                // Push arguments onto stack
                for arg in args {
                    self.compile_expression(arg, instructions);
                }
                
                // Push function reference
                self.compile_expression(callee, instructions);
                
                instructions.push(Instruction::Call(args.len()));
            }
            
            Expression::BackendCall { backend, method, args } => {
                // Push arguments onto stack
                for arg in args {
                    self.compile_expression(arg, instructions);
                }
                
                instructions.push(Instruction::BackendCall {
                    backend: backend.clone(),
                    method: method.clone(),
                    arg_count: args.len(),
                });
            }
            
            // Sprint 4: Data Model - Lists
            Expression::List(elements) => {
                // Push all elements onto stack
                for elem in elements {
                    self.compile_expression(elem, instructions);
                }
                
                // Create list with N elements
                instructions.push(Instruction::NewList(elements.len()));
            }

            Expression::Map(entries) => {
                for (key, value) in entries {
                    let key_id = self.bytecode.add_constant(Constant::String(key.clone()));
                    instructions.push(Instruction::LoadConst(key_id));
                    self.compile_expression(value, instructions);
                }

                instructions.push(Instruction::NewMap(entries.len()));
            }
            
            Expression::StructLiteral { type_name, fields } => {
                for (field, value) in fields {
                    let field_id = self.bytecode.add_constant(Constant::String(field.clone()));
                    instructions.push(Instruction::LoadConst(field_id));
                    self.compile_expression(value, instructions);
                }
                
                instructions.push(Instruction::NewStruct(type_name.clone(), fields.len()));
            }
            
            Expression::Field { target, field } => {
                self.compile_expression(target, instructions);
                instructions.push(Instruction::LoadField(field.clone()));
            }
            
            Expression::Index { target, index } => {
                // Compile: target[index]
                self.compile_expression(target, instructions);
                self.compile_expression(index, instructions);
                instructions.push(Instruction::LoadIndex);
            }
            
            Expression::MethodCall { target, method, args } => {
                self.compile_method_call(target, method, args, instructions);
            }
        }
    }

    fn compile_method_call(
        &mut self,
        target: &Expression,
        method: &str,
        args: &[Expression],
        instructions: &mut Vec<Instruction>,
    ) {
        if let Expression::Identifier(backend) = target {
            if is_builtin_backend(backend) {
                for arg in args {
                    self.compile_expression(arg, instructions);
                }
                instructions.push(Instruction::BackendCall {
                    backend: backend.clone(),
                    method: method.to_string(),
                    arg_count: args.len(),
                });
                return;
            }
        }

        match (target, method) {
            (Expression::Identifier(name), "push") => {
                for arg in args {
                    self.compile_expression(arg, instructions);
                }
                instructions.push(Instruction::ListPushVar(name.clone()));
            }
            (Expression::Identifier(name), "pop") => {
                instructions.push(Instruction::ListPopVar(name.clone()));
            }
            (_, "push") => {
                self.compile_expression(target, instructions);
                for arg in args {
                    self.compile_expression(arg, instructions);
                }
                instructions.push(Instruction::ListPush);
            }
            (_, "pop") => {
                self.compile_expression(target, instructions);
                instructions.push(Instruction::ListPop);
            }
            (_, "len") => {
                self.compile_expression(target, instructions);
                instructions.push(Instruction::ListLen);
            }
            (_, "has") => {
                self.compile_expression(target, instructions);
                for arg in args {
                    self.compile_expression(arg, instructions);
                }
                instructions.push(Instruction::MapHas);
            }
            (_, "keys") => {
                self.compile_expression(target, instructions);
                instructions.push(Instruction::MapKeys);
            }
            (_, "values") => {
                self.compile_expression(target, instructions);
                instructions.push(Instruction::MapValues);
            }
            (Expression::Identifier(backend), _) => {
                for arg in args {
                    self.compile_expression(arg, instructions);
                }
                instructions.push(Instruction::BackendCall {
                    backend: backend.clone(),
                    method: method.to_string(),
                    arg_count: args.len(),
                });
            }
            _ => panic!("Unknown method: {}", method),
        }
    }
}

impl Default for BytecodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compile_simple() {
        let program = Program::new(vec![
            Statement::Let {
                name: "x".to_string(),
                value: Expression::Int(10),
            },
        ]);
        
        let builder = BytecodeBuilder::new();
        let bytecode = builder.build(&program);
        
        assert_eq!(bytecode.magic, *b"MBC1");
        assert_eq!(bytecode.constants.len(), 1);
    }

    #[test]
    fn test_compile_list_mutation() {
        let program = Program::new(vec![
            Statement::Let {
                name: "items".to_string(),
                value: Expression::List(vec![Expression::Int(1)]),
            },
            Statement::Expression(Expression::MethodCall {
                target: Box::new(Expression::Identifier("items".to_string())),
                method: "push".to_string(),
                args: vec![Expression::Int(2)],
            }),
            Statement::Let {
                name: "last".to_string(),
                value: Expression::MethodCall {
                    target: Box::new(Expression::Identifier("items".to_string())),
                    method: "pop".to_string(),
                    args: Vec::new(),
                },
            },
            Statement::SetIndex {
                target: Expression::Identifier("items".to_string()),
                index: Expression::Int(0),
                value: Expression::Int(3),
            },
        ]);

        let bytecode = BytecodeBuilder::new().build(&program);

        assert!(bytecode
            .main_instructions
            .iter()
            .any(|instr| matches!(instr, Instruction::ListPushVar(name) if name == "items")));
        assert!(bytecode
            .main_instructions
            .iter()
            .any(|instr| matches!(instr, Instruction::ListPopVar(name) if name == "items")));
        assert!(bytecode
            .main_instructions
            .iter()
            .any(|instr| matches!(instr, Instruction::StoreIndexVar(name) if name == "items")));
    }

    #[test]
    fn test_compile_map_literal_and_methods() {
        let program = Program::new(vec![
            Statement::Let {
                name: "person".to_string(),
                value: Expression::Map(vec![("name".to_string(), Expression::String("Alice".to_string()))]),
            },
            Statement::Print(Expression::Index {
                target: Box::new(Expression::Identifier("person".to_string())),
                index: Box::new(Expression::String("name".to_string())),
            }),
            Statement::Print(Expression::MethodCall {
                target: Box::new(Expression::Identifier("person".to_string())),
                method: "has".to_string(),
                args: vec![Expression::String("name".to_string())],
            }),
        ]);

        let bytecode = BytecodeBuilder::new().build(&program);

        assert!(bytecode
            .main_instructions
            .iter()
            .any(|instr| matches!(instr, Instruction::NewMap(1))));
        assert!(bytecode
            .main_instructions
            .iter()
            .any(|instr| matches!(instr, Instruction::MapHas)));
    }

    #[test]
    fn test_compile_struct_literal_and_fields() {
        let program = Program::new(vec![
            Statement::StructDef {
                name: "User".to_string(),
                fields: vec![
                    ("name".to_string(), "string".to_string()),
                    ("age".to_string(), "int".to_string()),
                ],
            },
            Statement::Let {
                name: "user".to_string(),
                value: Expression::StructLiteral {
                    type_name: "User".to_string(),
                    fields: vec![("name".to_string(), Expression::String("Ana".to_string()))],
                },
            },
            Statement::Print(Expression::Field {
                target: Box::new(Expression::Identifier("user".to_string())),
                field: "name".to_string(),
            }),
            Statement::SetField {
                target: "user".to_string(),
                field: "name".to_string(),
                value: Expression::String("Ana Core".to_string()),
            },
        ]);

        let bytecode = BytecodeBuilder::new().build(&program);

        assert!(bytecode
            .main_instructions
            .iter()
            .any(|instr| matches!(instr, Instruction::NewStruct(type_name, 1) if type_name == "User")));
        assert!(bytecode
            .main_instructions
            .iter()
            .any(|instr| matches!(instr, Instruction::LoadField(field) if field == "name")));
        assert!(bytecode.main_instructions.iter().any(|instr| matches!(
            instr,
            Instruction::StoreFieldVar { target, field } if target == "user" && field == "name"
        )));
    }

    #[test]
    fn test_semantic_rejects_break_outside_loop() {
        let program = Program::new(vec![Statement::Break]);
        let error = BytecodeBuilder::new().build_checked(&program).unwrap_err();

        assert_eq!(error.to_string(), "'break' used outside of a loop");
    }

    #[test]
    fn test_semantic_rejects_return_outside_function() {
        let program = Program::new(vec![Statement::Return(Expression::Int(1))]);
        let error = BytecodeBuilder::new().build_checked(&program).unwrap_err();

        assert_eq!(error.to_string(), "'return' used outside of a function");
    }

    #[test]
    fn test_semantic_validates_struct_literal_shape() {
        let program = Program::new(vec![
            Statement::StructDef {
                name: "User".to_string(),
                fields: vec![
                    ("name".to_string(), "string".to_string()),
                    ("age".to_string(), "int".to_string()),
                ],
            },
            Statement::Let {
                name: "user".to_string(),
                value: Expression::StructLiteral {
                    type_name: "User".to_string(),
                    fields: vec![("name".to_string(), Expression::String("Ana".to_string()))],
                },
            },
        ]);

        let error = BytecodeBuilder::new().build_checked(&program).unwrap_err();

        assert_eq!(error.to_string(), "missing field 'age' for struct 'User'");
    }

    #[test]
    fn test_semantic_validates_struct_literal_field_type() {
        let program = Program::new(vec![
            Statement::StructDef {
                name: "User".to_string(),
                fields: vec![
                    ("name".to_string(), "string".to_string()),
                    ("age".to_string(), "int".to_string()),
                ],
            },
            Statement::Let {
                name: "user".to_string(),
                value: Expression::StructLiteral {
                    type_name: "User".to_string(),
                    fields: vec![
                        ("name".to_string(), Expression::String("Ana".to_string())),
                        ("age".to_string(), Expression::String("twenty".to_string())),
                    ],
                },
            },
        ]);

        let error = BytecodeBuilder::new().build_checked(&program).unwrap_err();

        assert_eq!(
            error.to_string(),
            "field 'age' for struct 'User' expects int, got string"
        );
    }

    #[test]
    fn test_semantic_validates_known_method_arity() {
        let program = Program::new(vec![Statement::Expression(Expression::MethodCall {
            target: Box::new(Expression::Identifier("items".to_string())),
            method: "push".to_string(),
            args: Vec::new(),
        })]);

        let error = BytecodeBuilder::new().build_checked(&program).unwrap_err();

        assert_eq!(
            error.to_string(),
            "method 'push' expects 1 argument(s), got 0"
        );
    }

    #[test]
    fn test_semantic_validates_function_arity() {
        let program = Program::new(vec![
            Statement::FunctionDef {
                name: "add".to_string(),
                params: vec!["a".to_string(), "b".to_string()],
                body: vec![Statement::Return(Expression::Binary {
                    left: Box::new(Expression::Identifier("a".to_string())),
                    op: BinaryOp::Add,
                    right: Box::new(Expression::Identifier("b".to_string())),
                })],
            },
            Statement::Expression(Expression::Call {
                callee: Box::new(Expression::Identifier("add".to_string())),
                args: vec![Expression::Int(1)],
            }),
        ]);

        let error = BytecodeBuilder::new().build_checked(&program).unwrap_err();

        assert_eq!(
            error.to_string(),
            "function 'add' expects 2 argument(s), got 1"
        );
    }

    #[test]
    fn test_semantic_rejects_unknown_function_call() {
        let program = Program::new(vec![Statement::Expression(Expression::Call {
            callee: Box::new(Expression::Identifier("missing".to_string())),
            args: Vec::new(),
        })]);

        let error = BytecodeBuilder::new().build_checked(&program).unwrap_err();

        assert_eq!(error.to_string(), "unknown function 'missing'");
    }

    #[test]
    fn test_semantic_rejects_undefined_variable() {
        let program = Program::new(vec![Statement::Print(Expression::Identifier(
            "missing".to_string(),
        ))]);

        let error = BytecodeBuilder::new().build_checked(&program).unwrap_err();

        assert_eq!(error.to_string(), "undefined variable 'missing'");
    }

    #[test]
    fn test_semantic_rejects_set_before_declaration() {
        let program = Program::new(vec![Statement::Set {
            name: "missing".to_string(),
            value: Expression::Int(1),
        }]);

        let error = BytecodeBuilder::new().build_checked(&program).unwrap_err();

        assert_eq!(
            error.to_string(),
            "cannot set undefined variable 'missing'"
        );
    }

    #[test]
    fn test_compile_for_loop() {
        let program = Program::new(vec![Statement::For {
            item: "item".to_string(),
            iterable: Expression::List(vec![Expression::Int(1), Expression::Int(2)]),
            body: vec![Statement::Print(Expression::Identifier("item".to_string()))],
        }]);

        let bytecode = BytecodeBuilder::new().build_checked(&program).unwrap();

        assert!(bytecode
            .main_instructions
            .iter()
            .any(|instr| matches!(instr, Instruction::ListLen)));
        assert!(bytecode
            .main_instructions
            .iter()
            .any(|instr| matches!(instr, Instruction::LoadIndex)));
    }
}
