//! Character sprite tag handlers (`[chara]`, `[chara_hide]`, `[chara_free]`,
//! `[chara_mod]`).

use bevy::prelude::*;

use crate::events::{
    EvFreeCharacter, EvHideCharacter, EvModCharacter, EvSetCharacter, EvTagRouted,
};
use super::{param, param_f32};

pub fn handle_chara_tags(
    mut reader: EventReader<EvTagRouted>,
    mut ev_set: EventWriter<EvSetCharacter>,
    mut ev_hide: EventWriter<EvHideCharacter>,
    mut ev_free: EventWriter<EvFreeCharacter>,
    mut ev_mod: EventWriter<EvModCharacter>,
) {
    for tag in reader.read() {
        let p = &tag.params;
        // Both `name=` and `id=` are acceptable identifiers across KAG variants.
        let id = || param(p, "name").or_else(|| param(p, "id"));

        match tag.name.as_str() {
            "chara" => {
                ev_set.write(EvSetCharacter {
                    id: id(),
                    storage: param(p, "storage"),
                    slot: param(p, "slot"),
                    x: param_f32(p, "x"),
                    y: param_f32(p, "y"),
                });
            }
            "chara_hide" => {
                ev_hide.write(EvHideCharacter { id: id(), slot: param(p, "slot") });
            }
            "chara_free" => {
                ev_free.write(EvFreeCharacter { id: id(), slot: param(p, "slot") });
            }
            "chara_mod" => {
                ev_mod.write(EvModCharacter {
                    id: id(),
                    face: param(p, "face"),
                    pose: param(p, "pose"),
                    storage: param(p, "storage"),
                });
            }
            _ => {}
        }
    }
}
