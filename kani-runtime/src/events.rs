//! All Bevy event types emitted or consumed by `kani-runtime`.
//!
//! Events are grouped into three categories:
//! 1. **Interpreter → Bevy** — emitted by `poll_interpreter` from `KagEvent`s.
//! 2. **Tag actions** — emitted by tag-handler systems from `KagEvent::Tag`.
//! 3. **Host → Interpreter** — sent by game code to drive the interpreter.

use bevy::prelude::Message;
use kag_interpreter::{ChoiceOption, InterpreterSnapshot, ResolvedTag, TextSpan};

// ─── 1. Interpreter → Bevy ───────────────────────────────────────────────────

/// Display a chunk of text in the message window.
///
/// `text` holds the plain text (XML tags stripped) for simple consumers.
/// `spans` holds the same content as styled [`TextSpan`] fragments for
/// rich-text rendering.
#[derive(Message, Debug, Clone)]
pub struct EvDisplayText {
    pub text: String,
    pub spans: Vec<TextSpan>,
    pub speaker: Option<String>,
    /// Per-character delay in ms (`None` = use host default).
    pub speed: Option<u64>,
    /// `false` inside `[nolog]…[endnolog]` blocks.
    pub log: bool,
}

/// Insert a line-break inside the current message window (`[r]`).
#[derive(Message, Debug, Clone, Copy)]
pub struct EvInsertLineBreak;

/// Clear the entire message window (`[cm]` / after `[p]`).
#[derive(Message, Debug, Clone, Copy)]
pub struct EvClearMessage;

/// Clear only the text of the current message layer (`[er]`).
#[derive(Message, Debug, Clone, Copy)]
pub struct EvClearCurrentMessage;

/// Present a set of choices to the player.
#[derive(Message, Debug, Clone)]
pub struct EvBeginChoices(pub Vec<ChoiceOption>);

/// Show a text-input dialog (`[input]`).
#[derive(Message, Debug, Clone)]
pub struct EvInputRequested {
    /// Variable the result will be stored in (e.g. `"f.username"`).
    pub name: String,
    pub prompt: String,
    pub title: String,
}

/// Inline expression result (`[emb exp=…]`).
#[derive(Message, Debug, Clone)]
pub struct EvEmbedText(pub String);

/// Push an entry into the backlog (`[pushlog]`).
#[derive(Message, Debug, Clone)]
pub struct EvPushBacklog {
    pub text: String,
    pub join: bool,
}

/// Full interpreter snapshot response.
#[derive(Message, Debug, Clone)]
pub struct EvSnapshot(pub Box<InterpreterSnapshot>);

/// Internal routing event: every `KagEvent::Tag` is emitted here first.
///
/// All built-in tag-handler systems match on the inner [`ResolvedTag`] to
/// dispatch strongly-typed Bevy events.  Game-specific code matches on
/// `ResolvedTag::Extension` to handle custom tags.
#[derive(Message, Debug, Clone)]
pub struct EvTagRouted(pub ResolvedTag);

// ─── 2a. Image / layer tag actions ───────────────────────────────────────────

/// Set the background image (`[bg storage=… time=… method=…]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetBackground {
    pub storage: String,
    pub time: Option<u64>,
    pub method: Option<String>,
}

/// Spawn or update an image layer (`[image]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetImageLayer {
    pub storage: String,
    pub layer: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub visible: Option<bool>,
}

/// Modify an existing layer's visibility/opacity (`[layopt]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetLayerOpt {
    pub layer: String,
    pub visible: Option<bool>,
    pub opacity: Option<f32>,
}

/// Despawn a layer (`[free layer=…]`).
#[derive(Message, Debug, Clone)]
pub struct EvFreeLayer {
    pub layer: String,
}

/// Move a layer to a new position (`[position]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetLayerPosition {
    pub layer: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

/// Copy the front layer to the back layer (`[backlay]`).
#[derive(Message, Debug, Clone, Copy)]
pub struct EvBacklay;

/// Set the active message/text layer (`[current]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetCurrentLayer {
    pub layer: Option<String>,
}

/// Position the text cursor within the current message layer (`[locate]`).
#[derive(Message, Debug, Clone)]
pub struct EvLocateCursor {
    pub x: Option<f32>,
    pub y: Option<f32>,
}

/// Set blend mode on a layer (`[layermode]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetLayerMode {
    pub layer: Option<String>,
    pub mode: Option<String>,
}

/// Reset blend mode to Normal (`[free_layermode]`).
#[derive(Message, Debug, Clone)]
pub struct EvResetLayerMode {
    pub layer: Option<String>,
}

/// Apply a shader effect to a layer (`[filter]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetFilter {
    pub layer: Option<String>,
    pub filter_type: Option<String>,
}

