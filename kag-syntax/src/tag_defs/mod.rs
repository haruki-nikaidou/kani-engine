//! Compile-time validation rules for all known KAG tags, plus canonical type
//! definitions for tag names and their parameters.
//!
//! Every tag handled by either the interpreter (`kag-interpreter`) or the
//! runtime bridge (`kani-runtime`) is listed here.  Two complementary types
//! are provided:
//!
//! * [`TagName`] — a lightweight `Copy` enum covering every distinct KAG tag
//!   name string.  Use it for fast dispatch without heap allocation.
//! * [`KnownTag`] — a richer `'src`-lifetime enum whose variants carry the
//!   parsed [`ParamValue`] for each attribute of the tag.  Construct one with
//!   [`KnownTag::from_tag`].
//!
//! The lowering pass calls [`validate::validate_tag`] for every [`Tag`] it encounters
//! and collects the resulting [`ParseDiagnostic`]s alongside the normal parse
//! errors.
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
//! Parameters that carry a [`ParamValue::MacroParam`] or [`ParamValue::Entity`]
//! value are counted as *present* by this validator, so a tag like
//! `[if exp=%cond]` or `[bg storage=&f.path]` will never trigger a false
//! positive.  Only a completely absent key triggers a diagnostic.

pub mod names;
pub mod validate;

use crate::ast::{ParamValue, Tag};
pub use crate::tag_defs::names::TagName;

// ─── KnownTag ─────────────────────────────────────────────────────────────────

