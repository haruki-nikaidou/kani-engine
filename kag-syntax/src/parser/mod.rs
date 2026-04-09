//! KAG CST parser — turns a flat `Vec<Spanned<Token>>` into a Rowan green
//! tree with full spans and error recovery.
//!
//! Public entry points:
//! - [`parse_cst`] — returns the lossless [`Parse`] result (tree + diagnostics).
//! - [`parse_script`] — convenience wrapper that lowers the CST to the
//!   semantic [`Script`] AST and returns it together with any diagnostics.

pub mod line;
pub mod tag;

use std::marker::PhantomData;

use miette::SourceSpan;
use rowan::{GreenNode, GreenNodeBuilder};

use crate::ast::Script;
use crate::cst::{AstNode, Root};
use crate::error::ParseDiagnostic;
use crate::lexer::{Spanned, Token, tokenize};
use crate::lower::lower_root;
use crate::syntax_kind::{KagLanguage, SyntaxKind, SyntaxNode};
use rowan::Language as _;

// ─── Parse result ─────────────────────────────────────────────────────────────

/// The result of parsing a KAG source file.
///
/// Contains the lossless Rowan green tree and any diagnostics collected
/// during error recovery.  Use [`Parse::tree`] to get the typed root node,
/// or [`Parse::syntax_node`] for the raw [`SyntaxNode`].
pub struct Parse<T> {
    green: GreenNode,
    pub errors: Vec<ParseDiagnostic>,
    _ty: PhantomData<fn() -> T>,
}

impl<T: AstNode> Parse<T> {
    fn new(green: GreenNode, errors: Vec<ParseDiagnostic>) -> Self {
        Self {
            green,
            errors,
            _ty: PhantomData,
        }
    }

    /// The raw Rowan syntax node for the root of the tree.
    pub fn syntax_node(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    /// The typed CST root node.
    #[allow(clippy::expect_used)] // invariant: the builder always produces a castable root
    pub fn tree(&self) -> T {
        T::cast(self.syntax_node()).expect("root node should cast")
    }

    /// Diagnostics collected during parsing (may be empty on clean input).
    pub fn errors(&self) -> &[ParseDiagnostic] {
        &self.errors
    }
}

// ─── Parser ───────────────────────────────────────────────────────────────────

/// Internal recursive-descent parser that drives a Rowan `GreenNodeBuilder`.
pub(crate) struct Parser<'src> {
    /// Flat token list from `logos`.
    pub(crate) tokens: Vec<Spanned<'src>>,
    /// Current read position (index into `tokens`).
    pub(crate) pos: usize,
    /// Rowan green tree builder.
    pub(crate) builder: GreenNodeBuilder<'static>,
    /// Non-fatal diagnostics collected so far.
    pub(crate) errors: Vec<ParseDiagnostic>,
    /// Original source text (for span computations and error messages).
    pub(crate) source: &'src str,
    /// Set by the tag dispatcher when an `iscript` tag is parsed; the line
    /// parser then emits an `ISCRIPT_BLOCK` sibling immediately after.
    pub(crate) pending_iscript: bool,
    /// Set by the tag dispatcher when a `macro` tag is parsed; the line
    /// parser then emits a `MACRO_DEF` sibling immediately after.
    pub(crate) pending_macro: bool,
}

#[allow(dead_code)] // parser helper API — not all methods are exercised yet
impl<'src> Parser<'src> {
    pub(crate) fn new(source: &'src str, tokens: Vec<Spanned<'src>>) -> Self {
        Self {
            tokens,
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            source,
            pending_iscript: false,
            pending_macro: false,
        }
    }

    // ── Token inspection ────────────────────────────────────────────────────

    /// `true` when all tokens have been consumed.
    pub(crate) fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// The `SyntaxKind` of the current token, or `ERROR` at end-of-input.
    pub(crate) fn current(&self) -> SyntaxKind {
        self.tokens
            .get(self.pos)
            .map(|s| token_to_kind(&s.token))
            .unwrap_or(SyntaxKind::ERROR)
    }

