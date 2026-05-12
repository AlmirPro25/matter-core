//! Matter Error System
//! Sistema de erros estruturado com stack traces e contexto

use std::fmt;

/// Tipo de erro
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    Lexer,
    Parser,
    Semantic,
    Runtime,
    Backend,
    IO,
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorType::Lexer => write!(f, "Lexer Error"),
            ErrorType::Parser => write!(f, "Parse Error"),
            ErrorType::Semantic => write!(f, "Semantic Error"),
            ErrorType::Runtime => write!(f, "Runtime Error"),
            ErrorType::Backend => write!(f, "Backend Error"),
            ErrorType::IO => write!(f, "IO Error"),
        }
    }
}

/// Localização no código fonte
#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(file: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }

    pub fn unknown() -> Self {
        Self {
            file: "<unknown>".to_string(),
            line: 0,
            column: 0,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// Frame de stack trace
#[derive(Debug, Clone, PartialEq)]
pub struct StackFrame {
    pub function: String,
    pub location: SourceLocation,
}

impl StackFrame {
    pub fn new(function: impl Into<String>, location: SourceLocation) -> Self {
        Self {
            function: function.into(),
            location,
        }
    }
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "  at {} ({})", self.function, self.location)
    }
}

/// Erro Matter completo
#[derive(Debug, Clone)]
pub struct MatterError {
    pub error_type: ErrorType,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub stack_trace: Vec<StackFrame>,
    pub hint: Option<String>,
    pub source_snippet: Option<String>,
}

impl MatterError {
    pub fn new(error_type: ErrorType, message: impl Into<String>) -> Self {
        Self {
            error_type,
            message: message.into(),
            location: None,
            stack_trace: Vec::new(),
            hint: None,
            source_snippet: None,
        }
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.source_snippet = Some(snippet.into());
        self
    }

    pub fn push_stack_frame(&mut self, frame: StackFrame) {
        self.stack_trace.push(frame);
    }

    /// Formata o erro para display legível
    pub fn format_error(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!("\n{}: {}\n", self.error_type, self.message));

        // Location
        if let Some(loc) = &self.location {
            output.push_str(&format!("  --> {}\n", loc));
        }

        // Source snippet
        if let Some(snippet) = &self.source_snippet {
            output.push('\n');
            output.push_str(snippet);
            output.push('\n');
        }

        // Hint
        if let Some(hint) = &self.hint {
            output.push_str(&format!("\nHint: {}\n", hint));
        }

        // Stack trace
        if !self.stack_trace.is_empty() {
            output.push_str("\nStack trace:\n");
            for frame in &self.stack_trace {
                output.push_str(&format!("{}\n", frame));
            }
        }

        output
    }

    /// Converte para JSON
    pub fn to_json(&self) -> String {
        let location_json = if let Some(loc) = &self.location {
            format!(
                r#"{{"file":"{}","line":{},"column":{}}}"#,
                json_escape(&loc.file),
                loc.line,
                loc.column
            )
        } else {
            "null".to_string()
        };

        let hint_json = if let Some(hint) = &self.hint {
            format!(r#""{}""#, json_escape(hint))
        } else {
            "null".to_string()
        };

        let stack_json: Vec<String> = self
            .stack_trace
            .iter()
            .map(|frame| {
                format!(
                    r#"{{"function":"{}","location":{{"file":"{}","line":{},"column":{}}}}}"#,
                    json_escape(&frame.function),
                    json_escape(&frame.location.file),
                    frame.location.line,
                    frame.location.column
                )
            })
            .collect();

        format!(
            r#"{{"type":"{}","message":"{}","location":{},"hint":{},"stack":[{}]}}"#,
            format!("{:?}", self.error_type).to_lowercase(),
            json_escape(&self.message),
            location_json,
            hint_json,
            stack_json.join(",")
        )
    }
}

impl fmt::Display for MatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_error())
    }
}

impl std::error::Error for MatterError {}

/// Helper para escape de JSON
fn json_escape(s: &str) -> String {
    let mut escaped = String::new();
    for ch in s.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

/// Builders para tipos comuns de erro
impl MatterError {
    pub fn lexer_error(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::new(ErrorType::Lexer, message)
            .with_location(SourceLocation::new("<source>", line, column))
    }

    pub fn parser_error(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self::new(ErrorType::Parser, message)
            .with_location(SourceLocation::new("<source>", line, column))
    }

    pub fn semantic_error(message: impl Into<String>) -> Self {
        Self::new(ErrorType::Semantic, message)
    }

    pub fn runtime_error(message: impl Into<String>) -> Self {
        Self::new(ErrorType::Runtime, message)
    }

    pub fn backend_error(backend: &str, message: impl Into<String>) -> Self {
        Self::new(
            ErrorType::Backend,
            format!("Backend '{}': {}", backend, message.into()),
        )
    }

    pub fn io_error(message: impl Into<String>) -> Self {
        Self::new(ErrorType::IO, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = MatterError::parser_error("unexpected token", 10, 5);
        assert_eq!(error.error_type, ErrorType::Parser);
        assert_eq!(error.message, "unexpected token");
        assert!(error.location.is_some());
    }

    #[test]
    fn test_error_with_hint() {
        let error =
            MatterError::semantic_error("undefined variable 'x'").with_hint("Did you mean 'y'?");
        assert!(error.hint.is_some());
        assert_eq!(error.hint.unwrap(), "Did you mean 'y'?");
    }

    #[test]
    fn test_stack_trace() {
        let mut error = MatterError::runtime_error("division by zero");
        error.push_stack_frame(StackFrame::new(
            "divide",
            SourceLocation::new("test.matter", 10, 5),
        ));
        error.push_stack_frame(StackFrame::new(
            "main",
            SourceLocation::new("test.matter", 20, 1),
        ));
        assert_eq!(error.stack_trace.len(), 2);
    }

    #[test]
    fn test_json_output() {
        let error = MatterError::parser_error("unexpected token", 10, 5).with_hint("Expected '}'");
        let json = error.to_json();
        assert!(json.contains("\"type\":\"parser\""));
        assert!(json.contains("\"message\":\"unexpected token\""));
        assert!(json.contains("\"line\":10"));
    }

    #[test]
    fn test_format_error() {
        let error = MatterError::runtime_error("division by zero")
            .with_location(SourceLocation::new("test.matter", 15, 10))
            .with_hint("Cannot divide by zero");
        let formatted = error.format_error();
        assert!(formatted.contains("Runtime Error"));
        assert!(formatted.contains("division by zero"));
        assert!(formatted.contains("test.matter:15:10"));
        assert!(formatted.contains("Hint:"));
    }
}
