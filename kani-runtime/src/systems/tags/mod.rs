//! Tag-handler sub-systems.
//!
//! The `poll_interpreter` system emits every passthrough `KagEvent::Tag` as an
//! [`EvUnknownTag`].  Each handler module reads `EvUnknownTag`, matches on the
//! tag name, and emits a strongly-typed action event.  Tags not matched by any
//! handler remain as [`EvUnknownTag`] for game-specific code to consume.

pub mod audio;
pub mod chara;
pub mod effect;
pub mod image;
pub mod message;
pub mod transition;

use bevy::prelude::*;

use crate::events::{EvTagRouted, EvUnknownTag};

// ─── Shared param helpers ─────────────────────────────────────────────────────

/// Look up a parameter by name.
pub(crate) fn param(params: &[(String, String)], key: &str) -> Option<String> {
    params
        .iter()
        .find_map(|(k, v)| (k == key).then(|| v.clone()))
}

/// Parse a parameter as `f32`, returning `None` if absent or unparseable.
pub(crate) fn param_f32(params: &[(String, String)], key: &str) -> Option<f32> {
    param(params, key)?.parse().ok()
}

/// Parse a parameter as `u64`, returning `None` if absent or unparseable.
pub(crate) fn param_u64(params: &[(String, String)], key: &str) -> Option<u64> {
    param(params, key)?.parse().ok()
}

/// Parse a parameter as `u32`, returning `None` if absent or unparseable.
pub(crate) fn param_u32(params: &[(String, String)], key: &str) -> Option<u32> {
    param(params, key)?.parse().ok()
}

/// Parse a parameter as a boolean (`"false"` / `"0"` → false, anything else →
/// true).  Returns `None` if the parameter is absent.
pub(crate) fn param_bool(params: &[(String, String)], key: &str) -> Option<bool> {
    param(params, key).map(|v| !matches!(v.to_ascii_lowercase().as_str(), "false" | "0" | "off"))
}

// ─── Known-tag registry ───────────────────────────────────────────────────────

/// Returns `true` when `name` is handled by one of the built-in tag systems.
pub fn is_known_tag(name: &str) -> bool {
    matches!(
        name,
        // image
        "bg" | "image" | "layopt" | "free" | "position"
        // audio
        | "bgm" | "stopbgm" | "se" | "playSe" | "stopse" | "vo" | "voice" | "fadebgm"
        // transition
        | "trans" | "fadein" | "fadeout" | "movetrans"
        // effect
        | "quake" | "shake" | "flash"
        // message
        | "msgwnd" | "wndctrl" | "resetfont" | "font" | "size" | "bold" | "italic"
        | "ruby" | "nowrap" | "endnowrap"
        // chara
        | "chara" | "chara_hide" | "chara_free" | "chara_mod"
    )
}

/// Emit [`EvUnknownTag`] for every tag that no built-in handler covers.
pub fn emit_unknown_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut writer: MessageWriter<EvUnknownTag>,
) {
    for tag in reader.read() {
        if !is_known_tag(&tag.name) {
            writer.write(EvUnknownTag {
                name: tag.name.clone(),
                params: tag.params.clone(),
            });
        }
    }
}
