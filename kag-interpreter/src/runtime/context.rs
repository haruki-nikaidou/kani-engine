//! Runtime execution context for the KAG interpreter.
//!
//! Holds the program counter, all three stacks (call / if / macro), the
//! current speaker name, and the link-choice accumulator.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use rhai::Map;

use crate::error::InterpreterError;
use crate::events::FrameSpec;
use crate::snapshot::{CallFrameSnap, IfFrameSnap, InterpreterSnapshot, MacroFrameSnap};

// ─── Character registry types ─────────────────────────────────────────────────

/// A named face/expression variant for a character.
#[derive(Debug, Clone)]
pub struct FaceVariant {
    /// Face name (e.g. `"normal"`, `"happy"`).
    pub face: String,
    /// Path to the image for this face variant.
    pub storage: String,
}

/// A character definition stored in the registry.
#[derive(Debug, Clone)]
pub struct CharaDef {
    /// Character identifier (matches the `name=` attribute on all `chara_*` tags).
    pub name: String,
    /// Default sprite image path.
    pub storage: Option<String>,
    /// Declared sprite dimensions.
    pub width: Option<f32>,
    pub height: Option<f32>,
    /// Registered face variants.
    pub faces: Vec<FaceVariant>,
}

impl CharaDef {
    /// Look up the image path for a named face variant.
    /// Falls back to `self.storage` if the face is not found.
    pub fn resolve_face(&self, face: Option<&str>) -> Option<String> {
        if let Some(f) = face {
            if let Some(var) = self.faces.iter().find(|v| v.face == f) {
                return Some(var.storage.clone());
            }
        }
        self.storage.clone()
    }
}

use super::script_engine::ScriptEngine;

// ─── Stack frames ─────────────────────────────────────────────────────────────

/// A single frame on the `[call]` / `[return]` stack.
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Index in the op list to resume at after `[return]`.
    pub return_pc: usize,
    /// Name of the scenario file that was active when `[call]` was issued.
    pub return_storage: String,
}

/// A single frame on the macro invocation stack.
#[derive(Debug, Clone)]
pub struct MacroFrame {
    /// Name of the macro being executed.
    pub macro_name: String,
    /// Index in the op list to resume at when the macro body ends.
    pub return_pc: usize,
    /// The `mp` bindings that were active *before* entering this macro (so
    /// they can be restored on exit).
    pub saved_mp: Map,
}

/// Tracks `[if]` / `[elsif]` / `[else]` / `[endif]` nesting.
#[derive(Debug, Clone)]
pub struct IfFrame {
    /// Number of `[if]` tags deep we are.
    pub depth: usize,
    /// True while the current branch condition evaluated to `true` and we
    /// should execute ops.  When `false` the executor skips ops.
    pub executing: bool,
    /// True once *any* branch in this `[if]` block has been taken — used to
    /// skip remaining `[elsif]` / `[else]` branches.
    pub branch_taken: bool,
}

// ─── Runtime context ─────────────────────────────────────────────────────────

/// All mutable state for one running KAG scenario session.
#[derive(Debug)]
pub struct RuntimeContext {
    // ── Program counter ───────────────────────────────────────────────────────
    /// Index into `Script::ops` of the *next* op to execute.
    pub pc: usize,

    /// Name of the scenario file currently being executed.
    pub current_storage: String,

    // ── Stacks ────────────────────────────────────────────────────────────────
    /// `[call]` / `[return]` stack.
    pub call_stack: Vec<CallFrame>,

    /// `[if]` / `[elsif]` / `[else]` / `[endif]` nesting stack.
    pub if_stack: Vec<IfFrame>,

    /// Macro invocation stack.
    pub macro_stack: Vec<MacroFrame>,

    // ── Scripting engine ──────────────────────────────────────────────────────
    /// Rhai engine with persistent variable scope (`f`, `sf`, `tf`, `mp`).
    pub script_engine: ScriptEngine,

    // ── Narrative state ───────────────────────────────────────────────────────
    /// Speaker name set by the most recent `#name` shorthand (reset after the
    /// next text op is emitted).
    pub current_speaker: Option<String>,

    /// Accumulator for `[link]`/`[endlink]` choice spans.
    pub pending_choices: Vec<PendingChoice>,

    /// True while inside a `[link]` block accumulating choice text.
    pub in_link: bool,

    // ── Display mode flags ────────────────────────────────────────────────────
    /// When `true`, `[l]` and `[p]` do not emit `WaitForClick` (set by `[nowait]`).
    pub nowait: bool,

    /// Per-character display speed in ms, if overridden by `[delay]`.
    /// `None` means use the host/config default.
    pub text_speed: Option<u64>,

