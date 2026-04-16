//! Tag-execution logic for the KAG runtime.
//!
//! `execute_op` is a **synchronous** function that processes one op from the
//! script, mutates the `RuntimeContext` (PC, stacks, variables), and returns
//! any `KagEvent`s to be forwarded to the host.
//!
//! Async waiting (click waits, timers, choice input) is handled one level up
//! in the `KagInterpreter` actor.

use crate::ast::{Op, ParamValue, Script, Tag, TextPart};
use crate::error::InterpreterError;
use crate::events::{ChoiceOption, KagEvent};

use super::context::{JumpTarget, RuntimeContext, TimeoutHandler};

// ─── Core tag name constants ──────────────────────────────────────────────────

const TAG_L: &str = "l";
const TAG_P: &str = "p";
const TAG_R: &str = "r";
const TAG_S: &str = "s";
const TAG_WAIT: &str = "wait";
const TAG_CM: &str = "cm";
const TAG_JUMP: &str = "jump";
const TAG_CALL: &str = "call";
const TAG_RETURN: &str = "return";
const TAG_IF: &str = "if";
const TAG_ELSIF: &str = "elsif";
const TAG_ELSE: &str = "else";
const TAG_ENDIF: &str = "endif";
const TAG_IGNORE: &str = "ignore";
const TAG_ENDIGNORE: &str = "endignore";
const TAG_ENDMACRO: &str = "endmacro";
const TAG_EVAL: &str = "eval";
const TAG_EMB: &str = "emb";
const TAG_LINK: &str = "link";
const TAG_ENDLINK: &str = "endlink";
const TAG_GLINK: &str = "glink";
const TAG_CHARA_PTEXT: &str = "chara_ptext";

// ── Internal-state tags ───────────────────────────────────────────────────────
const TAG_CLEARVAR: &str = "clearvar";
const TAG_CLEARSYSVAR: &str = "clearsysvar";
const TAG_CLEARSTACK: &str = "clearstack";
const TAG_ERASEMACRO: &str = "erasemacro";
const TAG_TRACE: &str = "trace";
const TAG_NOWAIT: &str = "nowait";
const TAG_ENDNOWAIT: &str = "endnowait";
const TAG_DELAY: &str = "delay";
const TAG_RESETDELAY: &str = "resetdelay";
const TAG_CONFIGDELAY: &str = "configdelay";
const TAG_NOLOG: &str = "nolog";
const TAG_ENDNOLOG: &str = "endnolog";
const TAG_PUSHLOG: &str = "pushlog";

// ── Blocking wait tags ────────────────────────────────────────────────────────
const TAG_WA: &str = "wa";
const TAG_WM: &str = "wm";
const TAG_WT: &str = "wt";
const TAG_WQ: &str = "wq";
const TAG_WB: &str = "wb";
const TAG_WF: &str = "wf";
const TAG_WL: &str = "wl";
const TAG_WS: &str = "ws";
const TAG_WV: &str = "wv";
const TAG_WP: &str = "wp";
const TAG_WAITCLICK: &str = "waitclick";
const TAG_INPUT: &str = "input";
const TAG_WAITTRIG: &str = "waittrig";

// ── State-change tags ─────────────────────────────────────────────────────────
const TAG_CT: &str = "ct";
const TAG_ER: &str = "er";
const TAG_CH: &str = "ch";
const TAG_HCH: &str = "hch";
const TAG_AUTOWC: &str = "autowc";
const TAG_WC: &str = "wc";
const TAG_CLICKSKIP: &str = "clickskip";
const TAG_RESETWAIT: &str = "resetwait";
const TAG_CLICK: &str = "click";
const TAG_CCLICK: &str = "cclick";
const TAG_TIMEOUT: &str = "timeout";
const TAG_CTIMEOUT: &str = "ctimeout";
const TAG_WHEEL: &str = "wheel";
const TAG_CWHEEL: &str = "cwheel";

// ─── Public entry point ───────────────────────────────────────────────────────

/// Execute one op at `ctx.pc`.
///
/// On return `ctx.pc` has already been advanced (or redirected).
/// Returns `Ok(events)` on success; `Err` only on unrecoverable errors.
pub fn execute_op<'s>(
    script: &Script<'s>,
    ctx: &mut RuntimeContext,
) -> Result<Vec<KagEvent>, InterpreterError> {
    let pc = ctx.pc;
    if pc >= script.ops.len() {
        ctx.advance();
        return Ok(vec![KagEvent::End]);
    }

    let op = &script.ops[pc];

    // ── Control-flow tags are processed regardless of the if-skip state ───────
    // (we must track nesting to know when an `[endif]` closes the *current*
    //  if block rather than an outer one)
    if let Op::Tag(tag) = op
        && is_control_flow_tag(tag.name.as_ref())
    {
        return execute_control_flow(script, ctx, tag);
    }

    // ── When inside a skipped conditional branch, skip everything else ────────
    if !ctx.is_executing() {
        ctx.advance();
        return Ok(vec![]);
    }

    // ── Normal execution ──────────────────────────────────────────────────────
    match op {
        Op::Text { parts, .. } => execute_text(ctx, parts),
        Op::Tag(tag) => execute_tag(script, ctx, tag),
        Op::Label(_) => {
            ctx.advance();
            Ok(vec![])
        }
        Op::ScriptBlock {
            content: script_text,
            ..
        } => {
            let script_text = script_text.clone();
            ctx.advance();
            ctx.script_engine
                .exec(&script_text)
                .map(|_| vec![])
                .or_else(|e| {
                    tracing::error!("[kag] iscript block failed: {e}");
                    Ok(vec![KagEvent::Error(e.to_string())])
                })
        }
        // Skip past the macro body to the op after [endmacro].  skip_to was
        // encoded at compile time for *this specific definition*, so duplicate
        // macro names each jump to their own correct target.
        Op::MacroDef { skip_to, .. } => {
            ctx.jump_to(*skip_to);
            Ok(vec![])
        }
    }
}

// ─── Text op ─────────────────────────────────────────────────────────────────

fn execute_text<'s>(
    ctx: &mut RuntimeContext,
    parts: &[TextPart<'s>],
) -> Result<Vec<KagEvent>, InterpreterError> {
    let mut events = Vec::new();
    let mut text_buf = String::new();

    let speaker = ctx.current_speaker.take();
    let mut current_speed = ctx.text_speed;
    let mut current_log = ctx.log_enabled;

    for part in parts {
        match part {
            TextPart::Literal { text: s, .. } => {
                text_buf.push_str(s.as_ref());
            }
            TextPart::Entity { expr, .. } => {
                let val = ctx.script_engine.eval_soft(expr.as_ref()).to_string();
                text_buf.push_str(&val);
            }
            TextPart::InlineTag(tag) => {
                // Flush accumulated text before the inline tag using the state
                // that was active when those characters were produced.
                if !text_buf.is_empty() {
                    if ctx.in_link {
                        if let Some(c) = ctx.pending_choices.last_mut() {
                            c.text.push_str(&text_buf);
                        }
                        text_buf.clear();
                    } else {
                        events.push(KagEvent::DisplayText {
                            text: std::mem::take(&mut text_buf),
                            speaker: speaker.clone(),
                            speed: current_speed,
                            log: current_log,
                        });
                    }
                }
                // Execute the inline tag (may mutate ctx.text_speed / ctx.log_enabled)
                let mut inline_events = execute_inline_tag(ctx, tag)?;
                events.append(&mut inline_events);
                // Sync so subsequent segments see any speed/log change.
                current_speed = ctx.text_speed;
                current_log = ctx.log_enabled;
            }
        }
    }

    // Flush any remaining text
    if !text_buf.is_empty() {
        if ctx.in_link {
            if let Some(c) = ctx.pending_choices.last_mut() {
                c.text.push_str(&text_buf);
            }
        } else {
            events.push(KagEvent::DisplayText {
                text: text_buf,
                speaker,
                speed: current_speed,
                log: current_log,
            });
        }
    }

    ctx.advance();
    Ok(events)
}

