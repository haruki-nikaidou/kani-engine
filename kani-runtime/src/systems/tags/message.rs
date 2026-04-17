//! Message-window and text-style tag handlers (`[msgwnd]`, `[wndctrl]`,
//! `[resetfont]`, `[font]`, `[size]`, `[bold]`, `[italic]`, `[ruby]`,
//! `[nowrap]`, `[endnowrap]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{
    EvMessageWindow, EvResetFont, EvSetFont, EvSetNowrap, EvSetRuby, EvTagRouted, EvWindowControl,
};

pub fn handle_message_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_msgwnd: MessageWriter<EvMessageWindow>,
    mut ev_wndctrl: MessageWriter<EvWindowControl>,
    mut ev_resetfont: MessageWriter<EvResetFont>,
    mut ev_font: MessageWriter<EvSetFont>,
    mut ev_ruby: MessageWriter<EvSetRuby>,
    mut ev_nowrap: MessageWriter<EvSetNowrap>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Msgwnd { visible, layer } => {
                ev_msgwnd.write(EvMessageWindow { visible, layer });
            }
            ResolvedTag::Wndctrl {
                x,
                y,
                width,
                height,
            } => {
                ev_wndctrl.write(EvWindowControl {
                    x,
                    y,
                    width,
                    height,
                });
            }
            ResolvedTag::Resetfont => {
                ev_resetfont.write(EvResetFont);
            }
            ResolvedTag::Font {
                face,
                size,
                bold,
                italic,
            } => {
                ev_font.write(EvSetFont {
                    face,
                    size,
                    bold,
                    italic,
                });
            }
            ResolvedTag::Size { value } => {
                ev_font.write(EvSetFont {
                    face: None,
                    size: value,
                    bold: None,
                    italic: None,
                });
            }
            ResolvedTag::Bold { value } => {
                ev_font.write(EvSetFont {
                    face: None,
                    size: None,
                    bold: Some(value.unwrap_or(true)),
                    italic: None,
                });
            }
            ResolvedTag::Italic { value } => {
                ev_font.write(EvSetFont {
                    face: None,
                    size: None,
                    bold: None,
                    italic: Some(value.unwrap_or(true)),
                });
            }
            ResolvedTag::Ruby { text } => {
                ev_ruby.write(EvSetRuby { text });
            }
            ResolvedTag::Nowrap { enabled } => {
                ev_nowrap.write(EvSetNowrap { enabled });
            }
            _ => {}
        }
    }
}
