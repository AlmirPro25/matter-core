//! Matter Runtime
//! Sistema de eventos, estado e scheduler
//!
//! Phase 1: default is language-only (stdlib + energy). Optional features:
//! `polyglot`, `visual`, `frontier`, `device`, `experimental-full`.

use matter_backend::{Backend, GraphBackend, StoreBackend, ToolBackend, Value};
#[cfg(feature = "agent")]
use matter_backend::AgentBackend;
#[cfg(feature = "net")]
use matter_backend::NetBackend;
#[cfg(feature = "frontier")]
use matter_biological::backend::BiologicalBackend;
#[cfg(feature = "frontier")]
use matter_biophysics::backend::BiophysicsBackend;
#[cfg(feature = "frontier")]
use matter_geophysics::backend::GeophysicsBackend;
#[cfg(feature = "frontier")]
use matter_ocean::backend::OceanBackend;
#[cfg(feature = "frontier")]
use matter_atmosphere::backend::AtmosphereBackend;
#[cfg(feature = "frontier")]
use matter_materials::backend::MaterialsBackend;
#[cfg(feature = "frontier")]
use matter_acoustics::backend::AcousticsBackend;
#[cfg(feature = "frontier")]
use matter_electromagnetics::backend::ElectromagneticsBackend;
#[cfg(feature = "frontier")]
use matter_ml_physics::backend::MLPhysicsBackend;
#[cfg(feature = "frontier")]
use matter_climate::backend::ClimateBackend;
#[cfg(feature = "frontier")]
use matter_multiscale::backend::MultiscaleBackend;
#[cfg(feature = "frontier")]
use matter_cosmology::backend::CosmologyBackend;
#[cfg(feature = "polyglot")]
use matter_bridge_go::{Bridge as GoBridgeTrait, GoBridge};
#[cfg(feature = "polyglot")]
use matter_bridge_java::{Bridge as JavaBridgeTrait, JavaBridge};
#[cfg(feature = "polyglot")]
use matter_bridge_nodejs::NodeJSBridge;
#[cfg(feature = "polyglot")]
use matter_bridge_python::PythonBridge;
#[cfg(feature = "polyglot")]
use matter_bridge_rust::RustBridge;
use matter_bytecode::Bytecode;
#[cfg(feature = "frontier")]
use matter_chemistry::backend::ChemistryBackend;
#[cfg(feature = "device")]
use matter_device::DeviceBackend;
use matter_energy::{EnergyBackend, EnergyRuntime};
#[cfg(feature = "frontier")]
use matter_genesis::backend::GenesisBackend;
#[cfg(feature = "frontier")]
use matter_memristive::backend::MemristiveBackend;
#[cfg(feature = "frontier")]
use matter_molecular::backend::MolecularBackend;
#[cfg(feature = "frontier")]
use matter_neuromorphic::backend::NeuromorphicBackend;
#[cfg(feature = "frontier")]
use matter_photonic::backend::PhotonicBackend;
#[cfg(feature = "polyglot")]
use matter_polyglot::{bridge::LanguageBridge, LanguageTarget};
#[cfg(feature = "frontier")]
use matter_quantum::backend::QuantumBackend;
#[cfg(feature = "frontier")]
use matter_relativity::backend::RelativityBackend;
#[cfg(feature = "frontier")]
use matter_spintronics::backend::SpintronicsBackend;
use matter_stdlib::{
    AudioBackend, ConsoleBackend, FileBackend, FileIOBackend, FsCapabilityPolicy, HashMapBackend,
    JsonBackend, ListBackend, MapBackend, MathBackend, OptionBackend, RandomBackend, ResultBackend,
    StringBackend, TensorBackend, TimeBackend, TypeBackend, VecBackend, WorldBackend,
};
#[cfg(feature = "visual")]
use matter_visual::TraceVisualBackend;
#[cfg(feature = "frontier")]
use matter_wetware::backend::WetwareBackend;

use matter_vm::Vm;
use std::collections::HashMap;

pub struct Runtime {
    vm: Vm,
    energy_runtime: Option<EnergyRuntime>,
}

impl Runtime {
    /// Create a runtime with default-deny filesystem capabilities.
    pub fn new(bytecode: Bytecode) -> Self {
        Self::with_fs_policy(bytecode, FsCapabilityPolicy::deny_all(), false)
    }

    /// Silent stdlib (no extended I/O noise) with default-deny FS.
    pub fn new_silent(bytecode: Bytecode) -> Self {
        Self::with_fs_policy(bytecode, FsCapabilityPolicy::deny_all(), true)
    }

    /// Runtime with explicit File Capabilities v1 policy for `file.*` / `fileio.*`.
    ///
    /// Program-initiated FS access is denied unless roots were granted on `policy`.
    /// CLI compile `-o` and host packaging paths are unrelated to this policy.
    pub fn with_fs_policy(bytecode: Bytecode, fs_policy: FsCapabilityPolicy, silent: bool) -> Self {
        let mut vm = Vm::new(bytecode);

        register_default_backends(&mut vm, silent);
        register_stdlib_backends(&mut vm, !silent, fs_policy);
        #[cfg(feature = "polyglot")]
        register_polyglot_backends(&mut vm);

        Self {
            vm,
            energy_runtime: Some(EnergyRuntime::new()),
        }
    }