// ─── Inline tag dispatch (occurs within text lines) ───────────────────────────

fn execute_inline_tag(ctx: &mut RuntimeContext, tag: &Tag<'_>) -> Result<Vec<KagEvent>, InterpreterError> {
    // Honour optional `cond=` guard on any inline tag
    let cond_expr = tag.param_str("cond").map(str::to_owned);
    if let Some(ref expr) = cond_expr
        && !ctx.script_engine.eval_bool(expr).unwrap_or(true)
    {
        return Ok(vec![]);
    }

    match tag.name.as_ref() {
        TAG_R => Ok(vec![KagEvent::InsertLineBreak]),
        TAG_L => {
            if ctx.nowait {
                Ok(vec![])
            } else {
                Ok(vec![KagEvent::WaitForClick { clear_after: false }])
            }
        }
        // Always emit the clear signal; the host auto-advances when nowait is set.
        TAG_P => Ok(vec![KagEvent::WaitForClick { clear_after: true }]),
        TAG_S => Ok(vec![KagEvent::Stop]),
        TAG_WAIT => {
            let ms = resolve_u64(ctx, tag, "time").unwrap_or(0);
            Ok(vec![KagEvent::WaitMs(ms)])
        }
        TAG_EVAL => {
            let exp = tag.param_str("exp").unwrap_or("").to_owned();
            if let Err(e) = ctx.script_engine.exec(&exp) {
                Ok(vec![KagEvent::Warning(e.to_string())])
            } else {
                Ok(vec![])
            }
        }
        TAG_EMB => {
            let exp = tag.param_str("exp").unwrap_or("");
            let exp_owned = exp.to_owned();
            let result = ctx
                .script_engine
                .eval_to_string(&exp_owned)
                .unwrap_or_default();
            Ok(vec![KagEvent::EmbedText(result)])
        }
        TAG_DELAY | TAG_CONFIGDELAY => {
            ctx.text_speed = Some(resolve_u64(ctx, tag, "speed").unwrap_or(0));
            Ok(vec![])
        }
        TAG_RESETDELAY => {
            ctx.text_speed = None;
            Ok(vec![])
        }
        TAG_NOLOG => {
            ctx.log_enabled = false;
            Ok(vec![])
        }
        TAG_ENDNOLOG => {
            ctx.log_enabled = true;
            Ok(vec![])
        }
        _ => Ok(vec![build_generic_event(ctx, tag)]),
    }
}

// ─── Tag op dispatch ─────────────────────────────────────────────────────────