/// Remove a filter from a layer (`[free_filter]`).
#[derive(Message, Debug, Clone)]
pub struct EvFreeFilter {
    pub layer: Option<String>,
}

/// Position a filter within its layer (`[position_filter]`).
#[derive(Message, Debug, Clone)]
pub struct EvPositionFilter {
    pub layer: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

/// Apply an alpha mask image to a layer (`[mask]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetMask {
    pub layer: Option<String>,
    pub storage: Option<String>,
}

/// Remove a mask from a layer (`[mask_off]`).
#[derive(Message, Debug, Clone)]
pub struct EvRemoveMask {
    pub layer: Option<String>,
}

/// Draw a primitive shape on a layer (`[graph]`).
#[derive(Message, Debug, Clone)]
pub struct EvDrawGraph {
    pub layer: Option<String>,
    pub shape: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub color: Option<String>,
}

// ─── 2b. Audio tag actions ────────────────────────────────────────────────────

/// Play or crossfade BGM (`[bgm]`).
#[derive(Message, Debug, Clone)]
pub struct EvPlayBgm {
    pub storage: String,
    pub looping: bool,
    pub volume: Option<f32>,
    pub fadetime: Option<u64>,
}

/// Stop BGM, optionally fading out (`[stopbgm]`).
#[derive(Message, Debug, Clone)]
pub struct EvStopBgm {
    pub fadetime: Option<u64>,
}

/// Play a sound effect on a numbered buffer (`[se]` / `[playSe]`).
#[derive(Message, Debug, Clone)]
pub struct EvPlaySe {
    pub storage: String,
    pub buf: Option<u32>,
    pub volume: Option<f32>,
    pub looping: bool,
}

/// Stop a sound-effect buffer (`[stopse]`).
#[derive(Message, Debug, Clone)]
pub struct EvStopSe {
    pub buf: Option<u32>,
}

/// Play a voice clip (`[vo]` / `[voice]`).
#[derive(Message, Debug, Clone)]
pub struct EvPlayVoice {
    pub storage: String,
    pub buf: Option<u32>,
}

/// Fade BGM to a target volume over time (`[fadebgm]`).
#[derive(Message, Debug, Clone)]
pub struct EvFadeBgm {
    pub time: Option<u64>,
    pub volume: Option<f32>,
}

/// Pause BGM at the current seek position (`[pausebgm]`).
#[derive(Message, Debug, Clone)]
pub struct EvPauseBgm {
    pub buf: Option<u32>,
}

/// Resume paused BGM from the saved seek (`[resumebgm]`).
#[derive(Message, Debug, Clone)]
pub struct EvResumeBgm {
    pub buf: Option<u32>,
}

/// Cross-fade to a new BGM track (`[xchgbgm]`).
#[derive(Message, Debug, Clone)]
pub struct EvCrossFadeBgm {
    pub storage: String,
    pub time: Option<u64>,
}

/// Change options on the currently-playing BGM (`[bgmopt]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetBgmOpt {
    pub looping: Option<bool>,
    pub seek: Option<String>,
}

/// Pause a sound-effect buffer (`[pausese]`).
#[derive(Message, Debug, Clone)]
pub struct EvPauseSe {
    pub buf: Option<u32>,
}

/// Resume a paused sound-effect buffer (`[resumese]`).
#[derive(Message, Debug, Clone)]
pub struct EvResumeSe {
    pub buf: Option<u32>,
}

/// Change options on a currently-playing SE buffer (`[seopt]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetSeOpt {
    pub buf: Option<u32>,
    pub looping: Option<bool>,
}

/// Set the volume for a channel with optional fade (`[changevol]`).
#[derive(Message, Debug, Clone)]
pub struct EvChangeVol {
    pub target: Option<String>,
    pub vol: Option<f32>,
    pub time: Option<u64>,
}

// ─── 2b1. Animation tag actions ───────────────────────────────────────────────

pub use kag_interpreter::FrameSpec;

/// Play a preset animation on a layer (`[anim]`).
#[derive(Message, Debug, Clone)]
pub struct EvPlayAnim {
    pub layer: Option<String>,
    pub preset: Option<String>,
    pub time: Option<u64>,
    pub looping: bool,
    pub delay: Option<u64>,
}

/// Stop the animation on a layer (`[stopanim]`).
#[derive(Message, Debug, Clone)]
pub struct EvStopAnim {
    pub layer: Option<String>,
}

/// Play a keyframe animation on a layer (`[kanim]`).
#[derive(Message, Debug, Clone)]
pub struct EvPlayKanim {
    pub layer: Option<String>,
    pub frames: Vec<FrameSpec>,
    pub looping: bool,
}

/// Stop a keyframe animation on a layer (`[stop_kanim]`).
#[derive(Message, Debug, Clone)]
pub struct EvStopKanim {
    pub layer: Option<String>,
}

