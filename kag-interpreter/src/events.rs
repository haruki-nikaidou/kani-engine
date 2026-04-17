use kag_syntax::tag_defs::TagName;

use crate::error::InterpreterDiagnostic;
use crate::snapshot::InterpreterSnapshot;

// ─── Rich text types ──────────────────────────────────────────────────────────

/// Style attributes accumulated at a single text run.
///
/// Produced by parsing XML-style inline markup (`<b>`, `<i>`, `<color>`, …)
/// within a message text line.  A span with all defaults is semantically
/// equivalent to a plain string.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    /// CSS-style colour string (e.g. `"#ff0000"` or `"red"`).
    pub color: Option<String>,
    /// Font size in points.
    pub size: Option<f32>,
    pub shadow: bool,
    pub outline: bool,
    /// Furigana reading for the span (set by `<ruby rt="…">`).
    pub ruby: Option<String>,
    pub nowrap: bool,
}

/// A styled fragment of a message text line.
///
/// The concatenation of all `text` fields in a `Vec<TextSpan>` equals the
/// plain text of the message.
#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    pub text: String,
    pub style: TextStyle,
}

// ─── Animation types ──────────────────────────────────────────────────────────

/// One keyframe in a named keyframe animation sequence.
///
/// Produced by `[frame time=… opacity=… x=… y=…]` inside a
/// `[keyframe]`…`[endkeyframe]` block and carried inside resolved
/// [`ResolvedTag::Kanim`] / [`ResolvedTag::Xanim`] payloads.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameSpec {
    /// Time offset from animation start in milliseconds.
    pub time: u64,
    /// Target opacity at this frame (0.0–1.0).  `None` means unchanged.
    pub opacity: Option<f32>,
    /// Target x-position at this frame.  `None` means unchanged.
    pub x: Option<f32>,
    /// Target y-position at this frame.  `None` means unchanged.
    pub y: Option<f32>,
}

/// The variable-scope identifiers used in KAG scripts.
///
/// - `F`  — per-play game flags (`f.flag_name`)
/// - `Sf` — persistent system flags (`sf.flag_name`)
/// - `Tf` — transient (non-saved) flags (`tf.flag_name`)
/// - `Mp` — macro parameter bindings (`mp.param_name`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarScope {
    F,
    Sf,
    Tf,
    Mp,
}

// ─── ResolvedTag ─────────────────────────────────────────────────────────────

/// A KAG tag with all dynamic attribute values resolved to concrete types.
///
/// Produced by the interpreter after resolving all `MaybeResolved<T>` fields
/// from the parsed [`kag_syntax::tag_defs::KnownTag`] against the current
/// `RuntimeContext` (variable scopes and macro parameters).
///
/// The host bridge matches on this enum to dispatch Bevy events without any
/// further string parsing.  Tags not explicitly listed are represented as
/// [`ResolvedTag::Extension`], which game-specific code can match on.
#[derive(Debug, Clone)]
pub enum ResolvedTag {
    // ── Image / layer ────────────────────────────────────────────────────────
    Bg {
        storage: Option<String>,
        time: Option<u64>,
        method: Option<String>,
    },
    Image {
        storage: Option<String>,
        layer: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        visible: Option<bool>,
    },
    Layopt {
        layer: Option<String>,
        visible: Option<bool>,
        opacity: Option<f32>,
    },
    Free {
        layer: Option<String>,
    },
    Position {
        layer: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
    },
    /// Copy the current front layer to the back layer (`[backlay]`).
    Backlay,
    /// Set the active message/text layer (`[current]`).
    Current {
        layer: Option<String>,
    },
    /// Position the text cursor within the current message layer (`[locate]`).
    Locate {
        x: Option<f32>,
        y: Option<f32>,
    },
    /// Set the blend mode on a layer (`[layermode]`).
    Layermode {
        layer: Option<String>,
        mode: Option<String>,
    },
    /// Reset the blend mode of a layer to Normal (`[free_layermode]`).
    FreeLayermode {
        layer: Option<String>,
    },
    /// Apply a named shader effect to a layer (`[filter]`).
    Filter {
        layer: Option<String>,
        filter_type: Option<String>,
    },
    /// Remove a filter from a layer (`[free_filter]`).
    FreeFilter {
        layer: Option<String>,
    },
    /// Position a filter within its layer (`[position_filter]`).
    PositionFilter {
        layer: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
    },
    /// Apply an alpha mask image to a layer (`[mask]`).
    Mask {
        layer: Option<String>,
        storage: Option<String>,
    },
    /// Remove a mask from a layer (`[mask_off]`).
    MaskOff {
        layer: Option<String>,
    },
    /// Draw a primitive shape on a layer (`[graph]`).
    Graph {
        layer: Option<String>,
        shape: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        width: Option<f32>,
        height: Option<f32>,
        color: Option<String>,
    },

