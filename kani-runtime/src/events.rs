//! All Bevy event types emitted or consumed by `kani-runtime`.
//!
//! Events are grouped into three categories:
//! 1. **Interpreter → Bevy** — emitted by `poll_interpreter` from `KagEvent`s.
//! 2. **Tag actions** — emitted by tag-handler systems from `KagEvent::Tag`.
//! 3. **Host → Interpreter** — sent by game code to drive the interpreter.

use bevy::prelude::Event;
use kag_interpreter::{ChoiceOption, InterpreterSnapshot};

// ─── 1. Interpreter → Bevy ───────────────────────────────────────────────────

/// Display a chunk of text in the message window.
#[derive(Event, Debug, Clone)]
pub struct EvDisplayText {
    pub text: String,
    pub speaker: Option<String>,
    /// Per-character delay in ms (`None` = use host default).
    pub speed: Option<u64>,
    /// `false` inside `[nolog]…[endnolog]` blocks.
    pub log: bool,
}

/// Insert a line-break inside the current message window (`[r]`).
#[derive(Event, Debug, Clone, Copy)]
pub struct EvInsertLineBreak;

/// Clear the entire message window (`[cm]` / after `[p]`).
#[derive(Event, Debug, Clone, Copy)]
pub struct EvClearMessage;

/// Clear only the text of the current message layer (`[er]`).
#[derive(Event, Debug, Clone, Copy)]
pub struct EvClearCurrentMessage;

/// Present a set of choices to the player.
#[derive(Event, Debug, Clone)]
pub struct EvBeginChoices(pub Vec<ChoiceOption>);

/// Show a text-input dialog (`[input]`).
#[derive(Event, Debug, Clone)]
pub struct EvInputRequested {
    /// Variable the result will be stored in (e.g. `"f.username"`).
    pub name: String,
    pub prompt: String,
    pub title: String,
}

/// Inline expression result (`[emb exp=…]`).
#[derive(Event, Debug, Clone)]
pub struct EvEmbedText(pub String);

/// Push an entry into the backlog (`[pushlog]`).
#[derive(Event, Debug, Clone)]
pub struct EvPushBacklog {
    pub text: String,
    pub join: bool,
}

/// Full interpreter snapshot response.
#[derive(Event, Debug, Clone)]
pub struct EvSnapshot(pub Box<InterpreterSnapshot>);

/// Internal routing event: every `KagEvent::Tag` is emitted here first.
///
/// All built-in tag-handler systems read this event.  Game-specific code
/// should listen to [`EvUnknownTag`] instead, which is emitted only for tags
/// that are not claimed by any built-in handler.
#[derive(Event, Debug, Clone)]
pub struct EvTagRouted {
    pub name: String,
    pub params: Vec<(String, String)>,
}

/// A `KagEvent::Tag` that was not matched by any built-in handler.
///
/// Game-specific code can listen for this to implement custom tags.
#[derive(Event, Debug, Clone)]
pub struct EvUnknownTag {
    pub name: String,
    pub params: Vec<(String, String)>,
}

// ─── 2a. Image / layer tag actions ───────────────────────────────────────────

/// Set the background image (`[bg storage=… time=… method=…]`).
#[derive(Event, Debug, Clone)]
pub struct EvSetBackground {
    pub storage: String,
    pub time: Option<u64>,
    pub method: Option<String>,
}

/// Spawn or update an image layer (`[image]`).
#[derive(Event, Debug, Clone)]
pub struct EvSetImageLayer {
    pub storage: String,
    pub layer: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub visible: Option<bool>,
}

/// Modify an existing layer's visibility/opacity (`[layopt]`).
#[derive(Event, Debug, Clone)]
pub struct EvSetLayerOpt {
    pub layer: String,
    pub visible: Option<bool>,
    pub opacity: Option<f32>,
}

/// Despawn a layer (`[free layer=…]`).
#[derive(Event, Debug, Clone)]
pub struct EvFreeLayer {
    pub layer: String,
}

