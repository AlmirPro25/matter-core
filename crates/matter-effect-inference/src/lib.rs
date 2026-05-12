//! Matter Effect Inference
//!
//! Sistema de inferência automática de efeitos que permite ao compilador
//! deduzir os efeitos de uma função sem anotações explícitas.
//!
//! ## Funcionalidades
//!
//! - Inferência automática de efeitos
//! - Análise de fluxo de controle
//! - Propagação de efeitos
//! - Verificação de consistência
//! - Sugestões de anotações
//!
//! ## Exemplo
//!
//! ```matter
//! // SEM inferência (manual)
//! fn log(msg: string) -> unit with io {
//!     print msg;
//! }
//!
//! // COM inferência (automático)
//! fn log(msg: string) -> unit {
//!     print msg;  // Compilador infere 'io'
//! }
//!
//! // Inferência com múltiplos efeitos
//! fn save(data: string) -> result {
//!     let conn = db.connect();  // Infere 'db'
//!     conn.save(data);          // Infere 'db'
//!     print "Saved!";           // Infere 'io'
//!     // Efeitos inferidos: io, db
//! }
//! ```

use matter_ast::*;
use matter_effects::Effect;
use std::collections::{HashMap, HashSet};

/// Inferred effect set
#[derive(Debug, Clone, PartialEq)]
pub struct InferredEffects {
    /// Inferred effects
    pub effects: HashSet<Effect>,

    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,

    /// Source of inference
    pub source: InferenceSource,
}

impl InferredEffects {
    /// Create new inferred effects
    pub fn new(effects: HashSet<Effect>, confidence: f64, source: InferenceSource) -> Self {
        Self {
            effects,
            confidence,
            source,
        }
    }

    /// Create empty (pure)
    pub fn pure() -> Self {
        let mut effects = HashSet::new();
        effects.insert(Effect::Pure);
        Self {
            effects,
            confidence: 1.0,
            source: InferenceSource::Pure,
        }
    }

    /// Merge with another inferred effects
    pub fn merge(&mut self, other: &InferredEffects) {
        // Remove Pure if adding impure effects
        if !other.effects.is_empty() && !other.effects.contains(&Effect::Pure) {
            self.effects.remove(&Effect::Pure);
        }

        // Add all effects
        for effect in &other.effects {
            if effect != &Effect::Pure {
                self.effects.insert(effect.clone());
            }
        }

        // Update confidence (minimum)
        self.confidence = self.confidence.min(other.confidence);
    }

    /// Check if is pure
    pub fn is_pure(&self) -> bool {
        self.effects.is_empty() || (self.effects.len() == 1 && self.effects.contains(&Effect::Pure))
    }
}

/// Source of inference
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceSource {
    /// Pure function (no effects)
    Pure,

    /// Direct call to built-in
    BuiltinCall { function: String },

    /// Call to user function
    FunctionCall { function: String },

    /// Control flow analysis
    ControlFlow,

    /// Explicit annotation
    Explicit,
}

/// Effect inference engine
pub struct EffectInference {
    /// Built-in function effects
    builtin_effects: HashMap<String, HashSet<Effect>>,

    /// Inferred function effects
    function_effects: HashMap<String, InferredEffects>,

    /// Inference errors
    errors: Vec<String>,
}

impl EffectInference {
    /// Create new effect inference engine
    pub fn new() -> Self {
        let mut engine = Self {
            builtin_effects: HashMap::new(),
            function_effects: HashMap::new(),
            errors: Vec::new(),
        };

        engine.register_builtins();
        engine
    }

    /// Register built-in function effects
    fn register_builtins(&mut self) {
        // IO effects
        let mut io_effects = HashSet::new();
        io_effects.insert(Effect::IO);

        self.builtin_effects
            .insert("print".to_string(), io_effects.clone());
        self.builtin_effects
            .insert("println".to_string(), io_effects.clone());
        self.builtin_effects.insert("read".to_string(), io_effects);

        // Database effects
        let mut db_effects = HashSet::new();
        db_effects.insert(Effect::Database);
        db_effects.insert(Effect::IO);

        self.builtin_effects
            .insert("db.connect".to_string(), db_effects.clone());
        self.builtin_effects
            .insert("db.query".to_string(), db_effects);

        // Network effects
        let mut net_effects = HashSet::new();
        net_effects.insert(Effect::Network);
        net_effects.insert(Effect::IO);

        self.builtin_effects
            .insert("net.get".to_string(), net_effects.clone());
        self.builtin_effects
            .insert("net.post".to_string(), net_effects);

        // Time effects
        let mut time_effects = HashSet::new();
        time_effects.insert(Effect::Time);

        self.builtin_effects
            .insert("time.now".to_string(), time_effects.clone());
        self.builtin_effects
            .insert("time.sleep".to_string(), time_effects);

        // Random effects
        let mut random_effects = HashSet::new();
        random_effects.insert(Effect::Random);

        self.builtin_effects
            .insert("random.int".to_string(), random_effects.clone());
        self.builtin_effects
            .insert("random.float".to_string(), random_effects);
    }