    // ── Audio ─────────────────────────────────────────────────────────────────
    Bgm {
        storage: Option<String>,
        looping: bool,
        volume: Option<f32>,
        fadetime: Option<u64>,
    },
    Stopbgm {
        fadetime: Option<u64>,
    },
    Pausebgm {
        buf: Option<u32>,
    },
    Resumebgm {
        buf: Option<u32>,
    },
    Fadebgm {
        time: Option<u64>,
        volume: Option<f32>,
    },
    Xchgbgm {
        storage: Option<String>,
        time: Option<u64>,
    },
    Bgmopt {
        looping: Option<bool>,
        seek: Option<String>,
    },
    Se {
        storage: Option<String>,
        buf: Option<u32>,
        volume: Option<f32>,
        looping: bool,
    },
    Stopse {
        buf: Option<u32>,
    },
    Pausese {
        buf: Option<u32>,
    },
    Resumese {
        buf: Option<u32>,
    },
    Seopt {
        buf: Option<u32>,
        looping: Option<bool>,
    },
    Vo {
        storage: Option<String>,
        buf: Option<u32>,
    },
    Changevol {
        target: Option<String>,
        vol: Option<f32>,
        time: Option<u64>,
    },

    // ── Animation ─────────────────────────────────────────────────────────────
    /// Play a preset animation on a named layer (`[anim]`).
    Anim {
        layer: Option<String>,
        preset: Option<String>,
        time: Option<u64>,
        looping: bool,
        delay: Option<u64>,
    },
    /// Stop the animation on a layer (`[stopanim]`).
    StopAnim {
        layer: Option<String>,
    },
    /// Play a keyframe animation on a layer (`[kanim]`).
    Kanim {
        layer: Option<String>,
        frames: Vec<FrameSpec>,
        looping: bool,
    },
    /// Stop a keyframe animation on a layer (`[stop_kanim]`).
    StopKanim {
        layer: Option<String>,
    },
    /// Play a keyframe animation on a character layer (`[xanim]`).
    Xanim {
        layer: Option<String>,
        frames: Vec<FrameSpec>,
        looping: bool,
    },
    /// Stop a keyframe animation on a character layer (`[stop_xanim]`).
    StopXanim {
        layer: Option<String>,
    },

    // ── Video / Movie ─────────────────────────────────────────────────────────
    Bgmovie {
        storage: Option<String>,
        looping: bool,
        volume: Option<f32>,
    },
    StopBgmovie,
    Movie {
        storage: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        width: Option<f32>,
        height: Option<f32>,
    },

    // ── Transition ────────────────────────────────────────────────────────────
    Trans {
        method: Option<String>,
        time: Option<u64>,
        rule: Option<String>,
    },
    Fadein {
        time: Option<u64>,
        color: Option<String>,
    },
    Fadeout {
        time: Option<u64>,
        color: Option<String>,
    },
    Movetrans {
        layer: Option<String>,
        time: Option<u64>,
        x: Option<f32>,
        y: Option<f32>,
    },

    // ── Effect ────────────────────────────────────────────────────────────────
    Quake {
        time: Option<u64>,
        hmax: Option<f32>,
        vmax: Option<f32>,
    },
    Shake {
        time: Option<u64>,
        amount: Option<f32>,
        axis: Option<String>,
    },
    Flash {
        time: Option<u64>,
        color: Option<String>,
    },

    // ── Message window ────────────────────────────────────────────────────────
    Msgwnd {
        visible: Option<bool>,
        layer: Option<String>,
    },
    Wndctrl {
        x: Option<f32>,
        y: Option<f32>,
        width: Option<f32>,
        height: Option<f32>,
    },
    Resetfont,
    Font {
        face: Option<String>,
        size: Option<f32>,
        bold: Option<bool>,
        italic: Option<bool>,
    },
    /// `[size value=N]` — sets font size only.
    Size {
        value: Option<f32>,
    },
    /// `[bold value=true|false]` — sets bold style only. Defaults to `true` if absent.
    Bold {
        value: Option<bool>,
    },
    /// `[italic value=true|false]` — sets italic style only. Defaults to `true` if absent.
    Italic {
        value: Option<bool>,
    },
    Ruby {
        text: Option<String>,
    },
    /// `[nowrap]` sets `enabled = true`; `[endnowrap]` sets `enabled = false`.
    Nowrap {
        enabled: bool,
    },