    pub fn register_backend(&mut self, name: String, backend: Box<dyn Backend>) {
        self.vm.register_backend(name, backend);
    }

    pub fn call_backend(
        &mut self,
        backend: &str,
        method: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        self.vm
            .call_backend(backend, method, args)
            .map_err(|e| e.to_string())
    }

    pub fn set_stdout_enabled(&mut self, enabled: bool) {
        self.vm.set_stdout_enabled(enabled);
    }

    pub fn take_output(&mut self) -> Vec<String> {
        self.vm.take_output()
    }

    /// Extrai o estado global atual (para REPL)
    pub fn get_globals(&self) -> std::collections::HashMap<String, Value> {
        self.vm.get_globals()
    }

    /// Injeta estado global (para REPL)
    pub fn set_globals(&mut self, globals: std::collections::HashMap<String, Value>) {
        self.vm.set_globals(globals);
    }

    /// Mescla funções de outro bytecode (para REPL)
    pub fn merge_functions(&mut self, other_bytecode: &Bytecode) {
        self.vm.merge_functions(other_bytecode);
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.vm.run().map_err(|e| e.to_string())?;
        self.poll_energy_events()
    }

    pub fn emit_event(&mut self, event: &str) -> Result<(), String> {
        self.vm.emit_event(event).map_err(|e| e.to_string())
    }

    pub fn set_energy_runtime(&mut self, energy_runtime: Option<EnergyRuntime>) {
        self.energy_runtime = energy_runtime;
    }

    fn poll_energy_events(&mut self) -> Result<(), String> {
        let Some(energy_runtime) = &self.energy_runtime else {
            return Ok(());
        };
        let snapshot = energy_runtime.snapshot();

        if snapshot.cpu_usage > 80.0 {
            self.emit_event("energy.high")?;
        }
        if snapshot.cpu_usage < 20.0 {
            self.emit_event("energy.low")?;
        }
        if snapshot.cpu_usage > 90.0 || snapshot.memory_usage > 90.0 {
            self.emit_event("energy.spike")?;
            self.emit_event("performance.drop")?;
        }
        if snapshot.battery_level < 20.0 {
            self.emit_event("battery.low")?;
        }
        if snapshot.temperature > 75.0 {
            self.emit_event("heat.high")?;
        }
        Ok(())
    }

    pub fn vm(&self) -> &Vm {
        &self.vm
    }

    pub fn bridge_visual_events(&mut self, event_path: &str) -> Result<Value, String> {
        let dispatch = self
            .vm
            .call_backend(
                "visual",
                "dispatch_events",
                vec![Value::new_string(event_path.to_string())],
            )
            .map_err(|e| e.to_string())?;
        let loaded = self
            .vm
            .call_backend(
                "visual",
                "load_events",
                vec![Value::new_string(event_path.to_string())],
            )
            .map_err(|e| e.to_string())?;

        let emitted = self.emit_named_visual_events(&loaded)?;

        Ok(Value::new_map(HashMap::from([
            ("dispatch".to_string(), dispatch),
            ("emitted".to_string(), Value::new_list(emitted)),
        ])))
    }

    pub fn visual_app_step(&mut self, event_path: &str, delta_ms: i64) -> Result<Value, String> {
        let app = self
            .vm
            .call_backend(
                "visual",
                "app_step",
                vec![
                    Value::new_string(event_path.to_string()),
                    Value::Int(delta_ms),
                ],
            )
            .map_err(|e| e.to_string())?;
        let loaded = self
            .vm
            .call_backend(
                "visual",
                "load_events",
                vec![Value::new_string(event_path.to_string())],
            )
            .map_err(|e| e.to_string())?;
        let emitted = self.emit_named_visual_events(&loaded)?;

        Ok(Value::new_map(HashMap::from([
            ("app".to_string(), app),
            ("emitted".to_string(), Value::new_list(emitted)),
        ])))
    }

    pub fn visual_app_run(
        &mut self,
        event_path: &str,
        frames: i64,
        delta_ms: i64,
    ) -> Result<Value, String> {
        let app = self
            .vm
            .call_backend(
                "visual",
                "app_run",
                vec![
                    Value::new_string(event_path.to_string()),
                    Value::Int(frames),
                    Value::Int(delta_ms),
                ],
            )
            .map_err(|e| e.to_string())?;
        let loaded = self
            .vm
            .call_backend(
                "visual",
                "load_events",
                vec![Value::new_string(event_path.to_string())],
            )
            .map_err(|e| e.to_string())?;
        let emitted = self.emit_named_visual_events(&loaded)?;

        Ok(Value::new_map(HashMap::from([
            ("app".to_string(), app),
            ("emitted".to_string(), Value::new_list(emitted)),
        ])))
    }

    pub fn visual_export_web_runtime(
        &mut self,
        dir: &str,
        app_name: &str,
    ) -> Result<Value, String> {
        self.vm
            .call_backend(
                "visual",
                "web",
                vec![
                    Value::new_string(dir.to_string()),
                    Value::new_string(app_name.to_string()),
                ],
            )
            .map_err(|e| e.to_string())
    }

