//! Typed CST node wrappers built on top of the Rowan [`SyntaxNode`].
//!
//! Each struct is a thin newtype around a [`SyntaxNode`] or [`SyntaxToken`]
//! and provides accessor methods that navigate to the relevant child
//! nodes / tokens.  The pattern mirrors how `rust-analyzer` exposes its
//! typed AST: every accessor returns an `Option` (the tree may have
//! error-recovered nodes that are missing children).

use crate::syntax_kind::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

// ─── AstNode trait ────────────────────────────────────────────────────────────

/// Marker trait for typed CST wrappers.
pub trait AstNode: Sized {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(node: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn child_node<N: AstNode>(parent: &SyntaxNode) -> Option<N> {
    parent.children().find_map(N::cast)
}

fn child_nodes<'a, N: AstNode + 'a>(parent: &'a SyntaxNode) -> impl Iterator<Item = N> + 'a {
    parent.children().filter_map(N::cast)
}

fn token_of_kind(parent: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    parent
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|t| t.kind() == kind)
}

fn text_of_token(parent: &SyntaxNode, kind: SyntaxKind) -> Option<String> {
    token_of_kind(parent, kind).map(|t| t.text().to_owned())
}

/// Build a [`miette::SourceSpan`] from a [`SyntaxNode`]'s text range.
pub fn node_span(node: &SyntaxNode) -> miette::SourceSpan {
    let range = node.text_range();
    (usize::from(range.start()), usize::from(range.len())).into()
}

/// Build a [`miette::SourceSpan`] from a [`SyntaxToken`]'s text range.
pub fn token_span(token: &SyntaxToken) -> miette::SourceSpan {
    let range = token.text_range();
    (usize::from(range.start()), usize::from(range.len())).into()
}

// ─── Macro for repetitive AstNode impls ──────────────────────────────────────

macro_rules! ast_node {
    ($Name:ident, $Kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $Name(SyntaxNode);

        impl AstNode for $Name {
            fn can_cast(kind: SyntaxKind) -> bool {
                kind == SyntaxKind::$Kind
            }
            fn cast(node: SyntaxNode) -> Option<Self> {
                if node.kind() == SyntaxKind::$Kind {
                    Some(Self(node))
                } else {
                    None
                }
            }
            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

// ─── Root ─────────────────────────────────────────────────────────────────────

ast_node!(Root, ROOT);

impl Root {
    /// Iterate over all top-level items in the file.
    pub fn items(&self) -> impl Iterator<Item = Item> + '_ {
        self.0.children().filter_map(Item::cast_node)
    }
}

// ─── Top-level item (discriminated union) ─────────────────────────────────────

/// Any top-level item that can appear directly inside a [`Root`].
#[derive(Debug, Clone)]
pub enum Item {
    LineComment(LineCommentNode),
    BlockComment(BlockCommentNode),
    LabelDef(LabelDef),
    AtTag(AtTag),
    InlineTag(InlineTag),
    TextLine(TextLine),
    CharaLine(CharaLine),
    IscriptBlock(IscriptBlock),
    MacroDef(MacroDef),
    Error(SyntaxNode),
}

impl Item {
    fn cast_node(node: SyntaxNode) -> Option<Self> {
        Some(match node.kind() {
            SyntaxKind::LINE_COMMENT_NODE => Self::LineComment(LineCommentNode(node)),
            SyntaxKind::BLOCK_COMMENT_NODE => Self::BlockComment(BlockCommentNode(node)),
            SyntaxKind::LABEL_DEF => Self::LabelDef(LabelDef(node)),
            SyntaxKind::AT_TAG => Self::AtTag(AtTag(node)),
            SyntaxKind::INLINE_TAG => Self::InlineTag(InlineTag(node)),
            SyntaxKind::TEXT_LINE => Self::TextLine(TextLine(node)),
            SyntaxKind::CHARA_LINE => Self::CharaLine(CharaLine(node)),
            SyntaxKind::ISCRIPT_BLOCK => Self::IscriptBlock(IscriptBlock(node)),
            SyntaxKind::MACRO_DEF => Self::MacroDef(MacroDef(node)),
            SyntaxKind::ERROR => Self::Error(node),
            _ => return None,
        })
    }
}

// ─── Line comment ─────────────────────────────────────────────────────────────

ast_node!(LineCommentNode, LINE_COMMENT_NODE);

impl LineCommentNode {
    /// The raw comment text including the `;` or `//` prefix.
    pub fn text(&self) -> Option<String> {
        token_of_kind(&self.0, SyntaxKind::LINE_COMMENT)
            .map(|t| t.text().to_owned())
    }
}

// ─── Block comment ────────────────────────────────────────────────────────────

ast_node!(BlockCommentNode, BLOCK_COMMENT_NODE);

impl BlockCommentNode {
    /// All tokens inside this block comment concatenated as a string.
    pub fn raw_text(&self) -> String {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|t| {
                !matches!(
                    t.kind(),
                    SyntaxKind::BLOCK_COMMENT_OPEN | SyntaxKind::BLOCK_COMMENT_CLOSE
                )
            })
            .map(|t| t.text().to_owned())
            .collect()
    }
}

// ─── Label definition ─────────────────────────────────────────────────────────

ast_node!(LabelDef, LABEL_DEF);

impl LabelDef {
    /// The label name (without the `*` sigil).
    pub fn name_token(&self) -> Option<SyntaxToken> {
        // The IDENT immediately following the STAR token.
        let mut found_star = false;
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| {
                if t.kind() == SyntaxKind::STAR {
                    found_star = true;
                    false
                } else {
                    found_star && t.kind() == SyntaxKind::IDENT
                }
            })
    }

    /// The label name as a string.
    pub fn name(&self) -> Option<String> {
        self.name_token().map(|t| t.text().to_owned())
    }

    /// The display title (the part after `|`), if present.
    pub fn title(&self) -> Option<String> {
        let mut found_pipe = false;
        let parts: Vec<String> = self
            .0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|t| {
                if t.kind() == SyntaxKind::PIPE {
                    found_pipe = true;
                    false
                } else if found_pipe && !matches!(t.kind(), SyntaxKind::NEWLINE) {
                    true
                } else {
                    false
                }
            })
            .map(|t| t.text().to_owned())
            .collect();
        if parts.is_empty() { None } else { Some(parts.concat()) }
    }

    pub fn span(&self) -> miette::SourceSpan {
        node_span(&self.0)
    }
}

// ─── AT tag ──────────────────────────────────────────────────────────────────

ast_node!(AtTag, AT_TAG);

impl AtTag {
    pub fn tag_name_node(&self) -> Option<TagName> {
        child_node(&self.0)
    }

