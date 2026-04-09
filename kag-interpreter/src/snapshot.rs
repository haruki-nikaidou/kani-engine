//! Interpreter snapshot — the serialisable form of all runtime state.
//!
//! A snapshot captures the exact position and variable state of the
//! interpreter so that execution can be resumed later (save / load).
//!
//! ## What is saved
//!
//! | Field | Contents |
//! |-------|----------|
//! | `pc` | Op-list index the interpreter will resume from |
//! | `storage` | Name of the scenario file currently loaded |
//! | `f` | Per-play game variables (`f.flag`, `f.counter`, …) |
//! | `sf` | Persistent system variables (`sf.unlocked`, …) |
//! | `mp` | Active macro parameter map at the moment of the snapshot |
//! | `call_stack` | `[call]`/`[return]` frames (return addresses) |
//! | `if_stack` | `[if]`/`[else]`/`[endif]` nesting frames |
//! | `macro_stack` | Macro invocation frames with their saved `mp` |
//! | `nowait` | Whether `[nowait]` was active |
//! | `text_speed` | Per-character delay override from `[delay]` |
//! | `log_enabled` | Whether backlog recording was active |
//! | `erased_macros` | Macro names deleted via `[erasemacro]` |
//!
//! ## `sf` and cross-save persistence
//!
//! `sf` (system flags) is included in the snapshot for completeness, but
//! bridges may want to treat it as separate, globally-persistent state that
//! survives across save slots (e.g. "all routes cleared" flags).  In that
//! case, restore `sf` from a dedicated system file and ignore the `sf` field
//! in per-slot snapshots.
//!
//! ## Transient variables (`tf`)
//!
//! `tf` is intentionally **not** saved — it is reset to empty on restore,
//! matching the original TyranoScript behaviour.

use serde::{Deserialize, Serialize};

// ─── Public snapshot types ────────────────────────────────────────────────────

/// A complete, serialisable snapshot of the interpreter's runtime state.
///
/// Obtain one via [`KagInterpreter::take_snapshot`] (sends `HostEvent::TakeSnapshot`
/// and awaits the resulting `KagEvent::Snapshot`), or construct one directly
/// for testing.
///
/// Restore with [`KagInterpreter::spawn_from_snapshot`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpreterSnapshot {
    /// Op-list index to resume execution from.
    pub pc: usize,
    /// Name of the scenario file that was loaded when the snapshot was taken.
    pub storage: String,

    // ── Variable maps ─────────────────────────────────────────────────────
    /// Per-play game variables (`f.*`).
    pub f: serde_json::Value,
    /// Persistent system variables (`sf.*`).
    pub sf: serde_json::Value,
    /// Active macro parameter map (`mp.*`) at snapshot time.
    pub mp: serde_json::Value,

    // ── Execution stacks ──────────────────────────────────────────────────
    /// `[call]` / `[return]` stack frames.
    pub call_stack: Vec<CallFrameSnap>,
    /// `[if]` / `[elsif]` / `[else]` / `[endif]` nesting frames.
    pub if_stack: Vec<IfFrameSnap>,
    /// Macro invocation frames (innermost last).
    pub macro_stack: Vec<MacroFrameSnap>,

    // ── Display-mode flags ────────────────────────────────────────────────
    /// `true` while inside a `[nowait]` … `[endnowait]` block.
    pub nowait: bool,
    /// Per-character delay in ms set by `[delay]`; `None` for host default.
    pub text_speed: Option<u64>,
    /// `false` while inside `[nolog]` … `[endnolog]`.
    pub log_enabled: bool,
    /// Macro names deleted at runtime via `[erasemacro]`.
    pub erased_macros: Vec<String>,

    // ── Input-handling flags ──────────────────────────────────────────────
    /// Whether click-skip is enabled (controlled by `[clickskip]`).
    pub clickskip_enabled: bool,
    /// Whether auto-character-wait is active (`[autowc enabled=…]`).
    pub autowc_enabled: bool,
    /// Per-character wait overrides set by `[autowc]`.
    /// Each element is `(character_string, delay_ms)`.
    pub autowc_map: Vec<(String, u64)>,
}

/// A serialisable `[call]` stack frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallFrameSnap {
    /// Op index to resume at after `[return]`.
    pub return_pc: usize,
    /// Scenario file that was active when `[call]` was issued.
    pub return_storage: String,
}

/// A serialisable `[if]` nesting frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfFrameSnap {
    /// Nesting depth (1-based).
    pub depth: usize,
    /// Whether the current branch is being executed.
    pub executing: bool,
    /// Whether any branch in this block has been taken.
    pub branch_taken: bool,
}

/// A serialisable macro invocation frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroFrameSnap {
    /// Name of the macro.
    pub macro_name: String,
    /// Op index to return to after `[endmacro]`.
    pub return_pc: usize,
    /// The `mp` bindings that were active *before* entering this macro.
    pub saved_mp: serde_json::Value,
}
