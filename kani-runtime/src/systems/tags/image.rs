//! Background and layer image tag handlers (`[bg]`, `[image]`, `[layopt]`,
//! `[free]`, `[position]`).

use bevy::prelude::*;

use crate::events::{
    EvFreeLayer, EvSetBackground, EvSetImageLayer, EvSetLayerOpt, EvSetLayerPosition, EvTagRouted,
};
use super::{param, param_bool, param_f32, param_u64};

pub fn handle_image_tags(
    mut reader: MessageReader<EvTagRouted>,
    mut ev_bg: MessageWriter<EvSetBackground>,
    mut ev_image: MessageWriter<EvSetImageLayer>,
    mut ev_layopt: MessageWriter<EvSetLayerOpt>,
    mut ev_free: MessageWriter<EvFreeLayer>,
    mut ev_pos: MessageWriter<EvSetLayerPosition>,
) {
    for tag in reader.read() {
        let p = &tag.params;
        match tag.name.as_str() {
            "bg" => {
                if let Some(storage) = param(p, "storage") {
                    ev_bg.write(EvSetBackground {
                        storage,
                        time: param_u64(p, "time"),
                        method: param(p, "method"),
                    });
                }
            }
            "image" => {
                if let Some(storage) = param(p, "storage") {
                    ev_image.write(EvSetImageLayer {
                        storage,
                        layer: param(p, "layer"),
                        x: param_f32(p, "x"),
                        y: param_f32(p, "y"),
                        visible: param_bool(p, "visible"),
                    });
                }
            }
            "layopt" => {
                if let Some(layer) = param(p, "layer") {
                    ev_layopt.write(EvSetLayerOpt {
                        layer,
                        visible: param_bool(p, "visible"),
                        opacity: param_f32(p, "opacity"),
                    });
                }
            }
            "free" => {
                if let Some(layer) = param(p, "layer") {
                    ev_free.write(EvFreeLayer { layer });
                }
            }
            "position" => {
                if let Some(layer) = param(p, "layer") {
                    ev_pos.write(EvSetLayerPosition {
                        layer,
                        x: param_f32(p, "x"),
                        y: param_f32(p, "y"),
                    });
                }
            }
            _ => {}
        }
    }
}
