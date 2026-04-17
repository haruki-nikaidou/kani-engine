//! Tag-execution logic for the KAG runtime.
//!
//! `execute_op` is a **synchronous** function that processes one op from the
//! script, mutates the `RuntimeContext` (PC, stacks, variables), and returns
//! any `KagEvent`s to be forwarded to the host.
//!
//! Async waiting (click waits, timers, choice input) is handled one level up
//! in the `KagInterpreter` actor.

use kag_syntax::tag_defs::{AttributeString, KnownTag, MaybeResolved, TagName};

use crate::ast::{Op, Param, ParamValue, Script, Tag, TextPart};
use crate::error::{DiagnosticCategory, InterpreterDiagnostic};
use crate::events::{ChoiceOption, KagEvent, ResolvedTag, TextSpan, TextStyle};

use super::context::{JumpTarget, RuntimeContext, TimeoutHandler};

/// Build a plain (unstyled) `DisplayText` event.
fn plain_display_text(
    text: String,
    speaker: Option<String>,
    speed: Option<u64>,
    log: bool,
) -> KagEvent {
    let spans = vec![TextSpan {
        text: text.clone(),
        style: TextStyle::default(),
    }];
    KagEvent::DisplayText {
        text,
        spans,
        speaker,
        speed,
        log,
    }
}

// ─── Public entry point ───────────────────────────────────────────────────────

/// Execute one op at `ctx.pc`.
///
/// On return `ctx.pc` has already been advanced (or redirected).
/// Always returns a list of events; fatal problems are emitted as
/// `KagEvent::Diagnostic` with `Error` severity followed by `KagEvent::End`.
pub fn execute_op<'s>(script: &Script<'s>, ctx: &mut RuntimeContext) -> Vec<KagEvent> {
    let pc = ctx.pc;
    if pc >= script.ops.len() {
        ctx.advance();
        return vec![KagEvent::End];
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
        return vec![];
    }

    // ── Normal execution ──────────────────────────────────────────────────────
    match op {
        Op::Text { parts, .. } => execute_text(ctx, parts),
        Op::Tag(tag) => execute_tag(script, ctx, tag),
        Op::Label(_) => {
            ctx.advance();
            vec![]
        }
        Op::ScriptBlock {
            content: script_text,
            ..
        } => {
            let script_text = script_text.clone();
            ctx.advance();
            match ctx.script_engine.exec(&script_text) {
                Ok(_) => vec![],
                Err(e) => {
                    tracing::error!("[kag] iscript block failed: {e}");
                    vec![KagEvent::Diagnostic(
                        InterpreterDiagnostic::warning(DiagnosticCategory::ScriptEval, e)
                            .at(ctx.current_storage.clone(), pc),
                    )]
                }
            }
        }
        // Skip past the macro body to the op after [endmacro].  skip_to was
        // encoded at compile time for *this specific definition*, so duplicate
        // macro names each jump to their own correct target.
        Op::MacroDef { skip_to, .. } => {
            ctx.jump_to(*skip_to);
            vec![]
        }
    }
}

// ─── Text op ─────────────────────────────────────────────────────────────────

fn execute_text<'s>(ctx: &mut RuntimeContext, parts: &[TextPart<'s>]) -> Vec<KagEvent> {
    use crate::inline_markup::{parse_inline_markup, spans_to_plain};

    let mut events = Vec::new();
    let mut text_buf = String::new();

    let speaker = ctx.current_speaker.take();
    let mut current_speed = ctx.text_speed;
    let mut current_log = ctx.log_enabled;

    /// Flush `text_buf` as a `DisplayText` event (with inline markup parsed).
    fn flush(
        text_buf: &mut String,
        events: &mut Vec<KagEvent>,
        speaker: Option<String>,
        speed: Option<u64>,
        log: bool,
    ) {
        let raw = std::mem::take(text_buf);
        let spans = parse_inline_markup(&raw);
        let text = spans_to_plain(&spans);
        events.push(KagEvent::DisplayText {
            text,
            spans,
            speaker,
            speed,
            log,
        });
    }

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
                        flush(
                            &mut text_buf,
                            &mut events,
                            speaker.clone(),
                            current_speed,
                            current_log,
                        );
                    }
                }
                // Execute the inline tag (may mutate ctx.text_speed / ctx.log_enabled)
                let mut inline_events = execute_inline_tag(ctx, tag);
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
            flush(
                &mut text_buf,
                &mut events,
                speaker,
                current_speed,
                current_log,
            );
        }
    }

    ctx.advance();
    events
}

// ─── Inline tag dispatch (occurs within text lines) ───────────────────────────

fn execute_inline_tag(ctx: &mut RuntimeContext, tag: &Tag<'_>) -> Vec<KagEvent> {
    // Honour optional `cond=` guard on any inline tag
    let cond_expr = tag.param_str("cond").map(str::to_owned);
    if let Some(ref expr) = cond_expr
        && !ctx.script_engine.eval_bool(expr).unwrap_or(true)
    {
        return vec![];
    }

    let mut diags = Vec::new();
    match KnownTag::from_tag(tag, &mut diags) {
        KnownTag::R {} => vec![KagEvent::InsertLineBreak],
        KnownTag::L {} => {
            if ctx.nowait {
                vec![]
            } else {
                vec![KagEvent::WaitForClick { clear_after: false }]
            }
        }
        // Always emit the clear signal; the host auto-advances when nowait is set.
        KnownTag::P {} => vec![KagEvent::WaitForClick { clear_after: true }],
        KnownTag::S {} => vec![KagEvent::Stop],
        KnownTag::Wait { time, .. } => {
            let ms = resolve_typed_field(ctx, time).unwrap_or(0);
            vec![KagEvent::WaitMs(ms)]
        }
        KnownTag::Eval { exp } => {
            let exp_str = resolve_str_field(ctx, exp).unwrap_or_default();
            if let Err(e) = ctx.script_engine.exec(&exp_str) {
                vec![KagEvent::Diagnostic(InterpreterDiagnostic::warning(
                    DiagnosticCategory::ScriptEval,
                    e,
                ))]
            } else {
                vec![]
            }
        }
        KnownTag::Emb { exp } => {
            let exp_str = resolve_str_field(ctx, exp).unwrap_or_default();
            let result = ctx
                .script_engine
                .eval_to_string(&exp_str)
                .unwrap_or_default();
            vec![KagEvent::EmbedText(result)]
        }
        KnownTag::Delay { speed } | KnownTag::Configdelay { speed } => {
            ctx.text_speed = Some(resolve_typed_field(ctx, speed).unwrap_or(0));
            vec![]
        }
        KnownTag::Resetdelay {} => {
            ctx.text_speed = None;
            vec![]
        }
        KnownTag::Nolog {} => {
            ctx.log_enabled = false;
            vec![]
        }
        KnownTag::Endnolog {} => {
            ctx.log_enabled = true;
            vec![]
        }
        known => vec![extension_event_from_tag(ctx, tag, &known)],
    }
}

// ─── Tag op dispatch ─────────────────────────────────────────────────────────

