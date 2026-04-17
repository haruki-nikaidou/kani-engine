//! Background and layer image tag handlers.
//!
//! Handles: `[bg]`, `[image]`, `[layopt]`, `[free]`/`[freeimage]`/`[freelayer]`,
//! `[position]`, `[backlay]`, `[current]`, `[locate]`, `[layermode]`,
//! `[free_layermode]`, `[filter]`, `[free_filter]`, `[position_filter]`,
//! `[mask]`, `[mask_off]`, `[graph]`.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{EvLayerTag, EvTagRouted};

pub fn handle_image_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev: MessageWriter<EvLayerTag>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Bg {
                storage: Some(storage),
                time,
                method,
            } => {
                ev.write(EvLayerTag::SetBackground { storage, time, method });
            }
            ResolvedTag::Image {
                storage: Some(storage),
                layer,
                x,
                y,
                visible,
            } => {
                ev.write(EvLayerTag::SetImageLayer { storage, layer, x, y, visible });
            }
            ResolvedTag::Layopt {
                layer: Some(layer),
                visible,
                opacity,
            } => {
                ev.write(EvLayerTag::SetLayerOpt { layer, visible, opacity });
            }
            ResolvedTag::Free { layer: Some(layer) } => {
                ev.write(EvLayerTag::FreeLayer { layer });
            }
            ResolvedTag::Position {
                layer: Some(layer),
                x,
                y,
            } => {
                ev.write(EvLayerTag::SetLayerPosition { layer, x, y });
            }
            ResolvedTag::Backlay => {
                ev.write(EvLayerTag::Backlay);
            }
            ResolvedTag::Current { layer } => {
                ev.write(EvLayerTag::SetCurrentLayer { layer });
            }
            ResolvedTag::Locate { x, y } => {
                ev.write(EvLayerTag::LocateCursor { x, y });
            }
            ResolvedTag::Layermode { layer, mode } => {
                ev.write(EvLayerTag::SetLayerMode { layer, mode });
            }
            ResolvedTag::FreeLayermode { layer } => {
                ev.write(EvLayerTag::ResetLayerMode { layer });
            }
            ResolvedTag::Filter { layer, filter_type } => {
                ev.write(EvLayerTag::SetFilter { layer, filter_type });
            }
            ResolvedTag::FreeFilter { layer } => {
                ev.write(EvLayerTag::FreeFilter { layer });
            }
            ResolvedTag::PositionFilter { layer, x, y } => {
                ev.write(EvLayerTag::PositionFilter { layer, x, y });
            }
            ResolvedTag::Mask { layer, storage } => {
                ev.write(EvLayerTag::SetMask { layer, storage });
            }
            ResolvedTag::MaskOff { layer } => {
                ev.write(EvLayerTag::RemoveMask { layer });
            }
            ResolvedTag::Graph { layer, shape, x, y, width, height, color } => {
                ev.write(EvLayerTag::DrawGraph { layer, shape, x, y, width, height, color });
            }
            _ => {}
        }
    }
}
