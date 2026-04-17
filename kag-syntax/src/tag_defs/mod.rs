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

use crate::ast::{ParamValue, Tag};
use crate::error::SyntaxDiagnostic;

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

// ─── Private parsing helpers ──────────────────────────────────────────────────

/// Wrap a [`ParamValue`] as a string attribute.
fn parse_str_attr(pv: ParamValue) -> MaybeResolved<AttributeString> {
    match pv {
        ParamValue::Literal(s) => MaybeResolved::Literal(AttributeString(s)),
        other => MaybeResolved::Dynamic(other),
    }
}

/// Parse a [`ParamValue`] into a typed `T`.
fn parse_typed_attr<'src, T: FromStr>(
    pv: ParamValue<'src>,
    tag_name: &str,
    attr: &str,
    span: SourceSpan,
    diags: &mut Vec<SyntaxDiagnostic>,
) -> MaybeResolved<'src, T> {
    if let ParamValue::Literal(s) = &pv {
        if let Ok(v) = s.parse::<T>() {
            return MaybeResolved::Literal(v);
        }
        diags.push(SyntaxDiagnostic::warning(
            format!("[{tag_name}] attribute `{attr}=` has an unrecognised value"),
            span,
        ));
    }
    MaybeResolved::Dynamic(pv)
}

/// Emit an **error** diagnostic when `key` is absent from `tag`.
fn require_attr(tag: &Tag<'_>, key: &str, diags: &mut Vec<SyntaxDiagnostic>) {
    if tag.param(key).is_none() {
        diags.push(SyntaxDiagnostic::error(
            format!("[{}] is missing required attribute `{key}=`", tag.name),
            tag.span,
        ));
    }
}

/// Emit a **warning** diagnostic when `key` is absent from `tag`.
fn recommend_attr(tag: &Tag<'_>, key: &str, diags: &mut Vec<SyntaxDiagnostic>) {
    if tag.param(key).is_none() {
        diags.push(SyntaxDiagnostic::warning(
            format!(
                "[{}] is missing `{key}=`; tag will have no effect",
                tag.name
            ),
            tag.span,
        ));
    }
}

/// Emit a **warning** diagnostic when *none* of the given `keys` are present.
fn recommend_any_attr(tag: &Tag<'_>, keys: &[&str], diags: &mut Vec<SyntaxDiagnostic>) {
    if !keys.iter().any(|k| tag.param(k).is_some()) {
        let keys_fmt = keys
            .iter()
            .map(|k| format!("`{k}=`"))
            .collect::<Vec<_>>()
            .join(", ");
        diags.push(SyntaxDiagnostic::warning(
            format!(
                "[{}] should specify at least one of {keys_fmt}; tag will have no effect",
                tag.name
            ),
            tag.span,
        ));
    }
}

// ─── Macro definition & invocation ────────────────────────────────────────────

#[macro_use]
mod macros;

