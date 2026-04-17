//! Miscellaneous tag handlers (`[web]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvOpenUrl, EvTagRouted};

pub fn handle_misc_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_url: MessageWriter<EvOpenUrl>,
) {
    for tag in reader.read() {
        if let ResolvedTag::Web { url: Some(url) } = tag.0.clone() {
            // Also open via the system browser immediately.
            if let Err(e) = open::that(&url) {
                warn!("[kani-runtime] [web] failed to open URL {url:?}: {e}");
            }
            ev_url.write(EvOpenUrl { url });
        }
    }
}