    /// When `false`, `DisplayText` events should not be recorded in the backlog
    /// (controlled by `[nolog]` / `[endnolog]`).
    pub log_enabled: bool,

    /// Set of macro names that have been deleted at runtime via `[erasemacro]`.
    pub erased_macros: HashSet<String>,

    // ── [s]-wait handler system ───────────────────────────────────────────────
    /// Jump registered by `[click]` — fires when player clicks while at `[s]`.
    pub pending_click: Option<JumpTarget>,

    /// Jump registered by `[timeout]` — fires after `time_ms` at `[s]`.
    pub pending_timeout: Option<TimeoutHandler>,

    /// Jump registered by `[wheel]` — fires on mouse-wheel while at `[s]`.
    pub pending_wheel: Option<JumpTarget>,

    // ── [clickskip] state ─────────────────────────────────────────────────────
    /// Whether click-skip mode is enabled (controlled by `[clickskip]`).
    /// When `true` the host may use clicks to skip transitions/animations.
    pub clickskip_enabled: bool,

    // ── Skip mode ─────────────────────────────────────────────────────────────
    /// When `true`, `[l]` and `[p]` waits are auto-advanced at high speed
    /// (controlled by `[skipstart]` / `[skipstop]` / `[cancelskip]`).
    pub skip_mode: bool,

    // ── [autowc] state ────────────────────────────────────────────────────────
    /// Whether auto-character-wait is active (set by `[autowc enabled=true]`).
    pub autowc_enabled: bool,

    /// Per-character wait overrides.  Each entry maps a character string (may
    /// be multi-byte) to a delay in milliseconds.  Set by `[autowc ch=… time=…]`.
    pub autowc_map: Vec<(String, u64)>,

    // ── [resetwait] / [wait mode=until] ──────────────────────────────────────
    /// Baseline instant set by `[resetwait]`; used to compute elapsed time for
    /// `[wait mode=until time=N]`.  `None` means "not yet set".
    /// Not serialisable — resets to `None` on every interpreter start / restore.
    pub wait_base_time: Option<Instant>,

    // ── Character definition registry ────────────────────────────────────────
    /// Named character definitions registered by `[chara_new]`/`[chara_face]`.
    /// Persists across scene files.
    pub chara_registry: HashMap<String, CharaDef>,

    // ── Keyframe animation definitions ────────────────────────────────────────
    /// Named keyframe sequences defined by `[keyframe]`…`[endkeyframe]` blocks.
    /// Persists across scene files so animations can be defined once globally.
    pub keyframe_defs: HashMap<String, Vec<FrameSpec>>,

    /// Name of the keyframe sequence currently being defined (set by
    /// `[keyframe name=…]`, cleared by `[endkeyframe]`).  `None` when not
    /// inside a definition block.
    pub current_keyframe_name: Option<String>,
}

/// A choice being accumulated between `[link]` and `[endlink]`.
#[derive(Debug, Clone)]
pub struct PendingChoice {
    pub text: String,
    pub storage: Option<String>,
    pub target: Option<String>,
    pub exp: Option<String>,
}

/// A jump target registered by `[click]`, `[wheel]`, or `[timeout]`.
#[derive(Debug, Clone)]
pub struct JumpTarget {
    pub storage: Option<String>,
    pub target: Option<String>,
    pub exp: Option<String>,
}

/// A timed jump registered by `[timeout]`.
#[derive(Debug, Clone)]
pub struct TimeoutHandler {
    pub time_ms: u64,
    pub storage: Option<String>,
    pub target: Option<String>,
    pub exp: Option<String>,
}

impl RuntimeContext {
    pub fn new(storage_name: impl Into<String>) -> Self {
        Self {
            pc: 0,
            current_storage: storage_name.into(),
            call_stack: Vec::new(),
            if_stack: Vec::new(),
            macro_stack: Vec::new(),
            script_engine: ScriptEngine::new(),
            current_speaker: None,
            pending_choices: Vec::new(),
            in_link: false,
            nowait: false,
            text_speed: None,
            log_enabled: true,
            erased_macros: HashSet::new(),
            pending_click: None,
            pending_timeout: None,
            pending_wheel: None,
            clickskip_enabled: true,
            skip_mode: false,
            autowc_enabled: false,
            autowc_map: Vec::new(),
            wait_base_time: None,
            chara_registry: HashMap::new(),
            keyframe_defs: HashMap::new(),
            current_keyframe_name: None,
        }
    }

    // ── Stack clearing ────────────────────────────────────────────────────────

