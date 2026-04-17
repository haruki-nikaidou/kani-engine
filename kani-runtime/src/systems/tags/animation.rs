//! Animation tag handlers (`[anim]`, `[stopanim]`, `[kanim]`, `[stop_kanim]`,
//! `[xanim]`, `[stop_xanim]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{
    EvPlayAnim, EvPlayKanim, EvPlayXanim, EvStopAnim, EvStopKanim, EvStopXanim, EvTagRouted,
};

#[allow(clippy::too_many_arguments)]
pub fn handle_animation_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_anim: MessageWriter<EvPlayAnim>,
    mut ev_stop_anim: MessageWriter<EvStopAnim>,
    mut ev_kanim: MessageWriter<EvPlayKanim>,
    mut ev_stop_kanim: MessageWriter<EvStopKanim>,
    mut ev_xanim: MessageWriter<EvPlayXanim>,
    mut ev_stop_xanim: MessageWriter<EvStopXanim>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Anim {
                layer,
                preset,
                time,
                looping,
                delay,
            } => {
                ev_anim.write(EvPlayAnim {
                    layer,
                    preset,
                    time,
                    looping,
                    delay,
                });
            }
            ResolvedTag::StopAnim { layer } => {
                ev_stop_anim.write(EvStopAnim { layer });
            }
            ResolvedTag::Kanim {
                layer,
                frames,
                looping,
            } => {
                ev_kanim.write(EvPlayKanim {
                    layer,
                    frames,
                    looping,
                });
            }
            ResolvedTag::StopKanim { layer } => {
                ev_stop_kanim.write(EvStopKanim { layer });
            }
            ResolvedTag::Xanim {
                layer,
                frames,
                looping,
            } => {
                ev_xanim.write(EvPlayXanim {
                    layer,
                    frames,
                    looping,
                });
            }
            ResolvedTag::StopXanim { layer } => {
                ev_stop_xanim.write(EvStopXanim { layer });
            }
            _ => {}
        }
    }
}
