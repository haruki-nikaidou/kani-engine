//! Audio tag handlers (`[bgm]`, `[stopbgm]`, `[se]`/`[playSe]`, `[stopse]`,
//! `[vo]`/`[voice]`, `[fadebgm]`).

use bevy::prelude::*;

use crate::events::{
    EvFadeBgm, EvPlayBgm, EvPlaySe, EvPlayVoice, EvStopBgm, EvStopSe, EvTagRouted,
};
use super::{param, param_bool, param_f32, param_u32, param_u64};

pub fn handle_audio_tags(
    mut reader: EventReader<EvTagRouted>,
    mut ev_bgm: EventWriter<EvPlayBgm>,
    mut ev_stop_bgm: EventWriter<EvStopBgm>,
    mut ev_se: EventWriter<EvPlaySe>,
    mut ev_stop_se: EventWriter<EvStopSe>,
    mut ev_voice: EventWriter<EvPlayVoice>,
    mut ev_fade: EventWriter<EvFadeBgm>,
) {
    for tag in reader.read() {
        let p = &tag.params;
        match tag.name.as_str() {
            "bgm" => {
                if let Some(storage) = param(p, "storage") {
                    ev_bgm.write(EvPlayBgm {
                        storage,
                        looping: param_bool(p, "loop").unwrap_or(true),
                        volume: param_f32(p, "volume"),
                        fadetime: param_u64(p, "fadetime"),
                    });
                }
            }
            "stopbgm" => {
                ev_stop_bgm.write(EvStopBgm { fadetime: param_u64(p, "fadetime") });
            }
            "se" | "playSe" => {
                if let Some(storage) = param(p, "storage") {
                    ev_se.write(EvPlaySe {
                        storage,
                        buf: param_u32(p, "buf"),
                        volume: param_f32(p, "volume"),
                        looping: param_bool(p, "loop").unwrap_or(false),
                    });
                }
            }
            "stopse" => {
                ev_stop_se.write(EvStopSe { buf: param_u32(p, "buf") });
            }
            "vo" | "voice" => {
                if let Some(storage) = param(p, "storage") {
                    ev_voice.write(EvPlayVoice {
                        storage,
                        buf: param_u32(p, "buf"),
                    });
                }
            }
            "fadebgm" => {
                ev_fade.write(EvFadeBgm {
                    time: param_u64(p, "time"),
                    volume: param_f32(p, "volume"),
                });
            }
            _ => {}
        }
    }
}
