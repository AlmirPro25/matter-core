//! Lexer for Matter language
//! Converte código fonte em tokens

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
    From,
    As,
    Export,
    Match,
    Null,
    Spawn,
    Ok,
    Err,
    Some,
    None,
    Panic,

    // Literals
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),

    // Identifiers
    Ident(String),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent, // %
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,   // &&
    Or,    // ||
    Not,   // !
    Arrow, // ->
    QuestionMark, // ?

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
    Semicolon,

    // Special
    Newline,
    Eof,
    /// Unrecognized character (must be rejected by parser).
    Illegal(char),
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
        let current = chars.first().copied();
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

    fn peek(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
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
        // Suporte para comentários # e //
        if self.current == Some('#') {
            while self.current.is_some() && self.current != Some('\n') {
                self.advance();
            }
        } else if self.current == Some('/') {
            // Peek ahead para ver se é //
            if self.position + 1 < self.input.len() && self.input[self.position + 1] == '/' {
                // Skip //
                self.advance();
                self.advance();
                // Skip até o fim da linha
                while self.current.is_some() && self.current != Some('\n') {
                    self.advance();
                }
            }
        }
    }

    fn read_number(&mut self) -> Token {
        let mut num = String::new();
        while let Some(ch) = self.current {
            if ch.is_ascii_digit() {
                num.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check for decimal point (float)
        if self.current == Some('.') {
            // Peek ahead to make sure it's a digit after the dot (not a method call)
            if let Some(next) = self.peek() {
                if next.is_ascii_digit() {
                    num.push('.');
                    self.advance(); // skip '.'
                    while let Some(ch) = self.current {
                        if ch.is_ascii_digit() {
                            num.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    return Token::Float(num.parse().unwrap_or(0.0));
                }
            }
        }

        Token::Int(num.parse().unwrap_or(0))
    }

    fn read_string(&mut self) -> String {
        let mut s = String::new();
        self.advance(); // skip opening quote

        while let Some(ch) = self.current {
            if ch == '\\' {
                // Escape sequences
                self.advance();
                match self.current {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    Some('0') => s.push('\0'),
                    Some(other) => {
                        s.push('\\');
                        s.push(other);
                    }
                    None => break,
                }
                self.advance();
            } else if ch == '"' {
                self.advance(); // skip closing quote
                break;
            } else {
                s.push(ch);
                self.advance();
            }
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
        loop {
            self.skip_whitespace();

            // Check for comments before processing tokens
            let before_pos = self.position;
            self.skip_comment();

            // If we skipped a comment, loop again to skip any whitespace after it
            if self.position != before_pos {
                continue;
            }
            break;
        }

        let span = self.current_span();
        let token = match self.current {
            None => Token::Eof,
            Some('\n') => {
                self.advance();
                Token::Newline
            }
            Some('+') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::PlusEq
                } else {
                    Token::Plus
                }
            }
            Some('-') => {
                self.advance();
                if self.current == Some('>') {
                    self.advance();
                    Token::Arrow
                } else if self.current == Some('=') {
                    self.advance();
                    Token::MinusEq
                } else {
                    Token::Minus
                }
            }
            Some('*') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::StarEq
                } else {
                    Token::Star
                }
            }
            Some('/') => {
                self.advance();
                if self.current == Some('=') {
                    self.advance();
                    Token::SlashEq
                } else {
                    Token::Slash
                }
            }
            Some('%') => {
                self.advance();
                Token::Percent
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
                    Token::Not
                }
            }
            Some('&') => {
                self.advance();
                if self.current == Some('&') {
                    self.advance();
                    Token::And
                } else {
                    // Single & not used, treat as identifier-like
                    Token::Ident("&".to_string())
                }
            }
            Some('|') => {
                self.advance();
                if self.current == Some('|') {
                    self.advance();
                    Token::Or
                } else {
                    // Single | for union types
                    Token::Ident("|".to_string())
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
            Some(';') => {
                self.advance();
                Token::Semicolon
            }
            Some('?') => {
                self.advance();
                Token::QuestionMark
            }
            Some('"') => {
                let s = self.read_string();
                Token::String(s)
            }
            Some(ch) if ch.is_ascii_digit() => self.read_number(),
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
                    "from" => Token::From,
                    "as" => Token::As,
                    "export" => Token::Export,
                    "spawn" => Token::Spawn,
                    "match" => Token::Match,
                    "null" => Token::Null,
                    "ok" => Token::Ok,
                    "err" => Token::Err,
                    "some" => Token::Some,
                    "none" => Token::None,
                    "panic" => Token::Panic,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "not" => Token::Not,
                    "true" => Token::Bool(true),
                    "false" => Token::Bool(false),
                    _ => Token::Ident(ident),
                }
            }
            Some(ch) => {
                // Phase 2: do not silently skip garbage characters.
                self.advance();
                Token::Illegal(ch)
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

    #[test]
    fn test_float_literal() {
        let mut lexer = Lexer::new("3.14");
        assert_eq!(lexer.next_token(), Token::Float(3.15 - 0.01));
    }

    #[test]
    fn test_float_integer_distinction() {
        let mut lexer = Lexer::new("42 3.14 100");
        assert_eq!(lexer.next_token(), Token::Int(42));
        assert_eq!(lexer.next_token(), Token::Float(3.15 - 0.01));
        assert_eq!(lexer.next_token(), Token::Int(100));
    }

    #[test]
    fn test_logical_operators() {
        let mut lexer = Lexer::new("&& || !");
        assert_eq!(lexer.next_token(), Token::And);
        assert_eq!(lexer.next_token(), Token::Or);
        assert_eq!(lexer.next_token(), Token::Not);
    }

    #[test]
    fn test_word_logical_operators() {
        let mut lexer = Lexer::new("and or not");
        assert_eq!(lexer.next_token(), Token::And);
        assert_eq!(lexer.next_token(), Token::Or);
        assert_eq!(lexer.next_token(), Token::Not);
    }

    #[test]
    fn test_modulus() {
        let mut lexer = Lexer::new("10 % 3");
        assert_eq!(lexer.next_token(), Token::Int(10));
        assert_eq!(lexer.next_token(), Token::Percent);
        assert_eq!(lexer.next_token(), Token::Int(3));
    }

    #[test]
    fn test_arrow() {
        let mut lexer = Lexer::new("fn add(a, b) -> int");
        assert_eq!(lexer.next_token(), Token::Fn);
        assert_eq!(lexer.next_token(), Token::Ident("add".to_string()));
        assert_eq!(lexer.next_token(), Token::LParen);
        assert_eq!(lexer.next_token(), Token::Ident("a".to_string()));
        assert_eq!(lexer.next_token(), Token::Comma);
        assert_eq!(lexer.next_token(), Token::Ident("b".to_string()));
        assert_eq!(lexer.next_token(), Token::RParen);
        assert_eq!(lexer.next_token(), Token::Arrow);
        assert_eq!(lexer.next_token(), Token::Ident("int".to_string()));
    }

    #[test]
    fn test_string_escape_sequences() {
        let mut lexer = Lexer::new(r#""hello\nworld""#);
        assert_eq!(
            lexer.next_token(),
            Token::String("hello\nworld".to_string())
        );
    }

    #[test]
    fn test_semicolon() {
        let mut lexer = Lexer::new("let x = 10;");
        assert_eq!(lexer.next_token(), Token::Let);
        assert_eq!(lexer.next_token(), Token::Ident("x".to_string()));
        assert_eq!(lexer.next_token(), Token::Eq);
        assert_eq!(lexer.next_token(), Token::Int(10));
        assert_eq!(lexer.next_token(), Token::Semicolon);
    }

    #[test]
    fn test_illegal_char_is_not_skipped() {
        let mut lexer = Lexer::new("let x = 1 @ 2");
        assert_eq!(lexer.next_token(), Token::Let);
        assert_eq!(lexer.next_token(), Token::Ident("x".to_string()));
        assert_eq!(lexer.next_token(), Token::Eq);
        assert_eq!(lexer.next_token(), Token::Int(1));
        assert_eq!(lexer.next_token(), Token::Illegal('@'));
        assert_eq!(lexer.next_token(), Token::Int(2));
        assert_eq!(lexer.next_token(), Token::Eof);
    }

    #[test]
    fn test_illegal_backtick_and_at() {
        // '#' starts a line comment and is intentionally not Illegal.
        let mut lexer = Lexer::new("`@");
        assert_eq!(lexer.next_token(), Token::Illegal('`'));
        assert_eq!(lexer.next_token(), Token::Illegal('@'));
        assert_eq!(lexer.next_token(), Token::Eof);
    }

    #[test]
    fn test_empty_source_is_eof() {
        let mut lexer = Lexer::new("");
        assert_eq!(lexer.next_token(), Token::Eof);
    }

    #[test]
    fn test_unicode_ident_and_valid_program_tokens() {
        // Valid Matter still lexes identifiers that are ASCII-based;
        // non-ASCII garbage is Illegal so the parser can reject.
        let mut lexer = Lexer::new("let x = 1");
        assert_eq!(lexer.next_token(), Token::Let);
        assert_eq!(lexer.next_token(), Token::Ident("x".to_string()));
        assert_eq!(lexer.next_token(), Token::Eq);
        assert_eq!(lexer.next_token(), Token::Int(1));
        assert_eq!(lexer.next_token(), Token::Eof);

        let mut bad = Lexer::new("€");
        assert!(matches!(bad.next_token(), Token::Illegal(_)));
    }
}