    /// Clear a specific stack by name (`"call"`, `"if"`, or `"macro"`), or all
    /// three if `which` is empty / unrecognised.
    pub fn clear_stack(&mut self, which: &str) {
        match which {
            "call" => self.call_stack.clear(),
            "if" => self.if_stack.clear(),
            "macro" => {
                // Restore the outermost mp before discarding macro frames
                if let Some(frame) = self.macro_stack.first() {
                    self.script_engine.set_mp(frame.saved_mp.clone());
                }
                self.macro_stack.clear();
            }
            _ => {
                self.call_stack.clear();
                self.if_stack.clear();
                if !self.macro_stack.is_empty() {
                    if let Some(frame) = self.macro_stack.first() {
                        self.script_engine.set_mp(frame.saved_mp.clone());
                    }
                    self.macro_stack.clear();
                }
            }
        }
    }

    // ── Program counter helpers ───────────────────────────────────────────────

    /// Advance the program counter by one.
    pub fn advance(&mut self) {
        self.pc += 1;
    }

    /// Set the program counter to an absolute op index.
    pub fn jump_to(&mut self, idx: usize) {
        self.pc = idx;
    }

    // ── Call stack ────────────────────────────────────────────────────────────

    /// Push a call frame (for `[call]`).
    pub fn push_call(&mut self, return_pc: usize) {
        self.call_stack.push(CallFrame {
            return_pc,
            return_storage: self.current_storage.clone(),
        });
    }

    /// Pop a call frame (for `[return]`).
    pub fn pop_call(&mut self) -> Option<CallFrame> {
        self.call_stack.pop()
    }

    // ── If stack ──────────────────────────────────────────────────────────────

    /// Enter a new `[if]` block; `cond` is the result of evaluating `exp=`.
    pub fn push_if(&mut self, cond: bool) {
        self.if_stack.push(IfFrame {
            depth: self.if_stack.len() + 1,
            executing: cond,
            branch_taken: cond,
        });
    }

    /// Handle `[elsif exp=…]`.  Only relevant when the *innermost* if block
    /// has not yet had a branch taken.
    pub fn elsif(&mut self, cond: bool) {
        if let Some(frame) = self.if_stack.last_mut() {
            if !frame.branch_taken && cond {
                frame.executing = true;
                frame.branch_taken = true;
            } else {
                frame.executing = false;
            }
        }
    }

    /// Handle `[else]`.
    pub fn else_branch(&mut self) {
        if let Some(frame) = self.if_stack.last_mut() {
            frame.executing = !frame.branch_taken;
        }
    }

    /// Pop the innermost if frame (for `[endif]`).
    pub fn pop_if(&mut self) {
        self.if_stack.pop();
    }

    /// True if the current op should actually be executed (not skipped by a
    /// false conditional branch).
    pub fn is_executing(&self) -> bool {
        self.if_stack.iter().all(|f| f.executing)
    }

    // ── Macro stack ───────────────────────────────────────────────────────────

    /// Enter a macro invocation, saving the current `mp` and setting the new one.
    pub fn push_macro(&mut self, macro_name: impl Into<String>, return_pc: usize, new_mp: Map) {
        let saved_mp = self.script_engine.mp();
        self.script_engine.set_mp(new_mp);
        self.macro_stack.push(MacroFrame {
            macro_name: macro_name.into(),
            return_pc,
            saved_mp,
        });
    }

    /// Exit the current macro, restoring the previous `mp`.
    pub fn pop_macro(&mut self) -> Option<MacroFrame> {
        if let Some(frame) = self.macro_stack.pop() {
            self.script_engine.set_mp(frame.saved_mp.clone());
            Some(frame)
        } else {
            None
        }
    }

    // ── Entity / expression resolution ───────────────────────────────────────

    /// Resolve a raw parameter value string that may be a literal, an entity
    /// expression (`&expr`), or a macro parameter reference (`%key|default`).
    ///
    /// - Bare string → returned as-is.
    /// - Starts with `&` → evaluated as a Rhai expression.
    /// - Starts with `%` → looked up in `mp` (macro params).
    pub fn resolve_value(&mut self, raw: &str) -> String {
        if let Some(expr) = raw.strip_prefix('&') {
            self.script_engine.resolve_entity(expr)
        } else if let Some(rest) = raw.strip_prefix('%') {
            let (key, default) = if let Some(idx) = rest.find('|') {
                (&rest[..idx], Some(&rest[idx + 1..]))
            } else {
                (rest, None)
            };
            let mp = self.script_engine.mp();
            if let Some(val) = mp.get(key) {
                val.to_string()
            } else {
                default.unwrap_or("").to_string()
            }
        } else {
            raw.to_owned()
        }
    }

