//! Effect checking integration for bytecode compiler
//! Sprint 27.3: Compile-time effect verification

use matter_ast::*;
use std::collections::HashMap;

/// Effect checking error
#[derive(Debug, Clone)]
pub struct EffectError {
    pub message: String,
    pub function: String,
}

impl EffectError {
    pub fn new(message: String, function: String) -> Self {
        Self { message, function }
    }
}

/// Effect checker for bytecode compilation
pub struct BytecodeEffectChecker {
    /// Function effect declarations
    function_effects: HashMap<String, Vec<String>>,

    /// Built-in function effects
    builtin_effects: HashMap<String, Vec<String>>,

    /// Errors collected during checking
    errors: Vec<EffectError>,
}

impl BytecodeEffectChecker {
    /// Create a new effect checker
    pub fn new() -> Self {
        let mut checker = Self {
            function_effects: HashMap::new(),
            builtin_effects: HashMap::new(),
            errors: Vec::new(),
        };

        checker.register_builtins();
        checker
    }

    /// Register built-in function effects
    fn register_builtins(&mut self) {
        // IO effects
        self.builtin_effects
            .insert("print".to_string(), vec!["io".to_string()]);
        self.builtin_effects
            .insert("println".to_string(), vec!["io".to_string()]);
        self.builtin_effects
            .insert("read".to_string(), vec!["io".to_string()]);

        // Database effects
        self.builtin_effects.insert(
            "db.connect".to_string(),
            vec!["db".to_string(), "io".to_string()],
        );
        self.builtin_effects.insert(
            "db.query".to_string(),
            vec!["db".to_string(), "io".to_string()],
        );

        // Network effects
        self.builtin_effects.insert(
            "net.get".to_string(),
            vec!["network".to_string(), "io".to_string()],
        );
        self.builtin_effects.insert(
            "net.post".to_string(),
            vec!["network".to_string(), "io".to_string()],
        );

        // Time effects
        self.builtin_effects
            .insert("time.now".to_string(), vec!["time".to_string()]);
        self.builtin_effects
            .insert("time.sleep".to_string(), vec!["time".to_string()]);

        // Random effects
        self.builtin_effects
            .insert("random.int".to_string(), vec!["random".to_string()]);
        self.builtin_effects
            .insert("random.float".to_string(), vec!["random".to_string()]);
    }

    /// Check a program for effect violations
    pub fn check_program(&mut self, program: &Program) {
        // First pass: collect all function effect declarations
        for stmt in &program.statements {
            if let Statement::FunctionDef {
                name,
                effects: Some(effect_list),
                ..
            } = stmt
            {
                self.function_effects
                    .insert(name.clone(), effect_list.clone());
            }
        }

        // Second pass: validate function bodies
        for stmt in &program.statements {
            if let Statement::FunctionDef {
                name,
                body,
                effects,
                ..
            } = stmt
            {
                let declared_effects = effects.clone().unwrap_or_default();
                self.check_function_body(name, body, &declared_effects);
            }
        }
    }

    /// Check a function body for effect violations
    fn check_function_body(
        &mut self,
        function_name: &str,
        body: &[Statement],
        declared_effects: &[String],
    ) {
        for stmt in body {
            self.check_statement(function_name, stmt, declared_effects);
        }
    }

