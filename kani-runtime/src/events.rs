//! All Bevy event types emitted or consumed by `kani-runtime`.
//!
//! Events are grouped into three categories:
//! 1. **Interpreter → Bevy** — emitted by `poll_interpreter` from `KagEvent`s.
//! 2. **Tag actions** — emitted by tag-handler systems from `KagEvent::Tag`.
//! 3. **Host → Interpreter** — sent by game code to drive the interpreter.

use bevy::prelude::Message;
use kag_interpreter::{ChoiceOption, InterpreterSnapshot, ResolvedTag, TextSpan};

pub use kag_interpreter::FrameSpec;

// ─── 1. Interpreter → Bevy ───────────────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvInterpreterCall {
    /// Display a chunk of text in the message window.
    ///
    /// `text` holds the plain text (XML tags stripped) for simple consumers.
    /// `spans` holds the same content as styled [`TextSpan`] fragments for
    /// rich-text rendering.
    DisplayText {
        text: String,
        spans: Vec<TextSpan>,
        speaker: Option<String>,
        /// Per-character delay in ms (`None` = use host default).
        speed: Option<u64>,
        /// `false` inside `[nolog]…[endnolog]` blocks.
        log: bool,
    },
    /// Insert a line-break inside the current message window (`[r]`).
    InsertLineBreak,
    /// Clear the entire message window (`[cm]` / after `[p]`).
    ClearMessage,
    /// Clear only the text of the current message layer (`[er]`).
    ClearCurrentMessage,
    /// Present a set of choices to the player.
    BeginChoice(Vec<ChoiceOption>),
    /// Show a text-input dialog (`[input]`).
    InputRequested {
        /// Variable the result will be stored in (e.g. `"f.username"`).
        name: String,
        prompt: String,
        title: String,
    },
    /// Inline expression result (`[emb exp=…]`).
    EmbedTest(String),
    /// Push an entry into the backlog (`[pushlog]`).
    PushBacklog {
        text: String,
        join: bool,
    },
    Snapshot(Box<InterpreterSnapshot>),
    TagRouted(EvTagRouted)
}

#[derive(Message, Debug, Clone)]
pub struct EvTagRouted(pub ResolvedTag);

// ─── 2a. Image / layer tag actions ───────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvLayerTag {
    /// Set the background image (`[bg storage=… time=… method=…]`).
    SetBackground {
        storage: String,
        time: Option<u64>,
        method: Option<String>,
    },
    /// Spawn or update an image layer (`[image]`).
    SetImageLayer {
        storage: String,
        layer: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        visible: Option<bool>,
    },
    /// Modify an existing layer's visibility/opacity (`[layopt]`).
    SetLayerOpt {
        layer: String,
        visible: Option<bool>,
        opacity: Option<f32>,
    },
    /// Despawn a layer (`[free layer=…]`).
    FreeLayer { layer: String },
    /// Move a layer to a new position (`[position]`).
    SetLayerPosition {
        layer: String,
        x: Option<f32>,
        y: Option<f32>,
    },
    /// Copy the front layer to the back layer (`[backlay]`).
    Backlay,
    /// Set the active message/text layer (`[current]`).
    SetCurrentLayer { layer: Option<String> },
    /// Position the text cursor within the current message layer (`[locate]`).
    LocateCursor { x: Option<f32>, y: Option<f32> },
    /// Set blend mode on a layer (`[layermode]`).
    SetLayerMode {
        layer: Option<String>,
        mode: Option<String>,
    },
    /// Reset blend mode to Normal (`[free_layermode]`).
    ResetLayerMode { layer: Option<String> },
    /// Apply a shader effect to a layer (`[filter]`).
    SetFilter {
        layer: Option<String>,
        filter_type: Option<String>,
    },
    /// Remove a filter from a layer (`[free_filter]`).
    FreeFilter { layer: Option<String> },
    /// Position a filter within its layer (`[position_filter]`).
    PositionFilter {
        layer: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
    },
    /// Apply an alpha mask image to a layer (`[mask]`).
    SetMask {
        layer: Option<String>,
        storage: Option<String>,
    },
    /// Remove a mask from a layer (`[mask_off]`).
    RemoveMask { layer: Option<String> },
    /// Draw a primitive shape on a layer (`[graph]`).
    DrawGraph {
        layer: Option<String>,
        shape: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        width: Option<f32>,
        height: Option<f32>,
        color: Option<String>,
    },
}

