//! Animation tag handlers (`[anim]`, `[stopanim]`, `[kanim]`, `[stop_kanim]`,
//! `[xanim]`, `[stop_xanim]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvAnimTag, EvTagRouted};

pub fn handle_animation_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev: MessageWriter<EvAnimTag>,
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
                ev.write(EvAnimTag::PlayAnim { layer, preset, time, looping, delay });
            }
            ResolvedTag::StopAnim { layer } => {
                ev.write(EvAnimTag::StopAnim { layer });
            }
            ResolvedTag::Kanim { layer, frames, looping } => {
                ev.write(EvAnimTag::PlayKanim { layer, frames, looping });
            }
            ResolvedTag::StopKanim { layer } => {
                ev.write(EvAnimTag::StopKanim { layer });
            }
            ResolvedTag::Xanim { layer, frames, looping } => {
                ev.write(EvAnimTag::PlayXanim { layer, frames, looping });
            }
            ResolvedTag::StopXanim { layer } => {
                ev.write(EvAnimTag::StopXanim { layer });
            }
            _ => {}
        }
    }
}