    pub fn name(&self) -> Option<String> {
        self.tag_name_node()
            .and_then(|n| text_of_token(n.syntax(), SyntaxKind::IDENT))
    }

    pub fn param_list(&self) -> Option<ParamList> {
        child_node(&self.0)
    }

    pub fn params(&self) -> impl Iterator<Item = Param> + '_ {
        self.param_list()
            .into_iter()
            .flat_map(|pl| child_nodes::<Param>(pl.syntax()).collect::<Vec<_>>())
    }

    pub fn span(&self) -> miette::SourceSpan {
        node_span(&self.0)
    }
}

// ─── Inline tag ───────────────────────────────────────────────────────────────

ast_node!(InlineTag, INLINE_TAG);

impl InlineTag {
    pub fn tag_name_node(&self) -> Option<TagName> {
        child_node(&self.0)
    }

    pub fn name(&self) -> Option<String> {
        self.tag_name_node()
            .and_then(|n| text_of_token(n.syntax(), SyntaxKind::IDENT))
    }

    pub fn param_list(&self) -> Option<ParamList> {
        child_node(&self.0)
    }

    pub fn params(&self) -> impl Iterator<Item = Param> + '_ {
        self.param_list()
            .into_iter()
            .flat_map(|pl| child_nodes::<Param>(pl.syntax()).collect::<Vec<_>>())
    }

    pub fn span(&self) -> miette::SourceSpan {
        node_span(&self.0)
    }
}

// ─── TagName ──────────────────────────────────────────────────────────────────

