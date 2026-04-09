//! Semantic index and analysis helpers built on top of the KAG CST.

pub mod completion;
pub mod goto_def;
pub mod hover;
pub mod references;
pub mod symbols;

use std::collections::HashMap;

use kag_syntax::cst::{self, AstNode, Item};
use kag_syntax::parser::Parse;
use rowan::TextRange;

// ─── Index ────────────────────────────────────────────────────────────────────

/// Semantic index extracted from a single document.
///
/// Built once per parse and consulted by all LSP handlers.
pub struct Index {
    /// `label_name` → `TextRange` of the `LABEL_DEF` node.
    pub labels: HashMap<String, TextRange>,
    /// `macro_name` → `TextRange` of the `MACRO_DEF` node.
    pub macros: HashMap<String, TextRange>,
    /// All tag-name tokens found in the file: `(name_text, TextRange)`.
    ///
    /// Includes both `AT_TAG` and `INLINE_TAG` names.  Used by
    /// find-references and completion.
    pub tag_refs: Vec<(String, TextRange)>,
}

impl Index {
    /// Build an [`Index`] by walking the CST.
    pub fn build(parse: &Parse<cst::Root>, _source: &str) -> Self {
        let mut labels: HashMap<String, TextRange> = HashMap::new();
        let mut macros: HashMap<String, TextRange> = HashMap::new();
        let mut tag_refs: Vec<(String, TextRange)> = Vec::new();

        let root = parse.tree();

        for item in root.items() {
            index_item(&item, &mut labels, &mut macros, &mut tag_refs);
        }

        Self {
            labels,
            macros,
            tag_refs,
        }
    }
}

// ─── Item walker ─────────────────────────────────────────────────────────────

fn index_item(
    item: &Item,
    labels: &mut HashMap<String, TextRange>,
    macros: &mut HashMap<String, TextRange>,
    tag_refs: &mut Vec<(String, TextRange)>,
) {
    match item {
        Item::LabelDef(label) => {
            if let Some(name) = label.name() {
                labels
                    .entry(name)
                    .or_insert_with(|| label.syntax().text_range());
            }
        }
        Item::AtTag(tag) => {
            index_tag_name(tag.tag_name_node().as_ref(), tag_refs);

            // If this is `@macro name=foo`, record the macro definition.
            // The MACRO_DEF sibling is walked separately.
            if tag.name().as_deref() == Some("macro")
                && let Some(macro_name) = tag_param_value(tag.params(), "name")
            {
                macros
                    .entry(macro_name)
                    .or_insert_with(|| tag.syntax().text_range());
            }
        }
        Item::InlineTag(tag) => {
            index_tag_name(tag.tag_name_node().as_ref(), tag_refs);

            if tag.name().as_deref() == Some("macro")
                && let Some(macro_name) = tag_param_value(tag.params(), "name")
            {
                macros
                    .entry(macro_name)
                    .or_insert_with(|| tag.syntax().text_range());
            }
        }
        Item::MacroDef(def) => {
            // Walk body items recursively.
            for child in def.items() {
                index_item(&child, labels, macros, tag_refs);
            }
        }
        Item::TextLine(line) => {
            for part in line.parts() {
                if let cst::TextPart::InlineTag(tag) = part {
                    index_tag_name(tag.tag_name_node().as_ref(), tag_refs);
                }
            }
        }
        _ => {}
    }
}

fn index_tag_name(name_node: Option<&cst::TagName>, tag_refs: &mut Vec<(String, TextRange)>) {
    if let Some(node) = name_node
        && let Some(tok) = node.ident_token()
    {
        tag_refs.push((tok.text().to_owned(), tok.text_range()));
    }
}

/// Extract the value of a named parameter from a tag's params iterator.
fn tag_param_value(mut params: impl Iterator<Item = cst::Param>, key: &str) -> Option<String> {
    params.find_map(|p| {
        if p.key().as_deref() == Some(key)
            && let Some(cst::ParamValue::Literal(lit)) = p.value()
        {
            return Some(lit.text());
        }
        None
    })
}
