//! Tag-name and parameter parsers using `winnow` combinators.
//!
//! These parsers operate on `&[Spanned<'src>]` sub-slices that have already
//! been extracted by the line-level parser in `super::line`.

use std::borrow::Cow;

use miette::SourceSpan;
use winnow::{
    combinator::{alt, opt, repeat},
    error::ContextError,
    prelude::*,
    token::any,
};

// In winnow 1.0, internal helper parsers return `winnow::error::Result<O, E>`
// (= `std::result::Result<O, E>` with ContextError), NOT ModalResult.
// ModalResult (= Result<O, ErrMode<E>>) is only for top-level entry parsers.
type WRes<O> = Result<O, ContextError>;

use crate::ast::{Param, ParamValue, Tag};
use crate::lexer::{Spanned, Token};

// ─── Stream type alias ────────────────────────────────────────────────────────

/// Winnow stream: a mutable reference to a slice of spanned tokens.
pub type TokInput<'src, 'toks> = &'toks [Spanned<'src>];

// ─── Low-level token matchers ─────────────────────────────────────────────────

/// Consume and return the next token that satisfies `pred`.
fn satisfy<'src, 'toks, F>(
    pred: F,
) -> impl Parser<TokInput<'src, 'toks>, Spanned<'src>, ContextError>
where
    F: Fn(&Spanned<'src>) -> bool,
    'src: 'toks,
{
    any.verify(move |t: &Spanned<'src>| pred(t))
}

/// Consume the next token and return it if it is an `Ident`.
fn next_ident<'src, 'toks>(
    input: &mut TokInput<'src, 'toks>,
) -> WRes<Spanned<'src>>
where
    'src: 'toks,
{
    satisfy(|t| matches!(t.token, Token::Ident(_))).parse_next(input)
}

/// Consume an `Eq` (`=`) token.
fn expect_eq<'src, 'toks>(
    input: &mut TokInput<'src, 'toks>,
) -> WRes<()>
where
    'src: 'toks,
{
    satisfy(|t| matches!(t.token, Token::Eq))
        .map(|_| ())
        .parse_next(input)
}

// ─── Parameter value parsers ─────────────────────────────────────────────────

/// Parse a quoted string, stripping the surrounding quote characters.
fn parse_quoted<'src, 'toks>(
    input: &mut TokInput<'src, 'toks>,
) -> WRes<Cow<'src, str>>
where
    'src: 'toks,
{
    satisfy(|t| {
        matches!(t.token, Token::DoubleQuoted(_) | Token::SingleQuoted(_))
    })
    .map(|s: Spanned<'src>| {
        // Strip surrounding quote characters
        let inner = &s.slice[1..s.slice.len() - 1];
        Cow::Borrowed(inner)
    })
    .parse_next(input)
}

/// Parse an entity expression: `&expr_tokens_up_to_whitespace_or_]`.
///
/// Collects every token that is not a `]`, `Newline`, or whitespace-level
/// separator and concatenates their slices as the expression string.
fn parse_entity<'src, 'toks>(
    input: &mut TokInput<'src, 'toks>,
) -> WRes<Cow<'src, str>>
where
    'src: 'toks,
{
    // Consume `&`
    satisfy(|t| matches!(t.token, Token::Amp)).parse_next(input)?;

    // Collect tokens until a natural parameter boundary (whitespace stops collection)
    let expr_tokens: Vec<Spanned<'src>> = repeat(
        1..,
        satisfy(|t| {
            !matches!(
                t.token,
                Token::RBracket | Token::Newline | Token::Whitespace
            )
        }),
    )
    .parse_next(input)?;

    let expr: String = expr_tokens.iter().map(|s| s.slice).collect();
    Ok(Cow::Owned(expr))
}

/// Parse a macro parameter reference: `%key` or `%key|default`.
fn parse_macro_param<'src, 'toks>(
    input: &mut TokInput<'src, 'toks>,
) -> WRes<ParamValue<'src>>
where
    'src: 'toks,
{
    // Consume `%`
    satisfy(|t| matches!(t.token, Token::Percent)).parse_next(input)?;

    let key_tok = next_ident(input)?;
    let key: Cow<'src, str> = Cow::Borrowed(key_tok.slice);

    // Optional `|default`
    let default = opt(|inp: &mut TokInput<'src, 'toks>| {
        satisfy(|t| matches!(t.token, Token::Pipe)).parse_next(inp)?;
        // default value: collect until boundary
        let parts: Vec<Spanned<'src>> = repeat(
            0..,
            satisfy(|t| {
                !matches!(t.token, Token::RBracket | Token::Newline)
                    && !is_param_boundary(t)
            }),
        )
        .parse_next(inp)?;
        let s: String = parts.iter().map(|s| s.slice).collect();
        Ok(Cow::Owned(s))
    })
    .parse_next(input)?;

    Ok(ParamValue::MacroParam { key, default })
}