    // ── Character sprites ─────────────────────────────────────────────────────
    /// Show a character on screen with a resolved image path (`[chara_show]`).
    CharaShow {
        /// Character identifier.
        name: Option<String>,
        /// Resolved image path (face already looked up in the registry).
        storage: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        time: Option<u64>,
        method: Option<String>,
    },
    /// Hide a character with an optional exit transition (`[chara_hide]`).
    CharaHide {
        name: Option<String>,
        time: Option<u64>,
        method: Option<String>,
    },
    /// Hide all visible characters at once (`[chara_hide_all]`).
    CharaHideAll {
        time: Option<u64>,
        method: Option<String>,
    },
    /// Unload a character sprite from memory (`[chara_free]`).
    CharaFree {
        name: Option<String>,
    },
    /// Remove a character definition from the registry (`[chara_delete]`).
    CharaDelete {
        name: Option<String>,
    },
    /// Change the expression/pose of an on-screen character (`[chara_mod]`).
    CharaMod {
        name: Option<String>,
        /// Resolved image path after face/pose lookup.
        storage: Option<String>,
        face: Option<String>,
        pose: Option<String>,
    },
    /// Animate a character to a new position (`[chara_move]`).
    CharaMove {
        name: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        time: Option<u64>,
    },
    /// Assign a character to a z-layer (`[chara_layer]`).
    CharaLayer {
        name: Option<String>,
        layer: Option<String>,
    },
    /// Modify layer-level properties of a character (`[chara_layer_mod]`).
    CharaLayerMod {
        name: Option<String>,
        opacity: Option<f32>,
        visible: Option<bool>,
    },
    /// Set a compositable part on a character (`[chara_part]`).
    CharaPart {
        name: Option<String>,
        part: Option<String>,
        storage: Option<String>,
    },
    /// Reset all parts of a character to defaults (`[chara_part_reset]`).
    CharaPartReset {
        name: Option<String>,
    },

    // ── Skip / Key config ─────────────────────────────────────────────────────
    /// Enable or disable skip mode (`[skipstart]` / `[skipstop]` / `[cancelskip]`).
    SkipMode {
        enabled: bool,
    },
    /// Open or close the key-binding configuration UI.
    KeyConfig {
        open: bool,
    },

    // ── UI ────────────────────────────────────────────────────────────────────
    /// Spawn a clickable button widget (`[button]`).
    Button {
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
    Clickable {
        layer: Option<String>,
        target: Option<String>,
        storage: Option<String>,
        exp: Option<String>,
    },
    /// Open a built-in UI panel (`[showmenu]`, `[showload]`, etc.).
    OpenPanel {
        kind: String,
    },
    /// Display a modal dialog box (`[dialog]`).
    Dialog {
        text: Option<String>,
        title: Option<String>,
    },
    /// Change the mouse cursor image (`[cursor]`).
    Cursor {
        storage: Option<String>,
    },
    /// Toggle the speaker name box visibility (`[speak_on]` / `[speak_off]`).
    SetSpeakerBoxVisible {
        visible: bool,
    },
    /// Configure a click-wait glyph image (`[glyph]`, `[glyph_auto]`, `[glyph_skip]`).
    SetGlyph {
        kind: String,
        storage: Option<String>,
    },
    /// Visual effect for mode changes (`[mode_effect]`).
    ModeEffect {
        mode: Option<String>,
        effect: Option<String>,
    },

    // ── Web ───────────────────────────────────────────────────────────────────
    /// Open a URL in the system browser (`[web]`).
    Web {
        url: Option<String>,
    },

    /// A tag not handled by the engine's built-in dispatch — either an
    /// engine-internal tag forwarded for host information (e.g. `[ct]`,
    /// `[clickskip]`, `[chara_ptext]`) or a truly unknown game-specific tag.
    ///
    /// Game-specific systems should listen for this variant via `EvTagRouted`.
    Extension {
        name: String,
        params: Vec<(String, String)>,
    },
}

// ─── Events emitted by the interpreter ───────────────────────────────────────

/// Events produced by the KAG interpreter and sent to the host over a channel.
///
/// The host (e.g. a Bevy system) is responsible for rendering text, handling
/// user input, playing audio, etc.  The interpreter itself has no rendering
/// or I/O knowledge.
#[derive(Debug, Clone)]
pub enum KagEvent {
    // ── Text output ──────────────────────────────────────────────────────────
    /// Display a chunk of text in the current message window.
    /// `speaker` is set when a preceding `#name` shorthand was encountered.
    /// `speed` is the per-character delay in ms (`None` = host default).
    /// `log` indicates whether this text should be recorded in the backlog.
    ///
    /// `text` holds the plain text (XML markup stripped) for backward compatibility.
    /// `spans` holds the same content split into styled runs parsed from XML inline markup.
    /// The concatenation of all `spans[i].text` equals `text`.
    DisplayText {
        text: String,
        /// Styled spans derived from XML inline markup (`<b>`, `<ruby>`, …).
        spans: Vec<TextSpan>,
        speaker: Option<String>,
        /// Per-character display delay in ms set by `[delay]`, or `None` for default.
        speed: Option<u64>,
        /// `false` while inside a `[nolog]` … `[endnolog]` block.
        log: bool,
    },

