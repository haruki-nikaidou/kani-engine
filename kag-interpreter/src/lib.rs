#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

//! # kag-interpreter
//!
//! A Rust implementation of the KAG (Kirikiri Adventure Game) scenario-script
//! interpreter, designed to be embedded in a visual-novel game engine.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use kag_interpreter::{KagInterpreter, KagEvent, HostEvent};
//!
//! #[tokio::main]
//! async fn main() {
//!     let source = r#"
//! *start
//! Hello, world![l]
//! @jump target=*start
//! "#;
//!     let (mut interp, _task, _diags) =
//!         KagInterpreter::spawn_from_source(source, "demo.ks").unwrap();
//!
//!     loop {
//!         match interp.recv().await {
//!             Some(KagEvent::DisplayText { text, speaker, .. }) => {
//!                 if let Some(spk) = speaker { print!("{spk}: "); }
//!                 println!("{text}");
//!             }
//!             Some(KagEvent::WaitForClick { .. }) => {
//!                 // Simulate an immediate click
//!                 interp.send(HostEvent::Clicked).await.unwrap();
//!             }
//!             Some(KagEvent::End) | None => break,
//!             _ => {}
//!         }
//!     }
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! source .ks text
//!     └─ lexer (logos) ─────────────────── Vec<Spanned<Token>>
//!         └─ CST parser (Rowan) ────────── Parse<cst::Root>  (lossless)
//!             └─ lower::lower_root() ───── Script<'static>   (semantic AST)
//!                 └─ KagInterpreter (tokio task)
//!                         │ KagEvent channel (to host)
//!                         │ HostEvent channel (from host)
//!                     RuntimeContext
//!                         └─ ScriptEngine (rhai)
//! ```

// ─── Syntax crate re-exports (lexer, parser, AST) ────────────────────────────

pub use kag_syntax::ast;
pub use kag_syntax::lexer;
pub use kag_syntax::parser;

// ─── Interpreter-local modules ────────────────────────────────────────────────

pub mod error;
pub mod events;
pub mod runtime;
pub mod snapshot;

// ─── Primary public re-exports ────────────────────────────────────────────────

/// The main interpreter actor handle.
pub use runtime::KagInterpreter;

/// All event types used across the public API.
pub use events::{ChoiceOption, HostEvent, KagEvent, ResolvedTag, VarScope, VariableSnapshot};

/// Typed tag name enum — identifies every built-in KAG tag by variant.
pub use kag_syntax::tag_defs::TagName;

/// Jump-target registered by `[click]` or `[wheel]` before `[s]`.
pub use runtime::context::JumpTarget;
/// Timed jump registered by `[timeout]` before `[s]`.
pub use runtime::context::TimeoutHandler;

/// The parsed scenario representation.
pub use ast::{LabelDef, MacroDef, Op, Param, ParamValue, Script, Tag, TextPart};

/// Runtime error type covering all interpreter failures.
pub use error::InterpreterError;

/// Serialisable interpreter snapshot for save / load support.
pub use snapshot::{CallFrameSnap, IfFrameSnap, InterpreterSnapshot, MacroFrameSnap};

/// Parse a `.ks` source string into a `Script` together with any diagnostics.
/// Returns `(Script<'static>, Vec<SyntaxWarning>)`.
pub use parser::parse_script;

/// Non-fatal diagnostic emitted during parsing.
pub use kag_syntax::error::SyntaxDiagnostic;

/// Severity level for a [`SyntaxWarning`].
pub use kag_syntax::error::Severity;
