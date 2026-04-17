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
                storage,
                time,
                method,
            } => {
                if let Some(storage) = storage {
                    ev_bg.write(EvSetBackground {
                        storage,
                        time,
                        method,
                    });
                }
            }
            ResolvedTag::Image {
                storage,
                layer,
                x,
                y,
                visible,
            } => {
                if let Some(storage) = storage {
                    ev_image.write(EvSetImageLayer {
                        storage,
                        layer,
                        x,
                        y,
                        visible,
                    });
                }
            }
            ResolvedTag::Layopt {
                layer,
                visible,
                opacity,
            } => {
                if let Some(layer) = layer {
                    ev_layopt.write(EvSetLayerOpt {
                        layer,
                        visible,
                        opacity,
                    });
                }
            }
            ResolvedTag::Free { layer } => {
                if let Some(layer) = layer {
                    ev_free.write(EvFreeLayer { layer });
                }
            }
            ResolvedTag::Position { layer, x, y } => {
                if let Some(layer) = layer {
                    ev_pos.write(EvSetLayerPosition { layer, x, y });
                }
            }
            _ => {}
        }
    }
}
