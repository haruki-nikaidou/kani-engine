//! Video/movie tag handlers.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::EvVideoTag;

pub fn dispatch(resolved: ResolvedTag, ev: &mut MessageWriter<EvVideoTag>) {
    match resolved {
        ResolvedTag::Bgmovie {
            storage: Some(storage),
            looping,
            volume,
        } => {
            ev.write(EvVideoTag::PlayBgMovie {
                storage,
                looping,
                volume,
            });
        }
        ResolvedTag::StopBgmovie => {
            ev.write(EvVideoTag::StopBgMovie);
        }
        ResolvedTag::Movie {
            storage: Some(storage),
            x,
            y,
            width,
            height,
        } => {
            ev.write(EvVideoTag::PlayMovie {
                storage,
                x,
                y,
                width,
                height,
            });
        }
        _ => {}
    }
}
