//! Matter Effect System
//!
//! Sistema de rastreamento de efeitos colaterais que permite ao compilador
//! garantir que efeitos sejam tratados corretamente.
//!
//! ## Funcionalidades
//!
//! - Rastreamento automático de efeitos
//! - Verificação em compile-time
//! - Efeitos built-in (io, db, network, etc)
//! - Efeitos customizados
//! - Composição de efeitos
//! - Handlers de efeitos
//!
//! ## Exemplo
//!
//! ```matter
//! // Função pura (sem efeitos)
//! fn pure(x: int) -> int {
//!     return x * 2;
//! }
//!
//! // Função com efeito IO
//! fn log(message: string) -> unit with io {
//!     print message;
//! }
//!
//! // Função com múltiplos efeitos
//! fn save_to_db(data: string) -> result with io, db, network {
//!     let conn = db.connect();
//!     conn.save(data);
//!     return ok;
//! }
//! ```

use std::collections::{HashMap, HashSet};
use std::fmt;

/// Effect representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Pure (no effects)
    Pure,

    /// IO effect (print, read, write)
    IO,

    /// Database effect
    Database,

    /// Network effect (HTTP, TCP, etc)
    Network,

    /// File system effect
    FileSystem,

    /// Time effect (sleep, now, etc)
    Time,

    /// Random effect
    Random,

    /// State effect (mutable state)
    State,

    /// Exception effect (can throw)
    Exception,

    /// Async effect (async/await)
    Async,

    /// Custom effect
    Custom(String),
}

impl Effect {
    /// Check if effect is pure
    pub fn is_pure(&self) -> bool {
        matches!(self, Effect::Pure)
    }

    /// Check if effect is impure
    pub fn is_impure(&self) -> bool {
        !self.is_pure()
    }

    /// Get effect name
    pub fn name(&self) -> &str {
        match self {
            Effect::Pure => "pure",
            Effect::IO => "io",
            Effect::Database => "db",
            Effect::Network => "network",
            Effect::FileSystem => "fs",
            Effect::Time => "time",
            Effect::Random => "random",
            Effect::State => "state",
            Effect::Exception => "exception",
            Effect::Async => "async",
            Effect::Custom(name) => name,
        }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Effect set (collection of effects)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectSet {
    effects: HashSet<Effect>,
}

impl EffectSet {
    /// Create a new empty effect set
    pub fn new() -> Self {
        Self {
            effects: HashSet::new(),
        }
    }

    /// Create a pure effect set
    pub fn pure() -> Self {
        let mut set = Self::new();
        set.effects.insert(Effect::Pure);
        set
    }

    /// Add an effect
    pub fn add(&mut self, effect: Effect) {
        // Remove Pure if adding impure effect
        if effect.is_impure() {
            self.effects.remove(&Effect::Pure);
        }
        self.effects.insert(effect);
    }

    /// Check if has effect
    pub fn has(&self, effect: &Effect) -> bool {
        self.effects.contains(effect)
    }

    /// Check if is pure
    pub fn is_pure(&self) -> bool {
        self.effects.is_empty() || (self.effects.len() == 1 && self.has(&Effect::Pure))
    }

    /// Check if is impure
    pub fn is_impure(&self) -> bool {
        !self.is_pure()
    }

    /// Merge with another effect set
    pub fn merge(&mut self, other: &EffectSet) {
        for effect in &other.effects {
            self.add(effect.clone());
        }
    }

    /// Check if this set is compatible with another (subset)
    pub fn is_compatible_with(&self, other: &EffectSet) -> bool {
        // Pure is compatible with everything
        if self.is_pure() {
            return true;
        }

        // Check if all our effects are in the other set
        self.effects.iter().all(|e| other.has(e))
    }

    /// Get all effects
    pub fn effects(&self) -> Vec<Effect> {
        self.effects.iter().cloned().collect()
    }
}

impl Default for EffectSet {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EffectSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_pure() {
            write!(f, "pure")
        } else {
            let effects: Vec<_> = self.effects.iter().map(|e| e.to_string()).collect();
            write!(f, "with {}", effects.join(", "))
        }
    }
}

/// Effect signature for functions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectSignature {
    /// Function name
    pub name: String,

    /// Effect set
    pub effects: EffectSet,
}

impl EffectSignature {
    /// Create a new effect signature
    pub fn new(name: String, effects: EffectSet) -> Self {
        Self { name, effects }
    }

    /// Create a pure signature
    pub fn pure(name: String) -> Self {
        Self {
            name,
            effects: EffectSet::pure(),
        }
    }
}

/// Effect environment
#[derive(Debug, Clone)]
pub struct EffectEnv {
    /// Function signatures
    functions: HashMap<String, EffectSignature>,

    /// Current function effects
    current_effects: EffectSet,
}

