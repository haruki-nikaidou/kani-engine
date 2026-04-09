use miette::SourceSpan;
use std::borrow::Cow;
use std::collections::HashMap;

// ─── Core position type ───────────────────────────────────────────────────────

/// Byte-offset span inside the original source string.
pub type Span = SourceSpan;

// ─── Tag parameter values ─────────────────────────────────────────────────────

/// The value half of a `key=value` parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue<'src> {
    /// A plain string literal (quoted or bare word).
    Literal(Cow<'src, str>),
    /// A runtime-evaluated expression introduced by `&` (e.g. `&f.counter`).
    Entity(Cow<'src, str>),
    /// A macro parameter reference: `%key` or `%key|default`.
    MacroParam {
        key: Cow<'src, str>,
        default: Option<Cow<'src, str>>,
    },
    /// The bare `*` splat — passes all macro arguments to this parameter slot.
    MacroSplat,
}

impl<'src> ParamValue<'src> {
    pub fn into_owned(self) -> ParamValue<'static> {
        match self {
            Self::Literal(s) => ParamValue::Literal(Cow::Owned(s.into_owned())),
            Self::Entity(s) => ParamValue::Entity(Cow::Owned(s.into_owned())),
            Self::MacroParam { key, default } => ParamValue::MacroParam {
                key: Cow::Owned(key.into_owned()),
                default: default.map(|d| Cow::Owned(d.into_owned())),
            },
            Self::MacroSplat => ParamValue::MacroSplat,
        }
    }
}

// ─── Tag parameter ────────────────────────────────────────────────────────────

/// A single `key=value` or positional (`value` only) parameter on a tag.
#[derive(Debug, Clone, PartialEq)]
pub struct Param<'src> {
    /// `None` for positional (bare) values; `Some(key)` for named ones.
    pub key: Option<Cow<'src, str>>,
    pub value: ParamValue<'src>,
    /// Byte span of the entire parameter (key + `=` + value) in the source.
    pub span: Span,
}

impl<'src> Param<'src> {
    pub fn named(key: impl Into<Cow<'src, str>>, value: ParamValue<'src>, span: Span) -> Self {
        Self {
            key: Some(key.into()),
            value,
            span,
        }
    }

    pub fn literal(
        key: impl Into<Cow<'src, str>>,
        val: impl Into<Cow<'src, str>>,
        span: Span,
    ) -> Self {
        Self::named(key, ParamValue::Literal(val.into()), span)
    }

    /// Convenience constructor for synthetic params (e.g. from `#chara` sugar)
    /// where no meaningful source span exists.
    pub fn synthetic(key: impl Into<Cow<'src, str>>, val: impl Into<Cow<'src, str>>) -> Self {
        Self::named(
            key,
            ParamValue::Literal(val.into()),
            (0usize, 0usize).into(),
        )
    }

    pub fn into_owned(self) -> Param<'static> {
        Param {
            key: self.key.map(|k| Cow::Owned(k.into_owned())),
            value: self.value.into_owned(),
            span: self.span,
        }
    }
}

// ─── Tag ─────────────────────────────────────────────────────────────────────

/// A single KAG tag: `[name key=value …]` or `@name key=value …`.
#[derive(Debug, Clone, PartialEq)]
pub struct Tag<'src> {
    pub name: Cow<'src, str>,
    pub params: Vec<Param<'src>>,
    pub span: Span,
}

