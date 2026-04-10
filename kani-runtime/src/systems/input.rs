use std::time::Instant;

use bevy::input::ButtonInput;
use bevy::input::mouse::MouseButton;
use bevy::log::warn;
use bevy::prelude::{Res, ResMut};
use kag_interpreter::HostEvent;

use crate::bridge::{BridgeState, InterpreterBridge};

pub fn input_bridge(mouse: Res<ButtonInput<MouseButton>>, mut bridge: ResMut<InterpreterBridge>) {
    match &bridge.state {
        BridgeState::WaitingClick { .. } | BridgeState::Stopped => {
            if mouse.just_pressed(MouseButton::Left) {
                match bridge.input_tx.try_send(HostEvent::Clicked) {
                    Ok(()) => bridge.state = BridgeState::Running,
                    Err(err) => warn!("failed to send HostEvent::Clicked to interpreter: {err}"),
                }
            }
        }
        BridgeState::WaitingMs { deadline } => {
            if Instant::now() >= *deadline {
                match bridge.input_tx.try_send(HostEvent::TimerElapsed) {
                    Ok(()) => bridge.state = BridgeState::Running,
                    Err(err) => {
                        warn!("failed to send HostEvent::TimerElapsed to interpreter: {err}")
                    }
                }
            }
        }
        _ => {}
    }
}
