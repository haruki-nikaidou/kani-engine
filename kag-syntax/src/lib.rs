#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

//! # kag-syntax
//!
//! Lexer, parser, and AST for the KAG (Kirikiri Adventure Game) scenario-script
//! language.  This crate is intentionally free of runtime concerns so that
//! tooling (formatters, language servers, etc.) can depend on it without
//! pulling in the async interpreter machinery.
//!
//! ## Pipeline
//!
//! ```text
//! source .ks text
//!     └─ lexer (logos) ─────────────────── Vec<Spanned<Token>>
//!         └─ CST parser (Rowan) ────────── Parse<cst::Root>  (lossless)
//!             └─ lower::lower_root() ───── Script<'static>   (semantic AST)
//! ```
//!
//! ## Usage
//!
//! ### Semantic AST (fast path — suitable for the interpreter)
//!
//! ```no_run
//! use kag_syntax::parse_script;
//!
//! let (script, errors) = parse_script("@r\n", "test.ks");
//! assert!(errors.is_empty());
//! ```
//!
//! ### Lossless CST (for LSP / formatter)
//!
//! ```no_run
//! use kag_syntax::parse_cst;
//!
//! let parse = parse_cst("@r\n", "test.ks");
//! let root = parse.tree();
//! // Walk root.items() to inspect every node with full spans.
//! ```

pub mod ast;
pub mod cst;
pub mod error;
pub mod lexer;
pub mod lower;
pub mod parser;
pub mod syntax_kind;
pub mod tag_defs;

// ── Semantic AST re-exports ───────────────────────────────────────────────────

pub use ast::{LabelDef, MacroDef, Op, Param, ParamValue, Script, Tag, TextPart};
pub use error::{KagError, ParseDiagnostic, Severity};

// ── Parser entry points ───────────────────────────────────────────────────────

pub use parser::{Parse, parse_cst, parse_script};

// ── CST types ─────────────────────────────────────────────────────────────────

pub use cst::AstNode;
pub use syntax_kind::{KagLanguage, SyntaxKind, SyntaxNode, SyntaxToken};
