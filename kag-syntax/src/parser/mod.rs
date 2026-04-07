pub mod line;
pub mod tag;

use crate::ast::Script;
use crate::error::KagError;
use crate::lexer::{Spanned, tokenize};
use miette::NamedSource;

/// Parse a KAG `.ks` source string into a `Script<'src>`.
///
/// Internally:
/// 1. `logos` tokenises the source into a `Vec<Spanned<Token>>`.
/// 2. `winnow` combinators (in the `line` and `tag` sub-modules) parse the
///    flat token stream into the `Script` AST.
///
/// Any lex or parse error is returned as a `KagError` with full source
/// attribution via `miette`.
pub fn parse_script<'src>(
    source: &'src str,
    source_name: &str,
) -> Result<Script<'src>, KagError> {
    let (tokens, lex_errors) = tokenize(source);

    if let Some(first) = lex_errors.first() {
        return Err(KagError::LexError {
            offset: first.start,
            src: NamedSource::new(source_name, source.to_owned()),
            span: (first.start, first.len()).into(),
        });
    }

    let mut ctx = line::ParseCtx::new(source, source_name);
    ctx.parse_all(&tokens)?;
    Ok(ctx.into_script())
}

// ─── Stream helpers shared by sub-modules ────────────────────────────────────

/// The winnow input type: a mutable reference to a slice of spanned tokens.
/// `'src` borrows from the source string; `'toks` borrows from the token Vec.
pub type Input<'src, 'toks> = &'toks [Spanned<'src>];

