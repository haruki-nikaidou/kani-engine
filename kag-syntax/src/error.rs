use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

// ─── Non-fatal parse diagnostic ───────────────────────────────────────────────

/// Severity level for a [`SyntaxWarning`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The parser recovered and continued, but the result may be inaccurate.
    Error,
    /// A suspicious but valid construct (e.g. duplicate label).
    Warning,
}

/// A non-fatal diagnostic produced during parsing.
///
/// Unlike [`SyntaxError`], which aborts the current parse, diagnostics are
/// collected into a `Vec` and returned alongside the (possibly partial) tree.
/// Consumers can inspect them to surface warnings or errors to the user
/// without preventing further processing.
#[derive(Debug, Clone)]
pub struct SyntaxWarning {
    pub message: String,
    pub span: SourceSpan,
    pub severity: Severity,
}

impl SyntaxWarning {
    pub fn error(message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            message: message.into(),
            span,
            severity: Severity::Error,
        }
    }

    pub fn warning(message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            message: message.into(),
            span,
            severity: Severity::Warning,
        }
    }
}

// ─── Fatal SyntaxError ────────────────────────────────────────────────────────

/// A fatal error produced during KAG syntax analysis.
///
/// Implements `miette::Diagnostic` so callers can render human-readable
/// error reports with source-code highlighting.
#[derive(Debug, Error, Diagnostic)]
pub enum SyntaxError {
    #[error("lexer error at byte {offset}")]
    #[diagnostic(code(kag::lex_error), help("Check for unrecognised characters"))]
    LexError {
        offset: usize,
        #[source_code]
        src: NamedSource<String>,
        #[label("unexpected character")]
        span: SourceSpan,
    },

    #[error("parse error: {message}")]
    #[diagnostic(code(kag::parse_error))]
    ParseError {
        message: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("here")]
        span: SourceSpan,
    },

    /// A known tag failed validation (missing required attribute, wrong type, etc.).
    #[error("invalid tag [{tag_name}]: {message}")]
    #[diagnostic(code(kag::invalid_tag))]
    InvalidTag {
        tag_name: String,
        message: String,
        #[label("here")]
        span: SourceSpan,
    },
}

impl SyntaxError {
    pub fn parse(message: impl Into<String>, src: NamedSource<String>, span: SourceSpan) -> Self {
        Self::ParseError {
            message: message.into(),
            src,
            span,
        }
    }
}
