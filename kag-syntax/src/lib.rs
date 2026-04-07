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
//!         └─ parser (winnow) ───────────── Script<'src>
//!             └─ Script::into_owned() ──── Script<'static>
//! ```

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;

pub use ast::{LabelDef, MacroDef, Op, Param, ParamValue, Script, Tag, TextPart};
pub use error::KagError;
pub use parser::parse_script;
