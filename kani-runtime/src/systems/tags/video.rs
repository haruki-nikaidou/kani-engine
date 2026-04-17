//! Video/movie tag handlers (`[bgmovie]`, `[stop_bgmovie]`, `[movie]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvPlayBgMovie, EvPlayMovie, EvStopBgMovie, EvTagRouted};

pub fn handle_video_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_bgmovie: MessageWriter<EvPlayBgMovie>,
    mut ev_stop_bgmovie: MessageWriter<EvStopBgMovie>,
    mut ev_movie: MessageWriter<EvPlayMovie>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Bgmovie {
                storage: Some(storage),
                looping,
                volume,
            } => {
                ev_bgmovie.write(EvPlayBgMovie {
                    storage,
                    looping,
                    volume,
                });
            }
            ResolvedTag::StopBgmovie => {
                ev_stop_bgmovie.write(EvStopBgMovie);
            }
            ResolvedTag::Movie {
                storage: Some(storage),
                x,
                y,
                width,
                height,
            } => {
                ev_movie.write(EvPlayMovie {
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
}
