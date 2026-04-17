//! Character sprite tag handlers (`[chara]`, `[chara_hide]`, `[chara_free]`,
//! `[chara_mod]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{
    EvFreeCharacter, EvHideCharacter, EvModCharacter, EvSetCharacter, EvTagRouted,
};

pub fn handle_chara_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_set: MessageWriter<EvSetCharacter>,
    mut ev_hide: MessageWriter<EvHideCharacter>,
    mut ev_free: MessageWriter<EvFreeCharacter>,
    mut ev_mod: MessageWriter<EvModCharacter>,
) {
    for tag in reader.read() {
        // Both `name=` and `id=` are acceptable identifiers across KAG variants;
        // prefer `name` if present, fall back to `id`.
        match tag.0.clone() {
            ResolvedTag::Chara { name, id, storage, slot, x, y } => {
                ev_set.write(EvSetCharacter {
                    id: name.or(id),
                    storage,
                    slot,
                    x,
                    y,
                });
            }
            ResolvedTag::CharaHide { name, id, slot } => {
                ev_hide.write(EvHideCharacter {
                    id: name.or(id),
                    slot,
                });
            }
            ResolvedTag::CharaFree { name, id, slot } => {
                ev_free.write(EvFreeCharacter {
                    id: name.or(id),
                    slot,
                });
            }
            ResolvedTag::CharaMod { name, id, face, pose, storage } => {
                ev_mod.write(EvModCharacter {
                    id: name.or(id),
                    face,
                    pose,
                    storage,
                });
            }
            _ => {}
        }
    }
}