/// A KAG tag with its source-level attributes extracted as named fields.
///
/// Construct with [`KnownTag::from_tag`], which returns `None` for any tag
/// name not recognised by the engine (unknown tags pass through as generic
/// host events and produce no diagnostics).
///
/// The aliases `"playSe"` and `"voice"` are both decoded into the canonical
/// variants [`KnownTag::Se`] and [`KnownTag::Vo`] respectively.  If you need
/// to distinguish the original tag name string, read it from [`Tag::name`]
/// before calling `from_tag`.
///
/// # Attribute fields
///
/// All attribute fields use `Option<ParamValue<'src>>`:
///
/// | Value | Meaning |
/// |-------|---------|
/// | `None` | The attribute was absent from the source. |
/// | `Some(ParamValue::Literal(…))` | A plain string or bare-word value. |
/// | `Some(ParamValue::Entity(…))` | An `&expr` runtime expression. |
/// | `Some(ParamValue::MacroParam { … })` | A `%key` macro substitution. |
///
/// Both `Entity` and `MacroParam` variants count as *present* for validation
/// purposes, so `[bg storage=&f.path]` never triggers a missing-`storage=`
/// diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub enum KnownTag<'src> {
    // ── Control flow ──────────────────────────────────────────────────────
    If {
        exp: Option<ParamValue<'src>>,
    },
    Elsif {
        exp: Option<ParamValue<'src>>,
    },
    Else,
    Endif,
    Ignore {
        exp: Option<ParamValue<'src>>,
    },
    Endignore,

    // ── Navigation ────────────────────────────────────────────────────────
    Jump {
        storage: Option<ParamValue<'src>>,
        target: Option<ParamValue<'src>>,
    },
    Call {
        storage: Option<ParamValue<'src>>,
        target: Option<ParamValue<'src>>,
    },
    Return,

    // ── Choice links ──────────────────────────────────────────────────────
    Link {
        storage: Option<ParamValue<'src>>,
        target: Option<ParamValue<'src>>,
        text: Option<ParamValue<'src>>,
    },
    Endlink,
    Glink {
        storage: Option<ParamValue<'src>>,
        target: Option<ParamValue<'src>>,
        text: Option<ParamValue<'src>>,
    },

    // ── Scripting / expressions ───────────────────────────────────────────
    Eval {
        exp: Option<ParamValue<'src>>,
    },
    Emb {
        exp: Option<ParamValue<'src>>,
    },
    Trace {
        exp: Option<ParamValue<'src>>,
    },

    // ── Display control ───────────────────────────────────────────────────
    L,
    P,
    R,
    S,
    Cm,
    Er,
    Ch {
        text: Option<ParamValue<'src>>,
    },
    Hch {
        text: Option<ParamValue<'src>>,
    },

    // ── Timed waits ───────────────────────────────────────────────────────
    Wait {
        time: Option<ParamValue<'src>>,
        canskip: Option<ParamValue<'src>>,
    },
    Wc {
        time: Option<ParamValue<'src>>,
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
        canskip: Option<ParamValue<'src>>,
        buf: Option<ParamValue<'src>>,
    },
    /// Cancel all in-progress asynchronous operations (`[ct]`).
    Ct,

    // ── Input / event handlers ────────────────────────────────────────────
    Timeout {
        time: Option<ParamValue<'src>>,
        storage: Option<ParamValue<'src>>,
        target: Option<ParamValue<'src>>,
    },
    Waitclick,
    Cclick,
    Ctimeout,
    Cwheel,
    Click {
        storage: Option<ParamValue<'src>>,
        target: Option<ParamValue<'src>>,
        exp: Option<ParamValue<'src>>,
    },
    Wheel {
        storage: Option<ParamValue<'src>>,
        target: Option<ParamValue<'src>>,
        exp: Option<ParamValue<'src>>,
    },

    // ── Log control ───────────────────────────────────────────────────────
    Nolog,
    Endnolog,

    // ── Display-speed control ─────────────────────────────────────────────
    Nowait,
    Endnowait,
    Resetdelay,
    Delay {
        speed: Option<ParamValue<'src>>,
    },
    Configdelay {
        speed: Option<ParamValue<'src>>,
    },
    Resetwait,
    Autowc {
        time: Option<ParamValue<'src>>,
    },

    // ── Backlog ───────────────────────────────────────────────────────────
    Pushlog {
        text: Option<ParamValue<'src>>,
        join: Option<ParamValue<'src>>,
    },

    // ── Player input / triggers ───────────────────────────────────────────
    Input {
        name: Option<ParamValue<'src>>,
        prompt: Option<ParamValue<'src>>,
        title: Option<ParamValue<'src>>,
    },
    Waittrig {
        name: Option<ParamValue<'src>>,
    },

    // ── Macro management ──────────────────────────────────────────────────
    Macro {
        name: Option<ParamValue<'src>>,
    },
    Erasemacro {
        name: Option<ParamValue<'src>>,
    },
    Endmacro,

    // ── Variable management ───────────────────────────────────────────────
    Clearvar,
    Clearsysvar,
    Clearstack,

    // ── Misc ──────────────────────────────────────────────────────────────
    Clickskip {
        enabled: Option<ParamValue<'src>>,
    },
    CharaPtext {
        name: Option<ParamValue<'src>>,
    },

    // ── Image / layer (runtime passthrough) ───────────────────────────────
    Bg {
        storage: Option<ParamValue<'src>>,
        time: Option<ParamValue<'src>>,
        method: Option<ParamValue<'src>>,
    },
    Image {
        storage: Option<ParamValue<'src>>,
        layer: Option<ParamValue<'src>>,
        x: Option<ParamValue<'src>>,
        y: Option<ParamValue<'src>>,
        visible: Option<ParamValue<'src>>,
    },
    Layopt {
        layer: Option<ParamValue<'src>>,
        visible: Option<ParamValue<'src>>,
        opacity: Option<ParamValue<'src>>,
    },
    Free {
        layer: Option<ParamValue<'src>>,
    },
    Position {
        layer: Option<ParamValue<'src>>,
        x: Option<ParamValue<'src>>,
        y: Option<ParamValue<'src>>,
    },

    // ── Audio (runtime passthrough) ───────────────────────────────────────
    Bgm {
        storage: Option<ParamValue<'src>>,
        /// KAG parameter key `loop`.
        r#loop: Option<ParamValue<'src>>,
        volume: Option<ParamValue<'src>>,
        fadetime: Option<ParamValue<'src>>,
    },
    Stopbgm {
        fadetime: Option<ParamValue<'src>>,
    },
    /// Covers both `[se]` and `[playSe]`.
    Se {
        storage: Option<ParamValue<'src>>,
        buf: Option<ParamValue<'src>>,
        volume: Option<ParamValue<'src>>,
        /// KAG parameter key `loop`.
        r#loop: Option<ParamValue<'src>>,
    },
    Stopse {
        buf: Option<ParamValue<'src>>,
    },
    /// Covers both `[vo]` and `[voice]`.
    Vo {
        storage: Option<ParamValue<'src>>,
        buf: Option<ParamValue<'src>>,
    },
    Fadebgm {
        time: Option<ParamValue<'src>>,
        volume: Option<ParamValue<'src>>,
    },

    // ── Transition (runtime passthrough) ──────────────────────────────────
    Trans {
        method: Option<ParamValue<'src>>,
        time: Option<ParamValue<'src>>,
        rule: Option<ParamValue<'src>>,
    },
    Fadein {
        time: Option<ParamValue<'src>>,
        color: Option<ParamValue<'src>>,
    },
    Fadeout {
        time: Option<ParamValue<'src>>,
        color: Option<ParamValue<'src>>,
    },
    Movetrans {
        layer: Option<ParamValue<'src>>,
        time: Option<ParamValue<'src>>,
        x: Option<ParamValue<'src>>,
        y: Option<ParamValue<'src>>,
    },

    // ── Effect (runtime passthrough) ──────────────────────────────────────
    Quake {
        time: Option<ParamValue<'src>>,
        hmax: Option<ParamValue<'src>>,
        vmax: Option<ParamValue<'src>>,
    },
    Shake {
        time: Option<ParamValue<'src>>,
        amount: Option<ParamValue<'src>>,
        axis: Option<ParamValue<'src>>,
    },
    Flash {
        time: Option<ParamValue<'src>>,
        color: Option<ParamValue<'src>>,
    },

    // ── Message window (runtime passthrough) ──────────────────────────────
    Msgwnd {
        visible: Option<ParamValue<'src>>,
        layer: Option<ParamValue<'src>>,
    },
    Wndctrl {
        x: Option<ParamValue<'src>>,
        y: Option<ParamValue<'src>>,
        width: Option<ParamValue<'src>>,
        height: Option<ParamValue<'src>>,
    },
    Resetfont,
    Font {
        face: Option<ParamValue<'src>>,
        size: Option<ParamValue<'src>>,
        bold: Option<ParamValue<'src>>,
        italic: Option<ParamValue<'src>>,
    },
    Size {
        value: Option<ParamValue<'src>>,
    },
    Bold {
        value: Option<ParamValue<'src>>,
    },
    Italic {
        value: Option<ParamValue<'src>>,
    },
    Ruby {
        text: Option<ParamValue<'src>>,
    },
    Nowrap,
    Endnowrap,

    // ── Character sprites (runtime passthrough) ───────────────────────────
    Chara {
        name: Option<ParamValue<'src>>,
        id: Option<ParamValue<'src>>,
        storage: Option<ParamValue<'src>>,
        slot: Option<ParamValue<'src>>,
        x: Option<ParamValue<'src>>,
        y: Option<ParamValue<'src>>,
    },
    CharaHide {
        name: Option<ParamValue<'src>>,
        id: Option<ParamValue<'src>>,
        slot: Option<ParamValue<'src>>,
    },
    CharaFree {
        name: Option<ParamValue<'src>>,
        id: Option<ParamValue<'src>>,
        slot: Option<ParamValue<'src>>,
    },
    CharaMod {
        name: Option<ParamValue<'src>>,
        id: Option<ParamValue<'src>>,
        face: Option<ParamValue<'src>>,
        pose: Option<ParamValue<'src>>,
        storage: Option<ParamValue<'src>>,
    },
}

