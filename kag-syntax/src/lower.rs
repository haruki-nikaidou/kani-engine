//! CST → semantic AST lowering.
//!
//! Walks a [`cst::Root`] node and produces a [`Script<'static>`] together with
//! any diagnostics that arise during the lowering (e.g. duplicate labels).
//! All string data is owned; the resulting `Script` is `'static`.

use std::borrow::Cow;
use std::collections::HashMap;

use miette::SourceSpan;

use crate::ast::{LabelDef, MacroDef, Op, Param, ParamValue, Script, Tag, TextPart};
use crate::cst::{self, Item};
use crate::error::SyntaxWarning;
use crate::tag_defs::KnownTag;
// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn lower_root(root: cst::Root, source_name: &str) -> (Script<'static>, Vec<SyntaxWarning>) {
    let mut ctx = LowerCtx::new(source_name);
    ctx.lower_items(root.items());
    let LowerCtx {
        ops,
        label_map,
        macro_map,
        source_name: sn,
        errors,
        ..
    } = ctx;
    let script = Script {
        ops,
        label_map,
        macro_map,
        source_name: sn,
    };
    (script, errors)
}

// ─── Lowering context ─────────────────────────────────────────────────────────

struct LowerCtx {
    ops: Vec<Op<'static>>,
    label_map: HashMap<Cow<'static, str>, usize>,
    macro_map: HashMap<Cow<'static, str>, MacroDef>,
    macro_stack: Vec<Cow<'static, str>>,
    source_name: String,
    errors: Vec<SyntaxWarning>,
}

impl LowerCtx {
    fn new(source_name: &str) -> Self {
        Self {
            ops: Vec::new(),
            label_map: HashMap::new(),
            macro_map: HashMap::new(),
            macro_stack: Vec::new(),
            source_name: source_name.to_owned(),
            errors: Vec::new(),
        }
    }

    fn emit(&mut self, op: Op<'static>) {
        self.ops.push(op);
    }

    fn push_error(&mut self, message: impl Into<String>, span: SourceSpan) {
        self.errors.push(SyntaxWarning::error(message, span));
    }

    fn push_warning(&mut self, message: impl Into<String>, span: SourceSpan) {
        self.errors.push(SyntaxWarning::warning(message, span));
    }
}

// ─── Item dispatch ────────────────────────────────────────────────────────────

impl LowerCtx {
    fn lower_items(&mut self, items: impl Iterator<Item = Item>) {
        // We need to look ahead for ISCRIPT_BLOCK and MACRO_DEF siblings that
        // belong to the preceding iscript / macro tag.  Collect into a vec first.
        let items: Vec<Item> = items.collect();
        let mut i = 0;
        while i < items.len() {
            match &items[i] {
                Item::AtTag(tag) => {
                    let name = tag.name().unwrap_or_default();
                    match name.as_str() {
                        "iscript" => {
                            // The next sibling should be an ISCRIPT_BLOCK.
                            if let Some(Item::IscriptBlock(block)) = items.get(i + 1) {
                                self.lower_iscript_block(block, tag.span());
                                i += 2;
                                continue;
                            } else {
                                // Iscript without a body (error already in parser).
                                self.emit(Op::ScriptBlock {
                                    content: String::new(),
                                    span: tag.span(),
                                });
                            }
                        }
                        "macro" => {
                            if let Some(Item::MacroDef(def)) = items.get(i + 1) {
                                self.lower_macro_def(tag, def);
                                i += 2;
                                continue;
                            }
                            // No body.
                        }
                        "endmacro" | "endscript" => {
                            // Should not appear outside their respective blocks.
                        }
                        _ => {
                            if let Some(ast_tag) =
                                self.lower_tag_node(tag.name(), tag.params(), tag.span())
                            {
                                self.handle_tag(ast_tag);
                            }
                        }
                    }
                }
                Item::InlineTag(tag) => {
                    let name = tag.name().unwrap_or_default();
                    match name.as_str() {
                        "iscript" => {
                            if let Some(Item::IscriptBlock(block)) = items.get(i + 1) {
                                self.lower_iscript_block(block, tag.span());
                                i += 2;
                                continue;
                            } else {
                                self.emit(Op::ScriptBlock {
                                    content: String::new(),
                                    span: tag.span(),
                                });
                            }
                        }
                        "macro" => {
                            if let Some(Item::MacroDef(def)) = items.get(i + 1) {
                                self.lower_macro_def_from_inline(tag, def);
                                i += 2;
                                continue;
                            }
                        }
                        "endmacro" | "endscript" => {}
                        _ => {
                            if let Some(ast_tag) =
                                self.lower_tag_node(tag.name(), tag.params(), tag.span())
                            {
                                self.handle_tag(ast_tag);
                            }
                        }
                    }
                }
                Item::LabelDef(label) => {
                    self.lower_label_def(label);
                }
                Item::TextLine(line) => {
                    // When `[iscript]` or `[macro]` appears as the sole content
                    // of a text line, the parser emits a TEXT_LINE sibling
                    // followed by an ISCRIPT_BLOCK / MACRO_DEF sibling.
                    // Detect that pattern here before delegating.
                    let parts: Vec<cst::TextPart> = line.parts().collect();
                    if parts.len() == 1
                        && let cst::TextPart::InlineTag(ref tag) = parts[0]
                    {
                        match tag.name().as_deref() {
                            Some("iscript") => {
                                if let Some(Item::IscriptBlock(block)) = items.get(i + 1) {
                                    self.lower_iscript_block(block, tag.span());
                                    i += 2;
                                    continue;
                                } else {
                                    self.emit(Op::ScriptBlock {
                                        content: String::new(),
                                        span: tag.span(),
                                    });
                                    i += 1;
                                    continue;
                                }
                            }
                            Some("macro") => {
                                if let Some(Item::MacroDef(def)) = items.get(i + 1) {
                                    self.lower_macro_def_from_inline(tag, def);
                                    i += 2;
                                    continue;
                                }
                            }
                            _ => {}
                        }
                    }
                    self.lower_text_line(line);
                }
                Item::CharaLine(chara) => {
                    self.lower_chara_line(chara);
                }
                Item::IscriptBlock(_) | Item::MacroDef(_) => {
                    // Orphaned blocks (parser already emitted an error).
                }
                Item::LineComment(_) | Item::BlockComment(_) | Item::Error(_) => {
                    // Trivia / error nodes — skip silently.
                }
            }
            i += 1;
        }
    }
}

