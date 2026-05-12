//! Matter Hot Code Reloading
//!
//! Permite atualização de código em tempo real sem reiniciar o programa.
//!
//! ## Funcionalidades
//!
//! - File watching automático
//! - Recompilação incremental
//! - State preservation
//! - Event hooks (on_reload)
//! - Zero downtime
//!
//! ## Exemplo
//!
//! ```matter
//! let counter = 0;
//!
//! on http_request {
//!     set counter = counter + 1;
//!     print "Request #" + counter;
//! }
//!
//! on code_reload {
//!     print "Código atualizado! Counter: " + counter;
//! }
//! ```

use crossbeam_channel::{unbounded, Receiver};
use matter_runtime::Runtime;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

/// Hot reload manager
pub struct HotReloadManager {
    /// Path to watch
    watch_path: PathBuf,

    /// File watcher
    watcher: Option<Box<dyn Watcher>>,

    /// Event receiver
    receiver: Option<Receiver<notify::Result<Event>>>,

    /// Current runtime
    runtime: Rc<RefCell<Option<Runtime>>>,

    /// Reload callback
    on_reload: Option<ReloadCallback>,
}

type ReloadCallback = Box<dyn Fn(&Runtime)>;

impl HotReloadManager {
    /// Create a new hot reload manager
    pub fn new<P: AsRef<Path>>(watch_path: P) -> Result<Self, String> {
        Ok(Self {
            watch_path: watch_path.as_ref().to_path_buf(),
            watcher: None,
            receiver: None,
            runtime: Rc::new(RefCell::new(None)),
            on_reload: None,
        })
    }

    /// Set reload callback
    pub fn on_reload<F>(&mut self, callback: F)
    where
        F: Fn(&Runtime) + 'static,
    {
        self.on_reload = Some(Box::new(callback));
    }

    /// Start watching for changes
    pub fn start(&mut self) -> Result<(), String> {
        let (tx, rx) = unbounded();

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        watcher
            .watch(&self.watch_path, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path: {}", e))?;

        self.watcher = Some(Box::new(watcher));
        self.receiver = Some(rx);

        Ok(())
    }

    /// Stop watching
    pub fn stop(&mut self) {
        self.watcher = None;
        self.receiver = None;
    }

    /// Check for changes and reload if needed
    pub fn check_and_reload(&mut self) -> Result<bool, String> {
        let receiver = match &self.receiver {
            Some(rx) => rx,
            None => return Ok(false),
        };

        // Check for events (non-blocking)
        match receiver.try_recv() {
            Ok(Ok(event)) => {
                // Check if it's a modify event
                if matches!(event.kind, EventKind::Modify(_)) {
                    // Check if it's a .matter file
                    if let Some(path) = event.paths.first() {
                        if path.extension().and_then(|s| s.to_str()) == Some("matter") {
                            self.reload(path)?;
                            return Ok(true);
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                return Err(format!("Watch error: {}", e));
            }
            Err(_) => {
                // No events, continue
            }
        }

        Ok(false)
    }

    /// Reload code from file
    fn reload(&mut self, path: &Path) -> Result<(), String> {
        println!("🔄 Reloading: {}", path.display());

        // Read source code
        let source =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        // Parse and compile
        let mut parser = matter_parser::Parser::from_source(&source);
        let program = parser
            .parse()
            .map_err(|e| format!("Parse error: {:?}", e))?;

        let builder = matter_bytecode::BytecodeBuilder::new();
        let bytecode = builder
            .build_checked(&program)
            .map_err(|e| format!("Compile error: {:?}", e))?;

        // Get current runtime state
        let mut runtime_lock = self.runtime.borrow_mut();

        if let Some(old_runtime) = runtime_lock.as_ref() {
            // Preserve state
            let globals = old_runtime.get_globals();

            // Create new runtime with preserved state
            let mut new_runtime = Runtime::new(bytecode);
            new_runtime.set_globals(globals);

            // Replace runtime
            *runtime_lock = Some(new_runtime);

            if let Some(runtime) = runtime_lock.as_mut() {
                let _ = runtime.emit_event("code_reload");
            }

            // Call reload callback
            if let Some(callback) = &self.on_reload {
                if let Some(runtime) = runtime_lock.as_ref() {
                    callback(runtime);
                }
            }

            println!("✅ Reload complete!");
        } else {
            // First load
            *runtime_lock = Some(Runtime::new(bytecode));
            println!("✅ Initial load complete!");
        }

        Ok(())
    }

    /// Get current runtime
    pub fn runtime(&self) -> Rc<RefCell<Option<Runtime>>> {
        Rc::clone(&self.runtime)
    }

    /// Run with hot reload enabled
    pub fn run_with_reload<P: AsRef<Path>>(
        path: P,
        check_interval: Duration,
    ) -> Result<(), String> {
        let watch_root = path
            .as_ref()
            .parent()
            .ok_or_else(|| "Invalid path: missing parent directory".to_string())?;
        let mut manager = HotReloadManager::new(watch_root)?;

        // Set reload callback
        manager.on_reload(|runtime| {
            let _ = runtime;
        });

        // Start watching
        manager.start()?;

        // Initial load
        manager.reload(path.as_ref())?;

        // Main loop
        loop {
            // Check for changes
            manager.check_and_reload()?;

            // Run runtime
            let runtime_lock = manager.runtime();
            if let Some(runtime) = runtime_lock.borrow_mut().as_mut() {
                if let Err(e) = runtime.run() {
                    eprintln!("Runtime error: {:?}", e);
                }
            }

            // Sleep
            std::thread::sleep(check_interval);
        }
    }
}

/// Hot reload configuration
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    /// Enable hot reload
    pub enabled: bool,

    /// Check interval (milliseconds)
    pub check_interval_ms: u64,

    /// Preserve state on reload
    pub preserve_state: bool,

    /// Trigger reload event
    pub trigger_event: bool,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_ms: 100,
            preserve_state: true,
            trigger_event: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hot_reload_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = HotReloadManager::new(temp_dir.path());
        assert!(manager.is_ok());
    }

    #[test]
    fn test_hot_reload_config_default() {
        let config = HotReloadConfig::default();
        assert!(config.enabled);
        assert_eq!(config.check_interval_ms, 100);
        assert!(config.preserve_state);
        assert!(config.trigger_event);
    }
}
