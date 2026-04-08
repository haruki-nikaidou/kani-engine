//! Find-references provider.
//!
//! Given a symbol name under the cursor, returns all locations in the document
//! where that name appears as a tag name or as a `target=` / `storage=` value.

use kag_syntax::SyntaxKind;
use rowan::{TextSize, TokenAtOffset};
use tower_lsp::lsp_types::{Location, Url};

use crate::convert::text_range_to_lsp_range;
use crate::store::ParsedDoc;

/// Return all reference locations for the symbol at `offset`.
pub fn find_references(
    doc: &ParsedDoc,
    uri: &Url,
    offset: usize,
    include_declaration: bool,
) -> Vec<Location> {
    let name = match symbol_name_at(doc, offset) {
        Some(n) => n,
        None => return Vec::new(),
    };

    let mut locations: Vec<Location> = Vec::new();

    // All tag-name occurrences matching the name.
    for (ref_name, range) in &doc.index.tag_refs {
        if ref_name == &name {
            locations.push(Location {
                uri: uri.clone(),
                range: text_range_to_lsp_range(&doc.source, *range),
            });
        }
    }

    // Also scan `target=<name>` param values in the CST.
    collect_param_value_refs(doc, uri, &name, &mut locations);

    // Optionally include the declaration itself.
    if include_declaration {
        if let Some(&label_range) = doc.index.labels.get(&name) {
            let lsp_range = text_range_to_lsp_range(&doc.source, label_range);
            if !locations.iter().any(|l| l.range == lsp_range) {
                locations.push(Location { uri: uri.clone(), range: lsp_range });
            }
        }
        if let Some(&macro_range) = doc.index.macros.get(&name) {
            let lsp_range = text_range_to_lsp_range(&doc.source, macro_range);
            if !locations.iter().any(|l| l.range == lsp_range) {
                locations.push(Location { uri: uri.clone(), range: lsp_range });
            }
        }
    }

    locations
}

/// Extract the identifier text at `offset` in the document.
fn symbol_name_at(doc: &ParsedDoc, offset: usize) -> Option<String> {
    let root_syntax = doc.parse.syntax_node();
    let offset_u32 = TextSize::from(offset as u32);

    let token = match root_syntax.token_at_offset(offset_u32) {
        TokenAtOffset::None => return None,
        TokenAtOffset::Single(t) => t,
        TokenAtOffset::Between(left, right) => {
            if left.kind() == SyntaxKind::WHITESPACE { right } else { left }
        }
    };

    if token.kind() == SyntaxKind::IDENT {
        Some(token.text().to_owned())
    } else {
        None
    }
}

/// Walk param values looking for `target=<name>` or `storage=<name>` matches.
fn collect_param_value_refs(
    doc: &ParsedDoc,
    uri: &Url,
    name: &str,
    out: &mut Vec<Location>,
) {
    use kag_syntax::cst::{Item, TextPart};
    let root = doc.parse.tree();

    for item in root.items() {
        match item {
            Item::AtTag(tag) => scan_params(tag.params(), name, doc, uri, out),
            Item::InlineTag(tag) => scan_params(tag.params(), name, doc, uri, out),
            Item::TextLine(line) => {
                for part in line.parts() {
                    if let TextPart::InlineTag(tag) = part {
                        scan_params(tag.params(), name, doc, uri, out);
                    }
                }
            }
            Item::MacroDef(def) => {
                for child in def.items() {
                    match child {
                        Item::AtTag(tag) => scan_params(tag.params(), name, doc, uri, out),
                        Item::InlineTag(tag) => scan_params(tag.params(), name, doc, uri, out),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn scan_params(
    params: impl Iterator<Item = kag_syntax::cst::Param>,
    name: &str,
    doc: &ParsedDoc,
    uri: &Url,
    out: &mut Vec<Location>,
) {
    use kag_syntax::cst::{AstNode, ParamValue};
    for param in params {
        if !matches!(param.key().as_deref(), Some("target") | Some("storage")) {
            continue;
        }
        if let Some(ParamValue::Literal(lit)) = param.value()
            && lit.text() == name
        {
            let range = text_range_to_lsp_range(&doc.source, lit.syntax().text_range());
            if !out.iter().any(|l| l.range == range) {
                out.push(Location { uri: uri.clone(), range });
            }
        }
    }
}
