//! Matter Rust Bridge - FFI direto para Rust crates
//!
//! Permite importar e usar crates Rust compilados como bibliotecas dinâmicas

use matter_backend::Value;
use matter_polyglot::{bridge::LanguageBridge, LanguageTarget};
use std::collections::HashMap;

/// Bridge para Rust via FFI direto
///
/// Nota: Esta é uma implementação simplificada que assume que os crates
/// Rust foram compilados como bibliotecas dinâmicas (.so/.dll/.dylib)
pub struct RustBridge {
    loaded_crates: HashMap<String, String>,
    initialized: bool,
}

impl RustBridge {
    pub fn new() -> Self {
        Self {
            loaded_crates: HashMap::new(),
            initialized: false,
        }
    }

    /// Verifica se um crate Rust está disponível
    fn check_crate(&self, crate_name: &str) -> Result<(), String> {
        // Por enquanto, apenas registra o crate
        // Uma implementação completa carregaria a biblioteca dinâmica
        println!(
            "Rust crate '{}' will be linked during compilation",
            crate_name
        );
        Ok(())
    }
}

impl Default for RustBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageBridge for RustBridge {
    fn language(&self) -> LanguageTarget {
        LanguageTarget::Rust
    }

    fn initialize(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        // Verifica se cargo está instalado
        let output = std::process::Command::new("cargo")
            .arg("--version")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("Cargo version: {}", version.trim());
                self.initialized = true;
                Ok(())
            }
            _ => Err("Cargo not found. Please install Rust toolchain.".to_string()),
        }
    }

    fn import_module(&mut self, module: &str) -> Result<(), String> {
        self.check_crate(module)?;
        self.loaded_crates
            .insert(module.to_string(), module.to_string());
        Ok(())
    }

    fn call_function(
        &mut self,
        module: &str,
        function: &str,
        _args: Vec<Value>,
    ) -> Result<Value, String> {
        if !self.loaded_crates.contains_key(module) {
            return Err(format!("Crate '{}' not imported", module));
        }

        // Implementação simplificada
        // Uma implementação completa usaria libloading para chamar funções via FFI
        Err(format!(
            "Rust FFI call not yet implemented: {}::{}",
            module, function
        ))
    }

    fn get_attribute(&mut self, module: &str, attribute: &str) -> Result<Value, String> {
        if !self.loaded_crates.contains_key(module) {
            return Err(format!("Crate '{}' not imported", module));
        }

        Err(format!(
            "Rust attribute access not yet implemented: {}::{}",
            module, attribute
        ))
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.loaded_crates.clear();
        self.initialized = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_bridge_initialization() {
        let mut bridge = RustBridge::new();
        // Só testa se cargo está instalado
        let _ = bridge.initialize();
    }

    #[test]
    fn test_import_crate() {
        let mut bridge = RustBridge::new();
        bridge.initialize().ok();

        // Testa importação (apenas registra)
        let result = bridge.import_module("serde");
        assert!(result.is_ok());
        assert!(bridge.loaded_crates.contains_key("serde"));
    }
}
