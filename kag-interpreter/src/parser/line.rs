//! Line-level KAG parser.
//!
//! Processes a flat `&[Spanned<Token>]` stream line by line, classifying
//! each line and delegating tag / parameter parsing to `super::tag`.
//!
//! The `ParseCtx` struct accumulates parsed ops into a `Script` and tracks
//! parser state (block-comment mode, iscript mode, macro nesting, etc.).

use std::borrow::Cow;
use std::collections::HashMap;

use miette::{NamedSource, SourceSpan};

use crate::ast::{LabelDef, MacroDef, Op, Param, Script, Tag, TextPart};
use crate::error::KagError;
use crate::lexer::{Spanned, Token};

use super::tag::parse_tag_from_tokens;

// ─── Parse context ────────────────────────────────────────────────────────────

/// Mutable state accumulated while parsing a whole scenario file.
pub struct ParseCtx<'src> {
    source: &'src str,
    source_name: String,
    ops: Vec<Op<'src>>,
    label_map: HashMap<Cow<'src, str>, usize>,
    macro_map: HashMap<Cow<'src, str>, MacroDef>,
    /// Stack of macro names being defined (for nesting detection).
    macro_stack: Vec<Cow<'src, str>>,
    /// When `Some`, accumulate raw lines as iscript content.
    iscript_buf: Option<String>,
    /// True when inside a `/* … */` block comment.
    in_block_comment: bool,
    /// Pending speaker name from `#name` shorthand on the previous line.
    pending_speaker: Option<String>,
}

impl<'src> ParseCtx<'src> {
    pub fn new(source: &'src str, source_name: &str) -> Self {
        Self {
            source,
            source_name: source_name.to_owned(),
            ops: Vec::new(),
            label_map: HashMap::new(),
            macro_map: HashMap::new(),
            macro_stack: Vec::new(),
            iscript_buf: None,
            in_block_comment: false,
            pending_speaker: None,
        }
    }

    pub fn into_script(self) -> Script<'src> {
        Script {
            ops: self.ops,
            label_map: self.label_map,
            macro_map: self.macro_map,
            source_name: self.source_name,
        }
    }

    fn named_source(&self) -> NamedSource<String> {
        NamedSource::new(&self.source_name, self.source.to_owned())
    }
}

// ─── Top-level driver ─────────────────────────────────────────────────────────

