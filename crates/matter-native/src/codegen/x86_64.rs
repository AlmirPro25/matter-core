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
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PeImport {
    ExitProcess,
    GetStdHandle,
    WriteFile,
    GetProcessHeap,
    HeapAlloc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeValueKind {
    Int,
    String,
    ListInt,
    ListString,
}

const RUNTIME_PTR_STACK_OFFSET: i32 = -40;

#[derive(Debug, Clone)]
struct ValueShape {
    kind: NativeValueKind,
    string_value: Option<String>,
    fields: HashMap<String, NativeValueKind>,
    field_shapes: HashMap<String, ValueShape>,
    key_kinds: Vec<NativeValueKind>,
    value_kinds: Vec<NativeValueKind>,
}

impl ValueShape {
    fn new(kind: NativeValueKind) -> Self {
        Self {
            kind,
            string_value: None,
            fields: HashMap::new(),
            field_shapes: HashMap::new(),
            key_kinds: Vec::new(),
            value_kinds: Vec::new(),
        }
    }

    fn string(value: String) -> Self {
        Self {
            kind: NativeValueKind::String,
            string_value: Some(value),
            fields: HashMap::new(),
            field_shapes: HashMap::new(),
            key_kinds: vec![NativeValueKind::String],
            value_kinds: Vec::new(),
        }
    }
}

/// x86-64 code generator
pub struct X86CodeGen {
    /// Generated machine code
    code: Vec<u8>,

    /// Data section (strings, constants)
    data: Vec<u8>,

    /// Variable stack offsets (name -> rbp-relative offset)
    variables: HashMap<String, i32>,

    /// Best-effort compile-time value kinds for locals.
    variable_kinds: HashMap<String, NativeValueKind>,

    /// Best-effort compile-time value kinds for generated stack values.
    value_stack: Vec<NativeValueKind>,

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

    /// Pending RIP-relative function address patches (code offset -> function name)
    pending_function_address_patches: Vec<(usize, String)>,

    /// Function addresses (name -> code offset)
    function_addresses: HashMap<String, usize>,

    /// Best-effort return kinds inferred before lowering function callsites.
    function_return_kinds: HashMap<String, NativeValueKind>,

    /// Best-effort return shapes inferred before lowering function callsites.
    function_return_shapes: HashMap<String, ValueShape>,

    /// Best-effort parameter kinds inferred from named callsites.
    function_param_kinds: HashMap<String, Vec<NativeValueKind>>,

    /// Function currently being lowered, used to refine return kinds.
    current_function_name: Option<String>,

    /// Treat unresolved globals as frame locals for standalone executables.
    standalone_executable: bool,

    /// Standalone print callsites that target the emitted native stdout helper.
    pending_standalone_print_calls: Vec<usize>,

    /// Standalone print string callsites that target the emitted native stdout helper.
    pending_standalone_print_string_calls: Vec<usize>,

    /// Standalone string equality callsites that target the emitted native helper.
    pending_standalone_string_eq_calls: Vec<usize>,

    /// RIP-relative calls to PE imports that need final .idata RVAs.
    pending_pe_import_calls: Vec<(usize, PeImport)>,

    /// Interned standalone string data offsets, used for stable string identity in native maps.
    standalone_string_offsets: HashMap<String, usize>,

    /// Predicted result kinds for dynamic standalone index reads.
    predicted_load_index_kinds: VecDeque<NativeValueKind>,

    /// Predicted result kinds for dynamic standalone field reads.
    predicted_load_field_kinds: VecDeque<NativeValueKind>,

    /// Predicted list kinds for standalone map key/value materialization.
    predicted_map_view_kinds: VecDeque<NativeValueKind>,
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

    fn has_empty_top_level(instructions: &[Instruction]) -> bool {
        instructions.is_empty() || matches!(instructions, [Instruction::Halt])
    }

    fn constant_kind(constants: &[Constant], id: usize) -> NativeValueKind {
        match constants.get(id) {
            Some(Constant::String(_)) => NativeValueKind::String,
            _ => NativeValueKind::Int,
        }
    }

    fn merge_value_kind(target: &mut NativeValueKind, candidate: NativeValueKind) {
        if *target == NativeValueKind::Int && candidate != NativeValueKind::Int {
            *target = candidate;
        } else if candidate == NativeValueKind::String {
            *target = NativeValueKind::String;
        }
    }

    fn list_kind_from_elements(elements: &[NativeValueKind]) -> NativeValueKind {
        if !elements.is_empty() && elements.iter().all(|kind| *kind == NativeValueKind::String) {
            NativeValueKind::ListString
        } else {
            NativeValueKind::ListInt
        }
    }

    fn list_element_kind(kind: NativeValueKind) -> NativeValueKind {
        match kind {
            NativeValueKind::ListString => NativeValueKind::String,
            _ => NativeValueKind::Int,
        }
    }

    fn list_kind_after_push(
        list_kind: NativeValueKind,
        value_kind: NativeValueKind,
    ) -> NativeValueKind {
        if list_kind == NativeValueKind::ListString || value_kind == NativeValueKind::String {
            NativeValueKind::ListString
        } else {
            NativeValueKind::ListInt
        }
    }

    fn function_return_shape(&self, name: &str) -> ValueShape {
        self.function_return_shapes
            .get(name)
            .cloned()
            .unwrap_or_else(|| {
                ValueShape::new(
                    self.function_return_kinds
                        .get(name)
                        .copied()
                        .unwrap_or(NativeValueKind::Int),
                )
            })
    }

    fn infer_dynamic_load_kinds_for_instructions(
        &self,
        instructions: &[Instruction],
        constants: &[Constant],
        current_params: &[NativeValueKind],
    ) -> (
        Vec<NativeValueKind>,
        Vec<NativeValueKind>,
        Vec<NativeValueKind>,
    ) {
        let mut stack: Vec<ValueShape> = Vec::new();
        let mut locals: HashMap<String, ValueShape> = HashMap::new();
        let mut load_index_kinds = Vec::new();
        let mut load_field_kinds = Vec::new();
        let mut map_view_kinds = Vec::new();

        for instruction in instructions {
            match instruction {
                Instruction::LoadConst(id) => match constants.get(*id) {
                    Some(Constant::String(value)) => stack.push(ValueShape::string(value.clone())),
                    _ => stack.push(ValueShape::new(Self::constant_kind(constants, *id))),
                },
                Instruction::LoadLocal(name) | Instruction::LoadGlobal(name) => {
                    stack.push(
                        locals
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int)),
                    );
                }
                Instruction::LoadParam(index) => {
                    stack.push(ValueShape::new(
                        current_params
                            .get(*index)
                            .copied()
                            .unwrap_or(NativeValueKind::Int),
                    ));
                }
                Instruction::StoreLocal(name)
                | Instruction::StoreGlobal(name)
                | Instruction::StoreExisting(name) => {
                    let shape = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    locals.insert(name.clone(), shape);
                }
                Instruction::NewList(count) => {
                    let mut elements = Vec::new();
                    for _ in 0..*count {
                        elements.push(
                            stack
                                .pop()
                                .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int))
                                .kind,
                        );
                    }
                    stack.push(ValueShape::new(Self::list_kind_from_elements(&elements)));
                }
                Instruction::NewMap(count) | Instruction::NewStruct(_, count) => {
                    let mut fields = HashMap::new();
                    let mut field_shapes = HashMap::new();
                    let mut key_kinds = Vec::new();
                    let mut value_kinds = Vec::new();
                    for _ in 0..*count {
                        let value = stack
                            .pop()
                            .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                        let key = stack
                            .pop()
                            .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                        key_kinds.push(key.kind);
                        value_kinds.push(value.kind);
                        if let Some(key) = key.string_value {
                            fields.insert(key.clone(), value.kind);
                            field_shapes.insert(key, value);
                        }
                    }
                    let mut shape = ValueShape::new(NativeValueKind::Int);
                    shape.fields = fields;
                    shape.field_shapes = field_shapes;
                    shape.key_kinds = key_kinds;
                    shape.value_kinds = value_kinds;
                    stack.push(shape);
                }
                Instruction::LoadIndex => {
                    let index = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let collection = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let kind = index
                        .string_value
                        .as_ref()
                        .and_then(|key| collection.fields.get(key))
                        .copied()
                        .unwrap_or_else(|| Self::list_element_kind(collection.kind));
                    load_index_kinds.push(kind);
                    let shape = index
                        .string_value
                        .as_ref()
                        .and_then(|key| collection.field_shapes.get(key))
                        .cloned()
                        .unwrap_or_else(|| ValueShape::new(kind));
                    stack.push(shape);
                }
                Instruction::LoadField(field) => {
                    let target = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let kind = target
                        .fields
                        .get(field)
                        .copied()
                        .unwrap_or(NativeValueKind::Int);
                    load_field_kinds.push(kind);
                    stack.push(
                        target
                            .field_shapes
                            .get(field)
                            .cloned()
                            .unwrap_or_else(|| ValueShape::new(kind)),
                    );
                }
                Instruction::StoreIndexVar(name) => {
                    let value = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let index = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    if let (Some(target), Some(key)) = (locals.get_mut(name), index.string_value) {
                        target.fields.insert(key.clone(), value.kind);
                        target.field_shapes.insert(key, value.clone());
                        target.key_kinds.push(NativeValueKind::String);
                        target.value_kinds.push(value.kind);
                    }
                }
                Instruction::StoreFieldVar { target, field } => {
                    let value = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    if let Some(target) = locals.get_mut(target) {
                        target.fields.insert(field.clone(), value.kind);
                        target.field_shapes.insert(field.clone(), value.clone());
                        target.key_kinds.push(NativeValueKind::String);
                        target.value_kinds.push(value.kind);
                    }
                }
                Instruction::ListPush => {
                    let value = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let list = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let kind = Self::list_kind_after_push(list.kind, value.kind);
                    stack.push(ValueShape::new(kind));
                }
                Instruction::ListPushVar(name) => {
                    let value = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    if let Some(target) = locals.get_mut(name) {
                        target.kind = Self::list_kind_after_push(target.kind, value.kind);
                    }
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::ListPop => {
                    let list = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    stack.push(ValueShape::new(Self::list_element_kind(list.kind)));
                    stack.push(list);
                }
                Instruction::ListPopVar(name) => {
                    let list_kind = locals
                        .get(name)
                        .map(|shape| shape.kind)
                        .unwrap_or(NativeValueKind::Int);
                    stack.push(ValueShape::new(Self::list_element_kind(list_kind)));
                }
                Instruction::ListLen | Instruction::MapHas => {
                    let _ = stack.pop();
                    if matches!(instruction, Instruction::MapHas) {
                        let _ = stack.pop();
                    }
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::MapKeys => {
                    let map = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let kind = Self::list_kind_from_elements(&map.key_kinds);
                    map_view_kinds.push(kind);
                    stack.push(ValueShape::new(kind));
                }
                Instruction::MapValues => {
                    let map = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let kind = Self::list_kind_from_elements(&map.value_kinds);
                    map_view_kinds.push(kind);
                    stack.push(ValueShape::new(kind));
                }
                Instruction::CallNamed { name, arg_count } => {
                    for _ in 0..*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(self.function_return_shape(name));
                }
                Instruction::Call(arg_count) => {
                    for _ in 0..=*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::Print | Instruction::Pop | Instruction::JumpIfFalse(_) => {
                    let _ = stack.pop();
                }
                Instruction::Add
                | Instruction::Sub
                | Instruction::Mul
                | Instruction::Div
                | Instruction::Mod
                | Instruction::And
                | Instruction::Or
                | Instruction::Eq
                | Instruction::NotEq
                | Instruction::Lt
                | Instruction::Gt
                | Instruction::LtEq
                | Instruction::GtEq => {
                    let _ = stack.pop();
                    let _ = stack.pop();
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::Neg | Instruction::Not => {
                    let _ = stack.pop();
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::Return => {
                    let _ = stack.pop();
                }
                Instruction::BackendCall { arg_count, .. } => {
                    for _ in 0..*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::StoreIndex
                | Instruction::PushScope
                | Instruction::PopScope
                | Instruction::Jump(_)
                | Instruction::SpawnEvent(_)
                | Instruction::MakeClosure { .. }
                | Instruction::Halt => {}
            }
        }

        (load_index_kinds, load_field_kinds, map_view_kinds)
    }

    fn infer_function_param_kinds(
        &self,
        bytecode: &Bytecode,
    ) -> HashMap<String, Vec<NativeValueKind>> {
        let mut params: HashMap<String, Vec<NativeValueKind>> = bytecode
            .functions
            .iter()
            .map(|(name, function)| {
                (
                    name.clone(),
                    vec![NativeValueKind::Int; function.param_count],
                )
            })
            .collect();

        for _ in 0..4 {
            self.infer_call_argument_kinds_for_instructions(
                &bytecode.main_instructions,
                &bytecode.constants,
                &[],
                &mut params,
            );

            for (name, function) in &bytecode.functions {
                let local_params = params.get(name).cloned().unwrap_or_default();
                self.infer_call_argument_kinds_for_instructions(
                    &function.instructions,
                    &bytecode.constants,
                    &local_params,
                    &mut params,
                );
            }
        }

        params
    }

    fn infer_call_argument_kinds_for_instructions(
        &self,
        instructions: &[Instruction],
        constants: &[Constant],
        current_params: &[NativeValueKind],
        inferred_params: &mut HashMap<String, Vec<NativeValueKind>>,
    ) {
        let mut stack = Vec::new();
        let mut locals = HashMap::new();

        for instruction in instructions {
            match instruction {
                Instruction::LoadConst(id) => stack.push(Self::constant_kind(constants, *id)),
                Instruction::LoadLocal(name) | Instruction::LoadGlobal(name) => {
                    stack.push(*locals.get(name).unwrap_or(&NativeValueKind::Int));
                }
                Instruction::LoadParam(index) => {
                    stack.push(
                        current_params
                            .get(*index)
                            .copied()
                            .unwrap_or(NativeValueKind::Int),
                    );
                }
                Instruction::StoreLocal(name)
                | Instruction::StoreGlobal(name)
                | Instruction::StoreExisting(name) => {
                    let kind = stack.pop().unwrap_or(NativeValueKind::Int);
                    locals.insert(name.clone(), kind);
                }
                Instruction::CallNamed { name, arg_count } => {
                    let mut args = Vec::new();
                    for _ in 0..*arg_count {
                        args.push(stack.pop().unwrap_or(NativeValueKind::Int));
                    }
                    args.reverse();
                    if let Some(param_kinds) = inferred_params.get_mut(name) {
                        for (index, kind) in args.into_iter().enumerate() {
                            if let Some(param_kind) = param_kinds.get_mut(index) {
                                Self::merge_value_kind(param_kind, kind);
                            }
                        }
                    }
                    stack.push(
                        self.function_return_kinds
                            .get(name)
                            .copied()
                            .unwrap_or(NativeValueKind::Int),
                    );
                }
                Instruction::Call(arg_count) => {
                    for _ in 0..=*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(NativeValueKind::Int);
                }
                Instruction::Print | Instruction::Pop | Instruction::JumpIfFalse(_) => {
                    let _ = stack.pop();
                }
                Instruction::Add
                | Instruction::Sub
                | Instruction::Mul
                | Instruction::Div
                | Instruction::Mod
                | Instruction::And
                | Instruction::Or
                | Instruction::Eq
                | Instruction::NotEq
                | Instruction::Lt
                | Instruction::Gt
                | Instruction::LtEq
                | Instruction::GtEq => {
                    let _ = stack.pop();
                    let _ = stack.pop();
                    stack.push(NativeValueKind::Int);
                }
                Instruction::Neg | Instruction::Not => {
                    let _ = stack.pop();
                    stack.push(NativeValueKind::Int);
                }
                Instruction::Return => {
                    let _ = stack.pop();
                }
                Instruction::NewList(count) => {
                    let mut elements = Vec::new();
                    for _ in 0..*count {
                        elements.push(stack.pop().unwrap_or(NativeValueKind::Int));
                    }
                    stack.push(Self::list_kind_from_elements(&elements));
                }
                Instruction::LoadIndex => {
                    let _ = stack.pop();
                    let list_kind = stack.pop().unwrap_or(NativeValueKind::Int);
                    stack.push(Self::list_element_kind(list_kind));
                }
                Instruction::ListPop => {
                    let list_kind = stack.pop().unwrap_or(NativeValueKind::Int);
                    stack.push(Self::list_element_kind(list_kind));
                    stack.push(list_kind);
                }
                Instruction::ListLen
                | Instruction::NewMap(_)
                | Instruction::MapHas
                | Instruction::MapKeys
                | Instruction::MapValues
                | Instruction::NewStruct(_, _)
                | Instruction::LoadField(_) => stack.push(NativeValueKind::Int),
                Instruction::ListPopVar(name) => {
                    let list_kind = locals.get(name).copied().unwrap_or(NativeValueKind::Int);
                    stack.push(Self::list_element_kind(list_kind));
                }
                Instruction::BackendCall { arg_count, .. } => {
                    for _ in 0..*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(NativeValueKind::Int);
                }
                Instruction::StoreIndex
                | Instruction::StoreIndexVar(_)
                | Instruction::ListPush
                | Instruction::ListPushVar(_)
                | Instruction::StoreFieldVar { .. }
                | Instruction::PushScope
                | Instruction::PopScope
                | Instruction::Jump(_)
                | Instruction::SpawnEvent(_)
                | Instruction::MakeClosure { .. }
                | Instruction::Halt => {}
            }
        }
    }

    fn infer_function_return_kinds(&self, bytecode: &Bytecode) -> HashMap<String, NativeValueKind> {
        let mut known = HashMap::new();
        for _ in 0..4 {
            for (name, function) in &bytecode.functions {
                let kind = self.infer_return_kind_for_instructions(
                    name,
                    &function.instructions,
                    &bytecode.constants,
                    &known,
                );
                known.insert(name.clone(), kind);
            }
        }
        known
    }

    fn infer_function_return_shapes(&self, bytecode: &Bytecode) -> HashMap<String, ValueShape> {
        let mut shapes = HashMap::new();
        for _ in 0..4 {
            for (name, function) in &bytecode.functions {
                let params = self
                    .function_param_kinds
                    .get(name)
                    .cloned()
                    .unwrap_or_default();
                let shape = self.infer_return_shape_for_instructions(
                    &function.instructions,
                    &bytecode.constants,
                    &params,
                );
                shapes.insert(name.clone(), shape);
            }
        }
        shapes
    }

    fn infer_return_shape_for_instructions(
        &self,
        instructions: &[Instruction],
        constants: &[Constant],
        current_params: &[NativeValueKind],
    ) -> ValueShape {
        let mut stack: Vec<ValueShape> = Vec::new();
        let mut locals: HashMap<String, ValueShape> = HashMap::new();

        for instruction in instructions {
            match instruction {
                Instruction::LoadConst(id) => match constants.get(*id) {
                    Some(Constant::String(value)) => stack.push(ValueShape::string(value.clone())),
                    _ => stack.push(ValueShape::new(Self::constant_kind(constants, *id))),
                },
                Instruction::LoadLocal(name) | Instruction::LoadGlobal(name) => {
                    stack.push(
                        locals
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int)),
                    );
                }
                Instruction::LoadParam(index) => {
                    stack.push(ValueShape::new(
                        current_params
                            .get(*index)
                            .copied()
                            .unwrap_or(NativeValueKind::Int),
                    ));
                }
                Instruction::StoreLocal(name)
                | Instruction::StoreGlobal(name)
                | Instruction::StoreExisting(name) => {
                    let shape = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    locals.insert(name.clone(), shape);
                }
                Instruction::NewList(count) => {
                    let mut elements = Vec::new();
                    for _ in 0..*count {
                        elements.push(
                            stack
                                .pop()
                                .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int))
                                .kind,
                        );
                    }
                    stack.push(ValueShape::new(Self::list_kind_from_elements(&elements)));
                }
                Instruction::NewMap(count) | Instruction::NewStruct(_, count) => {
                    let mut fields = HashMap::new();
                    let mut field_shapes = HashMap::new();
                    let mut key_kinds = Vec::new();
                    let mut value_kinds = Vec::new();
                    for _ in 0..*count {
                        let value = stack
                            .pop()
                            .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                        let key = stack
                            .pop()
                            .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                        key_kinds.push(key.kind);
                        value_kinds.push(value.kind);
                        if let Some(key) = key.string_value {
                            fields.insert(key.clone(), value.kind);
                            field_shapes.insert(key, value);
                        }
                    }
                    let mut shape = ValueShape::new(NativeValueKind::Int);
                    shape.fields = fields;
                    shape.field_shapes = field_shapes;
                    shape.key_kinds = key_kinds;
                    shape.value_kinds = value_kinds;
                    stack.push(shape);
                }
                Instruction::LoadIndex => {
                    let index = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let collection = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let kind = index
                        .string_value
                        .as_ref()
                        .and_then(|key| collection.fields.get(key))
                        .copied()
                        .unwrap_or_else(|| Self::list_element_kind(collection.kind));
                    stack.push(
                        index
                            .string_value
                            .as_ref()
                            .and_then(|key| collection.field_shapes.get(key))
                            .cloned()
                            .unwrap_or_else(|| ValueShape::new(kind)),
                    );
                }
                Instruction::LoadField(field) => {
                    let target = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let kind = target
                        .fields
                        .get(field)
                        .copied()
                        .unwrap_or(NativeValueKind::Int);
                    stack.push(
                        target
                            .field_shapes
                            .get(field)
                            .cloned()
                            .unwrap_or_else(|| ValueShape::new(kind)),
                    );
                }
                Instruction::StoreIndexVar(name) => {
                    let value = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let index = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    if let (Some(target), Some(key)) = (locals.get_mut(name), index.string_value) {
                        target.fields.insert(key.clone(), value.kind);
                        target.field_shapes.insert(key, value.clone());
                        target.key_kinds.push(NativeValueKind::String);
                        target.value_kinds.push(value.kind);
                    }
                }
                Instruction::StoreFieldVar { target, field } => {
                    let value = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    if let Some(target) = locals.get_mut(target) {
                        target.fields.insert(field.clone(), value.kind);
                        target.field_shapes.insert(field.clone(), value.clone());
                        target.key_kinds.push(NativeValueKind::String);
                        target.value_kinds.push(value.kind);
                    }
                }
                Instruction::ListPush => {
                    let value = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    let list = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    stack.push(ValueShape::new(Self::list_kind_after_push(
                        list.kind, value.kind,
                    )));
                }
                Instruction::ListPushVar(name) => {
                    let value = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    if let Some(target) = locals.get_mut(name) {
                        target.kind = Self::list_kind_after_push(target.kind, value.kind);
                    }
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::ListPop => {
                    let list = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    stack.push(ValueShape::new(Self::list_element_kind(list.kind)));
                    stack.push(list);
                }
                Instruction::ListPopVar(name) => {
                    let list_kind = locals
                        .get(name)
                        .map(|shape| shape.kind)
                        .unwrap_or(NativeValueKind::Int);
                    stack.push(ValueShape::new(Self::list_element_kind(list_kind)));
                }
                Instruction::ListLen | Instruction::MapHas => {
                    let _ = stack.pop();
                    if matches!(instruction, Instruction::MapHas) {
                        let _ = stack.pop();
                    }
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::MapKeys => {
                    let map = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    stack.push(ValueShape::new(Self::list_kind_from_elements(
                        &map.key_kinds,
                    )));
                }
                Instruction::MapValues => {
                    let map = stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                    stack.push(ValueShape::new(Self::list_kind_from_elements(
                        &map.value_kinds,
                    )));
                }
                Instruction::CallNamed { name, arg_count } => {
                    for _ in 0..*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(self.function_return_shape(name));
                }
                Instruction::Call(arg_count) => {
                    for _ in 0..=*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::Return => {
                    return stack
                        .pop()
                        .unwrap_or_else(|| ValueShape::new(NativeValueKind::Int));
                }
                Instruction::Print | Instruction::Pop | Instruction::JumpIfFalse(_) => {
                    let _ = stack.pop();
                }
                Instruction::Add
                | Instruction::Sub
                | Instruction::Mul
                | Instruction::Div
                | Instruction::Mod
                | Instruction::And
                | Instruction::Or
                | Instruction::Eq
                | Instruction::NotEq
                | Instruction::Lt
                | Instruction::Gt
                | Instruction::LtEq
                | Instruction::GtEq => {
                    let _ = stack.pop();
                    let _ = stack.pop();
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::Neg | Instruction::Not => {
                    let _ = stack.pop();
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::BackendCall { arg_count, .. } => {
                    for _ in 0..*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(ValueShape::new(NativeValueKind::Int));
                }
                Instruction::StoreIndex
                | Instruction::PushScope
                | Instruction::PopScope
                | Instruction::Jump(_)
                | Instruction::SpawnEvent(_)
                | Instruction::MakeClosure { .. }
                | Instruction::Halt => {}
            }
        }

        ValueShape::new(NativeValueKind::Int)
    }

    fn infer_return_kind_for_instructions(
        &self,
        function_name: &str,
        instructions: &[Instruction],
        constants: &[Constant],
        known_functions: &HashMap<String, NativeValueKind>,
    ) -> NativeValueKind {
        let mut stack = Vec::new();
        let mut locals = HashMap::new();

        for instruction in instructions {
            match instruction {
                Instruction::LoadConst(id) => stack.push(Self::constant_kind(constants, *id)),
                Instruction::LoadLocal(name) | Instruction::LoadGlobal(name) => {
                    stack.push(*locals.get(name).unwrap_or(&NativeValueKind::Int));
                }
                Instruction::LoadParam(index) => {
                    let kind = self
                        .function_param_kinds
                        .get(function_name)
                        .and_then(|params| params.get(*index))
                        .copied()
                        .unwrap_or(NativeValueKind::Int);
                    stack.push(kind);
                }
                Instruction::StoreLocal(name)
                | Instruction::StoreGlobal(name)
                | Instruction::StoreExisting(name) => {
                    let kind = stack.pop().unwrap_or(NativeValueKind::Int);
                    locals.insert(name.clone(), kind);
                }
                Instruction::CallNamed { name, arg_count } => {
                    for _ in 0..*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(*known_functions.get(name).unwrap_or(&NativeValueKind::Int));
                }
                Instruction::Call(arg_count) => {
                    for _ in 0..=*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(NativeValueKind::Int);
                }
                Instruction::Return => return stack.pop().unwrap_or(NativeValueKind::Int),
                Instruction::Print | Instruction::Pop | Instruction::JumpIfFalse(_) => {
                    let _ = stack.pop();
                }
                Instruction::Add
                | Instruction::Sub
                | Instruction::Mul
                | Instruction::Div
                | Instruction::Mod
                | Instruction::And
                | Instruction::Or
                | Instruction::Eq
                | Instruction::NotEq
                | Instruction::Lt
                | Instruction::Gt
                | Instruction::LtEq
                | Instruction::GtEq => {
                    let _ = stack.pop();
                    let _ = stack.pop();
                    stack.push(NativeValueKind::Int);
                }
                Instruction::Neg | Instruction::Not => {
                    let _ = stack.pop();
                    stack.push(NativeValueKind::Int);
                }
                Instruction::NewList(count) => {
                    let mut elements = Vec::new();
                    for _ in 0..*count {
                        elements.push(stack.pop().unwrap_or(NativeValueKind::Int));
                    }
                    stack.push(Self::list_kind_from_elements(&elements));
                }
                Instruction::LoadIndex => {
                    let _ = stack.pop();
                    let list_kind = stack.pop().unwrap_or(NativeValueKind::Int);
                    stack.push(Self::list_element_kind(list_kind));
                }
                Instruction::ListPop => {
                    let list_kind = stack.pop().unwrap_or(NativeValueKind::Int);
                    stack.push(Self::list_element_kind(list_kind));
                    stack.push(list_kind);
                }
                Instruction::ListLen
                | Instruction::NewMap(_)
                | Instruction::MapHas
                | Instruction::MapKeys
                | Instruction::MapValues
                | Instruction::NewStruct(_, _)
                | Instruction::LoadField(_) => stack.push(NativeValueKind::Int),
                Instruction::ListPopVar(name) => {
                    let list_kind = locals.get(name).copied().unwrap_or(NativeValueKind::Int);
                    stack.push(Self::list_element_kind(list_kind));
                }
                Instruction::BackendCall { arg_count, .. } => {
                    for _ in 0..*arg_count {
                        let _ = stack.pop();
                    }
                    stack.push(NativeValueKind::Int);
                }
                Instruction::StoreIndex
                | Instruction::StoreIndexVar(_)
                | Instruction::ListPush
                | Instruction::ListPushVar(_)
                | Instruction::StoreFieldVar { .. }
                | Instruction::PushScope
                | Instruction::PopScope
                | Instruction::Jump(_)
                | Instruction::SpawnEvent(_)
                | Instruction::MakeClosure { .. }
                | Instruction::Halt => {}
            }
        }

        NativeValueKind::Int
    }

    /// Create a new x86-64 code generator
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            data: Vec::new(),
            variables: HashMap::new(),
            variable_kinds: HashMap::new(),
            value_stack: Vec::new(),
            stack_offset: 0,
            stack_depth: 0,
            jump_targets: HashMap::new(),
            pending_jumps: Vec::new(),
            pending_data_patches: Vec::new(),
            pending_function_address_patches: Vec::new(),
            function_addresses: HashMap::new(),
            function_return_kinds: HashMap::new(),
            function_return_shapes: HashMap::new(),
            function_param_kinds: HashMap::new(),
            current_function_name: None,
            standalone_executable: false,
            pending_standalone_print_calls: Vec::new(),
            pending_standalone_print_string_calls: Vec::new(),
            pending_standalone_string_eq_calls: Vec::new(),
            pending_pe_import_calls: Vec::new(),
            standalone_string_offsets: HashMap::new(),
            predicted_load_index_kinds: VecDeque::new(),
            predicted_load_field_kinds: VecDeque::new(),
            predicted_map_view_kinds: VecDeque::new(),
        }
    }

    /// Create a code generator for standalone executable output.
    pub fn new_standalone_executable() -> Self {
        let mut codegen = Self::new();
        codegen.standalone_executable = true;
        codegen
    }

    /// Compile Matter bytecode to x86-64 machine code
    pub fn compile(&mut self, bytecode: &Bytecode) -> Result<Vec<u8>, String> {
        let entry_patch = if self.standalone_executable {
            Some(self.emit_standalone_process_entry())
        } else {
            None
        };

        self.function_param_kinds.clear();
        self.function_return_kinds.clear();
        self.function_return_shapes.clear();
        for _ in 0..4 {
            self.function_param_kinds = self.infer_function_param_kinds(bytecode);
            self.function_return_kinds = self.infer_function_return_kinds(bytecode);
            self.function_return_shapes = self.infer_function_return_shapes(bytecode);
        }

        // First pass: compile all functions
        for (name, function) in &bytecode.functions {
            self.compile_function(name, function, &bytecode.constants)?;
        }

        let standalone_named_main_entry = if self.standalone_executable
            && Self::has_empty_top_level(&bytecode.main_instructions)
            && bytecode.functions.contains_key("main")
        {
            Some(
                *self
                    .function_addresses
                    .get("main")
                    .ok_or_else(|| "Compiled main function address is missing".to_string())?,
            )
        } else {
            None
        };

        // Record main function start
        let main_start = self.code.len();
        if let Some(entry_patch) = entry_patch {
            self.patch_call_rel32(
                entry_patch,
                standalone_named_main_entry.unwrap_or(main_start),
            );
        }

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
        let (load_index_kinds, load_field_kinds, map_view_kinds) = self
            .infer_dynamic_load_kinds_for_instructions(
                &bytecode.main_instructions,
                &bytecode.constants,
                &[],
            );
        self.predicted_load_index_kinds.extend(load_index_kinds);
        self.predicted_load_field_kinds.extend(load_field_kinds);
        self.predicted_map_view_kinds.extend(map_view_kinds);

        for (ip, instr) in bytecode.main_instructions.iter().enumerate() {
            // Mark jump target
            if self.jump_targets.contains_key(&ip) {
                self.jump_targets.insert(ip, self.code.len());
            }

            self.compile_instruction(instr, &bytecode.constants)?;
        }

        // Emit main function epilogue
        self.emit_epilogue();

        if self.standalone_executable && !self.pending_standalone_print_calls.is_empty() {
            let helper_offset = self.emit_standalone_print_i64_helper()?;
            let calls = std::mem::take(&mut self.pending_standalone_print_calls);
            for call_pos in calls {
                self.patch_call_rel32(call_pos, helper_offset);
            }
        }

        if self.standalone_executable && !self.pending_standalone_print_string_calls.is_empty() {
            let helper_offset = self.emit_standalone_print_string_helper()?;
            let calls = std::mem::take(&mut self.pending_standalone_print_string_calls);
            for call_pos in calls {
                self.patch_call_rel32(call_pos, helper_offset);
            }
        }

        if self.standalone_executable && !self.pending_standalone_string_eq_calls.is_empty() {
            let helper_offset = self.emit_standalone_string_eq_helper();
            let calls = std::mem::take(&mut self.pending_standalone_string_eq_calls);
            for call_pos in calls {
                self.patch_call_rel32(call_pos, helper_offset);
            }
        }

        // Fourth pass: patch jumps
        self.patch_jumps()?;
        self.patch_function_address_patches()?;

        // Capture code length for data patching. PE import calls must use the
        // final .text payload size because this generator appends literal data
        // to .text before the linker places .idata.
        let code_len = self.code.len();
        let text_len = code_len + self.data.len();
        self.patch_pe_import_calls(text_len)?;

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
        let saved_variable_kinds = self.variable_kinds.clone();
        let saved_value_stack = self.value_stack.clone();
        let saved_current_function_name = self.current_function_name.clone();
        let saved_stack_offset = self.stack_offset;
        let saved_stack_depth = self.stack_depth;
        let saved_jump_targets = self.jump_targets.clone();
        let saved_pending_jumps = self.pending_jumps.clone();

        // Reset for function compilation
        self.variables.clear();
        self.variable_kinds.clear();
        self.value_stack.clear();
        self.stack_depth = 0;
        self.current_function_name = Some(name.to_string());
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
            let param_kind = self
                .function_param_kinds
                .get(name)
                .and_then(|params| params.get(i))
                .copied()
                .unwrap_or(NativeValueKind::Int);
            self.variable_kinds.insert(param_name.clone(), param_kind);

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
        let param_kinds = self
            .function_param_kinds
            .get(name)
            .cloned()
            .unwrap_or_default();
        let (load_index_kinds, load_field_kinds, map_view_kinds) = self
            .infer_dynamic_load_kinds_for_instructions(
                &function.instructions,
                constants,
                &param_kinds,
            );
        self.predicted_load_index_kinds.extend(load_index_kinds);
        self.predicted_load_field_kinds.extend(load_field_kinds);
        self.predicted_map_view_kinds.extend(map_view_kinds);

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
        self.variable_kinds = saved_variable_kinds;
        self.value_stack = saved_value_stack;
        self.current_function_name = saved_current_function_name;
        self.stack_offset = saved_stack_offset;
        self.stack_depth = saved_stack_depth;
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
                self.compile_mod()?;
            }
            Instruction::Neg => {
                self.compile_neg()?;
            }
            Instruction::And => {
                self.compile_and()?;
            }
            Instruction::Or => {
                self.compile_or()?;
            }
            Instruction::Not => {
                self.compile_not()?;
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
            Instruction::CallNamed { name, arg_count } => {
                self.compile_named_call(name, *arg_count)?;
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
            Instruction::MakeClosure { .. } => {
                // Closure compilation: same as LoadFunction for native
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
                if self.standalone_executable {
                    let string_offset = self.add_data_standalone_string(s);
                    let patch_pos = self.code.len() + 3; // Offset in lea rax, [rip + disp32]
                    self.emit_lea_rip(Register::RAX, 0);
                    self.pending_data_patches.push((patch_pos, string_offset));
                    self.emit_push_typed(Register::RAX, NativeValueKind::String);
                    return Ok(());
                }

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
            Constant::Null => {
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

    fn compile_mod(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RBX, "Mod")?; // Divisor
        self.emit_pop_checked(Register::RAX, "Mod")?; // Dividend

        self.emit_cqo();
        self.emit_div_reg(Register::RBX);
        self.emit_push(Register::RDX);

        Ok(())
    }

    fn compile_neg(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RAX, "Neg")?;
        self.code.extend_from_slice(&[0x48, 0xF7, 0xD8]); // neg rax
        self.emit_push(Register::RAX);

        Ok(())
    }

    fn compile_and(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RBX, "And")?;
        self.emit_pop_checked(Register::RAX, "And")?;
        self.emit_boolize(Register::RAX);
        self.emit_boolize(Register::RBX);
        self.code.extend_from_slice(&[0x48, 0x21, 0xD8]); // and rax, rbx
        self.emit_push(Register::RAX);

        Ok(())
    }

    fn compile_or(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RBX, "Or")?;
        self.emit_pop_checked(Register::RAX, "Or")?;
        self.emit_boolize(Register::RAX);
        self.emit_boolize(Register::RBX);
        self.code.extend_from_slice(&[0x48, 0x09, 0xD8]); // or rax, rbx
        self.emit_push(Register::RAX);

        Ok(())
    }

    fn compile_not(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RAX, "Not")?;
        self.emit_boolize(Register::RAX);
        self.code.extend_from_slice(&[0x48, 0x83, 0xF0, 0x01]); // xor rax, 1
        self.emit_push(Register::RAX);

        Ok(())
    }

    /// Compile comparison instruction
    fn compile_comparison(&mut self, predicate: &str) -> Result<(), String> {
        let right_kind = self.emit_pop_checked(Register::RBX, predicate)?; // Right
        let left_kind = self.emit_pop_checked(Register::RAX, predicate)?; // Left

        if left_kind == NativeValueKind::String || right_kind == NativeValueKind::String {
            if !self.standalone_executable {
                return Err(format!(
                    "String comparison is only supported in standalone native executables for predicate {}",
                    predicate
                ));
            }
            if left_kind != NativeValueKind::String || right_kind != NativeValueKind::String {
                return Err(format!(
                    "Cannot compare string and non-string values with predicate {}",
                    predicate
                ));
            }
            if predicate != "e" && predicate != "ne" {
                return Err(format!(
                    "Standalone native string comparison only supports equality and inequality, got {}",
                    predicate
                ));
            }

            let call_pos = self.code.len();
            self.code.push(0xE8); // call rel32
            self.code.extend_from_slice(&0i32.to_le_bytes());
            self.pending_standalone_string_eq_calls.push(call_pos);
            if predicate == "ne" {
                self.code.extend_from_slice(&[0x48, 0x83, 0xF0, 0x01]); // xor rax, 1
            }
            self.emit_push(Register::RAX);
            return Ok(());
        }

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
        let kind = self.emit_pop_checked(Register::RAX, "StoreLocal")?;

        // Allocate stack space if variable doesn't exist
        if !self.variables.contains_key(name) {
            self.stack_offset -= 8;
            self.variables.insert(name.to_string(), self.stack_offset);
        }
        self.variable_kinds.insert(name.to_string(), kind);

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
            .copied()
            .ok_or_else(|| format!("Undefined variable: {}", name))?;

        // mov rax, [rbp + offset]
        self.emit_mov_from_stack(Register::RAX, offset);
        let kind = self
            .variable_kinds
            .get(name)
            .copied()
            .unwrap_or(NativeValueKind::Int);
        self.emit_push_typed(Register::RAX, kind);

        Ok(())
    }

    /// Compile LoadGlobal instruction
    fn compile_load_global(&mut self, name: &str) -> Result<(), String> {
        // Check if this is a function name
        if self.function_addresses.contains_key(name) {
            self.emit_function_address(name)?;
            return Ok(());
        }

        if self.standalone_executable {
            return self.compile_load_local(name);
        }

        // Otherwise, load from runtime globals
        let name_offset = self.add_data_string(name);

        // 1. Load runtime pointer from [rbp - 8] into RCX/RDI
        #[cfg(windows)]
        let arg1_reg = Register::RCX;
        #[cfg(not(windows))]
        let arg1_reg = Register::RDI;

        self.emit_mov_from_stack(arg1_reg, RUNTIME_PTR_STACK_OFFSET);

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
        if self.standalone_executable {
            return self.compile_store_local(name);
        }

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
        self.emit_mov_from_stack(arg1_reg, RUNTIME_PTR_STACK_OFFSET);

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

    fn add_data_standalone_string(&mut self, s: &str) -> usize {
        if let Some(offset) = self.standalone_string_offsets.get(s) {
            return *offset;
        }

        let offset = self.data.len();
        self.data.extend_from_slice(&(s.len() as u64).to_le_bytes());
        self.data.extend_from_slice(s.as_bytes());
        self.standalone_string_offsets.insert(s.to_string(), offset);
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
        if self.standalone_executable {
            let kind = self.emit_pop_checked(Register::RAX, "Print")?;
            let call_pos = self.code.len();
            self.code.push(0xE8); // call rel32
            self.code.extend_from_slice(&0i32.to_le_bytes());
            match kind {
                NativeValueKind::Int => self.pending_standalone_print_calls.push(call_pos),
                NativeValueKind::String => {
                    self.pending_standalone_print_string_calls.push(call_pos)
                }
                NativeValueKind::ListInt | NativeValueKind::ListString => {
                    return Err(
                        "Standalone native Print does not support list values yet".to_string()
                    )
                }
            }
            return Ok(());
        }

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
        self.compile_call_with_return_kind(arg_count, NativeValueKind::Int)
    }

    fn compile_call_with_return_kind(
        &mut self,
        arg_count: usize,
        return_kind: NativeValueKind,
    ) -> Result<(), String> {
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
        self.emit_mov_from_stack(arg_regs[0], RUNTIME_PTR_STACK_OFFSET);

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
        self.emit_push_typed(Register::RAX, return_kind);

        Ok(())
    }

    /// Compile Return instruction
    fn compile_return(&mut self) -> Result<(), String> {
        // Pop return value from stack if present
        let return_kind = if self.stack_depth > 0 {
            self.emit_pop(Register::RAX)
        } else {
            // Default return 0 (Unit)
            self.emit_mov_imm(Register::RAX, 0);
            NativeValueKind::Int
        };

        if let Some(name) = &self.current_function_name {
            self.function_return_kinds.insert(name.clone(), return_kind);
        }

        // Restore the native stack frame before returning to either the Matter
        // executable shim or another generated function.
        self.emit_epilogue();

        Ok(())
    }

    /// Compile Pop instruction
    fn compile_pop(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RAX, "Pop")?;
        Ok(())
    }

    fn compile_named_call(&mut self, name: &str, arg_count: usize) -> Result<(), String> {
        let return_kind = self
            .function_return_kinds
            .get(name)
            .copied()
            .unwrap_or(NativeValueKind::Int);
        self.emit_function_address(name)?;
        self.compile_call_with_return_kind(arg_count, return_kind)
    }

    fn emit_function_address(&mut self, name: &str) -> Result<(), String> {
        let patch_pos = self.code.len() + 3;
        self.emit_lea_rip(Register::RAX, 0);
        if let Some(&func_addr) = self.function_addresses.get(name) {
            self.patch_rip_relative_disp32(patch_pos, func_addr)
                .map_err(|_| format!("Function address out of range: {}", name))?;
        } else {
            self.pending_function_address_patches
                .push((patch_pos, name.to_string()));
        }
        self.emit_push(Register::RAX);
        Ok(())
    }

    fn emit_standalone_process_entry(&mut self) -> usize {
        self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]); // sub rsp, 40
        self.code.extend_from_slice(&[0x48, 0x31, 0xC9]); // xor rcx, rcx

        let call_pos = self.code.len();
        self.code.push(0xE8); // call Matter main
        self.code.extend_from_slice(&0i32.to_le_bytes());

        self.code.extend_from_slice(&[0x31, 0xC9]); // xor ecx, ecx
        self.emit_call_pe_import(PeImport::ExitProcess);
        self.code.push(0xF4); // hlt, unreachable after ExitProcess

        call_pos
    }

    #[cfg(windows)]
    fn emit_standalone_print_i64_helper(&mut self) -> Result<usize, String> {
        let helper_offset = self.code.len();

        self.code.push(0x55); // push rbp
        self.code.extend_from_slice(&[0x48, 0x89, 0xE5]); // mov rbp, rsp
        self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x60]); // sub rsp, 96

        self.code
            .extend_from_slice(&[0x4C, 0x8D, 0x95, 0xFF, 0xFF, 0xFF, 0xFF]); // lea r10, [rbp - 1]
        self.code.extend_from_slice(&[0x41, 0xC6, 0x02, 0x0A]); // mov byte ptr [r10], '\n'
        self.emit_mov_imm(Register::R8, 1); // bytes to write
        self.code.extend_from_slice(&[0x45, 0x31, 0xC9]); // xor r9d, r9d (negative flag)

        self.emit_test_reg(Register::RAX);
        let nonzero_jump = self.code.len();
        self.emit_jne(0);
        self.emit_standalone_decimal_digit(b'0');
        let zero_done_jump = self.code.len();
        self.emit_jmp(0);

        let nonzero_pos = self.code.len();
        self.patch_jcc_rel32(nonzero_jump, nonzero_pos);
        self.emit_test_reg(Register::RAX);
        let positive_jump = self.code.len();
        self.emit_jge(0);
        self.code.extend_from_slice(&[0x48, 0xF7, 0xD8]); // neg rax
        self.emit_mov_imm(Register::R9, 1);

        let convert_pos = self.code.len();
        self.patch_jcc_rel32(positive_jump, convert_pos);
        self.emit_mov_imm(Register::RCX, 10);
        let loop_pos = self.code.len();
        self.code.extend_from_slice(&[0x31, 0xD2]); // xor edx, edx
        self.code.extend_from_slice(&[0x48, 0xF7, 0xF1]); // div rcx
        self.code.extend_from_slice(&[0x80, 0xC2, b'0']); // add dl, '0'
        self.code.extend_from_slice(&[0x49, 0xFF, 0xCA]); // dec r10
        self.code.extend_from_slice(&[0x41, 0x88, 0x12]); // mov [r10], dl
        self.code.extend_from_slice(&[0x49, 0xFF, 0xC0]); // inc r8
        self.emit_test_reg(Register::RAX);
        let loop_jump = self.code.len();
        self.emit_jne(0);
        self.patch_jcc_rel32(loop_jump, loop_pos);

        self.code.extend_from_slice(&[0x4D, 0x85, 0xC9]); // test r9, r9
        let write_jump = self.code.len();
        self.emit_je(0);
        self.emit_standalone_decimal_digit(b'-');

        let write_pos = self.code.len();
        self.patch_jmp_rel32(zero_done_jump, write_pos);
        self.patch_jcc_rel32(write_jump, write_pos);

        self.code.extend_from_slice(&[0xB9, 0xF5, 0xFF, 0xFF, 0xFF]); // mov ecx, -11
        self.emit_call_pe_import(PeImport::GetStdHandle);
        self.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // mov rcx, rax
        self.code.extend_from_slice(&[0x4C, 0x89, 0xD2]); // mov rdx, r10
        self.code
            .extend_from_slice(&[0x4C, 0x8D, 0x8D, 0xF0, 0xFF, 0xFF, 0xFF]); // lea r9, [rbp - 16]
        self.code
            .extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0, 0, 0, 0]); // overlapped = null
        self.emit_call_pe_import(PeImport::WriteFile);

        self.emit_epilogue();
        Ok(helper_offset)
    }

    #[cfg(not(windows))]
    fn emit_standalone_print_i64_helper(&mut self) -> Result<usize, String> {
        Err("Standalone native Print is only implemented for Windows x86-64 PE output".to_string())
    }

    #[cfg(windows)]
    fn emit_standalone_print_string_helper(&mut self) -> Result<usize, String> {
        let helper_offset = self.code.len();

        self.code.push(0x55); // push rbp
        self.code.extend_from_slice(&[0x48, 0x89, 0xE5]); // mov rbp, rsp
        self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x60]); // sub rsp, 96

        self.emit_mov_to_stack(-24, Register::RAX); // Preserve [len][bytes] pointer.

        self.code.extend_from_slice(&[0xB9, 0xF5, 0xFF, 0xFF, 0xFF]); // mov ecx, -11
        self.emit_call_pe_import(PeImport::GetStdHandle);
        self.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // mov rcx, rax
        self.emit_mov_from_stack(Register::R11, -24);
        self.code.extend_from_slice(&[0x49, 0x8D, 0x53, 0x08]); // lea rdx, [r11 + 8]
        self.code.extend_from_slice(&[0x4D, 0x8B, 0x03]); // mov r8, [r11]
        self.code
            .extend_from_slice(&[0x4C, 0x8D, 0x8D, 0xF0, 0xFF, 0xFF, 0xFF]); // lea r9, [rbp - 16]
        self.code
            .extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0, 0, 0, 0]); // overlapped = null
        self.emit_call_pe_import(PeImport::WriteFile);

        self.code
            .extend_from_slice(&[0xC6, 0x85, 0xEF, 0xFF, 0xFF, 0xFF, 0x0A]); // mov byte [rbp - 17], '\n'
        self.code.extend_from_slice(&[0xB9, 0xF5, 0xFF, 0xFF, 0xFF]); // mov ecx, -11
        self.emit_call_pe_import(PeImport::GetStdHandle);
        self.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // mov rcx, rax
        self.code
            .extend_from_slice(&[0x48, 0x8D, 0x95, 0xEF, 0xFF, 0xFF, 0xFF]); // lea rdx, [rbp - 17]
        self.emit_mov_imm(Register::R8, 1);
        self.code
            .extend_from_slice(&[0x4C, 0x8D, 0x8D, 0xF0, 0xFF, 0xFF, 0xFF]); // lea r9, [rbp - 16]
        self.code
            .extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0, 0, 0, 0]); // overlapped = null
        self.emit_call_pe_import(PeImport::WriteFile);

        self.emit_epilogue();
        Ok(helper_offset)
    }

    #[cfg(not(windows))]
    fn emit_standalone_print_string_helper(&mut self) -> Result<usize, String> {
        Err("Standalone native Print is only implemented for Windows x86-64 PE output".to_string())
    }

    fn emit_standalone_string_eq_helper(&mut self) -> usize {
        let helper_offset = self.code.len();

        self.code.push(0x55); // push rbp
        self.code.extend_from_slice(&[0x48, 0x89, 0xE5]); // mov rbp, rsp

        self.code.extend_from_slice(&[0x48, 0x8B, 0x10]); // mov rdx, [rax]
        self.code.extend_from_slice(&[0x48, 0x3B, 0x13]); // cmp rdx, [rbx]
        let length_mismatch_jump = self.code.len();
        self.emit_jne(0);

        self.code.extend_from_slice(&[0x4C, 0x8D, 0x40, 0x08]); // lea r8, [rax + 8]
        self.code.extend_from_slice(&[0x4C, 0x8D, 0x4B, 0x08]); // lea r9, [rbx + 8]
        self.emit_test_reg(Register::RDX);
        let empty_string_jump = self.code.len();
        self.emit_je(0);

        let loop_pos = self.code.len();
        self.code.extend_from_slice(&[0x41, 0x8A, 0x08]); // mov cl, [r8]
        self.code.extend_from_slice(&[0x41, 0x3A, 0x09]); // cmp cl, [r9]
        let byte_mismatch_jump = self.code.len();
        self.emit_jne(0);
        self.code.extend_from_slice(&[0x49, 0xFF, 0xC0]); // inc r8
        self.code.extend_from_slice(&[0x49, 0xFF, 0xC1]); // inc r9
        self.code.extend_from_slice(&[0x48, 0xFF, 0xCA]); // dec rdx
        let loop_jump = self.code.len();
        self.emit_jne(0);
        self.patch_jcc_rel32(loop_jump, loop_pos);

        let equal_pos = self.code.len();
        self.patch_jcc_rel32(empty_string_jump, equal_pos);
        self.emit_mov_imm(Register::RAX, 1);
        let done_jump = self.code.len();
        self.emit_jmp(0);

        let not_equal_pos = self.code.len();
        self.patch_jcc_rel32(length_mismatch_jump, not_equal_pos);
        self.patch_jcc_rel32(byte_mismatch_jump, not_equal_pos);
        self.code.extend_from_slice(&[0x48, 0x31, 0xC0]); // xor rax, rax

        let done_pos = self.code.len();
        self.patch_jmp_rel32(done_jump, done_pos);
        self.emit_epilogue();

        helper_offset
    }

    fn emit_standalone_decimal_digit(&mut self, byte: u8) {
        self.code.extend_from_slice(&[0x49, 0xFF, 0xCA]); // dec r10
        self.code.extend_from_slice(&[0x41, 0xC6, 0x02, byte]); // mov byte ptr [r10], byte
        self.code.extend_from_slice(&[0x49, 0xFF, 0xC0]); // inc r8
    }

    fn emit_call_pe_import(&mut self, import: PeImport) {
        let call_pos = self.code.len();
        self.code.extend_from_slice(&[0xFF, 0x15]);
        self.code.extend_from_slice(&0i32.to_le_bytes());
        self.pending_pe_import_calls.push((call_pos, import));
    }

    #[cfg(windows)]
    fn emit_standalone_heap_alloc(&mut self, size: usize) {
        self.emit_sub_imm(Register::RSP, 32);
        self.emit_call_pe_import(PeImport::GetProcessHeap);
        self.emit_mov_reg(Register::RCX, Register::RAX);
        self.code.extend_from_slice(&[0x31, 0xD2]); // xor edx, edx
        self.emit_mov_imm(Register::R8, size as i64);
        self.emit_call_pe_import(PeImport::HeapAlloc);
        self.emit_add_imm(Register::RSP, 32);
    }

    #[cfg(windows)]
    fn emit_standalone_heap_alloc_reg(&mut self, size: Register) {
        self.emit_mov_reg(Register::RBX, size);
        self.emit_sub_imm(Register::RSP, 32);
        self.emit_call_pe_import(PeImport::GetProcessHeap);
        self.emit_mov_reg(Register::RCX, Register::RAX);
        self.code.extend_from_slice(&[0x31, 0xD2]); // xor edx, edx
        self.emit_mov_reg(Register::R8, Register::RBX);
        self.emit_call_pe_import(PeImport::HeapAlloc);
        self.emit_add_imm(Register::RSP, 32);
    }

    #[cfg(not(windows))]
    fn emit_standalone_heap_alloc(&mut self, _size: usize) {
        self.emit_mov_imm(Register::RAX, 0);
    }

    #[cfg(not(windows))]
    fn emit_standalone_heap_alloc_reg(&mut self, _size: Register) {
        self.emit_mov_imm(Register::RAX, 0);
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
            ("math", "abs" | "neg") => Some(1),
            ("math", "clamp") => Some(3),
            ("math", "max" | "min" | "mod" | "pow") => Some(2),
            ("math", "sqrt") => Some(1),
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

        let expected_string_arity = match (backend, method) {
            ("string", "contains") => Some(2),
            ("string", "len") => Some(1),
            _ => None,
        };
        if let Some(expected_arity) = expected_string_arity {
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
                ("abs", 1) => {
                    self.pop_for_backend_call(Register::RAX, backend, method)?;

                    self.emit_mov_imm(Register::RCX, i64::MIN);
                    self.emit_cmp_reg(Register::RAX, Register::RCX);
                    let abs_overflow_jump_pos = self.code.len();
                    self.emit_je(0);

                    self.emit_test_reg(Register::RAX);
                    let non_negative_jump_pos = self.code.len();
                    self.emit_jge(0);
                    self.code.extend_from_slice(&[0x48, 0xF7, 0xD8]); // neg rax

                    let push_pos = self.code.len();
                    self.patch_jcc_rel32(non_negative_jump_pos, push_pos);
                    self.emit_push(Register::RAX);

                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let panic_pos = self.code.len();
                    self.patch_jcc_rel32(abs_overflow_jump_pos, panic_pos);
                    self.emit_panic_message("Overflow in math.abs");

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(end_jump_pos, end_pos);
                    return Ok(());
                }
                ("min", 2) => {
                    self.pop_for_backend_call(Register::RBX, backend, method)?; // rhs
                    self.pop_for_backend_call(Register::RAX, backend, method)?; // lhs

                    self.emit_cmp_reg(Register::RAX, Register::RBX);
                    let lhs_is_min_jump_pos = self.code.len();
                    self.emit_jl(0);
                    self.emit_mov_reg(Register::RAX, Register::RBX);

                    let push_pos = self.code.len();
                    self.patch_jcc_rel32(lhs_is_min_jump_pos, push_pos);
                    self.emit_push(Register::RAX);
                    return Ok(());
                }
                ("max", 2) => {
                    self.pop_for_backend_call(Register::RBX, backend, method)?; // rhs
                    self.pop_for_backend_call(Register::RAX, backend, method)?; // lhs

                    self.emit_cmp_reg(Register::RAX, Register::RBX);
                    let lhs_is_max_jump_pos = self.code.len();
                    self.emit_jge(0);
                    self.emit_mov_reg(Register::RAX, Register::RBX);

                    let push_pos = self.code.len();
                    self.patch_jcc_rel32(lhs_is_max_jump_pos, push_pos);
                    self.emit_push(Register::RAX);
                    return Ok(());
                }
                ("mod", 2) => {
                    self.pop_for_backend_call(Register::RBX, backend, method)?; // rhs
                    self.pop_for_backend_call(Register::RAX, backend, method)?; // lhs

                    self.emit_test_reg(Register::RBX);
                    let div_zero_jump_pos = self.code.len();
                    self.emit_je(0);

                    self.emit_mov_imm(Register::RCX, i64::MIN);
                    self.emit_cmp_reg(Register::RAX, Register::RCX);
                    let maybe_overflow_jump_pos = self.code.len();
                    self.emit_jne(0);
                    self.emit_cmp_imm(Register::RBX, -1);
                    let div_overflow_jump_pos = self.code.len();
                    self.emit_je(0);

                    let continue_div_pos = self.code.len();
                    self.emit_cqo();
                    self.emit_div_reg(Register::RBX);
                    self.emit_push(Register::RDX);

                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let div_zero_panic_pos = self.code.len();
                    self.patch_jcc_rel32(div_zero_jump_pos, div_zero_panic_pos);
                    self.patch_jcc_rel32(maybe_overflow_jump_pos, continue_div_pos);
                    self.emit_panic_message("Division by zero in math.mod");

                    let div_overflow_panic_pos = self.code.len();
                    self.patch_jcc_rel32(div_overflow_jump_pos, div_overflow_panic_pos);
                    self.emit_panic_message("Overflow in math.mod");

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(end_jump_pos, end_pos);
                    return Ok(());
                }
                ("clamp", 3) => {
                    self.pop_for_backend_call(Register::RDX, backend, method)?; // max
                    self.pop_for_backend_call(Register::RBX, backend, method)?; // min
                    self.pop_for_backend_call(Register::RAX, backend, method)?; // value

                    self.emit_cmp_reg(Register::RDX, Register::RBX);
                    let invalid_range_jump_pos = self.code.len();
                    self.emit_jl(0);

                    self.emit_cmp_reg(Register::RAX, Register::RBX);
                    let above_min_jump_pos = self.code.len();
                    self.emit_jge(0);
                    self.emit_mov_reg(Register::RAX, Register::RBX);

                    let upper_check_pos = self.code.len();
                    self.patch_jcc_rel32(above_min_jump_pos, upper_check_pos);
                    self.emit_cmp_reg(Register::RAX, Register::RDX);
                    let below_max_jump_pos = self.code.len();
                    self.emit_jl(0);
                    self.emit_mov_reg(Register::RAX, Register::RDX);

                    let push_pos = self.code.len();
                    self.patch_jcc_rel32(below_max_jump_pos, push_pos);
                    self.emit_push(Register::RAX);

                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let panic_pos = self.code.len();
                    self.patch_jcc_rel32(invalid_range_jump_pos, panic_pos);
                    self.emit_panic_message("Invalid range in math.clamp");

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(end_jump_pos, end_pos);
                    return Ok(());
                }
                ("pow", 2) => {
                    self.pop_for_backend_call(Register::RBX, backend, method)?; // exponent
                    self.pop_for_backend_call(Register::RAX, backend, method)?; // base

                    self.emit_cmp_imm(Register::RBX, 0);
                    let negative_exponent_jump_pos = self.code.len();
                    self.emit_jl(0);

                    self.emit_mov_imm(Register::RCX, 1);
                    let loop_pos = self.code.len();
                    self.emit_cmp_imm(Register::RBX, 0);
                    let done_jump_pos = self.code.len();
                    self.emit_je(0);
                    self.emit_mul_reg(Register::RCX, Register::RAX);
                    let overflow_jump_pos = self.code.len();
                    self.emit_jo(0);
                    self.emit_sub_imm(Register::RBX, 1);
                    let loop_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let done_pos = self.code.len();
                    self.patch_jcc_rel32(done_jump_pos, done_pos);
                    self.emit_push(Register::RCX);

                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let negative_exponent_panic_pos = self.code.len();
                    self.patch_jcc_rel32(negative_exponent_jump_pos, negative_exponent_panic_pos);
                    self.emit_panic_message("Negative exponent in math.pow");

                    let overflow_panic_pos = self.code.len();
                    self.patch_jcc_rel32(overflow_jump_pos, overflow_panic_pos);
                    self.emit_panic_message("Overflow in math.pow");

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(loop_jump_pos, loop_pos);
                    self.patch_jmp_rel32(end_jump_pos, end_pos);
                    return Ok(());
                }
                ("sqrt", 1) => {
                    self.pop_for_backend_call(Register::RAX, backend, method)?;

                    self.emit_cmp_imm(Register::RAX, 0);
                    let negative_input_jump_pos = self.code.len();
                    self.emit_jl(0);

                    self.emit_cmp_imm(Register::RAX, 2);
                    let small_input_jump_pos = self.code.len();
                    self.emit_jl(0);

                    self.emit_mov_reg(Register::R12, Register::RAX); // n
                    self.emit_mov_reg(Register::RBX, Register::RAX); // x
                    let loop_pos = self.code.len();
                    self.emit_mov_reg(Register::RAX, Register::R12);
                    self.emit_cqo();
                    self.emit_div_reg(Register::RBX); // n / x
                    self.emit_add_reg(Register::RAX, Register::RBX);
                    self.emit_mov_imm(Register::RCX, 2);
                    self.emit_cqo();
                    self.emit_div_reg(Register::RCX); // y = (x + n / x) / 2
                    self.emit_cmp_reg(Register::RAX, Register::RBX);
                    let done_jump_pos = self.code.len();
                    self.emit_jge(0);
                    self.emit_mov_reg(Register::RBX, Register::RAX);
                    let loop_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let done_pos = self.code.len();
                    self.patch_jcc_rel32(done_jump_pos, done_pos);
                    self.emit_push(Register::RBX);

                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let small_input_pos = self.code.len();
                    self.patch_jcc_rel32(small_input_jump_pos, small_input_pos);
                    self.emit_push(Register::RAX);
                    let small_end_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let panic_pos = self.code.len();
                    self.patch_jcc_rel32(negative_input_jump_pos, panic_pos);
                    self.emit_panic_message("Negative input in math.sqrt");

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(loop_jump_pos, loop_pos);
                    self.patch_jmp_rel32(end_jump_pos, end_pos);
                    self.patch_jmp_rel32(small_end_jump_pos, end_pos);
                    return Ok(());
                }
                _ => {}
            }
        }

        if backend == "string" {
            match (method, arg_count) {
                ("len", 1) => {
                    self.pop_for_backend_call(Register::RAX, backend, method)?;
                    if self.standalone_executable {
                        self.emit_mov_from_mem(Register::RAX, Register::RAX, 0);
                        self.emit_push(Register::RAX);
                    } else {
                        self.emit_panic_message(
                            "BackendCall not supported in native runtime: string.len",
                        );
                    }
                    return Ok(());
                }
                ("contains", 2) => {
                    self.pop_for_backend_call(Register::RDX, backend, method)?; // needle
                    self.pop_for_backend_call(Register::RAX, backend, method)?; // haystack
                    if !self.standalone_executable {
                        self.emit_panic_message(
                            "BackendCall not supported in native runtime: string.contains",
                        );
                        return Ok(());
                    }

                    self.emit_mov_reg(Register::R8, Register::RAX); // haystack ptr
                    self.emit_mov_reg(Register::R9, Register::RDX); // needle ptr
                    self.emit_mov_from_mem(Register::RAX, Register::R8, 0); // haystack len
                    self.emit_mov_from_mem(Register::RDX, Register::R9, 0); // needle len

                    self.emit_test_reg(Register::RDX);
                    let empty_needle_jump_pos = self.code.len();
                    self.emit_je(0);

                    self.emit_cmp_reg(Register::RAX, Register::RDX);
                    let too_long_jump_pos = self.code.len();
                    self.emit_jl(0);

                    self.emit_sub_reg(Register::RAX, Register::RDX); // max start index
                    self.emit_mov_imm(Register::RCX, 0); // outer index

                    let outer_loop_pos = self.code.len();
                    self.emit_cmp_reg(Register::RCX, Register::RAX);
                    let not_found_jump_pos = self.code.len();
                    self.emit_jg(0);

                    self.emit_mov_imm(Register::R11, 0); // inner index
                    let inner_loop_pos = self.code.len();
                    self.emit_cmp_reg(Register::R11, Register::RDX);
                    let found_jump_pos = self.code.len();
                    self.emit_je(0);

                    self.emit_mov_reg(Register::RBX, Register::RCX);
                    self.emit_add_reg(Register::RBX, Register::R11);
                    self.emit_movzx_byte_from_base_index(
                        Register::R10,
                        Register::R8,
                        Register::RBX,
                        8,
                    );
                    self.emit_movzx_byte_from_base_index(
                        Register::RBX,
                        Register::R9,
                        Register::R11,
                        8,
                    );
                    self.emit_cmp_reg(Register::R10, Register::RBX);
                    let next_outer_jump_pos = self.code.len();
                    self.emit_jne(0);

                    self.emit_add_imm(Register::R11, 1);
                    let inner_loop_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let next_outer_pos = self.code.len();
                    self.patch_jcc_rel32(next_outer_jump_pos, next_outer_pos);
                    self.emit_add_imm(Register::RCX, 1);
                    let outer_loop_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let found_pos = self.code.len();
                    self.patch_jcc_rel32(empty_needle_jump_pos, found_pos);
                    self.patch_jcc_rel32(found_jump_pos, found_pos);
                    self.emit_mov_imm(Register::RAX, 1);
                    self.emit_push(Register::RAX);
                    let end_jump_pos = self.code.len();
                    self.emit_jmp(0);

                    let not_found_pos = self.code.len();
                    self.patch_jcc_rel32(too_long_jump_pos, not_found_pos);
                    self.patch_jcc_rel32(not_found_jump_pos, not_found_pos);
                    self.emit_mov_imm(Register::RAX, 0);
                    self.emit_push(Register::RAX);

                    let end_pos = self.code.len();
                    self.patch_jmp_rel32(inner_loop_jump_pos, inner_loop_pos);
                    self.patch_jmp_rel32(outer_loop_jump_pos, outer_loop_pos);
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

    fn patch_call_rel32(&mut self, call_pos: usize, target_pos: usize) {
        let offset = (target_pos as i32) - (call_pos as i32) - 5;
        self.code[call_pos + 1..call_pos + 5].copy_from_slice(&offset.to_le_bytes());
    }

    fn patch_pe_import_calls(&mut self, code_len: usize) -> Result<(), String> {
        for (call_pos, import) in &self.pending_pe_import_calls {
            let iat_rva = match import {
                PeImport::ExitProcess => crate::linker::pe::exit_process_iat_rva(code_len),
                PeImport::GetStdHandle => crate::linker::pe::get_std_handle_iat_rva(code_len),
                PeImport::WriteFile => crate::linker::pe::write_file_iat_rva(code_len),
                PeImport::GetProcessHeap => crate::linker::pe::get_process_heap_iat_rva(code_len),
                PeImport::HeapAlloc => crate::linker::pe::heap_alloc_iat_rva(code_len),
            };
            let next_instruction_rva = crate::linker::pe::TEXT_RVA as usize + *call_pos + 6;
            let relative = iat_rva as isize - next_instruction_rva as isize;
            let relative = i32::try_from(relative)
                .map_err(|_| "PE import call target out of range".to_string())?;
            self.code[*call_pos + 2..*call_pos + 6].copy_from_slice(&relative.to_le_bytes());
        }

        Ok(())
    }

    fn patch_function_address_patches(&mut self) -> Result<(), String> {
        let patches = std::mem::take(&mut self.pending_function_address_patches);
        for (patch_pos, name) in patches {
            let target_pos = *self
                .function_addresses
                .get(&name)
                .ok_or_else(|| format!("Undefined function: {}", name))?;
            self.patch_rip_relative_disp32(patch_pos, target_pos)
                .map_err(|_| format!("Function address out of range: {}", name))?;
        }

        Ok(())
    }

    fn patch_rip_relative_disp32(
        &mut self,
        patch_pos: usize,
        target_pos: usize,
    ) -> Result<(), std::num::TryFromIntError> {
        let next_instruction = patch_pos + 4;
        let relative = target_pos as isize - next_instruction as isize;
        let relative = i32::try_from(relative)?;
        self.code[patch_pos..patch_pos + 4].copy_from_slice(&relative.to_le_bytes());
        Ok(())
    }

    // ============================================================================
    // REGISTER MANAGEMENT
    // ============================================================================

    /// Emit: push reg
    fn emit_push(&mut self, reg: Register) {
        self.emit_push_typed(reg, NativeValueKind::Int);
    }

    fn emit_push_machine_only(&mut self, reg: Register) {
        if reg.encoding() >= 8 {
            self.code.push(0x41);
        }
        self.code.push(0x50 + (reg.encoding() & 7));
    }

    fn emit_pop_machine_only(&mut self, reg: Register) {
        if reg.encoding() >= 8 {
            self.code.push(0x41);
        }
        self.code.push(0x58 + (reg.encoding() & 7));
    }

    fn emit_push_typed(&mut self, reg: Register, kind: NativeValueKind) {
        // Opcode: 0x50 + reg
        // REX.B if needed
        if reg.encoding() >= 8 {
            self.code.push(0x41);
        }
        self.code.push(0x50 + (reg.encoding() & 7));
        self.stack_depth += 1;
        self.value_stack.push(kind);
    }

    /// Emit: pop reg
    fn emit_pop(&mut self, reg: Register) -> NativeValueKind {
        // Opcode: 0x58 + reg
        // REX.B if needed
        if reg.encoding() >= 8 {
            self.code.push(0x41);
        }
        self.code.push(0x58 + (reg.encoding() & 7));
        self.stack_depth -= 1;
        self.value_stack.pop().unwrap_or(NativeValueKind::Int)
    }

    fn emit_pop_checked(
        &mut self,
        reg: Register,
        instruction: &str,
    ) -> Result<NativeValueKind, String> {
        if self.stack_depth <= 0 {
            return Err(format!(
                "Stack underflow while compiling instruction {}",
                instruction
            ));
        }
        Ok(self.emit_pop(reg))
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

        // Preserve callee-saved registers that this code generator uses as temporaries.
        self.emit_push_machine_only(Register::RBX);
        self.emit_push_machine_only(Register::R12);
        self.emit_push_machine_only(Register::R14);
        self.emit_push_machine_only(Register::R15);

        // sub rsp, 256 (shadow space + fixed native compiler spill area)
        self.code.extend_from_slice(&[0x48, 0x81, 0xEC]);
        self.code.extend_from_slice(&256i32.to_le_bytes());

        // Save NativeRuntime pointer (1st arg) below the preserved registers.
        #[cfg(windows)]
        self.emit_mov_to_stack(RUNTIME_PTR_STACK_OFFSET, Register::RCX);
        #[cfg(not(windows))]
        self.emit_mov_to_stack(RUNTIME_PTR_STACK_OFFSET, Register::RDI);

        self.stack_offset = RUNTIME_PTR_STACK_OFFSET; // Start locals after runtime ptr
    }

    /// Emit function epilogue
    fn emit_epilogue(&mut self) {
        // Reset to the saved-register area. The generated code uses RSP as
        // its value stack, so returns must not assume the evaluation stack is
        // exactly balanced here.
        self.emit_mov_reg(Register::RSP, Register::RBP);
        self.emit_sub_imm(Register::RSP, 32);

        // Restore callee-saved temporaries.
        self.emit_pop_machine_only(Register::R15);
        self.emit_pop_machine_only(Register::R14);
        self.emit_pop_machine_only(Register::R12);
        self.emit_pop_machine_only(Register::RBX);

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

    fn emit_boolize(&mut self, reg: Register) {
        self.emit_test_reg(reg);

        // setne low 8-bit register
        self.code.extend_from_slice(&[0x0F, 0x95]);
        self.code.push(0xC0 | (reg.encoding() & 7));

        // movzx reg, reg8
        let rex = 0x48
            | if reg.encoding() >= 8 { 4 } else { 0 }
            | if reg.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);
        self.code.extend_from_slice(&[0x0F, 0xB6]);
        self.code
            .push(0xC0 | ((reg.encoding() & 7) << 3) | (reg.encoding() & 7));
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

    /// Emit: jg offset (jump if greater)
    fn emit_jg(&mut self, offset: i32) {
        // Opcode: 0x0F 0x8F (jg rel32)
        self.code.extend_from_slice(&[0x0F, 0x8F]);

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
        if self.standalone_executable {
            return self.compile_new_list_standalone(count);
        }

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

    fn compile_new_list_standalone(&mut self, count: usize) -> Result<(), String> {
        self.ensure_stack_items(count, "NewList", &format!("element_count={}", count))?;

        let element_kinds_start = self.value_stack.len().saturating_sub(count);
        let list_kind = Self::list_kind_from_elements(&self.value_stack[element_kinds_start..]);

        let saved_stack_offset = self.stack_offset;
        let mut element_offsets = vec![0i32; count];
        for i in (0..count).rev() {
            self.stack_offset -= 8;
            element_offsets[i] = self.stack_offset;
            self.emit_pop_checked(Register::RBX, "NewList")?;
            self.emit_mov_to_stack(element_offsets[i], Register::RBX);
        }

        self.emit_standalone_heap_alloc(32);
        self.emit_test_reg(Register::RAX);
        let list_alloc_fail_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_reg(Register::R15, Register::RAX);

        self.emit_mov_imm(Register::RBX, 0x01);
        self.emit_mov_to_mem(Register::R15, 0, Register::RBX);
        self.emit_mov_imm(Register::RBX, count as i64);
        self.emit_mov_to_mem(Register::R15, 8, Register::RBX);
        self.emit_mov_imm(Register::RBX, count as i64);
        self.emit_mov_to_mem(Register::R15, 16, Register::RBX);

        self.emit_standalone_heap_alloc(count.saturating_mul(8).max(8));
        self.emit_test_reg(Register::RAX);
        let data_alloc_fail_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_to_mem(Register::R15, 24, Register::RAX);

        for (i, element_offset) in element_offsets.iter().enumerate() {
            self.emit_mov_from_stack(Register::RBX, *element_offset);
            self.emit_mov_from_mem(Register::RCX, Register::R15, 24);
            self.emit_mov_to_mem_offset(Register::RCX, (i * 8) as i32, Register::RBX);
        }
        self.stack_offset = saved_stack_offset;

        self.emit_push_typed(Register::R15, list_kind);

        let end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let list_fail_pos = self.code.len();
        self.patch_jcc_rel32(list_alloc_fail_jump_pos, list_fail_pos);
        self.emit_panic_message("Standalone list allocation failed");

        let data_fail_pos = self.code.len();
        self.patch_jcc_rel32(data_alloc_fail_jump_pos, data_fail_pos);
        self.emit_panic_message("Standalone list data allocation failed");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        Ok(())
    }

    /// Compile LoadIndex instruction
    /// Pop index, pop list, push value
    /// Includes bounds checking with panic on out-of-bounds access
    fn compile_load_index(&mut self) -> Result<(), String> {
        self.ensure_stack_items(2, "LoadIndex", &Self::ctx_operands("list,index"))?;
        if self.standalone_executable {
            return self.compile_load_index_standalone();
        }

        // 1. Pop index
        self.emit_pop_checked(Register::RBX, "LoadIndex")?;

        // 2. Pop list
        let list_kind = self.emit_pop_checked(Register::RAX, "LoadIndex")?;

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
        self.emit_push_typed(Register::RAX, Self::list_element_kind(list_kind));

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

    fn compile_load_index_standalone(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RBX, "LoadIndex")?;
        let list_kind = self.emit_pop_checked(Register::RAX, "LoadIndex")?;

        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RCX, 0x01);
        let list_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_cmp_imm(Register::RCX, 0x02);
        let map_jump_pos = self.code.len();
        self.emit_je(0);

        self.emit_panic_message("Expected list or map for indexing");

        let list_pos = self.code.len();
        self.patch_jcc_rel32(list_jump_pos, list_pos);
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 8);
        self.emit_cmp_imm(Register::RBX, 0);
        let list_negative_jump_pos = self.code.len();
        self.emit_jl(0);
        self.emit_cmp_reg(Register::RBX, Register::RCX);
        let list_oob_jump_pos = self.code.len();
        self.emit_jge(0);
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 24);
        self.emit_shl_imm(Register::RBX, 3);
        self.emit_add_reg(Register::RCX, Register::RBX);
        self.emit_mov_from_mem(Register::RAX, Register::RCX, 0);
        self.emit_push_machine_only(Register::RAX);
        let list_end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let list_oob_pos = self.code.len();
        self.patch_jcc_rel32(list_negative_jump_pos, list_oob_pos);
        self.patch_jcc_rel32(list_oob_jump_pos, list_oob_pos);
        self.emit_panic_message("Index out of bounds or non-list");

        let map_pos = self.code.len();
        self.patch_jcc_rel32(map_jump_pos, map_pos);
        self.emit_standalone_map_bucket_slot(Register::RAX, Register::RBX, Register::RDX);
        self.emit_mov_from_mem(Register::RDX, Register::RDX, 0);

        let search_loop_pos = self.code.len();
        self.emit_test_reg(Register::RDX);
        let not_found_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_from_mem(Register::RCX, Register::RDX, 0);
        self.emit_cmp_reg(Register::RCX, Register::RBX);
        let found_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_from_mem(Register::RDX, Register::RDX, 16);
        let search_next_jump_pos = self.code.len();
        self.emit_jmp(0);
        self.patch_jmp_rel32(search_next_jump_pos, search_loop_pos);

        let found_pos = self.code.len();
        self.patch_jcc_rel32(found_jump_pos, found_pos);
        self.emit_mov_from_mem(Register::RAX, Register::RDX, 8);
        self.emit_push_machine_only(Register::RAX);
        let map_end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let not_found_pos = self.code.len();
        self.patch_jcc_rel32(not_found_jump_pos, not_found_pos);
        self.emit_panic_message("Map key not found");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(list_end_jump_pos, end_pos);
        self.patch_jmp_rel32(map_end_jump_pos, end_pos);
        let predicted_kind = self
            .predicted_load_index_kinds
            .pop_front()
            .unwrap_or_else(|| Self::list_element_kind(list_kind));
        self.stack_depth += 1;
        self.value_stack.push(predicted_kind);

        Ok(())
    }

    /// Compile StoreIndex instruction
    /// Pop value, pop index, pop list
    fn compile_store_index(&mut self) -> Result<(), String> {
        self.ensure_stack_items(3, "StoreIndex", &Self::ctx_operands("list,index,value"))?;
        if self.standalone_executable {
            return self.compile_store_index_standalone();
        }

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

    fn compile_store_index_standalone(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::R8, "StoreIndex")?;
        self.emit_pop_checked(Register::RBX, "StoreIndex")?;
        self.emit_pop_checked(Register::RAX, "StoreIndex")?;

        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RCX, 0x01);
        let list_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_cmp_imm(Register::RCX, 0x02);
        let map_jump_pos = self.code.len();
        self.emit_je(0);

        self.emit_panic_message("Expected list or map for index store");

        let list_pos = self.code.len();
        self.patch_jcc_rel32(list_jump_pos, list_pos);
        self.emit_cmp_imm(Register::RBX, 0);
        let list_oob_jump_pos = self.code.len();
        self.emit_jl(0);
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 8);
        self.emit_cmp_reg(Register::RBX, Register::RCX);
        let list_oob2_jump_pos = self.code.len();
        self.emit_jge(0);
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 24);
        self.emit_shl_imm(Register::RBX, 3);
        self.emit_add_reg(Register::RCX, Register::RBX);
        self.emit_mov_to_mem(Register::RCX, 0, Register::R8);
        let list_end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let list_oob_pos = self.code.len();
        self.patch_jcc_rel32(list_oob_jump_pos, list_oob_pos);
        self.patch_jcc_rel32(list_oob2_jump_pos, list_oob_pos);
        self.emit_panic_message("Index out of bounds (store) or non-list");

        let map_pos = self.code.len();
        self.patch_jcc_rel32(map_jump_pos, map_pos);
        self.emit_mov_reg(Register::R9, Register::R8);
        self.emit_mov_reg(Register::R8, Register::RBX);
        self.emit_standalone_map_insert()?;

        let end_pos = self.code.len();
        self.patch_jmp_rel32(list_end_jump_pos, end_pos);

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
            .copied()
            .ok_or_else(|| format!("Undefined variable: {}", name))?;
        self.emit_mov_from_stack(Register::RAX, offset);

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

        if self.standalone_executable {
            self.emit_mov_reg(Register::R9, Register::R8);
            self.emit_mov_reg(Register::R8, Register::RBX);
            self.emit_standalone_map_insert()?;
            let map_end_pos = self.code.len();
            self.patch_jmp_rel32(list_end_jump_pos, map_end_pos);
            return Ok(());
        }

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
        if self.standalone_executable {
            return self.compile_list_push_standalone();
        }

        self.ensure_stack_items(2, "ListPush", &Self::ctx_operands("list,value"))?;

        // 1. Pop value
        let value_kind = self.emit_pop_checked(Register::R8, "ListPush")?;

        // 2. Pop list
        let list_kind = self.emit_pop_checked(Register::RAX, "ListPush")?;

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
        let result_kind = Self::list_kind_after_push(list_kind, value_kind);
        self.emit_push_typed(Register::RAX, result_kind);

        Ok(())
    }

    /// Compile ListPushVar instruction
    /// Pop value, mutate variable list, push unit
    fn compile_list_push_var(&mut self, name: &str) -> Result<(), String> {
        if self.standalone_executable {
            return self.compile_list_push_var_standalone(name);
        }

        self.ensure_stack_items(1, "ListPushVar", &format!("target={},operands=value", name))?;

        // Pop value
        let value_kind = self.emit_pop_checked(Register::R8, "ListPushVar")?;
        let list_kind = self
            .variable_kinds
            .get(name)
            .copied()
            .unwrap_or(NativeValueKind::ListInt);
        let updated_list_kind = Self::list_kind_after_push(list_kind, value_kind);
        self.variable_kinds
            .insert(name.to_string(), updated_list_kind);

        // Load list from variable
        let offset = self
            .variables
            .get(name)
            .copied()
            .ok_or_else(|| format!("Undefined variable: {}", name))?;
        self.emit_mov_from_stack(Register::RAX, offset);

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

    fn compile_list_push_standalone(&mut self) -> Result<(), String> {
        self.ensure_stack_items(2, "ListPush", &Self::ctx_operands("list,value"))?;
        let value_kind = self.emit_pop_checked(Register::R8, "ListPush")?;
        let list_kind = self.emit_pop_checked(Register::RAX, "ListPush")?;
        self.emit_standalone_list_append(value_kind)?;
        let result_kind = Self::list_kind_after_push(list_kind, value_kind);
        self.emit_push_typed(Register::RAX, result_kind);
        Ok(())
    }

    fn compile_list_push_var_standalone(&mut self, name: &str) -> Result<(), String> {
        self.ensure_stack_items(1, "ListPushVar", &format!("target={},operands=value", name))?;
        let value_kind = self.emit_pop_checked(Register::R8, "ListPushVar")?;
        let offset = self
            .variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;
        let list_kind = self
            .variable_kinds
            .get(name)
            .copied()
            .unwrap_or(NativeValueKind::ListInt);
        let updated_list_kind = Self::list_kind_after_push(list_kind, value_kind);
        self.variable_kinds
            .insert(name.to_string(), updated_list_kind);
        self.emit_mov_from_stack(Register::RAX, *offset);
        self.emit_standalone_list_append(value_kind)?;
        self.emit_mov_imm(Register::RAX, 0);
        self.emit_push(Register::RAX);
        Ok(())
    }

    fn emit_standalone_list_append(&mut self, _value_kind: NativeValueKind) -> Result<(), String> {
        let saved_stack_offset = self.stack_offset;
        self.stack_offset -= 8;
        let list_slot = self.stack_offset;
        self.stack_offset -= 8;
        let value_slot = self.stack_offset;
        self.stack_offset -= 8;
        let len_slot = self.stack_offset;
        self.stack_offset -= 8;
        let new_cap_slot = self.stack_offset;
        self.stack_offset -= 8;
        let allocation_size_slot = self.stack_offset;
        self.stack_offset -= 8;
        let new_data_slot = self.stack_offset;

        self.emit_mov_from_mem(Register::RDX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RDX, 0x01);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0);

        self.emit_mov_to_stack(list_slot, Register::RAX);
        self.emit_mov_to_stack(value_slot, Register::R8);
        self.emit_mov_from_mem(Register::RBX, Register::RAX, 8); // len
        self.emit_mov_to_stack(len_slot, Register::RBX);
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 16); // cap
        self.emit_cmp_reg(Register::RBX, Register::RCX);
        let resize_jump_pos = self.code.len();
        self.emit_jge(0);

        let no_resize_jump_pos = self.code.len();
        self.emit_jmp(0);

        let resize_pos = self.code.len();
        self.patch_jcc_rel32(resize_jump_pos, resize_pos);
        self.emit_add_imm(Register::RCX, 1);
        self.emit_mov_to_stack(new_cap_slot, Register::RCX);
        self.emit_mov_reg(Register::RDX, Register::RCX);
        self.emit_shl_imm(Register::RDX, 3);
        self.emit_mov_to_stack(allocation_size_slot, Register::RDX);
        self.emit_standalone_heap_alloc_reg(Register::RDX);
        self.emit_test_reg(Register::RAX);
        let alloc_fail_jump_pos = self.code.len();
        self.emit_je(0);

        self.emit_mov_to_stack(new_data_slot, Register::RAX);
        self.emit_mov_from_stack(Register::RAX, list_slot);
        self.emit_mov_from_mem(Register::R8, Register::RAX, 24); // old data
        self.emit_mov_from_stack(Register::R9, new_data_slot);
        self.emit_mov_from_stack(Register::RBX, len_slot);
        self.emit_test_reg(Register::RBX);
        let copy_done_jump_pos = self.code.len();
        self.emit_je(0);

        let copy_loop_pos = self.code.len();
        self.emit_mov_from_mem(Register::RDX, Register::R8, 0);
        self.emit_mov_to_mem(Register::R9, 0, Register::RDX);
        self.emit_add_imm(Register::R8, 8);
        self.emit_add_imm(Register::R9, 8);
        self.emit_sub_imm(Register::RBX, 1);
        let copy_loop_jump_pos = self.code.len();
        self.emit_jne(0);
        self.patch_jcc_rel32(copy_loop_jump_pos, copy_loop_pos);

        let copy_done_pos = self.code.len();
        self.patch_jcc_rel32(copy_done_jump_pos, copy_done_pos);

        self.emit_mov_from_stack(Register::RAX, list_slot);
        self.emit_mov_from_stack(Register::RDX, new_data_slot);
        self.emit_mov_to_mem(Register::RAX, 24, Register::RDX);
        self.emit_mov_from_stack(Register::RDX, new_cap_slot);
        self.emit_mov_to_mem(Register::RAX, 16, Register::RDX);

        let after_resize_pos = self.code.len();
        self.patch_jmp_rel32(no_resize_jump_pos, after_resize_pos);

        self.emit_mov_from_stack(Register::RAX, list_slot);
        self.emit_mov_from_stack(Register::RBX, len_slot);
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 24); // data
        self.emit_mov_reg(Register::RDX, Register::RBX);
        self.emit_shl_imm(Register::RDX, 3);
        self.emit_add_reg(Register::RCX, Register::RDX);
        self.emit_mov_from_stack(Register::R8, value_slot);
        self.emit_mov_to_mem(Register::RCX, 0, Register::R8);
        self.emit_add_imm(Register::RBX, 1);
        self.emit_mov_to_mem(Register::RAX, 8, Register::RBX);

        let end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let type_panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, type_panic_pos);
        self.emit_panic_message("Expected list for push");

        let alloc_fail_pos = self.code.len();
        self.patch_jcc_rel32(alloc_fail_jump_pos, alloc_fail_pos);
        self.emit_panic_message("Standalone list resize failed");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        self.stack_offset = saved_stack_offset;
        Ok(())
    }

    /// Compile ListPop instruction
    /// Pop list, push value, push list (mutated)
    fn compile_list_pop(&mut self) -> Result<(), String> {
        self.ensure_stack_items(1, "ListPop", &Self::ctx_operands("list"))?;

        // 1. Pop list
        let list_kind = self.emit_pop_checked(Register::RAX, "ListPop")?;
        let value_kind = Self::list_element_kind(list_kind);

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
        self.emit_push_typed(Register::R8, value_kind);

        // 10. Push list
        self.emit_push_typed(Register::RAX, list_kind);

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
        let list_kind = self
            .variable_kinds
            .get(name)
            .copied()
            .unwrap_or(NativeValueKind::Int);
        let value_kind = Self::list_element_kind(list_kind);

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
        self.emit_push_typed(Register::R8, value_kind);
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
        if self.standalone_executable {
            return self.compile_new_map_standalone(count);
        }

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

    fn compile_new_map_standalone(&mut self, count: usize) -> Result<(), String> {
        let saved_stack_offset = self.stack_offset;
        self.stack_offset -= 8;
        let map_slot = self.stack_offset;
        self.stack_offset -= 8;
        let buckets_slot = self.stack_offset;

        self.emit_standalone_heap_alloc(24);
        self.emit_test_reg(Register::RAX);
        let map_alloc_fail_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_to_stack(map_slot, Register::RAX);

        self.emit_standalone_heap_alloc(16 * 8);
        self.emit_test_reg(Register::RAX);
        let buckets_alloc_fail_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_to_stack(buckets_slot, Register::RAX);

        self.emit_mov_reg(Register::RCX, Register::RAX);
        self.emit_mov_imm(Register::RDX, 0);
        self.emit_mov_imm(Register::RBX, 16);
        let zero_loop_pos = self.code.len();
        self.emit_mov_to_mem(Register::RCX, 0, Register::RDX);
        self.emit_add_imm(Register::RCX, 8);
        self.emit_sub_imm(Register::RBX, 1);
        let zero_loop_jump_pos = self.code.len();
        self.emit_jne(0);
        self.patch_jcc_rel32(zero_loop_jump_pos, zero_loop_pos);

        self.emit_mov_from_stack(Register::RAX, map_slot);
        self.emit_mov_imm(Register::RDX, 0x02);
        self.emit_mov_to_mem(Register::RAX, 0, Register::RDX);
        self.emit_mov_imm(Register::RDX, 0);
        self.emit_mov_to_mem(Register::RAX, 8, Register::RDX);
        self.emit_mov_from_stack(Register::RDX, buckets_slot);
        self.emit_mov_to_mem(Register::RAX, 16, Register::RDX);

        for _ in 0..count {
            self.emit_pop_checked(Register::R9, "NewMap")?;
            self.emit_pop_checked(Register::R8, "NewMap")?;
            self.emit_mov_from_stack(Register::RAX, map_slot);
            self.emit_standalone_map_insert()?;
        }

        self.emit_mov_from_stack(Register::RAX, map_slot);
        self.emit_push(Register::RAX);

        let end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let map_alloc_fail_pos = self.code.len();
        self.patch_jcc_rel32(map_alloc_fail_jump_pos, map_alloc_fail_pos);
        self.emit_panic_message("Map allocation failed");

        let buckets_alloc_fail_pos = self.code.len();
        self.patch_jcc_rel32(buckets_alloc_fail_jump_pos, buckets_alloc_fail_pos);
        self.emit_panic_message("Map bucket allocation failed");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        self.stack_offset = saved_stack_offset;
        Ok(())
    }

    fn emit_standalone_map_bucket_slot(&mut self, map: Register, key: Register, dest: Register) {
        self.emit_mov_from_mem(dest, map, 16);
        self.emit_mov_reg(Register::RCX, key);
        self.emit_and_imm(Register::RCX, 15);
        self.emit_shl_imm(Register::RCX, 3);
        self.emit_add_reg(dest, Register::RCX);
    }

    fn emit_standalone_map_insert(&mut self) -> Result<(), String> {
        let saved_stack_offset = self.stack_offset;
        self.stack_offset -= 8;
        let map_slot = self.stack_offset;
        self.stack_offset -= 8;
        let key_slot = self.stack_offset;
        self.stack_offset -= 8;
        let value_slot = self.stack_offset;
        self.stack_offset -= 8;
        let bucket_slot_addr_slot = self.stack_offset;
        self.stack_offset -= 8;
        let current_slot = self.stack_offset;
        self.stack_offset -= 8;
        let new_bucket_slot = self.stack_offset;

        self.emit_mov_to_stack(map_slot, Register::RAX);
        self.emit_mov_to_stack(key_slot, Register::R8);
        self.emit_mov_to_stack(value_slot, Register::R9);

        self.emit_standalone_map_bucket_slot(Register::RAX, Register::R8, Register::RDX);
        self.emit_mov_to_stack(bucket_slot_addr_slot, Register::RDX);
        self.emit_mov_from_mem(Register::RBX, Register::RDX, 0);
        self.emit_mov_to_stack(current_slot, Register::RBX);

        let search_loop_pos = self.code.len();
        self.emit_mov_from_stack(Register::RBX, current_slot);
        self.emit_test_reg(Register::RBX);
        let insert_new_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_from_mem(Register::RCX, Register::RBX, 0);
        self.emit_mov_from_stack(Register::R8, key_slot);
        self.emit_cmp_reg(Register::RCX, Register::R8);
        let update_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_from_mem(Register::RBX, Register::RBX, 16);
        self.emit_mov_to_stack(current_slot, Register::RBX);
        let search_next_jump_pos = self.code.len();
        self.emit_jmp(0);
        self.patch_jmp_rel32(search_next_jump_pos, search_loop_pos);

        let update_pos = self.code.len();
        self.patch_jcc_rel32(update_jump_pos, update_pos);
        self.emit_mov_from_stack(Register::RBX, current_slot);
        self.emit_mov_from_stack(Register::R9, value_slot);
        self.emit_mov_to_mem(Register::RBX, 8, Register::R9);
        let end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let insert_new_pos = self.code.len();
        self.patch_jcc_rel32(insert_new_jump_pos, insert_new_pos);
        self.emit_standalone_heap_alloc(24);
        self.emit_test_reg(Register::RAX);
        let alloc_fail_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_to_stack(new_bucket_slot, Register::RAX);
        self.emit_mov_from_stack(Register::R8, key_slot);
        self.emit_mov_to_mem(Register::RAX, 0, Register::R8);
        self.emit_mov_from_stack(Register::R9, value_slot);
        self.emit_mov_to_mem(Register::RAX, 8, Register::R9);
        self.emit_mov_from_stack(Register::RDX, bucket_slot_addr_slot);
        self.emit_mov_from_mem(Register::RCX, Register::RDX, 0);
        self.emit_mov_to_mem(Register::RAX, 16, Register::RCX);
        self.emit_mov_to_mem(Register::RDX, 0, Register::RAX);
        self.emit_mov_from_stack(Register::RAX, map_slot);
        self.emit_mov_from_mem(Register::RBX, Register::RAX, 8);
        self.emit_add_imm(Register::RBX, 1);
        self.emit_mov_to_mem(Register::RAX, 8, Register::RBX);
        let insert_end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let alloc_fail_pos = self.code.len();
        self.patch_jcc_rel32(alloc_fail_jump_pos, alloc_fail_pos);
        self.emit_panic_message("Map bucket allocation failed");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);
        self.patch_jmp_rel32(insert_end_jump_pos, end_pos);

        self.stack_offset = saved_stack_offset;
        Ok(())
    }

    /// Compile MapHas instruction
    /// Pop key, pop map, push bool
    fn compile_map_has(&mut self) -> Result<(), String> {
        self.ensure_stack_items(2, "MapHas", &Self::ctx_operands("map,key"))?;
        if self.standalone_executable {
            return self.compile_map_has_standalone();
        }

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

    fn compile_map_has_standalone(&mut self) -> Result<(), String> {
        self.emit_pop_checked(Register::RBX, "MapHas")?;
        self.emit_pop_checked(Register::RAX, "MapHas")?;

        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RCX, 0x02);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0);

        self.emit_standalone_map_bucket_slot(Register::RAX, Register::RBX, Register::RDX);
        self.emit_mov_from_mem(Register::RDX, Register::RDX, 0);

        let search_loop_pos = self.code.len();
        self.emit_test_reg(Register::RDX);
        let not_found_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_from_mem(Register::RCX, Register::RDX, 0);
        self.emit_cmp_reg(Register::RCX, Register::RBX);
        let found_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_from_mem(Register::RDX, Register::RDX, 16);
        let search_next_jump_pos = self.code.len();
        self.emit_jmp(0);
        self.patch_jmp_rel32(search_next_jump_pos, search_loop_pos);

        let found_pos = self.code.len();
        self.patch_jcc_rel32(found_jump_pos, found_pos);
        self.emit_mov_imm(Register::RAX, 1);
        let end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let not_found_pos = self.code.len();
        self.patch_jcc_rel32(not_found_jump_pos, not_found_pos);
        self.emit_mov_imm(Register::RAX, 0);
        let not_found_end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, panic_pos);
        self.emit_panic_message("Expected map for has");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);
        self.patch_jmp_rel32(not_found_end_jump_pos, end_pos);
        self.emit_push(Register::RAX);

        Ok(())
    }

    /// Compile MapKeys instruction
    /// Pop map, push list of keys
    fn compile_map_keys(&mut self) -> Result<(), String> {
        self.ensure_stack_items(1, "MapKeys", &Self::ctx_operands("map"))?;
        if self.standalone_executable {
            let result_kind = self
                .predicted_map_view_kinds
                .pop_front()
                .unwrap_or(NativeValueKind::ListInt);
            return self.compile_map_view_standalone(
                0,
                result_kind,
                "MapKeys",
                "Expected map for keys",
            );
        }

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
        if self.standalone_executable {
            let result_kind = self
                .predicted_map_view_kinds
                .pop_front()
                .unwrap_or(NativeValueKind::ListInt);
            return self.compile_map_view_standalone(
                8,
                result_kind,
                "MapValues",
                "Expected map for values",
            );
        }

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

    fn compile_map_view_standalone(
        &mut self,
        bucket_value_offset: i32,
        result_kind: NativeValueKind,
        instruction: &str,
        type_panic: &str,
    ) -> Result<(), String> {
        self.emit_pop_checked(Register::RAX, instruction)?;

        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RCX, 0x02);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0);

        let saved_stack_offset = self.stack_offset;
        self.stack_offset -= 8;
        let map_slot = self.stack_offset;
        self.stack_offset -= 8;
        let list_slot = self.stack_offset;
        self.stack_offset -= 8;
        let size_slot = self.stack_offset;
        self.stack_offset -= 8;
        let data_slot = self.stack_offset;
        self.stack_offset -= 8;
        let buckets_cursor_slot = self.stack_offset;
        self.stack_offset -= 8;
        let bucket_count_slot = self.stack_offset;
        self.stack_offset -= 8;
        let current_bucket_slot = self.stack_offset;
        self.stack_offset -= 8;
        let write_index_slot = self.stack_offset;

        self.emit_mov_to_stack(map_slot, Register::RAX);
        self.emit_mov_from_mem(Register::RBX, Register::RAX, 8);
        self.emit_mov_to_stack(size_slot, Register::RBX);

        self.emit_standalone_heap_alloc(32);
        self.emit_test_reg(Register::RAX);
        let list_alloc_fail_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_to_stack(list_slot, Register::RAX);

        self.emit_mov_from_stack(Register::RDX, size_slot);
        self.emit_shl_imm(Register::RDX, 3);
        self.emit_test_reg(Register::RDX);
        let non_empty_data_jump_pos = self.code.len();
        self.emit_jne(0);
        self.emit_mov_imm(Register::RDX, 8);
        let data_size_done_jump_pos = self.code.len();
        self.emit_jmp(0);
        let non_empty_data_pos = self.code.len();
        self.patch_jcc_rel32(non_empty_data_jump_pos, non_empty_data_pos);
        let data_size_done_pos = self.code.len();
        self.patch_jmp_rel32(data_size_done_jump_pos, data_size_done_pos);

        self.emit_standalone_heap_alloc_reg(Register::RDX);
        self.emit_test_reg(Register::RAX);
        let data_alloc_fail_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_to_stack(data_slot, Register::RAX);

        self.emit_mov_from_stack(Register::RAX, list_slot);
        self.emit_mov_imm(Register::RBX, 0x01);
        self.emit_mov_to_mem(Register::RAX, 0, Register::RBX);
        self.emit_mov_from_stack(Register::RBX, size_slot);
        self.emit_mov_to_mem(Register::RAX, 8, Register::RBX);
        self.emit_mov_to_mem(Register::RAX, 16, Register::RBX);
        self.emit_mov_from_stack(Register::RBX, data_slot);
        self.emit_mov_to_mem(Register::RAX, 24, Register::RBX);

        self.emit_mov_from_stack(Register::RAX, map_slot);
        self.emit_mov_from_mem(Register::RBX, Register::RAX, 16);
        self.emit_mov_to_stack(buckets_cursor_slot, Register::RBX);
        self.emit_mov_imm(Register::RBX, 16);
        self.emit_mov_to_stack(bucket_count_slot, Register::RBX);
        self.emit_mov_imm(Register::RBX, 0);
        self.emit_mov_to_stack(write_index_slot, Register::RBX);

        let outer_loop_pos = self.code.len();
        self.emit_mov_from_stack(Register::RBX, bucket_count_slot);
        self.emit_test_reg(Register::RBX);
        let done_jump_pos = self.code.len();
        self.emit_je(0);

        self.emit_mov_from_stack(Register::RCX, buckets_cursor_slot);
        self.emit_mov_from_mem(Register::RBX, Register::RCX, 0);
        self.emit_mov_to_stack(current_bucket_slot, Register::RBX);

        let inner_loop_pos = self.code.len();
        self.emit_mov_from_stack(Register::RBX, current_bucket_slot);
        self.emit_test_reg(Register::RBX);
        let next_bucket_jump_pos = self.code.len();
        self.emit_je(0);

        self.emit_mov_from_mem(Register::R8, Register::RBX, bucket_value_offset);
        self.emit_mov_from_stack(Register::R9, data_slot);
        self.emit_mov_from_stack(Register::RDX, write_index_slot);
        self.emit_shl_imm(Register::RDX, 3);
        self.emit_add_reg(Register::R9, Register::RDX);
        self.emit_mov_to_mem(Register::R9, 0, Register::R8);
        self.emit_mov_from_stack(Register::RDX, write_index_slot);
        self.emit_add_imm(Register::RDX, 1);
        self.emit_mov_to_stack(write_index_slot, Register::RDX);
        self.emit_mov_from_mem(Register::RBX, Register::RBX, 16);
        self.emit_mov_to_stack(current_bucket_slot, Register::RBX);
        let inner_continue_jump_pos = self.code.len();
        self.emit_jmp(0);
        self.patch_jmp_rel32(inner_continue_jump_pos, inner_loop_pos);

        let next_bucket_pos = self.code.len();
        self.patch_jcc_rel32(next_bucket_jump_pos, next_bucket_pos);
        self.emit_mov_from_stack(Register::RCX, buckets_cursor_slot);
        self.emit_add_imm(Register::RCX, 8);
        self.emit_mov_to_stack(buckets_cursor_slot, Register::RCX);
        self.emit_mov_from_stack(Register::RBX, bucket_count_slot);
        self.emit_sub_imm(Register::RBX, 1);
        self.emit_mov_to_stack(bucket_count_slot, Register::RBX);
        let outer_continue_jump_pos = self.code.len();
        self.emit_jmp(0);
        self.patch_jmp_rel32(outer_continue_jump_pos, outer_loop_pos);

        let done_pos = self.code.len();
        self.patch_jcc_rel32(done_jump_pos, done_pos);
        self.emit_mov_from_stack(Register::RAX, list_slot);
        self.emit_push_typed(Register::RAX, result_kind);
        let end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let type_panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, type_panic_pos);
        self.emit_panic_message(type_panic);

        let list_alloc_fail_pos = self.code.len();
        self.patch_jcc_rel32(list_alloc_fail_jump_pos, list_alloc_fail_pos);
        self.emit_panic_message("Map view list allocation failed");

        let data_alloc_fail_pos = self.code.len();
        self.patch_jcc_rel32(data_alloc_fail_jump_pos, data_alloc_fail_pos);
        self.emit_panic_message("Map view data allocation failed");

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

        self.stack_offset = saved_stack_offset;
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
        if self.standalone_executable {
            return self.compile_new_map_standalone(field_count);
        }

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
        if self.standalone_executable {
            return self.compile_load_field_standalone(field);
        }

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

    fn compile_load_field_standalone(&mut self, field: &str) -> Result<(), String> {
        self.emit_pop_checked(Register::RAX, "LoadField")?;

        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RCX, 0x02);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0);

        let field_offset = self.add_data_standalone_string(field);
        let patch_pos = self.code.len() + 3;
        self.emit_lea_rip(Register::RBX, 0);
        self.pending_data_patches.push((patch_pos, field_offset));

        self.emit_standalone_map_bucket_slot(Register::RAX, Register::RBX, Register::RDX);
        self.emit_mov_from_mem(Register::RDX, Register::RDX, 0);

        let search_loop_pos = self.code.len();
        self.emit_test_reg(Register::RDX);
        let missing_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_from_mem(Register::RCX, Register::RDX, 0);
        self.emit_cmp_reg(Register::RCX, Register::RBX);
        let found_jump_pos = self.code.len();
        self.emit_je(0);
        self.emit_mov_from_mem(Register::RDX, Register::RDX, 16);
        let search_next_jump_pos = self.code.len();
        self.emit_jmp(0);
        self.patch_jmp_rel32(search_next_jump_pos, search_loop_pos);

        let found_pos = self.code.len();
        self.patch_jcc_rel32(found_jump_pos, found_pos);
        self.emit_mov_from_mem(Register::RAX, Register::RDX, 8);
        let result_kind = self
            .predicted_load_field_kinds
            .pop_front()
            .unwrap_or(NativeValueKind::Int);
        self.emit_push_typed(Register::RAX, result_kind);
        let end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let type_panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, type_panic_pos);
        self.emit_panic_message("Expected struct or map for field access");

        let missing_pos = self.code.len();
        self.patch_jcc_rel32(missing_jump_pos, missing_pos);
        let panic_msg = format!("Field '{}' not found", field);
        self.emit_panic_message(&panic_msg);

        let end_pos = self.code.len();
        self.patch_jmp_rel32(end_jump_pos, end_pos);

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
        if self.standalone_executable {
            return self.compile_store_field_var_standalone(field);
        }

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

    fn compile_store_field_var_standalone(&mut self, field: &str) -> Result<(), String> {
        self.emit_mov_from_mem(Register::RCX, Register::RAX, 0);
        self.emit_cmp_imm(Register::RCX, 0x02);
        let type_panic_jump_pos = self.code.len();
        self.emit_jne(0);

        let field_offset = self.add_data_standalone_string(field);
        let patch_pos = self.code.len() + 3;
        self.emit_lea_rip(Register::R8, 0);
        self.pending_data_patches.push((patch_pos, field_offset));
        self.emit_mov_reg(Register::R9, Register::RBX);
        self.emit_standalone_map_insert()?;

        let end_jump_pos = self.code.len();
        self.emit_jmp(0);

        let type_panic_pos = self.code.len();
        self.patch_jcc_rel32(type_panic_jump_pos, type_panic_pos);
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

    /// Emit: movzx dest, byte ptr [base + index + offset]
    fn emit_movzx_byte_from_base_index(
        &mut self,
        dest: Register,
        base: Register,
        index: Register,
        offset: i32,
    ) {
        let rex = 0x48
            | if dest.encoding() >= 8 { 4 } else { 0 }
            | if index.encoding() >= 8 { 2 } else { 0 }
            | if base.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);
        self.code.extend_from_slice(&[0x0F, 0xB6]);

        // ModR/M: mod=10 disp32, r=dest, r/m=100 SIB follows.
        let modrm = 0x84 | ((dest.encoding() & 7) << 3);
        self.code.push(modrm);

        // SIB: scale=1, index=index, base=base.
        let sib = ((index.encoding() & 7) << 3) | (base.encoding() & 7);
        self.code.push(sib);
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

    /// Emit: and reg, imm
    fn emit_and_imm(&mut self, reg: Register, value: i32) {
        // REX.W + B
        let rex = 0x48 | if reg.encoding() >= 8 { 1 } else { 0 };
        self.code.push(rex);

        if (-128..=127).contains(&value) {
            // Opcode: 0x83 /4 (and r/m64, imm8)
            self.code.push(0x83);
            let modrm = 0xE0 | (reg.encoding() & 7);
            self.code.push(modrm);
            self.code.push(value as u8);
        } else {
            // Opcode: 0x81 /4 (and r/m64, imm32)
            self.code.push(0x81);
            let modrm = 0xE0 | (reg.encoding() & 7);
            self.code.push(modrm);
            self.code.extend_from_slice(&value.to_le_bytes());
        }
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
    fn test_boolean_ops_codegen() {
        let mut bytecode = Bytecode::new();
        let c0 = bytecode.add_constant(Constant::Int(0));
        let c2 = bytecode.add_constant(Constant::Int(2));
        let c3 = bytecode.add_constant(Constant::Int(3));

        bytecode.main_instructions = vec![
            Instruction::LoadConst(c2),
            Instruction::LoadConst(c3),
            Instruction::And,
            Instruction::Print,
            Instruction::LoadConst(c0),
            Instruction::LoadConst(c3),
            Instruction::Or,
            Instruction::Print,
            Instruction::LoadConst(c0),
            Instruction::Not,
            Instruction::Print,
            Instruction::Halt,
        ];

        let mut codegen = X86CodeGen::new();
        let code = codegen
            .compile(&bytecode)
            .expect("boolean operators should compile");

        assert!(!code.is_empty());
        assert!(
            code.windows([0x0F, 0x95].len()).any(|w| w == [0x0F, 0x95]),
            "boolean operators should lower through setne normalization"
        );
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
