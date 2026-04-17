//! UI tag handlers.
//!
//! Handles: `[button]`, `[clickable]`, `[showmenu]`/`[showload]`/`[showsave]`/`[showlog]`,
//! `[hidemessage]`, `[showmenubutton]`/`[hidemenubutton]`, `[dialog]`, `[cursor]`,
//! `[speak_on]`/`[speak_off]`, `[glyph]`/`[glyph_auto]`/`[glyph_skip]`, `[mode_effect]`.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{
    EvKeyConfig, EvModeEffect, EvOpenPanel, EvSetClickable, EvSetCursor, EvSetGlyph,
    EvSetSpeakerBoxVisible, EvShowDialog, EvSkipMode, EvSpawnButton, EvTagRouted,
};

#[allow(clippy::too_many_arguments)]
pub fn handle_ui_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_button: MessageWriter<EvSpawnButton>,
    mut ev_clickable: MessageWriter<EvSetClickable>,
    mut ev_panel: MessageWriter<EvOpenPanel>,
    mut ev_dialog: MessageWriter<EvShowDialog>,
    mut ev_cursor: MessageWriter<EvSetCursor>,
    mut ev_speaker: MessageWriter<EvSetSpeakerBoxVisible>,
    mut ev_glyph: MessageWriter<EvSetGlyph>,
    mut ev_mode_effect: MessageWriter<EvModeEffect>,
    mut ev_skip: MessageWriter<EvSkipMode>,
    mut ev_keyconfig: MessageWriter<EvKeyConfig>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Button {
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
                ev_button.write(EvSpawnButton {
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
                });
            }
            ResolvedTag::Clickable {
                layer,
                target,
                storage,
                exp,
            } => {
                ev_clickable.write(EvSetClickable {
                    layer,
                    target,
                    storage,
                    exp,
                });
            }
            ResolvedTag::OpenPanel { kind } => {
                ev_panel.write(EvOpenPanel { kind });
            }
            ResolvedTag::Dialog { text, title } => {
                ev_dialog.write(EvShowDialog { text, title });
            }
            ResolvedTag::Cursor { storage } => {
                ev_cursor.write(EvSetCursor { storage });
            }
            ResolvedTag::SetSpeakerBoxVisible { visible } => {
                ev_speaker.write(EvSetSpeakerBoxVisible { visible });
            }
            ResolvedTag::SetGlyph { kind, storage } => {
                ev_glyph.write(EvSetGlyph { kind, storage });
            }
            ResolvedTag::ModeEffect { mode, effect } => {
                ev_mode_effect.write(EvModeEffect { mode, effect });
            }
            ResolvedTag::SkipMode { enabled } => {
                ev_skip.write(EvSkipMode { enabled });
            }
            ResolvedTag::KeyConfig { open } => {
                ev_keyconfig.write(EvKeyConfig { open });
            }
            _ => {}
        }
    }
}