// ─── Tag lowering ─────────────────────────────────────────────────────────────

impl LowerCtx {
    fn lower_tag_node(
        &mut self,
        name: Option<String>,
        params_iter: impl Iterator<Item = cst::Param>,
        span: SourceSpan,
    ) -> Option<Tag<'static>> {
        let name = name?;
        let params = self.lower_params(params_iter);
        Some(Tag {
            name: Cow::Owned(name),
            params,
            span,
        })
    }

    fn lower_params(&mut self, params: impl Iterator<Item = cst::Param>) -> Vec<Param<'static>> {
        params.filter_map(|p| self.lower_param(p)).collect()
    }

    fn lower_param(&mut self, p: cst::Param) -> Option<Param<'static>> {
        let span = p.span();
        let key = p.key().map(Cow::Owned);
        let value = match p.value()? {
            cst::ParamValue::Literal(lit) => ParamValue::Literal(Cow::Owned(lit.text())),
            cst::ParamValue::Entity(ent) => ParamValue::Entity(Cow::Owned(ent.expr())),
            cst::ParamValue::Macro(mp) => ParamValue::MacroParam {
                key: Cow::Owned(mp.key().unwrap_or_default()),
                default: mp.default().map(Cow::Owned),
            },
            cst::ParamValue::Splat(_) => ParamValue::MacroSplat,
        };
        Some(Param { key, value, span })
    }

    /// Dispatch a tag to the appropriate handler (macro registration, iscript,
    /// or emit as `Op::Tag`).
    ///
    /// Before emitting the op, the tag is validated against its known parameter
    /// requirements.  Any resulting [`SyntaxWarning`]s are appended to the
    /// lowering context's error list so callers receive them alongside the
    /// (possibly partial) [`Script`].
    fn handle_tag(&mut self, tag: Tag<'static>) {
        KnownTag::from_tag(&tag, &mut self.errors);
        self.emit(Op::Tag(tag));
    }
}

// ─── Label ────────────────────────────────────────────────────────────────────

impl LowerCtx {
    fn lower_label_def(&mut self, label: &cst::LabelDef) {
        let Some(name) = label.name() else {
            self.push_error("label definition is missing a name", label.span());
            return;
        };
        let span = label.span();
        let title = label.title().map(Cow::Owned);
        let def = LabelDef {
            name: Cow::Owned(name.clone()),
            title,
            span,
        };
        let idx = self.ops.len();
        if self.label_map.contains_key(name.as_str()) {
            self.push_warning(format!("duplicate label: {name}"), span);
        } else {
            self.label_map.insert(Cow::Owned(name), idx);
            self.emit(Op::Label(def));
        }
    }
}

// ─── Text line ────────────────────────────────────────────────────────────────