    pub fn visual_verify_web_runtime(&mut self, dir: &str) -> Result<Value, String> {
        self.vm
            .call_backend(
                "visual",
                "verify_web",
                vec![Value::new_string(dir.to_string())],
            )
            .map_err(|e| e.to_string())
    }

    pub fn tool_invoke_frame(&mut self, frame_payload: Value) -> Result<Value, String> {
        let result = self
            .vm
            .call_backend("tool", "invoke_frame", vec![frame_payload])
            .map_err(|e| e.to_string())?;

        if let Value::Map(map) = &result {
            if let Some(Value::String(kind)) = map.get("response_kind") {
                let response_kind = kind.to_ascii_lowercase();
                if response_kind == "needscontext" || response_kind == "needs_context" {
                    self.emit_event("tool.needs_context")?;
                } else if response_kind == "blocked" {
                    self.emit_event("tool.blocked")?;
                } else if response_kind == "accepted" {
                    self.emit_event("tool.accepted")?;
                } else if response_kind == "completed" {
                    self.emit_event("tool.completed")?;
                }
            }
        }
        Ok(result)
    }

    pub fn tool_validate_wire(&mut self, wire: &str) -> Result<Value, String> {
        self.vm
            .call_backend(
                "tool",
                "validate_wire",
                vec![Value::new_string(wire.to_string())],
            )
            .map_err(|e| e.to_string())
    }

    pub fn tool_merge_wire(
        &mut self,
        left_wire: &str,
        right_wire: &str,
        strategy: Option<&str>,
    ) -> Result<Value, String> {
        let mut args = vec![
            Value::new_string(left_wire.to_string()),
            Value::new_string(right_wire.to_string()),
        ];
        if let Some(strategy) = strategy {
            args.push(Value::new_string(strategy.to_string()));
        }
        self.vm
            .call_backend("tool", "merge_wire", args)
            .map_err(|e| e.to_string())
    }

    pub fn energy_mode_backend(&mut self) -> Result<String, String> {
        self.vm
            .call_backend("energy", "mode", vec![])
            .map_err(|e| e.to_string())?
            .as_string()
    }

    fn emit_named_visual_events(&mut self, loaded: &Value) -> Result<Vec<Value>, String> {
        let mut emitted = Vec::new();
        if let Value::List(events) = loaded {
            for event in events.iter() {
                if let Value::Map(fields) = event {
                    if let Some(Value::String(name)) = fields.get("event") {
                        let event_name = (**name).clone();
                        self.vm.emit_event(&event_name).map_err(|e| e.to_string())?;
                        emitted.push(Value::new_string(event_name));
                    }
                }
            }
        }
        Ok(emitted)
    }
}

fn register_default_backends(vm: &mut Vm, silent: bool) {
    // Agent/visual/net are experimental surfaces (shell/UI/network). Language-only
    // builds omit them so the default binary does not depend on those stacks.
    #[cfg(feature = "agent")]
    if silent {
        vm.register_backend("agent".to_string(), Box::new(SilentAgentBackend));
    } else {
        vm.register_backend("agent".to_string(), Box::new(AgentBackend::new()));
    }

    #[cfg(feature = "visual")]
    if silent {
        vm.register_backend(
            "visual".to_string(),
            Box::new(TraceVisualBackend::new_silent()),
        );
    } else {
        vm.register_backend("visual".to_string(), Box::new(TraceVisualBackend::new()));
    }

    vm.register_backend("graph".to_string(), Box::new(GraphBackend::new()));
    vm.register_backend("store".to_string(), Box::new(StoreBackend::new()));
    #[cfg(feature = "net")]
    vm.register_backend("net".to_string(), Box::new(NetBackend::new()));
    vm.register_backend("energy".to_string(), Box::new(EnergyBackend::new()));
    #[cfg(feature = "device")]
    vm.register_backend("device".to_string(), Box::new(DeviceBackend::new()));
    vm.register_backend("tool".to_string(), Box::new(ToolBackend::new()));
    let _ = silent;
}

