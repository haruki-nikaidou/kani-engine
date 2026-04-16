//! Type definitions for KAG tag names and typed attributes.
//!
//! Two complementary types are provided:
//!
//! * [`TagName`] — a lightweight `Copy` enum covering every distinct KAG tag
//!   name string.  Use it for fast dispatch without heap allocation.
//! * [`KnownTag`] — a richer `'src`-lifetime enum whose variants carry typed,
//!   validated attributes.  Construct one with [`KnownTag::from_tag`], which
//!   simultaneously validates the tag and emits any diagnostics.
//!
//! # Attribute types
//!
//! | KAG attribute kind | Rust field type |
//! |--------------------|-----------------|
//! | string / path / identifier | `Option<MaybeResolved<'src, AttributeString<'src>>>` |
//! | integer (`time=`, `fadetime=`) | `Option<MaybeResolved<'src, u64>>` |
//! | buffer slot (`buf=`) | `Option<MaybeResolved<'src, u32>>` |
//! | float (`x=`, `volume=`, `opacity=`) | `Option<MaybeResolved<'src, f32>>` |
//! | boolean (`visible=`, `loop=`, `canskip=`) | `Option<MaybeResolved<'src, bool>>` |
//!
//! # Severity policy
//!
//! | Severity | When |
//! |----------|------|
//! | **Error** | The tag *cannot function correctly* without the attribute — the runtime will silently discard the instruction or produce undefined behaviour. |
//! | **Warning** | The tag has a fallback (e.g. defaults to zero / empty), but the absence almost certainly indicates a typo or forgotten attribute. |
//!
//! # Macro-body safety
//!
//! Attributes carrying a [`ParamValue::Entity`] or [`ParamValue::MacroParam`]
//! value are represented as [`MaybeResolved::Dynamic`] and never trigger a
//! missing-attribute diagnostic — only a completely absent key does.

pub mod names;

use std::borrow::Cow;
use std::str::FromStr;

use miette::SourceSpan;

use crate::ast::{Param, ParamValue, Tag};
use crate::error::SyntaxWarning;
pub use crate::tag_defs::names::TagName;

// ─── AttributeString ─────────────────────────────────────────────────────────

/// A string-typed tag attribute value.
///
/// Distinguishes plain string attributes (like `storage=`, `target=`, `exp=`)
/// from numeric or boolean ones.  The inner [`Cow`] preserves the source
/// lifetime when the text is borrowed, or holds an owned copy otherwise.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeString<'src>(pub Cow<'src, str>);

// ─── MaybeResolved ───────────────────────────────────────────────────────────

/// An attribute value that is either statically known or requires runtime
/// resolution.
///
/// * `Literal(T)` — the source contained a plain string and it was parsed
///   successfully into `T`.
/// * `Dynamic(ParamValue)` — the source contained an `&expr` entity or a
///   `%key` macro parameter that can only be resolved at runtime.  The raw
///   [`ParamValue`] is preserved so the interpreter can resolve it later.
#[derive(Debug, Clone, PartialEq)]
pub enum MaybeResolved<'src, T> {
    Literal(T),
    Dynamic(ParamValue<'src>),
}

// ─── KnownTag ─────────────────────────────────────────────────────────────────