// ─── 2b. Audio tag actions ────────────────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvAudioTag {
    /// Play or crossfade BGM (`[bgm]`).
    PlayBgm {
        storage: String,
        looping: bool,
        volume: Option<f32>,
        fadetime: Option<u64>,
    },
    /// Stop BGM, optionally fading out (`[stopbgm]`).
    StopBgm { fadetime: Option<u64> },
    /// Play a sound effect on a numbered buffer (`[se]` / `[playSe]`).
    PlaySe {
        storage: String,
        buf: Option<u32>,
        volume: Option<f32>,
        looping: bool,
    },
    /// Stop a sound-effect buffer (`[stopse]`).
    StopSe { buf: Option<u32> },
    /// Play a voice clip (`[vo]` / `[voice]`).
    PlayVoice { storage: String, buf: Option<u32> },
    /// Fade BGM to a target volume over time (`[fadebgm]`).
    FadeBgm {
        time: Option<u64>,
        volume: Option<f32>,
    },
    /// Pause BGM at the current seek position (`[pausebgm]`).
    PauseBgm { buf: Option<u32> },
    /// Resume paused BGM from the saved seek (`[resumebgm]`).
    ResumeBgm { buf: Option<u32> },
    /// Cross-fade to a new BGM track (`[xchgbgm]`).
    CrossFadeBgm { storage: String, time: Option<u64> },
    /// Change options on the currently-playing BGM (`[bgmopt]`).
    SetBgmOpt {
        looping: Option<bool>,
        seek: Option<String>,
    },
    /// Pause a sound-effect buffer (`[pausese]`).
    PauseSe { buf: Option<u32> },
    /// Resume a paused sound-effect buffer (`[resumese]`).
    ResumeSe { buf: Option<u32> },
    /// Change options on a currently-playing SE buffer (`[seopt]`).
    SetSeOpt {
        buf: Option<u32>,
        looping: Option<bool>,
    },
    /// Set the volume for a channel with optional fade (`[changevol]`).
    ChangeVol {
        target: Option<String>,
        vol: Option<f32>,
        time: Option<u64>,
    },
}

// ─── 2b1. Animation tag actions ───────────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvAnimTag {
    /// Play a preset animation on a layer (`[anim]`).
    PlayAnim {
        layer: Option<String>,
        preset: Option<String>,
        time: Option<u64>,
        looping: bool,
        delay: Option<u64>,
    },
    /// Stop the animation on a layer (`[stopanim]`).
    StopAnim { layer: Option<String> },
    /// Play a keyframe animation on a layer (`[kanim]`).
    PlayKanim {
        layer: Option<String>,
        frames: Vec<FrameSpec>,
        looping: bool,
    },
    /// Stop a keyframe animation on a layer (`[stop_kanim]`).
    StopKanim { layer: Option<String> },
    /// Play a keyframe animation on a character layer (`[xanim]`).
    PlayXanim {
        layer: Option<String>,
        frames: Vec<FrameSpec>,
        looping: bool,
    },
    /// Stop a keyframe animation on a character layer (`[stop_xanim]`).
    StopXanim { layer: Option<String> },
}

// ─── 2b2. Video / Movie tag actions ──────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvVideoTag {
    /// Play a video as the background (`[bgmovie]`).
    PlayBgMovie {
        storage: String,
        looping: bool,
        volume: Option<f32>,
    },
    /// Stop the background video (`[stop_bgmovie]`).
    StopBgMovie,
    /// Play a video as a foreground overlay (`[movie]`).
    PlayMovie {
        storage: String,
        x: Option<f32>,
        y: Option<f32>,
        width: Option<f32>,
        height: Option<f32>,
    },
}

// ─── 2c. Transition tag actions ───────────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvTransitionTag {
    /// Run a scene transition (`[trans]`).
    RunTransition {
        method: Option<String>,
        time: Option<u64>,
        rule: Option<String>,
    },
    /// Fade the screen in or out (`[fadein]` / `[fadeout]`).
    ///
    /// `kind` is `"fadein"` or `"fadeout"`.
    FadeScreen {
        kind: String,
        time: Option<u64>,
        color: Option<String>,
    },
    /// Translate a layer during a transition (`[movetrans]`).
    MoveLayerTransition {
        layer: Option<String>,
        time: Option<u64>,
        x: Option<f32>,
        y: Option<f32>,
    },
}

// ─── 2d. Effect tag actions ───────────────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvEffectTag {
    /// Camera quake effect (`[quake]`).
    Quake {
        time: Option<u64>,
        hmax: Option<f32>,
        vmax: Option<f32>,
    },
    /// Directional shake (`[shake]`).
    Shake {
        time: Option<u64>,
        amount: Option<f32>,
        axis: Option<String>,
    },
    /// Screen flash (`[flash]`).
    Flash {
        time: Option<u64>,
        color: Option<String>,
    },
}

// ─── 2e. Message-window tag actions ──────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvMessageWindowTag {
    /// Show/hide/configure the message window (`[msgwnd]`).
    MessageWindow {
        visible: Option<bool>,
        layer: Option<String>,
    },
    /// Resize/reposition the message window (`[wndctrl]`).
    WindowControl {
        x: Option<f32>,
        y: Option<f32>,
        width: Option<f32>,
        height: Option<f32>,
    },
    /// Reset font/text style to defaults (`[resetfont]`).
    ResetFont,
    /// Update text style properties (`[font]` / `[size]` / `[bold]` / `[italic]`).
    SetFont {
        face: Option<String>,
        size: Option<f32>,
        bold: Option<bool>,
        italic: Option<bool>,
    },
    /// Set furigana annotation (`[ruby]`).
    SetRuby { text: Option<String> },
    /// Enable or disable text wrapping (`[nowrap]` / `[endnowrap]`).
    SetNowrap { enabled: bool },
}

