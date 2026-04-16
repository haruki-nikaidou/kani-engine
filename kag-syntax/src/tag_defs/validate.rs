use crate::{ParamValue, ParseDiagnostic, Tag, TagName};

/// Validate a single KAG tag against its known parameter requirements.
///
/// Returns a (possibly empty) list of diagnostics.  Unknown tags produce no
/// diagnostics — they are forwarded to the host as generic events.
pub fn validate_tag(tag: &Tag<'_>) -> Vec<ParseDiagnostic> {
    let mut diags = Vec::new();
    validate(tag, &mut diags);
    diags
}

fn validate(tag: &Tag<'_>, diags: &mut Vec<ParseDiagnostic>) {
    match TagName::from_name(tag.name.as_ref()) {
        // ── Interpreter: control flow ──────────────────────────────────────
        //
        // `exp=` is the condition expression; without it the branch condition
        // is undefined (the interpreter falls back to an empty Rhai string).
        Some(TagName::If | TagName::Elsif | TagName::Ignore) => {
            require(tag, "exp", diags);
        }

        // ── Interpreter: navigation ────────────────────────────────────────
        //
        // A `[jump]` / `[call]` with neither `storage=` nor `target=` is a
        // no-op jump that stays in the same file at the next instruction —
        // almost always a mistake.
        Some(TagName::Jump | TagName::Call) => {
            recommend_any_of(tag, &["storage", "target"], diags);
        }

        // ── Interpreter: choice links ──────────────────────────────────────
        //
        // `[link]` and `[glink]` need a destination to be useful; a choice
        // that goes nowhere will confuse the player.
        Some(TagName::Link | TagName::Glink) => {
            recommend_any_of(tag, &["storage", "target"], diags);
        }

        // ── Interpreter: scripting / expressions ───────────────────────────
        //
        // Without `exp=` these tags are silent no-ops (eval) or emit an empty
        // string inline (emb / trace).
        Some(TagName::Eval | TagName::Emb | TagName::Trace) => {
            recommend(tag, "exp", diags);
        }

        // ── Interpreter: timed waits ───────────────────────────────────────
        //
        // `[wait]` and `[wc]` default to 0 ms without `time=`, which makes
        // them a no-op and almost certainly unintentional.
        Some(TagName::Wait | TagName::Wc) => {
            recommend(tag, "time", diags);
        }

        // ── Interpreter: timeout handler ───────────────────────────────────
        //
        // A timeout without a duration fires immediately (0 ms), which is
        // almost certainly a mistake.
        Some(TagName::Timeout) => {
            recommend(tag, "time", diags);
        }

        // ── Interpreter: single-character display ──────────────────────────
        //
        // Without `text=` nothing is displayed; the tag is a no-op.
        Some(TagName::Ch | TagName::Hch) => {
            recommend(tag, "text", diags);
        }

        // ── Interpreter: macro management ─────────────────────────────────
        //
        // `[erasemacro]` without `name=` silently does nothing; the author
        // almost certainly forgot to name the macro to erase.
        Some(TagName::Erasemacro) => {
            recommend(tag, "name", diags);
        }

        // ── Interpreter: text-display speed ───────────────────────────────
        //
        // `[delay]` / `[configdelay]` without `speed=` defaults to 0 (instant),
        // which is valid but almost certainly unintentional.
        Some(TagName::Delay | TagName::Configdelay) => {
            recommend(tag, "speed", diags);
        }

        // ── Interpreter: backlog ───────────────────────────────────────────
        //
        // `[pushlog]` without `text=` pushes an empty entry — harmless but
        // almost certainly an oversight.
        Some(TagName::Pushlog) => {
            recommend(tag, "text", diags);
        }

        // ── Interpreter: input / triggers ─────────────────────────────────
        //
        // `[input]` stores the player's answer in a variable; without `name=`
        // the variable name is an empty string and the result is discarded.
        // `[waittrig]` without `name=` waits for a trigger whose name is "",
        // which is unlikely to ever fire.
        Some(TagName::Input) => {
            recommend(tag, "name", diags);
        }
        Some(TagName::Waittrig) => {
            recommend(tag, "name", diags);
        }

        // ── Interpreter: click / wheel handlers ────────────────────────────
        //
        // A handler with no destination and no expression does nothing when
        // the event fires.
        Some(TagName::Click | TagName::Wheel) => {
            recommend_any_of(tag, &["storage", "target", "exp"], diags);
        }

        // ── Interpreter: character nameplate ──────────────────────────────
        //
        // `[chara_ptext]` without `name=` leaves the current-speaker state
        // unchanged, which is most likely a bug.
        Some(TagName::CharaPtext) => {
            recommend(tag, "name", diags);
        }

        // ── Runtime bridge: image layer ────────────────────────────────────
        //
        // The Bevy handler silently skips these tags when `storage=` or
        // `layer=` is absent, so the visual change is simply lost.
        Some(TagName::Bg | TagName::Image) => {
            require(tag, "storage", diags);
        }
        Some(TagName::Layopt | TagName::Free | TagName::Position) => {
            require(tag, "layer", diags);
        }

        // ── Runtime bridge: audio ──────────────────────────────────────────
        //
        // Without `storage=` the Bevy handler ignores the tag entirely —
        // no sound plays.
        Some(TagName::Bgm | TagName::Se | TagName::PlaySe | TagName::Vo | TagName::Voice) => {
            require(tag, "storage", diags);
        }

        // ── Runtime bridge: character sprites ─────────────────────────────
        //
        // Character tags accept either `name=` or `id=` as the character
        // identifier.  Without either the handler has no way to look up the
        // character.
        Some(TagName::Chara) => {
            recommend_any_of(tag, &["name", "id"], diags);
        }
        Some(TagName::CharaHide | TagName::CharaFree | TagName::CharaMod) => {
            recommend_any_of(tag, &["name", "id"], diags);
        }

        // ── Everything else (known no-param tags, runtime extensions, macros) ─
        //
        // Known tags with no required or recommended params need no further
        // checking.  Unknown tags are forwarded to the host as generic events;
        // we have no schema for them and emit no diagnostics.
        Some(_) | None => {}
    }
}