/// Play a keyframe animation on a character layer (`[xanim]`).
#[derive(Message, Debug, Clone)]
pub struct EvPlayXanim {
    pub layer: Option<String>,
    pub frames: Vec<FrameSpec>,
    pub looping: bool,
}

/// Stop a keyframe animation on a character layer (`[stop_xanim]`).
#[derive(Message, Debug, Clone)]
pub struct EvStopXanim {
    pub layer: Option<String>,
}

// ─── 2b2. Video / Movie tag actions ──────────────────────────────────────────

/// Play a video as the background (`[bgmovie]`).
#[derive(Message, Debug, Clone)]
pub struct EvPlayBgMovie {
    pub storage: String,
    pub looping: bool,
    pub volume: Option<f32>,
}

/// Stop the background video (`[stop_bgmovie]`).
#[derive(Message, Debug, Clone, Copy)]
pub struct EvStopBgMovie;

/// Play a video as a foreground overlay (`[movie]`).
#[derive(Message, Debug, Clone)]
pub struct EvPlayMovie {
    pub storage: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

// ─── 2c. Transition tag actions ───────────────────────────────────────────────

/// Run a scene transition (`[trans]`).
#[derive(Message, Debug, Clone)]
pub struct EvRunTransition {
    pub method: Option<String>,
    pub time: Option<u64>,
    pub rule: Option<String>,
}

/// Fade the screen in or out (`[fadein]` / `[fadeout]`).
#[derive(Message, Debug, Clone)]
pub struct EvFadeScreen {
    /// `"fadein"` or `"fadeout"`.
    pub kind: String,
    pub time: Option<u64>,
    pub color: Option<String>,
}

/// Translate a layer during a transition (`[movetrans]`).
#[derive(Message, Debug, Clone)]
pub struct EvMoveLayerTransition {
    pub layer: Option<String>,
    pub time: Option<u64>,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

// ─── 2d. Effect tag actions ───────────────────────────────────────────────────

/// Camera quake effect (`[quake]`).
#[derive(Message, Debug, Clone)]
pub struct EvQuake {
    pub time: Option<u64>,
    pub hmax: Option<f32>,
    pub vmax: Option<f32>,
}

/// Directional shake (`[shake]`).
#[derive(Message, Debug, Clone)]
pub struct EvShake {
    pub time: Option<u64>,
    pub amount: Option<f32>,
    pub axis: Option<String>,
}

/// Screen flash (`[flash]`).
#[derive(Message, Debug, Clone)]
pub struct EvFlash {
    pub time: Option<u64>,
    pub color: Option<String>,
}

// ─── 2e. Message-window tag actions ──────────────────────────────────────────

/// Show/hide/configure the message window (`[msgwnd]`).
#[derive(Message, Debug, Clone)]
pub struct EvMessageWindow {
    pub visible: Option<bool>,
    pub layer: Option<String>,
}

/// Resize/reposition the message window (`[wndctrl]`).
#[derive(Message, Debug, Clone)]
pub struct EvWindowControl {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// Reset font/text style to defaults (`[resetfont]`).
#[derive(Message, Debug, Clone, Copy)]
pub struct EvResetFont;

/// Update text style properties (`[font]` / `[size]` / `[bold]` / `[italic]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetFont {
    pub face: Option<String>,
    pub size: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
}

/// Set furigana annotation (`[ruby]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetRuby {
    pub text: Option<String>,
}

/// Enable or disable text wrapping (`[nowrap]` / `[endnowrap]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetNowrap {
    pub enabled: bool,
}

// ─── 2f. Character sprite tag actions ────────────────────────────────────────

/// Show a character on screen (`[chara_show]`).
/// `storage` is already resolved against the character registry.
#[derive(Message, Debug, Clone)]
pub struct EvShowCharacter {
    pub name: Option<String>,
    pub storage: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub time: Option<u64>,
    pub method: Option<String>,
}

/// Hide a character (`[chara_hide]`).
#[derive(Message, Debug, Clone)]
pub struct EvHideCharacter {
    pub name: Option<String>,
    pub time: Option<u64>,
    pub method: Option<String>,
}

/// Hide all visible characters (`[chara_hide_all]`).
#[derive(Message, Debug, Clone)]
pub struct EvHideAllCharacters {
    pub time: Option<u64>,
    pub method: Option<String>,
}

/// Unload a character sprite from memory (`[chara_free]`).
#[derive(Message, Debug, Clone)]
pub struct EvFreeCharacter {
    pub name: Option<String>,
}

/// Signal that a character definition was deleted (`[chara_delete]`).
#[derive(Message, Debug, Clone)]
pub struct EvDeleteCharacter {
    pub name: Option<String>,
}

/// Update a character's expression/pose (`[chara_mod]`).
/// `storage` is already resolved against the registry.
#[derive(Message, Debug, Clone)]
pub struct EvModCharacter {
    pub name: Option<String>,
    pub storage: Option<String>,
    pub face: Option<String>,
    pub pose: Option<String>,
}

