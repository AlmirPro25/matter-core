use matter_ast::*;
use matter_parser::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintDiagnostic {
    pub severity: Severity,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub rule: String,
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    pub unused_variables: Severity,
    pub unused_functions: Severity,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            unused_variables: Severity::Warning,
            unused_functions: Severity::Warning,
        }
    }
}

pub struct Linter {
    config: LintConfig,
}

impl Linter {
    pub fn new(config: LintConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(LintConfig::default())
    }

    pub fn lint(&self, source: &str) -> Result<Vec<LintDiagnostic>, String> {
        let mut parser = Parser::from_source(source);
        let program = parser.parse().map_err(|e| e.to_string())?;

        let mut diags = Vec::new();
        diags.extend(lint_unused_variables(
            &program,
            self.config.unused_variables,
        ));
        diags.extend(lint_unused_functions(
            &program,
            self.config.unused_functions,
        ));
        Ok(diags)
    }
}

fn lint_unused_variables(program: &Program, severity: Severity) -> Vec<LintDiagnostic> {
    let mut defined = HashSet::new();
    let mut used = HashSet::new();
    for stmt in &program.statements {
        collect_stmt_defs_uses(stmt, &mut defined, &mut used);
    }

    defined
        .difference(&used)
        .map(|name| LintDiagnostic {
            severity,
            message: format!("Variable '{}' is never used", name),
            line: 0,
            column: 0,
            rule: "unused_variable".to_string(),
            fix: None,
        })
        .collect()
}

fn lint_unused_functions(program: &Program, severity: Severity) -> Vec<LintDiagnostic> {
    let mut defined = HashSet::new();
    let mut called = HashSet::new();
    for stmt in &program.statements {
        if let Statement::FunctionDef { name, .. } = stmt {
            defined.insert(name.clone());
        }
        collect_stmt_calls(stmt, &mut called);
    }

    defined
        .difference(&called)
        .map(|name| LintDiagnostic {
            severity,
            message: format!("Function '{}' is never used", name),
            line: 0,
            column: 0,
            rule: "unused_function".to_string(),
            fix: None,
        })
        .collect()
}

fn collect_stmt_defs_uses(
    stmt: &Statement,
    defined: &mut HashSet<String>,
    used: &mut HashSet<String>,
) {
    match stmt {
        Statement::Let { name, value, .. } => {
            defined.insert(name.clone());
            collect_expr_uses(value, used);
        }
        Statement::Set { name, value } => {
            used.insert(name.clone());
            collect_expr_uses(value, used);
        }
        Statement::SetIndex {
            target,
            index,
            value,
        } => {
            collect_expr_uses(target, used);
            collect_expr_uses(index, used);
            collect_expr_uses(value, used);
        }
        Statement::SetField { target, value, .. } => {
            used.insert(target.clone());
            collect_expr_uses(value, used);
        }
        Statement::Print(value) | Statement::Return(value) | Statement::Expression(value) => {
            collect_expr_uses(value, used);
        }
        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            collect_expr_uses(condition, used);
            for inner in then_body {
                collect_stmt_defs_uses(inner, defined, used);
            }
            if let Some(inner) = else_body {
                for stmt in inner {
                    collect_stmt_defs_uses(stmt, defined, used);
                }
            }
        }
        Statement::While { condition, body } => {
            collect_expr_uses(condition, used);
            for inner in body {
                collect_stmt_defs_uses(inner, defined, used);
            }
        }
        Statement::For {
            item,
            iterable,
            body,
        } => {
            defined.insert(item.clone());
            collect_expr_uses(iterable, used);
            for inner in body {
                collect_stmt_defs_uses(inner, defined, used);
            }
        }
        Statement::Loop { body } | Statement::OnEvent { body, .. } => {
            for inner in body {
                collect_stmt_defs_uses(inner, defined, used);
            }
        }
        Statement::FunctionDef { params, body, .. } => {
            for param in params {
                defined.insert(param.name.clone());
            }
            for inner in body {
                collect_stmt_defs_uses(inner, defined, used);
            }
        }
        Statement::Import { .. }
        | Statement::StructDef { .. }
        | Statement::Spawn { .. }
        | Statement::Break
        | Statement::Continue => {}
    }
}

