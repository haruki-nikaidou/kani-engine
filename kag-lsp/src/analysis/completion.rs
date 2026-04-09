//! Completion provider — tag names, macro names, and label targets.

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind};

use crate::store::ParsedDoc;

/// Built-in KAG tag names offered as keyword completions.
const BUILTIN_TAG_NAMES: &[&str] = &[
    "r",
    "p",
    "l",
    "jump",
    "call",
    "return",
    "wait",
    "macro",
    "endmacro",
    "iscript",
    "endscript",
    "eval",
    "if",
    "else",
    "endif",
    "emb",
    "set",
];

/// Return completion items for the given context.
///
/// `in_param_value` — true when the cursor is after `=` in a tag param.
/// `param_key` — the key whose value is being completed (e.g. `"target"`).
pub fn completions(
    doc: &ParsedDoc,
    in_param_value: bool,
    param_key: Option<&str>,
) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = Vec::new();

    if in_param_value {
        // Only offer label names when completing `target=` / `storage=`.
        if matches!(param_key, Some("target") | Some("storage")) {
            for label_name in doc.index.labels.keys() {
                items.push(CompletionItem {
                    label: label_name.as_str().to_owned(),
                    kind: Some(CompletionItemKind::REFERENCE),
                    detail: Some("label".into()),
                    ..Default::default()
                });
            }
        }
        return items;
    }

    // Tag-position completions: built-ins + user macros.
    for &name in BUILTIN_TAG_NAMES {
        items.push(CompletionItem {
            label: name.to_owned(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("built-in tag".into()),
            ..Default::default()
        });
    }

    for macro_name in doc.index.macros.keys() {
        // Don't duplicate if a macro shadows a built-in.
        let name_str: &str = macro_name.as_str();
        if !BUILTIN_TAG_NAMES.contains(&name_str) {
            items.push(CompletionItem {
                label: name_str.to_owned(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("macro".into()),
                ..Default::default()
            });
        }
    }

    items
}
