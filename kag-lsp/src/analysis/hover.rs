//! Hover provider — returns Markdown documentation for the token under cursor.

use kag_syntax::SyntaxKind;
use rowan::{TextSize, TokenAtOffset};
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use crate::convert::{offset_to_position, text_range_to_lsp_range};
use crate::store::ParsedDoc;

/// Built-in KAG tags and their brief descriptions.
const BUILTIN_TAGS: &[(&str, &str)] = &[
    ("r", "Insert a line break in the message window."),
    ("p", "Wait for a page-break click, then clear the window."),
    ("l", "Wait for a click (line wait)."),
    (
        "jump",
        "Jump to another label or file. Params: `storage`, `target`.",
    ),
    ("call", "Call a subroutine. Params: `storage`, `target`."),
    ("return", "Return from a subroutine."),
    (
        "wait",
        "Wait for a fixed number of milliseconds. Params: `time`.",
    ),
    ("macro", "Define a macro. Params: `name`."),
    ("endmacro", "End a macro definition."),
    ("iscript", "Begin an inline Rhai script block."),
    ("endscript", "End an inline script block."),
    ("eval", "Evaluate a Rhai expression. Params: `exp`."),
];

/// Return hover content for the symbol at `offset` in `doc`, or `None`.
pub fn hover(doc: &ParsedDoc, offset: usize) -> Option<Hover> {
    let root_syntax = doc.parse.syntax_node();
    let offset_u32 = TextSize::from(offset as u32);

    let token = match root_syntax.token_at_offset(offset_u32) {
        TokenAtOffset::None => return None,
        TokenAtOffset::Single(t) => t,
        TokenAtOffset::Between(left, right) => {
            if right.kind() == SyntaxKind::IDENT {
                right
            } else if left.kind() == SyntaxKind::WHITESPACE {
                right
            } else {
                left
            }
        }
    };

    if token.kind() != SyntaxKind::IDENT {
        return None;
    }

    let text = token.text();
    let token_range = text_range_to_lsp_range(&doc.source, token.text_range());

    // Walk up to find the containing node kind.
    let parent = token.parent()?;
    let grandparent = parent.parent();

    let markdown = match parent.kind() {
        SyntaxKind::TAG_NAME => {
            // Tag name — show built-in docs or macro summary.
            if let Some(desc) = builtin_tag_description(text) {
                format!("**tag** `{text}`\n\n{desc}")
            } else if let Some(&macro_range) = doc.index.macros.get(text) {
                let start = offset_to_position(&doc.source, usize::from(macro_range.start()));
                format!("**macro** `{text}`\n\nDefined at line {}.", start.line + 1)
            } else {
                format!("**tag** `{text}`")
            }
        }
        SyntaxKind::LABEL_DEF => {
            format!("**label definition** `*{text}`")
        }
        SyntaxKind::PARAM_VALUE_LITERAL | SyntaxKind::PARAM_VALUE_MACRO => {
            // Check if the param key is `target` or `storage` (jump/call dest).
            if is_target_param(&grandparent) {
                if let Some(&label_range) = doc.index.labels.get(text) {
                    let start = offset_to_position(&doc.source, usize::from(label_range.start()));
                    format!("**label** `*{text}`\n\nDefined at line {}.", start.line + 1)
                } else {
                    format!("**label target** `{text}` *(not found in this file)*")
                }
            } else {
                return None;
            }
        }
        SyntaxKind::PARAM_KEY => {
            format!("**param** `{text}`")
        }
        _ => return None,
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(token_range),
    })
}

fn builtin_tag_description(name: &str) -> Option<&'static str> {
    BUILTIN_TAGS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, d)| *d)
}

/// Returns `true` when `node` is a `PARAM` whose key is `target` or `storage`.
fn is_target_param(node: &Option<kag_syntax::SyntaxNode>) -> bool {
    let Some(param) = node else { return false };
    if param.kind() != SyntaxKind::PARAM {
        return false;
    }
    param
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .any(|t: kag_syntax::SyntaxToken| {
            t.kind() == SyntaxKind::IDENT && matches!(t.text(), "target" | "storage")
        })
}
