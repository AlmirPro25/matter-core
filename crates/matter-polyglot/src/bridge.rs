//! Bridge abstraction para diferentes linguagens

use crate::LanguageTarget;
use matter_backend::Value;

pub trait LanguageBridge {
    fn language(&self) -> LanguageTarget;
    fn initialize(&mut self) -> Result<(), String>;
    fn import_module(&mut self, module: &str) -> Result<(), String>;
    fn call_function(
        &mut self,
        module: &str,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Value, String>;
    fn get_attribute(&mut self, module: &str, attribute: &str) -> Result<Value, String>;
    fn shutdown(&mut self) -> Result<(), String>;
}

pub struct BridgeRegistry {
    bridges: std::collections::HashMap<LanguageTarget, Box<dyn LanguageBridge>>,
}

impl BridgeRegistry {
    pub fn new() -> Self {
        Self {
            bridges: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, bridge: Box<dyn LanguageBridge>) {
        let language = bridge.language();
        self.bridges.insert(language, bridge);
    }

    pub fn get(&mut self, language: LanguageTarget) -> Option<&mut Box<dyn LanguageBridge>> {
        self.bridges.get_mut(&language)
    }

    pub fn initialize_all(&mut self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for (lang, bridge) in &mut self.bridges {
            if let Err(e) = bridge.initialize() {
                errors.push(format!("{:?} bridge initialization failed: {}", lang, e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn shutdown_all(&mut self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for (lang, bridge) in &mut self.bridges {
            if let Err(e) = bridge.shutdown() {
                errors.push(format!("{:?} bridge shutdown failed: {}", lang, e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for BridgeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
