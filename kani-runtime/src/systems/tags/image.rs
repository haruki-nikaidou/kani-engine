//! Background and layer image tag handlers (`[bg]`, `[image]`, `[layopt]`,
//! `[free]`, `[position]`).

use bevy::prelude::*;
use kag_interpreter::ResolvedTag;

use crate::events::{
    EvFreeLayer, EvSetBackground, EvSetImageLayer, EvSetLayerOpt, EvSetLayerPosition, EvTagRouted,
};

pub fn handle_image_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_bg: MessageWriter<EvSetBackground>,
    mut ev_image: MessageWriter<EvSetImageLayer>,
    mut ev_layopt: MessageWriter<EvSetLayerOpt>,
    mut ev_free: MessageWriter<EvFreeLayer>,
    mut ev_pos: MessageWriter<EvSetLayerPosition>,
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
            _ => {}
        }
    }
}
