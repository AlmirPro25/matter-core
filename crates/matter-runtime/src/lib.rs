/// Matter Runtime
/// Sistema de eventos, estado e scheduler

use matter_backend::{AgentBackend, Backend, NetBackend, StoreBackend, Value, VisualBackend};
use matter_bytecode::Bytecode;
use matter_stdlib::{ListBackend, MathBackend, StringBackend};
use matter_vm::Vm;

pub struct Runtime {
    vm: Vm,
}

impl Runtime {
    pub fn new(bytecode: Bytecode) -> Self {
        let mut vm = Vm::new(bytecode);
        
        // Register default backends
        vm.register_backend("agent".to_string(), Box::new(AgentBackend::new()));
        vm.register_backend("visual".to_string(), Box::new(VisualBackend::new()));
        vm.register_backend("store".to_string(), Box::new(StoreBackend::new()));
        vm.register_backend("net".to_string(), Box::new(NetBackend::new()));
        
        // Register standard library backends
        vm.register_backend("math".to_string(), Box::new(MathBackend::new()));
        vm.register_backend("string".to_string(), Box::new(StringBackend::new()));
        vm.register_backend("list".to_string(), Box::new(ListBackend::new()));
        
        Self { vm }
    }

    pub fn new_silent(bytecode: Bytecode) -> Self {
        let mut vm = Vm::new(bytecode);

        vm.register_backend("agent".to_string(), Box::new(SilentAgentBackend));
        vm.register_backend("visual".to_string(), Box::new(SilentVisualBackend));
        vm.register_backend("store".to_string(), Box::new(StoreBackend::new()));
        vm.register_backend("net".to_string(), Box::new(NetBackend::new()));
        
        // Register standard library backends
        vm.register_backend("math".to_string(), Box::new(MathBackend::new()));
        vm.register_backend("string".to_string(), Box::new(StringBackend::new()));
        vm.register_backend("list".to_string(), Box::new(ListBackend::new()));

        Self { vm }
    }
    
    pub fn register_backend(&mut self, name: String, backend: Box<dyn Backend>) {
        self.vm.register_backend(name, backend);
    }

    pub fn set_stdout_enabled(&mut self, enabled: bool) {
        self.vm.set_stdout_enabled(enabled);
    }

    pub fn take_output(&mut self) -> Vec<String> {
        self.vm.take_output()
    }
    
    pub fn run(&mut self) -> Result<(), String> {
        self.vm.run().map_err(|e| e.to_string())
    }
    
    pub fn emit_event(&mut self, event: &str) -> Result<(), String> {
        self.vm.emit_event(event).map_err(|e| e.to_string())
    }
}

struct SilentAgentBackend;

impl Backend for SilentAgentBackend {
    fn call(&mut self, method: &str, _args: Vec<Value>) -> Result<Value, String> {
        match method {
            "say" => Ok(Value::Unit),
            _ => Err(format!("Unknown agent method: {}", method)),
        }
    }
}

struct SilentVisualBackend;

impl Backend for SilentVisualBackend {
    fn call(&mut self, method: &str, _args: Vec<Value>) -> Result<Value, String> {
        match method {
            "run" => Ok(Value::Unit),
            _ => Err(format!("Unknown visual method: {}", method)),
        }
    }
}
