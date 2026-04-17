//! Screen-effect tag handlers (`[quake]`, `[shake]`, `[flash]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvEffectTag, EvTagRouted};

pub fn handle_effect_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev: MessageWriter<EvEffectTag>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
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
}