fn register_stdlib_backends(vm: &mut Vm, include_extended: bool, fs_policy: FsCapabilityPolicy) {
    // Language-core stdlib (no optional crates).
    vm.register_backend("math".to_string(), Box::new(MathBackend::new()));
    vm.register_backend("string".to_string(), Box::new(StringBackend::new()));
    vm.register_backend("list".to_string(), Box::new(ListBackend::new()));
    vm.register_backend("time".to_string(), Box::new(TimeBackend::new()));
    vm.register_backend("random".to_string(), Box::new(RandomBackend::new()));
    vm.register_backend("json".to_string(), Box::new(JsonBackend::new()));
    vm.register_backend("world".to_string(), Box::new(WorldBackend::new()));
    vm.register_backend("audio".to_string(), Box::new(AudioBackend::new()));
    vm.register_backend("Vec".to_string(), Box::new(VecBackend::new()));
    vm.register_backend("HashMap".to_string(), Box::new(HashMapBackend::new()));
    vm.register_backend("tensor".to_string(), Box::new(TensorBackend::new()));
    vm.register_backend("result".to_string(), Box::new(ResultBackend::new()));
    vm.register_backend("option".to_string(), Box::new(OptionBackend::new()));

    if include_extended {
        vm.register_backend("map".to_string(), Box::new(MapBackend::new()));
        vm.register_backend("type".to_string(), Box::new(TypeBackend::new()));
        vm.register_backend("console".to_string(), Box::new(ConsoleBackend::new()));
        // Shared policy for file.* and fileio.* (File Capabilities v1).
        vm.register_backend(
            "file".to_string(),
            Box::new(FileBackend::with_policy(fs_policy.clone())),
        );
        vm.register_backend(
            "fileio".to_string(),
            Box::new(FileIOBackend::with_policy(fs_policy)),
        );
    }

    #[cfg(feature = "frontier")]
    {
        vm.register_backend("wetware".to_string(), Box::new(WetwareBackend::new()));
        vm.register_backend("quantum".to_string(), Box::new(QuantumBackend::new()));
        vm.register_backend("memristive".to_string(), Box::new(MemristiveBackend::new()));
        vm.register_backend("photonic".to_string(), Box::new(PhotonicBackend::new()));
        vm.register_backend(
            "spintronics".to_string(),
            Box::new(SpintronicsBackend::new()),
        );
        vm.register_backend("molecular".to_string(), Box::new(MolecularBackend::new()));
        vm.register_backend("relativity".to_string(), Box::new(RelativityBackend::new()));
        vm.register_backend("chemistry".to_string(), Box::new(ChemistryBackend::new()));
        vm.register_backend("genesis".to_string(), Box::new(GenesisBackend::new()));
        vm.register_backend("biology".to_string(), Box::new(BiologicalBackend::new()));
        vm.register_backend("biophysics".to_string(), Box::new(BiophysicsBackend));
        vm.register_backend("geophysics".to_string(), Box::new(GeophysicsBackend));
        vm.register_backend("ocean".to_string(), Box::new(OceanBackend));
        vm.register_backend("atmosphere".to_string(), Box::new(AtmosphereBackend));
        vm.register_backend("materials".to_string(), Box::new(MaterialsBackend));
        vm.register_backend("acoustics".to_string(), Box::new(AcousticsBackend));
        vm.register_backend(
            "electromagnetics".to_string(),
            Box::new(ElectromagneticsBackend),
        );
        vm.register_backend("ml_physics".to_string(), Box::new(MLPhysicsBackend));
        vm.register_backend("climate".to_string(), Box::new(ClimateBackend));
        vm.register_backend("multiscale".to_string(), Box::new(MultiscaleBackend));
        vm.register_backend("cosmology".to_string(), Box::new(CosmologyBackend));
        vm.register_backend(
            "neuromorphic".to_string(),
            Box::new(NeuromorphicBackend::new()),
        );
    }
}

#[cfg(feature = "polyglot")]
fn register_polyglot_backends(vm: &mut Vm) {
    vm.register_backend(
        "python".to_string(),
        Box::new(PolyglotBackend::new(
            "Python",
            Box::new(PythonBridge::new()),
        )),
    );
    vm.register_backend(
        "node".to_string(),
        Box::new(PolyglotBackend::new(
            "Node.js",
            Box::new(NodeJSBridge::new()),
        )),
    );
    vm.register_backend(
        "java".to_string(),
        Box::new(PolyglotBackend::new(
            "Java",
            Box::new(JavaLanguageBridge::new()),
        )),
    );
    vm.register_backend(
        "go".to_string(),
        Box::new(PolyglotBackend::new(
            "Go",
            Box::new(GoLanguageBridge::new()),
        )),
    );
    vm.register_backend(
        "rust".to_string(),
        Box::new(PolyglotBackend::new("Rust", Box::new(RustBridge::new()))),
    );
}

#[cfg(feature = "agent")]
struct SilentAgentBackend;

#[cfg(feature = "agent")]
impl Backend for SilentAgentBackend {
    fn call(&mut self, method: &str, _args: Vec<Value>) -> Result<Value, String> {
        match method {
            "say" => Ok(Value::Unit),
            _ => Err(format!(
                "Runtime backend call failed [context:backend=agent,method={}]: unknown method",
                method
            )),
        }
    }
}

#[cfg(feature = "polyglot")]
pub struct PolyglotBackend {
    display_name: String,
    bridge: Box<dyn LanguageBridge>,
    initialized: bool,
    init_error: Option<String>,
}

#[cfg(feature = "polyglot")]
impl PolyglotBackend {
    pub fn new(display_name: impl Into<String>, bridge: Box<dyn LanguageBridge>) -> Self {
        Self {
            display_name: display_name.into(),
            bridge,
            initialized: false,
            init_error: None,
        }
    }

    fn ensure_initialized(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        match self.bridge.initialize() {
            Ok(()) => {
                self.initialized = true;
                self.init_error = None;
                Ok(())
            }
            Err(error) => {
                self.init_error = Some(error.clone());
                Err(error)
            }
        }
    }

    fn status(&mut self) -> Value {
        let ready = self.ensure_initialized().is_ok();
        let mut map = HashMap::new();
        map.insert(
            "backend".to_string(),
            Value::new_string(self.display_name.clone()),
        );
        map.insert("ready".to_string(), Value::Bool(ready));
        map.insert("stub".to_string(), Value::Bool(false));
        map.insert(
            "language".to_string(),
            Value::new_string(self.bridge.language().as_str().to_string()),
        );
        map.insert("mode".to_string(), Value::new_string("real".to_string()));
        if let Some(error) = &self.init_error {
            map.insert("error".to_string(), Value::new_string(error.clone()));
        }
        Value::new_map(map)
    }