    /// Insert a line break (`[r]`) inside the message window.
    InsertLineBreak,

    /// Clear the current message window (`[cm]` or page-break after `[p]`).
    ClearMessage,

    /// Clear only the text content of the current message layer, without
    /// resetting the layer's font/style settings (`[er]`).
    ClearCurrentMessage,

    // ── Input waits ──────────────────────────────────────────────────────────
    /// Pause until the player clicks/taps.
    /// `clear_after = false` → `[l]` (keep text), `true` → `[p]` (clear on advance).
    WaitForClick { clear_after: bool },

    /// Pause for a fixed number of milliseconds (`[wait time=…]`).
    WaitMs(u64),

    /// Hard stop — the interpreter will not advance without an explicit
    /// `HostEvent::Resume` (`[s]` tag).
    Stop,

    /// Pause until the host signals that an asynchronous operation has
    /// finished.  Emitted by `[wa]`, `[wm]`, `[wt]`, `[wq]`, `[wb]`, `[wf]`,
    /// `[wl]`, `[ws]`, `[wv]`, `[wp]`.  The host can distinguish them by
    /// inspecting `which`.  `canskip` mirrors the KAG `canskip=` attribute;
    /// when `true` the host may resolve the wait early on click.
    WaitForCompletion {
        /// Which wait-for-completion tag was encountered.
        which: TagName,
        /// Whether the host may dismiss this wait on click.
        canskip: Option<bool>,
        /// Audio/animation buffer slot (for `[wb]`, `[ws]`, `[wv]`, etc.).
        buf: Option<u32>,
    },

    /// Pause until the next raw click, like `[waitclick]`.
    /// Unlike `[l]` / `[p]` this cannot be dismissed by skip mode.
    WaitForRawClick,

    /// Ask the host to display a text-input dialog and wait for the result.
    /// Emitted by `[input]`.  The host responds with `HostEvent::InputResult`.
    /// The interpreter sets the named variable once the result arrives.
    InputRequested {
        /// Variable to store the result in, e.g. `"f.username"`.
        name: String,
        /// Prompt string shown in the dialog (may be empty).
        prompt: String,
        /// Dialog title (may be empty).
        title: String,
    },

    /// Pause until the host fires a named trigger.  Emitted by `[waittrig]`.
    /// The host responds with `HostEvent::TriggerFired`.
    WaitForTrigger {
        /// Trigger name to wait for.
        name: String,
    },

    // ── Navigation ───────────────────────────────────────────────────────────
    /// Jump to a label (and optionally a different scenario file).
    /// Both fields `None` is invalid but handled gracefully.
    Jump {
        storage: Option<String>,
        target: Option<String>,
    },

    /// Return from a `[call]` that crossed into a different scenario file.
    /// The host must respond with `HostEvent::ScenarioLoaded` containing the
    /// caller's file. The interpreter will resume at the saved return PC.
    Return { storage: String },

    // ── Choices ──────────────────────────────────────────────────────────────
    /// Present a set of choices to the player.  The host responds with
    /// `HostEvent::ChoiceSelected(index)`.
    BeginChoices(Vec<ChoiceOption>),

    // ── Embedded expression output ───────────────────────────────────────────
    /// The result of an `[emb exp=…]` tag — display this string inline.
    EmbedText(String),

    // ── Debug output ─────────────────────────────────────────────────────────
    /// Result of a `[trace exp=…]` tag — the host may log this value.
    Trace(String),

    // ── Backlog control ───────────────────────────────────────────────────────
    /// Inject an arbitrary string into the backlog (`[pushlog text=… join=…]`).
    /// `join = true` means append to the previous log entry rather than creating
    /// a new one.
    PushBacklog { text: String, join: bool },

