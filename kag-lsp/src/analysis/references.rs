//! Find-references provider.
//!
//! Given a symbol under the cursor, returns all locations in the document
//! where that name appears — scoped to the correct symbol kind so that a
//! tag named `foo` and a label named `foo` never produce cross-kind hits.

use kag_syntax::SyntaxKind;
use rowan::{TextSize, TokenAtOffset};
use tower_lsp::lsp_types::{Location, Url};

use crate::analysis::goto_def::is_target_param_node;
use crate::convert::text_range_to_lsp_range;
use crate::store::ParsedDoc;

// ─── Symbol context ───────────────────────────────────────────────────────────

/// The syntactic role of the identifier under the cursor.
#[derive(Debug, PartialEq, Eq)]
enum SymbolKind {
    /// Inside a `TAG_NAME` node — a tag invocation such as `@foo` or `[foo]`.
    TagName,
    /// Inside a `PARAM_VALUE_LITERAL` that belongs to a `target=` parameter
    /// — a reference to a label name in the current document.
    ParamTarget,
    /// Inside a `PARAM_VALUE_LITERAL` that belongs to a `storage=` parameter
    /// — a reference to an external script file name.
    ParamStorage,
    /// Parent node kind is neither of the above; fall back to searching both
    /// index buckets and deduplicate.
    Unknown,
}

struct CursorSymbol {
    name: String,
    kind: SymbolKind,
}

// ─── Public entry point ───────────────────────────────────────────────────────

/// Return all reference locations for the symbol at `offset`.
///
/// The search is scoped to the symbol's syntactic context:
/// * **Tag name** → only `doc.index.tag_refs` is consulted.
/// * **ParamTarget** (`target=`) → only `target=` param-value occurrences;
///   label declaration is included when requested.
/// * **ParamStorage** (`storage=`) → only `storage=` param-value occurrences;
///   no label declaration is attached (storage values are file names, not
///   labels).
/// * **Unknown** → both buckets are searched (safe fallback).
pub fn find_references(
    doc: &ParsedDoc,
    uri: &Url,
    offset: usize,
    include_declaration: bool,
) -> Vec<Location> {
    let sym = match symbol_at_offset(doc, offset) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut locations: Vec<Location> = Vec::new();

    match sym.kind {
        SymbolKind::TagName => {
            // Only tag-name occurrences — never mix in param-value hits.
            for (ref_name, range) in &doc.index.tag_refs {
                if ref_name == &sym.name {
                    locations.push(Location {
                        uri: uri.clone(),
                        range: text_range_to_lsp_range(&doc.source, *range),
                    });
                }
            }

            // Declaration: a tag name resolves to a macro definition.
            if include_declaration && let Some(&macro_range) = doc.index.macros.get(&sym.name) {
                let lsp_range = text_range_to_lsp_range(&doc.source, macro_range);
                if !locations.iter().any(|l| l.range == lsp_range) {
                    locations.push(Location {
                        uri: uri.clone(),
                        range: lsp_range,
                    });
                }
            }
        }

        SymbolKind::ParamTarget => {
            // Only target= param-value occurrences — never mix in tag-name
            // hits or storage= hits.
            collect_param_value_refs(doc, uri, &sym.name, &["target"], &mut locations);

            // Declaration: a target= value resolves to a label definition.
            if include_declaration && let Some(&label_range) = doc.index.labels.get(&sym.name) {
                let lsp_range = text_range_to_lsp_range(&doc.source, label_range);
                if !locations.iter().any(|l| l.range == lsp_range) {
                    locations.push(Location {
                        uri: uri.clone(),
                        range: lsp_range,
                    });
                }
            }
        }

        SymbolKind::ParamStorage => {
            // Only storage= param-value occurrences — never mix in tag-name
            // hits or target= hits.  Storage values are file names, not label
            // names, so consulting doc.index.labels would produce spurious
            // hits for any label that happens to share the file's stem.  The
            // actual declaration lives in an external document, so no
            // declaration range is attached here.
            collect_param_value_refs(doc, uri, &sym.name, &["storage"], &mut locations);
        }

        SymbolKind::Unknown => {
            // Context is ambiguous — search every bucket and deduplicate.
            // This is the old behaviour, kept as a safe fallback for any
            // IDENT nodes that the parser places outside TAG_NAME /
            // PARAM_VALUE_LITERAL.
            for (ref_name, range) in &doc.index.tag_refs {
                if ref_name == &sym.name {
                    locations.push(Location {
                        uri: uri.clone(),
                        range: text_range_to_lsp_range(&doc.source, *range),
                    });
                }
            }

            collect_param_value_refs(doc, uri, &sym.name, &["target", "storage"], &mut locations);

            if include_declaration {
                if let Some(&label_range) = doc.index.labels.get(&sym.name) {
                    let lsp_range = text_range_to_lsp_range(&doc.source, label_range);
                    if !locations.iter().any(|l| l.range == lsp_range) {
                        locations.push(Location {
                            uri: uri.clone(),
                            range: lsp_range,
                        });
                    }
                }
                if let Some(&macro_range) = doc.index.macros.get(&sym.name) {
                    let lsp_range = text_range_to_lsp_range(&doc.source, macro_range);
                    if !locations.iter().any(|l| l.range == lsp_range) {
                        locations.push(Location {
                            uri: uri.clone(),
                            range: lsp_range,
                        });
                    }
                }
            }
        }
    }

    locations
}

