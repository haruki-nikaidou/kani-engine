//! Video/movie tag handlers (`[bgmovie]`, `[stop_bgmovie]`, `[movie]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvTagRouted, EvVideoTag};

pub fn handle_video_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev: MessageWriter<EvVideoTag>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Bgmovie {
                storage: Some(storage),
                looping,
                volume,
            } => {
                ev.write(EvVideoTag::PlayBgMovie { storage, looping, volume });
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
                ev.write(EvVideoTag::PlayMovie { storage, x, y, width, height });
            }
            _ => {}
        }
    }
}
