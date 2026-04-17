//! Animation tag handlers.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::EvAnimTag;

pub fn dispatch(resolved: ResolvedTag, ev: &mut MessageWriter<EvAnimTag>) {
    match resolved {
        ResolvedTag::Anim {
            layer,
            preset,
            time,
            looping,
            delay,
        } => {
            ev.write(EvAnimTag::PlayAnim {
                layer,
                preset,
                time,
                looping,
                delay,
            });
        }
        ResolvedTag::StopAnim { layer } => {
            ev.write(EvAnimTag::StopAnim { layer });
        }
        ResolvedTag::Kanim {
            layer,
            frames,
            looping,
        } => {
            ev.write(EvAnimTag::PlayKanim {
                layer,
                frames,
                looping,
            });
        }
        ResolvedTag::StopKanim { layer } => {
            ev.write(EvAnimTag::StopKanim { layer });
        }
        ResolvedTag::Xanim {
            layer,
            frames,
            looping,
        } => {
            ev.write(EvAnimTag::PlayXanim {
                layer,
                frames,
                looping,
            });
        }
        ResolvedTag::StopXanim { layer } => {
            ev.write(EvAnimTag::StopXanim { layer });
        }
        _ => {}
    }
}