/// A KAG tag with typed, validated attributes extracted as named fields.
///
/// Construct with [`KnownTag::from_tag`], which both parses attributes into
/// their typed forms and emits diagnostics for any missing required or
/// recommended attributes.
///
/// Unknown tag names produce a [`KnownTag::Extension`] variant rather than an
/// error.  The aliases `"playSe"` and `"voice"` are decoded into the canonical
/// variants [`KnownTag::Se`] and [`KnownTag::Vo`] respectively.  If you need
/// to distinguish the original tag name string, read it from [`Tag::name`]
/// before calling `from_tag`.
#[derive(Debug, Clone, PartialEq)]
pub enum KnownTag<'src> {
    // ── Control flow ──────────────────────────────────────────────────────
    If {
        exp: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Elsif {
        exp: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Else,
    Endif,
    Ignore {
        exp: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Endignore,

    // ── Navigation ────────────────────────────────────────────────────────
    Jump {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        target: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Call {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        target: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Return,

    // ── Choice links ──────────────────────────────────────────────────────
    Link {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        target: Option<MaybeResolved<'src, AttributeString<'src>>>,
        text: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Endlink,
    Glink {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        target: Option<MaybeResolved<'src, AttributeString<'src>>>,
        text: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },

    // ── Scripting / expressions ───────────────────────────────────────────
    Eval {
        exp: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Emb {
        exp: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Trace {
        exp: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },

    // ── Display control ───────────────────────────────────────────────────
    L,
    P,
    R,
    S,
    Cm,
    Er,
    Ch {
        text: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Hch {
        text: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },

    // ── Timed waits ───────────────────────────────────────────────────────
    Wait {
        time: Option<MaybeResolved<'src, u64>>,
        canskip: Option<MaybeResolved<'src, bool>>,
    },
    Wc {
        time: Option<MaybeResolved<'src, u64>>,
    },

    // ── Async-completion waits (`wa`/`wm`/`wt`/`wq`/`wb`/`wf`/`wl`/`ws`/`wv`/`wp`) ──
    /// Covers the entire `w*` family of async-completion waits.
    ///
    /// `which` identifies the specific tag ([`TagName::Wa`] … [`TagName::Wp`]).
    /// `canskip` mirrors the KAG `canskip=` attribute; when `true` the host
    /// may resolve the wait early on a click.  `buf` selects the audio/effect
    /// buffer slot on waits that support it (e.g. `[ws]`, `[wv]`).
    WaitForCompletion {
        which: TagName,
        canskip: Option<MaybeResolved<'src, bool>>,
        buf: Option<MaybeResolved<'src, u32>>,
    },
    /// Cancel all in-progress asynchronous operations (`[ct]`).
    Ct,

    // ── Input / event handlers ────────────────────────────────────────────
    Timeout {
        time: Option<MaybeResolved<'src, u64>>,
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        target: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Waitclick,
    Cclick,
    Ctimeout,
    Cwheel,
    Click {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        target: Option<MaybeResolved<'src, AttributeString<'src>>>,
        exp: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Wheel {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        target: Option<MaybeResolved<'src, AttributeString<'src>>>,
        exp: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },

    // ── Log control ───────────────────────────────────────────────────────
    Nolog,
    Endnolog,

    // ── Display-speed control ─────────────────────────────────────────────
    Nowait,
    Endnowait,
    Resetdelay,
    Delay {
        speed: Option<MaybeResolved<'src, u64>>,
    },
    Configdelay {
        speed: Option<MaybeResolved<'src, u64>>,
    },
    Resetwait,
    Autowc {
        time: Option<MaybeResolved<'src, u64>>,
    },

    // ── Backlog ───────────────────────────────────────────────────────────
    Pushlog {
        text: Option<MaybeResolved<'src, AttributeString<'src>>>,
        join: Option<MaybeResolved<'src, bool>>,
    },

    // ── Player input / triggers ───────────────────────────────────────────
    Input {
        name: Option<MaybeResolved<'src, AttributeString<'src>>>,
        prompt: Option<MaybeResolved<'src, AttributeString<'src>>>,
        title: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Waittrig {
        name: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },

    // ── Macro management ──────────────────────────────────────────────────
    Macro {
        name: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Erasemacro {
        name: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Endmacro,

    // ── Variable management ───────────────────────────────────────────────
    Clearvar,
    Clearsysvar,
    Clearstack,

    // ── Misc ──────────────────────────────────────────────────────────────
    Clickskip {
        enabled: Option<MaybeResolved<'src, bool>>,
    },
    CharaPtext {
        name: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },

    // ── Image / layer (runtime passthrough) ───────────────────────────────
    Bg {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        time: Option<MaybeResolved<'src, u64>>,
        method: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Image {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        layer: Option<MaybeResolved<'src, AttributeString<'src>>>,
        x: Option<MaybeResolved<'src, f32>>,
        y: Option<MaybeResolved<'src, f32>>,
        visible: Option<MaybeResolved<'src, bool>>,
    },
    Layopt {
        layer: Option<MaybeResolved<'src, AttributeString<'src>>>,
        visible: Option<MaybeResolved<'src, bool>>,
        opacity: Option<MaybeResolved<'src, f32>>,
    },
    Free {
        layer: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Position {
        layer: Option<MaybeResolved<'src, AttributeString<'src>>>,
        x: Option<MaybeResolved<'src, f32>>,
        y: Option<MaybeResolved<'src, f32>>,
    },

    // ── Audio (runtime passthrough) ───────────────────────────────────────
    Bgm {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        /// KAG parameter key `loop`.
        r#loop: Option<MaybeResolved<'src, bool>>,
        volume: Option<MaybeResolved<'src, f32>>,
        fadetime: Option<MaybeResolved<'src, u64>>,
    },
    Stopbgm {
        fadetime: Option<MaybeResolved<'src, u64>>,
    },
    /// Covers both `[se]` and `[playSe]`.
    Se {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        buf: Option<MaybeResolved<'src, u32>>,
        volume: Option<MaybeResolved<'src, f32>>,
        /// KAG parameter key `loop`.
        r#loop: Option<MaybeResolved<'src, bool>>,
    },
    Stopse {
        buf: Option<MaybeResolved<'src, u32>>,
    },
    /// Covers both `[vo]` and `[voice]`.
    Vo {
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        buf: Option<MaybeResolved<'src, u32>>,
    },
    Fadebgm {
        time: Option<MaybeResolved<'src, u64>>,
        volume: Option<MaybeResolved<'src, f32>>,
    },

    // ── Transition (runtime passthrough) ──────────────────────────────────
    Trans {
        method: Option<MaybeResolved<'src, AttributeString<'src>>>,
        time: Option<MaybeResolved<'src, u64>>,
        rule: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Fadein {
        time: Option<MaybeResolved<'src, u64>>,
        color: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Fadeout {
        time: Option<MaybeResolved<'src, u64>>,
        color: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Movetrans {
        layer: Option<MaybeResolved<'src, AttributeString<'src>>>,
        time: Option<MaybeResolved<'src, u64>>,
        x: Option<MaybeResolved<'src, f32>>,
        y: Option<MaybeResolved<'src, f32>>,
    },

    // ── Effect (runtime passthrough) ──────────────────────────────────────
    Quake {
        time: Option<MaybeResolved<'src, u64>>,
        hmax: Option<MaybeResolved<'src, f32>>,
        vmax: Option<MaybeResolved<'src, f32>>,
    },
    Shake {
        time: Option<MaybeResolved<'src, u64>>,
        amount: Option<MaybeResolved<'src, f32>>,
        axis: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Flash {
        time: Option<MaybeResolved<'src, u64>>,
        color: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },

    // ── Message window (runtime passthrough) ──────────────────────────────
    Msgwnd {
        visible: Option<MaybeResolved<'src, bool>>,
        layer: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Wndctrl {
        x: Option<MaybeResolved<'src, f32>>,
        y: Option<MaybeResolved<'src, f32>>,
        width: Option<MaybeResolved<'src, f32>>,
        height: Option<MaybeResolved<'src, f32>>,
    },
    Resetfont,
    Font {
        face: Option<MaybeResolved<'src, AttributeString<'src>>>,
        size: Option<MaybeResolved<'src, f32>>,
        bold: Option<MaybeResolved<'src, bool>>,
        italic: Option<MaybeResolved<'src, bool>>,
    },
    Size {
        value: Option<MaybeResolved<'src, f32>>,
    },
    Bold {
        value: Option<MaybeResolved<'src, bool>>,
    },
    Italic {
        value: Option<MaybeResolved<'src, bool>>,
    },
    Ruby {
        text: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    Nowrap,
    Endnowrap,

    // ── Character sprites (runtime passthrough) ───────────────────────────
    Chara {
        name: Option<MaybeResolved<'src, AttributeString<'src>>>,
        id: Option<MaybeResolved<'src, AttributeString<'src>>>,
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
        slot: Option<MaybeResolved<'src, AttributeString<'src>>>,
        x: Option<MaybeResolved<'src, f32>>,
        y: Option<MaybeResolved<'src, f32>>,
    },
    CharaHide {
        name: Option<MaybeResolved<'src, AttributeString<'src>>>,
        id: Option<MaybeResolved<'src, AttributeString<'src>>>,
        slot: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    CharaFree {
        name: Option<MaybeResolved<'src, AttributeString<'src>>>,
        id: Option<MaybeResolved<'src, AttributeString<'src>>>,
        slot: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },
    CharaMod {
        name: Option<MaybeResolved<'src, AttributeString<'src>>>,
        id: Option<MaybeResolved<'src, AttributeString<'src>>>,
        face: Option<MaybeResolved<'src, AttributeString<'src>>>,
        pose: Option<MaybeResolved<'src, AttributeString<'src>>>,
        storage: Option<MaybeResolved<'src, AttributeString<'src>>>,
    },

    // ── Extensions ────────────────────────────────────────────────────────
    /// A tag not recognised by the engine.
    ///
    /// Game-specific or plugin code can match on this variant to handle
    /// custom tags.  The raw `name` and `params` are preserved so the tag can
    /// be forwarded to the host without loss of information.
    Extension {
        name: Cow<'src, str>,
        params: Vec<Param<'src>>,
    },
}

// ─── Private parsing helpers ──────────────────────────────────────────────────

/// Wrap a [`ParamValue`] as a string attribute.  Literals become
/// [`MaybeResolved::Literal`]; everything else becomes [`MaybeResolved::Dynamic`].
fn parse_str_attr<'src>(pv: ParamValue<'src>) -> MaybeResolved<'src, AttributeString<'src>> {
    match pv {
        ParamValue::Literal(s) => MaybeResolved::Literal(AttributeString(s)),
        other => MaybeResolved::Dynamic(other),
    }
}

/// Parse a [`ParamValue`] into a typed `T`.
///
/// * Literals are parsed with [`FromStr`]; on failure a warning is pushed and
///   the raw value is returned as [`MaybeResolved::Dynamic`].
/// * Non-literal values (entities, macro params) pass through as
///   [`MaybeResolved::Dynamic`] without touching `diags`.
fn parse_typed_attr<'src, T: FromStr>(
    pv: ParamValue<'src>,
    tag_name: &str,
    attr: &str,
    span: SourceSpan,
    diags: &mut Vec<SyntaxWarning>,
) -> MaybeResolved<'src, T> {
    if let ParamValue::Literal(s) = &pv {
        if let Ok(v) = s.parse::<T>() {
            return MaybeResolved::Literal(v);
        }
        diags.push(SyntaxWarning::warning(
            format!("[{tag_name}] attribute `{attr}=` has an unrecognised value"),
            span,
        ));
    }
    MaybeResolved::Dynamic(pv)
}

/// Emit an **error** diagnostic when `key` is absent from `tag`.
fn require_attr(tag: &Tag<'_>, key: &str, diags: &mut Vec<SyntaxWarning>) {
    if tag.param(key).is_none() {
        diags.push(SyntaxWarning::error(
            format!("[{}] is missing required attribute `{key}=`", tag.name),
            tag.span,
        ));
    }
}

/// Emit a **warning** diagnostic when `key` is absent from `tag`.
fn recommend_attr(tag: &Tag<'_>, key: &str, diags: &mut Vec<SyntaxWarning>) {
    if tag.param(key).is_none() {
        diags.push(SyntaxWarning::warning(
            format!("[{}] is missing `{key}=`; tag will have no effect", tag.name),
            tag.span,
        ));
    }
}

/// Emit a **warning** diagnostic when *none* of the given `keys` are present.
fn recommend_any_attr(tag: &Tag<'_>, keys: &[&str], diags: &mut Vec<SyntaxWarning>) {
    if !keys.iter().any(|k| tag.param(k).is_some()) {
        let keys_fmt = keys
            .iter()
            .map(|k| format!("`{k}=`"))
            .collect::<Vec<_>>()
            .join(", ");
        diags.push(SyntaxWarning::warning(
            format!(
                "[{}] should specify at least one of {keys_fmt}; tag will have no effect",
                tag.name
            ),
            tag.span,
        ));
    }
}

// ─── KnownTag impl ────────────────────────────────────────────────────────────

impl<'src> KnownTag<'src> {
    /// Parse and validate a raw [`Tag`] into a typed [`KnownTag`].
    ///
    /// Always returns a [`KnownTag`], using [`KnownTag::Extension`] for any
    /// tag name not recognised by the engine.  Diagnostics for missing
    /// required/recommended attributes or unparseable typed values are appended
    /// to `diags`.
    ///
    /// The aliases `"playSe"` and `"voice"` are decoded into the canonical
    /// variants [`KnownTag::Se`] and [`KnownTag::Vo`] respectively.
    pub fn from_tag(tag: &Tag<'src>, diags: &mut Vec<SyntaxWarning>) -> Self {
        let name = tag.name.as_ref();
        let span = tag.span;

        // String-attribute shorthand: look up a key and wrap it.
        let ps = |key: &str| tag.param(key).cloned().map(parse_str_attr);

        match name {
            // ── Control flow ───────────────────────────────────────────────
            "if" => {
                require_attr(tag, "exp", diags);
                Self::If { exp: ps("exp") }
            }
            "elsif" => {
                require_attr(tag, "exp", diags);
                Self::Elsif { exp: ps("exp") }
            }
            "else" => Self::Else,
            "endif" => Self::Endif,
            "ignore" => {
                require_attr(tag, "exp", diags);
                Self::Ignore { exp: ps("exp") }
            }
            "endignore" => Self::Endignore,

            // ── Navigation ────────────────────────────────────────────────
            "jump" => {
                recommend_any_attr(tag, &["storage", "target"], diags);
                Self::Jump {
                    storage: ps("storage"),
                    target: ps("target"),
                }
            }
            "call" => {
                recommend_any_attr(tag, &["storage", "target"], diags);
                Self::Call {
                    storage: ps("storage"),
                    target: ps("target"),
                }
            }
            "return" => Self::Return,

            // ── Choice links ──────────────────────────────────────────────
            "link" => {
                recommend_any_attr(tag, &["storage", "target"], diags);
                Self::Link {
                    storage: ps("storage"),
                    target: ps("target"),
                    text: ps("text"),
                }
            }
            "endlink" => Self::Endlink,
            "glink" => {
                recommend_any_attr(tag, &["storage", "target"], diags);
                Self::Glink {
                    storage: ps("storage"),
                    target: ps("target"),
                    text: ps("text"),
                }
            }

            // ── Scripting / expressions ───────────────────────────────────
            "eval" => {
                recommend_attr(tag, "exp", diags);
                Self::Eval { exp: ps("exp") }
            }
            "emb" => {
                recommend_attr(tag, "exp", diags);
                Self::Emb { exp: ps("exp") }
            }
            "trace" => {
                recommend_attr(tag, "exp", diags);
                Self::Trace { exp: ps("exp") }
            }

            // ── Display control ───────────────────────────────────────────
            "l" => Self::L,
            "p" => Self::P,
            "r" => Self::R,
            "s" => Self::S,
            "cm" => Self::Cm,
            "er" => Self::Er,
            "ch" => {
                recommend_attr(tag, "text", diags);
                Self::Ch { text: ps("text") }
            }
            "hch" => {
                recommend_attr(tag, "text", diags);
                Self::Hch { text: ps("text") }
            }

            // ── Timed waits ───────────────────────────────────────────────
            "wait" => {
                recommend_attr(tag, "time", diags);
                let time_pv = tag.param("time").cloned();
                let canskip_pv = tag.param("canskip").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                let canskip =
                    canskip_pv.map(|pv| parse_typed_attr(pv, name, "canskip", span, diags));
                Self::Wait { time, canskip }
            }
            "wc" => {
                recommend_attr(tag, "time", diags);
                let time_pv = tag.param("time").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                Self::Wc { time }
            }

            // ── Async-completion waits ─────────────────────────────────────
            "wa" | "wm" | "wt" | "wq" | "wb" | "wf" | "wl" | "ws" | "wv" | "wp" => {
                let which = match name {
                    "wa" => TagName::Wa,
                    "wm" => TagName::Wm,
                    "wt" => TagName::Wt,
                    "wq" => TagName::Wq,
                    "wb" => TagName::Wb,
                    "wf" => TagName::Wf,
                    "wl" => TagName::Wl,
                    "ws" => TagName::Ws,
                    "wv" => TagName::Wv,
                    _ => TagName::Wp,
                };
                let canskip_pv = tag.param("canskip").cloned();
                let buf_pv = tag.param("buf").cloned();
                let canskip =
                    canskip_pv.map(|pv| parse_typed_attr(pv, name, "canskip", span, diags));
                let buf = buf_pv.map(|pv| parse_typed_attr(pv, name, "buf", span, diags));
                Self::WaitForCompletion { which, canskip, buf }
            }
            "ct" => Self::Ct,

            // ── Input / event handlers ────────────────────────────────────
            "timeout" => {
                recommend_attr(tag, "time", diags);
                let time_pv = tag.param("time").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                Self::Timeout {
                    time,
                    storage: ps("storage"),
                    target: ps("target"),
                }
            }
            "waitclick" => Self::Waitclick,
            "cclick" => Self::Cclick,
            "ctimeout" => Self::Ctimeout,
            "cwheel" => Self::Cwheel,
            "click" => {
                recommend_any_attr(tag, &["storage", "target", "exp"], diags);
                Self::Click {
                    storage: ps("storage"),
                    target: ps("target"),
                    exp: ps("exp"),
                }
            }
            "wheel" => {
                recommend_any_attr(tag, &["storage", "target", "exp"], diags);
                Self::Wheel {
                    storage: ps("storage"),
                    target: ps("target"),
                    exp: ps("exp"),
                }
            }

            // ── Log control ───────────────────────────────────────────────
            "nolog" => Self::Nolog,
            "endnolog" => Self::Endnolog,

            // ── Display-speed control ─────────────────────────────────────
            "nowait" => Self::Nowait,
            "endnowait" => Self::Endnowait,
            "resetdelay" => Self::Resetdelay,
            "delay" => {
                recommend_attr(tag, "speed", diags);
                let speed_pv = tag.param("speed").cloned();
                let speed = speed_pv.map(|pv| parse_typed_attr(pv, name, "speed", span, diags));
                Self::Delay { speed }
            }
            "configdelay" => {
                recommend_attr(tag, "speed", diags);
                let speed_pv = tag.param("speed").cloned();
                let speed = speed_pv.map(|pv| parse_typed_attr(pv, name, "speed", span, diags));
                Self::Configdelay { speed }
            }
            "resetwait" => Self::Resetwait,
            "autowc" => {
                let time_pv = tag.param("time").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                Self::Autowc { time }
            }

            // ── Backlog ───────────────────────────────────────────────────
            "pushlog" => {
                recommend_attr(tag, "text", diags);
                let join_pv = tag.param("join").cloned();
                let join = join_pv.map(|pv| parse_typed_attr(pv, name, "join", span, diags));
                Self::Pushlog {
                    text: ps("text"),
                    join,
                }
            }

            // ── Player input / triggers ───────────────────────────────────
            "input" => {
                recommend_attr(tag, "name", diags);
                Self::Input {
                    name: ps("name"),
                    prompt: ps("prompt"),
                    title: ps("title"),
                }
            }
            "waittrig" => {
                recommend_attr(tag, "name", diags);
                Self::Waittrig { name: ps("name") }
            }

            // ── Macro management ──────────────────────────────────────────
            "macro" => Self::Macro { name: ps("name") },
            "erasemacro" => {
                recommend_attr(tag, "name", diags);
                Self::Erasemacro { name: ps("name") }
            }
            "endmacro" => Self::Endmacro,

            // ── Variable management ───────────────────────────────────────
            "clearvar" => Self::Clearvar,
            "clearsysvar" => Self::Clearsysvar,
            "clearstack" => Self::Clearstack,

            // ── Misc ──────────────────────────────────────────────────────
            "clickskip" => {
                let enabled_pv = tag.param("enabled").cloned();
                let enabled =
                    enabled_pv.map(|pv| parse_typed_attr(pv, name, "enabled", span, diags));
                Self::Clickskip { enabled }
            }
            "chara_ptext" => {
                recommend_attr(tag, "name", diags);
                Self::CharaPtext { name: ps("name") }
            }

            // ── Image / layer ─────────────────────────────────────────────
            "bg" => {
                require_attr(tag, "storage", diags);
                let time_pv = tag.param("time").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                Self::Bg {
                    storage: ps("storage"),
                    time,
                    method: ps("method"),
                }
            }
            "image" => {
                require_attr(tag, "storage", diags);
                let x_pv = tag.param("x").cloned();
                let y_pv = tag.param("y").cloned();
                let visible_pv = tag.param("visible").cloned();
                let x = x_pv.map(|pv| parse_typed_attr(pv, name, "x", span, diags));
                let y = y_pv.map(|pv| parse_typed_attr(pv, name, "y", span, diags));
                let visible =
                    visible_pv.map(|pv| parse_typed_attr(pv, name, "visible", span, diags));
                Self::Image {
                    storage: ps("storage"),
                    layer: ps("layer"),
                    x,
                    y,
                    visible,
                }
            }
            "layopt" => {
                require_attr(tag, "layer", diags);
                let visible_pv = tag.param("visible").cloned();
                let opacity_pv = tag.param("opacity").cloned();
                let visible =
                    visible_pv.map(|pv| parse_typed_attr(pv, name, "visible", span, diags));
                let opacity =
                    opacity_pv.map(|pv| parse_typed_attr(pv, name, "opacity", span, diags));
                Self::Layopt {
                    layer: ps("layer"),
                    visible,
                    opacity,
                }
            }
            "free" => {
                require_attr(tag, "layer", diags);
                Self::Free { layer: ps("layer") }
            }
            "position" => {
                require_attr(tag, "layer", diags);
                let x_pv = tag.param("x").cloned();
                let y_pv = tag.param("y").cloned();
                let x = x_pv.map(|pv| parse_typed_attr(pv, name, "x", span, diags));
                let y = y_pv.map(|pv| parse_typed_attr(pv, name, "y", span, diags));
                Self::Position {
                    layer: ps("layer"),
                    x,
                    y,
                }
            }

            // ── Audio ─────────────────────────────────────────────────────
            "bgm" => {
                require_attr(tag, "storage", diags);
                let loop_pv = tag.param("loop").cloned();
                let volume_pv = tag.param("volume").cloned();
                let fadetime_pv = tag.param("fadetime").cloned();
                let r#loop =
                    loop_pv.map(|pv| parse_typed_attr(pv, name, "loop", span, diags));
                let volume =
                    volume_pv.map(|pv| parse_typed_attr(pv, name, "volume", span, diags));
                let fadetime =
                    fadetime_pv.map(|pv| parse_typed_attr(pv, name, "fadetime", span, diags));
                Self::Bgm {
                    storage: ps("storage"),
                    r#loop,
                    volume,
                    fadetime,
                }
            }
            "stopbgm" => {
                let fadetime_pv = tag.param("fadetime").cloned();
                let fadetime =
                    fadetime_pv.map(|pv| parse_typed_attr(pv, name, "fadetime", span, diags));
                Self::Stopbgm { fadetime }
            }
            // "se" and "playSe" are semantically identical.
            "se" | "playSe" => {
                require_attr(tag, "storage", diags);
                let buf_pv = tag.param("buf").cloned();
                let volume_pv = tag.param("volume").cloned();
                let loop_pv = tag.param("loop").cloned();
                let buf = buf_pv.map(|pv| parse_typed_attr(pv, name, "buf", span, diags));
                let volume =
                    volume_pv.map(|pv| parse_typed_attr(pv, name, "volume", span, diags));
                let r#loop =
                    loop_pv.map(|pv| parse_typed_attr(pv, name, "loop", span, diags));
                Self::Se {
                    storage: ps("storage"),
                    buf,
                    volume,
                    r#loop,
                }
            }
            "stopse" => {
                let buf_pv = tag.param("buf").cloned();
                let buf = buf_pv.map(|pv| parse_typed_attr(pv, name, "buf", span, diags));
                Self::Stopse { buf }
            }
            // "vo" and "voice" are semantically identical.
            "vo" | "voice" => {
                require_attr(tag, "storage", diags);
                let buf_pv = tag.param("buf").cloned();
                let buf = buf_pv.map(|pv| parse_typed_attr(pv, name, "buf", span, diags));
                Self::Vo {
                    storage: ps("storage"),
                    buf,
                }
            }
            "fadebgm" => {
                let time_pv = tag.param("time").cloned();
                let volume_pv = tag.param("volume").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                let volume =
                    volume_pv.map(|pv| parse_typed_attr(pv, name, "volume", span, diags));
                Self::Fadebgm { time, volume }
            }

            // ── Transition ────────────────────────────────────────────────
            "trans" => {
                let time_pv = tag.param("time").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                Self::Trans {
                    method: ps("method"),
                    time,
                    rule: ps("rule"),
                }
            }
            "fadein" => {
                let time_pv = tag.param("time").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                Self::Fadein {
                    time,
                    color: ps("color"),
                }
            }
            "fadeout" => {
                let time_pv = tag.param("time").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                Self::Fadeout {
                    time,
                    color: ps("color"),
                }
            }
            "movetrans" => {
                let time_pv = tag.param("time").cloned();
                let x_pv = tag.param("x").cloned();
                let y_pv = tag.param("y").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                let x = x_pv.map(|pv| parse_typed_attr(pv, name, "x", span, diags));
                let y = y_pv.map(|pv| parse_typed_attr(pv, name, "y", span, diags));
                Self::Movetrans {
                    layer: ps("layer"),
                    time,
                    x,
                    y,
                }
            }

            // ── Effect ────────────────────────────────────────────────────
            "quake" => {
                let time_pv = tag.param("time").cloned();
                let hmax_pv = tag.param("hmax").cloned();
                let vmax_pv = tag.param("vmax").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                let hmax = hmax_pv.map(|pv| parse_typed_attr(pv, name, "hmax", span, diags));
                let vmax = vmax_pv.map(|pv| parse_typed_attr(pv, name, "vmax", span, diags));
                Self::Quake { time, hmax, vmax }
            }
            "shake" => {
                let time_pv = tag.param("time").cloned();
                let amount_pv = tag.param("amount").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                let amount =
                    amount_pv.map(|pv| parse_typed_attr(pv, name, "amount", span, diags));
                Self::Shake {
                    time,
                    amount,
                    axis: ps("axis"),
                }
            }
            "flash" => {
                let time_pv = tag.param("time").cloned();
                let time = time_pv.map(|pv| parse_typed_attr(pv, name, "time", span, diags));
                Self::Flash {
                    time,
                    color: ps("color"),
                }
            }

            // ── Message window ────────────────────────────────────────────
            "msgwnd" => {
                let visible_pv = tag.param("visible").cloned();
                let visible =
                    visible_pv.map(|pv| parse_typed_attr(pv, name, "visible", span, diags));
                Self::Msgwnd {
                    visible,
                    layer: ps("layer"),
                }
            }
            "wndctrl" => {
                let x_pv = tag.param("x").cloned();
                let y_pv = tag.param("y").cloned();
                let width_pv = tag.param("width").cloned();
                let height_pv = tag.param("height").cloned();
                let x = x_pv.map(|pv| parse_typed_attr(pv, name, "x", span, diags));
                let y = y_pv.map(|pv| parse_typed_attr(pv, name, "y", span, diags));
                let width = width_pv.map(|pv| parse_typed_attr(pv, name, "width", span, diags));
                let height =
                    height_pv.map(|pv| parse_typed_attr(pv, name, "height", span, diags));
                Self::Wndctrl { x, y, width, height }
            }
            "resetfont" => Self::Resetfont,
            "font" => {
                let size_pv = tag.param("size").cloned();
                let bold_pv = tag.param("bold").cloned();
                let italic_pv = tag.param("italic").cloned();
                let size = size_pv.map(|pv| parse_typed_attr(pv, name, "size", span, diags));
                let bold = bold_pv.map(|pv| parse_typed_attr(pv, name, "bold", span, diags));
                let italic =
                    italic_pv.map(|pv| parse_typed_attr(pv, name, "italic", span, diags));
                Self::Font {
                    face: ps("face"),
                    size,
                    bold,
                    italic,
                }
            }
            "size" => {
                let value_pv = tag.param("value").cloned();
                let value = value_pv.map(|pv| parse_typed_attr(pv, name, "value", span, diags));
                Self::Size { value }
            }
            "bold" => {
                let value_pv = tag.param("value").cloned();
                let value = value_pv.map(|pv| parse_typed_attr(pv, name, "value", span, diags));
                Self::Bold { value }
            }
            "italic" => {
                let value_pv = tag.param("value").cloned();
                let value = value_pv.map(|pv| parse_typed_attr(pv, name, "value", span, diags));
                Self::Italic { value }
            }
            "ruby" => Self::Ruby { text: ps("text") },
            "nowrap" => Self::Nowrap,
            "endnowrap" => Self::Endnowrap,

            // ── Character sprites ─────────────────────────────────────────
            "chara" => {
                recommend_any_attr(tag, &["name", "id"], diags);
                let x_pv = tag.param("x").cloned();
                let y_pv = tag.param("y").cloned();
                let x = x_pv.map(|pv| parse_typed_attr(pv, name, "x", span, diags));
                let y = y_pv.map(|pv| parse_typed_attr(pv, name, "y", span, diags));
                Self::Chara {
                    name: ps("name"),
                    id: ps("id"),
                    storage: ps("storage"),
                    slot: ps("slot"),
                    x,
                    y,
                }
            }
            "chara_hide" => {
                recommend_any_attr(tag, &["name", "id"], diags);
                Self::CharaHide {
                    name: ps("name"),
                    id: ps("id"),
                    slot: ps("slot"),
                }
            }
            "chara_free" => {
                recommend_any_attr(tag, &["name", "id"], diags);
                Self::CharaFree {
                    name: ps("name"),
                    id: ps("id"),
                    slot: ps("slot"),
                }
            }
            "chara_mod" => {
                recommend_any_attr(tag, &["name", "id"], diags);
                Self::CharaMod {
                    name: ps("name"),
                    id: ps("id"),
                    face: ps("face"),
                    pose: ps("pose"),
                    storage: ps("storage"),
                }
            }

            _ => Self::Extension {
                name: tag.name.clone(),
                params: tag.params.clone(),
            },
        }
    }

    /// Return the [`TagName`] corresponding to this variant, or `None` for
    /// [`KnownTag::Extension`] (which has no canonical tag name).
    ///
    /// For [`KnownTag::Se`] this returns [`Some(TagName::Se)`] (not
    /// [`TagName::PlaySe`]), and for [`KnownTag::Vo`] this returns
    /// [`Some(TagName::Vo)`] (not [`TagName::Voice`]).  For
    /// [`KnownTag::WaitForCompletion`] this returns the `which` field wrapped
    /// in `Some`.
    pub fn tag_name(&self) -> Option<TagName> {
        Some(match self {
            Self::If { .. } => TagName::If,
            Self::Elsif { .. } => TagName::Elsif,
            Self::Else => TagName::Else,
            Self::Endif => TagName::Endif,
            Self::Ignore { .. } => TagName::Ignore,
            Self::Endignore => TagName::Endignore,
            Self::Jump { .. } => TagName::Jump,
            Self::Call { .. } => TagName::Call,
            Self::Return => TagName::Return,
            Self::Link { .. } => TagName::Link,
            Self::Endlink => TagName::Endlink,
            Self::Glink { .. } => TagName::Glink,
            Self::Eval { .. } => TagName::Eval,
            Self::Emb { .. } => TagName::Emb,
            Self::Trace { .. } => TagName::Trace,
            Self::L => TagName::L,
            Self::P => TagName::P,
            Self::R => TagName::R,
            Self::S => TagName::S,
            Self::Cm => TagName::Cm,
            Self::Er => TagName::Er,
            Self::Ch { .. } => TagName::Ch,
            Self::Hch { .. } => TagName::Hch,
            Self::Wait { .. } => TagName::Wait,
            Self::Wc { .. } => TagName::Wc,
            Self::WaitForCompletion { which, .. } => *which,
            Self::Ct => TagName::Ct,
            Self::Timeout { .. } => TagName::Timeout,
            Self::Waitclick => TagName::Waitclick,
            Self::Cclick => TagName::Cclick,
            Self::Ctimeout => TagName::Ctimeout,
            Self::Cwheel => TagName::Cwheel,
            Self::Click { .. } => TagName::Click,
            Self::Wheel { .. } => TagName::Wheel,
            Self::Nolog => TagName::Nolog,
            Self::Endnolog => TagName::Endnolog,
            Self::Nowait => TagName::Nowait,
            Self::Endnowait => TagName::Endnowait,
            Self::Resetdelay => TagName::Resetdelay,
            Self::Delay { .. } => TagName::Delay,
            Self::Configdelay { .. } => TagName::Configdelay,
            Self::Resetwait => TagName::Resetwait,
            Self::Autowc { .. } => TagName::Autowc,
            Self::Pushlog { .. } => TagName::Pushlog,
            Self::Input { .. } => TagName::Input,
            Self::Waittrig { .. } => TagName::Waittrig,
            Self::Macro { .. } => TagName::Macro,
            Self::Erasemacro { .. } => TagName::Erasemacro,
            Self::Endmacro => TagName::Endmacro,
            Self::Clearvar => TagName::Clearvar,
            Self::Clearsysvar => TagName::Clearsysvar,
            Self::Clearstack => TagName::Clearstack,
            Self::Clickskip { .. } => TagName::Clickskip,
            Self::CharaPtext { .. } => TagName::CharaPtext,
            Self::Bg { .. } => TagName::Bg,
            Self::Image { .. } => TagName::Image,
            Self::Layopt { .. } => TagName::Layopt,
            Self::Free { .. } => TagName::Free,
            Self::Position { .. } => TagName::Position,
            Self::Bgm { .. } => TagName::Bgm,
            Self::Stopbgm { .. } => TagName::Stopbgm,
            Self::Se { .. } => TagName::Se,
            Self::Stopse { .. } => TagName::Stopse,
            Self::Vo { .. } => TagName::Vo,
            Self::Fadebgm { .. } => TagName::Fadebgm,
            Self::Trans { .. } => TagName::Trans,
            Self::Fadein { .. } => TagName::Fadein,
            Self::Fadeout { .. } => TagName::Fadeout,
            Self::Movetrans { .. } => TagName::Movetrans,
            Self::Quake { .. } => TagName::Quake,
            Self::Shake { .. } => TagName::Shake,
            Self::Flash { .. } => TagName::Flash,
            Self::Msgwnd { .. } => TagName::Msgwnd,
            Self::Wndctrl { .. } => TagName::Wndctrl,
            Self::Resetfont => TagName::Resetfont,
            Self::Font { .. } => TagName::Font,
            Self::Size { .. } => TagName::Size,
            Self::Bold { .. } => TagName::Bold,
            Self::Italic { .. } => TagName::Italic,
            Self::Ruby { .. } => TagName::Ruby,
            Self::Nowrap => TagName::Nowrap,
            Self::Endnowrap => TagName::Endnowrap,
            Self::Chara { .. } => TagName::Chara,
            Self::CharaHide { .. } => TagName::CharaHide,
            Self::CharaFree { .. } => TagName::CharaFree,
            Self::CharaMod { .. } => TagName::CharaMod,
            Self::Extension { .. } => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::ast::{Param, ParamValue, Tag};
    use crate::error::Severity;

    fn span() -> miette::SourceSpan {
        (0usize, 0usize).into()
    }

    fn tag_no_params(name: &'static str) -> Tag<'static> {
        Tag {
            name: Cow::Borrowed(name),
            params: vec![],
            span: span(),
        }
    }

    fn tag_with_param(name: &'static str, key: &'static str, val: &'static str) -> Tag<'static> {
        Tag {
            name: Cow::Borrowed(name),
            params: vec![Param::literal(key, val, span())],
            span: span(),
        }
    }

    fn tag_with_entity(name: &'static str, key: &'static str) -> Tag<'static> {
        Tag {
            name: Cow::Borrowed(name),
            params: vec![Param::named(
                key,
                ParamValue::Entity(Cow::Borrowed("f.path")),
                span(),
            )],
            span: span(),
        }
    }

    fn tag_with_macro_param(name: &'static str, key: &'static str) -> Tag<'static> {
        Tag {
            name: Cow::Borrowed(name),
            params: vec![Param::named(
                key,
                ParamValue::MacroParam {
                    key: Cow::Borrowed(key),
                    default: None,
                },
                span(),
            )],
            span: span(),
        }
    }

    // ── KnownTag::from_tag ────────────────────────────────────────────────────

    #[test]
    fn known_tag_from_unknown_produces_extension() {
        let mut diags = vec![];
        let known = KnownTag::from_tag(&tag_no_params("my_custom_tag"), &mut diags);
        assert!(diags.is_empty());
        assert!(matches!(known, KnownTag::Extension { .. }));
        assert_eq!(known.tag_name(), None);
    }

    #[test]
    fn known_tag_bg_extracts_params() {
        let tag = tag_with_param("bg", "storage", "bg001.jpg");
        let mut diags = vec![];
        let known = KnownTag::from_tag(&tag, &mut diags);
        assert!(diags.is_empty(), "no diags expected: {diags:?}");
        assert_eq!(known.tag_name(), Some(TagName::Bg));
        assert!(matches!(
            known,
            KnownTag::Bg {
                storage: Some(MaybeResolved::Literal(_)),
                time: None,
                method: None
            }
        ));
    }

    #[test]
    fn known_tag_se_and_play_se_unify() {
        let mut diags = vec![];
        let se = KnownTag::from_tag(&tag_with_param("se", "storage", "beep.ogg"), &mut diags);
        let play =
            KnownTag::from_tag(&tag_with_param("playSe", "storage", "beep.ogg"), &mut diags);
        assert!(diags.is_empty(), "no diags expected: {diags:?}");
        assert_eq!(se.tag_name(), Some(TagName::Se));
        assert_eq!(play.tag_name(), Some(TagName::Se));
        assert!(matches!(se, KnownTag::Se { .. }));
        assert!(matches!(play, KnownTag::Se { .. }));
    }

    #[test]
    fn known_tag_vo_and_voice_unify() {
        let mut diags = vec![];
        let vo = KnownTag::from_tag(&tag_with_param("vo", "storage", "v01.ogg"), &mut diags);
        let voice =
            KnownTag::from_tag(&tag_with_param("voice", "storage", "v01.ogg"), &mut diags);
        assert!(diags.is_empty(), "no diags expected: {diags:?}");
        assert_eq!(vo.tag_name(), Some(TagName::Vo));
        assert_eq!(voice.tag_name(), Some(TagName::Vo));
        assert!(matches!(vo, KnownTag::Vo { .. }));
        assert!(matches!(voice, KnownTag::Vo { .. }));
    }

    #[test]
    fn known_tag_wait_for_completion_carries_which() {
        for (name, expected) in &[
            ("wa", TagName::Wa),
            ("wm", TagName::Wm),
            ("wt", TagName::Wt),
            ("wq", TagName::Wq),
            ("wb", TagName::Wb),
            ("wf", TagName::Wf),
            ("wl", TagName::Wl),
            ("ws", TagName::Ws),
            ("wv", TagName::Wv),
            ("wp", TagName::Wp),
        ] {
            let mut diags = vec![];
            let known = KnownTag::from_tag(&tag_no_params(name), &mut diags);
            assert_eq!(known.tag_name(), Some(*expected), "failed for [{name}]");
            assert!(
                matches!(known, KnownTag::WaitForCompletion { which, .. } if which == *expected),
                "[{name}] should decode as WaitForCompletion"
            );
        }
    }

    #[test]
    fn known_tag_jump_extracts_both_params() {
        let tag = Tag {
            name: Cow::Borrowed("jump"),
            params: vec![
                Param::literal("storage", "scene02.ks", span()),
                Param::literal("target", "*start", span()),
            ],
            span: span(),
        };
        let mut diags = vec![];
        let known = KnownTag::from_tag(&tag, &mut diags);
        assert!(diags.is_empty(), "no diags expected: {diags:?}");
        assert!(matches!(
            known,
            KnownTag::Jump {
                storage: Some(MaybeResolved::Literal(_)),
                target: Some(MaybeResolved::Literal(_)),
            }
        ));
    }

    #[test]
    fn typed_attr_parses_u64() {
        let tag = tag_with_param("wait", "time", "500");
        let mut diags = vec![];
        let known = KnownTag::from_tag(&tag, &mut diags);
        assert!(diags.is_empty(), "no diags expected: {diags:?}");
        assert!(matches!(
            known,
            KnownTag::Wait {
                time: Some(MaybeResolved::Literal(500u64)),
                canskip: None,
            }
        ));
    }

    #[test]
    fn typed_attr_bad_value_is_warning_and_dynamic() {
        let tag = tag_with_param("wait", "time", "not_a_number");
        let mut diags = vec![];
        let known = KnownTag::from_tag(&tag, &mut diags);
        // One warning from recommend_attr (time absent? no, time IS present),
        // one warning from parse_typed_attr (bad value).
        assert_eq!(diags.len(), 1, "expected one bad-value warning: {diags:?}");
        assert_eq!(diags[0].severity, Severity::Warning);
        assert!(matches!(
            known,
            KnownTag::Wait {
                time: Some(MaybeResolved::Dynamic(_)),
                ..
            }
        ));
    }

    #[test]
    fn entity_attr_is_dynamic() {
        let tag = tag_with_entity("bg", "storage");
        let mut diags = vec![];
        let known = KnownTag::from_tag(&tag, &mut diags);
        assert!(diags.is_empty(), "no diags expected: {diags:?}");
        assert!(matches!(
            known,
            KnownTag::Bg {
                storage: Some(MaybeResolved::Dynamic(_)),
                ..
            }
        ));
    }

    // ── Required (error) ──────────────────────────────────────────────────────

    #[test]
    fn if_without_exp_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("if"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("exp="));
    }

    #[test]
    fn if_with_exp_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("if", "exp", "f.flag == 1"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn elsif_without_exp_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("elsif"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn ignore_without_exp_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("ignore"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn bg_without_storage_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("bg"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("storage="));
    }

    #[test]
    fn bg_with_storage_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("bg", "storage", "bg001.jpg"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn image_without_storage_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("image"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn layopt_without_layer_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("layopt"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn free_without_layer_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("free"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn position_without_layer_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("position"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn bgm_without_storage_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("bgm"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn se_without_storage_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("se"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn play_se_without_storage_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("playSe"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn vo_without_storage_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("vo"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn voice_without_storage_is_error() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("voice"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    // ── Recommended (warning) ─────────────────────────────────────────────────

    #[test]
    fn eval_without_exp_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("eval"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn eval_with_exp_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("eval", "exp", "f.x = 1"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn emb_without_exp_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("emb"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn trace_without_exp_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("trace"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn wait_without_time_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("wait"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn wait_with_time_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("wait", "time", "500"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn wc_without_time_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("wc"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn timeout_without_time_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("timeout"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn ch_without_text_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("ch"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn hch_without_text_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("hch"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn erasemacro_without_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("erasemacro"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn delay_without_speed_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("delay"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn configdelay_without_speed_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("configdelay"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn pushlog_without_text_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("pushlog"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn input_without_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("input"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn waittrig_without_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("waittrig"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_ptext_without_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("chara_ptext"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    // ── Any-of (warning) ──────────────────────────────────────────────────────

    #[test]
    fn jump_without_storage_or_target_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("jump"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
        assert!(diags[0].message.contains("storage="));
        assert!(diags[0].message.contains("target="));
    }

    #[test]
    fn jump_with_only_target_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("jump", "target", "*start"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn jump_with_only_storage_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("jump", "storage", "scene01.ks"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn call_without_destination_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("call"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn link_without_destination_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("link"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn link_with_target_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("link", "target", "*choice_a"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn glink_without_destination_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("glink"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn click_without_any_handler_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("click"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn click_with_exp_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("click", "exp", "f.handler()"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn wheel_without_any_handler_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("wheel"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_without_id_or_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("chara"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_with_name_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("chara", "name", "alice"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn chara_with_id_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("chara", "id", "alice"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn chara_hide_without_id_or_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("chara_hide"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_free_without_id_or_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("chara_free"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_mod_without_id_or_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("chara_mod"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    // ── Entity / macro-param values count as present ──────────────────────────

    #[test]
    fn bg_with_entity_storage_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_entity("bg", "storage"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn if_with_macro_param_exp_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_macro_param("if", "exp"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn bgm_with_entity_storage_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_entity("bgm", "storage"), &mut diags);
        assert!(diags.is_empty());
    }

    // ── Unknown tags produce Extension, no diagnostics ────────────────────────

    #[test]
    fn unknown_tag_is_extension_with_no_diags() {
        let mut diags = vec![];
        let known = KnownTag::from_tag(&tag_no_params("my_custom_game_tag"), &mut diags);
        assert!(diags.is_empty());
        assert!(matches!(known, KnownTag::Extension { .. }));
    }

    #[test]
    fn no_params_tags_are_clean() {
        for name in &[
            "l",
            "p",
            "r",
            "s",
            "cm",
            "return",
            "else",
            "endif",
            "endignore",
            "endlink",
            "endmacro",
            "nowait",
            "endnowait",
            "resetdelay",
            "nolog",
            "endnolog",
            "resetwait",
            "waitclick",
            "cclick",
            "ctimeout",
            "cwheel",
            "wa",
            "wm",
            "wt",
            "wq",
            "wb",
            "wf",
            "wl",
            "ws",
            "wv",
            "wp",
            "ct",
            "er",
            "clearvar",
            "clearsysvar",
            "clearstack",
            "stopbgm",
            "stopse",
            "trans",
            "fadein",
            "fadeout",
            "movetrans",
            "quake",
            "shake",
            "flash",
            "msgwnd",
            "wndctrl",
            "resetfont",
            "font",
            "size",
            "bold",
            "italic",
            "ruby",
            "nowrap",
            "endnowrap",
            "autowc",
            "clickskip",
        ] {
            let mut diags = vec![];
            KnownTag::from_tag(&tag_no_params(name), &mut diags);
            assert!(
                diags.is_empty(),
                "[{name}] should produce no diagnostics when used without params"
            );
        }
    }
}
