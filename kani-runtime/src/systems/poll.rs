//! `poll_interpreter` — drain the interpreter event channel each frame.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use kag_interpreter::KagEvent;
use tokio::sync::mpsc::error::TryRecvError;

use crate::asset::AssetBackend;
use crate::bridge::{BridgeState, InterpreterBridge};
use crate::events::*;
use crate::systems::scenario::load_and_send;

/// Bevy system — called every `Update` frame.
///
/// Drains `InterpreterBridge::event_rx` in a tight loop and maps each
/// `KagEvent` variant to a `BridgeState` change and/or a Bevy event emission.
///
/// `Jump` and `Return` events that reference a different scenario file are
/// handled here via `load_and_send`: the `.ks` source is read synchronously
/// from `AssetBackend` and immediately queued back as `HostEvent::ScenarioLoaded`.
#[allow(clippy::too_many_arguments)]
pub fn poll_interpreter(
    mut bridge: ResMut<InterpreterBridge>,
    backend: Res<AssetBackend>,
    mut ev_text: MessageWriter<EvDisplayText>,
    mut ev_br: MessageWriter<EvInsertLineBreak>,
    mut ev_clear: MessageWriter<EvClearMessage>,
    mut ev_clear_cur: MessageWriter<EvClearCurrentMessage>,
    mut ev_choices: MessageWriter<EvBeginChoices>,
    mut ev_input: MessageWriter<EvInputRequested>,
    mut ev_embed: MessageWriter<EvEmbedText>,
    mut ev_backlog: MessageWriter<EvPushBacklog>,
    mut ev_snap: MessageWriter<EvSnapshot>,
    mut ev_tag: MessageWriter<EvTagRouted>,
) {
    loop {
        let event = match bridge.event_rx.try_recv() {
            Ok(e) => e,
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                bridge.state = BridgeState::Ended;
                break;
            }
        };

        match event {
            // ── Text output ───────────────────────────────────────────────────
            KagEvent::DisplayText { text, speaker, speed, log } => {
                ev_text.write(EvDisplayText { text, speaker, speed, log });
            }
            KagEvent::InsertLineBreak => {
                ev_br.write(EvInsertLineBreak);
            }
            KagEvent::ClearMessage => {
                ev_clear.write(EvClearMessage);
            }
            KagEvent::ClearCurrentMessage => {
                ev_clear_cur.write(EvClearCurrentMessage);
            }

            // ── Input waits ───────────────────────────────────────────────────
            KagEvent::WaitForClick { clear_after } => {
                bridge.state = BridgeState::WaitingClick { clear_after };
            }
            KagEvent::WaitMs(ms) => {
                bridge.state = BridgeState::WaitingMs {
                    deadline: Instant::now() + Duration::from_millis(ms),
                };
            }
            KagEvent::Stop => {
                bridge.state = BridgeState::Stopped;
            }
            KagEvent::WaitForCompletion { tag, params } => {
                bridge.state = BridgeState::WaitingCompletion { tag, params };
            }
            KagEvent::WaitForRawClick => {
                bridge.state = BridgeState::WaitingClick { clear_after: false };
            }
            KagEvent::InputRequested { name, prompt, title } => {
                bridge.state = BridgeState::WaitingInput { var_name: name.clone() };
                ev_input.write(EvInputRequested { name, prompt, title });
            }
            KagEvent::WaitForTrigger { name } => {
                bridge.state = BridgeState::WaitingTrigger { name };
            }

            // ── Navigation ────────────────────────────────────────────────────
            KagEvent::Jump { storage: Some(storage), .. } => {
                if let Err(e) = load_and_send(&backend, &bridge.input_tx, &storage) {
                    error!("[kani-runtime] Jump load failed: {e:#}");
                    bridge.state = BridgeState::Ended;
                }
            }
            KagEvent::Jump { storage: None, .. } => {
                // In-file jump — the interpreter resolves it internally, no host action needed.
            }
            KagEvent::Return { storage } => {
                if let Err(e) = load_and_send(&backend, &bridge.input_tx, &storage) {
                    error!("[kani-runtime] Return load failed: {e:#}");
                    bridge.state = BridgeState::Ended;
                }
            }

            // ── Choices ───────────────────────────────────────────────────────
            KagEvent::BeginChoices(opts) => {
                bridge.state = BridgeState::WaitingChoice;
                ev_choices.write(EvBeginChoices(opts));
            }

            // ── Misc output ───────────────────────────────────────────────────
            KagEvent::EmbedText(text) => {
                ev_embed.write(EvEmbedText(text));
            }
            KagEvent::Trace(msg) => {
                info!("[kag trace] {msg}");
            }
            KagEvent::PushBacklog { text, join } => {
                ev_backlog.write(EvPushBacklog { text, join });
            }

            // ── Passthrough tags ──────────────────────────────────────────────
            KagEvent::Tag { name, params } => {
                ev_tag.write(EvTagRouted { name, params });
            }

            // ── Lifecycle ─────────────────────────────────────────────────────
            KagEvent::End => {
                bridge.state = BridgeState::Ended;
            }
            KagEvent::Warning(msg) => warn!("[kag] {msg}"),
            KagEvent::Error(msg) => {
                error!("[kag] {msg}");
                bridge.state = BridgeState::Ended;
            }
            KagEvent::Snapshot(snap) => {
                ev_snap.write(EvSnapshot(snap));
            }
        }
    }
}
