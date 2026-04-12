//! Message-window and text-style tag handlers (`[msgwnd]`, `[wndctrl]`,
//! `[resetfont]`, `[font]`, `[size]`, `[bold]`, `[italic]`, `[ruby]`,
//! `[nowrap]`, `[endnowrap]`).

use bevy::prelude::*;

use crate::events::{
    EvMessageWindow, EvResetFont, EvSetFont, EvSetNowrap, EvSetRuby, EvTagRouted, EvWindowControl,
};
use super::{param, param_bool, param_f32};

pub fn handle_message_tags(
    mut reader: EventReader<EvTagRouted>,
    mut ev_msgwnd: EventWriter<EvMessageWindow>,
    mut ev_wndctrl: EventWriter<EvWindowControl>,
    mut ev_resetfont: EventWriter<EvResetFont>,
    mut ev_font: EventWriter<EvSetFont>,
    mut ev_ruby: EventWriter<EvSetRuby>,
    mut ev_nowrap: EventWriter<EvSetNowrap>,
) {
    for tag in reader.read() {
        let p = &tag.params;
        match tag.name.as_str() {
            "msgwnd" => {
                ev_msgwnd.write(EvMessageWindow {
                    visible: param_bool(p, "visible"),
                    layer: param(p, "layer"),
                });
            }
            "wndctrl" => {
                ev_wndctrl.write(EvWindowControl {
                    x: param_f32(p, "x"),
                    y: param_f32(p, "y"),
                    width: param_f32(p, "width"),
                    height: param_f32(p, "height"),
                });
            }
            "resetfont" => {
                ev_resetfont.write(EvResetFont);
            }
            "font" => {
                ev_font.write(EvSetFont {
                    face: param(p, "face"),
                    size: param_f32(p, "size"),
                    bold: param_bool(p, "bold"),
                    italic: param_bool(p, "italic"),
                });
            }
            "size" => {
                ev_font.write(EvSetFont {
                    face: None,
                    size: param_f32(p, "size").or_else(|| {
                        // `[size value=…]` variant used in some KAG dialects
                        param_f32(p, "value")
                    }),
                    bold: None,
                    italic: None,
                });
            }
            "bold" => {
                ev_font.write(EvSetFont {
                    face: None,
                    size: None,
                    bold: Some(param_bool(p, "enabled").unwrap_or(true)),
                    italic: None,
                });
            }
            "italic" => {
                ev_font.write(EvSetFont {
                    face: None,
                    size: None,
                    bold: None,
                    italic: Some(param_bool(p, "enabled").unwrap_or(true)),
                });
            }
            "ruby" => {
                ev_ruby.write(EvSetRuby { text: param(p, "text") });
            }
            "nowrap" => {
                ev_nowrap.write(EvSetNowrap { enabled: true });
            }
            "endnowrap" => {
                ev_nowrap.write(EvSetNowrap { enabled: false });
            }
            _ => {}
        }
    }
}
