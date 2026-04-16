use thiserror::Error;

/// All runtime errors that can occur during KAG script interpretation.
#[derive(Debug, Error)]
pub enum InterpreterError {
    /// Wraps a syntax-level error from parsing.
    #[error(transparent)]
    Syntax(#[from] kag_syntax::SyntaxError),

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
