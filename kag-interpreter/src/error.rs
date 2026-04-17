use std::fmt;

use thiserror::Error;

// ─── Diagnostic severity ──────────────────────────────────────────────────────

/// Severity level for an [`InterpreterDiagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    /// Non-fatal issue — the interpreter continues execution.
    Warning,
    /// Fatal issue — the interpreter will shut down after emitting this.
    Error,
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

// ─── Diagnostic category ─────────────────────────────────────────────────────

/// Classifies the origin of an [`InterpreterDiagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCategory {
    /// One or more problems detected during `.ks` file parsing.
    Syntax,
    /// Rhai script evaluation failure (`[eval]`, `[if exp=…]`, `[iscript]`, …).
    ScriptEval,
    /// General runtime problem (snapshot failure, unsupported operation, …).
    Runtime,
    /// A `[jump]`/`[call]` target label could not be resolved.
    LabelNotFound,
    /// `[return]` without a matching `[call]`, or similar stack mismatch.
    CallStack,
    /// Macro expansion error (missing definition, parameter issues).
    Macro,
    /// Save/load serialisation failure.
    Serialization,
}

impl fmt::Display for DiagnosticCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax => write!(f, "syntax"),
            Self::ScriptEval => write!(f, "script-eval"),
            Self::Runtime => write!(f, "runtime"),
            Self::LabelNotFound => write!(f, "label-not-found"),
            Self::CallStack => write!(f, "call-stack"),
            Self::Macro => write!(f, "macro"),
            Self::Serialization => write!(f, "serialization"),
        }
    }
}

// ─── InterpreterDiagnostic ────────────────────────────────────────────────────

/// A structured diagnostic emitted during KAG script interpretation.
///
/// Replaces the former bare-string `KagEvent::Warning` / `KagEvent::Error`
/// variants.  All diagnostics carry a severity, a category that classifies the
/// origin of the problem, a human-readable message, and optional location
/// context (scenario file name and op-list index).
#[derive(Debug, Clone)]
pub struct InterpreterDiagnostic {
    /// Whether this diagnostic is fatal (`Error`) or informational (`Warning`).
    pub severity: DiagnosticSeverity,
    /// Classification of the problem.
    pub category: DiagnosticCategory,
    /// Human-readable description.
    pub message: String,
    /// The scenario file that was active when the diagnostic was produced.
    pub storage: Option<String>,
    /// Op-list index at which the diagnostic was produced.
    pub pc: Option<usize>,
}

impl InterpreterDiagnostic {
    /// Create a **warning**-level diagnostic (non-fatal).
    pub fn warning(category: DiagnosticCategory, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            category,
            message: message.into(),
            storage: None,
            pc: None,
        }
    }

    /// Create an **error**-level diagnostic (fatal).
    pub fn error(category: DiagnosticCategory, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            category,
            message: message.into(),
            storage: None,
            pc: None,
        }
    }

    /// Attach location context (builder pattern).
    pub fn at(mut self, storage: impl Into<String>, pc: usize) -> Self {
        self.storage = Some(storage.into());
        self.pc = Some(pc);
        self
    }

    /// Attach only the storage name (when pc is not meaningful).
    pub fn in_file(mut self, storage: impl Into<String>) -> Self {
        self.storage = Some(storage.into());
        self
    }

    /// Returns `true` when this diagnostic is fatal.
    pub fn is_fatal(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }
}

impl fmt::Display for InterpreterDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.category, self.message)?;
        if let Some(ref s) = self.storage {
            write!(f, " (in '{s}'")?;
            if let Some(pc) = self.pc {
                write!(f, " at op {pc}")?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

// ─── InterpreterError (public API errors only) ────────────────────────────────

/// Errors returned from the public [`KagInterpreter`] handle methods.
///
/// These represent failures in the **host ↔ interpreter communication layer**,
/// not problems inside the scenario script.  Script-level problems are reported
/// as [`InterpreterDiagnostic`] events on the channel.
#[derive(Debug, Error)]
pub enum InterpreterError {
    /// The async channel between host and interpreter has been closed.
    #[error("channel closed unexpectedly")]
    ChannelClosed,

    /// Save/load serialization failure (used internally by snapshot methods,
    /// surfaced to the host only when snapshot restore fails at spawn time).
    #[error("serialization error: {0}")]
    SerializationError(String),
}