    // ── Passthrough for non-core tags ────────────────────────────────────────
    /// A tag the interpreter does not handle internally is forwarded here as a
    /// typed [`ResolvedTag`].  The host bridge dispatches it to the appropriate
    /// Bevy system.  Game-specific tags arrive as [`ResolvedTag::Extension`].
    Tag(Box<ResolvedTag>),

    // ── Interpreter lifecycle ────────────────────────────────────────────────
    /// The scenario has reached its end naturally.
    End,

    /// A structured diagnostic (warning or error).
    ///
    /// When `diagnostic.severity == DiagnosticSeverity::Error` the interpreter
    /// will emit `KagEvent::End` immediately after this event and shut down.
    Diagnostic(InterpreterDiagnostic),

    /// A complete snapshot of the current interpreter state, emitted in
    /// response to `HostEvent::TakeSnapshot`.
    ///
    /// The host should serialise this to JSON (via `serde_json::to_string`) and
    /// write it to disk as a save file.  Restore with
    /// `KagInterpreter::spawn_from_snapshot`.
    Snapshot(Box<InterpreterSnapshot>),
}

// ─── Events sent from the host to the interpreter ────────────────────────────

/// Events that the host sends to the interpreter to drive forward execution.
#[derive(Debug)]
pub enum HostEvent {
    /// The player clicked / tapped (advances past `[l]`, `[p]`, `Stop`).
    Clicked,

    /// A `WaitMs` timer has elapsed.
    TimerElapsed,

    /// The player scrolled the mouse wheel (fires the `[wheel]` handler at `[s]`).
    WheelScrolled,

    /// The player selected choice at the given index from a `BeginChoices`.
    ChoiceSelected(usize),

    /// The host has loaded a scenario file and provides its raw text.
    /// Used when the interpreter asks for a `[jump storage=…]`,
    /// `[call storage=…]`, or `[return]` that targets a different file.
    ScenarioLoaded { name: String, source: String },

    /// Explicit signal to resume from a `Stop` state.
    Resume,

    /// Set a single variable. `value_expr` is evaluated as a Rhai expression
    /// (e.g. `"42"`, `"true"`, `"\"Alice\""`).
    SetVariable {
        scope: VarScope,
        key: String,
        value_expr: String,
    },

    /// Request a point-in-time snapshot of all variable scopes.
    /// The reply arrives through the oneshot channel — valid to call
    /// whenever the interpreter is blocked at any pause point.
    QueryVariables(tokio::sync::oneshot::Sender<VariableSnapshot>),

    /// Request an [`InterpreterSnapshot`] of the current runtime state.
    ///
    /// The interpreter will respond with `KagEvent::Snapshot(…)` on the event
    /// channel.  This may only be sent while the interpreter is paused at a
    /// wait point (`WaitForClick`, `WaitMs`, or `Stop`); sending it at other
    /// times is silently ignored.
    TakeSnapshot,

    /// Signals that the asynchronous operation the interpreter is blocked on
    /// has finished.  Unblocks `KagEvent::WaitForCompletion`.
    CompletionSignal,

    /// Delivers the player's text-input result for `KagEvent::InputRequested`.
    /// Passing an empty string is valid and means the player cancelled.
    InputResult(String),

    /// Fires a named trigger, unblocking any `KagEvent::WaitForTrigger` that
    /// is waiting for this name.
    TriggerFired { name: String },
}

// ─── Variable snapshot ────────────────────────────────────────────────────────

/// A point-in-time copy of all three variable scopes, with every value
/// stringified for uniform handling by the host.
#[derive(Debug, Clone)]
pub struct VariableSnapshot {
    /// Per-play game flags (`f.*`).
    pub f: std::collections::HashMap<String, String>,
    /// Persistent system flags (`sf.*`).
    pub sf: std::collections::HashMap<String, String>,
    /// Transient flags (`tf.*`).
    pub tf: std::collections::HashMap<String, String>,
}

// ─── Supporting types ─────────────────────────────────────────────────────────

/// A single option in a multiple-choice prompt.
#[derive(Debug, Clone)]
pub struct ChoiceOption {
    /// Display text shown to the player.
    pub text: String,
    /// Scenario file to jump to (if different from the current one).
    pub storage: Option<String>,
    /// Label target to jump to after selection.
    pub target: Option<String>,
    /// Optional rhai expression evaluated before executing the jump.
    pub exp: Option<String>,
}
