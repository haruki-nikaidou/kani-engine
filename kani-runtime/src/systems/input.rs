use std::time::Instant;

use bevy::input::ButtonInput;
use bevy::input::mouse::MouseButton;
use bevy::prelude::{Res, ResMut};
use kag_interpreter::HostEvent;

use crate::bridge::{BridgeState, InterpreterBridge};

pub fn input_bridge(mouse: Res<ButtonInput<MouseButton>>, mut bridge: ResMut<InterpreterBridge>) {
    match &bridge.state {
        BridgeState::WaitingClick { .. } | BridgeState::Stopped => {
            if mouse.just_pressed(MouseButton::Left)
                && bridge.input_tx.try_send(HostEvent::Clicked).is_ok()
            {
                bridge.state = BridgeState::Running;
            }
        }
        BridgeState::WaitingMs { deadline } => {
            if Instant::now() >= *deadline
                && bridge.input_tx.try_send(HostEvent::TimerElapsed).is_ok()
            {
                bridge.state = BridgeState::Running;
            }
        }
        _ => {}
    }
}
