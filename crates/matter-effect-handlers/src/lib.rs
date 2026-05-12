//! Matter Effect Handlers
//!
//! Sistema de handlers que permite interceptar e modificar efeitos em runtime.
//!
//! ## Funcionalidades
//!
//! - Interceptação de efeitos
//! - Modificação de comportamento
//! - Composição de handlers
//! - Handlers customizados
//! - Zero overhead quando não usado
//!
//! ## Exemplo
//!
//! ```matter
//! // Handler de logging
//! handler logging {
//!     on io.print(msg) {
//!         file.write("log.txt", msg);
//!         resume;
//!     }
//! }
//!
//! // Handler de retry
//! handler retry {
//!     on network.get(url) {
//!         let result = try_operation();
//!         if result.is_error() {
//!             retry(3);
//!         }
//!         return result;
//!     }
//! }
//!
//! // Usar handlers
//! with logging, retry {
//!     let data = net.get("https://api.example.com");
//!     print data;
//! }
//! ```

use std::collections::HashMap;
use std::fmt;

type HandlerFn = dyn Fn(&[HandlerValue]) -> HandlerAction;
type HandlerMap = HashMap<EffectOperation, Box<HandlerFn>>;

/// Handler action
#[derive(Debug, Clone, PartialEq)]
pub enum HandlerAction {
    /// Resume with original behavior
    Resume,

    /// Return a value
    Return(HandlerValue),

    /// Retry the operation
    Retry { max_attempts: usize },

    /// Abort the operation
    Abort { reason: String },

    /// Delegate to another handler
    Delegate { handler: String },
}

/// Handler value
#[derive(Debug, Clone, PartialEq)]
pub enum HandlerValue {
    Unit,
    Int(i64),
    Bool(bool),
    String(String),
    List(Vec<HandlerValue>),
    Map(HashMap<String, HandlerValue>),
}

impl fmt::Display for HandlerValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandlerValue::Unit => write!(f, "unit"),
            HandlerValue::Int(n) => write!(f, "{}", n),
            HandlerValue::Bool(b) => write!(f, "{}", b),
            HandlerValue::String(s) => write!(f, "\"{}\"", s),
            HandlerValue::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            HandlerValue::Map(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

/// Effect operation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectOperation {
    /// Effect name (e.g., "io", "db", "network")
    pub effect: String,

    /// Operation name (e.g., "print", "query", "get")
    pub operation: String,
}

impl EffectOperation {
    /// Create a new effect operation
    pub fn new(effect: String, operation: String) -> Self {
        Self { effect, operation }
    }

    /// Get full name (e.g., "io.print")
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.effect, self.operation)
    }
}

impl fmt::Display for EffectOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full_name())
    }
}

/// Handler definition
pub struct Handler {
    /// Handler name
    pub name: String,

    /// Operations this handler intercepts
    pub operations: HandlerMap,
}

impl Handler {
    /// Create a new handler
    pub fn new(name: String) -> Self {
        Self {
            name,
            operations: HashMap::new(),
        }
    }

    /// Add an operation handler
    pub fn on<F>(&mut self, operation: EffectOperation, handler: F)
    where
        F: Fn(&[HandlerValue]) -> HandlerAction + 'static,
    {
        self.operations.insert(operation, Box::new(handler));
    }

    /// Handle an operation
    pub fn handle(
        &self,
        operation: &EffectOperation,
        args: &[HandlerValue],
    ) -> Option<HandlerAction> {
        self.operations.get(operation).map(|f| f(args))
    }
}