define_tags! {
    // ── Control flow ────────────────────────────────────────────────────────
    /// Conditional branch — begin.
    If("if") {
        exp: required<str>,
    },
    /// Conditional branch — else-if.
    Elsif("elsif") {
        exp: required<str>,
    },
    /// Conditional branch — else.
    Else("else") {},
    /// Conditional branch — end.
    Endif("endif") {},
    /// Ignore block — begin.
    Ignore("ignore") {
        exp: required<str>,
    },
    /// Ignore block — end.
    Endignore("endignore") {},

    // ── Navigation ──────────────────────────────────────────────────────────
    /// Jump to another label or file.
    Jump("jump") {
        storage: recommended_any_of("storage", "target")<str>,
        target: recommended_any_of("storage", "target")<str>,
    },
    /// Call a subroutine.
    Call("call") {
        storage: recommended_any_of("storage", "target")<str>,
        target: recommended_any_of("storage", "target")<str>,
    },
    /// Return from a subroutine.
    Return("return") {},

    // ── Choice links ────────────────────────────────────────────────────────
    /// Begin a clickable link region.
    Link("link") {
        storage: recommended_any_of("storage", "target")<str>,
        target: recommended_any_of("storage", "target")<str>,
        text: optional<str>,
    },
    /// End a clickable link region.
    Endlink("endlink") {},
    /// A graphical link button.
    Glink("glink") {
        storage: recommended_any_of("storage", "target")<str>,
        target: recommended_any_of("storage", "target")<str>,
        text: optional<str>,
    },

    // ── Scripting / expressions ─────────────────────────────────────────────
    /// Evaluate a Rhai expression.
    Eval("eval") {
        exp: recommended<str>,
    },
    /// Embed expression result as text.
    Emb("emb") {
        exp: recommended<str>,
    },
    /// Trace an expression to the debug log.
    Trace("trace") {
        exp: recommended<str>,
    },

    // ── Display control ─────────────────────────────────────────────────────
    /// Wait for a click (line wait).
    L("l") {},
    /// Wait for a page-break click, then clear the window.
    P("p") {},
    /// Insert a line break in the message window.
    R("r") {},
    /// Stop script execution.
    S("s") {},
    /// Clear the current message layer.
    Cm("cm") {},
    /// Erase all layers.
    Er("er") {},
    /// Output a single character.
    Ch("ch") {
        text: recommended<str>,
    },
    /// Output a half-width character.
    Hch("hch") {
        text: recommended<str>,
    },

    // ── Timed waits ─────────────────────────────────────────────────────────
    /// Wait for a fixed number of milliseconds.
    Wait("wait") {
        time: recommended<u64>,
        canskip: optional<bool>,
    },
    /// Wait for a click with timeout.
    Wc("wc") {
        time: recommended<u64>,
    },

    // ── Cancel async ────────────────────────────────────────────────────────
    /// Cancel all in-progress asynchronous operations.
    Ct("ct") {},

    // ── Input / event handlers ──────────────────────────────────────────────
    /// Set a timeout handler.
    Timeout("timeout") {
        time: recommended<u64>,
        storage: optional<str>,
        target: optional<str>,
    },
    /// Wait for a click.
    Waitclick("waitclick") {},
    /// Cancel click handler.
    Cclick("cclick") {},
    /// Cancel timeout handler.
    Ctimeout("ctimeout") {},
    /// Cancel wheel handler.
    Cwheel("cwheel") {},
    /// Set a click handler.
    Click("click") {
        storage: recommended_any_of("storage", "target", "exp")<str>,
        target: recommended_any_of("storage", "target", "exp")<str>,
        exp: recommended_any_of("storage", "target", "exp")<str>,
    },
    /// Set a wheel handler.
    Wheel("wheel") {
        storage: recommended_any_of("storage", "target", "exp")<str>,
        target: recommended_any_of("storage", "target", "exp")<str>,
        exp: recommended_any_of("storage", "target", "exp")<str>,
    },

    // ── Log control ─────────────────────────────────────────────────────────
    /// Disable backlog recording.
    Nolog("nolog") {},
    /// Re-enable backlog recording.
    Endnolog("endnolog") {},

    // ── Display-speed control ───────────────────────────────────────────────
    /// Disable per-character delay.
    Nowait("nowait") {},
    /// Re-enable per-character delay.
    Endnowait("endnowait") {},
    /// Reset character display delay to default.
    Resetdelay("resetdelay") {},
    /// Set character display delay.
    Delay("delay") {
        speed: recommended<u64>,
    },
    /// Set the config-layer display delay.
    Configdelay("configdelay") {
        speed: recommended<u64>,
    },
    /// Reset auto-mode wait time.
    Resetwait("resetwait") {},
    /// Set auto-mode wait after each character.
    Autowc("autowc") {
        time: optional<u64>,
    },

    // ── Backlog ─────────────────────────────────────────────────────────────
    /// Push text to the backlog.
    Pushlog("pushlog") {
        text: recommended<str>,
        join: optional<bool>,
    },

    // ── Player input / triggers ─────────────────────────────────────────────
    /// Prompt the player for text input.
    Input("input") {
        name: recommended<str>,
        prompt: optional<str>,
        title: optional<str>,
    },
    /// Wait for a named trigger.
    Waittrig("waittrig") {
        name: recommended<str>,
    },

    // ── Macro management ────────────────────────────────────────────────────
    /// Define a macro.
    Macro("macro") {
        name: optional<str>,
    },
    /// Delete a macro definition.
    Erasemacro("erasemacro") {
        name: recommended<str>,
    },
    /// End a macro definition.
    Endmacro("endmacro") {},

    // ── Variable management ─────────────────────────────────────────────────
    /// Clear game variables.
    Clearvar("clearvar") {},
    /// Clear system variables.
    Clearsysvar("clearsysvar") {},
    /// Clear the call stack.
    Clearstack("clearstack") {},

    // ── UI / Menus ──────────────────────────────────────────────────────────
    /// Spawn a clickable button widget.
    Button("button") {
        text: optional<str>,
        graphic: optional<str>,
        x: optional<f32>,
        y: optional<f32>,
        width: optional<f32>,
        height: optional<f32>,
        bg: optional<str>,
        hover_bg: optional<str>,
        press_bg: optional<str>,
        color: optional<str>,
        font_size: optional<f32>,
        target: optional<str>,
        storage: optional<str>,
        exp: optional<str>,
        key: optional<str>,
        visible: optional<bool>,
        opacity: optional<f32>,
    },
    /// Make a layer respond to click events.
    Clickable("clickable") {
        layer: required<str>,
        target: optional<str>,
        storage: optional<str>,
        exp: optional<str>,
    },
    /// Open the main menu panel.
    Showmenu("showmenu") {},
    /// Open the load screen panel.
    Showload("showload") {},
    /// Open the save screen panel.
    Showsave("showsave") {},
    /// Open the backlog viewer panel.
    Showlog("showlog") {},
    /// Temporarily hide the message window.
    Hidemessage("hidemessage") {},
    /// Show the persistent menu button.
    Showmenubutton("showmenubutton") {},
    /// Hide the persistent menu button.
    Hidemenubutton("hidemenubutton") {},
    /// Display a modal dialog box.
    Dialog("dialog") {
        text: optional<str>,
        title: optional<str>,
    },
    /// Change the mouse cursor image.
    Cursor("cursor") {
        storage: optional<str>,
    },
    /// Enable display of the speaker name box.
    SpeakOn("speak_on") {},
    /// Disable display of the speaker name box.
    SpeakOff("speak_off") {},
    /// Configure the default click-wait glyph.
    Glyph("glyph") {
        storage: optional<str>,
    },
    /// Configure the auto-mode glyph.
    GlyphAuto("glyph_auto") {
        storage: optional<str>,
    },
    /// Configure the skip-mode glyph.
    GlyphSkip("glyph_skip") {
        storage: optional<str>,
    },
    /// Set visual defaults for glink buttons.
    GlinkConfig("glink_config") {},
    /// Visual effect when skip/auto mode starts or stops.
    ModeEffect("mode_effect") {
        mode: recommended<str>,
        effect: optional<str>,
    },

    // ── Skip control ────────────────────────────────────────────────────────
    /// Enable skip mode — [l]/[p] waits auto-advance at high speed.
    Skipstart("skipstart") {},
    /// Disable skip mode.
    Skipstop("skipstop") {},
    /// Cancel skip mode (alias for skipstop).
    Cancelskip("cancelskip") {},
    /// Open the key-binding configuration UI.
    StartKeyconfig("start_keyconfig") {},
    /// Close the key-binding configuration UI.
    StopKeyconfig("stop_keyconfig") {},

    // ── Misc ────────────────────────────────────────────────────────────────
    /// Enable or disable click-to-skip.
    Clickskip("clickskip") {
        enabled: optional<bool>,
    },
    /// Open a URL in the system browser.
    Web("web") {
        url: recommended<str>,
    },
    // ── Image / layer ───────────────────────────────────────────────────────
    /// Copy the current front layer to the back layer (for transition prep).
    Backlay("backlay") {},
    /// Set the active message/text layer.
    Current("current") {
        layer: optional<str>,
    },
    /// Position the text cursor within the current message layer.
    Locate("locate") {
        x: optional<f32>,
        y: optional<f32>,
    },
    /// Set the blend mode on a layer.
    Layermode("layermode") {
        layer: required<str>,
        mode: recommended<str>,
    },
    /// Reset the blend mode of a layer to Normal.
    FreeLayermode("free_layermode") {
        layer: required<str>,
    },
    /// Apply a named shader effect to a layer.
    Filter("filter") {
        layer: required<str>,
        r#type: recommended<str>,
    },
    /// Remove a filter from a layer.
    FreeFilter("free_filter") {
        layer: required<str>,
    },
    /// Position a filter within its layer.
    PositionFilter("position_filter") {
        layer: required<str>,
        x: optional<f32>,
        y: optional<f32>,
    },
    /// Apply an alpha mask image to a layer.
    Mask("mask") {
        layer: required<str>,
        storage: required<str>,
    },
    /// Remove a mask from a layer.
    MaskOff("mask_off") {
        layer: required<str>,
    },
    /// Draw a primitive shape on a layer.
    Graph("graph") {
        layer: optional<str>,
        shape: recommended<str>,
        x: optional<f32>,
        y: optional<f32>,
        width: optional<f32>,
        height: optional<f32>,
        color: optional<str>,
    },
    /// Set the background image.
    Bg("bg") {
        storage: required<str>,
        time: optional<u64>,
        method: optional<str>,
    },
    /// Display an image on a layer.
    Image("image") {
        storage: required<str>,
        layer: optional<str>,
        x: optional<f32>,
        y: optional<f32>,
        visible: optional<bool>,
    },
    /// Set layer options.
    Layopt("layopt") {
        layer: required<str>,
        visible: optional<bool>,
        opacity: optional<f32>,
    },
    /// Free (remove) a layer.
    Free("free") {
        layer: required<str>,
    },
    /// Free a layer by image (alias for free).
    Freeimage("freeimage") {
        layer: required<str>,
    },
    /// Free a layer (alias for free).
    Freelayer("freelayer") {
        layer: required<str>,
    },
    /// Set layer position.
    Position("position") {
        layer: optional<str>,
        x: optional<f32>,
        y: optional<f32>,
    },

    // ── Audio ───────────────────────────────────────────────────────────────
    /// Play background music.
    Bgm("bgm", alias Playbgm "playbgm") {
        storage: required<str>,
        r#loop: optional<bool>,
        volume: optional<f32>,
        fadetime: optional<u64>,
    },
    /// Stop background music.
    Stopbgm("stopbgm") {
        fadetime: optional<u64>,
    },
    /// Start BGM with a fade-in (alias for bgm with fadetime set).
    Fadeinbgm("fadeinbgm") {
        storage: recommended<str>,
        time: optional<u64>,
    },
    /// Stop BGM with a fade-out (alias for stopbgm with fadetime set).
    Fadeoutbgm("fadeoutbgm") {
        time: optional<u64>,
    },
    /// Pause BGM at the current seek position.
    Pausebgm("pausebgm") {
        buf: optional<u32>,
    },
    /// Resume paused BGM from the saved seek position.
    Resumebgm("resumebgm") {
        buf: optional<u32>,
    },
    /// Fade background music volume.
    Fadebgm("fadebgm") {
        time: optional<u64>,
        volume: optional<f32>,
    },
    /// Cross-fade: start new BGM while fading out the current one.
    Xchgbgm("xchgbgm") {
        storage: recommended<str>,
        time: optional<u64>,
    },
    /// Change options on the currently-playing BGM without restarting.
    Bgmopt("bgmopt") {
        r#loop: optional<bool>,
        seek: optional<str>,
    },
    /// Play a sound effect.
    Se("se", alias PlaySe "playSe") {
        storage: required<str>,
        buf: optional<u32>,
        volume: optional<f32>,
        r#loop: optional<bool>,
    },
    /// Stop a sound effect.
    Stopse("stopse") {
        buf: optional<u32>,
    },
    /// Pause a sound effect buffer.
    Pausese("pausese") {
        buf: optional<u32>,
    },
    /// Resume a paused sound effect buffer.
    Resumese("resumese") {
        buf: optional<u32>,
    },
    /// Change options on a currently-playing SE buffer without restarting.
    Seopt("seopt") {
        buf: optional<u32>,
        r#loop: optional<bool>,
    },
    /// Play a voice clip.
    Vo("vo", alias Voice "voice") {
        storage: required<str>,
        buf: optional<u32>,
    },
    /// Set the volume for a target channel (bgm, se, voice).
    Changevol("changevol") {
        target: optional<str>,
        vol: optional<f32>,
        time: optional<u64>,
    },

    // ── Animation ───────────────────────────────────────────────────────────
    /// Play a preset animation on a named layer.
    Anim("anim") {
        layer: optional<str>,
        preset: optional<str>,
        time: optional<u64>,
        r#loop: optional<bool>,
        delay: optional<u64>,
    },
    /// Cancel the ongoing animation on a layer.
    Stopanim("stopanim") {
        layer: optional<str>,
    },
    /// Begin a named keyframe sequence definition.
    Keyframe("keyframe") {
        name: recommended<str>,
    },
    /// One keyframe inside a [keyframe] block.
    Frame("frame") {
        time: recommended<u64>,
        opacity: optional<f32>,
        x: optional<f32>,
        y: optional<f32>,
    },
    /// End a keyframe sequence definition.
    Endkeyframe("endkeyframe") {},
    /// Play a named keyframe animation on a layer.
    Kanim("kanim") {
        layer: optional<str>,
        name: recommended<str>,
        r#loop: optional<bool>,
    },
    /// Stop a keyframe animation on a layer.
    StopKanim("stop_kanim") {
        layer: optional<str>,
    },
    /// Play a named keyframe animation on a character layer.
    Xanim("xanim") {
        layer: optional<str>,
        name: recommended<str>,
        r#loop: optional<bool>,
    },
    /// Stop a keyframe animation on a character layer.
    StopXanim("stop_xanim") {
        layer: optional<str>,
    },

    // ── Video / Movie ────────────────────────────────────────────────────────
    /// Play a video as the background.
    Bgmovie("bgmovie") {
        storage: required<str>,
        r#loop: optional<bool>,
        volume: optional<f32>,
    },
    /// Stop the background video.
    StopBgmovie("stop_bgmovie") {},
    /// Play a video as a foreground overlay.
    Movie("movie") {
        storage: required<str>,
        x: optional<f32>,
        y: optional<f32>,
        width: optional<f32>,
        height: optional<f32>,
    },

    // ── Transition ──────────────────────────────────────────────────────────
    /// Apply a visual transition.
    Trans("trans") {
        method: optional<str>,
        time: optional<u64>,
        rule: optional<str>,
    },
    /// Fade in from a solid color.
    Fadein("fadein") {
        time: optional<u64>,
        color: optional<str>,
    },
    /// Fade out to a solid color.
    Fadeout("fadeout") {
        time: optional<u64>,
        color: optional<str>,
    },
    /// Move-transition a layer.
    Movetrans("movetrans") {
        layer: optional<str>,
        time: optional<u64>,
        x: optional<f32>,
        y: optional<f32>,
    },

    // ── Effect ──────────────────────────────────────────────────────────────
    /// Screen quake effect.
    Quake("quake") {
        time: optional<u64>,
        hmax: optional<f32>,
        vmax: optional<f32>,
    },
    /// Screen shake effect.
    Shake("shake") {
        time: optional<u64>,
        amount: optional<f32>,
        axis: optional<str>,
    },
    /// Screen flash effect.
    Flash("flash") {
        time: optional<u64>,
        color: optional<str>,
    },

    // ── Message window ──────────────────────────────────────────────────────
    /// Control the message window.
    Msgwnd("msgwnd") {
        visible: optional<bool>,
        layer: optional<str>,
    },
    /// Set message window geometry.
    Wndctrl("wndctrl") {
        x: optional<f32>,
        y: optional<f32>,
        width: optional<f32>,
        height: optional<f32>,
    },
    /// Reset font to defaults.
    Resetfont("resetfont") {},
    /// Set font properties.
    Font("font") {
        face: optional<str>,
        size: optional<f32>,
        bold: optional<bool>,
        italic: optional<bool>,
    },
    /// Set font size.
    Size("size") {
        value: optional<f32>,
    },
    /// Set bold style.
    Bold("bold") {
        value: optional<bool>,
    },
    /// Set italic style.
    Italic("italic") {
        value: optional<bool>,
    },
    /// Set ruby (furigana) text.
    Ruby("ruby") {
        text: optional<str>,
    },
    /// Disable word wrapping.
    Nowrap("nowrap") {},
    /// Re-enable word wrapping.
    Endnowrap("endnowrap") {},

    // ── Character sprites ───────────────────────────────────────────────────
    /// Register a new character definition (no visual output).
    CharaNew("chara_new") {
        name: recommended<str>,
        storage: optional<str>,
        width: optional<f32>,
        height: optional<f32>,
    },
    /// Add a face/expression variant to a registered character.
    CharaFace("chara_face") {
        name: recommended<str>,
        face: recommended<str>,
        storage: recommended<str>,
    },
    /// Update configuration for a registered character.
    CharaConfig("chara_config") {
        name: recommended<str>,
    },
    /// Display a registered character on screen.
    CharaShow("chara_show") {
        name: recommended<str>,
        face: optional<str>,
        x: optional<f32>,
        y: optional<f32>,
        time: optional<u64>,
        method: optional<str>,
    },
    /// Hide a character with an optional exit transition.
    CharaHide("chara_hide") {
        name: recommended<str>,
        time: optional<u64>,
        method: optional<str>,
    },
    /// Hide all visible characters at once.
    CharaHideAll("chara_hide_all") {
        time: optional<u64>,
        method: optional<str>,
    },
    /// Unload a character sprite from memory.
    CharaFree("chara_free") {
        name: recommended<str>,
    },
    /// Remove a character definition from the registry.
    CharaDelete("chara_delete") {
        name: recommended<str>,
    },
    /// Change expression/pose of an on-screen character.
    CharaMod("chara_mod") {
        name: recommended<str>,
        face: optional<str>,
        pose: optional<str>,
        storage: optional<str>,
    },
    /// Animate a character to a new screen position.
    CharaMove("chara_move") {
        name: recommended<str>,
        x: optional<f32>,
        y: optional<f32>,
        time: optional<u64>,
    },
    /// Assign a character to a specific z-layer.
    CharaLayer("chara_layer") {
        name: recommended<str>,
        layer: recommended<str>,
    },
    /// Modify layer-level properties of an on-screen character.
    CharaLayerMod("chara_layer_mod") {
        name: recommended<str>,
        opacity: optional<f32>,
        visible: optional<bool>,
    },
    /// Set a compositable part on a character.
    CharaPart("chara_part") {
        name: recommended<str>,
        part: recommended<str>,
        storage: recommended<str>,
    },
    /// Reset all compositable parts of a character to defaults.
    CharaPartReset("chara_part_reset") {
        name: recommended<str>,
    },
    /// Set the character name shown in the ptext name box.
    CharaPtext("chara_ptext") {
        name: recommended<str>,
    }

    ;

    // ── Wait-for-completion group ───────────────────────────────────────────
    @wait_group {
        Wa("wa"),
        Wm("wm"),
        Wt("wt"),
        Wq("wq"),
        Wb("wb"),
        Wf("wf"),
        Wl("wl"),
        Ws("ws"),
        Wv("wv"),
        Wp("wp"),
        Wbgm("wbgm"),
        Wse("wse"),
        WaitBgmovie("wait_bgmovie"),
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::ast::{Param, ParamValue, Tag};
    use crate::error::Severity;

    fn span() -> SourceSpan {
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
        let play = KnownTag::from_tag(&tag_with_param("playSe", "storage", "beep.ogg"), &mut diags);
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
        let voice = KnownTag::from_tag(&tag_with_param("voice", "storage", "v01.ogg"), &mut diags);
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
    fn position_without_layer_is_ok() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("position"), &mut diags);
        assert_eq!(diags.len(), 0);
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
    fn chara_new_without_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("chara_new"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_new_with_name_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("chara_new", "name", "alice"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn chara_show_without_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("chara_show"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_show_with_name_is_clean() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_with_param("chara_show", "name", "alice"), &mut diags);
        assert!(diags.is_empty());
    }

    #[test]
    fn chara_hide_without_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("chara_hide"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_free_without_name_is_warning() {
        let mut diags = vec![];
        KnownTag::from_tag(&tag_no_params("chara_free"), &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_mod_without_name_is_warning() {
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

    // ── Metadata accessors ────────────────────────────────────────────────────

    #[test]
    fn tag_name_all_includes_aliases() {
        let all: Vec<_> = TagName::all().collect();
        assert!(all.contains(&TagName::Se));
        assert!(all.contains(&TagName::PlaySe));
        assert!(all.contains(&TagName::Vo));
        assert!(all.contains(&TagName::Voice));
        assert!(all.contains(&TagName::Wa));
        assert!(all.contains(&TagName::Bgm));
        assert!(all.contains(&TagName::Playbgm));
    }

    #[test]
    fn tag_name_canonical_maps_aliases() {
        assert_eq!(TagName::PlaySe.canonical(), TagName::Se);
        assert_eq!(TagName::Voice.canonical(), TagName::Vo);
        assert_eq!(TagName::Playbgm.canonical(), TagName::Bgm);
        assert_eq!(TagName::Bg.canonical(), TagName::Bg);
    }

    #[test]
    fn tag_name_param_names_for_bg() {
        assert_eq!(TagName::Bg.param_names(), &["storage", "time", "method"]);
    }

    #[test]
    fn tag_name_param_names_for_wait_group() {
        assert_eq!(TagName::Wa.param_names(), &["canskip", "buf"]);
    }

    #[test]
    fn tag_name_doc_summary_nonempty() {
        assert!(!TagName::Bg.doc_summary().is_empty());
        assert!(!TagName::Jump.doc_summary().is_empty());
    }

    #[test]
    fn tag_name_from_name_round_trips() {
        for name in &[
            "if",
            "elsif",
            "else",
            "endif",
            "ignore",
            "endignore",
            "jump",
            "call",
            "return",
            "link",
            "endlink",
            "glink",
            "eval",
            "emb",
            "trace",
            "l",
            "p",
            "r",
            "s",
            "cm",
            "er",
            "ch",
            "hch",
            "wait",
            "wc",
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
            "wbgm",
            "wse",
            "wait_bgmovie",
            "ct",
            "timeout",
            "waitclick",
            "cclick",
            "ctimeout",
            "cwheel",
            "click",
            "wheel",
            "nolog",
            "endnolog",
            "nowait",
            "endnowait",
            "resetdelay",
            "delay",
            "configdelay",
            "resetwait",
            "autowc",
            "pushlog",
            "input",
            "waittrig",
            "macro",
            "erasemacro",
            "endmacro",
            "clearvar",
            "clearsysvar",
            "clearstack",
            "clickskip",
            "bg",
            "image",
            "layopt",
            "free",
            "freeimage",
            "freelayer",
            "position",
            "backlay",
            "current",
            "locate",
            "layermode",
            "free_layermode",
            "filter",
            "free_filter",
            "position_filter",
            "mask",
            "mask_off",
            "graph",
            "bgm",
            "playbgm",
            "stopbgm",
            "fadeinbgm",
            "fadeoutbgm",
            "pausebgm",
            "resumebgm",
            "fadebgm",
            "xchgbgm",
            "bgmopt",
            "se",
            "playSe",
            "stopse",
            "pausese",
            "resumese",
            "seopt",
            "vo",
            "voice",
            "changevol",
            "bgmovie",
            "stop_bgmovie",
            "movie",
            "anim",
            "stopanim",
            "keyframe",
            "frame",
            "endkeyframe",
            "kanim",
            "stop_kanim",
            "xanim",
            "stop_xanim",
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
            "chara_new",
            "chara_face",
            "chara_config",
            "chara_show",
            "chara_hide",
            "chara_hide_all",
            "chara_free",
            "chara_delete",
            "chara_mod",
            "chara_move",
            "chara_layer",
            "chara_layer_mod",
            "chara_part",
            "chara_part_reset",
            "chara_ptext",
            "skipstart",
            "skipstop",
            "cancelskip",
            "start_keyconfig",
            "stop_keyconfig",
            "web",
            "showmenu",
            "showload",
            "showsave",
            "showlog",
            "hidemessage",
            "showmenubutton",
            "hidemenubutton",
            "speak_on",
            "speak_off",
            "glyph",
            "glyph_auto",
            "glyph_skip",
            "glink_config",
        ] {
            let tag_name = TagName::from_name(name)
                .unwrap_or_else(|| panic!("TagName::from_name({name:?}) returned None"));
            assert_eq!(
                tag_name.as_str(),
                *name,
                "TagName::as_str() did not round-trip for {name:?}"
            );
        }
    }

    #[test]
    fn tag_name_unknown_returns_none() {
        assert!(TagName::from_name("my_custom_tag").is_none());
        assert!(TagName::from_name("").is_none());
        assert!(TagName::from_name("JUMP").is_none());
        assert!(TagName::from_name("playse").is_none());
    }
}