ast_node!(TagName, TAG_NAME);

impl TagName {
    pub fn ident_token(&self) -> Option<SyntaxToken> {
        token_of_kind(&self.0, SyntaxKind::IDENT)
    }
    pub fn text(&self) -> Option<String> {
        self.ident_token().map(|t| t.text().to_owned())
    }
}

// ─── ParamList ────────────────────────────────────────────────────────────────

ast_node!(ParamList, PARAM_LIST);

impl ParamList {
    pub fn params(&self) -> impl Iterator<Item = Param> + '_ {
        child_nodes(&self.0)
    }
}

// ─── Param ────────────────────────────────────────────────────────────────────

ast_node!(Param, PARAM);

impl Param {
    /// The key of a named parameter, or `None` for positional parameters.
    pub fn key(&self) -> Option<String> {
        child_node::<ParamKey>(&self.0)
            .and_then(|k| text_of_token(k.syntax(), SyntaxKind::IDENT))
    }

    /// The value node (one of the PARAM_VALUE_* variants).
    pub fn value(&self) -> Option<ParamValue> {
        self.0.children().find_map(ParamValue::cast_node)
    }

    pub fn span(&self) -> miette::SourceSpan {
        node_span(&self.0)
    }
}

ast_node!(ParamKey, PARAM_KEY);

// ─── ParamValue ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ParamValue {
    Literal(ParamValueLiteral),
    Entity(ParamValueEntity),
    Macro(ParamValueMacro),
    Splat(ParamValueSplat),
}

impl ParamValue {
    fn cast_node(node: SyntaxNode) -> Option<Self> {
        Some(match node.kind() {
            SyntaxKind::PARAM_VALUE_LITERAL => Self::Literal(ParamValueLiteral(node)),
            SyntaxKind::PARAM_VALUE_ENTITY => Self::Entity(ParamValueEntity(node)),
            SyntaxKind::PARAM_VALUE_MACRO => Self::Macro(ParamValueMacro(node)),
            SyntaxKind::PARAM_VALUE_SPLAT => Self::Splat(ParamValueSplat(node)),
            _ => return None,
        })
    }

    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Literal(n) => &n.0,
            Self::Entity(n) => &n.0,
            Self::Macro(n) => &n.0,
            Self::Splat(n) => &n.0,
        }
    }

    pub fn span(&self) -> miette::SourceSpan {
        node_span(self.syntax())
    }
}

ast_node!(ParamValueLiteral, PARAM_VALUE_LITERAL);
impl ParamValueLiteral {
    /// The raw text of the literal (quotes stripped for quoted values).
    pub fn text(&self) -> String {
        let raw: String = self
            .0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .map(|t| t.text().to_owned())
            .collect();
        // Strip surrounding quote characters if present.
        if (raw.starts_with('"') && raw.ends_with('"'))
            || (raw.starts_with('\'') && raw.ends_with('\''))
        {
            raw[1..raw.len() - 1].to_owned()
        } else {
            raw
        }
    }
    pub fn span(&self) -> miette::SourceSpan { node_span(&self.0) }
}

ast_node!(ParamValueEntity, PARAM_VALUE_ENTITY);
impl ParamValueEntity {
    /// The expression after `&`.
    pub fn expr(&self) -> String {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|t| t.kind() != SyntaxKind::AMP)
            .map(|t| t.text().to_owned())
            .collect()
    }
    pub fn span(&self) -> miette::SourceSpan { node_span(&self.0) }
}

ast_node!(ParamValueMacro, PARAM_VALUE_MACRO);
impl ParamValueMacro {
    /// The macro parameter key (after `%`).
    pub fn key(&self) -> Option<String> {
        text_of_token(&self.0, SyntaxKind::IDENT)
    }

    /// The default value (after `|`), if present.
    pub fn default(&self) -> Option<String> {
        let mut found_pipe = false;
        let parts: Vec<String> = self
            .0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|t| {
                if t.kind() == SyntaxKind::PIPE {
                    found_pipe = true;
                    false
                } else {
                    found_pipe
                }
            })
            .map(|t| t.text().to_owned())
            .collect();
        if parts.is_empty() { None } else { Some(parts.concat()) }
    }
    pub fn span(&self) -> miette::SourceSpan { node_span(&self.0) }
}

