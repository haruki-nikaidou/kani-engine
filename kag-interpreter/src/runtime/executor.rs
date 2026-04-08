//! Tag-execution logic for the KAG runtime.
//!
//! `execute_op` is a **synchronous** function that processes one op from the
//! script, mutates the `RuntimeContext` (PC, stacks, variables), and returns
//! any `KagEvent`s to be forwarded to the host.
//!
//! Async waiting (click waits, timers, choice input) is handled one level up
//! in the `KagInterpreter` actor.

use crate::ast::{Op, ParamValue, Script, Tag, TextPart};
use crate::error::KagError;
use crate::events::{ChoiceOption, KagEvent};

use super::context::RuntimeContext;

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
const TAG_MACRO: &str = "macro";
const TAG_ENDMACRO: &str = "endmacro";
const TAG_EVAL: &str = "eval";
const TAG_EMB: &str = "emb";
const TAG_LINK: &str = "link";
const TAG_ENDLINK: &str = "endlink";
const TAG_GLINK: &str = "glink";
const TAG_CHARA_PTEXT: &str = "chara_ptext";
const TAG_WARNING: &str = "_warning";

// ─── Public entry point ───────────────────────────────────────────────────────

/// Execute one op at `ctx.pc`.
///
/// On return `ctx.pc` has already been advanced (or redirected).
/// Returns `Ok(events)` on success; `Err` only on unrecoverable errors.
pub fn execute_op<'s>(
    script: &Script<'s>,
    ctx: &mut RuntimeContext,
) -> Result<Vec<KagEvent>, KagError> {
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
        && is_control_flow_tag(tag.name.as_ref()) {
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
        Op::ScriptBlock { content: script_text, .. } => {
            let script_text = script_text.clone();
            ctx.advance();
            ctx.script_engine
                .exec(&script_text)
                .map(|_| vec![])
                .or_else(|e| Ok(vec![KagEvent::Error(e.to_string())]))
        }
    }
}

// ─── Text op ─────────────────────────────────────────────────────────────────

fn execute_text<'s>(
    ctx: &mut RuntimeContext,
    parts: &[TextPart<'s>],
) -> Result<Vec<KagEvent>, KagError> {
    let mut events = Vec::new();
    let mut text_buf = String::new();

    let speaker = ctx.current_speaker.take();

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
                // Flush accumulated text before the inline tag
                if !text_buf.is_empty() {
                    events.push(KagEvent::DisplayText {
                        text: std::mem::take(&mut text_buf),
                        speaker: speaker.clone(),
                    });
                }
                // Execute the inline tag
                let mut inline_events = execute_inline_tag(ctx, tag)?;
                events.append(&mut inline_events);
            }
        }
    }

    // Flush any remaining text
    if !text_buf.is_empty() {
        events.push(KagEvent::DisplayText {
            text: text_buf,
            speaker,
        });
    }

    ctx.advance();
    Ok(events)
}

// ─── Inline tag dispatch (occurs within text lines) ───────────────────────────

fn execute_inline_tag(
    ctx: &mut RuntimeContext,
    tag: &Tag<'_>,
) -> Result<Vec<KagEvent>, KagError> {
    // Honour optional `cond=` guard on any inline tag
    let cond_expr = tag.param_str("cond").map(str::to_owned);
    if let Some(ref expr) = cond_expr
        && !ctx.script_engine.eval_bool(expr).unwrap_or(true) {
            return Ok(vec![]);
        }

    match tag.name.as_ref() {
        TAG_R => Ok(vec![KagEvent::InsertLineBreak]),
        TAG_L => Ok(vec![KagEvent::WaitForClick { clear_after: false }]),
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
        _ => Ok(vec![build_generic_event(ctx, tag)]),
    }
}

// ─── Tag op dispatch ─────────────────────────────────────────────────────────