impl<'src> KnownTag<'src> {
    /// Decode a [`Tag`] into a `KnownTag`, extracting its named attributes.
    ///
    /// Returns `None` when the tag name is not recognised by the engine.
    ///
    /// The aliases `"playSe"` and `"voice"` are decoded into the canonical
    /// variants [`KnownTag::Se`] and [`KnownTag::Vo`] respectively.
    pub fn from_tag(tag: &Tag<'src>) -> Option<Self> {
        // Convenience closure: look up a parameter and clone its value.
        let p = |key: &str| tag.param(key).cloned();

        Some(match tag.name.as_ref() {
            // ── Control flow ───────────────────────────────────────────────
            "if" => Self::If { exp: p("exp") },
            "elsif" => Self::Elsif { exp: p("exp") },
            "else" => Self::Else,
            "endif" => Self::Endif,
            "ignore" => Self::Ignore { exp: p("exp") },
            "endignore" => Self::Endignore,

            // ── Navigation ────────────────────────────────────────────────
            "jump" => Self::Jump {
                storage: p("storage"),
                target: p("target"),
            },
            "call" => Self::Call {
                storage: p("storage"),
                target: p("target"),
            },
            "return" => Self::Return,

            // ── Choice links ──────────────────────────────────────────────
            "link" => Self::Link {
                storage: p("storage"),
                target: p("target"),
                text: p("text"),
            },
            "endlink" => Self::Endlink,
            "glink" => Self::Glink {
                storage: p("storage"),
                target: p("target"),
                text: p("text"),
            },

            // ── Scripting / expressions ───────────────────────────────────
            "eval" => Self::Eval { exp: p("exp") },
            "emb" => Self::Emb { exp: p("exp") },
            "trace" => Self::Trace { exp: p("exp") },

            // ── Display control ───────────────────────────────────────────
            "l" => Self::L,
            "p" => Self::P,
            "r" => Self::R,
            "s" => Self::S,
            "cm" => Self::Cm,
            "er" => Self::Er,
            "ch" => Self::Ch { text: p("text") },
            "hch" => Self::Hch { text: p("text") },

            // ── Timed waits ───────────────────────────────────────────────
            "wait" => Self::Wait {
                time: p("time"),
                canskip: p("canskip"),
            },
            "wc" => Self::Wc { time: p("time") },

            // ── Async-completion waits ─────────────────────────────────────
            "wa" => Self::WaitForCompletion {
                which: TagName::Wa,
                canskip: p("canskip"),
                buf: p("buf"),
            },
            "wm" => Self::WaitForCompletion {
                which: TagName::Wm,
                canskip: p("canskip"),
                buf: p("buf"),
            },
            "wt" => Self::WaitForCompletion {
                which: TagName::Wt,
                canskip: p("canskip"),
                buf: p("buf"),
            },
            "wq" => Self::WaitForCompletion {
                which: TagName::Wq,
                canskip: p("canskip"),
                buf: p("buf"),
            },
            "wb" => Self::WaitForCompletion {
                which: TagName::Wb,
                canskip: p("canskip"),
                buf: p("buf"),
            },
            "wf" => Self::WaitForCompletion {
                which: TagName::Wf,
                canskip: p("canskip"),
                buf: p("buf"),
            },
            "wl" => Self::WaitForCompletion {
                which: TagName::Wl,
                canskip: p("canskip"),
                buf: p("buf"),
            },
            "ws" => Self::WaitForCompletion {
                which: TagName::Ws,
                canskip: p("canskip"),
                buf: p("buf"),
            },
            "wv" => Self::WaitForCompletion {
                which: TagName::Wv,
                canskip: p("canskip"),
                buf: p("buf"),
            },
            "wp" => Self::WaitForCompletion {
                which: TagName::Wp,
                canskip: p("canskip"),
                buf: p("buf"),
            },
            "ct" => Self::Ct,

            // ── Input / event handlers ────────────────────────────────────
            "timeout" => Self::Timeout {
                time: p("time"),
                storage: p("storage"),
                target: p("target"),
            },
            "waitclick" => Self::Waitclick,
            "cclick" => Self::Cclick,
            "ctimeout" => Self::Ctimeout,
            "cwheel" => Self::Cwheel,
            "click" => Self::Click {
                storage: p("storage"),
                target: p("target"),
                exp: p("exp"),
            },
            "wheel" => Self::Wheel {
                storage: p("storage"),
                target: p("target"),
                exp: p("exp"),
            },

            // ── Log control ───────────────────────────────────────────────
            "nolog" => Self::Nolog,
            "endnolog" => Self::Endnolog,

            // ── Display-speed control ─────────────────────────────────────
            "nowait" => Self::Nowait,
            "endnowait" => Self::Endnowait,
            "resetdelay" => Self::Resetdelay,
            "delay" => Self::Delay { speed: p("speed") },
            "configdelay" => Self::Configdelay { speed: p("speed") },
            "resetwait" => Self::Resetwait,
            "autowc" => Self::Autowc { time: p("time") },

            // ── Backlog ───────────────────────────────────────────────────
            "pushlog" => Self::Pushlog {
                text: p("text"),
                join: p("join"),
            },

            // ── Player input / triggers ───────────────────────────────────
            "input" => Self::Input {
                name: p("name"),
                prompt: p("prompt"),
                title: p("title"),
            },
            "waittrig" => Self::Waittrig { name: p("name") },

            // ── Macro management ──────────────────────────────────────────
            "macro" => Self::Macro { name: p("name") },
            "erasemacro" => Self::Erasemacro { name: p("name") },
            "endmacro" => Self::Endmacro,

            // ── Variable management ───────────────────────────────────────
            "clearvar" => Self::Clearvar,
            "clearsysvar" => Self::Clearsysvar,
            "clearstack" => Self::Clearstack,

            // ── Misc ──────────────────────────────────────────────────────
            "clickskip" => Self::Clickskip {
                enabled: p("enabled"),
            },
            "chara_ptext" => Self::CharaPtext { name: p("name") },

            // ── Image / layer ─────────────────────────────────────────────
            "bg" => Self::Bg {
                storage: p("storage"),
                time: p("time"),
                method: p("method"),
            },
            "image" => Self::Image {
                storage: p("storage"),
                layer: p("layer"),
                x: p("x"),
                y: p("y"),
                visible: p("visible"),
            },
            "layopt" => Self::Layopt {
                layer: p("layer"),
                visible: p("visible"),
                opacity: p("opacity"),
            },
            "free" => Self::Free { layer: p("layer") },
            "position" => Self::Position {
                layer: p("layer"),
                x: p("x"),
                y: p("y"),
            },

            // ── Audio ─────────────────────────────────────────────────────
            "bgm" => Self::Bgm {
                storage: p("storage"),
                r#loop: p("loop"),
                volume: p("volume"),
                fadetime: p("fadetime"),
            },
            "stopbgm" => Self::Stopbgm {
                fadetime: p("fadetime"),
            },
            // "se" and "playSe" are semantically identical.
            "se" | "playSe" => Self::Se {
                storage: p("storage"),
                buf: p("buf"),
                volume: p("volume"),
                r#loop: p("loop"),
            },
            "stopse" => Self::Stopse { buf: p("buf") },
            // "vo" and "voice" are semantically identical.
            "vo" | "voice" => Self::Vo {
                storage: p("storage"),
                buf: p("buf"),
            },
            "fadebgm" => Self::Fadebgm {
                time: p("time"),
                volume: p("volume"),
            },

            // ── Transition ────────────────────────────────────────────────
            "trans" => Self::Trans {
                method: p("method"),
                time: p("time"),
                rule: p("rule"),
            },
            "fadein" => Self::Fadein {
                time: p("time"),
                color: p("color"),
            },
            "fadeout" => Self::Fadeout {
                time: p("time"),
                color: p("color"),
            },
            "movetrans" => Self::Movetrans {
                layer: p("layer"),
                time: p("time"),
                x: p("x"),
                y: p("y"),
            },

            // ── Effect ────────────────────────────────────────────────────
            "quake" => Self::Quake {
                time: p("time"),
                hmax: p("hmax"),
                vmax: p("vmax"),
            },
            "shake" => Self::Shake {
                time: p("time"),
                amount: p("amount"),
                axis: p("axis"),
            },
            "flash" => Self::Flash {
                time: p("time"),
                color: p("color"),
            },

            // ── Message window ────────────────────────────────────────────
            "msgwnd" => Self::Msgwnd {
                visible: p("visible"),
                layer: p("layer"),
            },
            "wndctrl" => Self::Wndctrl {
                x: p("x"),
                y: p("y"),
                width: p("width"),
                height: p("height"),
            },
            "resetfont" => Self::Resetfont,
            "font" => Self::Font {
                face: p("face"),
                size: p("size"),
                bold: p("bold"),
                italic: p("italic"),
            },
            "size" => Self::Size { value: p("value") },
            "bold" => Self::Bold { value: p("value") },
            "italic" => Self::Italic { value: p("value") },
            "ruby" => Self::Ruby { text: p("text") },
            "nowrap" => Self::Nowrap,
            "endnowrap" => Self::Endnowrap,

            // ── Character sprites ─────────────────────────────────────────
            "chara" => Self::Chara {
                name: p("name"),
                id: p("id"),
                storage: p("storage"),
                slot: p("slot"),
                x: p("x"),
                y: p("y"),
            },
            "chara_hide" => Self::CharaHide {
                name: p("name"),
                id: p("id"),
                slot: p("slot"),
            },
            "chara_free" => Self::CharaFree {
                name: p("name"),
                id: p("id"),
                slot: p("slot"),
            },
            "chara_mod" => Self::CharaMod {
                name: p("name"),
                id: p("id"),
                face: p("face"),
                pose: p("pose"),
                storage: p("storage"),
            },

            _ => return None,
        })
    }