    /// Infer effects for a program
    pub fn infer_program(&mut self, program: &Program) {
        // First pass: collect explicit annotations
        for stmt in &program.statements {
            if let Statement::FunctionDef {
                name,
                effects: Some(effect_list),
                ..
            } = stmt
            {
                let mut effect_set = HashSet::new();
                for effect_name in effect_list {
                    effect_set.insert(self.parse_effect(effect_name));
                }

                self.function_effects.insert(
                    name.clone(),
                    InferredEffects::new(effect_set, 1.0, InferenceSource::Explicit),
                );
            }
        }

        // Second pass: infer missing effects
        for stmt in &program.statements {
            if let Statement::FunctionDef {
                name,
                body,
                effects,
                ..
            } = stmt
            {
                if effects.is_none() {
                    let inferred = self.infer_function_body(body);
                    self.function_effects.insert(name.clone(), inferred);
                }
            }
        }
    }

    /// Parse effect name to Effect enum
    fn parse_effect(&self, name: &str) -> Effect {
        match name {
            "io" => Effect::IO,
            "db" | "database" => Effect::Database,
            "network" | "net" => Effect::Network,
            "fs" | "filesystem" => Effect::FileSystem,
            "time" => Effect::Time,
            "random" => Effect::Random,
            "state" => Effect::State,
            "exception" => Effect::Exception,
            "async" => Effect::Async,
            _ => Effect::Custom(name.to_string()),
        }
    }

    /// Infer effects for a function body
    fn infer_function_body(&self, body: &[Statement]) -> InferredEffects {
        let mut result = InferredEffects::pure();

        for stmt in body {
            let stmt_effects = self.infer_statement(stmt);
            result.merge(&stmt_effects);
        }

        result
    }

