//! `SyntaxKind` — the flat integer tag used by Rowan for every token and node
//! in the KAG CST.
//!
//! Token variants mirror the `Token` enum produced by the `logos` lexer so the
//! conversion is a simple numeric mapping.  Node variants are layered on top
//! and use higher discriminant values.

use rowan::Language;

/// A flat `u16` tag identifying every terminal token and every composite node
/// in the KAG concrete syntax tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
#[allow(non_camel_case_types)]
// `__LAST` is used as a bounds sentinel in `from_raw`, not as a non-exhaustive marker.
#[allow(clippy::manual_non_exhaustive)]
pub enum SyntaxKind {
    // ── Tokens (must stay in sync with `lexer::Token`) ────────────────────

    /// `\n` or `\r\n`
    NEWLINE = 0,
    /// `;…` or `//…` (to end of line)
    LINE_COMMENT,
    /// `/*` on its own (block-comment delimiter)
    BLOCK_COMMENT_OPEN,
    /// `*/` on its own (block-comment delimiter)
    BLOCK_COMMENT_CLOSE,

    /// `@` — line-tag sigil
    AT,
    /// `#` — character-name shorthand sigil
    HASH,
    /// `*` — label sigil or macro-splat inside a tag
    STAR,

    /// `[` — opens an inline tag
    L_BRACKET,
    /// `]` — closes an inline tag
    R_BRACKET,

    /// `=` — key/value separator inside a tag parameter
    EQ,
    /// `&` — entity / runtime-expression sigil
    AMP,
    /// `%` — macro-parameter reference sigil
    PERCENT,
    /// `|` — label title separator or macro-param default separator
    PIPE,
    /// `:` — face separator in `#name:face`
    COLON,

    /// `"…"` — double-quoted string (includes surrounding quotes)
    DOUBLE_QUOTED,
    /// `'…'` — single-quoted string (includes surrounding quotes)
    SINGLE_QUOTED,

    /// Identifier: `[a-zA-Z_][a-zA-Z0-9_\-\.]*`
    IDENT,
    /// Numeric literal: `[0-9]+` or `-?[0-9]+\.[0-9]+`
    NUMBER,
    /// `\` — backslash escape in text
    BACKSLASH,
    /// Plain text characters (punctuation, CJK, etc.)
    TEXT,
    /// Horizontal whitespace (spaces/tabs)
    WHITESPACE,
    /// `/` that is not part of `//` or `*/`
    SLASH,
    /// `<`
    LT,
    /// `>`
    GT,

    /// An unrecognised token or malformed sub-tree produced during error
    /// recovery.  The Rowan tree can contain ERROR nodes; the parser still
    /// continues after recording a `ParseDiagnostic`.
    ERROR,

    // ── Nodes ─────────────────────────────────────────────────────────────

    /// The root node — wraps the entire `.ks` file.
    ROOT,

    /// A line-comment node (`;…` or `//…`).
    LINE_COMMENT_NODE,
    /// A block comment: `/*` … `*/` (may span multiple lines).
    BLOCK_COMMENT_NODE,

    /// A label definition: `*name` or `*name|title`.
    LABEL_DEF,

    /// A line-level tag: `@tagname params…`.
    AT_TAG,
    /// An inline tag: `[tagname params…]`.
    INLINE_TAG,

    /// The name token wrapped in its own node for easy access.
    TAG_NAME,

    /// The list of parameters attached to a tag.
    PARAM_LIST,
    /// A single `key=value` or bare-`value` parameter.
    PARAM,
    /// The key part of a named parameter.
    PARAM_KEY,
    /// A literal string parameter value (bare or quoted).
    PARAM_VALUE_LITERAL,
    /// An entity expression parameter value: `&expr`.
    PARAM_VALUE_ENTITY,
    /// A macro-parameter reference: `%key` or `%key|default`.
    PARAM_VALUE_MACRO,
    /// The bare `*` macro-splat.
    PARAM_VALUE_SPLAT,

    /// A dialogue / text line, potentially containing inline tags and entities.
    TEXT_LINE,
    /// A contiguous run of literal text inside a `TEXT_LINE`.
    TEXT_LITERAL,
    /// An inline entity `&expr` appearing directly in running text.
    TEXT_ENTITY,

    /// A character-name shorthand line: `#name` or `#name:face`.
    CHARA_LINE,

    /// An `[iscript]` … `[endscript]` block (raw Rhai script content).
    ISCRIPT_BLOCK,

    /// A `[macro name=foo]` … `[endmacro]` block.
    MACRO_DEF,

    #[doc(hidden)]
    __LAST,
}

impl SyntaxKind {
    /// Convert a raw `u16` discriminant back to a `SyntaxKind`.
    ///
    /// # Panics
    /// Panics if `raw` is out of range.
    pub fn from_raw(raw: u16) -> Self {
        assert!(raw < Self::__LAST as u16);
        // Map the raw u16 to a SyntaxKind variant without unsafe transmute.
        // We use a generated match; `__LAST` ensures the assert above guards
        // against out-of-range values.
        macro_rules! from_raw_match {
            ($raw:expr; $($variant:ident),+ $(,)?) => {
                match $raw {
                    $(x if x == Self::$variant as u16 => Self::$variant,)+
                    _ => unreachable!("SyntaxKind::from_raw: out-of-range value {}", $raw),
                }
            };
        }
        from_raw_match!(raw;
            NEWLINE, LINE_COMMENT, BLOCK_COMMENT_OPEN, BLOCK_COMMENT_CLOSE,
            AT, HASH, STAR, L_BRACKET, R_BRACKET,
            EQ, AMP, PERCENT, PIPE, COLON,
            DOUBLE_QUOTED, SINGLE_QUOTED, IDENT, NUMBER, BACKSLASH,
            TEXT, WHITESPACE, SLASH, LT, GT,
            ERROR,
            ROOT, LINE_COMMENT_NODE, BLOCK_COMMENT_NODE,
            LABEL_DEF, AT_TAG, INLINE_TAG, TAG_NAME,
            PARAM_LIST, PARAM, PARAM_KEY,
            PARAM_VALUE_LITERAL, PARAM_VALUE_ENTITY, PARAM_VALUE_MACRO, PARAM_VALUE_SPLAT,
            TEXT_LINE, TEXT_LITERAL, TEXT_ENTITY,
            CHARA_LINE, ISCRIPT_BLOCK, MACRO_DEF,
        )
    }
}

// ─── Rowan language impl ──────────────────────────────────────────────────────

/// The Rowan `Language` tag for KAG syntax trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KagLanguage;

impl Language for KagLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        SyntaxKind::from_raw(raw.0)
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

/// Rowan `SyntaxNode` specialised for KAG.
pub type SyntaxNode = rowan::SyntaxNode<KagLanguage>;
/// Rowan `SyntaxToken` specialised for KAG.
pub type SyntaxToken = rowan::SyntaxToken<KagLanguage>;
/// Rowan `SyntaxElement` specialised for KAG.
pub type SyntaxElement = rowan::SyntaxElement<KagLanguage>;
