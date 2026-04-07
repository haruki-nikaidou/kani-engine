use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

/// A rich error type for KAG parse and runtime failures.
///
/// Implements `miette::Diagnostic` so callers can render human-readable
/// error reports with source-code highlighting.
#[derive(Debug, Error, Diagnostic)]
pub enum KagError {
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

    #[error("undefined tag: [{name}]")]
    #[diagnostic(code(kag::undefined_tag), help("Check spelling or define a macro named '{name}'"))]
    UndefinedTag { name: String },

    #[error("script evaluation error: {0}")]
    #[diagnostic(code(kag::script_error))]
    ScriptError(String),

    #[error("runtime error: {0}")]
    #[diagnostic(code(kag::runtime_error))]
    RuntimeError(String),

    #[error("label not found: '{label}' in '{storage}'")]
    #[diagnostic(code(kag::label_not_found))]
    LabelNotFound { label: String, storage: String },

    #[error("call stack underflow: [return] without matching [call]")]
    #[diagnostic(code(kag::stack_underflow))]
    CallStackUnderflow,

    #[error("macro parameter error: {0}")]
    #[diagnostic(code(kag::macro_error))]
    MacroError(String),

    #[error("channel closed unexpectedly")]
    #[diagnostic(code(kag::channel_closed))]
    ChannelClosed,
}

impl KagError {
    pub fn parse(message: impl Into<String>, src: NamedSource<String>, span: SourceSpan) -> Self {
        Self::ParseError {
            message: message.into(),
            src,
            span,
        }
    }
}
