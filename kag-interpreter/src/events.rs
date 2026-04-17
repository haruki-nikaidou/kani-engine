use kag_syntax::tag_defs::TagName;

use crate::snapshot::InterpreterSnapshot;

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
    Bg { storage: Option<String>, time: Option<u64>, method: Option<String> },
    Image {
        storage: Option<String>,
        layer: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
        visible: Option<bool>,
    },
    Layopt { layer: Option<String>, visible: Option<bool>, opacity: Option<f32> },
    Free { layer: Option<String> },
    Position { layer: Option<String>, x: Option<f32>, y: Option<f32> },

    // ── Audio ─────────────────────────────────────────────────────────────────
    Bgm {
        storage: Option<String>,
        looping: bool,
        volume: Option<f32>,
        fadetime: Option<u64>,
    },
    Stopbgm { fadetime: Option<u64> },
    Se {
        storage: Option<String>,
        buf: Option<u32>,
        volume: Option<f32>,
        looping: bool,
    },
    Stopse { buf: Option<u32> },
    Vo { storage: Option<String>, buf: Option<u32> },
    Fadebgm { time: Option<u64>, volume: Option<f32> },

    // ── Transition ────────────────────────────────────────────────────────────
    Trans { method: Option<String>, time: Option<u64>, rule: Option<String> },
    Fadein { time: Option<u64>, color: Option<String> },
    Fadeout { time: Option<u64>, color: Option<String> },
    Movetrans { layer: Option<String>, time: Option<u64>, x: Option<f32>, y: Option<f32> },

    // ── Effect ────────────────────────────────────────────────────────────────
    Quake { time: Option<u64>, hmax: Option<f32>, vmax: Option<f32> },
    Shake { time: Option<u64>, amount: Option<f32>, axis: Option<String> },
    Flash { time: Option<u64>, color: Option<String> },

    // ── Message window ────────────────────────────────────────────────────────
    Msgwnd { visible: Option<bool>, layer: Option<String> },
    Wndctrl { x: Option<f32>, y: Option<f32>, width: Option<f32>, height: Option<f32> },
    Resetfont,
    Font { face: Option<String>, size: Option<f32>, bold: Option<bool>, italic: Option<bool> },
    /// `[size value=N]` — sets font size only.
    Size { value: Option<f32> },
    /// `[bold value=true|false]` — sets bold style only. Defaults to `true` if absent.
    Bold { value: Option<bool> },
    /// `[italic value=true|false]` — sets italic style only. Defaults to `true` if absent.
    Italic { value: Option<bool> },
    Ruby { text: Option<String> },
    /// `[nowrap]` sets `enabled = true`; `[endnowrap]` sets `enabled = false`.
    Nowrap { enabled: bool },

    // ── Character sprites ─────────────────────────────────────────────────────
    Chara {
        name: Option<String>,
        id: Option<String>,
        storage: Option<String>,
        slot: Option<String>,
        x: Option<f32>,
        y: Option<f32>,
    },
    CharaHide { name: Option<String>, id: Option<String>, slot: Option<String> },
    CharaFree { name: Option<String>, id: Option<String>, slot: Option<String> },
    CharaMod {
        name: Option<String>,
        id: Option<String>,
        face: Option<String>,
        pose: Option<String>,
        storage: Option<String>,
    },

    /// A tag not handled by the engine's built-in dispatch — either an
    /// engine-internal tag forwarded for host information (e.g. `[ct]`,
    /// `[clickskip]`, `[chara_ptext]`) or a truly unknown game-specific tag.
    ///
    /// Game-specific systems should listen for this variant via `EvTagRouted`.
    Extension { name: String, params: Vec<(String, String)> },
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
    DisplayText {
        text: String,
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
    Tag(ResolvedTag),

    // ── Interpreter lifecycle ────────────────────────────────────────────────
    /// The scenario has reached its end naturally.
    End,

    /// A non-fatal warning (e.g. undefined tag, duplicate label).
    Warning(String),

    /// A fatal interpreter error.  The runtime will stop after emitting this.
    Error(String),

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
