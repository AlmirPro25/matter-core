/// Lexer for Matter language
/// Converte código fonte em tokens

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Let,
    Set,
    Fn,
    Return,
    If,
    Else,
    On,
    Print,
    While,
    For,
    In,
    Loop,
    Break,
    Continue,
    Struct,
    Import,
    Spawn,
    
    // Literals
    Int(i64),
    String(String),
    Bool(bool),
    
    // Identifiers
    Ident(String),
    
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    
    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    
    // Special
    Newline,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    current: Option<char>,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current = chars.get(0).copied();
        Self {
            input: chars,
            position: 0,
            current,
            line: 1,
            column: 1,
        }
    }
    
    fn advance(&mut self) {
        if self.current == Some('\n') {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.position += 1;
        self.current = self.input.get(self.position).copied();
    }

    fn current_span(&self) -> Span {
        Span {
            line: self.line,
            column: self.column,
        }
    }
    
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    fn skip_comment(&mut self) {
        if self.current == Some('#') {
            while self.current.is_some() && self.current != Some('\n') {
                self.advance();
            }
        }
    }
    
    fn read_number(&mut self) -> i64 {
        let mut num = String::new();
        while let Some(ch) = self.current {
            if ch.is_ascii_digit() {
                num.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        num.parse().unwrap_or(0)
    }
    
    fn read_string(&mut self) -> String {
        let mut s = String::new();
        self.advance(); // skip opening quote
        
        while let Some(ch) = self.current {
            if ch == '"' {
                self.advance(); // skip closing quote
                break;
            }
            s.push(ch);
            self.advance();
        }
        s
    }
    
    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();
        while let Some(ch) = self.current {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        ident
    }
    
    pub fn next_token(&mut self) -> Token {
        self.next_token_spanned().token
    }

    pub fn next_token_spanned(&mut self) -> SpannedToken {
        self.skip_whitespace();
        self.skip_comment();

        let span = self.current_span();
        let token = match self.current {
            None => Token::Eof,
            Some('\n') => {
                self.advance();
                Token::Newline
            }
            Some('+') => {
                self.advance();
                Token::Plus
            }
            Some('-') => {
                self.advance();
                Token::Minus
            }
            Some('*') => {
                self.advance();
                Token::Star
            }
            Some('/') => {
                self.advance();
                Token::Slash
            }
            Some('=') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::EqEq
                } else {
                    Token::Eq
                }
            }
            Some('!') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::NotEq
                } else {
                    Token::Ident("!".to_string())
                }
            }
            Some('<') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::LtEq
                } else {
                    Token::Lt
                }
            }
            Some('>') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::GtEq
                } else {
                    Token::Gt
                }
            }
            Some('(') => {
                self.advance();
                Token::LParen
            }
            Some(')') => {
                self.advance();
                Token::RParen
            }
            Some('{') => {
                self.advance();
                Token::LBrace
            }
            Some('}') => {
                self.advance();
                Token::RBrace
            }
            Some('[') => {
                self.advance();
                Token::LBracket
            }
            Some(']') => {
                self.advance();
                Token::RBracket
            }
            Some(',') => {
                self.advance();
                Token::Comma
            }
            Some('.') => {
                self.advance();
                Token::Dot
            }
            Some(':') => {
                self.advance();
                Token::Colon
            }
            Some('"') => {
                let s = self.read_string();
                Token::String(s)
            }
            Some(ch) if ch.is_ascii_digit() => {
                let num = self.read_number();
                Token::Int(num)
            }
            Some(ch) if ch.is_alphabetic() || ch == '_' => {
                let ident = self.read_identifier();
                match ident.as_str() {
                    "let" => Token::Let,
                    "set" => Token::Set,
                    "fn" => Token::Fn,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "on" => Token::On,
                    "print" => Token::Print,
                    "while" => Token::While,
                    "for" => Token::For,
                    "in" => Token::In,
                    "loop" => Token::Loop,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "struct" => Token::Struct,
                    "import" => Token::Import,
                    "spawn" => Token::Spawn,
                    "true" => Token::Bool(true),
                    "false" => Token::Bool(false),
                    _ => Token::Ident(ident),
                }
            }
            Some(_) => {
                self.advance();
                return self.next_token_spanned();
            }
        };

        SpannedToken { token, span }
    }
    
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            if token == Token::Eof {
                tokens.push(token);
                break;
            }
            if token != Token::Newline {
                tokens.push(token);
            }
        }
        tokens
    }

    pub fn tokenize_spanned(&mut self) -> Vec<SpannedToken> {
        let mut tokens = Vec::new();
        loop {
            let spanned = self.next_token_spanned();
            if spanned.token == Token::Eof {
                tokens.push(spanned);
                break;
            }
            if spanned.token != Token::Newline {
                tokens.push(spanned);
            }
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("let x = 10");
        assert_eq!(lexer.next_token(), Token::Let);
        assert_eq!(lexer.next_token(), Token::Ident("x".to_string()));
        assert_eq!(lexer.next_token(), Token::Eq);
        assert_eq!(lexer.next_token(), Token::Int(10));
    }
    
    #[test]
    fn test_string() {
        let mut lexer = Lexer::new(r#""hello world""#);
        assert_eq!(lexer.next_token(), Token::String("hello world".to_string()));
    }
    
    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("+ - * / == != < > <= >=");
        assert_eq!(lexer.next_token(), Token::Plus);
        assert_eq!(lexer.next_token(), Token::Minus);
        assert_eq!(lexer.next_token(), Token::Star);
        assert_eq!(lexer.next_token(), Token::Slash);
        assert_eq!(lexer.next_token(), Token::EqEq);
        assert_eq!(lexer.next_token(), Token::NotEq);
        assert_eq!(lexer.next_token(), Token::Lt);
        assert_eq!(lexer.next_token(), Token::Gt);
        assert_eq!(lexer.next_token(), Token::LtEq);
        assert_eq!(lexer.next_token(), Token::GtEq);
    }

    #[test]
    fn test_import_keyword() {
        let mut lexer = Lexer::new(r#"import "math.matter""#);
        assert_eq!(lexer.next_token(), Token::Import);
        assert_eq!(lexer.next_token(), Token::String("math.matter".to_string()));
    }
}
