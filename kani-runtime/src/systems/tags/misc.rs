//! Miscellaneous tag handlers (`[web]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::EvMiscTag;

pub fn dispatch(resolved: ResolvedTag, ev: &mut MessageWriter<EvMiscTag>) {
    if let ResolvedTag::Web { url: Some(url) } = resolved {
        if let Err(e) = open::that(&url) {
            warn!("[kani-runtime] [web] failed to open URL {url:?}: {e}");
        }
        ev.write(EvMiscTag::OpenUrl { url });
    }
}