fn execute_tag<'s>(script: &Script<'s>, ctx: &mut RuntimeContext, tag: &Tag<'s>) -> Vec<KagEvent> {
    // Check optional `cond=` guard — if false, skip the tag entirely
    let cond_expr = tag.param_str("cond").map(str::to_owned);
    if let Some(ref expr) = cond_expr
        && !ctx.script_engine.eval_bool(expr).unwrap_or(true)
    {
        ctx.advance();
        return vec![];
    }

    let name = tag.name.as_ref();

    // ── Check if this is a macro invocation ────────────────────────────────
    // A macro that has been erased at runtime via [erasemacro] must not be invoked.
    if script.macro_map.contains_key(name) && !ctx.erased_macros.contains(name) {
        return invoke_macro(script, ctx, tag);
    }

    let pc = ctx.pc;
    ctx.advance();

    let mut parse_diags = Vec::new();
    let known = KnownTag::from_tag(tag, &mut parse_diags);

    let mut events: Vec<KagEvent> = parse_diags
        .into_iter()
        .filter(|d| d.severity == kag_syntax::error::Severity::Error)
        .map(|d| {
            KagEvent::Diagnostic(
                InterpreterDiagnostic::warning(DiagnosticCategory::Syntax, d.message.clone())
                    .at(ctx.current_storage.clone(), pc),
            )
        })
        .collect();

    let mut tag_events: Vec<KagEvent> = match known {
        // ── Text flow ──────────────────────────────────────────────────────
        KnownTag::L {} => {
            if ctx.nowait {
                vec![]
            } else {
                vec![KagEvent::WaitForClick { clear_after: false }]
            }
        }
        // Always emit the clear signal; the host auto-advances when nowait is set.
        KnownTag::P {} => vec![KagEvent::WaitForClick { clear_after: true }],
        KnownTag::R {} => vec![KagEvent::InsertLineBreak],
        KnownTag::S {} => vec![KagEvent::Stop],
        KnownTag::Cm {} => vec![KagEvent::ClearMessage],

        // ── Timed wait ─────────────────────────────────────────────────────
        KnownTag::Wait { time, .. } => {
            let ms = resolve_typed_field(ctx, time).unwrap_or(0);
            let mode = tag.param_str("mode").unwrap_or("normal");
            if mode == "until" {
                let elapsed = ctx
                    .wait_base_time
                    .map(|t| t.elapsed().as_millis() as u64)
                    .unwrap_or(ms);
                let remaining = ms.saturating_sub(elapsed);
                if remaining == 0 {
                    vec![]
                } else {
                    vec![KagEvent::WaitMs(remaining)]
                }
            } else {
                vec![KagEvent::WaitMs(ms)]
            }
        }

        // ── Navigation ────────────────────────────────────────────────────
        KnownTag::Jump { storage, target } => {
            let storage = resolve_str_field(ctx, storage);
            let target = resolve_str_field(ctx, target);
            vec![KagEvent::Jump { storage, target }]
        }

        KnownTag::Call { storage, target } => {
            let return_pc = ctx.pc; // already advanced
            ctx.push_call(return_pc);
            let storage = resolve_str_field(ctx, storage);
            let target = resolve_str_field(ctx, target);
            vec![KagEvent::Jump { storage, target }]
        }

        KnownTag::Return {} => {
            if let Some(frame) = ctx.pop_call() {
                ctx.jump_to(frame.return_pc);
                if frame.return_storage != ctx.current_storage {
                    vec![KagEvent::Return {
                        storage: frame.return_storage,
                    }]
                } else {
                    vec![]
                }
            } else {
                // Fatal: call stack underflow
                vec![
                    KagEvent::Diagnostic(
                        InterpreterDiagnostic::error(
                            DiagnosticCategory::CallStack,
                            "[return] without matching [call]",
                        )
                        .at(ctx.current_storage.clone(), pc),
                    ),
                    KagEvent::End,
                ]
            }
        }

        // ── Eval / emb ────────────────────────────────────────────────────
        KnownTag::Eval { exp } => {
            let exp_str = resolve_str_field(ctx, exp).unwrap_or_default();
            let next = tag.param_str("next").unwrap_or("true");
            let result = ctx.script_engine.exec(&exp_str);
            let mut ev = Vec::new();
            if let Err(e) = result {
                tracing::warn!("[kag] [eval] expression failed: {e}");
                ev.push(KagEvent::Diagnostic(
                    InterpreterDiagnostic::warning(DiagnosticCategory::ScriptEval, e)
                        .at(ctx.current_storage.clone(), pc),
                ));
            }
            if next == "false" {
                ev.push(KagEvent::Stop);
            }
            ev
        }

        KnownTag::Emb { exp } => {
            let exp_str = resolve_str_field(ctx, exp).unwrap_or_default();
            let result = ctx
                .script_engine
                .eval_to_string(&exp_str)
                .unwrap_or_default();
            vec![KagEvent::EmbedText(result)]
        }

        // ── Choice links ──────────────────────────────────────────────────
        KnownTag::Link {
            storage, target, ..
        } => {
            ctx.in_link = true;
            let storage = resolve_str_field(ctx, storage);
            let target = resolve_str_field(ctx, target);
            let exp = tag.param_str("exp").map(str::to_owned);
            ctx.pending_choices
                .push(crate::runtime::context::PendingChoice {
                    text: String::new(),
                    storage,
                    target,
                    exp,
                });
            vec![KagEvent::Tag(ResolvedTag::Extension {
                name: "link".to_owned(),
                params: resolve_raw_params(ctx, &tag.params),
            })]
        }

        KnownTag::Endlink {} => {
            ctx.in_link = false;
            if !ctx.pending_choices.is_empty() {
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
                vec![KagEvent::BeginChoices(choices)]
            } else {
                vec![KagEvent::Tag(ResolvedTag::Extension {
                    name: "endlink".to_owned(),
                    params: vec![],
                })]
            }
        }

        KnownTag::Glink {
            storage,
            target,
            text,
        } => {
            let text_val = resolve_str_field(ctx, text).unwrap_or_default();
            let storage = resolve_str_field(ctx, storage);
            let target = resolve_str_field(ctx, target);
            let exp = tag.param_str("exp").map(str::to_owned);
            vec![KagEvent::BeginChoices(vec![ChoiceOption {
                text: text_val,
                storage,
                target,
                exp,
            }])]
        }

        // ── Character nameplate ───────────────────────────────────────────
        KnownTag::CharaPtext { name } => {
            if let Some(name_val) = resolve_str_field(ctx, name) {
                ctx.current_speaker = Some(name_val);
            }
            vec![KagEvent::Tag(ResolvedTag::Extension {
                name: "chara_ptext".to_owned(),
                params: resolve_raw_params(ctx, &tag.params),
            })]
        }

        KnownTag::Endmacro {} => {
            if let Some(frame) = ctx.pop_macro() {
                ctx.jump_to(frame.return_pc);
            }
            vec![]
        }

        // ── Variable clearing ─────────────────────────────────────────────
        KnownTag::Clearvar {} => {
            let exp = tag.param_str("exp").unwrap_or("").trim().to_owned();
            if exp.is_empty() {
                ctx.script_engine.clear_f();
                ctx.script_engine.clear_tf();
            } else {
                remove_var_by_expr(ctx, &exp);
            }
            vec![]
        }

        KnownTag::Clearsysvar {} => {
            ctx.script_engine.clear_sf();
            vec![]
        }

        // ── Stack clearing ────────────────────────────────────────────────
        KnownTag::Clearstack {} => {
            let which = tag.param_str("stack").unwrap_or("").trim().to_owned();
            ctx.clear_stack(&which);
            vec![]
        }

        // ── Macro deletion ────────────────────────────────────────────────
        KnownTag::Erasemacro { name } => {
            let name_val = resolve_str_field(ctx, name).unwrap_or_default();
            if !name_val.is_empty() {
                ctx.erased_macros.insert(name_val);
            }
            vec![]
        }

        // ── Debug trace ───────────────────────────────────────────────────
        KnownTag::Trace { exp } => {
            let exp_str = resolve_str_field(ctx, exp).unwrap_or_default();
            let result = ctx
                .script_engine
                .eval_to_string(&exp_str)
                .unwrap_or_default();
            vec![KagEvent::Trace(result)]
        }

        // ── Nowait mode ───────────────────────────────────────────────────
        KnownTag::Nowait {} => {
            ctx.nowait = true;
            vec![]
        }
        KnownTag::Endnowait {} => {
            ctx.nowait = false;
            vec![]
        }

        // ── Text display speed ────────────────────────────────────────────
        KnownTag::Delay { speed } | KnownTag::Configdelay { speed } => {
            ctx.text_speed = Some(resolve_typed_field(ctx, speed).unwrap_or(0));
            vec![]
        }
        KnownTag::Resetdelay {} => {
            ctx.text_speed = None;
            vec![]
        }

        // ── Backlog control ───────────────────────────────────────────────
        KnownTag::Nolog {} => {
            ctx.log_enabled = false;
            vec![]
        }
        KnownTag::Endnolog {} => {
            ctx.log_enabled = true;
            vec![]
        }
        KnownTag::Pushlog { text, join } => {
            let text_val = resolve_str_field(ctx, text).unwrap_or_default();
            let join_val = resolve_typed_field(ctx, join).unwrap_or(false);
            vec![KagEvent::PushBacklog {
                text: text_val,
                join: join_val,
            }]
        }

        // ── Message-layer clear variants ──────────────────────────────────
        KnownTag::Ct {} => vec![
            KagEvent::ClearMessage,
            KagEvent::Tag(ResolvedTag::Extension {
                name: "ct".to_owned(),
                params: vec![],
            }),
        ],

        KnownTag::Er {} => vec![KagEvent::ClearCurrentMessage],

        // ── Single-character display ──────────────────────────────────────
        KnownTag::Ch { text } => {
            let text_val = resolve_str_field(ctx, text).unwrap_or_default();
            if text_val.is_empty() {
                vec![]
            } else {
                vec![plain_display_text(
                    text_val,
                    ctx.current_speaker.clone(),
                    ctx.text_speed,
                    ctx.log_enabled,
                )]
            }
        }

        KnownTag::Hch { text } => {
            let text_val = resolve_str_field(ctx, text).unwrap_or_default();
            let mut ev = vec![KagEvent::Tag(ResolvedTag::Extension {
                name: "hch".to_owned(),
                params: resolve_raw_params(ctx, &tag.params),
            })];
            if !text_val.is_empty() {
                ev.push(plain_display_text(
                    text_val,
                    ctx.current_speaker.clone(),
                    ctx.text_speed,
                    ctx.log_enabled,
                ));
            }
            ev
        }

        // ── [autowc] — configure automatic per-character waits ────────────
        KnownTag::Autowc { .. } => {
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
            vec![]
        }

        // ── [wc] — wait for N characters of display time ──────────────────
        KnownTag::Wc { time } => {
            let time_ms = resolve_typed_field(ctx, time).unwrap_or(0);
            vec![KagEvent::WaitMs(time_ms)]
        }

        // ── [clickskip] — toggle click-skip mode ─────────────────────────
        KnownTag::Clickskip { enabled } => {
            let enabled_val = resolve_typed_field(ctx, enabled).unwrap_or(true);
            ctx.clickskip_enabled = enabled_val;
            vec![KagEvent::Tag(ResolvedTag::Extension {
                name: "clickskip".to_owned(),
                params: resolve_raw_params(ctx, &tag.params),
            })]
        }

        // ── [resetwait] — set wait baseline for mode=until ───────────────
        KnownTag::Resetwait {} => {
            ctx.wait_base_time = Some(std::time::Instant::now());
            vec![]
        }

        // ── [click]/[cclick] — register/clear click handler at [s] ──────
        KnownTag::Click {
            storage,
            target,
            exp,
        } => {
            let storage = resolve_str_field(ctx, storage);
            let target = resolve_str_field(ctx, target);
            let exp_val = resolve_str_field(ctx, exp);
            ctx.pending_click = Some(JumpTarget {
                storage,
                target,
                exp: exp_val,
            });
            vec![]
        }
        KnownTag::Cclick {} => {
            ctx.pending_click = None;
            vec![]
        }

        // ── [timeout]/[ctimeout] — register/clear timeout handler ────────
        KnownTag::Timeout {
            time,
            storage,
            target,
        } => {
            let time_ms = resolve_typed_field(ctx, time).unwrap_or(0);
            let storage = resolve_str_field(ctx, storage);
            let target = resolve_str_field(ctx, target);
            let exp_val = tag.param_str("exp").map(str::to_owned);
            ctx.pending_timeout = Some(TimeoutHandler {
                time_ms,
                storage,
                target,
                exp: exp_val,
            });
            vec![]
        }
        KnownTag::Ctimeout {} => {
            ctx.pending_timeout = None;
            vec![]
        }

        // ── [wheel]/[cwheel] — register/clear wheel handler ──────────────
        KnownTag::Wheel {
            storage,
            target,
            exp,
        } => {
            let storage = resolve_str_field(ctx, storage);
            let target = resolve_str_field(ctx, target);
            let exp_val = resolve_str_field(ctx, exp);
            ctx.pending_wheel = Some(JumpTarget {
                storage,
                target,
                exp: exp_val,
            });
            vec![]
        }
        KnownTag::Cwheel {} => {
            ctx.pending_wheel = None;
            vec![]
        }

        // ── Blocking wait tags ────────────────────────────────────────────
        KnownTag::WaitForCompletion {
            which,
            canskip,
            buf,
        } => {
            vec![KagEvent::WaitForCompletion {
                which,
                canskip: resolve_typed_field(ctx, canskip),
                buf: resolve_typed_field(ctx, buf),
            }]
        }

        KnownTag::Waitclick {} => vec![KagEvent::WaitForRawClick],

        // ── [input] — text-input dialog ───────────────────────────────────
        KnownTag::Input {
            name,
            prompt,
            title,
        } => {
            let var_name = resolve_str_field(ctx, name).unwrap_or_default();
            let prompt_val = resolve_str_field(ctx, prompt).unwrap_or_default();
            let title_val = resolve_str_field(ctx, title).unwrap_or_default();
            vec![KagEvent::InputRequested {
                name: var_name,
                prompt: prompt_val,
                title: title_val,
            }]
        }

        // ── [waittrig] — wait for a named trigger ─────────────────────────
        KnownTag::Waittrig { name } => {
            let trig_name = resolve_str_field(ctx, name).unwrap_or_default();
            vec![KagEvent::WaitForTrigger { name: trig_name }]
        }

        // ── Image / layer tags ────────────────────────────────────────────
        KnownTag::Bg {
            storage,
            time,
            method,
        } => vec![KagEvent::Tag(ResolvedTag::Bg {
            storage: resolve_str_field(ctx, storage),
            time: resolve_typed_field(ctx, time),
            method: resolve_str_field(ctx, method),
        })],
        KnownTag::Image {
            storage,
            layer,
            x,
            y,
            visible,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Image {
                storage: resolve_str_field(ctx, storage),
                layer: resolve_str_field(ctx, layer),
                x: resolve_typed_field(ctx, x),
                y: resolve_typed_field(ctx, y),
                visible: resolve_typed_field(ctx, visible),
            })]
        }
        KnownTag::Layopt {
            layer,
            visible,
            opacity,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Layopt {
                layer: resolve_str_field(ctx, layer),
                visible: resolve_typed_field(ctx, visible),
                opacity: resolve_typed_field(ctx, opacity),
            })]
        }
        KnownTag::Free { layer }
        | KnownTag::Freeimage { layer }
        | KnownTag::Freelayer { layer } => {
            vec![KagEvent::Tag(ResolvedTag::Free {
                layer: resolve_str_field(ctx, layer),
            })]
        }
        KnownTag::Position { layer, x, y } => vec![KagEvent::Tag(ResolvedTag::Position {
            layer: resolve_str_field(ctx, layer),
            x: resolve_typed_field(ctx, x),
            y: resolve_typed_field(ctx, y),
        })],
        KnownTag::Backlay {} => vec![KagEvent::Tag(ResolvedTag::Backlay)],
        KnownTag::Current { layer } => vec![KagEvent::Tag(ResolvedTag::Current {
            layer: resolve_str_field(ctx, layer),
        })],
        KnownTag::Locate { x, y } => vec![KagEvent::Tag(ResolvedTag::Locate {
            x: resolve_typed_field(ctx, x),
            y: resolve_typed_field(ctx, y),
        })],
        KnownTag::Layermode { layer, mode } => vec![KagEvent::Tag(ResolvedTag::Layermode {
            layer: resolve_str_field(ctx, layer),
            mode: resolve_str_field(ctx, mode),
        })],
        KnownTag::FreeLayermode { layer } => vec![KagEvent::Tag(ResolvedTag::FreeLayermode {
            layer: resolve_str_field(ctx, layer),
        })],
        KnownTag::Filter { layer, r#type } => vec![KagEvent::Tag(ResolvedTag::Filter {
            layer: resolve_str_field(ctx, layer),
            filter_type: resolve_str_field(ctx, r#type),
        })],
        KnownTag::FreeFilter { layer } => vec![KagEvent::Tag(ResolvedTag::FreeFilter {
            layer: resolve_str_field(ctx, layer),
        })],
        KnownTag::PositionFilter { layer, x, y } => {
            vec![KagEvent::Tag(ResolvedTag::PositionFilter {
                layer: resolve_str_field(ctx, layer),
                x: resolve_typed_field(ctx, x),
                y: resolve_typed_field(ctx, y),
            })]
        }
        KnownTag::Mask { layer, storage } => vec![KagEvent::Tag(ResolvedTag::Mask {
            layer: resolve_str_field(ctx, layer),
            storage: resolve_str_field(ctx, storage),
        })],
        KnownTag::MaskOff { layer } => vec![KagEvent::Tag(ResolvedTag::MaskOff {
            layer: resolve_str_field(ctx, layer),
        })],
        KnownTag::Graph {
            layer,
            shape,
            x,
            y,
            width,
            height,
            color,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Graph {
                layer: resolve_str_field(ctx, layer),
                shape: resolve_str_field(ctx, shape),
                x: resolve_typed_field(ctx, x),
                y: resolve_typed_field(ctx, y),
                width: resolve_typed_field(ctx, width),
                height: resolve_typed_field(ctx, height),
                color: resolve_str_field(ctx, color),
            })]
        }

        // ── Audio tags ────────────────────────────────────────────────────
        KnownTag::Bgm {
            storage,
            r#loop,
            volume,
            fadetime,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Bgm {
                storage: resolve_str_field(ctx, storage),
                looping: resolve_typed_field(ctx, r#loop).unwrap_or(true),
                volume: resolve_typed_field(ctx, volume),
                fadetime: resolve_typed_field(ctx, fadetime),
            })]
        }
        KnownTag::Stopbgm { fadetime } => vec![KagEvent::Tag(ResolvedTag::Stopbgm {
            fadetime: resolve_typed_field(ctx, fadetime),
        })],
        // fadeinbgm maps to Bgm with the time arg as fadetime
        KnownTag::Fadeinbgm { storage, time } => vec![KagEvent::Tag(ResolvedTag::Bgm {
            storage: resolve_str_field(ctx, storage),
            looping: true,
            volume: None,
            fadetime: resolve_typed_field(ctx, time),
        })],
        // fadeoutbgm maps to Stopbgm with the time arg as fadetime
        KnownTag::Fadeoutbgm { time } => vec![KagEvent::Tag(ResolvedTag::Stopbgm {
            fadetime: resolve_typed_field(ctx, time),
        })],
        KnownTag::Pausebgm { buf } => vec![KagEvent::Tag(ResolvedTag::Pausebgm {
            buf: resolve_typed_field(ctx, buf),
        })],
        KnownTag::Resumebgm { buf } => vec![KagEvent::Tag(ResolvedTag::Resumebgm {
            buf: resolve_typed_field(ctx, buf),
        })],
        KnownTag::Fadebgm { time, volume } => vec![KagEvent::Tag(ResolvedTag::Fadebgm {
            time: resolve_typed_field(ctx, time),
            volume: resolve_typed_field(ctx, volume),
        })],
        KnownTag::Xchgbgm { storage, time } => vec![KagEvent::Tag(ResolvedTag::Xchgbgm {
            storage: resolve_str_field(ctx, storage),
            time: resolve_typed_field(ctx, time),
        })],
        KnownTag::Bgmopt { r#loop, seek } => vec![KagEvent::Tag(ResolvedTag::Bgmopt {
            looping: resolve_typed_field(ctx, r#loop),
            seek: resolve_str_field(ctx, seek),
        })],
        KnownTag::Se {
            storage,
            buf,
            volume,
            r#loop,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Se {
                storage: resolve_str_field(ctx, storage),
                buf: resolve_typed_field(ctx, buf),
                volume: resolve_typed_field(ctx, volume),
                looping: resolve_typed_field(ctx, r#loop).unwrap_or(false),
            })]
        }
        KnownTag::Stopse { buf } => vec![KagEvent::Tag(ResolvedTag::Stopse {
            buf: resolve_typed_field(ctx, buf),
        })],
        KnownTag::Pausese { buf } => vec![KagEvent::Tag(ResolvedTag::Pausese {
            buf: resolve_typed_field(ctx, buf),
        })],
        KnownTag::Resumese { buf } => vec![KagEvent::Tag(ResolvedTag::Resumese {
            buf: resolve_typed_field(ctx, buf),
        })],
        KnownTag::Seopt { buf, r#loop } => vec![KagEvent::Tag(ResolvedTag::Seopt {
            buf: resolve_typed_field(ctx, buf),
            looping: resolve_typed_field(ctx, r#loop),
        })],
        KnownTag::Vo { storage, buf } => vec![KagEvent::Tag(ResolvedTag::Vo {
            storage: resolve_str_field(ctx, storage),
            buf: resolve_typed_field(ctx, buf),
        })],
        KnownTag::Changevol { target, vol, time } => {
            vec![KagEvent::Tag(ResolvedTag::Changevol {
                target: resolve_str_field(ctx, target),
                vol: resolve_typed_field(ctx, vol),
                time: resolve_typed_field(ctx, time),
            })]
        }

        // ── Animation tags ────────────────────────────────────────────────
        KnownTag::Anim {
            layer,
            preset,
            time,
            r#loop,
            delay,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Anim {
                layer: resolve_str_field(ctx, layer),
                preset: resolve_str_field(ctx, preset),
                time: resolve_typed_field(ctx, time),
                looping: resolve_typed_field(ctx, r#loop).unwrap_or(false),
                delay: resolve_typed_field(ctx, delay),
            })]
        }
        KnownTag::Stopanim { layer } => vec![KagEvent::Tag(ResolvedTag::StopAnim {
            layer: resolve_str_field(ctx, layer),
        })],
        KnownTag::Keyframe { name } => {
            if let Some(n) = resolve_str_field(ctx, name) {
                ctx.keyframe_defs.insert(n.clone(), Vec::new());
                ctx.current_keyframe_name = Some(n);
            }
            vec![]
        }
        KnownTag::Frame {
            time,
            opacity,
            x,
            y,
        } => {
            let frame = crate::events::FrameSpec {
                time: resolve_typed_field(ctx, time).unwrap_or(0),
                opacity: resolve_typed_field(ctx, opacity),
                x: resolve_typed_field(ctx, x),
                y: resolve_typed_field(ctx, y),
            };
            if let Some(name) = &ctx.current_keyframe_name {
                if let Some(seq) = ctx.keyframe_defs.get_mut(name) {
                    seq.push(frame);
                }
            }
            vec![]
        }
        KnownTag::Endkeyframe {} => {
            ctx.current_keyframe_name = None;
            vec![]
        }
        KnownTag::Kanim {
            layer,
            name,
            r#loop,
        } => {
            let frames = resolve_str_field(ctx, name)
                .and_then(|n| ctx.keyframe_defs.get(&n).cloned())
                .unwrap_or_default();
            vec![KagEvent::Tag(ResolvedTag::Kanim {
                layer: resolve_str_field(ctx, layer),
                frames,
                looping: resolve_typed_field(ctx, r#loop).unwrap_or(false),
            })]
        }
        KnownTag::StopKanim { layer } => vec![KagEvent::Tag(ResolvedTag::StopKanim {
            layer: resolve_str_field(ctx, layer),
        })],
        KnownTag::Xanim {
            layer,
            name,
            r#loop,
        } => {
            let frames = resolve_str_field(ctx, name)
                .and_then(|n| ctx.keyframe_defs.get(&n).cloned())
                .unwrap_or_default();
            vec![KagEvent::Tag(ResolvedTag::Xanim {
                layer: resolve_str_field(ctx, layer),
                frames,
                looping: resolve_typed_field(ctx, r#loop).unwrap_or(false),
            })]
        }
        KnownTag::StopXanim { layer } => vec![KagEvent::Tag(ResolvedTag::StopXanim {
            layer: resolve_str_field(ctx, layer),
        })],

        // ── Video / Movie tags ────────────────────────────────────────────
        KnownTag::Bgmovie {
            storage,
            r#loop,
            volume,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Bgmovie {
                storage: resolve_str_field(ctx, storage),
                looping: resolve_typed_field(ctx, r#loop).unwrap_or(false),
                volume: resolve_typed_field(ctx, volume),
            })]
        }
        KnownTag::StopBgmovie {} => vec![KagEvent::Tag(ResolvedTag::StopBgmovie)],
        KnownTag::Movie {
            storage,
            x,
            y,
            width,
            height,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Movie {
                storage: resolve_str_field(ctx, storage),
                x: resolve_typed_field(ctx, x),
                y: resolve_typed_field(ctx, y),
                width: resolve_typed_field(ctx, width),
                height: resolve_typed_field(ctx, height),
            })]
        }

        // ── Transition tags ───────────────────────────────────────────────
        KnownTag::Trans { method, time, rule } => vec![KagEvent::Tag(ResolvedTag::Trans {
            method: resolve_str_field(ctx, method),
            time: resolve_typed_field(ctx, time),
            rule: resolve_str_field(ctx, rule),
        })],
        KnownTag::Fadein { time, color } => vec![KagEvent::Tag(ResolvedTag::Fadein {
            time: resolve_typed_field(ctx, time),
            color: resolve_str_field(ctx, color),
        })],
        KnownTag::Fadeout { time, color } => vec![KagEvent::Tag(ResolvedTag::Fadeout {
            time: resolve_typed_field(ctx, time),
            color: resolve_str_field(ctx, color),
        })],
        KnownTag::Movetrans { layer, time, x, y } => {
            vec![KagEvent::Tag(ResolvedTag::Movetrans {
                layer: resolve_str_field(ctx, layer),
                time: resolve_typed_field(ctx, time),
                x: resolve_typed_field(ctx, x),
                y: resolve_typed_field(ctx, y),
            })]
        }

        // ── Effect tags ───────────────────────────────────────────────────
        KnownTag::Quake { time, hmax, vmax } => vec![KagEvent::Tag(ResolvedTag::Quake {
            time: resolve_typed_field(ctx, time),
            hmax: resolve_typed_field(ctx, hmax),
            vmax: resolve_typed_field(ctx, vmax),
        })],
        KnownTag::Shake { time, amount, axis } => vec![KagEvent::Tag(ResolvedTag::Shake {
            time: resolve_typed_field(ctx, time),
            amount: resolve_typed_field(ctx, amount),
            axis: resolve_str_field(ctx, axis),
        })],
        KnownTag::Flash { time, color } => vec![KagEvent::Tag(ResolvedTag::Flash {
            time: resolve_typed_field(ctx, time),
            color: resolve_str_field(ctx, color),
        })],

        // ── Message window tags ───────────────────────────────────────────
        KnownTag::Msgwnd { visible, layer } => vec![KagEvent::Tag(ResolvedTag::Msgwnd {
            visible: resolve_typed_field(ctx, visible),
            layer: resolve_str_field(ctx, layer),
        })],
        KnownTag::Wndctrl {
            x,
            y,
            width,
            height,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Wndctrl {
                x: resolve_typed_field(ctx, x),
                y: resolve_typed_field(ctx, y),
                width: resolve_typed_field(ctx, width),
                height: resolve_typed_field(ctx, height),
            })]
        }
        KnownTag::Resetfont {} => vec![KagEvent::Tag(ResolvedTag::Resetfont)],
        KnownTag::Font {
            face,
            size,
            bold,
            italic,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Font {
                face: resolve_str_field(ctx, face),
                size: resolve_typed_field(ctx, size),
                bold: resolve_typed_field(ctx, bold),
                italic: resolve_typed_field(ctx, italic),
            })]
        }
        KnownTag::Size { value } => vec![KagEvent::Tag(ResolvedTag::Size {
            value: resolve_typed_field(ctx, value),
        })],
        KnownTag::Bold { value } => vec![KagEvent::Tag(ResolvedTag::Bold {
            value: resolve_typed_field(ctx, value),
        })],
        KnownTag::Italic { value } => vec![KagEvent::Tag(ResolvedTag::Italic {
            value: resolve_typed_field(ctx, value),
        })],
        KnownTag::Ruby { text } => vec![KagEvent::Tag(ResolvedTag::Ruby {
            text: resolve_str_field(ctx, text),
        })],
        KnownTag::Nowrap {} => vec![KagEvent::Tag(ResolvedTag::Nowrap { enabled: true })],
        KnownTag::Endnowrap {} => {
            vec![KagEvent::Tag(ResolvedTag::Nowrap { enabled: false })]
        }

        // ── Character sprite tags ─────────────────────────────────────────
        KnownTag::CharaNew {
            name,
            storage,
            width,
            height,
        } => {
            if let Some(n) = resolve_str_field(ctx, name) {
                let def = crate::runtime::context::CharaDef {
                    name: n.clone(),
                    storage: resolve_str_field(ctx, storage),
                    width: resolve_typed_field(ctx, width),
                    height: resolve_typed_field(ctx, height),
                    faces: Vec::new(),
                };
                ctx.chara_registry.insert(n, def);
            }
            vec![]
        }
        KnownTag::CharaFace {
            name,
            face,
            storage,
        } => {
            if let (Some(n), Some(f), Some(s)) = (
                resolve_str_field(ctx, name),
                resolve_str_field(ctx, face),
                resolve_str_field(ctx, storage),
            ) {
                if let Some(def) = ctx.chara_registry.get_mut(&n) {
                    def.faces.retain(|v| v.face != f);
                    def.faces.push(crate::runtime::context::FaceVariant {
                        face: f,
                        storage: s,
                    });
                }
            }
            vec![]
        }
        KnownTag::CharaConfig { name } => {
            let name_val = resolve_str_field(ctx, name);
            vec![KagEvent::Tag(ResolvedTag::Extension {
                name: "chara_config".to_owned(),
                params: if let Some(n) = name_val {
                    vec![("name".to_owned(), n)]
                } else {
                    vec![]
                },
            })]
        }
        KnownTag::CharaShow {
            name,
            face,
            x,
            y,
            time,
            method,
        } => {
            let name_val = resolve_str_field(ctx, name);
            let face_val = resolve_str_field(ctx, face);
            let storage = name_val
                .as_deref()
                .and_then(|n| ctx.chara_registry.get(n))
                .and_then(|def| def.resolve_face(face_val.as_deref()));
            vec![KagEvent::Tag(ResolvedTag::CharaShow {
                name: name_val,
                storage,
                x: resolve_typed_field(ctx, x),
                y: resolve_typed_field(ctx, y),
                time: resolve_typed_field(ctx, time),
                method: resolve_str_field(ctx, method),
            })]
        }
        KnownTag::CharaHide { name, time, method } => {
            vec![KagEvent::Tag(ResolvedTag::CharaHide {
                name: resolve_str_field(ctx, name),
                time: resolve_typed_field(ctx, time),
                method: resolve_str_field(ctx, method),
            })]
        }
        KnownTag::CharaHideAll { time, method } => {
            vec![KagEvent::Tag(ResolvedTag::CharaHideAll {
                time: resolve_typed_field(ctx, time),
                method: resolve_str_field(ctx, method),
            })]
        }
        KnownTag::CharaFree { name } => {
            vec![KagEvent::Tag(ResolvedTag::CharaFree {
                name: resolve_str_field(ctx, name),
            })]
        }
        KnownTag::CharaDelete { name } => {
            let name_val = resolve_str_field(ctx, name);
            if let Some(n) = &name_val {
                ctx.chara_registry.remove(n);
            }
            vec![KagEvent::Tag(ResolvedTag::CharaDelete { name: name_val })]
        }
        KnownTag::CharaMod {
            name,
            face,
            pose,
            storage,
        } => {
            let name_val = resolve_str_field(ctx, name);
            let face_val = resolve_str_field(ctx, face);
            let direct_storage = resolve_str_field(ctx, storage);
            let resolved_storage = direct_storage.or_else(|| {
                name_val
                    .as_deref()
                    .and_then(|n| ctx.chara_registry.get(n))
                    .and_then(|def| def.resolve_face(face_val.as_deref()))
            });
            vec![KagEvent::Tag(ResolvedTag::CharaMod {
                name: name_val,
                storage: resolved_storage,
                face: face_val,
                pose: resolve_str_field(ctx, pose),
            })]
        }
        KnownTag::CharaMove { name, x, y, time } => {
            vec![KagEvent::Tag(ResolvedTag::CharaMove {
                name: resolve_str_field(ctx, name),
                x: resolve_typed_field(ctx, x),
                y: resolve_typed_field(ctx, y),
                time: resolve_typed_field(ctx, time),
            })]
        }
        KnownTag::CharaLayer { name, layer } => {
            vec![KagEvent::Tag(ResolvedTag::CharaLayer {
                name: resolve_str_field(ctx, name),
                layer: resolve_str_field(ctx, layer),
            })]
        }
        KnownTag::CharaLayerMod {
            name,
            opacity,
            visible,
        } => {
            vec![KagEvent::Tag(ResolvedTag::CharaLayerMod {
                name: resolve_str_field(ctx, name),
                opacity: resolve_typed_field(ctx, opacity),
                visible: resolve_typed_field(ctx, visible),
            })]
        }
        KnownTag::CharaPart {
            name,
            part,
            storage,
        } => {
            vec![KagEvent::Tag(ResolvedTag::CharaPart {
                name: resolve_str_field(ctx, name),
                part: resolve_str_field(ctx, part),
                storage: resolve_str_field(ctx, storage),
            })]
        }
        KnownTag::CharaPartReset { name } => {
            vec![KagEvent::Tag(ResolvedTag::CharaPartReset {
                name: resolve_str_field(ctx, name),
            })]
        }

        // ── Skip control tags ─────────────────────────────────────────────
        KnownTag::Skipstart {} => {
            ctx.skip_mode = true;
            vec![KagEvent::Tag(ResolvedTag::SkipMode { enabled: true })]
        }
        KnownTag::Skipstop {} | KnownTag::Cancelskip {} => {
            ctx.skip_mode = false;
            vec![KagEvent::Tag(ResolvedTag::SkipMode { enabled: false })]
        }
        KnownTag::StartKeyconfig {} => {
            vec![KagEvent::Tag(ResolvedTag::KeyConfig { open: true })]
        }
        KnownTag::StopKeyconfig {} => {
            vec![KagEvent::Tag(ResolvedTag::KeyConfig { open: false })]
        }

        // ── UI tags ───────────────────────────────────────────────────────
        KnownTag::Button {
            text,
            graphic,
            x,
            y,
            width,
            height,
            bg,
            hover_bg,
            press_bg,
            color,
            font_size,
            target,
            storage,
            exp,
            key,
            visible,
            opacity,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Button {
                text: resolve_str_field(ctx, text),
                graphic: resolve_str_field(ctx, graphic),
                x: resolve_typed_field(ctx, x),
                y: resolve_typed_field(ctx, y),
                width: resolve_typed_field(ctx, width),
                height: resolve_typed_field(ctx, height),
                bg: resolve_str_field(ctx, bg),
                hover_bg: resolve_str_field(ctx, hover_bg),
                press_bg: resolve_str_field(ctx, press_bg),
                color: resolve_str_field(ctx, color),
                font_size: resolve_typed_field(ctx, font_size),
                target: resolve_str_field(ctx, target),
                storage: resolve_str_field(ctx, storage),
                exp: resolve_str_field(ctx, exp),
                key: resolve_str_field(ctx, key),
                visible: resolve_typed_field(ctx, visible),
                opacity: resolve_typed_field(ctx, opacity),
            })]
        }
        KnownTag::Clickable {
            layer,
            target,
            storage,
            exp,
        } => {
            vec![KagEvent::Tag(ResolvedTag::Clickable {
                layer: resolve_str_field(ctx, layer),
                target: resolve_str_field(ctx, target),
                storage: resolve_str_field(ctx, storage),
                exp: resolve_str_field(ctx, exp),
            })]
        }
        KnownTag::Showmenu {} => vec![KagEvent::Tag(ResolvedTag::OpenPanel {
            kind: "menu".to_owned(),
        })],
        KnownTag::Showload {} => vec![KagEvent::Tag(ResolvedTag::OpenPanel {
            kind: "load".to_owned(),
        })],
        KnownTag::Showsave {} => vec![KagEvent::Tag(ResolvedTag::OpenPanel {
            kind: "save".to_owned(),
        })],
        KnownTag::Showlog {} => vec![KagEvent::Tag(ResolvedTag::OpenPanel {
            kind: "log".to_owned(),
        })],
        KnownTag::Hidemessage {} => vec![KagEvent::Tag(ResolvedTag::OpenPanel {
            kind: "hidemessage".to_owned(),
        })],
        KnownTag::Showmenubutton {} => vec![KagEvent::Tag(ResolvedTag::OpenPanel {
            kind: "showmenubutton".to_owned(),
        })],
        KnownTag::Hidemenubutton {} => vec![KagEvent::Tag(ResolvedTag::OpenPanel {
            kind: "hidemenubutton".to_owned(),
        })],
        KnownTag::Dialog { text, title } => vec![KagEvent::Tag(ResolvedTag::Dialog {
            text: resolve_str_field(ctx, text),
            title: resolve_str_field(ctx, title),
        })],
        KnownTag::Cursor { storage } => vec![KagEvent::Tag(ResolvedTag::Cursor {
            storage: resolve_str_field(ctx, storage),
        })],
        KnownTag::SpeakOn {} => vec![KagEvent::Tag(ResolvedTag::SetSpeakerBoxVisible {
            visible: true,
        })],
        KnownTag::SpeakOff {} => vec![KagEvent::Tag(ResolvedTag::SetSpeakerBoxVisible {
            visible: false,
        })],
        KnownTag::Glyph { storage } => vec![KagEvent::Tag(ResolvedTag::SetGlyph {
            kind: "default".to_owned(),
            storage: resolve_str_field(ctx, storage),
        })],
        KnownTag::GlyphAuto { storage } => vec![KagEvent::Tag(ResolvedTag::SetGlyph {
            kind: "auto".to_owned(),
            storage: resolve_str_field(ctx, storage),
        })],
        KnownTag::GlyphSkip { storage } => vec![KagEvent::Tag(ResolvedTag::SetGlyph {
            kind: "skip".to_owned(),
            storage: resolve_str_field(ctx, storage),
        })],
        KnownTag::GlinkConfig { .. } => vec![KagEvent::Tag(ResolvedTag::Extension {
            name: "glink_config".to_owned(),
            params: resolve_raw_params(ctx, &tag.params),
        })],
        KnownTag::ModeEffect { mode, effect } => {
            vec![KagEvent::Tag(ResolvedTag::ModeEffect {
                mode: resolve_str_field(ctx, mode),
                effect: resolve_str_field(ctx, effect),
            })]
        }

        // ── [web] — open a URL in the system browser ───────────────────────
        KnownTag::Web { url } => vec![KagEvent::Tag(ResolvedTag::Web {
            url: resolve_str_field(ctx, url),
        })],

        // ── Macro definition ──────────────────────────────────────────────
        KnownTag::Macro { .. } => vec![],

        // ── Extension / unknown tags ──────────────────────────────────────
        KnownTag::Extension { name, params } => vec![KagEvent::Tag(ResolvedTag::Extension {
            name: name.into_owned(),
            params: resolve_raw_params(ctx, &params),
        })],

        // ── Control-flow tags ────────────────────────────────────────────
        KnownTag::If { .. }
        | KnownTag::Elsif { .. }
        | KnownTag::Else {}
        | KnownTag::Endif {}
        | KnownTag::Ignore { .. }
        | KnownTag::Endignore {} => vec![],
    };

    events.append(&mut tag_events);
    events
}