    /// Resolve a `cond=` parameter expression.  Returns `true` when the
    /// condition is absent (unconditional) or evaluates to truthy.
    pub fn check_cond(&mut self, cond: Option<&str>) -> bool {
        match cond {
            None => true,
            Some(expr) => self.script_engine.eval_bool(expr).unwrap_or(true),
        }
    }

    // ── Snapshot ──────────────────────────────────────────────────────────────

    /// Serialise the full runtime state into an [`InterpreterSnapshot`].
    ///
    /// This captures `pc`, all variable maps (`f`, `sf`, `mp`), all three
    /// execution stacks, and the display-mode flags.  Transient variables
    /// (`tf`) are intentionally excluded.
    pub fn to_snapshot(&self) -> Result<InterpreterSnapshot, InterpreterError> {
        let f = self.script_engine.map_to_json("f")?;
        let sf = self.script_engine.map_to_json("sf")?;
        let mp = self.script_engine.map_to_json("mp")?;

        let call_stack = self
            .call_stack
            .iter()
            .map(|fr| CallFrameSnap {
                return_pc: fr.return_pc,
                return_storage: fr.return_storage.clone(),
            })
            .collect();

        let if_stack = self
            .if_stack
            .iter()
            .map(|fr| IfFrameSnap {
                depth: fr.depth,
                executing: fr.executing,
                branch_taken: fr.branch_taken,
            })
            .collect();

        let macro_stack = self
            .macro_stack
            .iter()
            .map(|fr| {
                let saved_mp = serde_json::to_value(&fr.saved_mp)
                    .map_err(|e| InterpreterError::SerializationError(e.to_string()))?;
                Ok(MacroFrameSnap {
                    macro_name: fr.macro_name.clone(),
                    return_pc: fr.return_pc,
                    saved_mp,
                })
            })
            .collect::<Result<Vec<_>, InterpreterError>>()?;

        let erased_macros = self.erased_macros.iter().cloned().collect();

        Ok(InterpreterSnapshot {
            pc: self.pc,
            storage: self.current_storage.clone(),
            f,
            sf,
            mp,
            call_stack,
            if_stack,
            macro_stack,
            nowait: self.nowait,
            text_speed: self.text_speed,
            log_enabled: self.log_enabled,
            erased_macros,
            clickskip_enabled: self.clickskip_enabled,
            autowc_enabled: self.autowc_enabled,
            autowc_map: self.autowc_map.clone(),
        })
    }

    /// Restore all runtime state from an [`InterpreterSnapshot`].
    ///
    /// The caller is responsible for re-parsing the correct scenario source
    /// and pointing the interpreter task at the restored `pc`.
    /// Transient variables (`tf`) are reset to empty.
    pub fn restore_from_snapshot(
        &mut self,
        snap: &InterpreterSnapshot,
    ) -> Result<(), InterpreterError> {
        self.pc = snap.pc;
        self.current_storage = snap.storage.clone();

        self.script_engine.restore_map("f", &snap.f)?;
        self.script_engine.restore_map("sf", &snap.sf)?;
        self.script_engine.restore_map("mp", &snap.mp)?;
        self.script_engine.clear_tf();

        self.call_stack = snap
            .call_stack
            .iter()
            .map(|fr| CallFrame {
                return_pc: fr.return_pc,
                return_storage: fr.return_storage.clone(),
            })
            .collect();

        self.if_stack = snap
            .if_stack
            .iter()
            .map(|fr| IfFrame {
                depth: fr.depth,
                executing: fr.executing,
                branch_taken: fr.branch_taken,
            })
            .collect();

        self.macro_stack = snap
            .macro_stack
            .iter()
            .map(|fr| {
                let saved_mp: Map = serde_json::from_value(fr.saved_mp.clone())
                    .map_err(|e| InterpreterError::SerializationError(e.to_string()))?;
                Ok(MacroFrame {
                    macro_name: fr.macro_name.clone(),
                    return_pc: fr.return_pc,
                    saved_mp,
                })
            })
            .collect::<Result<Vec<_>, InterpreterError>>()?;

        self.current_speaker = None;
        self.pending_choices.clear();
        self.in_link = false;
        self.nowait = snap.nowait;
        self.text_speed = snap.text_speed;
        self.log_enabled = snap.log_enabled;
        self.erased_macros = snap.erased_macros.iter().cloned().collect();
        self.clickskip_enabled = snap.clickskip_enabled;
        self.autowc_enabled = snap.autowc_enabled;
        self.autowc_map = snap.autowc_map.clone();
        // Transient runtime-only state — reset on restore
        self.pending_click = None;
        self.pending_timeout = None;
        self.pending_wheel = None;
        self.wait_base_time = None;

        Ok(())
    }
}
