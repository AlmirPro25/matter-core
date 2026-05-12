//! Parser for Matter language
//! Converte tokens em AST

use matter_ast::*;
use matter_lexer::{Lexer, Span, Token};
use std::fmt;

pub struct Parser {
    tokens: Vec<Token>,
    spans: Vec<Span>,
    position: usize,
    allow_struct_literals: bool,
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl ParseError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            line: span.line,
            column: span.column,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

type ParseResult<T> = Result<T, ParseError>;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let spans = vec![Span { line: 1, column: 1 }; tokens.len()];
        Self {
            tokens,
            spans,
            position: 0,
            allow_struct_literals: true,
        }
    }

    pub fn from_source(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let spanned_tokens = lexer.tokenize_spanned();
        let tokens = spanned_tokens
            .iter()
            .map(|spanned| spanned.token.clone())
            .collect();
        let spans = spanned_tokens.iter().map(|spanned| spanned.span).collect();
        Self {
            tokens,
            spans,
            position: 0,
            allow_struct_literals: true,
        }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    fn current_span(&self) -> Span {
        self.spans
            .get(self.position)
            .copied()
            .unwrap_or(Span { line: 1, column: 1 })
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError::new(message, self.current_span())
    }

    fn expect(&mut self, expected: Token) -> ParseResult<()> {
        if self.current() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("Expected {:?}, got {:?}", expected, self.current())))
        }
    }

    pub fn parse(&mut self) -> ParseResult<Program> {
        let mut statements = Vec::new();

        while self.current() != &Token::Eof {
            // Skip extra newlines or semicolons between statements
            if matches!(self.current(), Token::Newline | Token::Semicolon) {
                self.advance();
                continue;
            }

            statements.push(self.parse_statement()?);

            // Optional semicolon after statement
            if self.current() == &Token::Semicolon {
                self.advance();
            }
        }

        Ok(Program::new(statements))
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        if self.is_energy_profile_start() {
            return self.parse_energy_profile_statement();
        }

        match self.current() {
            Token::Let => self.parse_let(),
            Token::Set => self.parse_set(),
            Token::Print => self.parse_print(),
            Token::Fn => self.parse_function(),
            Token::Struct => self.parse_struct_def(),
            Token::Import => self.parse_import(),
            Token::On => self.parse_on_event(),
            Token::Spawn => self.parse_spawn(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::Loop => self.parse_loop(),
            Token::Break => self.parse_break(),
            Token::Continue => self.parse_continue(),
            Token::Return => self.parse_return(),
            _ => {
                let expr = self.parse_expression()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    fn is_energy_profile_start(&self) -> bool {
        matches!(self.current(), Token::Ident(name) if name == "energy")
            && matches!(self.tokens.get(self.position + 1), Some(Token::Ident(name)) if name == "profile")
            && matches!(self.tokens.get(self.position + 2), Some(Token::LBrace))
    }

    fn parse_energy_profile_statement(&mut self) -> ParseResult<Statement> {
        self.advance(); // energy
        self.advance(); // profile
        let entries = self.parse_profile_entries()?;

        Ok(Statement::Expression(Expression::MethodCall {
            target: Box::new(Expression::Identifier("energy".to_string())),
            method: "profile".to_string(),
            args: vec![Expression::Map(entries)],
        }))
    }

    fn parse_profile_entries(&mut self) -> ParseResult<Vec<(String, Expression)>> {
        self.expect(Token::LBrace)?;
        let mut entries = Vec::new();

        while self.current() != &Token::RBrace {
            let key = match self.current() {
                Token::String(key) | Token::Ident(key) => key.clone(),
                _ => return Err(self.error("Expected profile key")),
            };
            self.advance();

            self.expect(Token::Colon)?;
            let value = self.parse_expression()?;
            entries.push((key, value));

            if self.current() == &Token::Comma {
                self.advance();
            }
        }

        self.expect(Token::RBrace)?;
        Ok(entries)
    }

    fn parse_type_annotation(&mut self) -> ParseResult<TypeAnnotation> {
        let mut base = match self.current() {
            Token::Ident(name) => {
                let n = name.clone();
                self.advance();

                // Generic: Name<Type, Type>
                if self.current() == &Token::Lt {
                    self.advance();
                    let mut args = Vec::new();
                    while self.current() != &Token::Gt {
                        args.push(self.parse_type_annotation()?);
                        if self.current() == &Token::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(Token::Gt)?;
                    TypeAnnotation::Generic(n, args)
                } else {
                    TypeAnnotation::Simple(n)
                }
            }
            Token::LBracket => {
                // List: [Type]
                self.advance();
                let inner = self.parse_type_annotation()?;
                self.expect(Token::RBracket)?;
                TypeAnnotation::List(Box::new(inner))
            }
            Token::Fn => {
                // Function: fn(ArgType, ArgType) -> RetType
                self.advance();
                self.expect(Token::LParen)?;
                let mut args = Vec::new();
                while self.current() != &Token::RParen {
                    args.push(self.parse_type_annotation()?);
                    if self.current() == &Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(Token::RParen)?;
                self.expect(Token::Arrow)?;
                let ret = self.parse_type_annotation()?;
                TypeAnnotation::Function(args, Box::new(ret))
            }
            _ => {
                return Err(self.error(format!(
                    "Expected type annotation, got {:?}",
                    self.current()
                )))
            }
        };

        // Postfix type modifiers
        loop {
            match self.current() {
                Token::Ident(s) if s == "?" => {
                    self.advance();
                    base = TypeAnnotation::Nullable(Box::new(base));
                }
                Token::Not => {
                    // '!' modifier
                    self.advance();
                    base = TypeAnnotation::NonNullable(Box::new(base));
                }
                Token::Ident(s) if s == "|" => {
                    self.advance();
                    let mut types = vec![base];
                    types.push(self.parse_type_annotation()?);
                    // Handle further unions: T | U | V
                    while matches!(self.current(), Token::Ident(s) if s == "|") {
                        self.advance();
                        types.push(self.parse_type_annotation()?);
                    }
                    base = TypeAnnotation::Union(types);
                }
                _ => break,
            }
        }

        Ok(base)
    }

    fn parse_let(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'let'

        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err(self.error("Expected identifier")),
        };
        self.advance();

        let type_annotation = if self.current() == &Token::Colon {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        self.expect(Token::Eq)?;

        let value = self.parse_expression()?;

        Ok(Statement::Let {
            name,
            type_annotation,
            value,
        })
    }

    fn parse_set(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'set'

        // Parse the target (could be identifier or indexed expression)
        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err(self.error("Expected identifier")),
        };
        self.advance();

        match self.current() {
            Token::LBracket => {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(Token::RBracket)?;
                self.expect(Token::Eq)?;
                let value = self.parse_expression()?;

                return Ok(Statement::SetIndex {
                    target: Expression::Identifier(name),
                    index,
                    value,
                });
            }
            Token::Dot => {
                self.advance();
                let field = match self.current() {
                    Token::Ident(field) => field.clone(),
                    _ => return Err(self.error("Expected field name")),
                };
                self.advance();
                self.expect(Token::Eq)?;
                let value = self.parse_expression()?;

                return Ok(Statement::SetField {
                    target: name,
                    field,
                    value,
                });
            }
            _ => {}
        }

        // Regular assignment: set x = value
        self.expect(Token::Eq)?;
        let value = self.parse_expression()?;

        Ok(Statement::Set { name, value })
    }

    fn parse_print(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'print'
        let expr = self.parse_expression()?;
        Ok(Statement::Print(expr))
    }

    fn parse_function(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'fn'

        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err(self.error("Expected function name")),
        };
        self.advance();

        self.expect(Token::LParen)?;

        let mut params = Vec::new();
        while self.current() != &Token::RParen {
            if let Token::Ident(param_name) = self.current().clone() {
                self.advance();

                let type_annotation = if self.current() == &Token::Colon {
                    self.advance();
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };

                params.push(Param {
                    name: param_name,
                    type_annotation,
                });

                if self.current() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        self.expect(Token::RParen)?;

        let return_type = if self.current() == &Token::Arrow {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        // Sprint 27.3: Parse effect declarations (optional)
        let effects = if self.current() == &Token::Ident("with".to_string()) {
            self.advance(); // skip 'with'
            let mut effect_list = Vec::new();

            while let Token::Ident(effect) = self.current() {
                effect_list.push(effect.clone());
                self.advance();

                if self.current() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }

            Some(effect_list)
        } else {
            None
        };

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Statement::FunctionDef {
            name,
            params,
            return_type,
            body,
            effects,
        })
    }

    fn parse_struct_def(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'struct'

        let name = match self.current() {
            Token::Ident(name) => name.clone(),
            _ => return Err(self.error("Expected struct name")),
        };
        self.advance();

        self.expect(Token::LBrace)?;
        let mut fields = Vec::new();

        while self.current() != &Token::RBrace {
            let field_name = match self.current() {
                Token::Ident(field) => field.clone(),
                _ => return Err(self.error("Expected field name")),
            };
            self.advance();

            self.expect(Token::Colon)?;

            let field_type = match self.current() {
                Token::Ident(field_type) => field_type.clone(),
                _ => return Err(self.error("Expected field type")),
            };
            self.advance();

            fields.push((field_name, field_type));

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::StructDef { name, fields })
    }

    fn parse_import(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'import'

        let path = match self.current() {
            Token::String(p) => p.clone(),
            Token::Ident(p) => p.clone(),
            _ => return Err(self.error("Expected import path")),
        };
        self.advance();

        Ok(Statement::Import { path })
    }

    fn parse_on_event(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'on'

        let event = match self.current() {
            Token::Ident(e) => e.clone(),
            _ => return Err(self.error("Expected event name")),
        };
        self.advance();

        self.expect(Token::LBrace)?;

        let body = self.parse_block()?;

        self.expect(Token::RBrace)?;

        Ok(Statement::OnEvent { event, body })
    }

    fn parse_spawn(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'spawn'

        let event = match self.current() {
            Token::Ident(event) => event.clone(),
            _ => return Err(self.error("Expected event name")),
        };
        self.advance();

        Ok(Statement::Spawn { event })
    }

    fn parse_if(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'if'

        let condition = self.parse_control_condition()?;

        self.expect(Token::LBrace)?;
        let then_body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        let else_body = if self.current() == &Token::Else {
            self.advance();
            if self.current() == &Token::If {
                // Support 'else if'
                Some(vec![self.parse_if()?])
            } else {
                self.expect(Token::LBrace)?;
                let body = self.parse_block()?;
                self.expect(Token::RBrace)?;
                Some(body)
            }
        } else {
            None
        };

        Ok(Statement::If {
            condition,
            then_body,
            else_body,
        })
    }

    fn parse_return(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'return'
        let expr = self.parse_expression()?;
        Ok(Statement::Return(expr))
    }

    fn parse_while(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'while'

        let condition = self.parse_control_condition()?;

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Statement::While { condition, body })
    }

    fn parse_for(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'for'

        let item = match self.current() {
            Token::Ident(name) => name.clone(),
            _ => return Err(self.error("Expected loop variable")),
        };
        self.advance();

        self.expect(Token::In)?;
        let allow_struct_literals = self.allow_struct_literals;
        self.allow_struct_literals = false;
        let iterable = self.parse_expression()?;
        self.allow_struct_literals = allow_struct_literals;

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Statement::For {
            item,
            iterable,
            body,
        })
    }

    fn parse_loop(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'loop'

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Statement::Loop { body })
    }

    fn parse_break(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'break'
        Ok(Statement::Break)
    }

    fn parse_continue(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'continue'
        Ok(Statement::Continue)
    }

    fn parse_block(&mut self) -> ParseResult<Vec<Statement>> {
        let mut statements = Vec::new();

        while self.current() != &Token::RBrace && self.current() != &Token::Eof {
            // Optional separator
            if matches!(self.current(), Token::Newline | Token::Semicolon) {
                self.advance();
                continue;
            }
            statements.push(self.parse_statement()?);
            if self.current() == &Token::Semicolon {
                self.advance();
            }
        }

        Ok(statements)
    }

    fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_logical_or()
    }

    fn parse_control_condition(&mut self) -> ParseResult<Expression> {
        let allow_struct_literals = self.allow_struct_literals;
        self.allow_struct_literals = false;
        let condition = self.parse_expression();
        self.allow_struct_literals = allow_struct_literals;
        condition
    }

    fn parse_logical_or(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_logical_and()?;

        while self.current() == &Token::Or {
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_comparison()?;

        while self.current() == &Token::And {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.current() {
                Token::EqEq => BinaryOp::Eq,
                Token::NotEq => BinaryOp::NotEq,
                Token::Lt => BinaryOp::Lt,
                Token::Gt => BinaryOp::Gt,
                Token::LtEq => BinaryOp::LtEq,
                Token::GtEq => BinaryOp::GtEq,
                _ => break,
            };

            self.advance();
            let right = self.parse_additive()?;

            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_multiplicative()?;

        loop {
            let op = match self.current() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };

            self.advance();
            let right = self.parse_multiplicative()?;

            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.current() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Mod,
                _ => break,
            };

            self.advance();
            let right = self.parse_unary()?;

            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> ParseResult<Expression> {
        match self.current() {
            Token::Not => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expression::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                })
            }
            Token::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expression::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current() {
                Token::LParen => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(Token::RParen)?;

                    expr = Expression::Call {
                        callee: Box::new(expr),
                        args,
                    };
                }
                Token::LBracket => {
                    // Index access: expr[index]
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(Token::RBracket)?;

                    expr = Expression::Index {
                        target: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                Token::Dot => {
                    self.advance(); // skip '.'

                    let method = match self.current() {
                        Token::Ident(m) => m.clone(),
                        Token::Set => "set".to_string(),
                        _ => return Err(self.error("Expected method name")),
                    };
                    self.advance();

                    if self.current() == &Token::LParen {
                        self.advance();
                        let args = self.parse_args()?;
                        self.expect(Token::RParen)?;

                        expr = Expression::MethodCall {
                            target: Box::new(expr),
                            method,
                            args,
                        };
                    } else {
                        expr = Expression::Field {
                            target: Box::new(expr),
                            field: method,
                        };
                    }
                }
                Token::LBrace if self.allow_struct_literals => {
                    if let Expression::Identifier(type_name) = expr {
                        let fields = self.parse_struct_literal_fields()?;
                        expr = Expression::StructLiteral { type_name, fields };
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_args(&mut self) -> ParseResult<Vec<Expression>> {
        let mut args = Vec::new();

        while self.current() != &Token::RParen {
            args.push(self.parse_expression()?);

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(args)
    }

    fn parse_primary(&mut self) -> ParseResult<Expression> {
        match self.current().clone() {
            Token::Int(n) => {
                self.advance();
                Ok(Expression::Int(n))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expression::Float(f))
            }
            Token::Bool(b) => {
                self.advance();
                Ok(Expression::Bool(b))
            }
            Token::String(s) => {
                self.advance();
                Ok(Expression::String(s))
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Expression::Identifier(name))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => {
                // List literal: [1, 2, 3]
                self.advance();
                let mut elements = Vec::new();

                while self.current() != &Token::RBracket {
                    elements.push(self.parse_expression()?);

                    if self.current() == &Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }

                self.expect(Token::RBracket)?;
                Ok(Expression::List(elements))
            }
            Token::LBrace => self.parse_map_literal(),
            _ => Err(self.error(format!("Unexpected token: {:?}", self.current()))),
        }
    }

    fn parse_map_literal(&mut self) -> ParseResult<Expression> {
        self.advance(); // skip '{'
        let mut entries = Vec::new();

        while self.current() != &Token::RBrace {
            let key = match self.current() {
                Token::String(key) | Token::Ident(key) => key.clone(),
                _ => return Err(self.error("Expected map key")),
            };
            self.advance();

            self.expect(Token::Colon)?;
            let value = self.parse_expression()?;
            entries.push((key, value));

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(Token::RBrace)?;
        Ok(Expression::Map(entries))
    }

    fn parse_struct_literal_fields(&mut self) -> ParseResult<Vec<(String, Expression)>> {
        self.advance(); // skip '{'
        let mut fields = Vec::new();

        while self.current() != &Token::RBrace {
            let field_name = match self.current() {
                Token::Ident(field) => field.clone(),
                _ => return Err(self.error("Expected struct field")),
            };
            self.advance();

            self.expect(Token::Colon)?;
            let value = self.parse_expression()?;
            fields.push((field_name, value));

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(Token::RBrace)?;
        Ok(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_let_with_type() {
        let mut parser = Parser::from_source("let x: int = 10");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::Let {
            type_annotation, ..
        } = &program.statements[0]
        {
            assert!(matches!(type_annotation, Some(TypeAnnotation::Simple(s)) if s == "int"));
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_parse_function_with_types() {
        let mut parser = Parser::from_source("fn add(a: int, b: int) -> int { return a + b }");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
        if let Statement::FunctionDef {
            params,
            return_type,
            ..
        } = &program.statements[0]
        {
            assert_eq!(params.len(), 2);
            assert!(params[0].type_annotation.is_some());
            assert!(return_type.is_some());
        } else {
            panic!("Expected FunctionDef");
        }
    }

    #[test]
    fn test_parse_float() {
        let mut parser = Parser::from_source("let pi = 3.14");
        let program = parser.parse().unwrap();
        if let Statement::Let { value, .. } = &program.statements[0] {
            assert!(matches!(value, Expression::Float(f) if (f - 3.14).abs() < 0.001));
        }
    }

    #[test]
    fn test_parse_logical_ops() {
        let mut parser = Parser::from_source("if a && b || !c { print d }");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn test_parse_else_if() {
        let mut parser = Parser::from_source("if a { b } else if c { d } else { e }");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }
}