ast_node!(ParamValueSplat, PARAM_VALUE_SPLAT);
impl ParamValueSplat {
    pub fn span(&self) -> miette::SourceSpan { node_span(&self.0) }
}

// ─── TextLine ─────────────────────────────────────────────────────────────────

ast_node!(TextLine, TEXT_LINE);

impl TextLine {
    pub fn parts(&self) -> impl Iterator<Item = TextPart> + '_ {
        self.0.children().filter_map(TextPart::cast_node)
    }

    pub fn span(&self) -> miette::SourceSpan {
        node_span(&self.0)
    }
}

// ─── TextPart ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TextPart {
    Literal(TextLiteral),
    InlineTag(InlineTag),
    Entity(TextEntity),
}

impl TextPart {
    fn cast_node(node: SyntaxNode) -> Option<Self> {
        Some(match node.kind() {
            SyntaxKind::TEXT_LITERAL => Self::Literal(TextLiteral(node)),
            SyntaxKind::INLINE_TAG => Self::InlineTag(InlineTag(node)),
            SyntaxKind::TEXT_ENTITY => Self::Entity(TextEntity(node)),
            _ => return None,
        })
    }

    pub fn span(&self) -> miette::SourceSpan {
        match self {
            Self::Literal(n) => node_span(&n.0),
            Self::InlineTag(n) => node_span(&n.0),
            Self::Entity(n) => node_span(&n.0),
        }
    }
}

ast_node!(TextLiteral, TEXT_LITERAL);
impl TextLiteral {
    pub fn text(&self) -> String {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .map(|t| t.text().to_owned())
            .collect()
    }
    pub fn span(&self) -> miette::SourceSpan { node_span(&self.0) }
}

ast_node!(TextEntity, TEXT_ENTITY);
impl TextEntity {
    pub fn expr(&self) -> String {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|t| t.kind() != SyntaxKind::AMP)
            .map(|t| t.text().to_owned())
            .collect()
    }
    pub fn span(&self) -> miette::SourceSpan { node_span(&self.0) }
}

// ─── CharaLine ────────────────────────────────────────────────────────────────

ast_node!(CharaLine, CHARA_LINE);

impl CharaLine {
    /// The character name (after `#`).
    pub fn name(&self) -> Option<String> {
        let mut found_hash = false;
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| {
                if t.kind() == SyntaxKind::HASH {
                    found_hash = true;
                    false
                } else {
                    found_hash && t.kind() == SyntaxKind::IDENT
                }
            })
            .map(|t| t.text().to_owned())
    }

    /// The face name (after `:`), if present.
    pub fn face(&self) -> Option<String> {
        let mut found_colon = false;
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| {
                if t.kind() == SyntaxKind::COLON {
                    found_colon = true;
                    false
                } else {
                    found_colon && t.kind() == SyntaxKind::IDENT
                }
            })
            .map(|t| t.text().to_owned())
    }

    pub fn span(&self) -> miette::SourceSpan {
        node_span(&self.0)
    }
}

// ─── IscriptBlock ─────────────────────────────────────────────────────────────

ast_node!(IscriptBlock, ISCRIPT_BLOCK);

impl IscriptBlock {
    /// The raw Rhai script content (all tokens concatenated, excluding the
    /// enclosing `[iscript]` / `[endscript]` lines).
    pub fn content(&self) -> String {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .map(|t| t.text().to_owned())
            .collect()
    }

    pub fn span(&self) -> miette::SourceSpan {
        node_span(&self.0)
    }
}

// ─── MacroDef ─────────────────────────────────────────────────────────────────

ast_node!(MacroDef, MACRO_DEF);

impl MacroDef {
    /// Iterate over the items making up the macro body.
    pub fn items(&self) -> impl Iterator<Item = Item> + '_ {
        self.0.children().filter_map(Item::cast_node)
    }

    pub fn span(&self) -> miette::SourceSpan {
        node_span(&self.0)
    }
}