impl<'src> ParseCtx<'src> {
    /// Process the entire token stream.
    pub fn parse_all(&mut self, tokens: &[Spanned<'src>]) -> Result<(), KagError> {
        let mut pos = 0;
        while pos < tokens.len() {
            self.parse_line(tokens, &mut pos)?;
        }
        Ok(())
    }

    /// Parse one logical line starting at `pos`, advancing `pos` past the
    /// trailing newline (or end-of-input).
    fn parse_line(&mut self, tokens: &[Spanned<'src>], pos: &mut usize) -> Result<(), KagError> {
        // Skip bare newlines (blank lines)
        if matches!(tokens[*pos].token, Token::Newline) {
            *pos += 1;
            return Ok(());
        }

        // ── iscript accumulation mode ────────────────────────────────────────
        if self.iscript_buf.is_some() {
            return self.parse_iscript_line(tokens, pos);
        }

        // ── block comment mode ───────────────────────────────────────────────
        if self.in_block_comment {
            return self.parse_block_comment_body(tokens, pos);
        }

        // Skip leading whitespace (indented lines are text lines)
        while *pos < tokens.len() && matches!(tokens[*pos].token, Token::Whitespace) {
            *pos += 1;
        }
        if *pos >= tokens.len() || matches!(tokens[*pos].token, Token::Newline) {
            if *pos < tokens.len() {
                *pos += 1;
            }
            return Ok(());
        }

        // ── classify first non-whitespace token on the line ──────────────────
        match &tokens[*pos].token {
            Token::LineComment => {
                advance_to_newline(tokens, pos);
                Ok(())
            }
            Token::BlockCommentOpen => {
                self.in_block_comment = true;
                advance_to_newline(tokens, pos);
                Ok(())
            }
            Token::Hash => self.parse_chara_shorthand(tokens, pos),
            Token::Star => self.parse_label_def(tokens, pos),
            Token::At => self.parse_at_tag(tokens, pos),
            _ => self.parse_text_line(tokens, pos),
        }
    }

    // ── Block comment ─────────────────────────────────────────────────────────

    fn parse_block_comment_body(
        &mut self,
        tokens: &[Spanned<'src>],
        pos: &mut usize,
    ) -> Result<(), KagError> {
        if matches!(tokens[*pos].token, Token::BlockCommentClose) {
            self.in_block_comment = false;
        }
        advance_to_newline(tokens, pos);
        Ok(())
    }

    // ── iscript accumulation ──────────────────────────────────────────────────

    fn parse_iscript_line(
        &mut self,
        tokens: &[Spanned<'src>],
        pos: &mut usize,
    ) -> Result<(), KagError> {
        // Check if this line starts with `@endscript` or `[endscript]`
        let start = *pos;
        let is_endscript = is_tag_line_named(tokens, *pos, "endscript");

        if is_endscript {
            // Finalise the script block
            let script_text = self.iscript_buf.take().unwrap_or_default();
            self.emit(Op::ScriptBlock(script_text));
            advance_to_newline(tokens, pos);
        } else {
            // Accumulate raw source text for this line
            let buf = self.iscript_buf.as_mut().unwrap();
            let line_text = collect_line_source(tokens, *pos);
            buf.push_str(&line_text);
            buf.push('\n');
            advance_to_newline(tokens, pos);
        }
        let _ = start;
        Ok(())
    }

    // ── Character name shorthand (#name or #name:face) ────────────────────────

    fn parse_chara_shorthand(
        &mut self,
        tokens: &[Spanned<'src>],
        pos: &mut usize,
    ) -> Result<(), KagError> {
        *pos += 1; // consume `#`

        let name = expect_ident_at(tokens, pos, self)?;
        let face = if *pos < tokens.len() && matches!(tokens[*pos].token, Token::Colon) {
            *pos += 1;
            Some(expect_ident_at(tokens, pos, self)?)
        } else {
            None
        };

        self.pending_speaker = Some(name.to_string());

        // Emit as generic `chara_ptext` tag (matches JS reference)
        let mut params = vec![Param::literal("name", name)];
        if let Some(f) = face {
            params.push(Param::literal("face", f));
        }
        let span = tok_span(tokens, 0, *pos);
        self.emit(Op::Tag(Tag {
            name: Cow::Borrowed("chara_ptext"),
            params,
            span,
        }));

        advance_to_newline(tokens, pos);
        Ok(())
    }

    // ── Label definition (*name or *name|title) ───────────────────────────────

    fn parse_label_def(
        &mut self,
        tokens: &[Spanned<'src>],
        pos: &mut usize,
    ) -> Result<(), KagError> {
        let line_start = *pos;
        *pos += 1; // consume `*`

        let name = expect_ident_at(tokens, pos, self)?;
        let title = if *pos < tokens.len() && matches!(tokens[*pos].token, Token::Pipe) {
            *pos += 1;
            // Title: collect everything up to newline
            let title_str = collect_line_source(tokens, *pos);
            advance_to_newline(tokens, pos);
            Some(Cow::Owned(title_str))
        } else {
            advance_to_newline(tokens, pos);
            None
        };

        let idx = self.ops.len();
        let span = tok_span(tokens, line_start, *pos);
        let label_def = LabelDef { name: Cow::Borrowed(name), title, span };

        // Duplicate label detection
        if self.label_map.contains_key(label_def.name.as_ref()) {
            // Not a hard error — emit a warning tag
            self.emit(Op::Tag(Tag {
                name: Cow::Borrowed("_warning"),
                params: vec![Param::literal(
                    "msg",
                    format!("duplicate label: {}", label_def.name),
                )],
                span: label_def.span,
            }));
        } else {
            self.label_map.insert(Cow::Borrowed(name), idx);
            self.emit(Op::Label(label_def));
        }

        Ok(())
    }

    // ── Line-level tag (@tagname params…) ────────────────────────────────────

    fn parse_at_tag(
        &mut self,
        tokens: &[Spanned<'src>],
        pos: &mut usize,
    ) -> Result<(), KagError> {
        let line_start = *pos;
        *pos += 1; // consume `@`

        let line_end = find_newline(tokens, *pos);
        let tag_tokens = &tokens[*pos..line_end];

        let span = tok_span(tokens, line_start, line_end);
        let tag = parse_tag_from_tokens(tag_tokens, span).map_err(|e| {
            KagError::parse(
                format!("expected tag name after @: {e}"),
                self.named_source(),
                span,
            )
        })?;

        *pos = line_end;
        advance_to_newline(tokens, pos);

        self.handle_tag(tag)?;
        Ok(())
    }

    // ── Text line (may contain [inline_tag] fragments) ────────────────────────

    fn parse_text_line(
        &mut self,
        tokens: &[Spanned<'src>],
        pos: &mut usize,
    ) -> Result<(), KagError> {
        let mut parts: Vec<TextPart<'src>> = Vec::new();

        while *pos < tokens.len() && !matches!(tokens[*pos].token, Token::Newline) {
            match &tokens[*pos].token {
                Token::LBracket => {
                    let inline_tag = self.parse_inline_tag(tokens, pos)?;
                    parts.push(TextPart::InlineTag(inline_tag));
                }
                Token::Backslash => {
                    // Escape: the next character is literal
                    *pos += 1;
                    if *pos < tokens.len() && !matches!(tokens[*pos].token, Token::Newline) {
                        let slice = tokens[*pos].slice;
                        parts.push(TextPart::Literal(Cow::Borrowed(slice)));
                        *pos += 1;
                    }
                }
                Token::Amp => {
                    // Inline entity `&expr` in text
                    *pos += 1; // consume `&`
                    let mut expr = String::new();
                    while *pos < tokens.len()
                        && !matches!(tokens[*pos].token, Token::Newline | Token::LBracket)
                    {
                        expr.push_str(tokens[*pos].slice);
                        *pos += 1;
                    }
                    parts.push(TextPart::Entity(Cow::Owned(expr)));
                }
                _ => {
                    // Accumulate adjacent literal tokens (including Whitespace) into a single part
                    let slice = tokens[*pos].slice;
                    if let Some(TextPart::Literal(existing)) = parts.last_mut() {
                        *existing = Cow::Owned(format!("{}{}", existing, slice));
                    } else {
                        parts.push(TextPart::Literal(Cow::Borrowed(slice)));
                    }
                    *pos += 1;
                }
            }
        }

        // Consume trailing newline
        if *pos < tokens.len() && matches!(tokens[*pos].token, Token::Newline) {
            *pos += 1;
        }

        if parts.is_empty() {
            return Ok(());
        }

        // If the line consists ONLY of inline tags (no literal text), convert each
        // to Op::Tag so control-flow and block-level tags (if, eval, macro, …) are
        // handled correctly.
        let all_inline = parts
            .iter()
            .all(|p| matches!(p, TextPart::InlineTag(_)));

        if all_inline {
            let tags: Vec<Tag<'src>> = parts
                .drain(..)
                .filter_map(|p| {
                    if let TextPart::InlineTag(t) = p {
                        Some(t)
                    } else {
                        None
                    }
                })
                .collect();
            for tag in tags {
                self.handle_tag(tag)?;
            }
            return Ok(());
        }

        // Mixed line (literal text + maybe inline tags)
        let speaker = self.pending_speaker.take();
        if let Some(spk) = speaker {
            self.emit(Op::Tag(Tag {
                name: Cow::Borrowed("chara_ptext"),
                params: vec![Param::literal("name", spk)],
                span: (0usize, 0usize).into(),
            }));
        }
        self.emit(Op::Text(parts));

        Ok(())
    }

    // ── Inline [tag] parser ───────────────────────────────────────────────────

    fn parse_inline_tag(
        &mut self,
        tokens: &[Spanned<'src>],
        pos: &mut usize,
    ) -> Result<Tag<'src>, KagError> {
        let bracket_start = *pos;
        *pos += 1; // consume `[`

        // Collect tokens inside the brackets, respecting nesting
        let mut depth = 1usize;
        let inner_start = *pos;
        while *pos < tokens.len() {
            match &tokens[*pos].token {
                Token::LBracket => {
                    depth += 1;
                    *pos += 1;
                }
                Token::RBracket => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    *pos += 1;
                }
                Token::Newline => break,
                _ => {
                    *pos += 1;
                }
            }
        }

        let inner_tokens = &tokens[inner_start..*pos];

        // Consume closing `]`
        if *pos < tokens.len() && matches!(tokens[*pos].token, Token::RBracket) {
            *pos += 1;
        }

        let span = tok_span(tokens, bracket_start, *pos);
        parse_tag_from_tokens(inner_tokens, span).map_err(|e| {
            KagError::parse(
                format!("malformed inline tag: {e}"),
                self.named_source(),
                span,
            )
        })
    }

    // ── Tag dispatch ──────────────────────────────────────────────────────────

    /// Handle a parsed tag, performing any parse-time bookkeeping
    /// (macro registration, iscript mode, etc.) and then emitting the op.
    fn handle_tag(&mut self, tag: Tag<'src>) -> Result<(), KagError> {
        match tag.name.as_ref() {
            "iscript" => {
                self.iscript_buf = Some(String::new());
                // Don't emit the iscript tag itself; emit ScriptBlock at endscript
            }
            "endscript" => {
                // Handled inside parse_iscript_line; if we reach here it's stray
                let script_text = self.iscript_buf.take().unwrap_or_default();
                self.emit(Op::ScriptBlock(script_text));
            }
            "macro" => {
                if let Some(name_val) = tag.param_str("name") {
                    let name: Cow<'src, str> = Cow::Owned(name_val.to_owned());
                    // Record body start (ops after this point)
                    let body_start = self.ops.len();
                    self.macro_stack.push(name.clone());
                    // Store a placeholder; finalised in `endmacro`
                    self.macro_map.insert(
                        name,
                        MacroDef {
                            body_start,
                            body_end: body_start,
                        },
                    );
                }
            }
            "endmacro" => {
                if let Some(name) = self.macro_stack.pop()
                    && let Some(def) = self.macro_map.get_mut(&name) {
                        def.body_end = self.ops.len();
                    }
            }
            _ => {
                self.emit(Op::Tag(tag));
            }
        }
        Ok(())
    }

    fn emit(&mut self, op: Op<'src>) {
        self.ops.push(op);
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Advance `pos` to the next `Newline` token (exclusive) or end-of-tokens.
fn find_newline(tokens: &[Spanned<'_>], mut pos: usize) -> usize {
    while pos < tokens.len() && !matches!(tokens[pos].token, Token::Newline) {
        pos += 1;
    }
    pos
}

/// Advance `pos` past the next `Newline` token (consume it).
fn advance_to_newline(tokens: &[Spanned<'_>], pos: &mut usize) {
    while *pos < tokens.len() && !matches!(tokens[*pos].token, Token::Newline) {
        *pos += 1;
    }
    if *pos < tokens.len() {
        *pos += 1; // consume the newline
    }
}

/// Collect the raw source slices for all tokens from `pos` to the next newline.
fn collect_line_source<'src>(tokens: &[Spanned<'src>], pos: usize) -> String {
    let end = find_newline(tokens, pos);
    tokens[pos..end].iter().map(|s| s.slice).collect()
}

/// Return a `SourceSpan` covering `tokens[start..end]`.
fn tok_span(tokens: &[Spanned<'_>], start: usize, end: usize) -> SourceSpan {
    if start >= tokens.len() {
        return (0usize, 0usize).into();
    }
    let from = tokens[start].span.offset();
    let to = if end > 0 && end <= tokens.len() {
        let last = &tokens[end.min(tokens.len()) - 1];
        last.span.offset() + last.span.len()
    } else {
        from
    };
    (from, to - from).into()
}

/// True if the tokens starting at `pos` represent either `@name` or `[name]`
/// (i.e. a tag whose name matches `expected`).
fn is_tag_line_named(tokens: &[Spanned<'_>], pos: usize, expected: &str) -> bool {
    if pos >= tokens.len() {
        return false;
    }
    match &tokens[pos].token {
        Token::At => {
            tokens
                .get(pos + 1)
                .is_some_and(|t| t.slice == expected)
        }
        Token::LBracket => {
            tokens
                .get(pos + 1)
                .is_some_and(|t| t.slice == expected)
        }
        _ => false,
    }
}

/// Expect an `Ident` token at `pos`, return its slice, and advance `pos`.
fn expect_ident_at<'src>(
    tokens: &[Spanned<'src>],
    pos: &mut usize,
    ctx: &ParseCtx<'src>,
) -> Result<&'src str, KagError> {
    if *pos >= tokens.len() {
        return Err(KagError::parse(
            "expected identifier",
            ctx.named_source(),
            (0usize, 0usize).into(),
        ));
    }
    match &tokens[*pos].token {
        Token::Ident(s) => {
            let s = *s;
            *pos += 1;
            Ok(s)
        }
        _ => Err(KagError::parse(
            format!("expected identifier, got {:?}", tokens[*pos].token),
            ctx.named_source(),
            tokens[*pos].span,
        )),
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_script;

    #[test]
    fn test_label_parsing() {
        let script = parse_script("*start|Opening scene\n", "test.ks").unwrap();
        assert!(
            script.label_map.contains_key("start"),
            "label_map: {:?}",
            script.label_map.keys().collect::<Vec<_>>()
        );
        match &script.ops[0] {
            Op::Label(def) => {
                assert_eq!(def.name.as_ref(), "start");
                assert!(def.title.is_some());
            }
            other => panic!("expected Label op, got {:?}", other),
        }
    }

    #[test]
    fn test_line_comment_skipped() {
        let script = parse_script("; this is a comment\n", "test.ks").unwrap();
        assert!(script.ops.is_empty(), "ops: {:?}", script.ops);
    }

    #[test]
    fn test_block_comment_skipped() {
        let script = parse_script("/*\nsome content\n*/\n", "test.ks").unwrap();
        assert!(script.ops.is_empty());
    }

    #[test]
    fn test_at_tag_parsed() {
        let script = parse_script("@r\n", "test.ks").unwrap();
        assert_eq!(script.ops.len(), 1);
        match &script.ops[0] {
            Op::Tag(t) => assert_eq!(t.name.as_ref(), "r"),
            other => panic!("expected Tag, got {:?}", other),
        }
    }

    #[test]
    fn test_at_tag_with_params() {
        let script = parse_script("@jump storage=main target=*start\n", "test.ks").unwrap();
        match &script.ops[0] {
            Op::Tag(t) => {
                assert_eq!(t.name.as_ref(), "jump");
                assert_eq!(t.param_str("storage"), Some("main"));
            }
            other => panic!("expected Tag, got {:?}", other),
        }
    }

    #[test]
    fn test_text_line() {
        let script = parse_script("Hello, world!\n", "test.ks").unwrap();
        assert!(!script.ops.is_empty());
        assert!(matches!(script.ops[0], Op::Text(_)));
    }

    #[test]
    fn test_inline_tag_in_text() {
        let script = parse_script("Hello[r]World\n", "test.ks").unwrap();
        match &script.ops[0] {
            Op::Text(parts) => {
                let has_inline = parts
                    .iter()
                    .any(|p| matches!(p, TextPart::InlineTag(t) if t.name == "r"));
                assert!(has_inline, "parts: {:?}", parts);
            }
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_chara_shorthand() {
        let script = parse_script("#Alice:happy\n", "test.ks").unwrap();
        match &script.ops[0] {
            Op::Tag(t) => {
                assert_eq!(t.name.as_ref(), "chara_ptext");
                assert_eq!(t.param_str("name"), Some("Alice"));
                assert_eq!(t.param_str("face"), Some("happy"));
            }
            other => panic!("expected chara_ptext Tag, got {:?}", other),
        }
    }

    #[test]
    fn test_iscript_block() {
        let src = "[iscript]\nlet x = 42;\n[endscript]\n";
        let script = parse_script(src, "test.ks").unwrap();
        let has_script_block = script
            .ops
            .iter()
            .any(|op| matches!(op, Op::ScriptBlock(s) if s.contains("let x = 42")));
        assert!(has_script_block, "ops: {:?}", script.ops);
    }

    #[test]
    fn test_macro_registration() {
        let src = "[macro name=mymacro]\n@r\n[endmacro]\n";
        let script = parse_script(src, "test.ks").unwrap();
        assert!(
            script.macro_map.contains_key("mymacro"),
            "macro_map: {:?}",
            script.macro_map.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_multiple_lines() {
        let src = "*chapter1\nHello!\n@l\n";
        let script = parse_script(src, "test.ks").unwrap();
        assert_eq!(script.ops.len(), 3);
    }
}
