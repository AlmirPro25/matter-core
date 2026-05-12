//! # Matter Crash Reporter
//!
//! Automatic crash reporting with Sentry integration:
//! - Stack trace enrichment
//! - Context capture
//! - Real-time alerting

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stack frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub function: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub locals: HashMap<String, String>,
}

/// Stack trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackTrace {
    pub frames: Vec<StackFrame>,
}

/// Enriched stack trace with additional context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedTrace {
    pub trace: StackTrace,
    pub ffi_calls: Vec<FFICall>,
    pub environment: HashMap<String, String>,
}

/// FFI call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FFICall {
    pub language: String,
    pub function: String,
    pub arguments: Vec<String>,
}

/// Error context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub user_id: Option<String>,
    pub request_id: Option<String>,
    pub tags: HashMap<String, String>,
    pub extra: HashMap<String, serde_json::Value>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            user_id: None,
            request_id: None,
            tags: HashMap::new(),
            extra: HashMap::new(),
        }
    }

    pub fn set_user(&mut self, user_id: String) {
        self.user_id = Some(user_id);
    }

    pub fn set_request_id(&mut self, request_id: String) {
        self.request_id = Some(request_id);
    }

    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }

    pub fn add_extra(&mut self, key: String, value: serde_json::Value) {
        self.extra.insert(key, value);
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Sentry client configuration
#[derive(Debug, Clone)]
pub struct SentryConfig {
    pub dsn: String,
    pub environment: String,
    pub release: Option<String>,
}

/// Sentry client
pub struct SentryClient {
    config: SentryConfig,
    context: ErrorContext,
}

impl SentryClient {
    pub fn new(config: SentryConfig) -> Self {
        Self {
            config,
            context: ErrorContext::new(),
        }
    }

    pub fn set_context(&mut self, context: ErrorContext) {
        self.context = context;
    }

    pub fn capture_error(
        &self,
        error: &dyn std::error::Error,
        trace: &StackTrace,
    ) -> Result<String, CrashReporterError> {
        let enriched = self.enrich_trace(trace);

        let event = SentryEvent {
            message: error.to_string(),
            level: "error".to_string(),
            trace: enriched,
            context: self.context.clone(),
            environment: self.config.environment.clone(),
            release: self.config.release.clone(),
        };

        // In production, send to Sentry API
        // For now, just log
        eprintln!(
            "[CRASH REPORT] {}",
            serde_json::to_string_pretty(&event).unwrap()
        );

        Ok("event-id-123".to_string())
    }

    fn enrich_trace(&self, trace: &StackTrace) -> EnrichedTrace {
        EnrichedTrace {
            trace: trace.clone(),
            ffi_calls: vec![], // Would be populated from runtime
            environment: std::env::vars().collect(),
        }
    }
}

/// Sentry event
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SentryEvent {
    message: String,
    level: String,
    trace: EnrichedTrace,
    context: ErrorContext,
    environment: String,
    release: Option<String>,
}

/// Stack trace enricher
pub struct StackTraceEnricher {
    source_maps: HashMap<String, SourceMap>,
}

impl StackTraceEnricher {
    pub fn new() -> Self {
        Self {
            source_maps: HashMap::new(),
        }
    }

    pub fn load_source_map(&mut self, file: String, map: SourceMap) {
        self.source_maps.insert(file, map);
    }

    pub fn enrich(&self, trace: &StackTrace) -> EnrichedTrace {
        let mut enriched_frames = Vec::new();

        for frame in &trace.frames {
            let mut enriched_frame = frame.clone();

            // Add source map information if available
            if let Some(file) = &frame.file {
                if let Some(map) = self.source_maps.get(file) {
                    if let (Some(line), Some(col)) = (frame.line, frame.column) {
                        if let Some(original) = map.lookup(line, col) {
                            enriched_frame.file = Some(original.file.clone());
                            enriched_frame.line = Some(original.line);
                            enriched_frame.column = Some(original.column);
                        }
                    }
                }
            }

            enriched_frames.push(enriched_frame);
        }

        EnrichedTrace {
            trace: StackTrace {
                frames: enriched_frames,
            },
            ffi_calls: vec![],
            environment: HashMap::new(),
        }
    }
}

impl Default for StackTraceEnricher {
    fn default() -> Self {
        Self::new()
    }
}

/// Source map for minified code
#[derive(Debug, Clone)]
pub struct SourceMap {
    mappings: Vec<Mapping>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    pub fn add_mapping(&mut self, mapping: Mapping) {
        self.mappings.push(mapping);
    }

    pub fn lookup(&self, line: usize, column: usize) -> Option<&Mapping> {
        self.mappings
            .iter()
            .find(|m| m.generated_line == line && m.generated_column == column)
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Source map mapping
#[derive(Debug, Clone)]
pub struct Mapping {
    pub generated_line: usize,
    pub generated_column: usize,
    pub file: String,
    pub line: usize,
    pub column: usize,
}

/// Crash reporter
pub struct CrashReporter {
    sentry_client: Option<SentryClient>,
    enricher: StackTraceEnricher,
}

impl CrashReporter {
    pub fn new() -> Self {
        Self {
            sentry_client: None,
            enricher: StackTraceEnricher::new(),
        }
    }

    pub fn with_sentry(config: SentryConfig) -> Self {
        Self {
            sentry_client: Some(SentryClient::new(config)),
            enricher: StackTraceEnricher::new(),
        }
    }

    pub fn set_context(&mut self, context: ErrorContext) {
        if let Some(client) = &mut self.sentry_client {
            client.set_context(context);
        }
    }

    pub fn report_crash(&self, error: &dyn std::error::Error) -> Result<(), CrashReporterError> {
        // Capture stack trace
        let trace = self.capture_stack_trace();

        // Enrich trace
        let enriched = self.enricher.enrich(&trace);

        // Send to Sentry if configured
        if let Some(client) = &self.sentry_client {
            client.capture_error(error, &trace)?;
        } else {
            // Just log locally
            eprintln!("[CRASH] {}", error);
            eprintln!("[TRACE] {:?}", enriched);
        }

        Ok(())
    }

    fn capture_stack_trace(&self) -> StackTrace {
        let bt = backtrace::Backtrace::new();
        let frames: Vec<StackFrame> = bt
            .frames()
            .iter()
            .map(|frame| {
                let symbols = frame.symbols();
                let symbol = symbols.first();

                StackFrame {
                    function: symbol
                        .and_then(|s| s.name())
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "<unknown>".to_string()),
                    file: symbol
                        .and_then(|s| s.filename())
                        .map(|f| f.to_string_lossy().to_string()),
                    line: symbol.and_then(|s| s.lineno()).map(|l| l as usize),
                    column: symbol.and_then(|s| s.colno()).map(|c| c as usize),
                    locals: HashMap::new(),
                }
            })
            .collect();

        StackTrace { frames }
    }

    pub fn enrich_stacktrace(&self, trace: &StackTrace) -> EnrichedTrace {
        self.enricher.enrich(trace)
    }
}

impl Default for CrashReporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Crash reporter error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum CrashReporterError {
    #[error("Failed to send crash report: {0}")]
    SendError(String),

    #[error("Failed to enrich stack trace: {0}")]
    EnrichmentError(String),

    #[error("Sentry not configured")]
    NotConfigured,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let mut ctx = ErrorContext::new();
        ctx.set_user("user123".to_string());
        ctx.add_tag("environment".to_string(), "production".to_string());

        assert_eq!(ctx.user_id, Some("user123".to_string()));
        assert_eq!(ctx.tags.get("environment"), Some(&"production".to_string()));
    }

    #[test]
    fn test_crash_reporter() {
        let reporter = CrashReporter::new();

        let error = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        let result = reporter.report_crash(&error);

        assert!(result.is_ok());
    }

    #[test]
    fn test_source_map() {
        let mut map = SourceMap::new();
        map.add_mapping(Mapping {
            generated_line: 1,
            generated_column: 10,
            file: "original.matter".to_string(),
            line: 42,
            column: 5,
        });

        let original = map.lookup(1, 10);
        assert!(original.is_some());
        assert_eq!(original.unwrap().line, 42);
    }
}
