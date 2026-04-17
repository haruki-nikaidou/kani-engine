//! Background and layer image tag handlers.
//!
//! Handles: `[bg]`, `[image]`, `[layopt]`, `[free]`/`[freeimage]`/`[freelayer]`,
//! `[position]`, `[backlay]`, `[current]`, `[locate]`, `[layermode]`,
//! `[free_layermode]`, `[filter]`, `[free_filter]`, `[position_filter]`,
//! `[mask]`, `[mask_off]`, `[graph]`.

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{
    EvBacklay, EvDrawGraph, EvFreeFilter, EvFreeLayer, EvLocateCursor, EvPositionFilter,
    EvRemoveMask, EvResetLayerMode, EvSetBackground, EvSetCurrentLayer, EvSetFilter,
    EvSetImageLayer, EvSetLayerMode, EvSetLayerOpt, EvSetLayerPosition, EvSetMask, EvTagRouted,
};

#[allow(clippy::too_many_arguments)]
pub fn handle_image_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_bg: MessageWriter<EvSetBackground>,
    mut ev_image: MessageWriter<EvSetImageLayer>,
    mut ev_layopt: MessageWriter<EvSetLayerOpt>,
    mut ev_free: MessageWriter<EvFreeLayer>,
    mut ev_pos: MessageWriter<EvSetLayerPosition>,
    mut ev_backlay: MessageWriter<EvBacklay>,
    mut ev_current: MessageWriter<EvSetCurrentLayer>,
    mut ev_locate: MessageWriter<EvLocateCursor>,
    mut ev_layermode: MessageWriter<EvSetLayerMode>,
    mut ev_reset_layermode: MessageWriter<EvResetLayerMode>,
    mut ev_filter: MessageWriter<EvSetFilter>,
    mut ev_free_filter: MessageWriter<EvFreeFilter>,
    mut ev_pos_filter: MessageWriter<EvPositionFilter>,
    mut ev_mask: MessageWriter<EvSetMask>,
    mut ev_remove_mask: MessageWriter<EvRemoveMask>,
    mut ev_graph: MessageWriter<EvDrawGraph>,
) {
    for tag in reader.read() {
        match tag.0.clone() {
            ResolvedTag::Bg {
                storage: Some(storage),
                time,
                method,
            } => {
                ev_bg.write(EvSetBackground {
                    storage,
                    time,
                    method,
                });
            }
            ResolvedTag::Image {
                storage: Some(storage),
                layer,
                x,
                y,
                visible,
            } => {
                ev_image.write(EvSetImageLayer {
                    storage,
                    layer,
                    x,
                    y,
                    visible,
                });
            }
            ResolvedTag::Layopt {
                layer: Some(layer),
                visible,
                opacity,
            } => {
                ev_layopt.write(EvSetLayerOpt {
                    layer,
                    visible,
                    opacity,
                });
            }
            ResolvedTag::Free { layer: Some(layer) } => {
                ev_free.write(EvFreeLayer { layer });
            }
            ResolvedTag::Position {
                layer: Some(layer),
                x,
                y,
            } => {
                ev_pos.write(EvSetLayerPosition { layer, x, y });
            }
            ResolvedTag::Backlay => {
                ev_backlay.write(EvBacklay);
            }
            ResolvedTag::Current { layer } => {
                ev_current.write(EvSetCurrentLayer { layer });
            }
            ResolvedTag::Locate { x, y } => {
                ev_locate.write(EvLocateCursor { x, y });
            }
            ResolvedTag::Layermode { layer, mode } => {
                ev_layermode.write(EvSetLayerMode { layer, mode });
            }
            ResolvedTag::FreeLayermode { layer } => {
                ev_reset_layermode.write(EvResetLayerMode { layer });
            }
            ResolvedTag::Filter { layer, filter_type } => {
                ev_filter.write(EvSetFilter { layer, filter_type });
            }
            ResolvedTag::FreeFilter { layer } => {
                ev_free_filter.write(EvFreeFilter { layer });
            }
            ResolvedTag::PositionFilter { layer, x, y } => {
                ev_pos_filter.write(EvPositionFilter { layer, x, y });
            }
            ResolvedTag::Mask { layer, storage } => {
                ev_mask.write(EvSetMask { layer, storage });
            }
            ResolvedTag::MaskOff { layer } => {
                ev_remove_mask.write(EvRemoveMask { layer });
            }
            ResolvedTag::Graph {
                layer,
                shape,
                x,
                y,
                width,
                height,
                color,
            } => {
                ev_graph.write(EvDrawGraph {
                    layer,
                    shape,
                    x,
                    y,
                    width,
                    height,
                    color,
                });
            }
            _ => {}
        }
    }
}