fn execute_tag<'s>(
    script: &Script<'s>,
    ctx: &mut RuntimeContext,
    tag: &Tag<'s>,
) -> Result<Vec<KagEvent>, KagError> {
    // Check optional `cond=` guard — if false, skip the tag entirely
    let cond_expr = tag.param_str("cond").map(str::to_owned);
    if let Some(ref expr) = cond_expr
        && !ctx.script_engine.eval_bool(expr).unwrap_or(true) {
            ctx.advance();
            return Ok(vec![]);
        }

    let name = tag.name.as_ref();

    // ── Check if this is a macro invocation ────────────────────────────────
    if script.macro_map.contains_key(name) {
        return invoke_macro(script, ctx, tag);
    }

    ctx.advance();

    match name {
        // ── Text flow ──────────────────────────────────────────────────────
        TAG_L => Ok(vec![KagEvent::WaitForClick { clear_after: false }]),
        TAG_P => Ok(vec![KagEvent::WaitForClick { clear_after: true }]),
        TAG_R => Ok(vec![KagEvent::InsertLineBreak]),
        TAG_S => Ok(vec![KagEvent::Stop]),
        TAG_CM => Ok(vec![KagEvent::ClearMessage]),

        // ── Timed wait ─────────────────────────────────────────────────────
        TAG_WAIT => {
            let ms = resolve_u64(ctx, tag, "time").unwrap_or(0);
            Ok(vec![KagEvent::WaitMs(ms)])
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
                    Ok(vec![KagEvent::Return { storage: frame.return_storage }])
                } else {
                    Ok(vec![])
                }
            } else {
                Err(KagError::CallStackUnderflow)
            }
        }

        // ── Eval / emb ────────────────────────────────────────────────────
        TAG_EVAL => {
            let exp = tag.param_str("exp").unwrap_or("").to_owned();
            let next = tag.param_str("next").unwrap_or("true");
            let result = ctx.script_engine.exec(&exp);
            let mut events = Vec::new();
            if let Err(e) = result {
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
            let result = ctx
                .script_engine
                .eval_to_string(&exp)
                .unwrap_or_default();
            Ok(vec![KagEvent::EmbedText(result)])
        }

        // ── Choice links ──────────────────────────────────────────────────
        TAG_LINK => {
            ctx.in_link = true;
            let storage = resolved_str(ctx, tag, "storage");
            let target = resolved_str(ctx, tag, "target");
            let exp = tag.param_str("exp").map(str::to_owned);
            ctx.pending_choices.push(crate::runtime::context::PendingChoice {
                text: String::new(),
                storage,
                target,
                exp,
            });
            Ok(vec![build_generic_event(ctx, tag)])
        }

        TAG_ENDLINK => {
            ctx.in_link = false;
            if ctx.pending_choices.len() >= 2 {
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

        // ── Macro skeleton (body already registered at parse time) ────────
        TAG_MACRO => {
            // At runtime, skip past the macro body.  body_end is the index of
            // [endmacro]; jump to body_end + 1.
            let macro_name = tag.param_str("name").unwrap_or("").to_owned();
            if let Some(def) = script.macro_map.get(macro_name.as_str()) {
                ctx.jump_to(def.body_end + 1);
            }
            Ok(vec![])
        }

        TAG_ENDMACRO => {
            // If inside a macro invocation, return
            if let Some(frame) = ctx.pop_macro() {
                ctx.jump_to(frame.return_pc);
            }
            Ok(vec![])
        }

        // ── Internal warning ──────────────────────────────────────────────
        TAG_WARNING => {
            let msg = tag.param_str("msg").unwrap_or("warning").to_owned();
            Ok(vec![KagEvent::Warning(msg)])
        }

        // ── All other tags forwarded to host ──────────────────────────────
        _ => Ok(vec![build_generic_event(ctx, tag)]),
    }
}

// ─── Control-flow dispatch (always executed, even inside skipped blocks) ──────

fn is_control_flow_tag(name: &str) -> bool {
    matches!(name, TAG_IF | TAG_ELSIF | TAG_ELSE | TAG_ENDIF | TAG_IGNORE | TAG_ENDIGNORE)
}

fn execute_control_flow<'s>(
    _script: &Script<'s>,
    ctx: &mut RuntimeContext,
    tag: &Tag<'s>,
) -> Result<Vec<KagEvent>, KagError> {
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
) -> Result<Vec<KagEvent>, KagError> {
    let macro_name = tag.name.as_ref();
    let def = match script.macro_map.get(macro_name) {
        Some(d) => d.clone(),
        None => return Ok(vec![KagEvent::Warning(format!("macro not found: {macro_name}"))]),
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
    if tag.params.iter().any(|p| matches!(p.value, ParamValue::MacroSplat)) {
        let current_mp = ctx.script_engine.mp();
        for (k, v) in current_mp {
            mp.entry(k).or_insert(v);
        }
    }

    ctx.push_macro(macro_name, return_pc, mp);
    ctx.jump_to(def.body_start);

    Ok(vec![])
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
                KagEvent::WaitForClick { clear_after } => format!("wait_click:{}", clear_after),
                KagEvent::WaitMs(n) => format!("wait_ms:{}", n),
                KagEvent::Stop => "stop".to_string(),
                KagEvent::Jump { target, .. } => format!("jump:{}", target.as_deref().unwrap_or("")),
                KagEvent::BeginChoices(_) => "choices".to_string(),
                KagEvent::EmbedText(s) => format!("emb:{}", s),
                KagEvent::Tag { name, .. } => format!("tag:{}", name),
                KagEvent::End => "end".to_string(),
                KagEvent::Warning(w) => format!("warn:{}", w),
                KagEvent::Error(e) => format!("err:{}", e),
                KagEvent::VariableChanged { .. } => "var_changed".to_string(),
                KagEvent::Return { storage } => format!("return:{}", storage),
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
        assert!(names.contains(&"wait_click:false".to_string()), "{:?}", names);
    }

    #[test]
    fn test_wait_click_p() {
        let (events, _) = run_script("@p\n");
        let names = event_names(&events);
        assert!(names.contains(&"wait_click:true".to_string()), "{:?}", names);
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
        assert!(
            names.iter().any(|n| n.starts_with("jump:")),
            "{:?}",
            names
        );
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
}