/// Move a layer to a new position (`[position]`).
#[derive(Event, Debug, Clone)]
pub struct EvSetLayerPosition {
    pub layer: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

// ─── 2b. Audio tag actions ────────────────────────────────────────────────────

/// Play or crossfade BGM (`[bgm]`).
#[derive(Event, Debug, Clone)]
pub struct EvPlayBgm {
    pub storage: String,
    pub looping: bool,
    pub volume: Option<f32>,
    pub fadetime: Option<u64>,
}

/// Stop BGM, optionally fading out (`[stopbgm]`).
#[derive(Event, Debug, Clone)]
pub struct EvStopBgm {
    pub fadetime: Option<u64>,
}

/// Play a sound effect on a numbered buffer (`[se]` / `[playSe]`).
#[derive(Event, Debug, Clone)]
pub struct EvPlaySe {
    pub storage: String,
    pub buf: Option<u32>,
    pub volume: Option<f32>,
    pub looping: bool,
}

/// Stop a sound-effect buffer (`[stopse]`).
#[derive(Event, Debug, Clone)]
pub struct EvStopSe {
    pub buf: Option<u32>,
}

/// Play a voice clip (`[vo]` / `[voice]`).
#[derive(Event, Debug, Clone)]
pub struct EvPlayVoice {
    pub storage: String,
    pub buf: Option<u32>,
}

/// Fade BGM to a target volume over time (`[fadebgm]`).
#[derive(Event, Debug, Clone)]
pub struct EvFadeBgm {
    pub time: Option<u64>,
    pub volume: Option<f32>,
}

// ─── 2c. Transition tag actions ───────────────────────────────────────────────

/// Run a scene transition (`[trans]`).
#[derive(Event, Debug, Clone)]
pub struct EvRunTransition {
    pub method: Option<String>,
    pub time: Option<u64>,
    pub rule: Option<String>,
}

/// Fade the screen in or out (`[fadein]` / `[fadeout]`).
#[derive(Event, Debug, Clone)]
pub struct EvFadeScreen {
    /// `"fadein"` or `"fadeout"`.
    pub kind: String,
    pub time: Option<u64>,
    pub color: Option<String>,
}

/// Translate a layer during a transition (`[movetrans]`).
#[derive(Event, Debug, Clone)]
pub struct EvMoveLayerTransition {
    pub layer: Option<String>,
    pub time: Option<u64>,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

// ─── 2d. Effect tag actions ───────────────────────────────────────────────────

/// Camera quake effect (`[quake]`).
#[derive(Event, Debug, Clone)]
pub struct EvQuake {
    pub time: Option<u64>,
    pub hmax: Option<f32>,
    pub vmax: Option<f32>,
}

/// Directional shake (`[shake]`).
#[derive(Event, Debug, Clone)]
pub struct EvShake {
    pub time: Option<u64>,
    pub amount: Option<f32>,
    pub axis: Option<String>,
}

/// Screen flash (`[flash]`).
#[derive(Event, Debug, Clone)]
pub struct EvFlash {
    pub time: Option<u64>,
    pub color: Option<String>,
}

// ─── 2e. Message-window tag actions ──────────────────────────────────────────

/// Show/hide/configure the message window (`[msgwnd]`).
#[derive(Event, Debug, Clone)]
pub struct EvMessageWindow {
    pub visible: Option<bool>,
    pub layer: Option<String>,
}

/// Resize/reposition the message window (`[wndctrl]`).
#[derive(Event, Debug, Clone)]
pub struct EvWindowControl {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// Reset font/text style to defaults (`[resetfont]`).
#[derive(Event, Debug, Clone, Copy)]
pub struct EvResetFont;

/// Update text style properties (`[font]` / `[size]` / `[bold]` / `[italic]`).
#[derive(Event, Debug, Clone)]
pub struct EvSetFont {
    pub face: Option<String>,
    pub size: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
}

/// Set furigana annotation (`[ruby]`).
#[derive(Event, Debug, Clone)]
pub struct EvSetRuby {
    pub text: Option<String>,
}

/// Enable or disable text wrapping (`[nowrap]` / `[endnowrap]`).
#[derive(Event, Debug, Clone)]
pub struct EvSetNowrap {
    pub enabled: bool,
}

// ─── 2f. Character sprite tag actions ────────────────────────────────────────

/// Show or update a character sprite (`[chara]`).
#[derive(Event, Debug, Clone)]
pub struct EvSetCharacter {
    pub id: Option<String>,
    pub storage: Option<String>,
    pub slot: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

/// Hide a character sprite (`[chara_hide]`).
#[derive(Event, Debug, Clone)]
pub struct EvHideCharacter {
    pub id: Option<String>,
    pub slot: Option<String>,
}

/// Despawn a character sprite (`[chara_free]`).
#[derive(Event, Debug, Clone)]
pub struct EvFreeCharacter {
    pub id: Option<String>,
    pub slot: Option<String>,
}

/// Update a character sprite variant (`[chara_mod]`).
#[derive(Event, Debug, Clone)]
pub struct EvModCharacter {
    pub id: Option<String>,
    pub face: Option<String>,
    pub pose: Option<String>,
    pub storage: Option<String>,
}

// ─── 3. Host → Interpreter ───────────────────────────────────────────────────

/// Player selected a choice (index into the last `EvBeginChoices`).
#[derive(Event, Debug, Clone)]
pub struct EvSelectChoice(pub usize);

/// Player submitted a text-input value.
#[derive(Event, Debug, Clone)]
pub struct EvSubmitInput(pub String);

/// A named trigger was fired by game code.
#[derive(Event, Debug, Clone)]
pub struct EvFireTrigger {
    pub name: String,
}

/// An async operation (animation, audio, transition) completed.
///
/// Emit this to unblock a `WaitForCompletion` / `[wa]`-family wait.
#[derive(Event, Debug, Clone, Copy)]
pub struct EvCompletionSignal;
