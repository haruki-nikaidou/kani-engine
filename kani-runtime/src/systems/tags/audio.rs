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
                storage,
                looping,
                volume,
                fadetime,
            } => {
                if let Some(storage) = storage {
                    ev_bgm.write(EvPlayBgm {
                        storage,
                        looping,
                        volume,
                        fadetime,
                    });
                }
            }
            ResolvedTag::Stopbgm { fadetime } => {
                ev_stop_bgm.write(EvStopBgm { fadetime });
            }
            ResolvedTag::Se {
                storage,
                buf,
                volume,
                looping,
            } => {
                if let Some(storage) = storage {
                    ev_se.write(EvPlaySe {
                        storage,
                        buf,
                        volume,
                        looping,
                    });
                }
            }
            ResolvedTag::Stopse { buf } => {
                ev_stop_se.write(EvStopSe { buf });
            }
            ResolvedTag::Vo { storage, buf } => {
                if let Some(storage) = storage {
                    ev_voice.write(EvPlayVoice { storage, buf });
                }
            }
            ResolvedTag::Fadebgm { time, volume } => {
                ev_fade.write(EvFadeBgm { time, volume });
            }
            _ => {}
        }
    }
}