// ─── Control-flow dispatch (always executed, even inside skipped blocks) ──────

fn is_control_flow_tag(name: &str) -> bool {
    TagName::from_name(name)
        .map(|tn| {
            matches!(
                tn,
                TagName::If
                    | TagName::Elsif
                    | TagName::Else
                    | TagName::Endif
                    | TagName::Ignore
                    | TagName::Endignore
            )
        })
        .unwrap_or(false)
}

fn execute_control_flow<'s>(
    _script: &Script<'s>,
    ctx: &mut RuntimeContext,
    tag: &Tag<'s>,
) -> Vec<KagEvent> {
    ctx.advance();

    let mut diags = Vec::new();
    match KnownTag::from_tag(tag, &mut diags) {
        KnownTag::If { exp } => {
            let exp_str = resolve_str_field(ctx, exp).unwrap_or_else(|| "false".to_owned());
            let cond = if ctx.is_executing() {
                ctx.script_engine.eval_bool(&exp_str).unwrap_or(false)
            } else {
                false
            };
            ctx.push_if(cond);
        }
        KnownTag::Elsif { exp } => {
            if ctx.is_executing()
                || ctx
                    .if_stack
                    .last()
                    .map(|f| !f.branch_taken)
                    .unwrap_or(false)
            {
                let exp_str = resolve_str_field(ctx, exp).unwrap_or_else(|| "false".to_owned());
                let cond = ctx.script_engine.eval_bool(&exp_str).unwrap_or(false);
                ctx.elsif(cond);
            } else {
                ctx.elsif(false);
            }
        }
        KnownTag::Else {} => {
            ctx.else_branch();
        }
        KnownTag::Endif {} => {
            ctx.pop_if();
        }
        KnownTag::Ignore { exp } => {
            let exp_str = resolve_str_field(ctx, exp).unwrap_or_else(|| "false".to_owned());
            let skip = if ctx.is_executing() {
                ctx.script_engine.eval_bool(&exp_str).unwrap_or(false)
            } else {
                true
            };
            ctx.push_if(!skip);
        }
        KnownTag::Endignore {} => {
            ctx.pop_if();
        }
        _ => {}
    }

    vec![]
}

