//! Audio tag handlers.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::EvAudioTag;

pub fn dispatch(resolved: ResolvedTag, ev: &mut MessageWriter<EvAudioTag>) {
    match resolved {
        ResolvedTag::Bgm {
            storage: Some(storage),
            looping,
            volume,
            fadetime,
        } => {
            ev.write(EvAudioTag::PlayBgm {
                storage,
                looping,
                volume,
                fadetime,
            });
        }
        ResolvedTag::Stopbgm { fadetime } => {
            ev.write(EvAudioTag::StopBgm { fadetime });
        }
        ResolvedTag::Pausebgm { buf } => {
            ev.write(EvAudioTag::PauseBgm { buf });
        }
        ResolvedTag::Resumebgm { buf } => {
            ev.write(EvAudioTag::ResumeBgm { buf });
        }
        ResolvedTag::Fadebgm { time, volume } => {
            ev.write(EvAudioTag::FadeBgm { time, volume });
        }
        ResolvedTag::Xchgbgm {
            storage: Some(storage),
            time,
        } => {
            ev.write(EvAudioTag::CrossFadeBgm { storage, time });
        }
        ResolvedTag::Bgmopt { looping, seek } => {
            ev.write(EvAudioTag::SetBgmOpt { looping, seek });
        }
        ResolvedTag::Se {
            storage: Some(storage),
            buf,
            volume,
            looping,
        } => {
            ev.write(EvAudioTag::PlaySe {
                storage,
                buf,
                volume,
                looping,
            });
        }
        ResolvedTag::Stopse { buf } => {
            ev.write(EvAudioTag::StopSe { buf });
        }
        ResolvedTag::Pausese { buf } => {
            ev.write(EvAudioTag::PauseSe { buf });
        }
        ResolvedTag::Resumese { buf } => {
            ev.write(EvAudioTag::ResumeSe { buf });
        }
        ResolvedTag::Seopt { buf, looping } => {
            ev.write(EvAudioTag::SetSeOpt { buf, looping });
        }
        ResolvedTag::Vo {
            storage: Some(storage),
            buf,
        } => {
            ev.write(EvAudioTag::PlayVoice { storage, buf });
        }
        ResolvedTag::Changevol { target, vol, time } => {
            ev.write(EvAudioTag::ChangeVol { target, vol, time });
        }
        _ => {}
    }
}
