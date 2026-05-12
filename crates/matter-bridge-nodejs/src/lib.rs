//! Matter Node.js Bridge - FFI para Node.js/JavaScript
//!
//! Permite importar e usar bibliotecas Node.js diretamente em Matter

use matter_backend::Value;
use matter_polyglot::{bridge::LanguageBridge, LanguageTarget};
use std::collections::HashMap;
use std::process::{Command, Stdio};

pub mod converter;

use converter::JavaScriptTypeConverter;

/// Bridge para Node.js via subprocess (implementação simplificada)
///
/// Nota: Uma implementação completa usaria napi-rs para FFI direto,
/// mas esta versão usa subprocess para prototipagem rápida.
pub struct NodeJSBridge {
    modules: HashMap<String, String>,
    converter: JavaScriptTypeConverter,
    initialized: bool,
    node_path: String,
}

impl NodeJSBridge {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            converter: JavaScriptTypeConverter,
            initialized: false,
            node_path: "node".to_string(),
        }
    }

    /// Executa código JavaScript e retorna o resultado
    fn execute_js(&self, code: &str) -> Result<String, String> {
        let child = Command::new(&self.node_path)
            .arg("-e")
            .arg(code)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn node process: {}", e))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for node process: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!(
                "Node.js execution failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    /// Gera código JavaScript para importar módulo
    fn generate_import_code(&self, module: &str) -> String {
        format!(
            r#"
const module = require('{}');
console.log(JSON.stringify({{
    type: 'module',
    name: '{}',
    exports: Object.keys(module)
}}));
"#,
            module, module
        )
    }

    /// Gera código JavaScript para chamar função
    fn generate_call_code(&self, module: &str, function: &str, args: &str) -> String {
        format!(
            r#"
const module = require('{}');
const result = module.{}({});
console.log(JSON.stringify(result));
"#,
            module, function, args
        )
    }
}

impl Default for NodeJSBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageBridge for NodeJSBridge {
    fn language(&self) -> LanguageTarget {
        LanguageTarget::NodeJS
    }

    fn initialize(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        // Verifica se Node.js está instalado
        let output = Command::new(&self.node_path)
            .arg("--version")
            .output()
            .map_err(|e| format!("Node.js not found. Please install Node.js: {}", e))?;

        if !output.status.success() {
            return Err("Node.js not found. Please install Node.js.".to_string());
        }

        let version = String::from_utf8_lossy(&output.stdout);
        println!("Node.js version: {}", version.trim());

        self.initialized = true;
        Ok(())
    }

    fn import_module(&mut self, module: &str) -> Result<(), String> {
        let code = self.generate_import_code(module);
        let output = self.execute_js(&code)?;

        // Verifica se o módulo foi importado com sucesso
        if output.contains("\"type\":\"module\"") {
            self.modules.insert(module.to_string(), module.to_string());
            Ok(())
        } else {
            Err(format!("Failed to import Node.js module '{}'", module))
        }
    }

    fn call_function(
        &mut self,
        module: &str,
        function: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        // Verifica se o módulo foi importado
        if !self.modules.contains_key(module) {
            return Err(format!("Module '{}' not imported", module));
        }

        // Converte argumentos para JSON
        let args_json = self.converter.args_to_json(&args)?;

        // Gera e executa código JavaScript
        let code = self.generate_call_code(module, function, &args_json);
        let output = self.execute_js(&code)?;

        // Converte resultado de JSON para Matter
        self.converter.json_to_matter(&output)
    }

    fn get_attribute(&mut self, module: &str, attribute: &str) -> Result<Value, String> {
        // Verifica se o módulo foi importado
        if !self.modules.contains_key(module) {
            return Err(format!("Module '{}' not imported", module));
        }

        // Gera código para obter atributo
        let code = format!(
            r#"
const module = require('{}');
console.log(JSON.stringify(module.{}));
"#,
            module, attribute
        );

        let output = self.execute_js(&code)?;
        self.converter.json_to_matter(&output)
    }

    fn shutdown(&mut self) -> Result<(), String> {
        self.modules.clear();
        self.initialized = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nodejs_bridge_initialization() {
        let mut bridge = NodeJSBridge::new();
        // Só testa se Node.js está instalado
        let _ = bridge.initialize();
    }

    #[test]
    fn test_execute_simple_js() {
        let bridge = NodeJSBridge::new();
        let result = bridge.execute_js("console.log('Hello from Node.js')");
        // Pode falhar se Node.js não estiver instalado, mas não é erro crítico
        if result.is_ok() {
            assert!(result.unwrap().contains("Hello"));
        }
    }
}