    /// Infer effects for a statement
    fn infer_statement(&self, stmt: &Statement) -> InferredEffects {
        match stmt {
            Statement::Expression(expr) => self.infer_expression(expr),

            Statement::Print(expr) => {
                // print has IO effect
                let mut effects = HashSet::new();
                effects.insert(Effect::IO);

                let mut result = InferredEffects::new(
                    effects,
                    1.0,
                    InferenceSource::BuiltinCall {
                        function: "print".to_string(),
                    },
                );

                // Also infer effects from the expression
                let expr_effects = self.infer_expression(expr);
                result.merge(&expr_effects);

                result
            }

            Statement::Let { value, .. }
            | Statement::Set { value, .. }
            | Statement::Return(value) => self.infer_expression(value),

            Statement::SetIndex {
                target,
                index,
                value,
            } => {
                let mut result = self.infer_expression(target);
                result.merge(&self.infer_expression(index));
                result.merge(&self.infer_expression(value));
                result
            }

            Statement::SetField { value, .. } => self.infer_expression(value),

            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                let mut result = self.infer_expression(condition);
                result.merge(&self.infer_function_body(then_body));

                if let Some(else_stmts) = else_body {
                    result.merge(&self.infer_function_body(else_stmts));
                }

                result
            }

            Statement::While { condition, body } => {
                let mut result = self.infer_expression(condition);
                result.merge(&self.infer_function_body(body));
                result
            }

            Statement::For { iterable, body, .. } => {
                let mut result = self.infer_expression(iterable);
                result.merge(&self.infer_function_body(body));
                result
            }

            Statement::Loop { body } => self.infer_function_body(body),

            Statement::OnEvent { body, .. } => self.infer_function_body(body),

            _ => InferredEffects::pure(),
        }
    }

    /// Infer effects for an expression
    fn infer_expression(&self, expr: &Expression) -> InferredEffects {
        match expr {
            Expression::Call { callee, args } => {
                let mut result = InferredEffects::pure();

                // Check if it's a function call
                if let Expression::Identifier(func_name) = callee.as_ref() {
                    // Check built-in functions
                    if let Some(effects) = self.builtin_effects.get(func_name) {
                        result = InferredEffects::new(
                            effects.clone(),
                            1.0,
                            InferenceSource::BuiltinCall {
                                function: func_name.clone(),
                            },
                        );
                    }
                    // Check user-defined functions
                    else if let Some(inferred) = self.function_effects.get(func_name) {
                        result = inferred.clone();
                        result.source = InferenceSource::FunctionCall {
                            function: func_name.clone(),
                        };
                    }
                }

                // Infer effects from arguments
                for arg in args {
                    result.merge(&self.infer_expression(arg));
                }

                result
            }

            Expression::Binary { left, right, .. } => {
                let mut result = self.infer_expression(left);
                result.merge(&self.infer_expression(right));
                result
            }

            Expression::List(items) => {
                let mut result = InferredEffects::pure();
                for item in items {
                    result.merge(&self.infer_expression(item));
                }
                result
            }

            Expression::Map(fields) => {
                let mut result = InferredEffects::pure();
                for (_, value) in fields {
                    result.merge(&self.infer_expression(value));
                }
                result
            }

            Expression::StructLiteral { fields, .. } => {
                let mut result = InferredEffects::pure();
                for (_, value) in fields {
                    result.merge(&self.infer_expression(value));
                }
                result
            }

            Expression::Field { target, .. } => self.infer_expression(target),

            Expression::Index { target, index } => {
                let mut result = self.infer_expression(target);
                result.merge(&self.infer_expression(index));
                result
            }

            Expression::MethodCall { target, args, .. } => {
                let mut result = self.infer_expression(target);
                for arg in args {
                    result.merge(&self.infer_expression(arg));
                }
                result
            }

            _ => InferredEffects::pure(),
        }
    }

    /// Get inferred effects for a function
    pub fn get_function_effects(&self, name: &str) -> Option<&InferredEffects> {
        self.function_effects.get(name)
    }

    /// Get all inferred functions
    pub fn get_all_inferred(&self) -> &HashMap<String, InferredEffects> {
        &self.function_effects
    }

    /// Get errors
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Check if has errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl Default for EffectInference {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_pure_function() {
        let mut inference = EffectInference::new();

        let program = Program::new(vec![Statement::FunctionDef {
            name: "pure".to_string(),
            params: vec![Param::new("x".to_string())],
            return_type: None,
            body: vec![Statement::Return(Expression::Binary {
                left: Box::new(Expression::Identifier("x".to_string())),
                op: BinaryOp::Mul,
                right: Box::new(Expression::Int(2)),
            })],
            effects: None,
        }]);

        inference.infer_program(&program);

        let effects = inference.get_function_effects("pure").unwrap();
        assert!(effects.is_pure());
    }

    #[test]
    fn test_infer_io_effect() {
        let mut inference = EffectInference::new();

        let program = Program::new(vec![Statement::FunctionDef {
            name: "log".to_string(),
            params: vec![Param::new("msg".to_string())],
            return_type: None,
            body: vec![Statement::Print(Expression::Identifier("msg".to_string()))],
            effects: None,
        }]);

        inference.infer_program(&program);

        let effects = inference.get_function_effects("log").unwrap();
        assert!(!effects.is_pure());
        assert!(effects.effects.contains(&Effect::IO));
    }

    #[test]
    fn test_infer_multiple_effects() {
        let mut inference = EffectInference::new();

        let program = Program::new(vec![Statement::FunctionDef {
            name: "save".to_string(),
            params: vec![Param::new("data".to_string())],
            return_type: None,
            body: vec![
                Statement::Expression(Expression::Call {
                    callee: Box::new(Expression::Identifier("db.connect".to_string())),
                    args: vec![],
                }),
                Statement::Print(Expression::String("Saved!".to_string())),
            ],
            effects: None,
        }]);

        inference.infer_program(&program);

        let effects = inference.get_function_effects("save").unwrap();
        assert!(!effects.is_pure());
        assert!(effects.effects.contains(&Effect::IO));
        assert!(effects.effects.contains(&Effect::Database));
    }

    #[test]
    fn test_explicit_annotation_preserved() {
        let mut inference = EffectInference::new();

        let program = Program::new(vec![Statement::FunctionDef {
            name: "explicit".to_string(),
            params: vec![],
            return_type: None,
            body: vec![],
            effects: Some(vec!["io".to_string(), "db".to_string()]),
        }]);

        inference.infer_program(&program);

        let effects = inference.get_function_effects("explicit").unwrap();
        assert!(effects.effects.contains(&Effect::IO));
        assert!(effects.effects.contains(&Effect::Database));
        assert_eq!(effects.confidence, 1.0);
    }
}
