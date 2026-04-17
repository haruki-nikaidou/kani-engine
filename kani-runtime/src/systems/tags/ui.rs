//! UI tag handlers.
//!
//! Handles: `[button]`, `[clickable]`, `[showmenu]`/`[showload]`/`[showsave]`/`[showlog]`,
//! `[hidemessage]`, `[showmenubutton]`/`[hidemenubutton]`, `[dialog]`, `[cursor]`,
//! `[speak_on]`/`[speak_off]`, `[glyph]`/`[glyph_auto]`/`[glyph_skip]`, `[mode_effect]`,
//! `[skipstart]`/`[skipstop]`, `[start_keyconfig]`/`[stop_keyconfig]`.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvControlTag, EvUiTag};

pub fn dispatch_ui(resolved: ResolvedTag, ev_ui: &mut MessageWriter<EvUiTag>) {
    match resolved {
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
            ev_ui.write(EvUiTag::SpawnButton {
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
            ev_ui.write(EvUiTag::SetClickable {
                layer,
                target,
                storage,
                exp,
            });
        }
        ResolvedTag::OpenPanel { kind } => {
            ev_ui.write(EvUiTag::OpenPanel { kind });
        }
        ResolvedTag::Dialog { text, title } => {
            ev_ui.write(EvUiTag::ShowDialog { text, title });
        }
        ResolvedTag::Cursor { storage } => {
            ev_ui.write(EvUiTag::SetCursor { storage });
        }
        ResolvedTag::SetSpeakerBoxVisible { visible } => {
            ev_ui.write(EvUiTag::SetSpeakerBoxVisible { visible });
        }
        ResolvedTag::SetGlyph { kind, storage } => {
            ev_ui.write(EvUiTag::SetGlyph { kind, storage });
        }
        ResolvedTag::ModeEffect { mode, effect } => {
            ev_ui.write(EvUiTag::ModeEffect { mode, effect });
        }
        _ => {}
    }
}

pub fn dispatch_control(resolved: ResolvedTag, ev_ctrl: &mut MessageWriter<EvControlTag>) {
    match resolved {
        ResolvedTag::SkipMode { enabled } => {
            ev_ctrl.write(EvControlTag::SkipMode { enabled });
        }
        ResolvedTag::KeyConfig { open } => {
            ev_ctrl.write(EvControlTag::KeyConfig { open });
        }
        _ => {}
    }
}
