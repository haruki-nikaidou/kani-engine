//! Input systems — translate `BridgeState` + Bevy input → `HostEvent`.

use std::time::Instant;

use bevy::prelude::*;
use kag_interpreter::HostEvent;

use crate::bridge::{BridgeState, InterpreterBridge};
use crate::events::EvHostInput;

/// Left mouse-button click → `HostEvent::Clicked` when the interpreter is
/// waiting for a click or is paused at `[s]`.
pub fn handle_click_input(
    mouse: Res<ButtonInput<MouseButton>>,
    mut bridge: ResMut<InterpreterBridge>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    match bridge.state {
        BridgeState::WaitingClick { .. } | BridgeState::Stopped => {
            if bridge.input_tx.try_send(HostEvent::Clicked).is_ok() {
                bridge.state = BridgeState::Running;
            }
        }
        _ => {}
    }
}

/// Every frame: if the `WaitMs` deadline has passed, send `TimerElapsed`.
pub fn handle_timer(mut bridge: ResMut<InterpreterBridge>) {
    let deadline = match bridge.state {
        BridgeState::WaitingMs { deadline } => deadline,
        _ => return,
    };
    if Instant::now() >= deadline && bridge.input_tx.try_send(HostEvent::TimerElapsed).is_ok() {
        bridge.state = BridgeState::Running;
    }
}

/// Forward host input events to the interpreter while in the matching wait state.
///
/// - `EvHostInput::SelectChoice` → `HostEvent::ChoiceSelected`
/// - `EvHostInput::SubmitInput`  → `HostEvent::InputResult`
/// - `EvHostInput::FireTrigger`  → `HostEvent::TriggerFired`
/// - `EvHostInput::CompletionSignal` → `HostEvent::CompletionSignal`
pub fn handle_ui_inputs(
    mut bridge: ResMut<InterpreterBridge>,
    mut events: MessageReader<EvHostInput>,
) {
    for ev in events.read() {
        match ev.clone() {
            EvHostInput::SelectChoice(idx) => {
                if matches!(bridge.state, BridgeState::WaitingChoice)
                    && bridge
                        .input_tx
                        .try_send(HostEvent::ChoiceSelected(idx))
                        .is_ok()
                {
                    bridge.state = BridgeState::Running;
                }
            }
            EvHostInput::SubmitInput(text) => {
                if matches!(bridge.state, BridgeState::WaitingInput { .. })
                    && bridge
                        .input_tx
                        .try_send(HostEvent::InputResult(text))
                        .is_ok()
                {
                    bridge.state = BridgeState::Running;
                }
            }
            EvHostInput::FireTrigger { name } => {
                if let BridgeState::WaitingTrigger { name: expected } = &bridge.state {
                    if name == *expected
                        && bridge
                            .input_tx
                            .try_send(HostEvent::TriggerFired { name })
                            .is_ok()
                    {
                        bridge.state = BridgeState::Running;
                    }
                }
            }
            EvHostInput::CompletionSignal => {
                if matches!(bridge.state, BridgeState::WaitingCompletion { .. })
                    && bridge
                        .input_tx
                        .try_send(HostEvent::CompletionSignal)
                        .is_ok()
                {
                    bridge.state = BridgeState::Running;
                }
            }
        }
    }
}
