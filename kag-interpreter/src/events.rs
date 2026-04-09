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
    DisplayText {
        text: String,
        speaker: Option<String>,
    },

    /// Insert a line break (`[r]`) inside the message window.
    InsertLineBreak,

    /// Clear the current message window (`[cm]` or page-break after `[p]`).
    ClearMessage,

    // ── Input waits ──────────────────────────────────────────────────────────
    /// Pause until the player clicks/taps.
    /// `clear_after = false` → `[l]` (keep text), `true` → `[p]` (clear on advance).
    WaitForClick { clear_after: bool },

    /// Pause for a fixed number of milliseconds (`[wait time=…]`).
    WaitMs(u64),

    /// Hard stop — the interpreter will not advance without an explicit
    /// `HostEvent::Resume` (`[s]` tag).
    Stop,

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

    // ── Variable mutations ───────────────────────────────────────────────────
    /// Notifies the host that a variable was changed by the script.
    VariableChanged {
        scope: VarScope,
        key: String,
        /// JSON-compatible value serialised as a string for simplicity.
        value: String,
    },

    // ── Embedded expression output ───────────────────────────────────────────
    /// The result of an `[emb exp=…]` tag — display this string inline.
    EmbedText(String),

    // ── Passthrough for non-core tags ────────────────────────────────────────
    /// Any tag the interpreter does not handle internally is forwarded here.
    /// The host can use this for images, audio, transitions, etc.
    Tag {
        name: String,
        params: Vec<(String, String)>,
    },

    // ── Interpreter lifecycle ────────────────────────────────────────────────
    /// The scenario has reached its end naturally.
    End,

    /// A non-fatal warning (e.g. undefined tag, duplicate label).
    Warning(String),

    /// A fatal interpreter error.  The runtime will stop after emitting this.
    Error(String),
}

// ─── Events sent from the host to the interpreter ────────────────────────────

/// Events that the host sends to the interpreter to drive forward execution.
#[derive(Debug, Clone)]
pub enum HostEvent {
    /// The player clicked / tapped (advances past `[l]`, `[p]`, `Stop`).
    Clicked,

    /// A `WaitMs` timer has elapsed.
    TimerElapsed,

    /// The player selected choice at the given index from a `BeginChoices`.
    ChoiceSelected(usize),

    /// The host has loaded a scenario file and provides its raw text.
    /// Used when the interpreter asks for a `[jump storage=…]`,
    /// `[call storage=…]`, or `[return]` that targets a different file.
    ScenarioLoaded { name: String, source: String },

    /// Explicit signal to resume from a `Stop` state.
    Resume,
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
