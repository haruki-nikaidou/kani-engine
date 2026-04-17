//! Line-level KAG parser.
//!
//! Processes the flat token stream one logical line at a time, classifying
//! each line and delegating tag / parameter parsing to [`super::tag`].
//!
//! Every syntactic construct is wrapped in a Rowan node so the resulting CST
//! is fully lossless — whitespace, comments, and even error regions are
//! preserved as tree nodes.

use crate::lexer::Token;
use crate::syntax_kind::SyntaxKind;

use super::Parser;
use super::tag::{parse_at_tag_body, parse_inline_tag_body};

// ─── Root ─────────────────────────────────────────────────────────────────────

/// Parse the entire token stream into the `ROOT` node.
pub(crate) fn parse_root(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::ROOT);

    while !p.at_end() {
        parse_line(p);
    }

    p.finish_node(); // ROOT
}

// ─── Line dispatcher ──────────────────────────────────────────────────────────

pub(crate) fn parse_line(p: &mut Parser<'_>) {
    // Blank line.
    if p.at(SyntaxKind::NEWLINE) {
        p.bump();
        return;
    }

    // Peek past leading whitespace to classify the line.
    let first_meaningful = leading_meaningful_pos(p);

    if first_meaningful >= p.tokens.len()
        || matches!(p.tokens[first_meaningful].token, Token::Newline)
    {
        // Whitespace-only line — consume and continue.
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
        if p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
        return;
    }

    match &p.tokens[first_meaningful].token {
        Token::LineComment => {
            advance_to(p, first_meaningful);
            parse_line_comment(p);
        }
        Token::BlockCommentOpen => {
            advance_to(p, first_meaningful);
            parse_block_comment(p);
        }
        Token::Hash => {
            advance_to(p, first_meaningful);
            parse_chara_line(p);
        }
        Token::Star => {
            advance_to(p, first_meaningful);
            parse_label_def(p);
        }
        Token::At => {
            advance_to(p, first_meaningful);
            parse_at_tag_line(p);
        }
        _ => {
            // Text line — leading whitespace is part of the content.
            parse_text_line(p);
        }
    }
}

/// Advance `p.pos` to absolute index `target` by bumping whitespace tokens.
fn advance_to(p: &mut Parser<'_>, target: usize) {
    while p.pos < target {
        p.bump();
    }
}

/// Return the index of the first non-whitespace token at or after `p.pos`.
fn leading_meaningful_pos(p: &Parser<'_>) -> usize {
    let mut i = p.pos;
    while i < p.tokens.len() && matches!(p.tokens[i].token, Token::Whitespace) {
        i += 1;
    }
    i
}

// ─── Line comment ─────────────────────────────────────────────────────────────

fn parse_line_comment(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::LINE_COMMENT_NODE);
    p.bump(); // LINE_COMMENT (swallows to EOL)
    if p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }
    p.finish_node();
}

// ─── Block comment ────────────────────────────────────────────────────────────

fn parse_block_comment(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::BLOCK_COMMENT_NODE);
    p.bump(); // BLOCK_COMMENT_OPEN
    if p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }

    loop {
        if p.at_end() {
            p.push_error("unclosed block comment `/*` at end of file");
            break;
        }
        // Check for closing `*/` (must be first non-WS token on the line).
        let first = leading_meaningful_pos(p);
        if first < p.tokens.len() && matches!(p.tokens[first].token, Token::BlockCommentClose) {
            advance_to(p, first);
            p.bump(); // BLOCK_COMMENT_CLOSE
            if p.at(SyntaxKind::NEWLINE) {
                p.bump();
            }
            break;
        }
        // Body line — consume as raw tokens.
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
        if p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
    }

    p.finish_node();
}

// ─── Character shorthand (#name or #name:face) ────────────────────────────────

fn parse_chara_line(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::CHARA_LINE);
    p.bump(); // HASH

    // Accept IDENT (ASCII names), TEXT (Japanese/Unicode names, `?`, etc.),
    // or nothing (bare `#` = narrator / clear displayed name).
    if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::TEXT) {
        p.bump(); // name
    }

    if p.at(SyntaxKind::COLON) {
        p.bump(); // COLON
        if !p.at(SyntaxKind::IDENT) && !p.at(SyntaxKind::TEXT) {
            p.error_recover_to_newline("expected face name after `:` in `#name:face`");
            if p.at(SyntaxKind::NEWLINE) {
                p.bump();
            }
            p.finish_node();
            return;
        }
        p.bump(); // face
    }

    if !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        p.error_recover_to_newline("unexpected tokens after character shorthand");
    }
    if p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }
    p.finish_node();
}

// ─── Label definition (*name or *name|title) ──────────────────────────────────

fn parse_label_def(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::LABEL_DEF);
    p.bump(); // STAR

    if !p.at(SyntaxKind::IDENT) {
        p.error_recover_to_newline("expected label name after `*`");
        if p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
        p.finish_node();
        return;
    }
    p.bump(); // name IDENT

    if p.at(SyntaxKind::PIPE) {
        p.bump(); // PIPE
        // Title: consume everything to EOL as raw tokens.
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
    }

    if p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }
    p.finish_node();
}

// ─── Line-level tag (@tagname params…) ───────────────────────────────────────