// ─── Macro invocation ─────────────────────────────────────────────────────────

fn invoke_macro<'s>(script: &Script<'s>, ctx: &mut RuntimeContext, tag: &Tag<'s>) -> Vec<KagEvent> {
    let macro_name = tag.name.as_ref();
    let def = match script.macro_map.get(macro_name) {
        Some(d) => d.clone(),
        None => {
            return vec![KagEvent::Diagnostic(InterpreterDiagnostic::warning(
                DiagnosticCategory::Macro,
                format!("macro not found: {macro_name}"),
            ))];
        }
    };

    let return_pc = ctx.pc + 1;
    ctx.advance();

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
                    continue;
                }
            };
            mp.insert(key.as_ref().into(), rhai::Dynamic::from(raw_val));
        }
    }

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

    vec![]
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

// ─── Resolution helpers ───────────────────────────────────────────────────────

/// Evaluate a raw `ParamValue` to a `String`.
fn resolve_param_value(ctx: &mut RuntimeContext, pv: &ParamValue<'_>) -> String {
    match pv {
        ParamValue::Literal(s) => ctx.resolve_value(s.as_ref()),
        ParamValue::Entity(expr) => ctx.script_engine.resolve_entity(expr.as_ref()),
        ParamValue::MacroParam { key, default } => {
            let mp = ctx.script_engine.mp();
            mp.get(key.as_ref())
                .map(|v| v.to_string())
                .or_else(|| default.as_deref().map(str::to_owned))
                .unwrap_or_default()
        }
        ParamValue::MacroSplat => String::new(),
    }
}