    /// `true` if the current token has the given kind.
    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    /// The raw source slice of the current token, or `""` at end-of-input.
    pub(crate) fn current_slice(&self) -> &'src str {
        self.tokens.get(self.pos).map(|s| s.slice).unwrap_or("")
    }

    /// The `SourceSpan` of the current token, or a zero-width span.
    pub(crate) fn current_span(&self) -> SourceSpan {
        self.tokens
            .get(self.pos)
            .map(|s| s.span)
            .unwrap_or_else(|| (self.source.len(), 0usize).into())
    }

    /// The `SourceSpan` of the token at `offset` relative to `self.pos`,
    /// or the current span if out of range.
    pub(crate) fn span_at(&self, offset: usize) -> SourceSpan {
        self.tokens
            .get(self.pos + offset)
            .map(|s| s.span)
            .unwrap_or_else(|| self.current_span())
    }

    // ── Builder helpers ─────────────────────────────────────────────────────

    /// Open a new composite node of `kind`.  Must be matched by
    /// [`finish_node`](Self::finish_node).
    pub(crate) fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(KagLanguage::kind_to_raw(kind));
    }

    /// Close the most recently opened composite node.
    pub(crate) fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    // ── Token consumption ───────────────────────────────────────────────────

    /// Consume the current token and add it to the builder with its natural
    /// `SyntaxKind`.  Panics if already at end-of-input.
    pub(crate) fn bump(&mut self) {
        let tok = &self.tokens[self.pos];
        let kind = token_to_kind(&tok.token);
        self.builder
            .token(KagLanguage::kind_to_raw(kind), tok.slice);
        self.pos += 1;
    }

    /// Consume the current token but record it under `kind` instead of its
    /// natural kind.  Used to reinterpret tokens during error recovery.
    pub(crate) fn bump_as(&mut self, kind: SyntaxKind) {
        let slice = self.tokens[self.pos].slice;
        self.builder.token(KagLanguage::kind_to_raw(kind), slice);
        self.pos += 1;
    }

    /// Bump the current token if it matches `kind`; return whether it was consumed.
    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Skip any `WHITESPACE` tokens (adding them to the builder).
    pub(crate) fn skip_ws(&mut self) {
        while self.at(SyntaxKind::WHITESPACE) {
            self.bump();
        }
    }

    // ── Error recovery ──────────────────────────────────────────────────────

    /// Record a `ParseDiagnostic` at the current position.
    pub(crate) fn push_error(&mut self, message: impl Into<String>) {
        self.errors
            .push(ParseDiagnostic::error(message, self.current_span()));
    }

    /// Record a `ParseDiagnostic` at an explicit span.
    pub(crate) fn push_error_at(&mut self, message: impl Into<String>, span: SourceSpan) {
        self.errors.push(ParseDiagnostic::error(message, span));
    }

    /// Record a warning diagnostic at an explicit span.
    pub(crate) fn push_warning_at(&mut self, message: impl Into<String>, span: SourceSpan) {
        self.errors.push(ParseDiagnostic::warning(message, span));
    }

    /// Emit an ERROR node wrapping every token until (but not including) a
    /// `NEWLINE` or end-of-input, then return.
    ///
    /// Used when a line cannot be parsed correctly but we want to continue
    /// with the next line.
    pub(crate) fn error_recover_to_newline(&mut self, message: impl Into<String>) {
        self.push_error(message);
        self.start_node(SyntaxKind::ERROR);
        while !self.at_end() && !self.at(SyntaxKind::NEWLINE) {
            self.bump();
        }
        self.finish_node();
    }

    /// Emit an ERROR node wrapping tokens until (but not including) the first
    /// token in `until` or a `NEWLINE`.
    pub(crate) fn error_recover_until(&mut self, message: impl Into<String>, until: &[SyntaxKind]) {
        self.push_error(message);
        self.start_node(SyntaxKind::ERROR);
        while !self.at_end() && !self.at(SyntaxKind::NEWLINE) && !until.contains(&self.current()) {
            self.bump();
        }
        self.finish_node();
    }

    // ── Span helpers ────────────────────────────────────────────────────────

    /// Byte offset of the token at `pos` in the source.
    pub(crate) fn offset_at(&self, pos: usize) -> usize {
        self.tokens
            .get(pos)
            .map(|s| s.span.offset())
            .unwrap_or(self.source.len())
    }

    /// Build a `SourceSpan` from `start_pos..self.pos` in the token stream.
    pub(crate) fn span_from(&self, start_pos: usize) -> SourceSpan {
        let start = self.offset_at(start_pos);
        let end = if self.pos > 0 {
            let last = &self.tokens[self.pos.min(self.tokens.len()) - 1];
            last.span.offset() + last.span.len()
        } else {
            start
        };
        (start, end.saturating_sub(start)).into()
    }
}

