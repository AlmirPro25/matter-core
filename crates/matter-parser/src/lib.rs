/// Parser for Matter language
/// Converte tokens em AST

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
            Err(self.error(format!(
                "Expected {:?}, got {:?}",
                expected,
                self.current()
            )))
        }
    }
    
    pub fn parse(&mut self) -> ParseResult<Program> {
        let mut statements = Vec::new();
        
        while self.current() != &Token::Eof {
            statements.push(self.parse_statement()?);
        }
        
        Ok(Program::new(statements))
    }
    
    fn parse_statement(&mut self) -> ParseResult<Statement> {
        match self.current() {
            Token::Let => self.parse_let(),
            Token::Set => self.parse_set(),
            Token::Print => self.parse_print(),
            Token::Fn => self.parse_function(),
            Token::Struct => self.parse_struct_def(),
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
    
    fn parse_let(&mut self) -> ParseResult<Statement> {
        self.advance(); // skip 'let'
        
        let name = match self.current() {
            Token::Ident(n) => n.clone(),
            _ => return Err(self.error("Expected identifier")),
        };
        self.advance();
        
        self.expect(Token::Eq)?;
        
        let value = self.parse_expression()?;
        
        Ok(Statement::Let { name, value })
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

                return Ok(Statement::SetField { target: name, field, value });
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
            if let Token::Ident(param) = self.current() {
                params.push(param.clone());
                self.advance();
                
                if self.current() == &Token::Comma {
                    self.advance();
                }
            } else {
                break;
            }
        }
        
        self.expect(Token::RParen)?;
        self.expect(Token::LBrace)?;
        
        let body = self.parse_block()?;
        
        self.expect(Token::RBrace)?;
        
        Ok(Statement::FunctionDef { name, params, body })
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
            self.expect(Token::LBrace)?;
            let body = self.parse_block()?;
            self.expect(Token::RBrace)?;
            Some(body)
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
            statements.push(self.parse_statement()?);
        }
        
        Ok(statements)
    }
    
    fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_comparison()
    }

    fn parse_control_condition(&mut self) -> ParseResult<Expression> {
        let allow_struct_literals = self.allow_struct_literals;
        self.allow_struct_literals = false;
        let condition = self.parse_expression();
        self.allow_struct_literals = allow_struct_literals;
        condition
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
        let mut left = self.parse_call()?;
        
        loop {
            let op = match self.current() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => break,
            };
            
            self.advance();
            let right = self.parse_call()?;
            
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
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
            _ => Err(self.error(format!(
                "Unexpected token: {:?}",
                self.current()
            ))),
        }
    }

    fn parse_map_literal(&mut self) -> ParseResult<Expression> {
        self.advance(); // skip '{'
        let mut entries = Vec::new();

        while self.current() != &Token::RBrace {
            let key = match self.current() {
                Token::String(key) | Token::Ident(key) => key.clone(),
                _ => {
                    return Err(self.error("Expected map key"))
                }
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
    fn test_parse_let() {
        let mut parser = Parser::from_source("let x = 10");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }
    
    #[test]
    fn test_parse_function() {
        let mut parser = Parser::from_source("fn add(a, b) { return a + b }");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }
    
    #[test]
    fn test_parse_backend_call() {
        let mut parser = Parser::from_source(r#"agent.say("hello")"#);
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn test_parse_list_literal_and_methods() {
        let mut parser = Parser::from_source("let nums = [1, 2, 3]\nnums.push(4)\nprint nums.len()");
        let program = parser.parse().unwrap();

        assert_eq!(program.statements.len(), 3);
        assert!(matches!(
            &program.statements[0],
            Statement::Let {
                value: Expression::List(elements),
                ..
            } if elements.len() == 3
        ));
        assert!(matches!(
            &program.statements[1],
            Statement::Expression(Expression::MethodCall { method, .. }) if method == "push"
        ));
    }

    #[test]
    fn test_parse_map_literal() {
        let mut parser = Parser::from_source(r#"let user = { "name": "Alice", "age": 30 }"#);
        let program = parser.parse().unwrap();

        assert!(matches!(
            &program.statements[0],
            Statement::Let {
                value: Expression::Map(entries),
                ..
            } if entries.len() == 2
        ));
    }

    #[test]
    fn test_parse_struct_definition_and_literal() {
        let source = r#"
struct User { name: string, age: int }
let user = User { name: "Ana", age: 20 }
print user.name
"#;
        let mut parser = Parser::from_source(source);
        let program = parser.parse().unwrap();

        assert!(matches!(
            &program.statements[0],
            Statement::StructDef { name, fields } if name == "User" && fields.len() == 2
        ));
        assert!(matches!(
            &program.statements[1],
            Statement::Let {
                value: Expression::StructLiteral { type_name, fields },
                ..
            } if type_name == "User" && fields.len() == 2
        ));
        assert!(matches!(
            &program.statements[2],
            Statement::Print(Expression::Field { field, .. }) if field == "name"
        ));
    }

    #[test]
    fn test_parse_error_reports_line_and_column() {
        let mut parser = Parser::from_source("let x = 1\nlet y = ");
        let error = parser.parse().unwrap_err();

        assert_eq!(error.line, 2);
        assert_eq!(error.column, 9);
        assert!(error.message.contains("Unexpected token"));
    }

    #[test]
    fn test_parse_for_loop() {
        let mut parser = Parser::from_source("for item in [1, 2] { print item }");
        let program = parser.parse().unwrap();

        assert!(matches!(
            &program.statements[0],
            Statement::For { item, body, .. } if item == "item" && body.len() == 1
        ));
    }

    #[test]
    fn test_parse_identifier_comparison_condition() {
        let mut parser = Parser::from_source("fn min(a, b) { if a < b { return a } return b }");
        let program = parser.parse().unwrap();

        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn test_parse_keyword_method_name() {
        let mut parser = Parser::from_source(r#"store.set("counter", 1)"#);
        let program = parser.parse().unwrap();

        assert!(matches!(
            &program.statements[0],
            Statement::Expression(Expression::MethodCall { method, .. }) if method == "set"
        ));
    }

    #[test]
    fn test_parse_spawn_event() {
        let mut parser = Parser::from_source("spawn tick");
        let program = parser.parse().unwrap();

        assert!(matches!(
            &program.statements[0],
            Statement::Spawn { event } if event == "tick"
        ));
    }
}