/// Resolve an `Option<MaybeResolved<AttributeString>>` field to `Option<String>`.
fn resolve_str_field(
    ctx: &mut RuntimeContext,
    field: Option<MaybeResolved<'_, AttributeString<'_>>>,
) -> Option<String> {
    field.map(|mr| match mr {
        MaybeResolved::Literal(AttributeString(s)) => ctx.resolve_value(s.as_ref()),
        MaybeResolved::Dynamic(pv) => resolve_param_value(ctx, &pv),
    })
}

/// Resolve an `Option<MaybeResolved<T>>` typed field to `Option<T>`.
fn resolve_typed_field<T: Clone + std::str::FromStr>(
    ctx: &mut RuntimeContext,
    field: Option<MaybeResolved<'_, T>>,
) -> Option<T> {
    field.and_then(|mr| match mr {
        MaybeResolved::Literal(v) => Some(v),
        MaybeResolved::Dynamic(pv) => {
            let s = resolve_param_value(ctx, &pv);
            s.parse().ok()
        }
    })
}

/// Resolve all parameters of a raw `Tag` to a `Vec<(String, String)>`.
/// Used when building `ResolvedTag::Extension` from semi-known tags.
fn resolve_raw_params(ctx: &mut RuntimeContext, params: &[Param<'_>]) -> Vec<(String, String)> {
    params
        .iter()
        .filter_map(|p| {
            p.key.as_deref().map(|k| {
                let val = resolve_param_value(ctx, &p.value);
                (k.to_owned(), val)
            })
        })
        .collect()
}

/// Build a `KagEvent::Tag(ResolvedTag::Extension)` from the raw tag, used as
/// a fallback for known tags that are also forwarded to the host.
fn extension_event_from_tag(
    ctx: &mut RuntimeContext,
    tag: &Tag<'_>,
    _known: &KnownTag<'_>,
) -> KagEvent {
    KagEvent::Tag(ResolvedTag::Extension {
        name: tag.name.to_string(),
        params: resolve_raw_params(ctx, &tag.params),
    })
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
            let events = execute_op(&script, &mut ctx);
            all_events.extend(events);
        }

        (all_events, ctx)
    }

    fn resolved_tag_name(rt: &ResolvedTag) -> &'static str {
        match rt {
            ResolvedTag::Bg { .. } => "bg",
            ResolvedTag::Image { .. } => "image",
            ResolvedTag::Layopt { .. } => "layopt",
            ResolvedTag::Free { .. } => "free",
            ResolvedTag::Position { .. } => "position",
            ResolvedTag::Backlay => "backlay",
            ResolvedTag::Current { .. } => "current",
            ResolvedTag::Locate { .. } => "locate",
            ResolvedTag::Layermode { .. } => "layermode",
            ResolvedTag::FreeLayermode { .. } => "free_layermode",
            ResolvedTag::Filter { .. } => "filter",
            ResolvedTag::FreeFilter { .. } => "free_filter",
            ResolvedTag::PositionFilter { .. } => "position_filter",
            ResolvedTag::Mask { .. } => "mask",
            ResolvedTag::MaskOff { .. } => "mask_off",
            ResolvedTag::Graph { .. } => "graph",
            ResolvedTag::Bgm { .. } => "bgm",
            ResolvedTag::Stopbgm { .. } => "stopbgm",
            ResolvedTag::Pausebgm { .. } => "pausebgm",
            ResolvedTag::Resumebgm { .. } => "resumebgm",
            ResolvedTag::Fadebgm { .. } => "fadebgm",
            ResolvedTag::Xchgbgm { .. } => "xchgbgm",
            ResolvedTag::Bgmopt { .. } => "bgmopt",
            ResolvedTag::Se { .. } => "se",
            ResolvedTag::Stopse { .. } => "stopse",
            ResolvedTag::Pausese { .. } => "pausese",
            ResolvedTag::Resumese { .. } => "resumese",
            ResolvedTag::Seopt { .. } => "seopt",
            ResolvedTag::Vo { .. } => "vo",
            ResolvedTag::Changevol { .. } => "changevol",
            ResolvedTag::Anim { .. } => "anim",
            ResolvedTag::StopAnim { .. } => "stopanim",
            ResolvedTag::Kanim { .. } => "kanim",
            ResolvedTag::StopKanim { .. } => "stop_kanim",
            ResolvedTag::Xanim { .. } => "xanim",
            ResolvedTag::StopXanim { .. } => "stop_xanim",
            ResolvedTag::Bgmovie { .. } => "bgmovie",
            ResolvedTag::StopBgmovie => "stop_bgmovie",
            ResolvedTag::Movie { .. } => "movie",
            ResolvedTag::Trans { .. } => "trans",
            ResolvedTag::Fadein { .. } => "fadein",
            ResolvedTag::Fadeout { .. } => "fadeout",
            ResolvedTag::Movetrans { .. } => "movetrans",
            ResolvedTag::Quake { .. } => "quake",
            ResolvedTag::Shake { .. } => "shake",
            ResolvedTag::Flash { .. } => "flash",
            ResolvedTag::Msgwnd { .. } => "msgwnd",
            ResolvedTag::Wndctrl { .. } => "wndctrl",
            ResolvedTag::Resetfont => "resetfont",
            ResolvedTag::Font { .. } => "font",
            ResolvedTag::Size { .. } => "size",
            ResolvedTag::Bold { .. } => "bold",
            ResolvedTag::Italic { .. } => "italic",
            ResolvedTag::Ruby { .. } => "ruby",
            ResolvedTag::Nowrap { .. } => "nowrap",
            ResolvedTag::CharaShow { .. } => "chara_show",
            ResolvedTag::CharaHide { .. } => "chara_hide",
            ResolvedTag::CharaHideAll { .. } => "chara_hide_all",
            ResolvedTag::CharaFree { .. } => "chara_free",
            ResolvedTag::CharaDelete { .. } => "chara_delete",
            ResolvedTag::CharaMod { .. } => "chara_mod",
            ResolvedTag::CharaMove { .. } => "chara_move",
            ResolvedTag::CharaLayer { .. } => "chara_layer",
            ResolvedTag::CharaLayerMod { .. } => "chara_layer_mod",
            ResolvedTag::CharaPart { .. } => "chara_part",
            ResolvedTag::CharaPartReset { .. } => "chara_part_reset",
            ResolvedTag::SkipMode { .. } => "skip_mode",
            ResolvedTag::KeyConfig { .. } => "key_config",
            ResolvedTag::Button { .. } => "button",
            ResolvedTag::Clickable { .. } => "clickable",
            ResolvedTag::OpenPanel { .. } => "open_panel",
            ResolvedTag::Dialog { .. } => "dialog",
            ResolvedTag::Cursor { .. } => "cursor",
            ResolvedTag::SetSpeakerBoxVisible { .. } => "set_speaker_box_visible",
            ResolvedTag::SetGlyph { .. } => "set_glyph",
            ResolvedTag::ModeEffect { .. } => "mode_effect",
            ResolvedTag::Web { .. } => "web",
            ResolvedTag::Extension { name, .. } => {
                // Leak to get a &'static str for test output — only used in tests.
                Box::leak(name.clone().into_boxed_str())
            }
        }
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
                KagEvent::WaitForCompletion { which, .. } => {
                    format!("wait_completion:{}", which.as_str())
                }
                KagEvent::WaitForRawClick => "wait_raw_click".to_string(),
                KagEvent::InputRequested { name, .. } => format!("input:{}", name),
                KagEvent::WaitForTrigger { name } => format!("wait_trig:{}", name),
                KagEvent::Jump { target, .. } => {
                    format!("jump:{}", target.as_deref().unwrap_or(""))
                }
                KagEvent::BeginChoices(_) => "choices".to_string(),
                KagEvent::EmbedText(s) => format!("emb:{}", s),
                KagEvent::Tag(rt) => format!("tag:{}", resolved_tag_name(rt)),
                KagEvent::End => "end".to_string(),
                KagEvent::Diagnostic(diag) => format!("diag:{:?}", diag),
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
            let ev = execute_op(&script, &mut ctx);

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

        let _ = execute_op(&script, &mut ctx);

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

        let _ = execute_op(&script, &mut ctx);

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
            events.iter().any(|e| matches!(
                e,
                KagEvent::WaitForCompletion {
                    which: TagName::Wa,
                    ..
                }
            )),
            "wa must emit WaitForCompletion: {:?}",
            events
        );
    }

    #[test]
    fn test_wt_emits_wait_for_completion() {
        let (events, _) = run_script("@wt\n");
        assert!(
            events.iter().any(|e| matches!(
                e,
                KagEvent::WaitForCompletion {
                    which: TagName::Wt,
                    ..
                }
            )),
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
