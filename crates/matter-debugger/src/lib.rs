use matter_backend::Value;
use matter_bytecode::Bytecode;
use matter_vm::Vm;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugInfo {
    pub line_numbers: Vec<usize>,
    pub source_files: Vec<String>,
    pub variable_names: HashMap<usize, Vec<String>>,
    pub functions: Vec<FunctionDebugInfo>,
}

impl DebugInfo {
    pub fn new() -> Self {
        Self {
            line_numbers: Vec::new(),
            source_files: Vec::new(),
            variable_names: HashMap::new(),
            functions: Vec::new(),
        }
    }
}

impl Default for DebugInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDebugInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: usize,
    pub file: String,
    pub line: usize,
    pub condition: Option<String>,
    pub hit_count: usize,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: usize,
    pub name: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugState {
    Running,
    Paused,
    Stopped,
    StepOver,
    StepInto,
    StepOut,
}

pub struct InstrumentedVM {
    vm: Vm,
    _debug_info: DebugInfo,
    breakpoints: HashMap<String, Vec<Breakpoint>>,
    state: DebugState,
    next_breakpoint_id: usize,
}

impl InstrumentedVM {
    pub fn new(bytecode: Bytecode, debug_info: DebugInfo) -> Self {
        Self {
            vm: Vm::new(bytecode),
            _debug_info: debug_info,
            breakpoints: HashMap::new(),
            state: DebugState::Paused,
            next_breakpoint_id: 1,
        }
    }

    pub fn add_breakpoint(&mut self, file: String, line: usize) -> usize {
        let id = self.next_breakpoint_id;
        self.next_breakpoint_id += 1;
        self.breakpoints
            .entry(file.clone())
            .or_default()
            .push(Breakpoint {
                id,
                file,
                line,
                condition: None,
                hit_count: 0,
                enabled: true,
            });
        id
    }

    pub fn add_conditional_breakpoint(
        &mut self,
        file: String,
        line: usize,
        condition: String,
    ) -> usize {
        let id = self.add_breakpoint(file, line);
        if let Some(bp) = self
            .breakpoints
            .values_mut()
            .flat_map(|v| v.iter_mut())
            .find(|bp| bp.id == id)
        {
            bp.condition = Some(condition);
        }
        id
    }

    pub fn remove_breakpoint(&mut self, id: usize) -> bool {
        for list in self.breakpoints.values_mut() {
            if let Some(pos) = list.iter().position(|bp| bp.id == id) {
                list.remove(pos);
                return true;
            }
        }
        false
    }

    pub fn get_breakpoints(&self) -> Vec<&Breakpoint> {
        self.breakpoints.values().flat_map(|v| v.iter()).collect()
    }

    pub fn get_stack_frames(&self) -> Vec<StackFrame> {
        vec![StackFrame {
            id: 0,
            name: "<main>".to_string(),
            file: "<runtime>".to_string(),
            line: 0,
            column: 0,
        }]
    }

    pub fn get_locals(&self) -> HashMap<String, Value> {
        self.vm.get_globals()
    }

    pub fn get_globals(&self) -> HashMap<String, Value> {
        self.vm.get_globals()
    }

    pub fn evaluate(&mut self, _expression: &str) -> Result<Value, String> {
        Err("Expression evaluation not yet implemented".to_string())
    }

    pub fn continue_execution(&mut self) -> Result<(), String> {
        self.state = DebugState::Running;
        let result = self.vm.run().map_err(|e| e.to_string());
        self.state = DebugState::Stopped;
        result
    }

    pub fn step_over(&mut self) -> Result<(), String> {
        self.state = DebugState::Paused;
        Ok(())
    }

    pub fn step_into(&mut self) -> Result<(), String> {
        self.state = DebugState::Paused;
        Ok(())
    }

    pub fn step_out(&mut self) -> Result<(), String> {
        self.state = DebugState::Paused;
        Ok(())
    }

    pub fn pause(&mut self) {
        self.state = DebugState::Paused;
    }

    pub fn state(&self) -> DebugState {
        self.state
    }

    pub fn is_done(&self) -> bool {
        matches!(self.state, DebugState::Stopped)
    }
}

pub struct DebugAdapter {
    vm: InstrumentedVM,
    initialized: bool,
}

impl DebugAdapter {
    pub fn new(vm: InstrumentedVM) -> Self {
        Self {
            vm,
            initialized: false,
        }
    }

    pub fn initialize(&mut self) -> Result<(), String> {
        self.initialized = true;
        Ok(())
    }

    pub fn set_breakpoints(&mut self, file: String, lines: Vec<usize>) -> Vec<usize> {
        self.vm.breakpoints.remove(&file);
        lines
            .into_iter()
            .map(|line| self.vm.add_breakpoint(file.clone(), line))
            .collect()
    }

    pub fn continue_execution(&mut self) -> Result<(), String> {
        self.vm.continue_execution()
    }

    pub fn step_over(&mut self) -> Result<(), String> {
        self.vm.step_over()
    }

    pub fn step_into(&mut self) -> Result<(), String> {
        self.vm.step_into()
    }

    pub fn step_out(&mut self) -> Result<(), String> {
        self.vm.step_out()
    }

    pub fn pause(&mut self) {
        self.vm.pause()
    }

    pub fn get_stack_trace(&self) -> Vec<StackFrame> {
        self.vm.get_stack_frames()
    }

    pub fn get_variables(&self, scope: &str) -> HashMap<String, Value> {
        match scope {
            "locals" => self.vm.get_locals(),
            "globals" => self.vm.get_globals(),
            _ => HashMap::new(),
        }
    }

    pub fn evaluate(&mut self, expression: &str) -> Result<Value, String> {
        self.vm.evaluate(expression)
    }

    pub fn state(&self) -> DebugState {
        self.vm.state()
    }
}
