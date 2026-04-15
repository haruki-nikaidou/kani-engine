//! Transition tag handlers (`[trans]`, `[fadein]`, `[fadeout]`, `[movetrans]`).

use bevy::prelude::*;

use super::{param, param_f32, param_u64};
use crate::events::{EvFadeScreen, EvMoveLayerTransition, EvRunTransition, EvTagRouted};

pub fn handle_transition_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_trans: MessageWriter<EvRunTransition>,
    mut ev_fade: MessageWriter<EvFadeScreen>,
    mut ev_move: MessageWriter<EvMoveLayerTransition>,
) {
    for tag in reader.read() {
        let p = &tag.params;
        match tag.name.as_str() {
            "trans" => {
                ev_trans.write(EvRunTransition {
                    method: param(p, "method"),
                    time: param_u64(p, "time"),
                    rule: param(p, "rule"),
                });
            }
            "fadein" | "fadeout" => {
                ev_fade.write(EvFadeScreen {
                    kind: tag.name.clone(),
                    time: param_u64(p, "time"),
                    color: param(p, "color"),
                });
            }
            "movetrans" => {
                ev_move.write(EvMoveLayerTransition {
                    layer: param(p, "layer"),
                    time: param_u64(p, "time"),
                    x: param_f32(p, "x"),
                    y: param_f32(p, "y"),
                });
            }
            _ => {}
        }
    }
}
