//! Character sprite tag handlers.
//!
//! Handles: `[chara_show]`, `[chara_hide]`, `[chara_hide_all]`, `[chara_free]`,
//! `[chara_delete]`, `[chara_mod]`, `[chara_move]`, `[chara_layer]`,
//! `[chara_layer_mod]`, `[chara_part]`, `[chara_part_reset]`.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvCharacterTag, EvTagRouted};

pub fn handle_chara_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev: MessageWriter<EvCharacterTag>,
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
                ev.write(EvCharacterTag::ShowCharacter {
                    name,
                    storage,
                    x,
                    y,
                    time,
                    method,
                });
            }
            ResolvedTag::CharaHide { name, time, method } => {
                ev.write(EvCharacterTag::HideCharacter { name, time, method });
            }
            ResolvedTag::CharaHideAll { time, method } => {
                ev.write(EvCharacterTag::HideAllCharacters { time, method });
            }
            ResolvedTag::CharaFree { name } => {
                ev.write(EvCharacterTag::FreeCharacter { name });
            }
            ResolvedTag::CharaDelete { name } => {
                ev.write(EvCharacterTag::DeleteCharacter { name });
            }
            ResolvedTag::CharaMod {
                name,
                storage,
                face,
                pose,
            } => {
                ev.write(EvCharacterTag::ModCharacter {
                    name,
                    storage,
                    face,
                    pose,
                });
            }
            ResolvedTag::CharaMove { name, x, y, time } => {
                ev.write(EvCharacterTag::MoveCharacter { name, x, y, time });
            }
            ResolvedTag::CharaLayer { name, layer } => {
                ev.write(EvCharacterTag::SetCharacterLayer { name, layer });
            }
            ResolvedTag::CharaLayerMod {
                name,
                opacity,
                visible,
            } => {
                ev.write(EvCharacterTag::ModCharacterLayer {
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
                ev.write(EvCharacterTag::SetCharacterPart {
                    name,
                    part,
                    storage,
                });
            }
            ResolvedTag::CharaPartReset { name } => {
                ev.write(EvCharacterTag::ResetCharacterParts { name });
            }
            _ => {}
        }
    }
}