impl<'src> Tag<'src> {
    /// Retrieve the value of a named parameter as a borrowed string literal,
    /// if it exists and is a `Literal` variant.
    pub fn param_str(&self, key: &str) -> Option<&str> {
        self.params.iter().find_map(|p| {
            if p.key.as_deref() == Some(key) {
                if let ParamValue::Literal(ref s) = p.value {
                    Some(s.as_ref())
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Retrieve a named `ParamValue` by key.
    pub fn param(&self, key: &str) -> Option<&ParamValue<'src>> {
        self.params.iter().find_map(|p| {
            if p.key.as_deref() == Some(key) {
                Some(&p.value)
            } else {
                None
            }
        })
    }

    pub fn into_owned(self) -> Tag<'static> {
        Tag {
            name: Cow::Owned(self.name.into_owned()),
            params: self.params.into_iter().map(Param::into_owned).collect(),
            span: self.span,
        }
    }
}

// ─── Text content ─────────────────────────────────────────────────────────────

/// A fragment of a text line.
#[derive(Debug, Clone, PartialEq)]
pub enum TextPart<'src> {
    /// A literal string segment (may contain escaped characters already resolved).
    Literal { text: Cow<'src, str>, span: Span },
    /// An inline `[tag …]` embedded within running text.
    InlineTag(Tag<'src>),
    /// A runtime-evaluated entity `&expr` appearing directly in text.
    Entity { expr: Cow<'src, str>, span: Span },
}

impl<'src> TextPart<'src> {
    /// Byte span of this text fragment.
    pub fn span(&self) -> Span {
        match self {
            Self::Literal { span, .. } => *span,
            Self::InlineTag(t) => t.span,
            Self::Entity { span, .. } => *span,
        }
    }

    pub fn into_owned(self) -> TextPart<'static> {
        match self {
            Self::Literal { text, span } => TextPart::Literal {
                text: Cow::Owned(text.into_owned()),
                span,
            },
            Self::InlineTag(t) => TextPart::InlineTag(t.into_owned()),
            Self::Entity { expr, span } => TextPart::Entity {
                expr: Cow::Owned(expr.into_owned()),
                span,
            },
        }
    }
}

// ─── Label definition ─────────────────────────────────────────────────────────

/// `*label_name` or `*label_name|display title`
#[derive(Debug, Clone, PartialEq)]
pub struct LabelDef<'src> {
    pub name: Cow<'src, str>,
    pub title: Option<Cow<'src, str>>,
    pub span: Span,
}

impl<'src> LabelDef<'src> {
    pub fn into_owned(self) -> LabelDef<'static> {
        LabelDef {
            name: Cow::Owned(self.name.into_owned()),
            title: self.title.map(|t| Cow::Owned(t.into_owned())),
            span: self.span,
        }
    }
}

// ─── Macro definition ─────────────────────────────────────────────────────────

/// The body range of a `[macro name=foo]` … `[endmacro]` block.
/// Stores the inclusive op-list index range for the macro's body.
#[derive(Debug, Clone)]
pub struct MacroDef {
    /// Index into `Script::ops` where the body starts (one past the
    /// `[macro]` tag itself).
    pub body_start: usize,
    /// Index into `Script::ops` one past the final op before `[endmacro]`.
    pub body_end: usize,
}

// ─── Op stream ────────────────────────────────────────────────────────────────

/// A single unit in the parsed op stream — the output of the KAG parser.
#[derive(Debug, Clone)]
pub enum Op<'src> {
    /// One or more text fragments including inline tags.
    Text {
        parts: Vec<TextPart<'src>>,
        /// Byte span of the entire text line.
        span: Span,
    },
    /// A tag instruction (both `@tag` and `[tag]` parse to this).
    Tag(Tag<'src>),
    /// A label definition (`*name` or `*name|title`).
    Label(LabelDef<'src>),
    /// A raw script block between `[iscript]` and `[endscript]`.
    ScriptBlock {
        content: String,
        /// Byte span of the entire `[iscript]…[endscript]` block.
        span: Span,
    },
}

impl<'src> Op<'src> {
    /// Byte span of this op in the original source.
    pub fn span(&self) -> Span {
        match self {
            Self::Text { span, .. } => *span,
            Self::Tag(t) => t.span,
            Self::Label(l) => l.span,
            Self::ScriptBlock { span, .. } => *span,
        }
    }

    pub fn into_owned(self) -> Op<'static> {
        match self {
            Self::Text { parts, span } => Op::Text {
                parts: parts.into_iter().map(TextPart::into_owned).collect(),
                span,
            },
            Self::Tag(tag) => Op::Tag(tag.into_owned()),
            Self::Label(def) => Op::Label(def.into_owned()),
            Self::ScriptBlock { content, span } => Op::ScriptBlock { content, span },
        }
    }
}

// ─── Parsed script ────────────────────────────────────────────────────────────

/// The complete result of parsing a `.ks` scenario file.
///
/// `'src` borrows from the original source `&str` wherever possible.
/// Use `.into_owned()` to get a `Script<'static>` that owns all strings.
#[derive(Debug, Clone)]
pub struct Script<'src> {
    /// Flat, ordered op list.  The runtime advances a `pc` index into this.
    pub ops: Vec<Op<'src>>,
    /// Maps label names to their op-list index (position of the `Label` op).
    pub label_map: HashMap<Cow<'src, str>, usize>,
    /// Maps macro names to their definition (body range in `ops`).
    pub macro_map: HashMap<Cow<'src, str>, MacroDef>,
    /// Human-readable name (usually the filename) for error messages.
    pub source_name: String,
}

impl<'src> Script<'src> {
    pub fn new(source_name: impl Into<String>) -> Self {
        Self {
            ops: Vec::new(),
            label_map: HashMap::new(),
            macro_map: HashMap::new(),
            source_name: source_name.into(),
        }
    }

    /// Convert all borrowed strings to owned, producing a `Script<'static>`
    /// that can be sent across thread boundaries.
    pub fn into_owned(self) -> Script<'static> {
        Script {
            ops: self.ops.into_iter().map(Op::into_owned).collect(),
            label_map: self
                .label_map
                .into_iter()
                .map(|(k, v)| (Cow::Owned(k.into_owned()), v))
                .collect(),
            macro_map: self
                .macro_map
                .into_iter()
                .map(|(k, v)| (Cow::Owned(k.into_owned()), v))
                .collect(),
            source_name: self.source_name,
        }
    }
}