/// Move a character to a new position (`[chara_move]`).
#[derive(Message, Debug, Clone)]
pub struct EvMoveCharacter {
    pub name: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub time: Option<u64>,
}

/// Assign a character to a z-layer (`[chara_layer]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetCharacterLayer {
    pub name: Option<String>,
    pub layer: Option<String>,
}

/// Modify layer properties of a character (`[chara_layer_mod]`).
#[derive(Message, Debug, Clone)]
pub struct EvModCharacterLayer {
    pub name: Option<String>,
    pub opacity: Option<f32>,
    pub visible: Option<bool>,
}

/// Set a compositable part on a character (`[chara_part]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetCharacterPart {
    pub name: Option<String>,
    pub part: Option<String>,
    pub storage: Option<String>,
}

/// Reset all parts of a character to defaults (`[chara_part_reset]`).
#[derive(Message, Debug, Clone)]
pub struct EvResetCharacterParts {
    pub name: Option<String>,
}

// ─── 2f2. Skip / Key config actions ──────────────────────────────────────────

/// Skip mode was enabled or disabled (`[skipstart]` / `[skipstop]`).
#[derive(Message, Debug, Clone)]
pub struct EvSkipMode {
    pub enabled: bool,
}

/// Key-config UI was opened or closed (`[start_keyconfig]` / `[stop_keyconfig]`).
#[derive(Message, Debug, Clone)]
pub struct EvKeyConfig {
    pub open: bool,
}

// ─── 2g. UI tag actions ───────────────────────────────────────────────────────

/// Spawn a clickable button widget (`[button]`).
#[derive(Message, Debug, Clone)]
pub struct EvSpawnButton {
    pub text: Option<String>,
    pub graphic: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub bg: Option<String>,
    pub hover_bg: Option<String>,
    pub press_bg: Option<String>,
    pub color: Option<String>,
    pub font_size: Option<f32>,
    pub target: Option<String>,
    pub storage: Option<String>,
    pub exp: Option<String>,
    pub key: Option<String>,
    pub visible: Option<bool>,
    pub opacity: Option<f32>,
}

/// Make a layer respond to click events (`[clickable]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetClickable {
    pub layer: Option<String>,
    pub target: Option<String>,
    pub storage: Option<String>,
    pub exp: Option<String>,
}

/// Open or control a built-in UI panel (`[showmenu]`, `[showload]`, …).
///
/// `kind` is one of: `"menu"`, `"load"`, `"save"`, `"log"`,
/// `"hidemessage"`, `"showmenubutton"`, `"hidemenubutton"`.
#[derive(Message, Debug, Clone)]
pub struct EvOpenPanel {
    pub kind: String,
}

/// Display a modal dialog box (`[dialog]`).
#[derive(Message, Debug, Clone)]
pub struct EvShowDialog {
    pub text: Option<String>,
    pub title: Option<String>,
}

/// Change the mouse cursor image (`[cursor]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetCursor {
    pub storage: Option<String>,
}

/// Toggle the speaker name box (`[speak_on]` / `[speak_off]`).
#[derive(Message, Debug, Clone)]
pub struct EvSetSpeakerBoxVisible {
    pub visible: bool,
}

/// Configure a glyph image (`[glyph]`, `[glyph_auto]`, `[glyph_skip]`).
///
/// `kind` is `"default"`, `"auto"`, or `"skip"`.
#[derive(Message, Debug, Clone)]
pub struct EvSetGlyph {
    pub kind: String,
    pub storage: Option<String>,
}

/// Visual effect for mode changes (`[mode_effect]`).
#[derive(Message, Debug, Clone)]
pub struct EvModeEffect {
    pub mode: Option<String>,
    pub effect: Option<String>,
}

// ─── 2h. Misc tag actions ─────────────────────────────────────────────────────

/// Open a URL in the system browser (`[web]`).
#[derive(Message, Debug, Clone)]
pub struct EvOpenUrl {
    pub url: String,
}

// ─── 3. Host → Interpreter ───────────────────────────────────────────────────

/// Player selected a choice (index into the last `EvBeginChoices`).
#[derive(Message, Debug, Clone)]
pub struct EvSelectChoice(pub usize);

/// Player submitted a text-input value.
#[derive(Message, Debug, Clone)]
pub struct EvSubmitInput(pub String);

/// A named trigger was fired by game code.
#[derive(Message, Debug, Clone)]
pub struct EvFireTrigger {
    pub name: String,
}

/// An async operation (animation, audio, transition) completed.
///
/// Emit this to unblock a `WaitForCompletion` / `[wa]`-family wait.
#[derive(Message, Debug, Clone, Copy)]
pub struct EvCompletionSignal;
