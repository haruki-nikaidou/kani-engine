//! Audio tag handlers (`[bgm]`/`[playbgm]`, `[stopbgm]`, `[fadeinbgm]`, `[fadeoutbgm]`,
//! `[pausebgm]`, `[resumebgm]`, `[fadebgm]`, `[xchgbgm]`, `[bgmopt]`,
//! `[se]`/`[playSe]`, `[stopse]`, `[pausese]`, `[resumese]`, `[seopt]`,
//! `[vo]`/`[voice]`, `[changevol]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{
    EvChangeVol, EvCrossFadeBgm, EvFadeBgm, EvPauseBgm, EvPauseSe, EvPlayBgm, EvPlaySe,
    EvPlayVoice, EvResumeBgm, EvResumeSe, EvSetBgmOpt, EvSetSeOpt, EvStopBgm, EvStopSe,
    EvTagRouted,
};

#[allow(clippy::too_many_arguments)]
pub fn handle_audio_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_bgm: MessageWriter<EvPlayBgm>,
    mut ev_stop_bgm: MessageWriter<EvStopBgm>,
    mut ev_pause_bgm: MessageWriter<EvPauseBgm>,
    mut ev_resume_bgm: MessageWriter<EvResumeBgm>,
    mut ev_fade: MessageWriter<EvFadeBgm>,
    mut ev_xfade: MessageWriter<EvCrossFadeBgm>,
    mut ev_bgm_opt: MessageWriter<EvSetBgmOpt>,
    mut ev_se: MessageWriter<EvPlaySe>,
    mut ev_stop_se: MessageWriter<EvStopSe>,
    mut ev_pause_se: MessageWriter<EvPauseSe>,
    mut ev_resume_se: MessageWriter<EvResumeSe>,
    mut ev_se_opt: MessageWriter<EvSetSeOpt>,
    mut ev_voice: MessageWriter<EvPlayVoice>,
    mut ev_changevol: MessageWriter<EvChangeVol>,
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
            ResolvedTag::Pausebgm { buf } => {
                ev_pause_bgm.write(EvPauseBgm { buf });
            }
            ResolvedTag::Resumebgm { buf } => {
                ev_resume_bgm.write(EvResumeBgm { buf });
            }
            ResolvedTag::Fadebgm { time, volume } => {
                ev_fade.write(EvFadeBgm { time, volume });
            }
            ResolvedTag::Xchgbgm {
                storage: Some(storage),
                time,
            } => {
                ev_xfade.write(EvCrossFadeBgm { storage, time });
            }
            ResolvedTag::Bgmopt { looping, seek } => {
                ev_bgm_opt.write(EvSetBgmOpt { looping, seek });
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
            ResolvedTag::Pausese { buf } => {
                ev_pause_se.write(EvPauseSe { buf });
            }
            ResolvedTag::Resumese { buf } => {
                ev_resume_se.write(EvResumeSe { buf });
            }
            ResolvedTag::Seopt { buf, looping } => {
                ev_se_opt.write(EvSetSeOpt { buf, looping });
            }
            ResolvedTag::Vo {
                storage: Some(storage),
                buf,
            } => {
                ev_voice.write(EvPlayVoice { storage, buf });
            }
            ResolvedTag::Changevol { target, vol, time } => {
                ev_changevol.write(EvChangeVol { target, vol, time });
            }
            _ => {}
        }
    }
}
