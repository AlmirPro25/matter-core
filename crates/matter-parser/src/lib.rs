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
    recursion_depth: usize,
}

/// Cap AST recursion so controlled input cannot blow the host OS stack
/// before a structured `ParseError` is returned (debug frames are heavy).
const MAX_RECURSION_DEPTH: usize = 64;
/// Default maximum source length in bytes (configurable via MATTER_MAX_SOURCE_BYTES).
const DEFAULT_MAX_SOURCE_BYTES: usize = 1_048_576; // 1 MiB
/// Default maximum token count after lexing (configurable via MATTER_MAX_TOKENS).
const DEFAULT_MAX_TOKENS: usize = 250_000;

fn max_source_bytes() -> usize {
    std::env::var("MATTER_MAX_SOURCE_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_MAX_SOURCE_BYTES)
}

fn max_tokens() -> usize {
    std::env::var("MATTER_MAX_TOKENS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_MAX_TOKENS)
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
            recursion_depth: 0,
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
            recursion_depth: 0,
        }
    }

    /// Build a parser and enforce source-size / token-count limits.
    pub fn from_source_checked(source: &str) -> ParseResult<Self> {
        if source.len() > max_source_bytes() {
            return Err(ParseError::new(
                format!(
                    "source too large: {} bytes (limit {})",
                    source.len(),
                    max_source_bytes()
                ),
                Span { line: 1, column: 1 },
            ));
        }
        let parser = Self::from_source(source);
        if parser.tokens.len() > max_tokens() {
            return Err(ParseError::new(
                format!(
                    "too many tokens: {} (limit {})",
                    parser.tokens.len(),
                    max_tokens()
                ),
                Span { line: 1, column: 1 },
            ));
        }
        Ok(parser)
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

    fn check_recursion(&mut self) -> ParseResult<()> {
        if self.recursion_depth > MAX_RECURSION_DEPTH {
            Err(self.error("Maximum recursion depth exceeded (AST is too deep)"))
        } else {
            Ok(())
        }
    }

    pub fn parse(&mut self) -> ParseResult<Program> {
        let mut statements = Vec::new();

        while self.current() != &Token::Eof {
            // Phase 2: reject illegal/garbage tokens instead of treating as expressions.
            if let Token::Illegal(ch) = *self.current() {
                return Err(self.error(format!(
                    "illegal character {:?} (input must be valid Matter source)",
                    ch
                )));
            }

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

        // Full input must be consumed (only Eof remaining).
        if self.current() != &Token::Eof {
            return Err(self.error(format!(
                "unexpected trailing token {:?} after complete program",
                self.current()
            )));
        }

        Ok(Program::new(statements))
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        self.recursion_depth += 1;
        self.check_recursion()?;

        let res = if self.is_energy_profile_start() {
            self.parse_energy_profile_statement()
        } else {
            match self.current() {
                Token::Let => self.parse_let(),
                Token::Set => self.parse_set(),
                Token::Print => self.parse_print(),
                Token::Fn => self.parse_function(),
                Token::Struct => self.parse_struct_def(),
                Token::Import => self.parse_import(),
                Token::Export => self.parse_export(),
                Token::On => self.parse_on_event(),
                Token::Spawn => self.parse_spawn(),
                Token::If => self.parse_if(),
                Token::While => self.parse_while(),
                Token::For => self.parse_for(),
                Token::Loop => self.parse_loop(),
                Token::Break => self.parse_break(),
                Token::Continue => self.parse_continue(),
                Token::Return => self.parse_return(),
                Token::Match => self.parse_match(),
                // Semantic honesty (0.2.0): reserved but not implemented.
                Token::Panic => Err(self.error(
                    "'panic' is a reserved word, but the panic construct is not implemented \
                     in Matter Core 0.2.0 (not a language feature of this version)",
                )),
                _ => {
                    let expr = self.parse_expression()?;
                    Ok(Statement::Expression(expr))
                }
            }
        };

        self.recursion_depth -= 1;
        res
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

                let is_compound = match self.current() {
                    Token::PlusEq => Some(BinaryOp::Add),
                    Token::MinusEq => Some(BinaryOp::Sub),
                    Token::StarEq => Some(BinaryOp::Mul),
                    Token::SlashEq => Some(BinaryOp::Div),
                    _ => None,
                };

                let value = if let Some(op) = is_compound {
                    self.advance();
                    let right_expr = self.parse_expression()?;
                    Expression::Binary {
                        left: Box::new(Expression::Index {
                            target: Box::new(Expression::Identifier(name.clone())),
                            index: Box::new(index.clone()),
                        }),
                        op,
                        right: Box::new(right_expr),
                    }
                } else {
                    self.expect(Token::Eq)?;
                    self.parse_expression()?
                };

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

                let is_compound = match self.current() {
                    Token::PlusEq => Some(BinaryOp::Add),
                    Token::MinusEq => Some(BinaryOp::Sub),
                    Token::StarEq => Some(BinaryOp::Mul),
                    Token::SlashEq => Some(BinaryOp::Div),
                    _ => None,
                };

                let value = if let Some(op) = is_compound {
                    self.advance();
                    let right_expr = self.parse_expression()?;
                    Expression::Binary {
                        left: Box::new(Expression::Field {
                            target: Box::new(Expression::Identifier(name.clone())),
                            field: field.clone(),
                        }),
                        op,
                        right: Box::new(right_expr),
                    }
                } else {
                    self.expect(Token::Eq)?;
                    self.parse_expression()?
                };

                return Ok(Statement::SetField {
                    target: name,
                    field,
                    value,
                });
            }
            _ => {}
        }

        // Regular or compound assignment: set x = value or set x += value
        let is_compound = match self.current() {
            Token::PlusEq => Some(BinaryOp::Add),
            Token::MinusEq => Some(BinaryOp::Sub),
            Token::StarEq => Some(BinaryOp::Mul),
            Token::SlashEq => Some(BinaryOp::Div),
            _ => None,
        };

        let value = if let Some(op) = is_compound {
            self.advance();
            let right_expr = self.parse_expression()?;
            Expression::Binary {
                left: Box::new(Expression::Identifier(name.clone())),
                op,
                right: Box::new(right_expr),
            }
        } else {
            self.expect(Token::Eq)?;
            self.parse_expression()?
        };

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

        match self.current() {
            // import { a, b } from "path"
            Token::LBrace => {
                self.advance();
                let mut names = Vec::new();

                while self.current() != &Token::RBrace {
                    let name = match self.current() {
                        Token::Ident(n) => n.clone(),
                        _ => return Err(self.error("Expected import name")),
                    };
                    self.advance();

                    let alias = if self.current() == &Token::As {
                        self.advance();
                        match self.current() {
                            Token::Ident(a) => {
                                let alias = a.clone();
                                self.advance();
                                Some(alias)
                            }
                            _ => return Err(self.error("Expected alias name")),
                        }
                    } else {
                        None
                    };

                    names.push(ImportName { name, alias });

                    if self.current() == &Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }

                self.expect(Token::RBrace)?;
                self.expect(Token::From)?;

                let path = match self.current() {
                    Token::String(p) => p.clone(),
                    Token::Ident(p) => p.clone(),
                    _ => return Err(self.error("Expected import path")),
                };
                self.advance();

                Ok(Statement::ImportFrom { path, names })
            }
            // import "path" as alias
            Token::String(p) => {
                let path = p.clone();
                self.advance();

                if self.current() == &Token::As {
                    self.advance();
                    let alias = match self.current() {
                        Token::Ident(a) => a.clone(),
                        _ => return Err(self.error("Expected alias name")),
                    };
                    self.advance();
                    Ok(Statement::ImportAs { path, alias })
                } else {
                    Ok(Statement::Import { path })
                }
            }
            // import ident (legacy path)
            Token::Ident(p) => {
                let path = p.clone();
                self.advance();
                Ok(Statement::Import { path })
            }
            _ => Err(self.error("Expected import path")),
        }
    }

    fn parse_export(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'export'

        // export { a, b, c }
        if self.current() == &Token::LBrace {
            self.advance();
            let mut names = Vec::new();

            while self.current() != &Token::RBrace {
                let name = match self.current() {
                    Token::Ident(n) => n.clone(),
                    _ => return Err(self.error("Expected export name")),
                };
                self.advance();
                names.push(name);

                if self.current() == &Token::Comma {
                    self.advance();
                } else {
                    break;
                }
            }

            self.expect(Token::RBrace)?;
            return Ok(Statement::Export { names });
        }

        Err(self.error("Expected 'export {{ ... }}'"))
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

    fn parse_match(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'match'

        let subject = self.parse_control_condition()?;

        self.expect(Token::LBrace)?;

        let mut arms = Vec::new();

        while self.current() != &Token::RBrace && self.current() != &Token::Eof {
            // Skip newlines between arms
            if matches!(self.current(), Token::Newline | Token::Semicolon) {
                self.advance();
                continue;
            }

            // Parse pattern expression
            let pattern = self.parse_expression()?;

            // Expect '=>' (parsed as Eq followed by Gt)
            self.expect(Token::Eq)?;
            self.expect(Token::Gt)?;

            // Parse arm body
            self.expect(Token::LBrace)?;
            let body = self.parse_block()?;
            self.expect(Token::RBrace)?;

            arms.push(MatchArm { pattern, body });

            // Optional comma between arms
            if self.current() == &Token::Comma {
                self.advance();
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Statement::Match { subject, arms })
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
        self.recursion_depth += 1;
        self.check_recursion()?;
        let res = self.parse_logical_or();
        self.recursion_depth -= 1;
        res
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
                        Token::And => "and".to_string(),
                        Token::Or => "or".to_string(),
                        Token::Not => "not".to_string(),
                        Token::Let => "let".to_string(),
                        Token::Fn => "fn".to_string(),
                        Token::Return => "return".to_string(),
                        Token::If => "if".to_string(),
                        Token::Else => "else".to_string(),
                        Token::On => "on".to_string(),
                        Token::Print => "print".to_string(),
                        Token::While => "while".to_string(),
                        Token::For => "for".to_string(),
                        Token::In => "in".to_string(),
                        Token::Loop => "loop".to_string(),
                        Token::Break => "break".to_string(),
                        Token::Continue => "continue".to_string(),
                        Token::Struct => "struct".to_string(),
                        Token::Import => "import".to_string(),
                        Token::From => "from".to_string(),
                        Token::As => "as".to_string(),
                        Token::Export => "export".to_string(),
                        Token::Match => "match".to_string(),
                        Token::Null => "null".to_string(),
                        Token::Spawn => "spawn".to_string(),
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
                Token::QuestionMark => {
                    self.advance();
                    expr = Expression::TryPropagate(Box::new(expr));
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
        // Guard nested forms early (parens/calls) so depth is enforced before
        // descending the full precedence chain again.
        self.check_recursion()?;
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
                if s.contains('{') {
                    self.parse_interpolated_string(&s)
                } else {
                    Ok(Expression::String(s))
                }
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Expression::Identifier(name))
            }
            Token::Null => {
                self.advance();
                Ok(Expression::Null)
            }
            Token::Ok => {
                self.advance();
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expression::OkExpr(Box::new(expr)))
            }
            Token::Err => {
                self.advance();
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expression::ErrExpr(Box::new(expr)))
            }
            Token::Some => {
                self.advance();
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expression::SomeExpr(Box::new(expr)))
            }
            Token::None => {
                self.advance();
                Ok(Expression::NoneExpr)
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
            Token::Fn => self.parse_lambda(),
            _ => Err(self.error(format!("Unexpected token: {:?}", self.current()))),
        }
    }

    fn parse_lambda(&mut self) -> ParseResult<Expression> {
        self.advance(); // skip 'fn'

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
                params.push(Param { name: param_name, type_annotation });
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

        self.expect(Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(Token::RBrace)?;

        Ok(Expression::Lambda { params, body })
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

    fn parse_interpolated_string(&self, s: &str) -> ParseResult<Expression> {
        let mut segments = Vec::new();
        let mut current_literal = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                // Check if it's an escaped '{{'
                if chars.peek() == Some(&'{') {
                    chars.next();
                    current_literal.push('{');
                    continue;
                }

                // We have a dynamic segment!
                // First, push the current literal if it's not empty
                if !current_literal.is_empty() {
                    segments.push(Expression::String(current_literal.clone()));
                    current_literal.clear();
                }

                // Extract the expression inside braces
                let mut expr_str = String::new();
                let mut brace_count = 1;
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '{' {
                        brace_count += 1;
                    } else if next_ch == '}' {
                        brace_count -= 1;
                        if brace_count == 0 {
                            chars.next(); // Consume '}'
                            break;
                        }
                    }
                    expr_str.push(chars.next().unwrap());
                }

                if brace_count != 0 {
                    return Err(self.error("Unmatched '{' in string interpolation"));
                }

                // Parse the expression using a sub-parser!
                let mut sub_parser = Parser::from_source(&expr_str);
                let parsed_expr = sub_parser.parse_expression().map_err(|e| {
                    self.error(format!(
                        "Failed to parse expression '{}' in string interpolation: {}",
                        expr_str, e.message
                    ))
                })?;
                segments.push(parsed_expr);
            } else if ch == '}' {
                // Check if it's an escaped '}}'
                if chars.peek() == Some(&'}') {
                    chars.next();
                    current_literal.push('}');
                } else {
                    return Err(self.error("Unmatched '}' in string literal"));
                }
            } else {
                current_literal.push(ch);
            }
        }

        if !current_literal.is_empty() {
            segments.push(Expression::String(current_literal));
        }

        if segments.is_empty() {
            return Ok(Expression::String("".to_string()));
        }

        // Concatenate all segments with addition
        let mut result = segments.remove(0);
        for segment in segments {
            result = Expression::Binary {
                left: Box::new(result),
                op: BinaryOp::Add,
                right: Box::new(segment),
            };
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_let_with_type() {
        // Parsing still accepts annotations; Core 0.2.0 rejects them at semantic check.
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
    fn test_panic_reserved_not_implemented() {
        let mut parser = Parser::from_source("panic");
        let err = parser.parse().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("reserved") || msg.contains("not implemented"),
            "unexpected: {}",
            msg
        );
    }

    #[test]
    fn test_panic_with_call_still_not_implemented() {
        let mut parser = Parser::from_source(r#"panic("boom")"#);
        let err = parser.parse().unwrap_err();
        assert!(
            err.to_string().contains("not implemented")
                || err.to_string().contains("reserved"),
            "{}",
            err
        );
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
            assert!(
                matches!(value, Expression::Float(f) if (f - std::f64::consts::PI).abs() < 0.01)
            );
        }
    }

    #[test]
    fn test_parse_logical_ops() {
        let mut parser = Parser::from_source("if a && b || !c { print d }");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn test_parse_word_logical_ops() {
        let mut parser = Parser::from_source("if a and b or not c { print d }");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn test_parse_else_if() {
        let mut parser = Parser::from_source("if a { b } else if c { d } else { e }");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn test_reject_illegal_character() {
        let mut parser = Parser::from_source("let x = 1 @ 2");
        let err = parser.parse().expect_err("illegal @ must fail");
        assert!(
            err.message.contains("illegal") || err.message.contains('@'),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn test_reject_garbage_suffix() {
        let mut parser = Parser::from_source("print 1 ` junk");
        assert!(parser.parse().is_err());
    }

    #[test]
    fn test_empty_source_ok() {
        let mut parser = Parser::from_source("");
        let program = parser.parse().unwrap();
        assert!(program.statements.is_empty());
    }

    #[test]
    fn test_truncated_if_rejected() {
        let mut parser = Parser::from_source("if true {");
        assert!(parser.parse().is_err());
    }

    #[test]
    fn test_from_source_checked_respects_source_limit() {
        std::env::set_var("MATTER_MAX_SOURCE_BYTES", "8");
        let result = Parser::from_source_checked("print 12345");
        std::env::remove_var("MATTER_MAX_SOURCE_BYTES");
        match result {
            Ok(_) => panic!("expected source limit error"),
            Err(err) => {
                assert!(err.message.contains("source too large"), "{}", err.message);
            }
        }
    }

    #[test]
    fn test_valid_core_program_still_parses() {
        let src = r#"
fn add(a, b) {
    return a + b
}
print add(2, 3)
"#;
        let mut parser = Parser::from_source_checked(src).unwrap();
        let program = parser.parse().unwrap();
        assert!(program.statements.len() >= 2);
    }

    #[test]
    fn test_deep_nesting_hits_recursion_limit() {
        // Nested parens beyond MAX_RECURSION_DEPTH (64) must yield ParseError,
        // not an OS stack overflow / panic.
        let mut src = String::from("print ");
        for _ in 0..120 {
            src.push('(');
        }
        src.push('1');
        for _ in 0..120 {
            src.push(')');
        }
        let mut parser = Parser::from_source(&src);
        let err = parser.parse().expect_err("deep nest must fail");
        assert!(
            err.message.to_lowercase().contains("recursion")
                || err.message.to_lowercase().contains("depth"),
            "unexpected: {}",
            err.message
        );
    }
}