/// Parse the bare `*` splat used in macro definitions.
fn parse_splat<'src, 'toks>(
    input: &mut TokInput<'src, 'toks>,
) -> WRes<ParamValue<'src>>
where
    'src: 'toks,
{
    satisfy(|t| matches!(t.token, Token::Star))
        .map(|_| ParamValue::MacroSplat)
        .parse_next(input)
}

/// True if this token acts as a whitespace / parameter separator boundary.
fn is_param_boundary(t: &Spanned<'_>) -> bool {
    matches!(t.token, Token::Eq | Token::Whitespace)
}

/// Skip any leading `Whitespace` tokens.
fn skip_ws<'src, 'toks>(input: &mut TokInput<'src, 'toks>) {
    while !input.is_empty() && matches!(input[0].token, Token::Whitespace) {
        *input = &input[1..];
    }
}

/// A bare (unquoted) parameter value.
///
/// Handles two cases:
/// 1. `*ident` — a label reference like `target=*start` (two tokens)
/// 2. Any other single non-boundary token
fn parse_bare_value<'src, 'toks>(
    input: &mut TokInput<'src, 'toks>,
) -> WRes<Cow<'src, str>>
where
    'src: 'toks,
{
    // Special case: `*ident` for label targets (e.g. `target=*start`)
    // A standalone `*` is NOT a bare value — it is a MacroSplat handled by
    // `parse_splat`.  Return an error to let that arm fire.
    if !input.is_empty() && matches!(input[0].token, Token::Star) {
        let star = &input[0];
        if input.len() >= 2 && matches!(input[1].token, Token::Ident(_)) {
            let ident = &input[1];
            let val = Cow::Owned(format!("{}{}", star.slice, ident.slice));
            *input = &input[2..];
            return Ok(val);
        }
        // Standalone `*` — decline so parse_splat can handle it
        return Err(ContextError::new());
    }

    // Single non-boundary token
    satisfy(|t| {
        !matches!(
            t.token,
            Token::RBracket
                | Token::Newline
                | Token::Eq
                | Token::Amp
                | Token::Percent
                | Token::Whitespace
        )
    })
    .map(|s: Spanned<'src>| Cow::Borrowed(s.slice))
    .parse_next(input)
}

/// Parse a single `ParamValue`: quoted, entity, macro-param, bare (incl. `*ident`), or splat (`*` alone).
///
/// Order matters: `parse_bare_value` must come before `parse_splat` because
/// `*ident` is a label reference (bare value) while standalone `*` is a macro splat.
pub(crate) fn parse_param_value<'src, 'toks>(
    input: &mut TokInput<'src, 'toks>,
) -> WRes<ParamValue<'src>>
where
    'src: 'toks,
{
    alt((
        parse_macro_param,
        |inp: &mut TokInput<'src, 'toks>| {
            let s = parse_entity(inp)?;
            Ok(ParamValue::Entity(s))
        },
        |inp: &mut TokInput<'src, 'toks>| {
            let s = parse_quoted(inp)?;
            Ok(ParamValue::Literal(s))
        },
        // parse_bare_value handles `*ident` (label refs) — must precede parse_splat
        |inp: &mut TokInput<'src, 'toks>| {
            let s = parse_bare_value(inp)?;
            Ok(ParamValue::Literal(s))
        },
        // Standalone `*` (macro splat) — only reached when bare_value failed
        parse_splat,
    ))
    .parse_next(input)
}

// ─── Full parameter list parser ───────────────────────────────────────────────

/// Parse zero or more `key=value` (or bare `value`) parameters from a tag
/// parameter token slice.
///
/// Each iteration tries to parse `ident '=' value` (named), then falls back
/// to a bare `value` (positional).
pub fn parse_params<'src, 'toks>(
    input: &mut TokInput<'src, 'toks>,
) -> WRes<Vec<Param<'src>>>
where
    'src: 'toks,
{
    let mut params = Vec::new();

    loop {
        // Skip whitespace between parameters
        skip_ws(input);

        if input.is_empty() {
            break;
        }

        // Stop at line / tag boundaries
        if matches!(input[0].token, Token::Newline | Token::RBracket) {
            break;
        }

        // Try named parameter: ident = value
        if let Some(named) = opt(|inp: &mut TokInput<'src, 'toks>| {
            let key_tok = next_ident(inp)?;
            expect_eq(inp)?;
            let value = parse_param_value(inp)?;
            Ok(Param {
                key: Some(Cow::Borrowed(key_tok.slice)),
                value,
            })
        })
        .parse_next(input)?
        {
            params.push(named);
            continue;
        }

        // Try bare/splat value (positional)
        if let Some(pos) = opt(|inp: &mut TokInput<'src, 'toks>| {
            let value = parse_param_value(inp)?;
            Ok(Param { key: None, value })
        })
        .parse_next(input)?
        {
            params.push(pos);
            continue;
        }

        // Cannot parse further (e.g. hit a LineComment token)
        break;
    }

    Ok(params)
}

// ─── Full tag parser (name + params) ─────────────────────────────────────────

/// Parse a complete tag from a token sub-slice:
/// `name [key=value …]`
///
/// `span` is the source span of the entire tag (e.g. from `[` to `]`).
pub fn parse_tag_from_tokens<'src, 'toks>(
    tokens: &'toks [Spanned<'src>],
    span: SourceSpan,
) -> Result<Tag<'src>, ContextError>
where
    'src: 'toks,
{
    let mut input: TokInput<'src, 'toks> = tokens;

    // Tag name: first token must be an ident (or `*` prefix for labels in
    // param position — not handled here; handled by line parser)
    let name_tok = next_ident(&mut input).map_err(|_| ContextError::new())?;

    let params = parse_params(&mut input)?;

    Ok(Tag {
        name: Cow::Borrowed(name_tok.slice),
        params,
        span,
    })
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn tokenize_tag_body(src: &str) -> Vec<Spanned<'_>> {
        let (toks, _) = tokenize(src);
        toks
    }

    #[test]
    fn test_parse_simple_params() {
        let toks = tokenize_tag_body("storage=main target=*start");
        let mut input: TokInput<'_, '_> = &toks;
        let params = parse_params(&mut input).unwrap();
        assert_eq!(params.len(), 2, "params: {:?}", params);
        assert_eq!(params[0].key.as_deref(), Some("storage"));
        assert!(
            matches!(&params[0].value, ParamValue::Literal(v) if v == "main"),
            "got {:?}",
            params[0].value
        );
        assert_eq!(params[1].key.as_deref(), Some("target"));
        assert!(
            matches!(&params[1].value, ParamValue::Literal(v) if v.contains("start")),
            "got {:?}",
            params[1].value
        );
    }

    #[test]
    fn test_parse_quoted_param() {
        let toks = tokenize_tag_body(r#"storage="forest bg.png""#);
        let mut input: TokInput<'_, '_> = &toks;
        let params = parse_params(&mut input).unwrap();
        assert_eq!(params.len(), 1);
        assert!(
            matches!(&params[0].value, ParamValue::Literal(v) if v == "forest bg.png"),
            "got {:?}",
            params[0].value
        );
    }

    #[test]
    fn test_parse_entity_param() {
        let toks = tokenize_tag_body("exp=&f.counter");
        let mut input: TokInput<'_, '_> = &toks;
        let params = parse_params(&mut input).unwrap();
        assert_eq!(params.len(), 1);
        assert!(
            matches!(&params[0].value, ParamValue::Entity(e) if e.contains("f.counter")),
            "got {:?}",
            params[0].value
        );
    }

    #[test]
    fn test_parse_macro_param_with_default() {
        // Single-word default (spaces require quoting in KAG)
        let toks = tokenize_tag_body("val=%message|hello");
        let mut input: TokInput<'_, '_> = &toks;
        let params = parse_params(&mut input).unwrap();
        assert_eq!(params.len(), 1, "params: {:?}", params);
        match &params[0].value {
            ParamValue::MacroParam { key, default } => {
                assert_eq!(key.as_ref(), "message");
                assert!(default.is_some());
                assert_eq!(default.as_deref(), Some("hello"));
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn test_parse_splat_param() {
        let toks = tokenize_tag_body("*");
        let mut input: TokInput<'_, '_> = &toks;
        let params = parse_params(&mut input).unwrap();
        assert_eq!(params.len(), 1);
        assert!(matches!(params[0].value, ParamValue::MacroSplat));
    }

    #[test]
    fn test_parse_full_tag() {
        let toks = tokenize_tag_body("jump storage=main target=*start");
        let span: SourceSpan = (0usize, toks.iter().map(|t| t.span.len()).sum()).into();
        let tag = parse_tag_from_tokens(&toks, span).unwrap();
        assert_eq!(tag.name.as_ref(), "jump");
        assert_eq!(tag.params.len(), 2, "params: {:?}", tag.params);
        assert_eq!(tag.param_str("storage"), Some("main"));
    }

    #[test]
    fn test_empty_params() {
        let toks = tokenize_tag_body("r");
        let span: SourceSpan = (0usize, 1usize).into();
        let tag = parse_tag_from_tokens(&toks, span).unwrap();
        assert_eq!(tag.name.as_ref(), "r");
        assert!(tag.params.is_empty());
    }
}
