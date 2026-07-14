//! Bytecode generation and serialization for Matter
//! Formato MBC1 (Matter Bytecode v1)

use matter_ast::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufWriter, Result};
use std::path::Path;

mod deserialize;
mod effect_check;
mod serialize;
mod validate;

pub use effect_check::*;
pub use serialize::*;
pub use validate::BytecodeLimits;

/// Instruções da VM Matter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    // Stack operations
    LoadConst(ConstantId),
    LoadGlobal(String),
    StoreGlobal(String),
    LoadLocal(String),
    StoreLocal(String),
    LoadParam(usize),
    StoreExisting(String),

    // Scope management
    PushScope,
    PopScope,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,

    // Logical
    And,
    Or,
    Not,

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
    CallNamed {
        name: String,
        arg_count: usize,
    },
    MakeClosure {
        func_name: String,
        capture_names: Vec<String>,
    },
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
    NewList(usize),           // Create list with N elements from stack
    LoadIndex,                // Pop index, pop collection, push value
    StoreIndex,               // Pop value, pop index, pop collection
    StoreIndexVar(String),    // Pop value, pop index, mutate variable list[index]
    ListPush,                 // Pop value, pop list, push list (mutated)
    ListPop,                  // Pop list, push value, push list (mutated)
    ListLen,                  // Pop list, push length
    ListPushVar(String),      // Pop value, mutate variable list, push unit
    ListPopVar(String),       // Mutate variable list, push popped value
    NewMap(usize),            // Create map with N key/value pairs from stack
    MapHas,                   // Pop key, pop map, push bool
    MapKeys,                  // Pop map, push list of keys
    MapValues,                // Pop map, push list of values
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Constant {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub param_count: usize,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandler {
    pub event: String,
    pub instructions: Vec<Instruction>,
}

/// Bytecode completo de um programa Matter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bytecode {
    #[serde(with = "serde_bytes_array")]
    pub magic: [u8; 4], // "MBC1"
    pub constants: Vec<Constant>,
    pub functions: HashMap<String, Function>,
    pub event_handlers: HashMap<String, EventHandler>,
    pub main_instructions: Vec<Instruction>,
}