impl EffectEnv {
    /// Create a new effect environment
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            current_effects: EffectSet::new(),
        }
    }

    /// Add a function signature
    pub fn add_function(&mut self, signature: EffectSignature) {
        self.functions.insert(signature.name.clone(), signature);
    }

    /// Get a function signature
    pub fn get_function(&self, name: &str) -> Option<&EffectSignature> {
        self.functions.get(name)
    }

    /// Add effect to current context
    pub fn add_effect(&mut self, effect: Effect) {
        self.current_effects.add(effect);
    }

    /// Get current effects
    pub fn current_effects(&self) -> &EffectSet {
        &self.current_effects
    }

    /// Reset current effects
    pub fn reset_effects(&mut self) {
        self.current_effects = EffectSet::new();
    }
}

impl Default for EffectEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Effect checker
pub struct EffectChecker {
    env: EffectEnv,
    errors: Vec<String>,
}

impl EffectChecker {
    /// Create a new effect checker
    pub fn new() -> Self {
        let mut checker = Self {
            env: EffectEnv::new(),
            errors: Vec::new(),
        };

        // Register built-in effects
        checker.register_builtins();

        checker
    }

    /// Register built-in function effects
    fn register_builtins(&mut self) {
        // IO functions
        let mut io_effects = EffectSet::new();
        io_effects.add(Effect::IO);
        self.env.add_function(EffectSignature::new(
            "print".to_string(),
            io_effects.clone(),
        ));
        self.env.add_function(EffectSignature::new(
            "println".to_string(),
            io_effects.clone(),
        ));
        self.env
            .add_function(EffectSignature::new("read".to_string(), io_effects));

        // Database functions
        let mut db_effects = EffectSet::new();
        db_effects.add(Effect::Database);
        db_effects.add(Effect::IO);
        self.env.add_function(EffectSignature::new(
            "db.connect".to_string(),
            db_effects.clone(),
        ));
        self.env
            .add_function(EffectSignature::new("db.query".to_string(), db_effects));

        // Network functions
        let mut net_effects = EffectSet::new();
        net_effects.add(Effect::Network);
        net_effects.add(Effect::IO);
        self.env.add_function(EffectSignature::new(
            "net.get".to_string(),
            net_effects.clone(),
        ));
        self.env
            .add_function(EffectSignature::new("net.post".to_string(), net_effects));

        // Time functions
        let mut time_effects = EffectSet::new();
        time_effects.add(Effect::Time);
        self.env.add_function(EffectSignature::new(
            "time.now".to_string(),
            time_effects.clone(),
        ));
        self.env
            .add_function(EffectSignature::new("time.sleep".to_string(), time_effects));

        // Random functions
        let mut random_effects = EffectSet::new();
        random_effects.add(Effect::Random);
        self.env.add_function(EffectSignature::new(
            "random.int".to_string(),
            random_effects.clone(),
        ));
        self.env.add_function(EffectSignature::new(
            "random.float".to_string(),
            random_effects,
        ));
    }

    /// Check if function call is allowed in current context
    pub fn check_call(&mut self, function_name: &str, allowed_effects: &EffectSet) -> bool {
        if let Some(signature) = self.env.get_function(function_name) {
            if !signature.effects.is_compatible_with(allowed_effects) {
                self.errors.push(format!(
                    "Function '{}' has effects {} but only {} are allowed",
                    function_name, signature.effects, allowed_effects
                ));
                return false;
            }
        }
        true
    }

    /// Get errors
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Check if has errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get environment
    pub fn env(&self) -> &EffectEnv {
        &self.env
    }

    /// Get mutable environment
    pub fn env_mut(&mut self) -> &mut EffectEnv {
        &mut self.env
    }
}

impl Default for EffectChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Effect handler
pub trait EffectHandler {
    /// Handle an effect
    fn handle(&self, effect: &Effect) -> Result<(), String>;
}

/// Default effect handler (does nothing)
pub struct DefaultEffectHandler;

impl EffectHandler for DefaultEffectHandler {
    fn handle(&self, _effect: &Effect) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_set() {
        let mut set = EffectSet::new();
        assert!(set.is_pure());

        set.add(Effect::IO);
        assert!(set.is_impure());
        assert!(set.has(&Effect::IO));
    }

    #[test]
    fn test_effect_compatibility() {
        let pure = EffectSet::pure();
        let mut io = EffectSet::new();
        io.add(Effect::IO);

        assert!(pure.is_compatible_with(&io));
        assert!(!io.is_compatible_with(&pure));
    }

    #[test]
    fn test_effect_merge() {
        let mut set1 = EffectSet::new();
        set1.add(Effect::IO);

        let mut set2 = EffectSet::new();
        set2.add(Effect::Database);

        set1.merge(&set2);
        assert!(set1.has(&Effect::IO));
        assert!(set1.has(&Effect::Database));
    }

    #[test]
    fn test_effect_checker() {
        let mut checker = EffectChecker::new();

        let mut allowed = EffectSet::new();
        allowed.add(Effect::IO);

        // Should allow print (has IO effect)
        assert!(checker.check_call("print", &allowed));

        // Should not allow db.query (needs Database effect)
        assert!(!checker.check_call("db.query", &allowed));
        assert!(checker.has_errors());
    }
}
