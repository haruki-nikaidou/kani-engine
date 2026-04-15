//! Screen-effect tag handlers (`[quake]`, `[shake]`, `[flash]`).

use bevy::prelude::*;

use super::{param, param_f32, param_u64};
use crate::events::{EvFlash, EvQuake, EvShake, EvTagRouted};

pub fn handle_effect_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_quake: MessageWriter<EvQuake>,
    mut ev_shake: MessageWriter<EvShake>,
    mut ev_flash: MessageWriter<EvFlash>,
) {
    for tag in reader.read() {
        let p = &tag.params;
        match tag.name.as_str() {
            "quake" => {
                ev_quake.write(EvQuake {
                    time: param_u64(p, "time"),
                    hmax: param_f32(p, "hmax"),
                    vmax: param_f32(p, "vmax"),
                });
            }
            "shake" => {
                ev_shake.write(EvShake {
                    time: param_u64(p, "time"),
                    amount: param_f32(p, "amount"),
                    axis: param(p, "axis"),
                });
            }
            "flash" => {
                ev_flash.write(EvFlash {
                    time: param_u64(p, "time"),
                    color: param(p, "color"),
                });
            }
            _ => {}
        }
    }
}