impl fmt::Debug for Handler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handler")
            .field("name", &self.name)
            .field("operations", &self.operations.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Handler registry
pub struct HandlerRegistry {
    /// Registered handlers
    handlers: HashMap<String, Handler>,

    /// Active handler stack
    active_stack: Vec<String>,
}

impl HandlerRegistry {
    /// Create a new handler registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            active_stack: Vec::new(),
        }
    }

    /// Register a handler
    pub fn register(&mut self, handler: Handler) {
        self.handlers.insert(handler.name.clone(), handler);
    }

    /// Activate a handler
    pub fn activate(&mut self, name: &str) -> Result<(), String> {
        if !self.handlers.contains_key(name) {
            return Err(format!("Handler '{}' not found", name));
        }
        self.active_stack.push(name.to_string());
        Ok(())
    }

    /// Deactivate the last handler
    pub fn deactivate(&mut self) -> Option<String> {
        self.active_stack.pop()
    }

    /// Handle an operation
    pub fn handle(
        &self,
        operation: &EffectOperation,
        args: &[HandlerValue],
    ) -> Option<HandlerAction> {
        // Try handlers in reverse order (most recent first)
        for handler_name in self.active_stack.iter().rev() {
            if let Some(handler) = self.handlers.get(handler_name) {
                if let Some(action) = handler.handle(operation, args) {
                    return Some(action);
                }
            }
        }
        None
    }

    /// Get active handlers
    pub fn active_handlers(&self) -> &[String] {
        &self.active_stack
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in handlers
pub mod builtin {
    use super::*;

    /// Logging handler - logs all IO operations
    pub fn logging_handler() -> Handler {
        let mut handler = Handler::new("logging".to_string());

        // Intercept io.print
        handler.on(
            EffectOperation::new("io".to_string(), "print".to_string()),
            |args| {
                println!("[LOG] print: {:?}", args);
                HandlerAction::Resume
            },
        );

        handler
    }

    /// Tracing handler - traces all operations
    pub fn tracing_handler() -> Handler {
        let mut handler = Handler::new("tracing".to_string());

        // Intercept all operations (would need pattern matching in real impl)
        handler.on(
            EffectOperation::new("io".to_string(), "print".to_string()),
            |args| {
                println!("[TRACE] io.print({:?})", args);
                HandlerAction::Resume
            },
        );

        handler.on(
            EffectOperation::new("db".to_string(), "query".to_string()),
            |args| {
                println!("[TRACE] db.query({:?})", args);
                HandlerAction::Resume
            },
        );

        handler.on(
            EffectOperation::new("network".to_string(), "get".to_string()),
            |args| {
                println!("[TRACE] network.get({:?})", args);
                HandlerAction::Resume
            },
        );

        handler
    }

    /// Retry handler - retries failed operations
    pub fn retry_handler(max_attempts: usize) -> Handler {
        let mut handler = Handler::new("retry".to_string());

        // Intercept network operations
        handler.on(
            EffectOperation::new("network".to_string(), "get".to_string()),
            move |_args| HandlerAction::Retry { max_attempts },
        );

        handler.on(
            EffectOperation::new("network".to_string(), "post".to_string()),
            move |_args| HandlerAction::Retry { max_attempts },
        );

        handler
    }

    /// Mock handler - mocks operations for testing
    pub fn mock_handler() -> Handler {
        let mut handler = Handler::new("mock".to_string());

        // Mock io.print (do nothing)
        handler.on(
            EffectOperation::new("io".to_string(), "print".to_string()),
            |_args| HandlerAction::Return(HandlerValue::Unit),
        );

        // Mock network.get (return fake data)
        handler.on(
            EffectOperation::new("network".to_string(), "get".to_string()),
            |_args| HandlerAction::Return(HandlerValue::String("mock data".to_string())),
        );

        handler
    }

    /// Cache handler - caches operation results
    pub fn cache_handler() -> Handler {
        let mut handler = Handler::new("cache".to_string());

        // Cache network.get
        handler.on(
            EffectOperation::new("network".to_string(), "get".to_string()),
            |args| {
                // In real implementation, would check cache first
                println!("[CACHE] Checking cache for: {:?}", args);
                HandlerAction::Resume
            },
        );

        handler
    }

    /// Rate limiting handler
    pub fn rate_limit_handler(max_per_second: usize) -> Handler {
        let mut handler = Handler::new("rate_limit".to_string());

        // Rate limit network operations
        handler.on(
            EffectOperation::new("network".to_string(), "get".to_string()),
            move |_args| {
                println!("[RATE_LIMIT] Max {} requests/second", max_per_second);
                HandlerAction::Resume
            },
        );

        handler
    }
}

/// Handler context for execution
pub struct HandlerContext {
    /// Handler registry
    pub registry: HandlerRegistry,
}

impl HandlerContext {
    /// Create a new handler context
    pub fn new() -> Self {
        Self {
            registry: HandlerRegistry::new(),
        }
    }

    /// Create with built-in handlers
    pub fn with_builtins() -> Self {
        let mut ctx = Self::new();

        // Register built-in handlers
        ctx.registry.register(builtin::logging_handler());
        ctx.registry.register(builtin::tracing_handler());
        ctx.registry.register(builtin::retry_handler(3));
        ctx.registry.register(builtin::mock_handler());
        ctx.registry.register(builtin::cache_handler());
        ctx.registry.register(builtin::rate_limit_handler(10));

        ctx
    }

    /// Execute with handlers
    pub fn with_handlers<F, R>(&mut self, handlers: &[&str], f: F) -> Result<R, String>
    where
        F: FnOnce(&mut Self) -> R,
    {
        // Activate handlers
        for handler in handlers {
            self.registry.activate(handler)?;
        }

        // Execute function
        let result = f(self);

        // Deactivate handlers
        for _ in handlers {
            self.registry.deactivate();
        }

        Ok(result)
    }
}

impl Default for HandlerContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_creation() {
        let handler = Handler::new("test".to_string());
        assert_eq!(handler.name, "test");
        assert_eq!(handler.operations.len(), 0);
    }

    #[test]
    fn test_handler_operation() {
        let mut handler = Handler::new("test".to_string());

        let op = EffectOperation::new("io".to_string(), "print".to_string());
        handler.on(op.clone(), |_| HandlerAction::Resume);

        let action = handler.handle(&op, &[]);
        assert!(action.is_some());
        assert_eq!(action.unwrap(), HandlerAction::Resume);
    }

    #[test]
    fn test_handler_registry() {
        let mut registry = HandlerRegistry::new();

        let handler = Handler::new("test".to_string());
        registry.register(handler);

        assert!(registry.activate("test").is_ok());
        assert_eq!(registry.active_handlers().len(), 1);

        registry.deactivate();
        assert_eq!(registry.active_handlers().len(), 0);
    }

    #[test]
    fn test_builtin_logging_handler() {
        let handler = builtin::logging_handler();
        assert_eq!(handler.name, "logging");
    }

    #[test]
    fn test_builtin_mock_handler() {
        let handler = builtin::mock_handler();

        let op = EffectOperation::new("io".to_string(), "print".to_string());
        let action = handler.handle(&op, &[]);

        assert!(action.is_some());
        assert!(matches!(
            action.unwrap(),
            HandlerAction::Return(HandlerValue::Unit)
        ));
    }

    #[test]
    fn test_handler_context() {
        let mut ctx = HandlerContext::with_builtins();

        let result = ctx.with_handlers(&["logging", "tracing"], |_ctx| {
            // Simulate some operations
            42
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_handler_stack() {
        let mut registry = HandlerRegistry::new();

        let h1 = Handler::new("h1".to_string());
        let h2 = Handler::new("h2".to_string());

        registry.register(h1);
        registry.register(h2);

        registry.activate("h1").unwrap();
        registry.activate("h2").unwrap();

        assert_eq!(registry.active_handlers(), &["h1", "h2"]);

        registry.deactivate();
        assert_eq!(registry.active_handlers(), &["h1"]);
    }
}
