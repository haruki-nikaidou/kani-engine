//! Input systems — translate `BridgeState` + Bevy input → `HostEvent`.

use std::time::Instant;

use bevy::prelude::*;
use kag_interpreter::HostEvent;

use crate::bridge::{BridgeState, InterpreterBridge};
use crate::events::{EvCompletionSignal, EvFireTrigger, EvSelectChoice, EvSubmitInput};

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

/// Forward `EvSelectChoice` → `HostEvent::ChoiceSelected` while waiting for a
/// choice, `EvSubmitInput` → `HostEvent::InputResult` while waiting for input,
/// and `EvFireTrigger` → `HostEvent::TriggerFired` while waiting for a trigger.
pub fn handle_ui_inputs(
    mut bridge: ResMut<InterpreterBridge>,
    mut choices: EventReader<EvSelectChoice>,
    mut inputs: EventReader<EvSubmitInput>,
    mut triggers: EventReader<EvFireTrigger>,
) {
    // Choice selection
    if matches!(bridge.state, BridgeState::WaitingChoice) {
        if let Some(&EvSelectChoice(idx)) = choices.read().next() {
            if bridge.input_tx.try_send(HostEvent::ChoiceSelected(idx)).is_ok() {
                bridge.state = BridgeState::Running;
                return;
            }
        }
    }

    // Text-input result
    if matches!(bridge.state, BridgeState::WaitingInput { .. }) {
        if let Some(EvSubmitInput(text)) = inputs.read().next().cloned() {
            if bridge.input_tx.try_send(HostEvent::InputResult(text)).is_ok() {
                bridge.state = BridgeState::Running;
                return;
            }
        }
    }

    // Named trigger
    let expected = match &bridge.state {
        BridgeState::WaitingTrigger { name } => Some(name.clone()),
        _ => None,
    };
    if let Some(expected) = expected {
        for EvFireTrigger { name } in triggers.read() {
            if *name == expected
                && bridge
                    .input_tx
                    .try_send(HostEvent::TriggerFired { name: name.clone() })
                    .is_ok()
            {
                bridge.state = BridgeState::Running;
                break;
            }
        }
    }
}

/// Forward `EvCompletionSignal` → `HostEvent::CompletionSignal` while the
/// interpreter is blocked on a `WaitForCompletion` / `[wa]`-family wait.
pub fn handle_completion(
    mut bridge: ResMut<InterpreterBridge>,
    mut signals: EventReader<EvCompletionSignal>,
) {
    if !matches!(bridge.state, BridgeState::WaitingCompletion { .. }) {
        return;
    }
    if signals.read().next().is_some()
        && bridge.input_tx.try_send(HostEvent::CompletionSignal).is_ok()
    {
        bridge.state = BridgeState::Running;
    }
}
