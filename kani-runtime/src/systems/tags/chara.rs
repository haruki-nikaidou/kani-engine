//! Character sprite tag handlers.
//!
//! Handles: `[chara_show]`, `[chara_hide]`, `[chara_hide_all]`, `[chara_free]`,
//! `[chara_delete]`, `[chara_mod]`, `[chara_move]`, `[chara_layer]`,
//! `[chara_layer_mod]`, `[chara_part]`, `[chara_part_reset]`.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{
    EvDeleteCharacter, EvFreeCharacter, EvHideAllCharacters, EvHideCharacter, EvModCharacter,
    EvModCharacterLayer, EvMoveCharacter, EvResetCharacterParts, EvSetCharacterLayer,
    EvSetCharacterPart, EvShowCharacter, EvTagRouted,
};

#[allow(clippy::too_many_arguments)]
pub fn handle_chara_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_show: MessageWriter<EvShowCharacter>,
    mut ev_hide: MessageWriter<EvHideCharacter>,
    mut ev_hide_all: MessageWriter<EvHideAllCharacters>,
    mut ev_free: MessageWriter<EvFreeCharacter>,
    mut ev_delete: MessageWriter<EvDeleteCharacter>,
    mut ev_mod: MessageWriter<EvModCharacter>,
    mut ev_move: MessageWriter<EvMoveCharacter>,
    mut ev_layer: MessageWriter<EvSetCharacterLayer>,
    mut ev_layer_mod: MessageWriter<EvModCharacterLayer>,
    mut ev_part: MessageWriter<EvSetCharacterPart>,
    mut ev_part_reset: MessageWriter<EvResetCharacterParts>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::CharaShow {
                name,
                storage,
                x,
                y,
                time,
                method,
            } => {
                ev_show.write(EvShowCharacter {
                    name,
                    storage,
                    x,
                    y,
                    time,
                    method,
                });
            }
            ResolvedTag::CharaHide { name, time, method } => {
                ev_hide.write(EvHideCharacter { name, time, method });
            }
            ResolvedTag::CharaHideAll { time, method } => {
                ev_hide_all.write(EvHideAllCharacters { time, method });
            }
            ResolvedTag::CharaFree { name } => {
                ev_free.write(EvFreeCharacter { name });
            }
            ResolvedTag::CharaDelete { name } => {
                ev_delete.write(EvDeleteCharacter { name });
            }
            ResolvedTag::CharaMod {
                name,
                storage,
                face,
                pose,
            } => {
                ev_mod.write(EvModCharacter {
                    name,
                    storage,
                    face,
                    pose,
                });
            }
            ResolvedTag::CharaMove { name, x, y, time } => {
                ev_move.write(EvMoveCharacter { name, x, y, time });
            }
            ResolvedTag::CharaLayer { name, layer } => {
                ev_layer.write(EvSetCharacterLayer { name, layer });
            }
            ResolvedTag::CharaLayerMod {
                name,
                opacity,
                visible,
            } => {
                ev_layer_mod.write(EvModCharacterLayer {
                    name,
                    opacity,
                    visible,
                });
            }
            ResolvedTag::CharaPart {
                name,
                part,
                storage,
            } => {
                ev_part.write(EvSetCharacterPart {
                    name,
                    part,
                    storage,
                });
            }
            ResolvedTag::CharaPartReset { name } => {
                ev_part_reset.write(EvResetCharacterParts { name });
            }
            _ => {}
        }
    }
}
