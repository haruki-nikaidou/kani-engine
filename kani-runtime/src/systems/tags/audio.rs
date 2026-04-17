//! Audio tag handlers (`[bgm]`, `[stopbgm]`, `[se]`/`[playSe]`, `[stopse]`,
//! `[vo]`/`[voice]`, `[fadebgm]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{
    EvFadeBgm, EvPlayBgm, EvPlaySe, EvPlayVoice, EvStopBgm, EvStopSe, EvTagRouted,
};

pub fn handle_audio_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_bgm: MessageWriter<EvPlayBgm>,
    mut ev_stop_bgm: MessageWriter<EvStopBgm>,
    mut ev_se: MessageWriter<EvPlaySe>,
    mut ev_stop_se: MessageWriter<EvStopSe>,
    mut ev_voice: MessageWriter<EvPlayVoice>,
    mut ev_fade: MessageWriter<EvFadeBgm>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Bgm {
                storage: Some(storage),
                looping,
                volume,
                fadetime,
            } => {
                ev_bgm.write(EvPlayBgm {
                    storage,
                    looping,
                    volume,
                    fadetime,
                });
            }
            ResolvedTag::Stopbgm { fadetime } => {
                ev_stop_bgm.write(EvStopBgm { fadetime });
            }
            ResolvedTag::Se {
                storage: Some(storage),
                buf,
                volume,
                looping,
            } => {
                ev_se.write(EvPlaySe {
                    storage,
                    buf,
                    volume,
                    looping,
                });
            }
            ResolvedTag::Stopse { buf } => {
                ev_stop_se.write(EvStopSe { buf });
            }
            ResolvedTag::Vo {
                storage: Some(storage),
                buf,
            } => {
                ev_voice.write(EvPlayVoice { storage, buf });
            }
            ResolvedTag::Fadebgm { time, volume } => {
                ev_fade.write(EvFadeBgm { time, volume });
            }
            _ => {}
        }
    }
}