fn parse_at_tag_line(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::AT_TAG);
    p.bump(); // AT
    p.skip_ws();

    if !p.at(SyntaxKind::IDENT) {
        p.error_recover_to_newline("expected tag name after `@`");
        if p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
        p.finish_node();
        return;
    }

    parse_at_tag_body(p);

    if p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }
    p.finish_node(); // AT_TAG

    // Handle parse-time special tags AFTER the AT_TAG node is closed.
    handle_pending_special(p);
}

// ─── Text line ────────────────────────────────────────────────────────────────

pub(crate) fn parse_text_line(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::TEXT_LINE);

    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        match p.current() {
            SyntaxKind::L_BRACKET => {
                parse_inline_tag(p);
            }
            SyntaxKind::BACKSLASH => {
                p.bump(); // BACKSLASH
                if !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
                    p.start_node(SyntaxKind::TEXT_LITERAL);
                    p.bump();
                    p.finish_node();
                }
            }
            SyntaxKind::AMP => {
                parse_text_entity(p);
            }
            _ => {
                p.start_node(SyntaxKind::TEXT_LITERAL);
                while !p.at_end()
                    && !p.at(SyntaxKind::NEWLINE)
                    && !p.at(SyntaxKind::L_BRACKET)
                    && !p.at(SyntaxKind::BACKSLASH)
                    && !p.at(SyntaxKind::AMP)
                {
                    p.bump();
                }
                p.finish_node();
            }
        }
    }

    if p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }
    p.finish_node(); // TEXT_LINE

    // A text line may contain only inline tags; special tag side-effects apply.
    handle_pending_special(p);
}

// ─── Inline entity (&expr in text) ───────────────────────────────────────────

fn parse_text_entity(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::TEXT_ENTITY);
    p.bump(); // AMP
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) && !p.at(SyntaxKind::L_BRACKET) {
        p.bump();
    }
    p.finish_node();
}

// ─── Inline tag ([tagname params…]) ──────────────────────────────────────────

pub(crate) fn parse_inline_tag(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::INLINE_TAG);
    p.bump(); // L_BRACKET
    p.skip_ws();

    if !p.at(SyntaxKind::IDENT) {
        p.error_recover_until("expected tag name after `[`", &[SyntaxKind::R_BRACKET]);
        if p.at(SyntaxKind::R_BRACKET) {
            p.bump();
        }
        p.finish_node();
        return;
    }

    parse_inline_tag_body(p);

    if p.at(SyntaxKind::R_BRACKET) {
        p.bump();
    } else {
        p.push_error("unclosed inline tag — missing `]`");
    }
    p.finish_node(); // INLINE_TAG
    // Note: handle_pending_special is called by parse_text_line after the
    // whole text line is done; inline-tag side-effects are deferred.
}

// ─── Special tag side-effects ─────────────────────────────────────────────────

/// After emitting an AT_TAG or TEXT_LINE node, check whether the last parsed
/// tag was a special one (iscript, macro) and emit the corresponding body node.
fn handle_pending_special(p: &mut Parser<'_>) {
    if p.pending_iscript {
        p.pending_iscript = false;
        parse_iscript_block(p);
    } else if p.pending_macro {
        p.pending_macro = false;
        parse_macro_def_body(p);
    }
}

// ─── iscript block ────────────────────────────────────────────────────────────

/// Accumulates raw content into an `ISCRIPT_BLOCK` node until `[endscript]`.
pub(crate) fn parse_iscript_block(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::ISCRIPT_BLOCK);

    loop {
        if p.at_end() {
            p.push_error("unclosed `[iscript]` block — missing `[endscript]`");
            break;
        }
        if is_tag_named(p, "endscript") {
            // Close the ISCRIPT_BLOCK node before consuming [endscript] so
            // the closing tag tokens are not included in the script content.
            p.finish_node();
            consume_line(p);
            return;
        }
        // Raw content line.
        while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
        if p.at(SyntaxKind::NEWLINE) {
            p.bump();
        }
    }

    p.finish_node();
}

// ─── macro def body ───────────────────────────────────────────────────────────

/// After a `@macro name=foo` line, accumulate the body into a `MACRO_DEF`
/// node until `[endmacro]`.
pub(crate) fn parse_macro_def_body(p: &mut Parser<'_>) {
    p.start_node(SyntaxKind::MACRO_DEF);

    let mut depth = 1usize;
    loop {
        if p.at_end() {
            p.push_error("unclosed `[macro]` block — missing `[endmacro]`");
            break;
        }
        if is_tag_named(p, "endmacro") && depth == 1 {
            consume_line(p);
            break;
        }
        if is_tag_named(p, "macro") {
            depth += 1;
        } else if is_tag_named(p, "endmacro") {
            depth = depth.saturating_sub(1);
        }
        parse_line(p);
    }

    p.finish_node();
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Consume all tokens on the current line including the trailing newline.
fn consume_line(p: &mut Parser<'_>) {
    while !p.at_end() && !p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }
    if p.at(SyntaxKind::NEWLINE) {
        p.bump();
    }
}

/// `true` if the current line (without consuming) starts with `@name` or `[name]`.
pub(crate) fn is_tag_named(p: &Parser<'_>, name: &str) -> bool {
    let first = leading_meaningful_pos(p);
    if first >= p.tokens.len() {
        return false;
    }
    match &p.tokens[first].token {
        Token::At => p.tokens.get(first + 1).is_some_and(|t| t.slice == name),
        Token::LBracket => p.tokens.get(first + 1).is_some_and(|t| t.slice == name),
        _ => false,
    }
}