fn execute_tag<'s>(
    script: &Script<'s>,
    ctx: &mut RuntimeContext,
    tag: &Tag<'s>,
) -> Result<Vec<KagEvent>, InterpreterError> {
    // Check optional `cond=` guard — if false, skip the tag entirely
    let cond_expr = tag.param_str("cond").map(str::to_owned);
    if let Some(ref expr) = cond_expr
        && !ctx.script_engine.eval_bool(expr).unwrap_or(true)
    {
        ctx.advance();
        return Ok(vec![]);
    }

    let name = tag.name.as_ref();

    // ── Check if this is a macro invocation ────────────────────────────────
    // A macro that has been erased at runtime via [erasemacro] must not be invoked.
    if script.macro_map.contains_key(name) && !ctx.erased_macros.contains(name) {
        return invoke_macro(script, ctx, tag);
    }

    ctx.advance();

    match name {
        // ── Text flow ──────────────────────────────────────────────────────
        TAG_L => {
            if ctx.nowait {
                Ok(vec![])
            } else {
                Ok(vec![KagEvent::WaitForClick { clear_after: false }])
            }
        }
        // Always emit the clear signal; the host auto-advances when nowait is set.
        TAG_P => Ok(vec![KagEvent::WaitForClick { clear_after: true }]),
        TAG_R => Ok(vec![KagEvent::InsertLineBreak]),
        TAG_S => Ok(vec![KagEvent::Stop]),
        TAG_CM => Ok(vec![KagEvent::ClearMessage]),

        // ── Timed wait ─────────────────────────────────────────────────────
        TAG_WAIT => {
            let ms = resolve_u64(ctx, tag, "time").unwrap_or(0);
            let mode = tag.param_str("mode").unwrap_or("normal");
            if mode == "until" {
                // mode=until: wait until `time` ms have elapsed *since the
                // last [resetwait]*.  If the baseline is already past or was
                // never set, emit a zero-duration wait (no-op).
                let elapsed = ctx
                    .wait_base_time
                    .map(|t| t.elapsed().as_millis() as u64)
                    .unwrap_or(ms);
                let remaining = ms.saturating_sub(elapsed);
                if remaining == 0 {
                    Ok(vec![])
                } else {
                    Ok(vec![KagEvent::WaitMs(remaining)])
                }
            } else {
                Ok(vec![KagEvent::WaitMs(ms)])
            }
        }

        // ── Navigation ────────────────────────────────────────────────────
        TAG_JUMP => {
            let storage = resolved_str(ctx, tag, "storage");
            let target = resolved_str(ctx, tag, "target");
            Ok(vec![KagEvent::Jump { storage, target }])
        }

        TAG_CALL => {
            let return_pc = ctx.pc; // already advanced
            ctx.push_call(return_pc);
            let storage = resolved_str(ctx, tag, "storage");
            let target = resolved_str(ctx, tag, "target");
            Ok(vec![KagEvent::Jump { storage, target }])
        }

        TAG_RETURN => {
            if let Some(frame) = ctx.pop_call() {
                ctx.jump_to(frame.return_pc);
                if frame.return_storage != ctx.current_storage {
                    // Cross-file return: host must reload the caller's script.
                    // ctx.pc is already set to return_pc; the interpreter loop
                    // must NOT override it after loading.
                    Ok(vec![KagEvent::Return {
                        storage: frame.return_storage,
                    }])
                } else {
                    Ok(vec![])
                }
            } else {
                Err(InterpreterError::CallStackUnderflow)
            }
        }

        // ── Eval / emb ────────────────────────────────────────────────────
        TAG_EVAL => {
            let exp = tag.param_str("exp").unwrap_or("").to_owned();
            let next = tag.param_str("next").unwrap_or("true");
            let result = ctx.script_engine.exec(&exp);
            let mut events = Vec::new();
            if let Err(e) = result {
                tracing::warn!("[kag] [eval] expression failed: {e}");
                events.push(KagEvent::Warning(e.to_string()));
            }
            if next == "false" {
                // Caller requested no advance — unusual, treat as stop
                events.push(KagEvent::Stop);
            }
            Ok(events)
        }

        TAG_EMB => {
            let exp = tag.param_str("exp").unwrap_or("").to_owned();
            let result = ctx.script_engine.eval_to_string(&exp).unwrap_or_default();
            Ok(vec![KagEvent::EmbedText(result)])
        }

        // ── Choice links ──────────────────────────────────────────────────
        TAG_LINK => {
            ctx.in_link = true;
            let storage = resolved_str(ctx, tag, "storage");
            let target = resolved_str(ctx, tag, "target");
            let exp = tag.param_str("exp").map(str::to_owned);
            ctx.pending_choices
                .push(crate::runtime::context::PendingChoice {
                    text: String::new(),
                    storage,
                    target,
                    exp,
                });
            Ok(vec![build_generic_event(ctx, tag)])
        }

        TAG_ENDLINK => {
            ctx.in_link = false;
            if !ctx.pending_choices.is_empty() {
                // Emit all accumulated choices as a choice prompt
                let choices: Vec<ChoiceOption> = ctx
                    .pending_choices
                    .drain(..)
                    .map(|c| ChoiceOption {
                        text: c.text,
                        storage: c.storage,
                        target: c.target,
                        exp: c.exp,
                    })
                    .collect();
                Ok(vec![KagEvent::BeginChoices(choices)])
            } else {
                Ok(vec![build_generic_event(ctx, tag)])
            }
        }

        TAG_GLINK => {
            let text = resolved_str_owned(ctx, tag, "text");
            let storage = resolved_str(ctx, tag, "storage");
            let target = resolved_str(ctx, tag, "target");
            let exp = tag.param_str("exp").map(str::to_owned);
            Ok(vec![KagEvent::BeginChoices(vec![ChoiceOption {
                text: text.unwrap_or_default(),
                storage,
                target,
                exp,
            }])])
        }

        // ── Character nameplate ───────────────────────────────────────────
        TAG_CHARA_PTEXT => {
            if let Some(name_val) = tag.param_str("name") {
                ctx.current_speaker = Some(name_val.to_owned());
            }
            // Also forward as a generic event for the host's character system
            Ok(vec![build_generic_event(ctx, tag)])
        }

        TAG_ENDMACRO => {
            // If inside a macro invocation, return
            if let Some(frame) = ctx.pop_macro() {
                ctx.jump_to(frame.return_pc);
            }
            Ok(vec![])
        }

        // ── Variable clearing ─────────────────────────────────────────────
        TAG_CLEARVAR => {
            let exp = tag.param_str("exp").unwrap_or("").trim().to_owned();
            if exp.is_empty() {
                // Clear all game (f) and transient (tf) variables
                ctx.script_engine.clear_f();
                ctx.script_engine.clear_tf();
            } else {
                // Remove a specific variable: "f.key", "sf.key", or "tf.key"
                remove_var_by_expr(ctx, &exp);
            }
            Ok(vec![])
        }

        TAG_CLEARSYSVAR => {
            ctx.script_engine.clear_sf();
            Ok(vec![])
        }

        // ── Stack clearing ────────────────────────────────────────────────
        TAG_CLEARSTACK => {
            let which = tag.param_str("stack").unwrap_or("").trim().to_owned();
            ctx.clear_stack(&which);
            Ok(vec![])
        }

        // ── Macro deletion ────────────────────────────────────────────────
        TAG_ERASEMACRO => {
            let name = tag.param_str("name").unwrap_or("").to_owned();
            if !name.is_empty() {
                ctx.erased_macros.insert(name);
            }
            Ok(vec![])
        }

        // ── Debug trace ───────────────────────────────────────────────────
        TAG_TRACE => {
            let exp = tag.param_str("exp").unwrap_or("").to_owned();
            let result = ctx.script_engine.eval_to_string(&exp).unwrap_or_default();
            Ok(vec![KagEvent::Trace(result)])
        }

        // ── Nowait mode ───────────────────────────────────────────────────
        TAG_NOWAIT => {
            ctx.nowait = true;
            Ok(vec![])
        }
        TAG_ENDNOWAIT => {
            ctx.nowait = false;
            Ok(vec![])
        }

        // ── Text display speed ────────────────────────────────────────────
        TAG_DELAY | TAG_CONFIGDELAY => {
            // `delay speed=N` sets per-character ms; bare `[delay]` resets to 0 (instant)
            ctx.text_speed = Some(resolve_u64(ctx, tag, "speed").unwrap_or(0));
            Ok(vec![])
        }
        TAG_RESETDELAY => {
            ctx.text_speed = None;
            Ok(vec![])
        }

        // ── Backlog control ───────────────────────────────────────────────
        TAG_NOLOG => {
            ctx.log_enabled = false;
            Ok(vec![])
        }
        TAG_ENDNOLOG => {
            ctx.log_enabled = true;
            Ok(vec![])
        }
        TAG_PUSHLOG => {
            let text = resolved_str_owned(ctx, tag, "text").unwrap_or_default();
            let join = tag.param_str("join").unwrap_or("false") == "true";
            Ok(vec![KagEvent::PushBacklog { text, join }])
        }

        // ── Message-layer clear variants ──────────────────────────────────
        // [ct] resets the message layer position (like [cm] but with
        // context-target semantics in the original KAG). Emit ClearMessage
        // plus a generic Tag so the host can distinguish it from [cm].
        TAG_CT => Ok(vec![KagEvent::ClearMessage, build_generic_event(ctx, tag)]),

        // [er] clears only the text of the current layer, leaving font /
        // style state intact.
        TAG_ER => Ok(vec![KagEvent::ClearCurrentMessage]),

        // ── Single-character display ──────────────────────────────────────
        TAG_CH => {
            let text = resolved_str_owned(ctx, tag, "text").unwrap_or_default();
            if text.is_empty() {
                Ok(vec![])
            } else {
                Ok(vec![KagEvent::DisplayText {
                    text,
                    speaker: ctx.current_speaker.clone(),
                    speed: ctx.text_speed,
                    log: ctx.log_enabled,
                }])
            }
        }

        // [hch] is the vertical equivalent of [ch]; forward to host for
        // rendering, but also emit a DisplayText event so the backlog works.
        TAG_HCH => {
            let text = resolved_str_owned(ctx, tag, "text").unwrap_or_default();
            let mut events = vec![build_generic_event(ctx, tag)];
            if !text.is_empty() {
                events.push(KagEvent::DisplayText {
                    text,
                    speaker: ctx.current_speaker.clone(),
                    speed: ctx.text_speed,
                    log: ctx.log_enabled,
                });
            }
            Ok(events)
        }

        // ── [autowc] — configure automatic per-character waits ────────────
        TAG_AUTOWC => {
            let enabled = tag.param_str("enabled").unwrap_or("true");
            ctx.autowc_enabled = enabled != "false";
            if ctx.autowc_enabled {
                let ch_str = tag.param_str("ch").unwrap_or("").to_owned();
                let time_str = tag.param_str("time").unwrap_or("").to_owned();
                if !ch_str.is_empty() {
                    let chars: Vec<&str> = ch_str.split(',').collect();
                    let times: Vec<u64> = time_str
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    ctx.autowc_map.clear();
                    for (i, ch) in chars.iter().enumerate() {
                        let delay = times.get(i).or_else(|| times.last()).copied().unwrap_or(0);
                        ctx.autowc_map.push((ch.to_string(), delay));
                    }
                }
            } else {
                ctx.autowc_map.clear();
            }
            Ok(vec![])
        }

        // ── [wc] — wait for N characters of display time ──────────────────
        TAG_WC => {
            let time_ms = resolve_u64(ctx, tag, "time").unwrap_or(0);
            Ok(vec![KagEvent::WaitMs(time_ms)])
        }

        // ── [clickskip] — toggle click-skip mode ─────────────────────────
        TAG_CLICKSKIP => {
            let enabled = resolved_str(ctx, tag, "enabled").unwrap_or_default();
            ctx.clickskip_enabled = enabled != "false";
            Ok(vec![build_generic_event(ctx, tag)])
        }

        // ── [resetwait] — set wait baseline for mode=until ───────────────
        TAG_RESETWAIT => {
            ctx.wait_base_time = Some(std::time::Instant::now());
            Ok(vec![])
        }

        // ── [click]/[cclick] — register/clear click handler at [s] ──────
        TAG_CLICK => {
            let storage = resolved_str(ctx, tag, "storage");
            let target = resolved_str(ctx, tag, "target");
            let exp = tag.param_str("exp").map(str::to_owned);
            ctx.pending_click = Some(JumpTarget {
                storage,
                target,
                exp,
            });
            Ok(vec![])
        }
        TAG_CCLICK => {
            ctx.pending_click = None;
            Ok(vec![])
        }

        // ── [timeout]/[ctimeout] — register/clear timeout handler ────────
        TAG_TIMEOUT => {
            let time_ms = resolve_u64(ctx, tag, "time").unwrap_or(0);
            let storage = resolved_str(ctx, tag, "storage");
            let target = resolved_str(ctx, tag, "target");
            let exp = tag.param_str("exp").map(str::to_owned);
            ctx.pending_timeout = Some(TimeoutHandler {
                time_ms,
                storage,
                target,
                exp,
            });
            Ok(vec![])
        }
        TAG_CTIMEOUT => {
            ctx.pending_timeout = None;
            Ok(vec![])
        }

        // ── [wheel]/[cwheel] — register/clear wheel handler ──────────────
        TAG_WHEEL => {
            let storage = resolved_str(ctx, tag, "storage");
            let target = resolved_str(ctx, tag, "target");
            let exp = tag.param_str("exp").map(str::to_owned);
            ctx.pending_wheel = Some(JumpTarget {
                storage,
                target,
                exp,
            });
            Ok(vec![])
        }
        TAG_CWHEEL => {
            ctx.pending_wheel = None;
            Ok(vec![])
        }

        // ── Blocking wait tags ────────────────────────────────────────────
        // All w* completion-wait tags emit WaitForCompletion with their
        // resolved params so the host bridge can identify what to wait on.
        TAG_WA | TAG_WM | TAG_WT | TAG_WQ | TAG_WB | TAG_WF | TAG_WL | TAG_WS | TAG_WV | TAG_WP => {
            let params = build_resolved_params(ctx, tag);
            Ok(vec![KagEvent::WaitForCompletion {
                tag: name.to_owned(),
                params,
            }])
        }

        TAG_WAITCLICK => Ok(vec![KagEvent::WaitForRawClick]),

        // ── [input] — text-input dialog ───────────────────────────────────
        TAG_INPUT => {
            let var_name = resolved_str_owned(ctx, tag, "name").unwrap_or_default();
            let prompt = resolved_str_owned(ctx, tag, "prompt").unwrap_or_default();
            let title = resolved_str_owned(ctx, tag, "title").unwrap_or_default();
            Ok(vec![KagEvent::InputRequested {
                name: var_name,
                prompt,
                title,
            }])
        }

        // ── [waittrig] — wait for a named trigger ─────────────────────────
        TAG_WAITTRIG => {
            let trig_name = resolved_str_owned(ctx, tag, "name").unwrap_or_default();
            Ok(vec![KagEvent::WaitForTrigger { name: trig_name }])
        }

        // ── All other tags forwarded to host ──────────────────────────────
        _ => Ok(vec![build_generic_event(ctx, tag)]),
    }
}