// ─── 2f. Character sprite tag actions ────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvCharacterTag {
    /// Show a character on screen (`[chara_show]`).
    /// `storage` is already resolved against the character registry.
    ShowCharacter {
        name: Option<String>,
        storage: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        time: Option<u64>,
        method: Option<String>,
    },
    /// Hide a character (`[chara_hide]`).
    HideCharacter {
        name: Option<String>,
        time: Option<u64>,
        method: Option<String>,
    },
    /// Hide all visible characters (`[chara_hide_all]`).
    HideAllCharacters {
        time: Option<u64>,
        method: Option<String>,
    },
    /// Unload a character sprite from memory (`[chara_free]`).
    FreeCharacter { name: Option<String> },
    /// Signal that a character definition was deleted (`[chara_delete]`).
    DeleteCharacter { name: Option<String> },
    /// Update a character's expression/pose (`[chara_mod]`).
    /// `storage` is already resolved against the registry.
    ModCharacter {
        name: Option<String>,
        storage: Option<String>,
        face: Option<String>,
        pose: Option<String>,
    },
    /// Move a character to a new position (`[chara_move]`).
    MoveCharacter {
        name: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        time: Option<u64>,
    },
    /// Assign a character to a z-layer (`[chara_layer]`).
    SetCharacterLayer {
        name: Option<String>,
        layer: Option<String>,
    },
    /// Modify layer properties of a character (`[chara_layer_mod]`).
    ModCharacterLayer {
        name: Option<String>,
        opacity: Option<f32>,
        visible: Option<bool>,
    },
    /// Set a compositable part on a character (`[chara_part]`).
    SetCharacterPart {
        name: Option<String>,
        part: Option<String>,
        storage: Option<String>,
    },
    /// Reset all parts of a character to defaults (`[chara_part_reset]`).
    ResetCharacterParts { name: Option<String> },
}

// ─── 2f2. Skip / Key config actions ──────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvControlTag {
    /// Skip mode was enabled or disabled (`[skipstart]` / `[skipstop]`).
    SkipMode { enabled: bool },
    /// Key-config UI was opened or closed (`[start_keyconfig]` / `[stop_keyconfig]`).
    KeyConfig { open: bool },
}

// ─── 2g. UI tag actions ───────────────────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvUiTag {
    /// Spawn a clickable button widget (`[button]`).
    SpawnButton {
        text: Option<String>,
        graphic: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        width: Option<f32>,
        height: Option<f32>,
        bg: Option<String>,
        hover_bg: Option<String>,
        press_bg: Option<String>,
        color: Option<String>,
        font_size: Option<f32>,
        target: Option<String>,
        storage: Option<String>,
        exp: Option<String>,
        key: Option<String>,
        visible: Option<bool>,
        opacity: Option<f32>,
    },
    /// Make a layer respond to click events (`[clickable]`).
    SetClickable {
        layer: Option<String>,
        target: Option<String>,
        storage: Option<String>,
        exp: Option<String>,
    },
    /// Open or control a built-in UI panel (`[showmenu]`, `[showload]`, …).
    ///
    /// `kind` is one of: `"menu"`, `"load"`, `"save"`, `"log"`,
    /// `"hidemessage"`, `"showmenubutton"`, `"hidemenubutton"`.
    OpenPanel { kind: String },
    /// Display a modal dialog box (`[dialog]`).
    ShowDialog {
        text: Option<String>,
        title: Option<String>,
    },
    /// Change the mouse cursor image (`[cursor]`).
    SetCursor { storage: Option<String> },
    /// Toggle the speaker name box (`[speak_on]` / `[speak_off]`).
    SetSpeakerBoxVisible { visible: bool },
    /// Configure a glyph image (`[glyph]`, `[glyph_auto]`, `[glyph_skip]`).
    ///
    /// `kind` is `"default"`, `"auto"`, or `"skip"`.
    SetGlyph {
        kind: String,
        storage: Option<String>,
    },
    /// Visual effect for mode changes (`[mode_effect]`).
    ModeEffect {
        mode: Option<String>,
        effect: Option<String>,
    },
}

// ─── 2h. Misc tag actions ─────────────────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvMiscTag {
    /// Open a URL in the system browser (`[web]`).
    OpenUrl { url: String },
}

// ─── 3. Host → Interpreter ───────────────────────────────────────────────────

#[derive(Message, Debug, Clone)]
pub enum EvHostInput {
    /// Player selected a choice (index into the last `BeginChoice`).
    SelectChoice(usize),
    /// Player submitted a text-input value.
    SubmitInput(String),
    /// A named trigger was fired by game code.
    FireTrigger { name: String },
    /// An async operation (animation, audio, transition) completed.
    ///
    /// Emit this to unblock a `WaitForCompletion` / `[wa]`-family wait.
    CompletionSignal,
}
