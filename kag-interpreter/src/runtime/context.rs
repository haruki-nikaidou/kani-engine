//! Runtime execution context for the KAG interpreter.
//!
//! Holds the program counter, all three stacks (call / if / macro), the
//! current speaker name, and the link-choice accumulator.

use rhai::Map;

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
}

/// A choice being accumulated between `[link]` and `[endlink]`.
#[derive(Debug, Clone)]
pub struct PendingChoice {
    pub text: String,
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
}