// ─── Control-flow dispatch (always executed, even inside skipped blocks) ──────

fn is_control_flow_tag(name: &str) -> bool {
    matches!(
        name,
        TAG_IF | TAG_ELSIF | TAG_ELSE | TAG_ENDIF | TAG_IGNORE | TAG_ENDIGNORE
    )
}

fn execute_control_flow<'s>(
    _script: &Script<'s>,
    ctx: &mut RuntimeContext,
    tag: &Tag<'s>,
) -> Result<Vec<KagEvent>, InterpreterError> {
    ctx.advance();

    match tag.name.as_ref() {
        TAG_IF => {
            let exp = tag.param_str("exp").unwrap_or("false").to_owned();
            // Only evaluate condition when outer context is already executing
            let cond = if ctx.is_executing() {
                ctx.script_engine.eval_bool(&exp).unwrap_or(false)
            } else {
                false
            };
            ctx.push_if(cond);
        }
        TAG_ELSIF => {
            if ctx.is_executing()
                || ctx
                    .if_stack
                    .last()
                    .map(|f| !f.branch_taken)
                    .unwrap_or(false)
            {
                let exp = tag.param_str("exp").unwrap_or("false").to_owned();
                let cond = ctx.script_engine.eval_bool(&exp).unwrap_or(false);
                ctx.elsif(cond);
            } else {
                ctx.elsif(false);
            }
        }
        TAG_ELSE => {
            ctx.else_branch();
        }
        TAG_ENDIF => {
            ctx.pop_if();
        }
        TAG_IGNORE => {
            // `[ignore]` uses an `exp=` that is inverted (skip if true)
            let exp = tag.param_str("exp").unwrap_or("false").to_owned();
            let skip = if ctx.is_executing() {
                ctx.script_engine.eval_bool(&exp).unwrap_or(false)
            } else {
                true
            };
            ctx.push_if(!skip);
        }
        TAG_ENDIGNORE => {
            ctx.pop_if();
        }
        _ => {}
    }

    Ok(vec![])
}

// ─── Macro invocation ─────────────────────────────────────────────────────────

fn invoke_macro<'s>(
    script: &Script<'s>,
    ctx: &mut RuntimeContext,
    tag: &Tag<'s>,
) -> Result<Vec<KagEvent>, InterpreterError> {
    let macro_name = tag.name.as_ref();
    let def = match script.macro_map.get(macro_name) {
        Some(d) => d.clone(),
        None => {
            return Ok(vec![KagEvent::Warning(format!(
                "macro not found: {macro_name}"
            ))]);
        }
    };

    let return_pc = ctx.pc + 1; // return to op after the macro call
    ctx.advance();

    // Build `mp` from this tag's parameters
    let mut mp = rhai::Map::new();
    for param in &tag.params {
        if let Some(ref key) = param.key {
            let raw_val = match &param.value {
                ParamValue::Literal(s) => s.to_string(),
                ParamValue::Entity(expr) => ctx.script_engine.resolve_entity(expr),
                ParamValue::MacroParam { key: k, default } => {
                    let existing_mp = ctx.script_engine.mp();
                    existing_mp
                        .get(k.as_ref())
                        .map(|v| v.to_string())
                        .or_else(|| default.as_deref().map(str::to_owned))
                        .unwrap_or_default()
                }
                ParamValue::MacroSplat => {
                    // Splat: pass through all current mp entries (handled below)
                    continue;
                }
            };
            mp.insert(key.as_ref().into(), rhai::Dynamic::from(raw_val));
        }
    }

    // Handle MacroSplat: merge current mp into new mp
    if tag
        .params
        .iter()
        .any(|p| matches!(p.value, ParamValue::MacroSplat))
    {
        let current_mp = ctx.script_engine.mp();
        for (k, v) in current_mp {
            mp.entry(k).or_insert(v);
        }
    }

    ctx.push_macro(macro_name, return_pc, mp);
    ctx.jump_to(def.body_start);

    Ok(vec![])
}

// ─── Variable removal helper ──────────────────────────────────────────────────

/// Parse a `clearvar exp=` expression like `"f.key"` or `"sf.count"` and remove
/// the named key from the appropriate variable scope.
///
/// Only dot-notation with a known scope prefix is supported; anything else is
/// silently ignored.
fn remove_var_by_expr(ctx: &mut RuntimeContext, expr: &str) {
    let expr = expr.trim().trim_matches('"');
    if let Some(rest) = expr.strip_prefix("f.") {
        ctx.script_engine.remove_key("f", rest);
    } else if let Some(rest) = expr.strip_prefix("sf.") {
        ctx.script_engine.remove_key("sf", rest);
    } else if let Some(rest) = expr.strip_prefix("tf.") {
        ctx.script_engine.remove_key("tf", rest);
    }
}

