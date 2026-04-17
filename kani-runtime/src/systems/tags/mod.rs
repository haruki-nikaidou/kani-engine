//! Tag-handler dispatch system.
//!
//! A single [`dispatch_tags`] system reads every [`EvTagRouted`] once per frame
//! and emits the appropriate strongly-typed action event.  This replaces the
//! previous design where 10 separate systems each independently read (and
//! cloned) every tag.

pub mod animation;
pub mod audio;
pub mod chara;
pub mod effect;
pub mod image;
pub mod message;
pub mod misc;
pub mod transition;
pub mod ui;
pub mod video;

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{
    EvAnimTag, EvAudioTag, EvCharacterTag, EvControlTag, EvEffectTag, EvLayerTag,
    EvMessageWindowTag, EvMiscTag, EvTagRouted, EvTransitionTag, EvUiTag, EvVideoTag,
};

/// Single Bevy system that reads all [`EvTagRouted`] messages and dispatches
/// each one to the correct typed event channel.
pub fn dispatch_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_layer: MessageWriter<EvLayerTag>,
    mut ev_audio: MessageWriter<EvAudioTag>,
    mut ev_anim: MessageWriter<EvAnimTag>,
    mut ev_video: MessageWriter<EvVideoTag>,
    mut ev_transition: MessageWriter<EvTransitionTag>,
    mut ev_effect: MessageWriter<EvEffectTag>,
    mut ev_message: MessageWriter<EvMessageWindowTag>,
    mut ev_chara: MessageWriter<EvCharacterTag>,
    mut ev_ui: MessageWriter<EvUiTag>,
    mut ev_ctrl: MessageWriter<EvControlTag>,
    mut ev_misc: MessageWriter<EvMiscTag>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            // ── Image / Layer ──────────────────────────────────────────
            resolved @ (ResolvedTag::Bg { .. }
            | ResolvedTag::Image { .. }
            | ResolvedTag::Layopt { .. }
            | ResolvedTag::Free { .. }
            | ResolvedTag::Position { .. }
            | ResolvedTag::Backlay
            | ResolvedTag::Current { .. }
            | ResolvedTag::Locate { .. }
            | ResolvedTag::Layermode { .. }
            | ResolvedTag::FreeLayermode { .. }
            | ResolvedTag::Filter { .. }
            | ResolvedTag::FreeFilter { .. }
            | ResolvedTag::PositionFilter { .. }
            | ResolvedTag::Mask { .. }
            | ResolvedTag::MaskOff { .. }
            | ResolvedTag::Graph { .. }) => {
                image::dispatch(resolved, &mut ev_layer);
            }

            // ── Audio ──────────────────────────────────────────────────
            resolved @ (ResolvedTag::Bgm { .. }
            | ResolvedTag::Stopbgm { .. }
            | ResolvedTag::Pausebgm { .. }
            | ResolvedTag::Resumebgm { .. }
            | ResolvedTag::Fadebgm { .. }
            | ResolvedTag::Xchgbgm { .. }
            | ResolvedTag::Bgmopt { .. }
            | ResolvedTag::Se { .. }
            | ResolvedTag::Stopse { .. }
            | ResolvedTag::Pausese { .. }
            | ResolvedTag::Resumese { .. }
            | ResolvedTag::Seopt { .. }
            | ResolvedTag::Vo { .. }
            | ResolvedTag::Changevol { .. }) => {
                audio::dispatch(resolved, &mut ev_audio);
            }

            // ── Animation ──────────────────────────────────────────────
            resolved @ (ResolvedTag::Anim { .. }
            | ResolvedTag::StopAnim { .. }
            | ResolvedTag::Kanim { .. }
            | ResolvedTag::StopKanim { .. }
            | ResolvedTag::Xanim { .. }
            | ResolvedTag::StopXanim { .. }) => {
                animation::dispatch(resolved, &mut ev_anim);
            }

            // ── Video ──────────────────────────────────────────────────
            resolved @ (ResolvedTag::Bgmovie { .. }
            | ResolvedTag::StopBgmovie
            | ResolvedTag::Movie { .. }) => {
                video::dispatch(resolved, &mut ev_video);
            }

            // ── Transition ─────────────────────────────────────────────
            resolved @ (ResolvedTag::Trans { .. }
            | ResolvedTag::Fadein { .. }
            | ResolvedTag::Fadeout { .. }
            | ResolvedTag::Movetrans { .. }) => {
                transition::dispatch(resolved, &mut ev_transition);
            }

            // ── Effect ─────────────────────────────────────────────────
            resolved @ (ResolvedTag::Quake { .. }
            | ResolvedTag::Shake { .. }
            | ResolvedTag::Flash { .. }) => {
                effect::dispatch(resolved, &mut ev_effect);
            }

            // ── Message window ─────────────────────────────────────────
            resolved @ (ResolvedTag::Msgwnd { .. }
            | ResolvedTag::Wndctrl { .. }
            | ResolvedTag::Resetfont
            | ResolvedTag::Font { .. }
            | ResolvedTag::Size { .. }
            | ResolvedTag::Bold { .. }
            | ResolvedTag::Italic { .. }
            | ResolvedTag::Ruby { .. }
            | ResolvedTag::Nowrap { .. }) => {
                message::dispatch(resolved, &mut ev_message);
            }

            // ── Character ──────────────────────────────────────────────
            resolved @ (ResolvedTag::CharaShow { .. }
            | ResolvedTag::CharaHide { .. }
            | ResolvedTag::CharaHideAll { .. }
            | ResolvedTag::CharaFree { .. }
            | ResolvedTag::CharaDelete { .. }
            | ResolvedTag::CharaMod { .. }
            | ResolvedTag::CharaMove { .. }
            | ResolvedTag::CharaLayer { .. }
            | ResolvedTag::CharaLayerMod { .. }
            | ResolvedTag::CharaPart { .. }
            | ResolvedTag::CharaPartReset { .. }) => {
                chara::dispatch(resolved, &mut ev_chara);
            }

            // ── UI ─────────────────────────────────────────────────────
            resolved @ (ResolvedTag::Button { .. }
            | ResolvedTag::Clickable { .. }
            | ResolvedTag::OpenPanel { .. }
            | ResolvedTag::Dialog { .. }
            | ResolvedTag::Cursor { .. }
            | ResolvedTag::SetSpeakerBoxVisible { .. }
            | ResolvedTag::SetGlyph { .. }
            | ResolvedTag::ModeEffect { .. }) => {
                ui::dispatch_ui(resolved, &mut ev_ui);
            }

            // ── Control ────────────────────────────────────────────────
            resolved @ (ResolvedTag::SkipMode { .. } | ResolvedTag::KeyConfig { .. }) => {
                ui::dispatch_control(resolved, &mut ev_ctrl);
            }

            // ── Misc ───────────────────────────────────────────────────
            resolved @ ResolvedTag::Web { .. } => {
                misc::dispatch(resolved, &mut ev_misc);
            }

            // Unknown / extension tags — silently ignored for now.
            _ => {}
        }
    }
}
