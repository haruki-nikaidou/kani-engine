//! Screen-effect tag handlers (`[quake]`, `[shake]`, `[flash]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvFlash, EvQuake, EvShake, EvTagRouted};

pub fn handle_effect_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_quake: MessageWriter<EvQuake>,
    mut ev_shake: MessageWriter<EvShake>,
    mut ev_flash: MessageWriter<EvFlash>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Quake { time, hmax, vmax } => {
                ev_quake.write(EvQuake { time, hmax, vmax });
            }
            ResolvedTag::Shake { time, amount, axis } => {
                ev_shake.write(EvShake { time, amount, axis });
            }
            ResolvedTag::Flash { time, color } => {
                ev_flash.write(EvFlash { time, color });
            }
            _ => {}
        }
    }
}
