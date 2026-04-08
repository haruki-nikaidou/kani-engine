//! Document symbol provider — builds a flat outline of labels and macros.

use kag_syntax::cst::{AstNode, Item};
use rowan::TextRange;
use tower_lsp::lsp_types::{DocumentSymbol, SymbolKind};

use crate::convert::text_range_to_lsp_range;
use crate::store::ParsedDoc;

/// Build the document symbol list for `doc`.
///
/// Returns a flat list of [`DocumentSymbol`] entries for every label
/// definition (`*name`) and macro definition (`@macro name=...`).
pub fn document_symbols(doc: &ParsedDoc) -> Vec<DocumentSymbol> {
    let mut out = Vec::new();
    let root = doc.parse.tree();

    for item in root.items() {
        match &item {
            Item::LabelDef(label) => {
                let name = label.name().unwrap_or_else(|| "<anonymous>".into());
                let range = text_range_to_lsp_range(&doc.source, label.syntax().text_range());
                let detail = label.title();
                #[allow(deprecated)]
                out.push(DocumentSymbol {
                    name,
                    detail,
                    kind: SymbolKind::KEY,
                    range,
                    selection_range: range,
                    children: None,
                    tags: None,
                    deprecated: None,
                });
            }
            Item::MacroDef(def) => {
                // Macro name comes from the preceding @macro / [macro] tag.
                // The def node itself carries the body; we report the whole span.
                let range = text_range_to_lsp_range(&doc.source, def.syntax().text_range());

                // Try to find the macro name from index (it was registered
                // by the preceding @macro tag, whose span starts just before
                // the MacroDef).  Fall back to searching by range proximity.
                let name = find_macro_name_for_range(doc, def.syntax().text_range())
                    .unwrap_or_else(|| "<macro>".into());

                #[allow(deprecated)]
                out.push(DocumentSymbol {
                    name,
                    detail: None,
                    kind: SymbolKind::FUNCTION,
                    range,
                    selection_range: range,
                    children: None,
                    tags: None,
                    deprecated: None,
                });
            }
            _ => {}
        }
    }

    out
}

/// Find the macro name whose registered range immediately precedes `def_range`.
fn find_macro_name_for_range(doc: &ParsedDoc, def_range: TextRange) -> Option<String> {
    doc.index
        .macros
        .iter()
        .filter(|(_, tag_range): &(_, &TextRange)| tag_range.end() <= def_range.start())
        .max_by_key(|(_, r): &(_, &TextRange)| r.start())
        .map(|(name, _): (&String, _)| name.clone())
}