    /// Check a statement for effect violations
    fn check_statement(
        &mut self,
        function_name: &str,
        stmt: &Statement,
        declared_effects: &[String],
    ) {
        match stmt {
            Statement::Expression(expr) => {
                self.check_expression(function_name, expr, declared_effects);
            }
            Statement::Print(expr) => {
                // print requires 'io' effect
                if !declared_effects.contains(&"io".to_string()) {
                    self.errors.push(EffectError::new(
                        format!(
                            "Function '{}' uses 'print' but doesn't declare 'io' effect",
                            function_name
                        ),
                        function_name.to_string(),
                    ));
                }
                self.check_expression(function_name, expr, declared_effects);
            }
            Statement::Let { value, .. }
            | Statement::Set { value, .. }
            | Statement::Return(value) => {
                self.check_expression(function_name, value, declared_effects);
            }
            Statement::SetIndex {
                target,
                index,
                value,
            } => {
                self.check_expression(function_name, target, declared_effects);
                self.check_expression(function_name, index, declared_effects);
                self.check_expression(function_name, value, declared_effects);
            }
            Statement::SetField { value, .. } => {
                self.check_expression(function_name, value, declared_effects);
            }
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                self.check_expression(function_name, condition, declared_effects);
                self.check_function_body(function_name, then_body, declared_effects);
                if let Some(else_stmts) = else_body {
                    self.check_function_body(function_name, else_stmts, declared_effects);
                }
            }
            Statement::While { condition, body } => {
                self.check_expression(function_name, condition, declared_effects);
                self.check_function_body(function_name, body, declared_effects);
            }
            Statement::For { iterable, body, .. } => {
                self.check_expression(function_name, iterable, declared_effects);
                self.check_function_body(function_name, body, declared_effects);
            }
            Statement::Loop { body } => {
                self.check_function_body(function_name, body, declared_effects);
            }
            Statement::OnEvent { body, .. } => {
                self.check_function_body(function_name, body, declared_effects);
            }
            _ => {}
        }
    }

    /// Check an expression for effect violations
    fn check_expression(
        &mut self,
        function_name: &str,
        expr: &Expression,
        declared_effects: &[String],
    ) {
        match expr {
            Expression::Call { callee, args } => {
                // Check if it's a function call
                if let Expression::Identifier(called_func) = callee.as_ref() {
                    self.check_function_call(function_name, called_func, declared_effects);
                }

                // Check arguments
                for arg in args {
                    self.check_expression(function_name, arg, declared_effects);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.check_expression(function_name, left, declared_effects);
                self.check_expression(function_name, right, declared_effects);
            }
            Expression::List(items) => {
                for item in items {
                    self.check_expression(function_name, item, declared_effects);
                }
            }
            Expression::Map(fields) => {
                for (_, value) in fields {
                    self.check_expression(function_name, value, declared_effects);
                }
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, value) in fields {
                    self.check_expression(function_name, value, declared_effects);
                }
            }
            Expression::Field { target, .. } => {
                self.check_expression(function_name, target, declared_effects);
            }
            Expression::Index { target, index } => {
                self.check_expression(function_name, target, declared_effects);
                self.check_expression(function_name, index, declared_effects);
            }
            Expression::MethodCall { target, args, .. } => {
                self.check_expression(function_name, target, declared_effects);
                for arg in args {
                    self.check_expression(function_name, arg, declared_effects);
                }
            }
            _ => {}
        }
    }

    /// Check if a function call is allowed
    fn check_function_call(&mut self, caller: &str, callee: &str, declared_effects: &[String]) {
        // Check built-in functions
        if let Some(required_effects) = self.builtin_effects.get(callee) {
            for required in required_effects {
                if !declared_effects.contains(required) {
                    self.errors.push(EffectError::new(
                        format!(
                            "Function '{}' calls '{}' which requires '{}' effect, but it's not declared",
                            caller, callee, required
                        ),
                        caller.to_string(),
                    ));
                }
            }
            return;
        }

        // Check user-defined functions
        if let Some(required_effects) = self.function_effects.get(callee) {
            for required in required_effects {
                if !declared_effects.contains(required) {
                    self.errors.push(EffectError::new(
                        format!(
                            "Function '{}' calls '{}' which has '{}' effect, but it's not declared",
                            caller, callee, required
                        ),
                        caller.to_string(),
                    ));
                }
            }
        }
    }

    /// Get all errors
    pub fn errors(&self) -> &[EffectError] {
        &self.errors
    }

    /// Check if there are errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

impl Default for BytecodeEffectChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_function_allowed() {
        let mut checker = BytecodeEffectChecker::new();

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

        checker.check_program(&program);
        assert!(!checker.has_errors());
    }

    #[test]
    fn test_io_effect_required() {
        let mut checker = BytecodeEffectChecker::new();

        let program = Program::new(vec![Statement::FunctionDef {
            name: "log".to_string(),
            params: vec![Param::new("msg".to_string())],
            return_type: None,
            body: vec![Statement::Print(Expression::Identifier("msg".to_string()))],
            effects: None, // Missing 'io' effect!
        }]);

        checker.check_program(&program);
        assert!(checker.has_errors());
        assert_eq!(checker.errors().len(), 1);
    }

    #[test]
    fn test_io_effect_declared() {
        let mut checker = BytecodeEffectChecker::new();

        let program = Program::new(vec![Statement::FunctionDef {
            name: "log".to_string(),
            params: vec![Param::new("msg".to_string())],
            return_type: None,
            body: vec![Statement::Print(Expression::Identifier("msg".to_string()))],
            effects: Some(vec!["io".to_string()]),
        }]);

        checker.check_program(&program);
        assert!(!checker.has_errors());
    }
}
