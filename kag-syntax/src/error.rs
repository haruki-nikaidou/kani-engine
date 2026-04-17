use miette::SourceSpan;

// ─── Non-fatal parse diagnostic ───────────────────────────────────────────────

/// Severity level for a [`SyntaxDiagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The parser recovered and continued, but the result may be inaccurate.
    Error,
    /// A suspicious but valid construct (e.g. duplicate label).
    Warning,
}

/// A diagnostic produced during parsing or lowering.
///
/// Diagnostics are collected into a `Vec` and returned alongside the (possibly
/// partial) tree.  Consumers can inspect them to surface warnings or errors to
/// the user without preventing further processing.
#[derive(Debug, Clone)]
pub struct SyntaxDiagnostic {
    pub message: String,
    pub span: SourceSpan,
    pub severity: Severity,
}

impl SyntaxDiagnostic {
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