    /// Return the [`TagName`] corresponding to this variant.
    ///
    /// For [`KnownTag::Se`] this returns [`TagName::Se`] (not
    /// [`TagName::PlaySe`]), and for [`KnownTag::Vo`] this returns
    /// [`TagName::Vo`] (not [`TagName::Voice`]).  For
    /// [`KnownTag::WaitForCompletion`] this returns the `which` field.
    pub fn tag_name(&self) -> TagName {
        match self {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::ast::{Param, ParamValue, Tag};

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

    // ── KnownTag::from_tag ────────────────────────────────────────────────────

    #[test]
    fn known_tag_from_unknown_returns_none() {
        assert!(KnownTag::from_tag(&tag_no_params("my_custom_tag")).is_none());
    }

    #[test]
    fn known_tag_bg_extracts_params() {
        let tag = tag_with_param("bg", "storage", "bg001.jpg");
        let known = KnownTag::from_tag(&tag).unwrap();
        assert_eq!(known.tag_name(), TagName::Bg);
        assert!(matches!(
            known,
            KnownTag::Bg {
                storage: Some(ParamValue::Literal(_)),
                time: None,
                method: None
            }
        ));
    }

    #[test]
    fn known_tag_se_and_play_se_unify() {
        let se = KnownTag::from_tag(&tag_no_params("se")).unwrap();
        let play = KnownTag::from_tag(&tag_no_params("playSe")).unwrap();
        // Both decode as KnownTag::Se; tag_name() returns the primary variant.
        assert_eq!(se.tag_name(), TagName::Se);
        assert_eq!(play.tag_name(), TagName::Se);
        assert!(matches!(se, KnownTag::Se { .. }));
        assert!(matches!(play, KnownTag::Se { .. }));
    }

    #[test]
    fn known_tag_vo_and_voice_unify() {
        let vo = KnownTag::from_tag(&tag_no_params("vo")).unwrap();
        let voice = KnownTag::from_tag(&tag_no_params("voice")).unwrap();
        assert_eq!(vo.tag_name(), TagName::Vo);
        assert_eq!(voice.tag_name(), TagName::Vo);
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
            let known = KnownTag::from_tag(&tag_no_params(name)).unwrap();
            assert_eq!(known.tag_name(), *expected, "failed for [{name}]");
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
        let known = KnownTag::from_tag(&tag).unwrap();
        assert!(matches!(
            known,
            KnownTag::Jump {
                storage: Some(ParamValue::Literal(_)),
                target: Some(ParamValue::Literal(_)),
            }
        ));
    }
}
