//! Transition tag handlers (`[trans]`, `[fadein]`, `[fadeout]`, `[movetrans]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvTransitionTag, EvTagRouted};

pub fn handle_transition_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev: MessageWriter<EvTransitionTag>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Trans { method, time, rule } => {
                ev.write(EvTransitionTag::RunTransition { method, time, rule });
            }
            ResolvedTag::Fadein { time, color } => {
                ev.write(EvTransitionTag::FadeScreen {
                    kind: "fadein".to_owned(),
                    time,
                    color,
                });
            }
            ResolvedTag::Fadeout { time, color } => {
                ev.write(EvTransitionTag::FadeScreen {
                    kind: "fadeout".to_owned(),
                    time,
                    color,
                });
            }
            ResolvedTag::Movetrans { layer, time, x, y } => {
                ev.write(EvTransitionTag::MoveLayerTransition { layer, time, x, y });
            }
            _ => {}
        }
    }
}