fn collect_stmt_calls(stmt: &Statement, called: &mut HashSet<String>) {
    match stmt {
        Statement::Let { value, .. }
        | Statement::Set { value, .. }
        | Statement::Print(value)
        | Statement::Return(value)
        | Statement::Expression(value) => collect_expr_calls(value, called),
        Statement::SetIndex {
            target,
            index,
            value,
        } => {
            collect_expr_calls(target, called);
            collect_expr_calls(index, called);
            collect_expr_calls(value, called);
        }
        Statement::SetField { value, .. } => collect_expr_calls(value, called),
        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            collect_expr_calls(condition, called);
            for inner in then_body {
                collect_stmt_calls(inner, called);
            }
            if let Some(inner) = else_body {
                for stmt in inner {
                    collect_stmt_calls(stmt, called);
                }
            }
        }
        Statement::While { condition, body } => {
            collect_expr_calls(condition, called);
            for inner in body {
                collect_stmt_calls(inner, called);
            }
        }
        Statement::For { iterable, body, .. } => {
            collect_expr_calls(iterable, called);
            for inner in body {
                collect_stmt_calls(inner, called);
            }
        }
        Statement::Loop { body }
        | Statement::OnEvent { body, .. }
        | Statement::FunctionDef { body, .. } => {
            for inner in body {
                collect_stmt_calls(inner, called);
            }
        }
        Statement::Import { .. }
        | Statement::StructDef { .. }
        | Statement::Spawn { .. }
        | Statement::Break
        | Statement::Continue => {}
    }
}

fn collect_expr_uses(expr: &Expression, used: &mut HashSet<String>) {
    match expr {
        Expression::Identifier(name) => {
            used.insert(name.clone());
        }
        Expression::Binary { left, right, .. } => {
            collect_expr_uses(left, used);
            collect_expr_uses(right, used);
        }
        Expression::Unary { operand, .. } => collect_expr_uses(operand, used),
        Expression::Call { callee, args } => {
            collect_expr_uses(callee, used);
            for arg in args {
                collect_expr_uses(arg, used);
            }
        }
        Expression::BackendCall { args, .. } | Expression::MethodCall { args, .. } => {
            for arg in args {
                collect_expr_uses(arg, used);
            }
            if let Expression::MethodCall { target, .. } = expr {
                collect_expr_uses(target, used);
            }
        }
        Expression::List(items) => {
            for item in items {
                collect_expr_uses(item, used);
            }
        }
        Expression::Map(entries)
        | Expression::StructLiteral {
            fields: entries, ..
        } => {
            for (_, value) in entries {
                collect_expr_uses(value, used);
            }
        }
        Expression::Field { target, .. } => collect_expr_uses(target, used),
        Expression::Index { target, index } => {
            collect_expr_uses(target, used);
            collect_expr_uses(index, used);
        }
        Expression::Int(_)
        | Expression::Float(_)
        | Expression::Bool(_)
        | Expression::String(_)
        | Expression::Unit => {}
    }
}

fn collect_expr_calls(expr: &Expression, called: &mut HashSet<String>) {
    match expr {
        Expression::Call { callee, args } => {
            if let Expression::Identifier(name) = callee.as_ref() {
                called.insert(name.clone());
            }
            collect_expr_calls(callee, called);
            for arg in args {
                collect_expr_calls(arg, called);
            }
        }
        Expression::Binary { left, right, .. } => {
            collect_expr_calls(left, called);
            collect_expr_calls(right, called);
        }
        Expression::Unary { operand, .. } => collect_expr_calls(operand, called),
        Expression::BackendCall { args, .. } | Expression::MethodCall { args, .. } => {
            for arg in args {
                collect_expr_calls(arg, called);
            }
            if let Expression::MethodCall { target, .. } = expr {
                collect_expr_calls(target, called);
            }
        }
        Expression::List(items) => {
            for item in items {
                collect_expr_calls(item, called);
            }
        }
        Expression::Map(entries)
        | Expression::StructLiteral {
            fields: entries, ..
        } => {
            for (_, value) in entries {
                collect_expr_calls(value, called);
            }
        }
        Expression::Field { target, .. } => collect_expr_calls(target, called),
        Expression::Index { target, index } => {
            collect_expr_calls(target, called);
            collect_expr_calls(index, called);
        }
        Expression::Identifier(_)
        | Expression::Int(_)
        | Expression::Float(_)
        | Expression::Bool(_)
        | Expression::String(_)
        | Expression::Unit => {}
    }
}
