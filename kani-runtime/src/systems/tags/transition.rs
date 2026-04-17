//! Transition tag handlers (`[trans]`, `[fadein]`, `[fadeout]`, `[movetrans]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvFadeScreen, EvMoveLayerTransition, EvRunTransition, EvTagRouted};

pub fn handle_transition_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_trans: MessageWriter<EvRunTransition>,
    mut ev_fade: MessageWriter<EvFadeScreen>,
    mut ev_move: MessageWriter<EvMoveLayerTransition>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Trans { method, time, rule } => {
                ev_trans.write(EvRunTransition { method, time, rule });
            }
            ResolvedTag::Fadein { time, color } => {
                ev_fade.write(EvFadeScreen {
                    kind: "fadein".to_owned(),
                    time,
                    color,
                });
            }
            ResolvedTag::Fadeout { time, color } => {
                ev_fade.write(EvFadeScreen {
                    kind: "fadeout".to_owned(),
                    time,
                    color,
                });
            }
            ResolvedTag::Movetrans { layer, time, x, y } => {
                ev_move.write(EvMoveLayerTransition { layer, time, x, y });
            }
            _ => {}
        }
    }
}