// ─── Param resolution helpers ─────────────────────────────────────────────────

/// Resolve a named tag parameter to a `String`, evaluating entities / macro refs.
fn resolved_str_owned(ctx: &mut RuntimeContext, tag: &Tag<'_>, key: &str) -> Option<String> {
    tag.param(key).map(|pv| match pv {
        ParamValue::Literal(s) => ctx.resolve_value(s.as_ref()),
        ParamValue::Entity(expr) => ctx.script_engine.resolve_entity(expr.as_ref()),
        ParamValue::MacroParam { key: k, default } => {
            let mp = ctx.script_engine.mp();
            mp.get(k.as_ref())
                .map(|v| v.to_string())
                .or_else(|| default.as_deref().map(str::to_owned))
                .unwrap_or_default()
        }
        ParamValue::MacroSplat => String::new(),
    })
}

/// Resolve a named parameter to `Option<String>` — `None` when the parameter
/// is absent.
fn resolved_str(ctx: &mut RuntimeContext, tag: &Tag<'_>, key: &str) -> Option<String> {
    resolved_str_owned(ctx, tag, key)
}

/// Resolve a named parameter as a `u64`.
fn resolve_u64(ctx: &mut RuntimeContext, tag: &Tag<'_>, key: &str) -> Option<u64> {
    resolved_str(ctx, tag, key)
        .as_deref()
        .and_then(|s| s.parse().ok())
}

/// Resolve all tag parameters to a `Vec<(String, String)>` without wrapping
/// them in a `KagEvent`.  Used by the `WaitForCompletion` emitter.
fn build_resolved_params(ctx: &mut RuntimeContext, tag: &Tag<'_>) -> Vec<(String, String)> {
    tag.params
        .iter()
        .filter_map(|p| {
            p.key.as_deref().map(|k| {
                let val = match &p.value {
                    ParamValue::Literal(s) => ctx.resolve_value(s.as_ref()),
                    ParamValue::Entity(expr) => ctx.script_engine.resolve_entity(expr),
                    ParamValue::MacroParam { key, default } => {
                        let mp = ctx.script_engine.mp();
                        mp.get(key.as_ref())
                            .map(|v| v.to_string())
                            .or_else(|| default.as_deref().map(str::to_owned))
                            .unwrap_or_default()
                    }
                    ParamValue::MacroSplat => String::new(),
                };
                (k.to_owned(), val)
            })
        })
        .collect()
}

