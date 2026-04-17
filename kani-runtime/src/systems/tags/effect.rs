//! Screen-effect tag handlers.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::EvEffectTag;

pub fn dispatch(resolved: ResolvedTag, ev: &mut MessageWriter<EvEffectTag>) {
    match resolved {
        ResolvedTag::Quake { time, hmax, vmax } => {
            ev.write(EvEffectTag::Quake { time, hmax, vmax });
        }
        ResolvedTag::Shake { time, amount, axis } => {
            ev.write(EvEffectTag::Shake { time, amount, axis });
        }
        ResolvedTag::Flash { time, color } => {
            ev.write(EvEffectTag::Flash { time, color });
        }
        _ => {}
    }
}