mod serde_bytes_array {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &[u8; 4], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 4], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        if bytes.len() == 4 {
            Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
        } else {
            Err(serde::de::Error::custom("Expected 4 bytes for magic"))
        }
    }
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

    /// Load bytecode from file, then run full structural validation.
    /// Never returns bytecode that failed validation (do not execute unvalidated MBC1).
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let meta = std::fs::metadata(path_ref)?;
        let limits = BytecodeLimits::from_env();
        let len = meta.len() as usize;
        if len > limits.max_file_bytes {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "MBC1 file too large: {} bytes (limit {})",
                    len, limits.max_file_bytes
                ),
            ));
        }
        let file = File::open(path_ref)?;
        let mut reader = BufReader::new(file);
        let bytecode = Self::deserialize(&mut reader)?;
        bytecode.validate_with_limits(&limits)?;
        Ok(bytecode)
    }

    /// Deserialize from an in-memory buffer and validate (preferred for untrusted input).
    pub fn load_from_bytes(data: &[u8]) -> Result<Self> {
        let limits = BytecodeLimits::from_env();
        if data.len() > limits.max_file_bytes {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "MBC1 buffer too large: {} bytes (limit {})",
                    data.len(),
                    limits.max_file_bytes
                ),
            ));
        }
        let mut cursor = std::io::Cursor::new(data);
        let bytecode = Self::deserialize(&mut cursor)?;
        bytecode.validate_with_limits(&limits)?;
        Ok(bytecode)
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
        self.scopes.iter().rev().any(|scope| scope.contains(name))
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
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                validate_unique_names(&param_names, "function parameter")?;
                functions.insert(name.clone(), params.len());
            }
            Statement::OnEvent { event, .. } if !events.insert(event.clone()) => {
                return Err(SemanticError::new(format!(
                    "duplicate event handler '{}'",
                    event
                )));
            }
            Statement::OnEvent { .. } => {}
            Statement::StructDef { name, fields } => {
                if structs.contains_key(name) {
                    return Err(SemanticError::new(format!(
                        "duplicate struct definition '{}'",
                        name
                    )));
                }

                let field_names: Vec<String> =
                    fields.iter().map(|(field, _)| field.clone()).collect();
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
                ambient_context.scopes = vec![params.iter().map(|p| p.name.clone()).collect()];
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
        Statement::Let { name, value, .. } => {
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
        Statement::SetIndex {
            target,
            index,
            value,
        } => {
            validate_expression(target, structs, functions, context)?;
            validate_expression(index, structs, functions, context)?;
            validate_expression(value, structs, functions, context)?;
        }
        Statement::FunctionDef { .. } => {}
        Statement::StructDef { .. } => {}
        Statement::Import { .. } => {}
        Statement::ImportFrom { .. } => {}
        Statement::ImportAs { .. } => {}
        Statement::Export { .. } => {}
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
        Statement::Match { subject, arms } => {
            validate_expression(subject, structs, functions, context)?;
            for arm in arms {
                validate_expression(&arm.pattern, structs, functions, context)?;
                context.push_scope();
                validate_block(&arm.body, context, structs, functions)?;
                context.pop_scope();
            }
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
        | Expression::Float(_)
        | Expression::Bool(_)
        | Expression::String(_)
        | Expression::Null
        | Expression::Unit => {}
        Expression::Identifier(name) => {
            if !context.contains_variable(name, functions) {
                return Err(SemanticError::new(format!("undefined variable '{}'", name)));
            }
        }
        Expression::Binary { left, right, .. } => {
            validate_expression(left, structs, functions, context)?;
            validate_expression(right, structs, functions, context)?;
        }
        Expression::Unary { operand, .. } => {
            validate_expression(operand, structs, functions, context)?;
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
                &entries
                    .iter()
                    .map(|(key, _)| key.clone())
                    .collect::<Vec<_>>(),
                "map key",
            )?;
            for (_, value) in entries {
                validate_expression(value, structs, functions, context)?;
            }
        }
        Expression::StructLiteral { type_name, fields } => {
            let declared_fields = structs
                .get(type_name)
                .ok_or_else(|| SemanticError::new(format!("unknown struct '{}'", type_name)))?;
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

            for (field, _field_type) in declared_fields {
                if !field_names.contains(field) {
                    return Err(SemanticError::new(format!(
                        "missing field '{}' for struct '{}'",
                        field, type_name
                    )));
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
        Expression::MethodCall {
            target,
            method,
            args,
        } => {
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
        Expression::Lambda { .. } => {
            // Lambda body validation happens at compilation time
        }
        Expression::OkExpr(inner)
        | Expression::ErrExpr(inner)
        | Expression::SomeExpr(inner)
        | Expression::TryPropagate(inner) => {
            validate_expression(inner, structs, functions, context)?;
        }
        Expression::NoneExpr => {}
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

    matches!(target, Expression::Identifier(_))
        && !matches!(method, "push" | "pop" | "len" | "has" | "keys" | "values")
}

fn is_builtin_backend(name: &str) -> bool {
    matches!(
        name,
        "agent"
            | "visual"
            | "store"
            | "net"
            | "math"
            | "string"
            | "list"
            | "energy"
            | "tool"
            | "python"
            | "node"
            | "java"
            | "go"
            | "rust"
            | "polyglot"
            | "map"
            | "type"
            | "console"
            | "file"
            | "time"
            | "random"
            | "json"
            | "graph"
    )
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
        // Sprint 27.3: Effect checking
        let mut effect_checker = BytecodeEffectChecker::new();
        effect_checker.check_program(program);

        if effect_checker.has_errors() {
            let error_messages: Vec<String> = effect_checker
                .errors()
                .iter()
                .map(|e| e.message.clone())
                .collect();
            return Err(SemanticError::new(format!(
                "Effect checking failed:\n{}",
                error_messages.join("\n")
            )));
        }

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
            Statement::Let { name, value, .. } => {
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

            Statement::SetIndex {
                target,
                index,
                value,
            } => {
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

            Statement::SetField {
                target,
                field,
                value,
            } => {
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

            Statement::FunctionDef {
                name,
                params,
                body,
                effects: _,
                ..
            } => {
                let mut func_instructions = Vec::new();
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();

                // Copy parameters to local variables at function start
                for (idx, param) in params.iter().enumerate() {
                    func_instructions.push(Instruction::LoadParam(idx));
                    func_instructions.push(Instruction::StoreLocal(param.name.clone()));
                }

                // Compilar corpo da função
                for stmt in body {
                    self.compile_function_statement(stmt, &mut func_instructions, &param_names);
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

            Statement::Import { .. } => {
                // Imports are resolved before bytecode execution; keep the compiler tolerant
                // while module loading is still being wired through the runtime.
            }

            Statement::ImportFrom { .. } | Statement::ImportAs { .. } => {
                // Extended imports are resolved before bytecode execution.
            }

            Statement::Export { .. } => {
                // Exports are resolved at module boundary; no bytecode needed.
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

            Statement::Match { subject, arms } => {
                let match_temp = self.next_temp_name("match_subj");
                self.compile_expression(subject, instructions);
                if local_declarations {
                    instructions.push(Instruction::StoreLocal(match_temp.clone()));
                } else {
                    instructions.push(Instruction::StoreGlobal(match_temp.clone()));
                }

                let mut jump_end_positions = Vec::new();

                for arm in arms {
                    if local_declarations {
                        instructions.push(Instruction::LoadLocal(match_temp.clone()));
                    } else {
                        instructions.push(Instruction::LoadGlobal(match_temp.clone()));
                    }
                    self.compile_expression(&arm.pattern, instructions);
                    instructions.push(Instruction::Eq);

                    let jump_if_false_pos = instructions.len();
                    instructions.push(Instruction::JumpIfFalse(0));

                    instructions.push(Instruction::PushScope);
                    for stmt in &arm.body {
                        self.compile_statement_with_scope(stmt, instructions, true);
                    }
                    instructions.push(Instruction::PopScope);

                    jump_end_positions.push(instructions.len());
                    instructions.push(Instruction::Jump(0));

                    let next_arm_pos = instructions.len();
                    instructions[jump_if_false_pos] = Instruction::JumpIfFalse(next_arm_pos);
                }

                let end_pos = instructions.len();
                for pos in jump_end_positions {
                    instructions[pos] = Instruction::Jump(end_pos);
                }
            }

            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
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

    fn compile_function_statement(
        &mut self,
        stmt: &Statement,
        instructions: &mut Vec<Instruction>,
        params: &[String],
    ) {
        match stmt {
            Statement::Let { name, value, .. } => {
                self.compile_function_expression(value, instructions, params);
                instructions.push(Instruction::StoreLocal(name.clone()));
            }

            Statement::Set { name, value } => {
                self.compile_function_expression(value, instructions, params);
                instructions.push(Instruction::StoreExisting(name.clone()));
            }

            Statement::SetIndex {
                target,
                index,
                value,
            } => {
                if let Expression::Identifier(name) = target {
                    self.compile_function_expression(index, instructions, params);
                    self.compile_function_expression(value, instructions, params);
                    instructions.push(Instruction::StoreIndexVar(name.clone()));
                } else {
                    self.compile_function_expression(target, instructions, params);
                    self.compile_function_expression(index, instructions, params);
                    self.compile_function_expression(value, instructions, params);
                    instructions.push(Instruction::StoreIndex);
                }
            }

            Statement::SetField {
                target,
                field,
                value,
            } => {
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

            Statement::Match { subject, arms } => {
                let match_temp = self.next_temp_name("match_subj");
                self.compile_function_expression(subject, instructions, params);
                instructions.push(Instruction::StoreLocal(match_temp.clone()));

                let mut jump_end_positions = Vec::new();

                for arm in arms {
                    instructions.push(Instruction::LoadLocal(match_temp.clone()));
                    self.compile_function_expression(&arm.pattern, instructions, params);
                    instructions.push(Instruction::Eq);

                    let jump_if_false_pos = instructions.len();
                    instructions.push(Instruction::JumpIfFalse(0));

                    instructions.push(Instruction::PushScope);
                    for stmt in &arm.body {
                        self.compile_function_statement(stmt, instructions, params);
                    }
                    instructions.push(Instruction::PopScope);

                    jump_end_positions.push(instructions.len());
                    instructions.push(Instruction::Jump(0));

                    let next_arm_pos = instructions.len();
                    instructions[jump_if_false_pos] = Instruction::JumpIfFalse(next_arm_pos);
                }

                let end_pos = instructions.len();
                for pos in jump_end_positions {
                    instructions[pos] = Instruction::Jump(end_pos);
                }
            }

            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
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

    fn compile_function_expression(
        &mut self,
        expr: &Expression,
        instructions: &mut Vec<Instruction>,
        params: &[String],
    ) {
        match expr {
            Expression::Identifier(name) => {
                // Tentar local, depois global. Parâmetros copiados para variáveis locais
                // no início da função são resolvidos como variáveis locais.
                instructions.push(Instruction::LoadGlobal(name.clone()));
            }

            Expression::Binary { left, op, right } => {
                self.compile_function_expression(left, instructions, params);
                self.compile_function_expression(right, instructions, params);

                let instr = match op {
                    BinaryOp::Add => Instruction::Add,
                    BinaryOp::Sub => Instruction::Sub,
                    BinaryOp::Mul => Instruction::Mul,
                    BinaryOp::Div => Instruction::Div,
                    BinaryOp::Mod => Instruction::Mod,
                    BinaryOp::Eq => Instruction::Eq,
                    BinaryOp::NotEq => Instruction::NotEq,
                    BinaryOp::Lt => Instruction::Lt,
                    BinaryOp::Gt => Instruction::Gt,
                    BinaryOp::LtEq => Instruction::LtEq,
                    BinaryOp::GtEq => Instruction::GtEq,
                    BinaryOp::And => Instruction::And,
                    BinaryOp::Or => Instruction::Or,
                };

                instructions.push(instr);
            }

            Expression::Unary { op, operand } => {
                self.compile_function_expression(operand, instructions, params);
                let instr = match op {
                    UnaryOp::Not => Instruction::Not,
                    UnaryOp::Neg => Instruction::Neg,
                };
                instructions.push(instr);
            }

            Expression::Call { callee, args } => {
                for arg in args {
                    self.compile_function_expression(arg, instructions, params);
                }

                if let Expression::Identifier(name) = callee.as_ref() {
                    instructions.push(Instruction::CallNamed {
                        name: name.clone(),
                        arg_count: args.len(),
                    });
                } else {
                    self.compile_function_expression(callee, instructions, params);
                    instructions.push(Instruction::Call(args.len()));
                }
            }

            Expression::List(elements) => {
                for element in elements {
                    self.compile_function_expression(element, instructions, params);
                }
                instructions.push(Instruction::NewList(elements.len()));
            }
            Expression::Map(entries) => {
                for (key, value) in entries {
                    let id = self.bytecode.add_constant(Constant::String(key.clone()));
                    instructions.push(Instruction::LoadConst(id));
                    self.compile_function_expression(value, instructions, params);
                }
                instructions.push(Instruction::NewMap(entries.len()));
            }
            Expression::Index { target, index } => {
                self.compile_function_expression(target, instructions, params);
                self.compile_function_expression(index, instructions, params);
                instructions.push(Instruction::LoadIndex);
            }
            Expression::BackendCall {
                backend,
                method,
                args,
            } => {
                for arg in args {
                    self.compile_function_expression(arg, instructions, params);
                }
                instructions.push(Instruction::BackendCall {
                    backend: backend.clone(),
                    method: method.clone(),
                    arg_count: args.len(),
                });
            }
            Expression::MethodCall {
                target,
                method,
                args,
            } => {
                if is_backend_call(target, method) {
                    for arg in args {
                        self.compile_function_expression(arg, instructions, params);
                    }
                    if let Expression::Identifier(name) = target.as_ref() {
                        instructions.push(Instruction::BackendCall {
                            backend: name.clone(),
                            method: method.clone(),
                            arg_count: args.len(),
                        });
                    }
                } else {
                    if let Expression::Identifier(name) = target.as_ref() {
                        if method == "push" && args.len() == 1 {
                            self.compile_function_expression(&args[0], instructions, params);
                            instructions.push(Instruction::ListPushVar(name.clone()));
                            return;
                        } else if method == "pop" && args.is_empty() {
                            instructions.push(Instruction::ListPopVar(name.clone()));
                            return;
                        }
                    }
                    self.compile_function_expression(target, instructions, params);
                    for arg in args {
                        self.compile_function_expression(arg, instructions, params);
                    }
                    let instr = match method.as_str() {
                        "push" => Instruction::ListPush,
                        "pop" => Instruction::ListPop,
                        "len" => Instruction::ListLen,
                        "has" => Instruction::MapHas,
                        "keys" => Instruction::MapKeys,
                        "values" => Instruction::MapValues,
                        _ => unreachable!(),
                    };
                    instructions.push(instr);
                }
            }
            Expression::Field { target, field } => {
                self.compile_function_expression(target, instructions, params);
                instructions.push(Instruction::LoadField(field.clone()));
            }
            Expression::StructLiteral { type_name, fields } => {
                for (name, value) in fields {
                    let id = self.bytecode.add_constant(Constant::String(name.clone()));
                    instructions.push(Instruction::LoadConst(id));
                    self.compile_function_expression(value, instructions, params);
                }
                instructions.push(Instruction::NewStruct(type_name.clone(), fields.len()));
            }
            _ => {
                // Primitives fall back to standard compile
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

            Expression::Float(f) => {
                let id = self.bytecode.add_constant(Constant::Float(*f));
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

            Expression::Null => {
                let id = self.bytecode.add_constant(Constant::Null);
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
                    BinaryOp::Mod => Instruction::Mod,
                    BinaryOp::Eq => Instruction::Eq,
                    BinaryOp::NotEq => Instruction::NotEq,
                    BinaryOp::Lt => Instruction::Lt,
                    BinaryOp::Gt => Instruction::Gt,
                    BinaryOp::LtEq => Instruction::LtEq,
                    BinaryOp::GtEq => Instruction::GtEq,
                    BinaryOp::And => Instruction::And,
                    BinaryOp::Or => Instruction::Or,
                };

                instructions.push(instr);
            }

            Expression::Unary { op, operand } => {
                self.compile_expression(operand, instructions);
                let instr = match op {
                    UnaryOp::Not => Instruction::Not,
                    UnaryOp::Neg => Instruction::Neg,
                };
                instructions.push(instr);
            }

            Expression::Call { callee, args } => {
                // Push arguments onto stack
                for arg in args {
                    self.compile_expression(arg, instructions);
                }

                if let Expression::Identifier(name) = callee.as_ref() {
                    instructions.push(Instruction::CallNamed {
                        name: name.clone(),
                        arg_count: args.len(),
                    });
                } else {
                    // Push function reference
                    self.compile_expression(callee, instructions);
                    instructions.push(Instruction::Call(args.len()));
                }
            }

            Expression::BackendCall {
                backend,
                method,
                args,
            } => {
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

            Expression::List(elements) => {
                for element in elements {
                    self.compile_expression(element, instructions);
                }
                instructions.push(Instruction::NewList(elements.len()));
            }

            Expression::Map(entries) => {
                for (key, value) in entries {
                    let id = self.bytecode.add_constant(Constant::String(key.clone()));
                    instructions.push(Instruction::LoadConst(id));
                    self.compile_expression(value, instructions);
                }
                instructions.push(Instruction::NewMap(entries.len()));
            }

            Expression::StructLiteral { type_name, fields } => {
                for (name, value) in fields {
                    let id = self.bytecode.add_constant(Constant::String(name.clone()));
                    instructions.push(Instruction::LoadConst(id));
                    self.compile_expression(value, instructions);
                }
                instructions.push(Instruction::NewStruct(type_name.clone(), fields.len()));
            }

            Expression::Index { target, index } => {
                self.compile_expression(target, instructions);
                self.compile_expression(index, instructions);
                instructions.push(Instruction::LoadIndex);
            }

            Expression::Field { target, field } => {
                self.compile_expression(target, instructions);
                instructions.push(Instruction::LoadField(field.clone()));
            }

            Expression::MethodCall {
                target,
                method,
                args,
            } => {
                if is_backend_call(target, method) {
                    let backend = match target.as_ref() {
                        Expression::Identifier(name) => name.clone(),
                        _ => String::new(),
                    };

                    for arg in args {
                        self.compile_expression(arg, instructions);
                    }

                    instructions.push(Instruction::BackendCall {
                        backend,
                        method: method.clone(),
                        arg_count: args.len(),
                    });
                } else {
                    if let Expression::Identifier(name) = target.as_ref() {
                        if method == "push" && args.len() == 1 {
                            self.compile_expression(&args[0], instructions);
                            instructions.push(Instruction::ListPushVar(name.clone()));
                            return;
                        } else if method == "pop" && args.is_empty() {
                            instructions.push(Instruction::ListPopVar(name.clone()));
                            return;
                        }
                    }

                    // Method calls: push list/map, push args, call
                    self.compile_expression(target, instructions);
                    for arg in args {
                        self.compile_expression(arg, instructions);
                    }

                    let instr = match method.as_str() {
                        "push" => Instruction::ListPush,
                        "pop" => Instruction::ListPop,
                        "len" => Instruction::ListLen,
                        "has" => Instruction::MapHas,
                        "keys" => Instruction::MapKeys,
                        "values" => Instruction::MapValues,
                        _ => {
                            // Default to regular call if not a known method
                            let id = self.bytecode.add_constant(Constant::String(method.clone()));
                            instructions.push(Instruction::LoadConst(id));
                            Instruction::Call(args.len() + 1) // +1 for target
                        }
                    };

                    instructions.push(instr);
                }
            }

            Expression::OkExpr(inner) => {
                self.compile_expression(inner, instructions);
                instructions.push(Instruction::BackendCall {
                    backend: "result".to_string(),
                    method: "ok".to_string(),
                    arg_count: 1,
                });
            }

            Expression::ErrExpr(inner) => {
                self.compile_expression(inner, instructions);
                instructions.push(Instruction::BackendCall {
                    backend: "result".to_string(),
                    method: "err".to_string(),
                    arg_count: 1,
                });
            }

            Expression::SomeExpr(inner) => {
                self.compile_expression(inner, instructions);
                instructions.push(Instruction::BackendCall {
                    backend: "option".to_string(),
                    method: "some".to_string(),
                    arg_count: 1,
                });
            }

            Expression::NoneExpr => {
                instructions.push(Instruction::BackendCall {
                    backend: "option".to_string(),
                    method: "none".to_string(),
                    arg_count: 0,
                });
            }

            Expression::TryPropagate(inner) => {
                self.compile_expression(inner, instructions);
                instructions.push(Instruction::BackendCall {
                    backend: "result".to_string(),
                    method: "try_unwrap".to_string(),
                    arg_count: 1,
                });
            }

            Expression::Lambda { params, body } => {
                // Generate unique name for the lambda
                let lambda_id = self.bytecode.functions.len();
                let func_name = format!("__lambda_{}", lambda_id);

                // Collect free variables (captures) from the body
                let mut captures = Vec::new();
                let param_names: std::collections::HashSet<String> =
                    params.iter().map(|p| p.name.clone()).collect();
                for stmt in body {
                    collect_free_variables(stmt, &param_names, &mut captures);
                }
                captures.sort();
                captures.dedup();

                // Compile the function body
                let mut func_instructions = Vec::new();
                for stmt in body {
                    self.compile_statement_with_scope(stmt, &mut func_instructions, false);
                }
                func_instructions.push(Instruction::LoadConst(
                    self.bytecode.add_constant(Constant::Unit),
                ));
                func_instructions.push(Instruction::Return);

                self.bytecode.functions.insert(func_name.clone(), Function {
                    name: func_name.clone(),
                    param_count: params.len(),
                    instructions: func_instructions,
                });

                // Emit MakeClosure instruction
                instructions.push(Instruction::MakeClosure {
                    func_name,
                    capture_names: captures,
                });
            }
        }
    }
}

impl Default for BytecodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect free variables referenced in a statement (for closure capture)
fn collect_free_variables(
    stmt: &Statement,
    param_names: &std::collections::HashSet<String>,
    captures: &mut Vec<String>,
) {
    match stmt {
        Statement::Let { value, .. } => collect_free_variables_expr(value, param_names, captures),
        Statement::Set { value, .. } => collect_free_variables_expr(value, param_names, captures),
        Statement::Print(expr) => collect_free_variables_expr(expr, param_names, captures),
        Statement::Return(expr) => collect_free_variables_expr(expr, param_names, captures),
        Statement::Expression(expr) => collect_free_variables_expr(expr, param_names, captures),
        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            collect_free_variables_expr(condition, param_names, captures);
            for s in then_body {
                collect_free_variables(s, param_names, captures);
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    collect_free_variables(s, param_names, captures);
                }
            }
        }
        Statement::While { condition, body } => {
            collect_free_variables_expr(condition, param_names, captures);
            for s in body {
                collect_free_variables(s, param_names, captures);
            }
        }
        Statement::For { iterable, body, .. } => {
            collect_free_variables_expr(iterable, param_names, captures);
            for s in body {
                collect_free_variables(s, param_names, captures);
            }
        }
        Statement::Loop { body } => {
            for s in body {
                collect_free_variables(s, param_names, captures);
            }
        }
        _ => {}
    }
}

fn collect_free_variables_expr(
    expr: &Expression,
    param_names: &std::collections::HashSet<String>,
    captures: &mut Vec<String>,
) {
    match expr {
        Expression::Identifier(name) => {
            if !param_names.contains(name) && !captures.contains(name) {
                captures.push(name.clone());
            }
        }
        Expression::Binary { left, right, .. } => {
            collect_free_variables_expr(left, param_names, captures);
            collect_free_variables_expr(right, param_names, captures);
        }
        Expression::Unary { operand, .. } => {
            collect_free_variables_expr(operand, param_names, captures);
        }
        Expression::Call { callee, args } => {
            collect_free_variables_expr(callee, param_names, captures);
            for arg in args {
                collect_free_variables_expr(arg, param_names, captures);
            }
        }
        Expression::List(elements) => {
            for e in elements {
                collect_free_variables_expr(e, param_names, captures);
            }
        }
        Expression::Map(entries) => {
            for (_, v) in entries {
                collect_free_variables_expr(v, param_names, captures);
            }
        }
        Expression::Index { target, index } => {
            collect_free_variables_expr(target, param_names, captures);
            collect_free_variables_expr(index, param_names, captures);
        }
        Expression::Field { target, .. } => {
            collect_free_variables_expr(target, param_names, captures);
        }
        Expression::MethodCall { target, args, .. } => {
            collect_free_variables_expr(target, param_names, captures);
            for arg in args {
                collect_free_variables_expr(arg, param_names, captures);
            }
        }
        _ => {}
    }
}
