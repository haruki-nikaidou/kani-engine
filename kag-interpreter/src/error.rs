use thiserror::Error;

/// All runtime errors that can occur during KAG script interpretation.
#[derive(Debug, Error)]
pub enum InterpreterError {
    /// One or more syntax diagnostics from parsing a `.ks` file.
    #[error("syntax errors in '{source_name}': {diagnostics:?}")]
    Syntax {
        source_name: String,
        diagnostics: Vec<kag_syntax::SyntaxDiagnostic>,
    },

    /// Rhai script evaluation failed.
    #[error("script evaluation error: {0}")]
    ScriptError(String),

    /// Generic runtime failure.
    #[error("runtime error: {0}")]
    RuntimeError(String),

    /// `[jump]`/`[call]` target not found.
    #[error("label not found: '{label}' in '{storage}'")]
    LabelNotFound { label: String, storage: String },

    /// `[return]` without matching `[call]`.
    #[error("call stack underflow: [return] without matching [call]")]
    CallStackUnderflow,

    /// Macro expansion error.
    #[error("macro parameter error: {0}")]
    MacroError(String),

    /// Tokio channel closed.
    #[error("channel closed unexpectedly")]
    ChannelClosed,

    /// Save/load serialization failure.
    #[error("serialization error: {0}")]
    SerializationError(String),
}
