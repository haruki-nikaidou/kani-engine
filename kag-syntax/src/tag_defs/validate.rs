use crate::{SyntaxWarning, Tag, TagName};

/// Validate a single KAG tag against its known parameter requirements.
///
/// Returns a (possibly empty) list of diagnostics.  Unknown tags produce no
/// diagnostics — they are forwarded to the host as generic events.
pub fn validate_tag(tag: &Tag<'_>) -> Vec<SyntaxWarning> {
    let mut diags = Vec::new();
    validate(tag, &mut diags);
    diags
}

fn validate(tag: &Tag<'_>, diags: &mut Vec<SyntaxWarning>) {
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
fn require(tag: &Tag<'_>, key: &str, diags: &mut Vec<SyntaxWarning>) {
    if tag.param(key).is_none() {
        diags.push(SyntaxWarning::error(
            format!("[{}] is missing required attribute `{key}=`", tag.name),
            tag.span,
        ));
    }
}

/// Emit a **warning** diagnostic when `key` is absent from `tag`.
fn recommend(tag: &Tag<'_>, key: &str, diags: &mut Vec<SyntaxWarning>) {
    if tag.param(key).is_none() {
        diags.push(SyntaxWarning::warning(
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
fn recommend_any_of(tag: &Tag<'_>, keys: &[&str], diags: &mut Vec<SyntaxWarning>) {
    let any_present = keys.iter().any(|k| tag.param(k).is_some());
    if !any_present {
        let keys_fmt = keys
            .iter()
            .map(|k| format!("`{k}=`"))
            .collect::<Vec<_>>()
            .join(", ");
        diags.push(SyntaxWarning::warning(
            format!(
                "[{}] should specify at least one of {keys_fmt}; tag will have no effect",
                tag.name
            ),
            tag.span,
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::ast::{Param, ParamValue, Tag};
    use crate::error::Severity;

    fn span() -> miette::SourceSpan {
        (0usize, 0usize).into()
    }

    fn tag_no_params(name: &'static str) -> Tag<'static> {
        Tag {
            name: Cow::Borrowed(name),
            params: vec![],
            span: span(),
        }
    }

    fn tag_with_param(name: &'static str, key: &'static str, val: &'static str) -> Tag<'static> {
        Tag {
            name: Cow::Borrowed(name),
            params: vec![Param::literal(key, val, span())],
            span: span(),
        }
    }

    fn tag_with_entity(name: &'static str, key: &'static str) -> Tag<'static> {
        Tag {
            name: Cow::Borrowed(name),
            params: vec![Param::named(
                key,
                ParamValue::Entity(Cow::Borrowed("f.path")),
                span(),
            )],
            span: span(),
        }
    }

    fn tag_with_macro_param(name: &'static str, key: &'static str) -> Tag<'static> {
        Tag {
            name: Cow::Borrowed(name),
            params: vec![Param::named(
                key,
                ParamValue::MacroParam {
                    key: Cow::Borrowed(key),
                    default: None,
                },
                span(),
            )],
            span: span(),
        }
    }

    // ── Required (error) ──────────────────────────────────────────────────────

    #[test]
    fn if_without_exp_is_error() {
        let diags = validate_tag(&tag_no_params("if"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("exp="));
    }

    #[test]
    fn if_with_exp_is_clean() {
        let diags = validate_tag(&tag_with_param("if", "exp", "f.flag == 1"));
        assert!(diags.is_empty());
    }

    #[test]
    fn elsif_without_exp_is_error() {
        let diags = validate_tag(&tag_no_params("elsif"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn ignore_without_exp_is_error() {
        let diags = validate_tag(&tag_no_params("ignore"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn bg_without_storage_is_error() {
        let diags = validate_tag(&tag_no_params("bg"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("storage="));
    }

    #[test]
    fn bg_with_storage_is_clean() {
        let diags = validate_tag(&tag_with_param("bg", "storage", "bg001.jpg"));
        assert!(diags.is_empty());
    }

    #[test]
    fn image_without_storage_is_error() {
        let diags = validate_tag(&tag_no_params("image"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn layopt_without_layer_is_error() {
        let diags = validate_tag(&tag_no_params("layopt"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn free_without_layer_is_error() {
        let diags = validate_tag(&tag_no_params("free"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn position_without_layer_is_error() {
        let diags = validate_tag(&tag_no_params("position"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn bgm_without_storage_is_error() {
        let diags = validate_tag(&tag_no_params("bgm"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn se_without_storage_is_error() {
        let diags = validate_tag(&tag_no_params("se"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn play_se_without_storage_is_error() {
        let diags = validate_tag(&tag_no_params("playSe"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn vo_without_storage_is_error() {
        let diags = validate_tag(&tag_no_params("vo"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn voice_without_storage_is_error() {
        let diags = validate_tag(&tag_no_params("voice"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
    }

    // ── Recommended (warning) ─────────────────────────────────────────────────

    #[test]
    fn eval_without_exp_is_warning() {
        let diags = validate_tag(&tag_no_params("eval"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn eval_with_exp_is_clean() {
        let diags = validate_tag(&tag_with_param("eval", "exp", "f.x = 1"));
        assert!(diags.is_empty());
    }

    #[test]
    fn emb_without_exp_is_warning() {
        let diags = validate_tag(&tag_no_params("emb"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn trace_without_exp_is_warning() {
        let diags = validate_tag(&tag_no_params("trace"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn wait_without_time_is_warning() {
        let diags = validate_tag(&tag_no_params("wait"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn wait_with_time_is_clean() {
        let diags = validate_tag(&tag_with_param("wait", "time", "500"));
        assert!(diags.is_empty());
    }

    #[test]
    fn wc_without_time_is_warning() {
        let diags = validate_tag(&tag_no_params("wc"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn timeout_without_time_is_warning() {
        let diags = validate_tag(&tag_no_params("timeout"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn ch_without_text_is_warning() {
        let diags = validate_tag(&tag_no_params("ch"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn hch_without_text_is_warning() {
        let diags = validate_tag(&tag_no_params("hch"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn erasemacro_without_name_is_warning() {
        let diags = validate_tag(&tag_no_params("erasemacro"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn delay_without_speed_is_warning() {
        let diags = validate_tag(&tag_no_params("delay"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn configdelay_without_speed_is_warning() {
        let diags = validate_tag(&tag_no_params("configdelay"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn pushlog_without_text_is_warning() {
        let diags = validate_tag(&tag_no_params("pushlog"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn input_without_name_is_warning() {
        let diags = validate_tag(&tag_no_params("input"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn waittrig_without_name_is_warning() {
        let diags = validate_tag(&tag_no_params("waittrig"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_ptext_without_name_is_warning() {
        let diags = validate_tag(&tag_no_params("chara_ptext"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    // ── Any-of (warning) ──────────────────────────────────────────────────────

    #[test]
    fn jump_without_storage_or_target_is_warning() {
        let diags = validate_tag(&tag_no_params("jump"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
        assert!(diags[0].message.contains("storage="));
        assert!(diags[0].message.contains("target="));
    }

    #[test]
    fn jump_with_only_target_is_clean() {
        let diags = validate_tag(&tag_with_param("jump", "target", "*start"));
        assert!(diags.is_empty());
    }

    #[test]
    fn jump_with_only_storage_is_clean() {
        let diags = validate_tag(&tag_with_param("jump", "storage", "scene01.ks"));
        assert!(diags.is_empty());
    }

    #[test]
    fn call_without_destination_is_warning() {
        let diags = validate_tag(&tag_no_params("call"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn link_without_destination_is_warning() {
        let diags = validate_tag(&tag_no_params("link"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn link_with_target_is_clean() {
        let diags = validate_tag(&tag_with_param("link", "target", "*choice_a"));
        assert!(diags.is_empty());
    }

    #[test]
    fn glink_without_destination_is_warning() {
        let diags = validate_tag(&tag_no_params("glink"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn click_without_any_handler_is_warning() {
        let diags = validate_tag(&tag_no_params("click"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn click_with_exp_is_clean() {
        let diags = validate_tag(&tag_with_param("click", "exp", "f.handler()"));
        assert!(diags.is_empty());
    }

    #[test]
    fn wheel_without_any_handler_is_warning() {
        let diags = validate_tag(&tag_no_params("wheel"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_without_id_or_name_is_warning() {
        let diags = validate_tag(&tag_no_params("chara"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_with_name_is_clean() {
        let diags = validate_tag(&tag_with_param("chara", "name", "alice"));
        assert!(diags.is_empty());
    }

    #[test]
    fn chara_with_id_is_clean() {
        let diags = validate_tag(&tag_with_param("chara", "id", "alice"));
        assert!(diags.is_empty());
    }

    #[test]
    fn chara_hide_without_id_or_name_is_warning() {
        let diags = validate_tag(&tag_no_params("chara_hide"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_free_without_id_or_name_is_warning() {
        let diags = validate_tag(&tag_no_params("chara_free"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    #[test]
    fn chara_mod_without_id_or_name_is_warning() {
        let diags = validate_tag(&tag_no_params("chara_mod"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Warning);
    }

    // ── Entity / macro-param values count as present ──────────────────────────

    #[test]
    fn bg_with_entity_storage_is_clean() {
        let diags = validate_tag(&tag_with_entity("bg", "storage"));
        assert!(diags.is_empty());
    }

    #[test]
    fn if_with_macro_param_exp_is_clean() {
        let diags = validate_tag(&tag_with_macro_param("if", "exp"));
        assert!(diags.is_empty());
    }

    #[test]
    fn bgm_with_entity_storage_is_clean() {
        let diags = validate_tag(&tag_with_entity("bgm", "storage"));
        assert!(diags.is_empty());
    }

    // ── Unknown tags produce no diagnostics ───────────────────────────────────

    #[test]
    fn unknown_tag_is_clean() {
        let diags = validate_tag(&tag_no_params("my_custom_game_tag"));
        assert!(diags.is_empty());
    }

    #[test]
    fn no_params_tags_are_clean() {
        // Tags that have no required params and are valid with no attributes.
        for name in &[
            "l",
            "p",
            "r",
            "s",
            "cm",
            "return",
            "else",
            "endif",
            "endignore",
            "endlink",
            "endmacro",
            "nowait",
            "endnowait",
            "resetdelay",
            "nolog",
            "endnolog",
            "resetwait",
            "waitclick",
            "cclick",
            "ctimeout",
            "cwheel",
            "wa",
            "wm",
            "wt",
            "wq",
            "wb",
            "wf",
            "wl",
            "ws",
            "wv",
            "wp",
            "ct",
            "er",
            "clearvar",
            "clearsysvar",
            "clearstack",
            "stopbgm",
            "stopse",
            "trans",
            "fadein",
            "fadeout",
            "movetrans",
            "quake",
            "shake",
            "flash",
            "msgwnd",
            "wndctrl",
            "resetfont",
            "font",
            "size",
            "bold",
            "italic",
            "ruby",
            "nowrap",
            "endnowrap",
            "autowc",
            "clickskip",
        ] {
            let diags = validate_tag(&tag_no_params(name));
            assert!(
                diags.is_empty(),
                "[{name}] should produce no diagnostics when used without params"
            );
        }
    }
}

