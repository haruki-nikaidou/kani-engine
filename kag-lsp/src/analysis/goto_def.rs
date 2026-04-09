//! Go-to-definition provider.
//!
//! Resolves:
//! - A `target=<name>` param value → the `*name` label definition.
//! - A tag name that is a user macro → the `@macro name=<name>` tag.

use kag_syntax::SyntaxKind;
use rowan::{TextSize, TokenAtOffset};
use tower_lsp::lsp_types::{Location, Url};

use crate::convert::text_range_to_lsp_range;
use crate::store::ParsedDoc;

/// Return the definition [`Location`] for the symbol at `offset`, if any.
pub fn goto_definition(doc: &ParsedDoc, uri: &Url, offset: usize) -> Option<Location> {
    let root_syntax = doc.parse.syntax_node();
    let offset_u32 = TextSize::from(offset as u32);

    let token = match root_syntax.token_at_offset(offset_u32) {
        TokenAtOffset::None => return None,
        TokenAtOffset::Single(t) => t,
        TokenAtOffset::Between(left, right) => {
            if left.kind() == SyntaxKind::WHITESPACE {
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
    let parent = token.parent()?;

    match parent.kind() {
        SyntaxKind::TAG_NAME => {
            // Macro call → jump to macro definition tag.
            let target_range = *doc.index.macros.get(text)?;
            let lsp_range = text_range_to_lsp_range(&doc.source, target_range);
            Some(Location {
                uri: uri.clone(),
                range: lsp_range,
            })
        }
        SyntaxKind::PARAM_VALUE_LITERAL => {
            // Could be `target=labelname` or `storage=filename`.
            let grandparent = parent.parent()?;
            if grandparent.kind() == SyntaxKind::PARAM && is_target_param_node(&grandparent) {
                let target_range = *doc.index.labels.get(text)?;
                let lsp_range = text_range_to_lsp_range(&doc.source, target_range);
                Some(Location {
                    uri: uri.clone(),
                    range: lsp_range,
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn is_target_param_node(param: &kag_syntax::SyntaxNode) -> bool {
    // The parser wraps the key IDENT inside a PARAM_KEY child *node*, so it is
    // never a direct token child of PARAM.  We must descend into PARAM_KEY to
    // read the parameter name; only then check whether it is "target" or
    // "storage".  Iterating direct token children (as the old code did) always
    // yielded only the EQ token and therefore always returned false.
    param
        .children()
        .find(|n| n.kind() == SyntaxKind::PARAM_KEY)
        .and_then(|key_node| {
            key_node
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.kind() == SyntaxKind::IDENT)
        })
        .map(|t| matches!(t.text(), "target" | "storage"))
        .unwrap_or(false)
}