/// Build a generic `KagEvent::Tag` from any tag, resolving all param values.
fn build_generic_event(ctx: &mut RuntimeContext, tag: &Tag<'_>) -> KagEvent {
    let params: Vec<(String, String)> = tag
        .params
        .iter()
        .filter_map(|p| {
            p.key.as_deref().map(|k| {
                let val = match &p.value {
                    ParamValue::Literal(s) => ctx.resolve_value(s.as_ref()),
                    ParamValue::Entity(expr) => ctx.script_engine.resolve_entity(expr),
                    ParamValue::MacroParam { key, default } => {
                        let mp = ctx.script_engine.mp();
                        mp.get(key.as_ref())
                            .map(|v| v.to_string())
                            .or_else(|| default.as_deref().map(str::to_owned))
                            .unwrap_or_default()
                    }
                    ParamValue::MacroSplat => String::new(),
                };
                (k.to_owned(), val)
            })
        })
        .collect();
    KagEvent::Tag {
        name: tag.name.to_string(),
        params,
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_script;

    fn run_script(src: &str) -> (Vec<KagEvent>, RuntimeContext) {
        let (script, _diags) = parse_script(src, "test.ks");
        let mut ctx = RuntimeContext::new("test.ks");
        let mut all_events = Vec::new();

        for _ in 0..script.ops.len() + 2 {
            if ctx.pc >= script.ops.len() {
                break;
            }
            let events = execute_op(&script, &mut ctx).expect("execute failed");
            all_events.extend(events);
        }

        (all_events, ctx)
    }

    fn event_names(events: &[KagEvent]) -> Vec<String> {
        events
            .iter()
            .map(|e| match e {
                KagEvent::DisplayText { text, .. } => format!("text:{}", text),
                KagEvent::InsertLineBreak => "br".to_string(),
                KagEvent::ClearMessage => "cm".to_string(),
                KagEvent::ClearCurrentMessage => "clear_current".to_string(),
                KagEvent::WaitForClick { clear_after } => format!("wait_click:{}", clear_after),
                KagEvent::WaitMs(n) => format!("wait_ms:{}", n),
                KagEvent::Stop => "stop".to_string(),
                KagEvent::WaitForCompletion { tag, .. } => format!("wait_completion:{}", tag),
                KagEvent::WaitForRawClick => "wait_raw_click".to_string(),
                KagEvent::InputRequested { name, .. } => format!("input:{}", name),
                KagEvent::WaitForTrigger { name } => format!("wait_trig:{}", name),
                KagEvent::Jump { target, .. } => {
                    format!("jump:{}", target.as_deref().unwrap_or(""))
                }
                KagEvent::BeginChoices(_) => "choices".to_string(),
                KagEvent::EmbedText(s) => format!("emb:{}", s),
                KagEvent::Tag { name, .. } => format!("tag:{}", name),
                KagEvent::End => "end".to_string(),
                KagEvent::Warning(w) => format!("warn:{}", w),
                KagEvent::Error(e) => format!("err:{}", e),
                KagEvent::Return { storage } => format!("return:{}", storage),
                KagEvent::Trace(s) => format!("trace:{}", s),
                KagEvent::PushBacklog { text, join } => {
                    format!("pushlog:{}:{}", if *join { "join" } else { "add" }, text)
                }
                KagEvent::Snapshot(_) => "snapshot".to_string(),
            })
            .collect()
    }

    #[test]
    fn test_text_emit() {
        let (events, _) = run_script("Hello, world!\n");
        let names = event_names(&events);
        assert!(
            names.iter().any(|n| n.contains("Hello")),
            "events: {:?}",
            names
        );
    }

    #[test]
    fn test_wait_click_l() {
        let (events, _) = run_script("@l\n");
        let names = event_names(&events);
        assert!(
            names.contains(&"wait_click:false".to_string()),
            "{:?}",
            names
        );
    }

    #[test]
    fn test_wait_click_p() {
        let (events, _) = run_script("@p\n");
        let names = event_names(&events);
        assert!(
            names.contains(&"wait_click:true".to_string()),
            "{:?}",
            names
        );
    }

    #[test]
    fn test_line_break() {
        let (events, _) = run_script("@r\n");
        let names = event_names(&events);
        assert!(names.contains(&"br".to_string()), "{:?}", names);
    }

    #[test]
    fn test_stop_tag() {
        let (events, _) = run_script("@s\n");
        let names = event_names(&events);
        assert!(names.contains(&"stop".to_string()), "{:?}", names);
    }

    #[test]
    fn test_jump_tag() {
        let (events, _) = run_script("@jump storage=main target=*start\n");
        let names = event_names(&events);
        assert!(names.iter().any(|n| n.starts_with("jump:")), "{:?}", names);
    }

    #[test]
    fn test_wait_ms() {
        let (events, _) = run_script("@wait time=500\n");
        let names = event_names(&events);
        assert!(names.contains(&"wait_ms:500".to_string()), "{:?}", names);
    }

    #[test]
    fn test_if_true_branch() {
        let src = "[if exp=\"1 == 1\"]\nhello\n[endif]\n";
        let (events, _) = run_script(src);
        let names = event_names(&events);
        assert!(
            names.iter().any(|n| n.contains("hello")),
            "expected text in true branch: {:?}",
            names
        );
    }

    #[test]
    fn test_if_false_branch_skipped() {
        let src = "[if exp=\"1 == 2\"]\nhello\n[endif]\n";
        let (events, _) = run_script(src);
        let names = event_names(&events);
        assert!(
            !names.iter().any(|n| n.contains("hello")),
            "false branch should be skipped: {:?}",
            names
        );
    }

    #[test]
    fn test_if_else() {
        let src = "[if exp=\"false\"]\nA\n[else]\nB\n[endif]\n";
        let (events, _) = run_script(src);
        let names = event_names(&events);
        assert!(
            !names.iter().any(|n| n.contains("text:A")),
            "A should not appear: {:?}",
            names
        );
        assert!(
            names.iter().any(|n| n.contains("text:B")),
            "B should appear: {:?}",
            names
        );
    }

    #[test]
    fn test_if_elsif() {
        let src = "[if exp=\"1 == 2\"]\nA\n[elsif exp=\"1 == 1\"]\nB\n[endif]\n";
        let (events, _) = run_script(src);
        let names = event_names(&events);
        assert!(
            names.iter().any(|n| n.contains("text:B")),
            "elsif branch B expected: {:?}",
            names
        );
    }

    #[test]
    fn test_nested_if() {
        let src = "[if exp=\"true\"]\n[if exp=\"false\"]\ninner\n[endif]\nouter\n[endif]\n";
        let (events, _) = run_script(src);
        let names = event_names(&events);
        assert!(
            !names.iter().any(|n| n.contains("inner")),
            "inner should be skipped: {:?}",
            names
        );
        assert!(
            names.iter().any(|n| n.contains("outer")),
            "outer should appear: {:?}",
            names
        );
    }

    #[test]
    fn test_eval_tag_sets_variable() {
        let src = "[eval exp=\"f.x = 42;\"]\n";
        let (_, ctx) = run_script(src);
        let val = ctx.script_engine.get_f("x");
        assert!(val.is_some(), "f.x should be set");
    }

    #[test]
    fn test_emb_tag() {
        let src = "[eval exp=\"f.n = 7;\"]\n[emb exp=\"f.n\"]\n";
        let (events, _) = run_script(src);
        let names = event_names(&events);
        assert!(
            names.contains(&"emb:7".to_string()),
            "emb event expected: {:?}",
            names
        );
    }

    #[test]
    fn test_call_return() {
        // Script: main calls sub, sub runs body and returns, main prints after.
        // @s after "after" ensures we stop before re-entering sub.
        let src = "*main\n@call target=*sub\nafter\n@s\n*sub\nbody\n@return\n";
        let (script, _diags) = parse_script(src, "test.ks");
        let mut ctx = RuntimeContext::new("test.ks");
        let mut events = Vec::new();

        // Start at *main
        ctx.jump_to(*script.label_map.get("main").unwrap());

        for _ in 0..30 {
            if ctx.pc >= script.ops.len() {
                break;
            }
            let ev = execute_op(&script, &mut ctx).expect("execute");

            // Simulate runtime: follow Jump events (call/return)
            for e in &ev {
                if let KagEvent::Jump { target, .. } = e {
                    if let Some(t) = target {
                        let key = t.trim_start_matches('*');
                        if let Some(&idx) = script.label_map.get(key) {
                            ctx.jump_to(idx);
                        }
                    }
                }
                // Stop when we hit the @s after "after"
                if matches!(e, KagEvent::Stop) {
                    events.extend(ev.iter().cloned());
                    let names = event_names(&events);
                    assert!(
                        names.iter().any(|n| n.contains("body")),
                        "body from sub expected: {:?}",
                        names
                    );
                    assert!(
                        names.iter().any(|n| n.contains("after")),
                        "after from main expected: {:?}",
                        names
                    );
                    return;
                }
            }
            events.extend(ev);
        }

        let names = event_names(&events);
        assert!(
            names.iter().any(|n| n.contains("body")),
            "body from sub expected: {:?}",
            names
        );
        assert!(
            names.iter().any(|n| n.contains("after")),
            "after from main expected: {:?}",
            names
        );
    }

    #[test]
    fn test_generic_tag_forwarded() {
        let (events, _) = run_script("@bg storage=forest.png\n");
        let names = event_names(&events);
        assert!(
            names.iter().any(|n| n == "tag:bg"),
            "bg tag should be forwarded: {:?}",
            names
        );
    }

    #[test]
    fn test_macro_definition_and_call() {
        let src = "[macro name=greet]\nhello\n[endmacro]\n[greet]\n";
        let (events, _) = run_script(src);
        let names = event_names(&events);
        assert!(
            names.iter().any(|n| n.contains("hello")),
            "macro body should execute: {:?}",
            names
        );
    }

    #[test]
    fn test_cond_param_skips_tag() {
        let src = "[eval exp=\"f.x = 0;\"]\n[r cond=\"f.x == 1\"]\n";
        let (events, _) = run_script(src);
        let names = event_names(&events);
        assert!(
            !names.contains(&"br".to_string()),
            "cond=false should skip: {:?}",
            names
        );
    }

    #[test]
    fn test_iscript_block_executes() {
        let src = "[iscript]\nf.from_script = 99;\n[endscript]\n";
        let (_, ctx) = run_script(src);
        let val = ctx.script_engine.get_f("from_script");
        assert!(val.is_some(), "iscript variable should be set");
    }

    // ── New internal-state tag tests ──────────────────────────────────────────

    #[test]
    fn test_clearvar_clears_all_f() {
        let src = "[eval exp=\"f.a = 1; f.b = 2;\"]\n[clearvar]\n";
        let (_, ctx) = run_script(src);
        assert!(
            ctx.script_engine.f().is_empty(),
            "f should be empty after clearvar: {:?}",
            ctx.script_engine.f()
        );
    }

    #[test]
    fn test_clearvar_removes_specific_key() {
        let src = "[eval exp=\"f.keep = 1; f.remove = 2;\"]\n[clearvar exp=\"f.remove\"]\n";
        let (_, ctx) = run_script(src);
        let f = ctx.script_engine.f();
        assert!(f.contains_key("keep"), "f.keep should remain");
        assert!(!f.contains_key("remove"), "f.remove should be deleted");
    }

    #[test]
    fn test_clearsysvar_clears_sf() {
        let src = "[eval exp=\"sf.x = 99;\"]\n[clearsysvar]\n";
        let (_, ctx) = run_script(src);
        assert!(
            ctx.script_engine.sf().is_empty(),
            "sf should be empty after clearsysvar"
        );
    }

    #[test]
    fn test_clearstack_clears_call_stack() {
        // Build a context with a non-empty call stack, then run [clearstack stack=call]
        // and verify the stack is empty.
        let src = "@clearstack stack=call\n";
        let (script, _) = parse_script(src, "test.ks");
        let mut ctx = RuntimeContext::new("test.ks");

        // Manually push a fake call frame so there is something to clear
        ctx.push_call(42);
        assert_eq!(
            ctx.call_stack.len(),
            1,
            "should have one frame before clearstack"
        );

        let _ = execute_op(&script, &mut ctx).expect("execute");

        assert!(
            ctx.call_stack.is_empty(),
            "call stack should be empty after [clearstack stack=call]: {:?}",
            ctx.call_stack
        );
    }

    #[test]
    fn test_clearstack_clears_all_stacks() {
        let src = "@clearstack\n";
        let (script, _) = parse_script(src, "test.ks");
        let mut ctx = RuntimeContext::new("test.ks");

        ctx.push_call(1);
        ctx.push_if(true);
        assert!(!ctx.call_stack.is_empty());
        assert!(!ctx.if_stack.is_empty());

        let _ = execute_op(&script, &mut ctx).expect("execute");

        assert!(ctx.call_stack.is_empty(), "call stack cleared");
        assert!(ctx.if_stack.is_empty(), "if stack cleared");
    }

    #[test]
    fn test_erasemacro_prevents_invocation() {
        // The macro body (hello) lives in ops[0] and runs on the interpreter's first
        // iteration. After [erasemacro], a second [greet] call should be forwarded as
        // a generic tag (not re-enter the body). We verify `tag:greet` is emitted and
        // that the number of `text:hello` occurrences is exactly 1 (from body startup).
        let src = "[macro name=greet]\nhello\n[endmacro]\n[erasemacro name=greet]\n[greet]\n";
        let (events, _) = run_script(src);
        let names = event_names(&events);
        let hello_count = names.iter().filter(|n| n.as_str() == "text:hello").count();
        assert_eq!(
            hello_count, 0,
            "macro body is skipped at definition time, so 'hello' should never be emitted: {:?}",
            names
        );
        assert!(
            names.iter().any(|n| n == "tag:greet"),
            "erased macro should be forwarded as generic tag: {:?}",
            names
        );
    }

    #[test]
    fn test_trace_emits_trace_event() {
        let src = "[eval exp=\"f.val = 42;\"]\n[trace exp=\"f.val\"]\n";
        let (events, _) = run_script(src);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, KagEvent::Trace(s) if s.contains("42"))),
            "trace should emit Trace event with value: {:?}",
            events
        );
    }

    #[test]
    fn test_nowait_suppresses_l_wait() {
        let src = "[nowait]\n@l\nafter\n";
        let (events, _) = run_script(src);
        // With nowait active, [l] should not emit WaitForClick
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, KagEvent::WaitForClick { .. })),
            "WaitForClick should be suppressed by nowait: {:?}",
            events
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("after"))),
            "text after [l] should still appear: {:?}",
            events
        );
    }

    #[test]
    fn test_endnowait_restores_l_wait() {
        let src = "[nowait]\n[endnowait]\n@l\n";
        let (events, _) = run_script(src);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, KagEvent::WaitForClick { clear_after: false })),
            "WaitForClick should be restored after endnowait: {:?}",
            events
        );
    }

    /// Regression test: [p] must always emit `WaitForClick { clear_after: true }` so
    /// the host knows to clear the message window, even when nowait is active.
    ///
    /// Old code returned `Ok(vec![])` for [p] inside [nowait], silently dropping
    /// the clear signal.  New code always emits the event and relies on the host
    /// to auto-advance without blocking for real input.
    #[test]
    fn test_nowait_preserves_p_clear_signal() {
        // Both the bracketed and @-line forms of [p] should be covered.
        for src in &["[nowait]\n@p\nafter\n", "[nowait]\n[p]\nafter\n"] {
            let (events, _) = run_script(src);

            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, KagEvent::WaitForClick { clear_after: true })),
                "[p] inside [nowait] must still emit WaitForClick{{clear_after:true}}: {:?}",
                events
            );

            assert!(
                events.iter().any(
                    |e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("after"))
                ),
                "text after [p] should still appear when nowait is active: {:?}",
                events
            );
        }
    }

    #[test]
    fn test_delay_sets_text_speed() {
        let src = "[delay speed=50]\nhello\n";
        let (events, _) = run_script(src);
        assert!(
            events.iter().any(
                |e| matches!(e, KagEvent::DisplayText { text, speed: Some(50), .. } if text.contains("hello"))
            ),
            "DisplayText should carry speed=50: {:?}",
            events
        );
    }

    #[test]
    fn test_resetdelay_clears_speed() {
        let src = "[delay speed=50]\n[resetdelay]\nhello\n";
        let (events, _) = run_script(src);
        assert!(
            events.iter().any(
                |e| matches!(e, KagEvent::DisplayText { text, speed: None, .. } if text.contains("hello"))
            ),
            "DisplayText should have speed=None after resetdelay: {:?}",
            events
        );
    }

    #[test]
    fn test_nolog_sets_log_false() {
        let src = "[nolog]\nhidden\n[endnolog]\nvisible\n";
        let (events, _) = run_script(src);
        assert!(
            events.iter().any(
                |e| matches!(e, KagEvent::DisplayText { text, log: false, .. } if text.contains("hidden"))
            ),
            "text inside nolog should have log=false: {:?}",
            events
        );
        assert!(
            events.iter().any(
                |e| matches!(e, KagEvent::DisplayText { text, log: true, .. } if text.contains("visible"))
            ),
            "text after endnolog should have log=true: {:?}",
            events
        );
    }

    #[test]
    fn test_pushlog_emits_event() {
        let src = "[pushlog text=\"hello log\" join=false]\n";
        let (events, _) = run_script(src);
        assert!(
            events.iter().any(
                |e| matches!(e, KagEvent::PushBacklog { text, join: false } if text.contains("hello log"))
            ),
            "PushBacklog event expected: {:?}",
            events
        );
    }

    /// Regression: when a macro is defined twice, each `[macro]` header op must
    /// jump to *its own* `skip_to`, not the last definition's.  Before the fix
    /// the first `[macro]` header resolved the jump target by looking up the
    /// macro name in `macro_map`, which always returned the last definition's
    /// `body_end`, causing ops between the two definitions to be silently
    /// skipped.
    #[test]
    fn test_duplicate_macro_skip_target() {
        // Script layout:
        //   [macro name=greet] / v1 body / [endmacro]   <- first definition
        //   between                                       <- must NOT be skipped
        //   [macro name=greet] / v2 body / [endmacro]   <- second definition
        //   after                                         <- must appear
        let src = concat!(
            "[macro name=greet]\nv1\n[endmacro]\n",
            "between\n",
            "[macro name=greet]\nv2\n[endmacro]\n",
            "after\n",
        );
        let (events, _) = run_script(src);
        let names = event_names(&events);

        assert!(
            names.iter().any(|n| n.contains("between")),
            "ops between duplicate macro definitions must not be skipped: {:?}",
            names
        );
        assert!(
            names.iter().any(|n| n.contains("after")),
            "ops after the second definition must execute: {:?}",
            names
        );
        // Neither macro body should have been entered during definition skipping.
        assert!(
            !names.iter().any(|n| n.contains("v1") || n.contains("v2")),
            "macro bodies must not execute during definition: {:?}",
            names
        );
    }

    #[test]
    fn test_pushlog_join_true() {
        let src = "[pushlog text=\"appended\" join=true]\n";
        let (events, _) = run_script(src);
        assert!(
            events.iter().any(|e| matches!(
                e,
                KagEvent::PushBacklog {
                    text: _,
                    join: true
                }
            )),
            "PushBacklog with join=true expected: {:?}",
            events
        );
    }

    // ── Choice-system fix tests ───────────────────────────────────────────────

    /// Fix 1: text between [link] and [endlink] must be captured in the
    /// `ChoiceOption.text` field, not emitted as `DisplayText`.
    ///
    /// The KAG convention is to place each `@link` on its own line and close
    /// the entire group with a single `@endlink` at the end.
    #[test]
    fn test_link_text_captured_in_choice() {
        let src = "@link target=*a\nGo left\n@link target=*b\nGo right\n@endlink\n";
        let (events, _) = run_script(src);

        // No DisplayText events should appear for link labels
        assert!(
            !events.iter().any(
                |e| matches!(e, KagEvent::DisplayText { text, .. } if text.contains("Go left") || text.contains("Go right"))
            ),
            "link text must not be emitted as DisplayText: {:?}",
            events
        );

        // The BeginChoices event must carry the correct text
        let choices_event = events.iter().find_map(|e| {
            if let KagEvent::BeginChoices(c) = e {
                Some(c.clone())
            } else {
                None
            }
        });
        let choices = choices_event.expect("BeginChoices expected");
        assert_eq!(choices.len(), 2, "expected 2 choices: {:?}", choices);
        assert_eq!(choices[0].text, "Go left");
        assert_eq!(choices[1].text, "Go right");
    }

    /// Fix 2: a single [link]…[endlink] pair must emit BeginChoices.
    #[test]
    fn test_single_link_emits_begin_choices() {
        let src = "@link target=*a\nOnly option\n@endlink\n";
        let (events, _) = run_script(src);

        let choices_event = events.iter().find_map(|e| {
            if let KagEvent::BeginChoices(c) = e {
                Some(c.clone())
            } else {
                None
            }
        });
        let choices = choices_event.expect("BeginChoices expected for single link");
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].text, "Only option");
        assert_eq!(choices[0].target.as_deref(), Some("*a"));
    }

    // ── New tag coverage tests ────────────────────────────────────────────────

    #[test]
    fn test_ct_emits_clear_message_and_generic() {
        let (events, _) = run_script("@ct\n");
        let names = event_names(&events);
        assert!(
            names.contains(&"cm".to_string()),
            "ct must emit ClearMessage: {:?}",
            names
        );
        assert!(
            names.iter().any(|n| n == "tag:ct"),
            "ct must also emit generic tag: {:?}",
            names
        );
    }

    #[test]
    fn test_er_emits_clear_current_message() {
        let (events, _) = run_script("@er\n");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, KagEvent::ClearCurrentMessage)),
            "er must emit ClearCurrentMessage: {:?}",
            events
        );
    }

    #[test]
    fn test_ch_emits_display_text() {
        let (events, _) = run_script("[ch text=A]\n");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, KagEvent::DisplayText { text, .. } if text == "A")),
            "ch must emit DisplayText with the character: {:?}",
            events
        );
    }

    #[test]
    fn test_waitclick_emits_wait_for_raw_click() {
        let (events, _) = run_script("@waitclick\n");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, KagEvent::WaitForRawClick)),
            "waitclick must emit WaitForRawClick: {:?}",
            events
        );
    }

    #[test]
    fn test_wa_emits_wait_for_completion() {
        let (events, _) = run_script("[wa layer=0 seg=1]\n");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, KagEvent::WaitForCompletion { tag, .. } if tag == "wa")),
            "wa must emit WaitForCompletion: {:?}",
            events
        );
    }

    #[test]
    fn test_wt_emits_wait_for_completion() {
        let (events, _) = run_script("@wt\n");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, KagEvent::WaitForCompletion { tag, .. } if tag == "wt")),
            "wt must emit WaitForCompletion: {:?}",
            events
        );
    }

    #[test]
    fn test_input_emits_input_requested() {
        let (events, _) = run_script("[input name=f.user prompt=Enter title=Name]\n");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, KagEvent::InputRequested { name, .. } if name == "f.user")),
            "input must emit InputRequested: {:?}",
            events
        );
    }

    #[test]
    fn test_waittrig_emits_wait_for_trigger() {
        let (events, _) = run_script("[waittrig name=myevent]\n");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, KagEvent::WaitForTrigger { name } if name == "myevent")),
            "waittrig must emit WaitForTrigger: {:?}",
            events
        );
    }

    #[test]
    fn test_autowc_sets_ctx_state() {
        let src = "[autowc enabled=true ch=A,B time=3,5]\n";
        let (_, ctx) = run_script(src);
        assert!(ctx.autowc_enabled, "autowc_enabled should be true");
        assert_eq!(ctx.autowc_map.len(), 2);
        assert_eq!(ctx.autowc_map[0], ("A".to_string(), 3));
        assert_eq!(ctx.autowc_map[1], ("B".to_string(), 5));
    }

    #[test]
    fn test_autowc_disabled_clears_map() {
        let src = "[autowc enabled=true ch=X time=10]\n[autowc enabled=false]\n";
        let (_, ctx) = run_script(src);
        assert!(
            !ctx.autowc_enabled,
            "autowc_enabled should be false after disabling"
        );
        assert!(ctx.autowc_map.is_empty(), "autowc_map should be cleared");
    }

    #[test]
    fn test_clickskip_sets_ctx_state() {
        let src = "[clickskip enabled=false]\n";
        let (_, ctx) = run_script(src);
        assert!(!ctx.clickskip_enabled, "clickskip_enabled should be false");
    }

    #[test]
    fn test_click_sets_pending_click() {
        let src = "@click target=*dest\n";
        let (_, ctx) = run_script(src);
        assert!(ctx.pending_click.is_some(), "pending_click should be set");
        assert_eq!(ctx.pending_click.unwrap().target.as_deref(), Some("*dest"));
    }

    #[test]
    fn test_cclick_clears_pending_click() {
        let src = "@click target=*dest\n@cclick\n";
        let (_, ctx) = run_script(src);
        assert!(
            ctx.pending_click.is_none(),
            "pending_click should be cleared by cclick"
        );
    }

    #[test]
    fn test_timeout_sets_pending_timeout() {
        let src = "@timeout time=2000 target=*done\n";
        let (_, ctx) = run_script(src);
        let t = ctx.pending_timeout.expect("pending_timeout should be set");
        assert_eq!(t.time_ms, 2000);
        assert_eq!(t.target.as_deref(), Some("*done"));
    }

    #[test]
    fn test_ctimeout_clears_pending_timeout() {
        let src = "@timeout time=1000 target=*x\n@ctimeout\n";
        let (_, ctx) = run_script(src);
        assert!(
            ctx.pending_timeout.is_none(),
            "pending_timeout should be cleared by ctimeout"
        );
    }

    #[test]
    fn test_wheel_sets_pending_wheel() {
        let src = "@wheel target=*scroll\n";
        let (_, ctx) = run_script(src);
        assert!(ctx.pending_wheel.is_some());
        assert_eq!(
            ctx.pending_wheel.unwrap().target.as_deref(),
            Some("*scroll")
        );
    }

    #[test]
    fn test_cwheel_clears_pending_wheel() {
        let src = "@wheel target=*scroll\n@cwheel\n";
        let (_, ctx) = run_script(src);
        assert!(
            ctx.pending_wheel.is_none(),
            "cwheel should clear pending_wheel"
        );
    }

    #[test]
    fn test_resetwait_sets_base_time() {
        let src = "@resetwait\n";
        let (_, ctx) = run_script(src);
        assert!(
            ctx.wait_base_time.is_some(),
            "resetwait should set wait_base_time"
        );
    }

    #[test]
    fn test_wc_emits_wait_ms() {
        let (events, _) = run_script("@wc time=200\n");
        assert!(
            events.iter().any(|e| matches!(e, KagEvent::WaitMs(200))),
            "wc should emit WaitMs with the specified time: {:?}",
            events
        );
    }

    #[test]
    fn test_wait_mode_normal() {
        let (events, _) = run_script("@wait time=300\n");
        assert!(
            events.iter().any(|e| matches!(e, KagEvent::WaitMs(300))),
            "wait mode=normal should emit WaitMs(300): {:?}",
            events
        );
    }

    #[test]
    fn test_wait_mode_until_no_baseline_emits_nothing() {
        // With no [resetwait] baseline, elapsed >= time → no WaitMs emitted.
        let (events, _) = run_script("@wait time=0 mode=until\n");
        assert!(
            !events.iter().any(|e| matches!(e, KagEvent::WaitMs(_))),
            "wait mode=until with zero time should not emit WaitMs: {:?}",
            events
        );
    }
}
