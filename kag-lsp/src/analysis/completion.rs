//! Completion provider — tag names, parameter names, macro names, and label targets.

use kag_syntax::tag_defs::TagName;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind};

use crate::store::ParsedDoc;

/// Return completion items for the given context.
///
/// `in_param_value` — true when the cursor is after `=` in a tag param.
/// `param_key` — the key whose value is being completed (e.g. `"target"`).
/// `current_tag` — the tag name the cursor is inside, if known.
pub fn completions(
    doc: &ParsedDoc,
    in_param_value: bool,
    param_key: Option<&str>,
    current_tag: Option<&str>,
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

    // ── Tag-position completions ──────────────────────────────────────────────

    // If we know the current tag, offer its known parameter names first.
    if let Some(tag_str) = current_tag
        && let Some(tag_name) = TagName::from_name(tag_str)
    {
        for &param in tag_name.param_names() {
            items.push(CompletionItem {
                label: param.to_owned(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(format!("param of [{}]", tag_str)),
                ..Default::default()
            });
        }
        // Return early: when inside an existing tag, only params are relevant.
        return items;
    }

    // Built-in tag names from the macro-generated TagName enum.
    let builtin_names: std::collections::HashSet<&str> =
        TagName::all().map(|t| t.as_str()).collect();

    for name in &builtin_names {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("built-in tag".into()),
            ..Default::default()
        });
    }

    for macro_name in doc.index.macros.keys() {
        // Don't duplicate if a macro shadows a built-in.
        let name_str: &str = macro_name.as_str();
        if !builtin_names.contains(name_str) {
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
