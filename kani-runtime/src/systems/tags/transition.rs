//! Transition tag handlers.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::EvTransitionTag;

pub fn dispatch(resolved: ResolvedTag, ev: &mut MessageWriter<EvTransitionTag>) {
    match resolved {
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
