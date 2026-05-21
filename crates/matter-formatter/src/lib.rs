use matter_ast::*;
use matter_parser::Parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatConfig {
    pub indent_size: usize,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self { indent_size: 4 }
    }
}

pub struct Formatter {
    config: FormatConfig,
}

impl Formatter {
    pub fn new(config: FormatConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(FormatConfig::default())
    }

    pub fn format(&self, source: &str) -> Result<String, String> {
        let mut parser = Parser::from_source(source);
        let program = parser.parse().map_err(|e| e.to_string())?;
        Ok(self.format_program(&program))
    }

    fn format_program(&self, program: &Program) -> String {
        program
            .statements
            .iter()
            .map(|stmt| self.format_statement(stmt, 0))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_statement(&self, stmt: &Statement, indent: usize) -> String {
        let i = " ".repeat(self.config.indent_size * indent);
        match stmt {
            Statement::Let {
                name,
                type_annotation,
                value,
            } => {
                let type_suffix = type_annotation
                    .as_ref()
                    .map(|ty| format!(": {}", format_type_annotation(ty)))
                    .unwrap_or_default();
                format!(
                    "{}let {}{} = {}",
                    i,
                    name,
                    type_suffix,
                    self.format_expr(value)
                )
            }
            Statement::Set { name, value } => {
                format!("{}set {} = {}", i, name, self.format_expr(value))
            }
            Statement::SetIndex {
                target,
                index,
                value,
            } => format!(
                "{}set {}[{}] = {}",
                i,
                self.format_expr(target),
                self.format_expr(index),
                self.format_expr(value)
            ),
            Statement::SetField {
                target,
                field,
                value,
            } => {
                format!(
                    "{}set {}.{} = {}",
                    i,
                    target,
                    field,
                    self.format_expr(value)
                )
            }
            Statement::Print(value) => format!("{}print {}", i, self.format_expr(value)),
            Statement::FunctionDef {
                name,
                params,
                return_type,
                body,
                effects,
            } => {
                let formatted_params = params
                    .iter()
                    .map(format_param)
                    .collect::<Vec<_>>()
                    .join(", ");
                let return_suffix = return_type
                    .as_ref()
                    .map(|ty| format!(" -> {}", format_type_annotation(ty)))
                    .unwrap_or_default();
                let mut out = format!("{}fn {}({}){} ", i, name, formatted_params, return_suffix);

                // Sprint 27.3: Format effects if present
                if let Some(effect_list) = effects {
                    if !effect_list.is_empty() {
                        out.push_str(&format!("with {} ", effect_list.join(", ")));
                    }
                }

                out.push('{');
                if !body.is_empty() {
                    out.push('\n');
                    for stmt in body {
                        out.push_str(&self.format_statement(stmt, indent + 1));
                        out.push('\n');
                    }
                    out.push_str(&i);
                }
                out.push('}');
                out
            }
            Statement::StructDef { name, fields } => {
                let fields = fields
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}struct {} {{ {} }}", i, name, fields)
            }
            Statement::Import { path } => format!("{}import \"{}\"", i, path),
            Statement::OnEvent { event, body } => {
                let mut out = format!("{}on {} {{", i, event);
                if !body.is_empty() {
                    out.push('\n');
                    for stmt in body {
                        out.push_str(&self.format_statement(stmt, indent + 1));
                        out.push('\n');
                    }
                    out.push_str(&i);
                }
                out.push('}');
                out
            }
            Statement::Spawn { event } => format!("{}spawn {}", i, event),
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                let mut out = format!("{}if {} {{", i, self.format_expr(condition));
                if !then_body.is_empty() {
                    out.push('\n');
                    for stmt in then_body {
                        out.push_str(&self.format_statement(stmt, indent + 1));
                        out.push('\n');
                    }
                    out.push_str(&i);
                }
                out.push('}');
                if let Some(else_body) = else_body {
                    out.push_str(" else {");
                    if !else_body.is_empty() {
                        out.push('\n');
                        for stmt in else_body {
                            out.push_str(&self.format_statement(stmt, indent + 1));
                            out.push('\n');
                        }
                        out.push_str(&i);
                    }
                    out.push('}');
                }
                out
            }
            Statement::While { condition, body } => {
                let mut out = format!("{}while {} {{", i, self.format_expr(condition));
                if !body.is_empty() {
                    out.push('\n');
                    for stmt in body {
                        out.push_str(&self.format_statement(stmt, indent + 1));
                        out.push('\n');
                    }
                    out.push_str(&i);
                }
                out.push('}');
                out
            }
            Statement::For {
                item,
                iterable,
                body,
            } => {
                let mut out = format!("{}for {} in {} {{", i, item, self.format_expr(iterable));
                if !body.is_empty() {
                    out.push('\n');
                    for stmt in body {
                        out.push_str(&self.format_statement(stmt, indent + 1));
                        out.push('\n');
                    }
                    out.push_str(&i);
                }
                out.push('}');
                out
            }
            Statement::Loop { body } => {
                let mut out = format!("{}loop {{", i);
                if !body.is_empty() {
                    out.push('\n');
                    for stmt in body {
                        out.push_str(&self.format_statement(stmt, indent + 1));
                        out.push('\n');
                    }
                    out.push_str(&i);
                }
                out.push('}');
                out
            }
            Statement::Break => format!("{}break", i),
            Statement::Continue => format!("{}continue", i),
            Statement::Return(value) => format!("{}return {}", i, self.format_expr(value)),
            Statement::Expression(expr) => format!("{}{}", i, self.format_expr(expr)),
            Statement::Match { subject, arms } => {
                let mut out = format!("{}match {} {{", i, self.format_expr(subject));
                if !arms.is_empty() {
                    out.push('\n');
                    for arm in arms {
                        out.push_str(&format!(
                            "{}    {} => {{\n",
                            i,
                            self.format_expr(&arm.pattern)
                        ));
                        for stmt in &arm.body {
                            out.push_str(&self.format_statement(stmt, indent + 2));
                            out.push('\n');
                        }
                        out.push_str(&format!("{}    }}\n", i));
                    }
                    out.push_str(&i);
                }
                out.push('}');
                out
            }
        }
    }

    fn format_expr(&self, expr: &Expression) -> String {
        match expr {
            Expression::Int(v) => v.to_string(),
            Expression::Float(v) => v.to_string(),
            Expression::Bool(v) => v.to_string(),
            Expression::String(v) => format!("\"{}\"", v),
            Expression::Unit => "()".to_string(),
            Expression::Identifier(name) => name.clone(),
            Expression::Unary { op, operand } => {
                format!("{}{}", format_unaryop(op), self.format_expr(operand))
            }
            Expression::Binary { left, op, right } => format!(
                "{} {} {}",
                self.format_expr(left),
                format_binop(op.clone()),
                self.format_expr(right)
            ),
            Expression::Call { callee, args } => format!(
                "{}({})",
                self.format_expr(callee),
                args.iter()
                    .map(|a| self.format_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expression::BackendCall {
                backend,
                method,
                args,
            } => format!(
                "{}.{}({})",
                backend,
                method,
                args.iter()
                    .map(|a| self.format_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expression::List(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(|a| self.format_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expression::Map(entries) => format!(
                "{{{}}}",
                entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.format_expr(v)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expression::StructLiteral { type_name, fields } => format!(
                "{} {{ {} }}",
                type_name,
                fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.format_expr(v)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expression::Field { target, field } => {
                format!("{}.{}", self.format_expr(target), field)
            }
            Expression::Index { target, index } => {
                format!("{}[{}]", self.format_expr(target), self.format_expr(index))
            }
            Expression::MethodCall {
                target,
                method,
                args,
            } => format!(
                "{}.{}({})",
                self.format_expr(target),
                method,
                args.iter()
                    .map(|a| self.format_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expression::Null => "null".to_string(),
        }
    }
}

fn format_binop(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::LtEq => "<=",
        BinaryOp::GtEq => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

fn format_unaryop(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "!",
        UnaryOp::Neg => "-",
    }
}

fn format_param(param: &Param) -> String {
    param
        .type_annotation
        .as_ref()
        .map(|ty| format!("{}: {}", param.name, format_type_annotation(ty)))
        .unwrap_or_else(|| param.name.clone())
}

fn format_type_annotation(ty: &TypeAnnotation) -> String {
    match ty {
        TypeAnnotation::Simple(name) => name.clone(),
        TypeAnnotation::Nullable(inner) => format!("{}?", format_type_annotation(inner)),
        TypeAnnotation::NonNullable(inner) => format!("{}!", format_type_annotation(inner)),
        TypeAnnotation::List(inner) => format!("[{}]", format_type_annotation(inner)),
        TypeAnnotation::Map(key, value) => {
            format!(
                "map<{}, {}>",
                format_type_annotation(key),
                format_type_annotation(value)
            )
        }
        TypeAnnotation::Union(types) => types
            .iter()
            .map(format_type_annotation)
            .collect::<Vec<_>>()
            .join(" | "),
        TypeAnnotation::Function(params, ret) => {
            let params = params
                .iter()
                .map(format_type_annotation)
                .collect::<Vec<_>>()
                .join(", ");
            format!("fn({}) -> {}", params, format_type_annotation(ret))
        }
        TypeAnnotation::Generic(name, args) => {
            let args = args
                .iter()
                .map(format_type_annotation)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", name, args)
        }
    }
}