    fn import_module(&mut self, args: Vec<Value>) -> Result<Value, String> {
        self.ensure_initialized()?;
        let module = arg_string(&args, 0, "import module")?;
        self.bridge.import_module(&module)?;
        Ok(Value::Unit)
    }

    fn call_function(&mut self, args: Vec<Value>) -> Result<Value, String> {
        self.ensure_initialized()?;
        let module = arg_string(&args, 0, "call module")?;
        let function = arg_string(&args, 1, "call function")?;
        let call_args = if args.len() == 3 {
            match &args[2] {
                Value::List(values) => values.iter().cloned().collect(),
                value => vec![value.clone()],
            }
        } else if args.len() > 3 {
            args.into_iter().skip(2).collect()
        } else {
            Vec::new()
        };
        self.bridge.call_function(&module, &function, call_args)
    }

    fn get_attribute(&mut self, args: Vec<Value>) -> Result<Value, String> {
        self.ensure_initialized()?;
        let module = arg_string(&args, 0, "attribute module")?;
        let attribute = arg_string(&args, 1, "attribute name")?;
        self.bridge.get_attribute(&module, &attribute)
    }
}

#[cfg(feature = "polyglot")]
impl Backend for PolyglotBackend {
    fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        if method == "status" || method == "info" || method == "capabilities" {
            return Ok(self.status());
        }

        match method {
            "import" | "import_module" | "load" => self.import_module(args),
            "call" | "call_function" | "invoke" => self.call_function(args),
            "get" | "get_attribute" | "attr" => self.get_attribute(args),
            other => Err(format!(
                "{} bridge method '{}' is not supported; use status, import(module), call(module, function, args...), or get(module, attribute)",
                self.display_name, other
            )),
        }
    }
}

#[cfg(feature = "polyglot")]
fn arg_string(args: &[Value], index: usize, name: &str) -> Result<String, String> {
    args.get(index)
        .ok_or_else(|| format!("missing {}", name))?
        .as_string()
        .map_err(|e| format!("{} must be a string: {}", name, e))
}

#[cfg(feature = "polyglot")]
struct GoLanguageBridge {
    inner: GoBridge,
}

#[cfg(feature = "polyglot")]
impl GoLanguageBridge {
    fn new() -> Self {
        Self {
            inner: GoBridge::new(),
        }
    }
}

#[cfg(feature = "polyglot")]
impl LanguageBridge for GoLanguageBridge {
    fn language(&self) -> LanguageTarget {
        LanguageTarget::Go
    }

    fn initialize(&mut self) -> Result<(), String> {
        std::process::Command::new("go")
            .arg("version")
            .output()
            .map_err(|e| format!("Go not found: {}", e))
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    Err("Go runtime check failed".to_string())
                }
            })
    }

    fn import_module(&mut self, module: &str) -> Result<(), String> {
        self.inner
            .load_module(module)
            .map_err(|e| format!("{:?}", e))
    }

    fn call_function(
        &mut self,
        module: &str,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        self.inner
            .call(module, function, args)
            .map_err(|e| format!("{:?}", e))
    }

    fn get_attribute(&mut self, module: &str, attribute: &str) -> Result<Value, String> {
        self.inner
            .get_attribute(module, attribute)
            .map_err(|e| format!("{:?}", e))
    }

    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(feature = "polyglot")]
struct JavaLanguageBridge {
    inner: JavaBridge,
}

#[cfg(feature = "polyglot")]
impl JavaLanguageBridge {
    fn new() -> Self {
        Self {
            inner: JavaBridge::new(),
        }
    }
}

#[cfg(feature = "polyglot")]
impl LanguageBridge for JavaLanguageBridge {
    fn language(&self) -> LanguageTarget {
        LanguageTarget::Java
    }

