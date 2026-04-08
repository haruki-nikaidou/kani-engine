//! Tag-name and parameter parsers.
//!
//! These functions are called by the line-level parser in [`super::line`] and
//! emit Rowan nodes directly through the shared [`Parser`] struct.

use crate::syntax_kind::SyntaxKind;

use super::Parser;

// ─── AT-tag body ─────────────────────────────────────────────────────────────

/// Parse the tag name and parameter list for an `@tag` line.
///
/// Precondition: `p.current()` is the first `IDENT` token (tag name).
/// The surrounding `AT_TAG` node is managed by the caller in `line.rs`.
/// Does **not** consume the trailing `NEWLINE`.
pub(crate) fn parse_at_tag_body(p: &mut Parser<'_>) {
    let tag_name = p.current_slice().to_owned();

    p.start_node(SyntaxKind::TAG_NAME);
    p.bump(); // IDENT
    p.finish_node();

    parse_param_list(p, /*inline=*/ false);

    dispatch_special_tag(p, &tag_name);
}

// ─── Inline-tag body ─────────────────────────────────────────────────────────

/// Parse the tag name and parameter list for an inline `[tag]`.
///
/// Precondition: `p.current()` is the first `IDENT` token (tag name).
/// The surrounding `INLINE_TAG` node and `R_BRACKET` are managed by the caller.
pub(crate) fn parse_inline_tag_body(p: &mut Parser<'_>) {
    let tag_name = p.current_slice().to_owned();

    p.start_node(SyntaxKind::TAG_NAME);
    p.bump(); // IDENT
    p.finish_node();

    parse_param_list(p, /*inline=*/ true);

    dispatch_special_tag(p, &tag_name);
}

// ─── Special tag dispatch ─────────────────────────────────────────────────────

/// After parsing a tag's name and params, set parser flags for special tags
/// that require post-processing (iscript body, macro body).
///
/// The actual body parsing is done by `line::handle_pending_special` after
/// the enclosing tag node is closed.
fn dispatch_special_tag(p: &mut Parser<'_>, tag_name: &str) {
    match tag_name {
        "iscript" => {
            p.pending_iscript = true;
        }
        "macro" => {
            p.pending_macro = true;
        }
        _ => {}
    }
}

// ─── Parameter list ───────────────────────────────────────────────────────────

/// Parse zero or more parameters into a `PARAM_LIST` node.
///
/// Stops at `NEWLINE` (always) or `R_BRACKET` (when `inline = true`).
fn parse_param_list(p: &mut Parser<'_>, inline: bool) {
    p.start_node(SyntaxKind::PARAM_LIST);

    loop {
        p.skip_ws();
        if p.at_end() || p.at(SyntaxKind::NEWLINE) {
            break;
        }
        if inline && p.at(SyntaxKind::R_BRACKET) {
            break;
        }
        // A line comment after parameters terminates the list.
        if p.at(SyntaxKind::LINE_COMMENT) {
            p.bump();
            break;
        }
        if !parse_param(p, inline) {
            break;
        }
    }

    p.finish_node();
}

// ─── Single parameter ─────────────────────────────────────────────────────────

/// Parse a single `key=value` or positional-`value` parameter.
/// Returns `false` when no parameter could be parsed.
fn parse_param(p: &mut Parser<'_>, inline: bool) -> bool {
    // Named parameter: `ident = value`
    if p.at(SyntaxKind::IDENT) && peek_eq_follows(p) {
        p.start_node(SyntaxKind::PARAM);

        p.start_node(SyntaxKind::PARAM_KEY);
        p.bump(); // IDENT
        p.finish_node();

        p.skip_ws();
        p.bump(); // EQ
        p.skip_ws();

        parse_param_value(p, inline);
        p.finish_node(); // PARAM
        return true;
    }

    // Positional parameter value.
    let start = p.pos;
    p.start_node(SyntaxKind::PARAM);
    parse_param_value(p, inline);
    if p.pos == start {
        // Nothing consumed — stop the loop.
        p.finish_node();
        return false;
    }
    p.finish_node();
    true
}

/// `true` if an `EQ` token follows the current `IDENT` (possibly separated by
/// whitespace).
fn peek_eq_follows(p: &Parser<'_>) -> bool {
    let mut i = p.pos + 1;
    while i < p.tokens.len() && matches!(p.tokens[i].token, crate::lexer::Token::Whitespace) {
        i += 1;
    }
    i < p.tokens.len() && matches!(p.tokens[i].token, crate::lexer::Token::Eq)
}

// ─── Parameter value ──────────────────────────────────────────────────────────

fn parse_param_value(p: &mut Parser<'_>, inline: bool) {
    match p.current() {
        SyntaxKind::DOUBLE_QUOTED | SyntaxKind::SINGLE_QUOTED => {
            p.start_node(SyntaxKind::PARAM_VALUE_LITERAL);
            p.bump();
            p.finish_node();
        }
        SyntaxKind::AMP => {
            parse_entity_value(p, inline);
        }
        SyntaxKind::PERCENT => {
            parse_macro_param_value(p, inline);
        }
        SyntaxKind::STAR => {
            if p.tokens
                .get(p.pos + 1)
                .is_some_and(|t| matches!(t.token, crate::lexer::Token::Ident(_)))
            {
                // `*ident` — label reference literal.
                p.start_node(SyntaxKind::PARAM_VALUE_LITERAL);
                p.bump(); // STAR
                p.bump(); // IDENT
                p.finish_node();
            } else {
                // Bare `*` — macro splat.
                p.start_node(SyntaxKind::PARAM_VALUE_SPLAT);
                p.bump();
                p.finish_node();
            }
        }
        k if is_bare_value_token(k, inline) => {
            p.start_node(SyntaxKind::PARAM_VALUE_LITERAL);
            while !p.at_end() && is_bare_value_token(p.current(), inline) {
                p.bump();
            }
            p.finish_node();
        }
        _ => {
            // Unrecognised token — let the caller decide.
        }
    }
}

/// `true` when `kind` can appear inside an unquoted parameter value.
fn is_bare_value_token(kind: SyntaxKind, inline: bool) -> bool {
    match kind {
        SyntaxKind::IDENT
        | SyntaxKind::NUMBER
        | SyntaxKind::TEXT
        | SyntaxKind::SLASH
        | SyntaxKind::LT
        | SyntaxKind::GT
        | SyntaxKind::COLON => true,
        // `]` terminates an inline tag value but can appear in line-level values.
        SyntaxKind::R_BRACKET => !inline,
        _ => false,
    }
}

fn parse_entity_value(p: &mut Parser<'_>, inline: bool) {
    p.start_node(SyntaxKind::PARAM_VALUE_ENTITY);
    p.bump(); // AMP
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) && !p.at(SyntaxKind::WHITESPACE) {
        if inline && p.at(SyntaxKind::R_BRACKET) {
            break;
        }
        p.bump();
    }
    p.finish_node();
}

fn parse_macro_param_value(p: &mut Parser<'_>, inline: bool) {
    p.start_node(SyntaxKind::PARAM_VALUE_MACRO);
    p.bump(); // PERCENT
    if p.at(SyntaxKind::IDENT) {
        p.bump(); // key
    } else {
        p.push_error("expected identifier after `%` in macro parameter reference");
    }
    if p.at(SyntaxKind::PIPE) {
        p.bump(); // PIPE
        // Default value — collect until boundary.
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) && !p.at(SyntaxKind::WHITESPACE) {
            if inline && p.at(SyntaxKind::R_BRACKET) {
                break;
            }
            p.bump();
        }
    }
    p.finish_node();
}