impl LowerCtx {
    fn lower_text_line(&mut self, line: &cst::TextLine) {
        let span = line.span();
        let parts: Vec<cst::TextPart> = line.parts().collect();

        if parts.is_empty() {
            return;
        }

        // If the line consists only of inline tags (no literal text), convert
        // each to `Op::Tag` so control-flow tags work correctly.
        let all_inline = parts
            .iter()
            .all(|p| matches!(p, cst::TextPart::InlineTag(_)));
        if all_inline {
            for part in parts {
                if let cst::TextPart::InlineTag(tag) = part {
                    let name = tag.name().unwrap_or_default();
                    match name.as_str() {
                        "endmacro" | "endscript" => {}
                        _ => {
                            if let Some(ast_tag) =
                                self.lower_tag_node(Some(name), tag.params(), tag.span())
                            {
                                self.handle_tag(ast_tag);
                            }
                        }
                    }
                }
            }
            return;
        }

        // Mixed / pure text line.
        let mut ast_parts: Vec<TextPart<'static>> = Vec::new();
        for part in parts {
            match part {
                cst::TextPart::Literal(lit) => {
                    ast_parts.push(TextPart::Literal {
                        text: Cow::Owned(lit.text()),
                        span: lit.span(),
                    });
                }
                cst::TextPart::Entity(ent) => {
                    ast_parts.push(TextPart::Entity {
                        expr: Cow::Owned(ent.expr()),
                        span: ent.span(),
                    });
                }
                cst::TextPart::InlineTag(tag) => {
                    if let Some(ast_tag) = self.lower_tag_node(tag.name(), tag.params(), tag.span())
                    {
                        ast_parts.push(TextPart::InlineTag(ast_tag));
                    }
                }
            }
        }

        if !ast_parts.is_empty() {
            self.emit(Op::Text {
                parts: ast_parts,
                span,
            });
        }
    }
}

// ─── Chara shorthand ─────────────────────────────────────────────────────────

impl LowerCtx {
    fn lower_chara_line(&mut self, chara: &cst::CharaLine) {
        let span = chara.span();
        let name = chara.name().unwrap_or_default();
        let face = chara.face();

        let mut params = vec![Param::synthetic("name", name)];
        if let Some(f) = face {
            params.push(Param::synthetic("face", f));
        }
        self.emit(Op::Tag(Tag {
            name: Cow::Borrowed("chara_ptext"),
            params,
            span,
        }));
    }
}

// ─── iscript block ────────────────────────────────────────────────────────────

impl LowerCtx {
    fn lower_iscript_block(&mut self, block: &cst::IscriptBlock, _iscript_tag_span: SourceSpan) {
        let span = block.span();
        let content = block.content();
        self.emit(Op::ScriptBlock { content, span });
    }
}

// ─── macro definition ─────────────────────────────────────────────────────────

impl LowerCtx {
    fn lower_macro_def(&mut self, tag: &cst::AtTag, def: &cst::MacroDef) {
        let name = tag
            .params()
            .find_map(|p| {
                if p.key().as_deref() == Some("name")
                    && let Some(cst::ParamValue::Literal(lit)) = p.value()
                {
                    return Some(lit.text());
                }
                None
            })
            .unwrap_or_default();
        self.register_macro_def(name, def);
    }

    fn lower_macro_def_from_inline(&mut self, tag: &cst::InlineTag, def: &cst::MacroDef) {
        let name = tag
            .params()
            .find_map(|p| {
                if p.key().as_deref() == Some("name")
                    && let Some(cst::ParamValue::Literal(lit)) = p.value()
                {
                    return Some(lit.text());
                }
                None
            })
            .unwrap_or_default();
        self.register_macro_def(name, def);
    }

    fn register_macro_def(&mut self, name: String, def: &cst::MacroDef) {
        if name.is_empty() {
            self.push_error("macro definition is missing a `name` parameter", def.span());
            return;
        }
        let name_cow: Cow<'static, str> = Cow::Owned(name.clone());

        if self.macro_map.contains_key(&name_cow) {
            self.push_warning(format!("duplicate macro: {name}"), def.span());
        }

        // Emit the macro header op.  skip_to is not yet known; we backpatch it
        // below once the body and [endmacro] have been emitted.
        let header_idx = self.ops.len();
        self.emit(Op::MacroDef {
            name: name_cow.clone(),
            skip_to: 0, // placeholder — backpatched after [endmacro]
            span: def.span(),
        });

        let body_start = self.ops.len();

        self.macro_stack.push(name_cow.clone());
        self.macro_map
            .insert(name_cow.clone(), MacroDef { body_start });

        // Lower the body.
        self.lower_items(def.items());

        self.macro_stack.pop();

        self.emit(Op::Tag(Tag {
            name: Cow::Borrowed("endmacro"),
            params: vec![],
            span: def.span(),
        }));

        // skip_to is the op immediately after [endmacro].
        let skip_to = self.ops.len();
        if let Op::MacroDef {
            skip_to: ref mut slot,
            ..
        } = self.ops[header_idx]
        {
            *slot = skip_to;
        }
    }
}