    fn initialize(&mut self) -> Result<(), String> {
        std::process::Command::new("java")
            .arg("-version")
            .output()
            .map_err(|e| format!("Java not found: {}", e))
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    Err("Java runtime check failed".to_string())
                }
            })
    }

    fn import_module(&mut self, module: &str) -> Result<(), String> {
        self.inner
            .load_module(module)
            .map_err(|e| format!("{:?}", e))
    }

    fn call_function(
        &mut self,
        module: &str,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        self.inner
            .call(module, function, args)
            .map_err(|e| format!("{:?}", e))
    }

    fn get_attribute(&mut self, module: &str, attribute: &str) -> Result<Value, String> {
        self.inner
            .get_attribute(module, attribute)
            .map_err(|e| format!("{:?}", e))
    }

    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matter_bytecode::{Constant, EventHandler, Instruction};
    use matter_energy::{EnergyMonitor, EnergySnapshot};
    use std::collections::HashMap;
    use std::fs;

    struct LowEnergyMonitor;

    impl EnergyMonitor for LowEnergyMonitor {
        fn snapshot(&self) -> EnergySnapshot {
            EnergySnapshot {
                cpu_usage: 10.0,
                memory_usage: 20.0,
                io_usage: 5.0,
                network_usage: 5.0,
                battery_level: 50.0,
                temperature: 40.0,
                timestamp: 0,
            }
        }
    }

    struct LowBatteryMonitor;

    impl EnergyMonitor for LowBatteryMonitor {
        fn snapshot(&self) -> EnergySnapshot {
            EnergySnapshot {
                cpu_usage: 35.0,
                memory_usage: 40.0,
                io_usage: 10.0,
                network_usage: 10.0,
                battery_level: 8.0,
                temperature: 45.0,
                timestamp: 0,
            }
        }
    }

    #[test]
    #[cfg(feature = "visual")]
    fn bridge_visual_events_dispatches_and_emits_into_vm() {
        let path = std::env::temp_dir().join("matter_runtime_bridge_events_test.json");
        fs::write(
            &path,
            r#"{"format":"PXL_EVENT_QUEUE","version":1,"events":[{"type":"pointer","target":"button","event":"button_tap"}]}"#,
        )
        .unwrap();

        let mut bytecode = Bytecode::new();
        let ok_const = bytecode.add_constant(Constant::String("ok".to_string()));
        bytecode.event_handlers.insert(
            "button_tap".to_string(),
            EventHandler {
                event: "button_tap".to_string(),
                instructions: vec![
                    Instruction::LoadConst(ok_const),
                    Instruction::StoreGlobal("last_visual_event".to_string()),
                    Instruction::Halt,
                ],
            },
        );

        let mut runtime = Runtime::new_silent(bytecode);
        runtime
            .bridge_visual_events(path.to_str().unwrap_or_default())
            .unwrap();

        let globals = runtime.get_globals();
        assert_eq!(
            globals.get("last_visual_event"),
            Some(&Value::new_string("ok".to_string()))
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    #[cfg(feature = "visual")]
    fn visual_app_step_advances_loop_and_emits_events_into_vm() {
        let path = std::env::temp_dir().join("matter_runtime_visual_app_step_test.json");
        fs::write(
            &path,
            r#"{"format":"PXL_EVENT_QUEUE","version":1,"events":[{"type":"pointer","target":"play","event":"play_tap"}]}"#,
        )
        .unwrap();

        let mut bytecode = Bytecode::new();
        let fired_const = bytecode.add_constant(Constant::String("fired".to_string()));
        bytecode.event_handlers.insert(
            "play_tap".to_string(),
            EventHandler {
                event: "play_tap".to_string(),
                instructions: vec![
                    Instruction::LoadConst(fired_const),
                    Instruction::StoreGlobal("last_visual_step_event".to_string()),
                    Instruction::Halt,
                ],
            },
        );

        let mut runtime = Runtime::new_silent(bytecode);
        runtime
            .visual_app_step(path.to_str().unwrap_or_default(), 16)
            .unwrap();

        let globals = runtime.get_globals();
        assert_eq!(
            globals.get("last_visual_step_event"),
            Some(&Value::new_string("fired".to_string()))
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    #[cfg(feature = "agent")]
    fn silent_agent_backend_unknown_method_uses_context_contract() {
        let mut backend = SilentAgentBackend;
        let err = backend
            .call("do_magic", vec![])
            .expect_err("expected unknown method error");

        assert!(err.starts_with("Runtime backend call failed"));
        assert!(err.contains("[context:backend=agent,method=do_magic]"));
        assert!(err.contains("unknown method"));
    }

    #[test]
    fn emits_energy_low_event_when_snapshot_is_low() {
        let mut bytecode = Bytecode::new();
        let ok_const = bytecode.add_constant(Constant::String("ok".to_string()));
        bytecode.event_handlers.insert(
            "energy.low".to_string(),
            EventHandler {
                event: "energy.low".to_string(),
                instructions: vec![
                    Instruction::LoadConst(ok_const),
                    Instruction::StoreGlobal("energy_low_seen".to_string()),
                    Instruction::Halt,
                ],
            },
        );

        let mut runtime = Runtime::new_silent(bytecode);
        runtime.set_energy_runtime(Some(EnergyRuntime::with_monitor(Box::new(
            LowEnergyMonitor,
        ))));
        runtime.run().unwrap();

        let globals = runtime.get_globals();
        assert_eq!(
            globals.get("energy_low_seen"),
            Some(&Value::new_string("ok".to_string()))
        );
    }

    #[test]
    fn emits_battery_low_event() {
        let mut bytecode = Bytecode::new();
        let ok_const = bytecode.add_constant(Constant::String("ok".to_string()));
        bytecode.event_handlers.insert(
            "battery.low".to_string(),
            EventHandler {
                event: "battery.low".to_string(),
                instructions: vec![
                    Instruction::LoadConst(ok_const),
                    Instruction::StoreGlobal("battery_low_seen".to_string()),
                    Instruction::Halt,
                ],
            },
        );

        let mut runtime = Runtime::new_silent(bytecode);
        runtime.set_energy_runtime(Some(EnergyRuntime::with_monitor(Box::new(
            LowBatteryMonitor,
        ))));
        runtime.run().unwrap();

        let globals = runtime.get_globals();
        assert_eq!(
            globals.get("battery_low_seen"),
            Some(&Value::new_string("ok".to_string()))
        );
    }

    #[test]
    fn tool_invoke_frame_emits_blocked_event() {
        let mut bytecode = Bytecode::new();
        let ok_const = bytecode.add_constant(Constant::String("ok".to_string()));
        bytecode.event_handlers.insert(
            "tool.blocked".to_string(),
            EventHandler {
                event: "tool.blocked".to_string(),
                instructions: vec![
                    Instruction::LoadConst(ok_const),
                    Instruction::StoreGlobal("tool_blocked_seen".to_string()),
                    Instruction::Halt,
                ],
            },
        );

        let mut payload = HashMap::new();
        payload.insert(
            "from_name".to_string(),
            Value::new_string("planner".to_string()),
        );
        payload.insert(
            "from_role".to_string(),
            Value::new_string("planner".to_string()),
        );
        payload.insert(
            "to_name".to_string(),
            Value::new_string("worker".to_string()),
        );
        payload.insert(
            "to_role".to_string(),
            Value::new_string("worker".to_string()),
        );
        payload.insert(
            "task_id".to_string(),
            Value::new_string("tool-evt-1".to_string()),
        );
        payload.insert(
            "state".to_string(),
            Value::new_string("blocked".to_string()),
        );
        payload.insert(
            "goal".to_string(),
            Value::new_string("invoke blocked frame".to_string()),
        );
        payload.insert(
            "summary".to_string(),
            Value::new_string("blocked test".to_string()),
        );
        payload.insert(
            "next_action".to_string(),
            Value::new_string("resolve-blockers".to_string()),
        );
        payload.insert(
            "blockers".to_string(),
            Value::new_list(vec![Value::new_string("missing_token".to_string())]),
        );

        let mut runtime = Runtime::new_silent(bytecode);
        runtime.tool_invoke_frame(Value::new_map(payload)).unwrap();

        let globals = runtime.get_globals();
        assert_eq!(
            globals.get("tool_blocked_seen"),
            Some(&Value::new_string("ok".to_string()))
        );
    }

    #[test]
    fn tool_invoke_frame_emits_accepted_event() {
        let mut bytecode = Bytecode::new();
        let ok_const = bytecode.add_constant(Constant::String("ok".to_string()));
        bytecode.event_handlers.insert(
            "tool.accepted".to_string(),
            EventHandler {
                event: "tool.accepted".to_string(),
                instructions: vec![
                    Instruction::LoadConst(ok_const),
                    Instruction::StoreGlobal("tool_accepted_seen".to_string()),
                    Instruction::Halt,
                ],
            },
        );

        let mut payload = HashMap::new();
        payload.insert(
            "from_name".to_string(),
            Value::new_string("planner".to_string()),
        );
        payload.insert(
            "from_role".to_string(),
            Value::new_string("planner".to_string()),
        );
        payload.insert(
            "to_name".to_string(),
            Value::new_string("worker".to_string()),
        );
        payload.insert(
            "to_role".to_string(),
            Value::new_string("worker".to_string()),
        );
        payload.insert(
            "task_id".to_string(),
            Value::new_string("tool-evt-2".to_string()),
        );
        payload.insert(
            "state".to_string(),
            Value::new_string("proposed".to_string()),
        );
        payload.insert(
            "goal".to_string(),
            Value::new_string("invoke accepted frame".to_string()),
        );
        payload.insert(
            "summary".to_string(),
            Value::new_string("accepted test".to_string()),
        );
        payload.insert(
            "next_action".to_string(),
            Value::new_string("execute".to_string()),
        );

        let mut runtime = Runtime::new_silent(bytecode);
        runtime.tool_invoke_frame(Value::new_map(payload)).unwrap();

        let globals = runtime.get_globals();
        assert_eq!(
            globals.get("tool_accepted_seen"),
            Some(&Value::new_string("ok".to_string()))
        );
    }

    #[cfg(feature = "polyglot")]
    struct TestLanguageBridge {
        initialized: bool,
        imported: Vec<String>,
    }

    #[cfg(feature = "polyglot")]
    impl TestLanguageBridge {
        fn new() -> Self {
            Self {
                initialized: false,
                imported: Vec::new(),
            }
        }
    }

    #[cfg(feature = "polyglot")]
    impl LanguageBridge for TestLanguageBridge {
        fn language(&self) -> LanguageTarget {
            LanguageTarget::Python
        }

        fn initialize(&mut self) -> Result<(), String> {
            self.initialized = true;
            Ok(())
        }

        fn import_module(&mut self, module: &str) -> Result<(), String> {
            self.imported.push(module.to_string());
            Ok(())
        }

        fn call_function(
            &mut self,
            module: &str,
            function: &str,
            args: Vec<Value>,
        ) -> Result<Value, String> {
            Ok(Value::new_string(format!(
                "{}.{}({})",
                module,
                function,
                args.len()
            )))
        }

        fn get_attribute(&mut self, module: &str, attribute: &str) -> Result<Value, String> {
            Ok(Value::new_string(format!("{}.{}", module, attribute)))
        }

        fn shutdown(&mut self) -> Result<(), String> {
            self.initialized = false;
            Ok(())
        }
    }

    #[test]
    #[cfg(feature = "polyglot")]
    fn polyglot_backend_status_reports_real_mode() {
        let mut backend = PolyglotBackend::new("Python", Box::new(TestLanguageBridge::new()));
        let status = backend.call("status", vec![]).unwrap();

        let map = match status {
            Value::Map(map) => map,
            _ => panic!("status must return a map"),
        };

        assert_eq!(map.get("ready"), Some(&Value::Bool(true)));
        assert_eq!(map.get("stub"), Some(&Value::Bool(false)));
        assert_eq!(
            map.get("mode"),
            Some(&Value::new_string("real".to_string()))
        );
        assert!(matches!(map.get("backend"), Some(Value::String(_))));
    }

    #[test]
    #[cfg(feature = "polyglot")]
    fn polyglot_backend_import_and_call_dispatch_to_bridge() {
        let mut backend = PolyglotBackend::new("Python", Box::new(TestLanguageBridge::new()));
        backend
            .call("import", vec![Value::new_string("math".to_string())])
            .unwrap();
        let output = backend
            .call(
                "call",
                vec![
                    Value::new_string("math".to_string()),
                    Value::new_string("sqrt".to_string()),
                    Value::new_list(vec![Value::Int(16)]),
                ],
            )
            .unwrap()
            .as_string()
            .expect("call should return string");

        assert_eq!(output, "math.sqrt(1)");
    }

    #[test]
    #[cfg(feature = "frontier")]
    fn frontier_backends_execute_through_runtime() {
        let mut runtime = Runtime::new_silent(Bytecode::new());

        for backend in ["quantum", "photonic", "neuromorphic", "wetware"] {
            let status = runtime.call_backend(backend, "status", vec![]).unwrap();
            let Value::Map(map) = status else {
                panic!("{backend}.status should return a map");
            };
            assert_eq!(map.get("stub"), Some(&Value::Bool(false)));
            assert_eq!(map.get("hardware"), Some(&Value::Bool(false)));
            assert_eq!(map.get("simulated"), Some(&Value::Bool(true)));
            assert!(matches!(map.get("capabilities"), Some(Value::List(_))));
        }

        let bell = runtime
            .call_backend("quantum", "bell_state", vec![])
            .unwrap();
        let Value::List(bits) = bell else {
            panic!("quantum.bell_state should return measured bits");
        };
        assert_eq!(bits.len(), 2);
        assert!(bits.iter().all(|bit| matches!(bit, Value::Int(0 | 1))));

        let photonic_and = runtime
            .call_backend(
                "photonic",
                "and",
                vec![Value::Float(0.9), Value::Float(0.8)],
            )
            .unwrap();
        assert_eq!(photonic_and, Value::Float(1.0));

        runtime
            .call_backend(
                "neuromorphic",
                "init",
                vec![Value::Int(3), Value::Float(0.1)],
            )
            .unwrap();
        runtime
            .call_backend(
                "neuromorphic",
                "add_synapse",
                vec![Value::Int(0), Value::Int(1), Value::Float(0.5)],
            )
            .unwrap();
        let spikes = runtime
            .call_backend(
                "neuromorphic",
                "step",
                vec![Value::new_list(vec![
                    Value::Float(20.0),
                    Value::Float(0.0),
                    Value::Float(0.0),
                ])],
            )
            .unwrap();
        assert!(matches!(spikes, Value::List(_)));

        let wetware_response = runtime
            .call_backend(
                "wetware",
                "stimulate",
                vec![Value::new_list(vec![
                    Value::Bool(true),
                    Value::Bool(false),
                    Value::Bool(true),
                ])],
            )
            .unwrap();
        let Value::List(response) = wetware_response else {
            panic!("wetware.stimulate should return a spike response list");
        };
        assert_eq!(response.len(), 3);
    }

    #[test]
    fn world_backend_executes_through_runtime() {
        let mut runtime = Runtime::new_silent(Bytecode::new());
        runtime
            .call_backend(
                "world",
                "configure",
                vec![
                    Value::Float(100.0),
                    Value::Float(150.0),
                    Value::Int(2),
                    Value::Int(2),
                ],
            )
            .unwrap();

        for (id, x, y) in [
            ("p1", 10.0, 10.0),
            ("p2", 15.0, 15.0),
            ("p3", 20.0, 20.0),
            ("p4", 120.0, 10.0),
        ] {
            runtime
                .call_backend(
                    "world",
                    "place",
                    vec![
                        Value::new_string(id.to_string()),
                        Value::Float(x),
                        Value::Float(y),
                    ],
                )
                .unwrap();
        }

        let nearby = runtime
            .call_backend("world", "nearby", vec![Value::new_string("p1".to_string())])
            .unwrap();
        let Value::Map(nearby_map) = nearby else {
            panic!("world.nearby should return a map");
        };
        assert_eq!(nearby_map.get("visible_count"), Some(&Value::Int(2)));
        assert_eq!(nearby_map.get("hidden_count"), Some(&Value::Int(1)));

        let plan = runtime.call_backend("world", "plan", vec![]).unwrap();
        let Value::Map(plan_map) = plan else {
            panic!("world.plan should return a map");
        };
        assert_eq!(plan_map.get("entities"), Some(&Value::Int(4)));
        assert_eq!(plan_map.get("hot_cells"), Some(&Value::Int(1)));
        assert_eq!(plan_map.get("degraded"), Some(&Value::Bool(true)));
    }
}