// ─── Token → SyntaxKind ───────────────────────────────────────────────────────

/// Convert a `logos` `Token` to the corresponding `SyntaxKind` leaf tag.
pub(crate) fn token_to_kind(tok: &Token<'_>) -> SyntaxKind {
    match tok {
        Token::Newline => SyntaxKind::NEWLINE,
        Token::LineComment => SyntaxKind::LINE_COMMENT,
        Token::BlockCommentOpen => SyntaxKind::BLOCK_COMMENT_OPEN,
        Token::BlockCommentClose => SyntaxKind::BLOCK_COMMENT_CLOSE,
        Token::At => SyntaxKind::AT,
        Token::Hash => SyntaxKind::HASH,
        Token::Star => SyntaxKind::STAR,
        Token::LBracket => SyntaxKind::L_BRACKET,
        Token::RBracket => SyntaxKind::R_BRACKET,
        Token::Eq => SyntaxKind::EQ,
        Token::Amp => SyntaxKind::AMP,
        Token::Percent => SyntaxKind::PERCENT,
        Token::Pipe => SyntaxKind::PIPE,
        Token::Colon => SyntaxKind::COLON,
        Token::DoubleQuoted(_) => SyntaxKind::DOUBLE_QUOTED,
        Token::SingleQuoted(_) => SyntaxKind::SINGLE_QUOTED,
        Token::Ident(_) => SyntaxKind::IDENT,
        Token::Number(_) => SyntaxKind::NUMBER,
        Token::Backslash => SyntaxKind::BACKSLASH,
        Token::Text(_) => SyntaxKind::TEXT,
        Token::Whitespace => SyntaxKind::WHITESPACE,
        Token::Slash => SyntaxKind::SLASH,
        Token::Lt => SyntaxKind::LT,
        Token::Gt => SyntaxKind::GT,
    }
}

// ─── Public entry points ──────────────────────────────────────────────────────

/// Parse a KAG `.ks` source string into a lossless Rowan CST.
///
/// Lex errors are stored as `ERROR` tokens in the tree and as entries in
/// [`Parse::errors`].  The parser always returns a complete tree regardless
/// of the number of errors.
pub fn parse_cst(source: &str, _source_name: &str) -> Parse<Root> {
    let (tokens, lex_errors) = tokenize(source);

    let mut parser = Parser::new(source, tokens);

    // Surface lex errors as diagnostics (don't abort).
    for e in &lex_errors {
        parser.errors.push(ParseDiagnostic::error(
            "unexpected character",
            (e.start, e.len()).into(),
        ));
    }

    line::parse_root(&mut parser);

    let green = parser.builder.finish();
    Parse::new(green, parser.errors)
}

/// Parse a KAG `.ks` source string into the semantic [`Script`] AST.
///
/// Internally calls [`parse_cst`] and lowers the result.
/// Returns the script together with any diagnostics; a non-empty diagnostics
/// `Vec` does **not** mean the script is unusable — the interpreter will
/// still see a best-effort op stream.
pub fn parse_script(source: &str, source_name: &str) -> (Script<'static>, Vec<ParseDiagnostic>) {
    let parse = parse_cst(source, source_name);
    let root = parse.tree();
    let mut errors = parse.errors;
    let (script, lower_errors) = lower_root(root, source_name);
    errors.extend(lower_errors);
    (script, errors)
}

// ─── Stream helpers (kept for compatibility with sub-modules) ─────────────────

/// The winnow input type alias — kept so `tag.rs` can reference it if needed.
pub type Input<'src, 'toks> = &'toks [Spanned<'src>];