/// Emit an **error** diagnostic when `key` is absent from `tag`.
///
/// The check uses [`Tag::param`] so that [`ParamValue::Entity`] and
/// [`ParamValue::MacroParam`] values count as "present" — only a completely
/// missing key triggers the diagnostic.
fn require(tag: &Tag<'_>, key: &str, diags: &mut Vec<ParseDiagnostic>) {
    if tag.param(key).is_none() {
        diags.push(ParseDiagnostic::error(
            format!("[{}] is missing required attribute `{key}=`", tag.name),
            tag.span,
        ));
    }
}

/// Emit a **warning** diagnostic when `key` is absent from `tag`.
fn recommend(tag: &Tag<'_>, key: &str, diags: &mut Vec<ParseDiagnostic>) {
    if tag.param(key).is_none() {
        diags.push(ParseDiagnostic::warning(
            format!(
                "[{}] is missing `{key}=`; tag will have no effect",
                tag.name
            ),
            tag.span,
        ));
    }
}

/// Emit a **warning** diagnostic when *none* of the given `keys` are present
/// on `tag`.
///
/// Used for tags where several alternative attributes serve the same role
/// (e.g. `storage=` vs `target=` for navigation, or `name=` vs `id=` for
/// character identity).
fn recommend_any_of(tag: &Tag<'_>, keys: &[&str], diags: &mut Vec<ParseDiagnostic>) {
    let any_present = keys.iter().any(|k| tag.param(k).is_some());
    if !any_present {
        let keys_fmt = keys
            .iter()
            .map(|k| format!("`{k}=`"))
            .collect::<Vec<_>>()
            .join(", ");
        diags.push(ParseDiagnostic::warning(
            format!(
                "[{}] should specify at least one of {keys_fmt}; tag will have no effect",
                tag.name
            ),
            tag.span,
        ));
    }
}