// ─── Cursor resolution ────────────────────────────────────────────────────────

/// Resolve the IDENT token at `offset` to its name **and** syntactic context.
///
/// Uses the same `token.parent().kind()` pattern as `goto_def` so the two
/// providers stay consistent with each other.
fn symbol_at_offset(doc: &ParsedDoc, offset: usize) -> Option<CursorSymbol> {
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

    let name = token.text().to_owned();
    let parent = token.parent()?;

    let kind = match parent.kind() {
        SyntaxKind::TAG_NAME => SymbolKind::TagName,

        SyntaxKind::PARAM_VALUE_LITERAL => {
            // Only classify as a typed param reference when the enclosing
            // PARAM is a `target=` or `storage=` key, consistent with how
            // goto_def resolves the same node.
            let grandparent = parent.parent()?;
            if grandparent.kind() == SyntaxKind::PARAM && is_target_param_node(&grandparent) {
                // Use the specific key name to choose the right variant so
                // that the two referents are kept entirely separate: target=
                // names labels while storage= names external files.
                match param_key_name(&grandparent).as_deref() {
                    Some("target") => SymbolKind::ParamTarget,
                    Some("storage") => SymbolKind::ParamStorage,
                    // is_target_param_node only passes "target"/"storage", so
                    // this arm is a defensive fallback.
                    _ => SymbolKind::Unknown,
                }
            } else {
                SymbolKind::Unknown
            }
        }

        _ => SymbolKind::Unknown,
    };

    Some(CursorSymbol { name, kind })
}

/// Extract the key-name text from a `PARAM` node.
///
/// Mirrors the same `PARAM_KEY` descent used by [`is_target_param_node`] in
/// `goto_def` so the two modules stay consistent without creating a shared
/// dependency on an internal helper.
fn param_key_name(param: &kag_syntax::SyntaxNode) -> Option<String> {
    param
        .children()
        .find(|n| n.kind() == SyntaxKind::PARAM_KEY)
        .and_then(|key_node| {
            key_node
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| t.kind() == SyntaxKind::IDENT)
        })
        .map(|t| t.text().to_owned())
}

// ─── Param-value scanner ──────────────────────────────────────────────────────

/// Walk the CST and collect every param-value location whose key is one of
/// `keys` and whose value text matches `name`.
///
/// Pass `keys = &["target"]` to restrict to `target=` params, `&["storage"]`
/// for `storage=` params, or `&["target", "storage"]` for both (used by the
/// `Unknown` fallback path).
fn collect_param_value_refs(
    doc: &ParsedDoc,
    uri: &Url,
    name: &str,
    keys: &[&str],
    out: &mut Vec<Location>,
) {
    use kag_syntax::cst::{Item, TextPart};
    let root = doc.parse.tree();

    for item in root.items() {
        match item {
            Item::AtTag(tag) => scan_params(tag.params(), name, keys, doc, uri, out),
            Item::InlineTag(tag) => scan_params(tag.params(), name, keys, doc, uri, out),
            Item::TextLine(line) => {
                for part in line.parts() {
                    if let TextPart::InlineTag(tag) = part {
                        scan_params(tag.params(), name, keys, doc, uri, out);
                    }
                }
            }
            Item::MacroDef(def) => {
                for child in def.items() {
                    match child {
                        Item::AtTag(tag) => scan_params(tag.params(), name, keys, doc, uri, out),
                        Item::InlineTag(tag) => {
                            scan_params(tag.params(), name, keys, doc, uri, out)
                        }
                        Item::TextLine(line) => {
                            for part in line.parts() {
                                if let TextPart::InlineTag(tag) = part {
                                    scan_params(tag.params(), name, keys, doc, uri, out);
                                }
                            }
                        }
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
    keys: &[&str],
    doc: &ParsedDoc,
    uri: &Url,
    out: &mut Vec<Location>,
) {
    use kag_syntax::cst::{AstNode, ParamValue};
    for param in params {
        // Only process params whose key is in the caller-supplied allow-list.
        if !param.key().as_deref().is_some_and(|k| keys.contains(&k)) {
            continue;
        }
        if let Some(ParamValue::Literal(lit)) = param.value()
            && lit.text() == name
        {
            let range = text_range_to_lsp_range(&doc.source, lit.syntax().text_range());
            if !out.iter().any(|l| l.range == range) {
                out.push(Location {
                    uri: uri.clone(),
                    range,
                });
            }
        }
    }
}
