//! Message-window and text-style tag handlers (`[msgwnd]`, `[wndctrl]`,
//! `[resetfont]`, `[font]`, `[size]`, `[bold]`, `[italic]`, `[ruby]`,
//! `[nowrap]`, `[endnowrap]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvMessageWindowTag, EvTagRouted};

pub fn handle_message_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev: MessageWriter<EvMessageWindowTag>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Msgwnd { visible, layer } => {
                ev.write(EvMessageWindowTag::MessageWindow { visible, layer });
            }
            ResolvedTag::Wndctrl { x, y, width, height } => {
                ev.write(EvMessageWindowTag::WindowControl { x, y, width, height });
            }
            ResolvedTag::Resetfont => {
                ev.write(EvMessageWindowTag::ResetFont);
            }
            ResolvedTag::Font { face, size, bold, italic } => {
                ev.write(EvMessageWindowTag::SetFont { face, size, bold, italic });
            }
            ResolvedTag::Size { value } => {
                ev.write(EvMessageWindowTag::SetFont {
                    face: None,
                    size: value,
                    bold: None,
                    italic: None,
                });
            }
            ResolvedTag::Bold { value } => {
                ev.write(EvMessageWindowTag::SetFont {
                    face: None,
                    size: None,
                    bold: Some(value.unwrap_or(true)),
                    italic: None,
                });
            }
            ResolvedTag::Italic { value } => {
                ev.write(EvMessageWindowTag::SetFont {
                    face: None,
                    size: None,
                    bold: None,
                    italic: Some(value.unwrap_or(true)),
                });
            }
            ResolvedTag::Ruby { text } => {
                ev.write(EvMessageWindowTag::SetRuby { text });
            }
            ResolvedTag::Nowrap { enabled } => {
                ev.write(EvMessageWindowTag::SetNowrap { enabled });
            }
            _ => {}
        }
    }
}
