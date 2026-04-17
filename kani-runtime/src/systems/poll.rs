//! `poll_interpreter` — drain the interpreter event channel each frame.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use kag_interpreter::KagEvent;
use tokio::sync::mpsc::error::TryRecvError;

use crate::asset::AssetBackend;
use crate::bridge::{BridgeState, InterpreterBridge};
use crate::events::{EvInterpreterCall, EvTagRouted};
use crate::systems::scenario::load_and_send;

/// Bevy system — called every `Update` frame.
///
/// Drains `InterpreterBridge::event_rx` in a tight loop and maps each
/// `KagEvent` variant to a `BridgeState` change and/or a Bevy event emission.
///
/// `Jump` and `Return` events that reference a different scenario file are
/// handled here via `load_and_send`: the `.ks` source is read synchronously
/// from `AssetBackend` and immediately queued back as `HostEvent::ScenarioLoaded`.
pub fn poll_interpreter(
    mut bridge: ResMut<InterpreterBridge>,
    backend: Res<AssetBackend>,
    mut ev: MessageWriter<EvInterpreterCall>,
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
            KagEvent::DisplayText {
                text,
                spans,
                speaker,
                speed,
                log,
            } => {
                ev.write(EvInterpreterCall::DisplayText {
                    text,
                    spans,
                    speaker,
                    speed,
                    log,
                });
            }
            KagEvent::InsertLineBreak => {
                ev.write(EvInterpreterCall::InsertLineBreak);
            }
            KagEvent::ClearMessage => {
                ev.write(EvInterpreterCall::ClearMessage);
            }
            KagEvent::ClearCurrentMessage => {
                ev.write(EvInterpreterCall::ClearCurrentMessage);
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
            KagEvent::WaitForCompletion {
                which,
                canskip,
                buf,
            } => {
                bridge.state = BridgeState::WaitingCompletion {
                    which,
                    canskip,
                    buf,
                };
            }
            KagEvent::WaitForRawClick => {
                bridge.state = BridgeState::WaitingClick { clear_after: false };
            }
            KagEvent::InputRequested {
                name,
                prompt,
                title,
            } => {
                bridge.state = BridgeState::WaitingInput {
                    var_name: name.clone(),
                };
                ev.write(EvInterpreterCall::InputRequested {
                    name,
                    prompt,
                    title,
                });
            }
            KagEvent::WaitForTrigger { name } => {
                bridge.state = BridgeState::WaitingTrigger { name };
            }

            // ── Navigation ────────────────────────────────────────────────────
            KagEvent::Jump {
                storage: Some(storage),
                ..
            } => {
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
                ev.write(EvInterpreterCall::BeginChoice(opts));
            }

            // ── Misc output ───────────────────────────────────────────────────
            KagEvent::EmbedText(text) => {
                ev.write(EvInterpreterCall::EmbedTest(text));
            }
            KagEvent::Trace(msg) => {
                info!("[kag trace] {msg}");
            }
            KagEvent::PushBacklog { text, join } => {
                ev.write(EvInterpreterCall::PushBacklog { text, join });
            }

            // ── Passthrough tags ──────────────────────────────────────────────
            KagEvent::Tag(resolved_tag) => {
                ev.write(EvInterpreterCall::TagRouted(EvTagRouted(resolved_tag)));
            }

            // ── Lifecycle ─────────────────────────────────────────────────────
            KagEvent::End => {
                bridge.state = BridgeState::Ended;
            }
            KagEvent::Diagnostic(diag) => {
                if diag.is_fatal() {
                    error!("[kag] {diag}");
                    bridge.state = BridgeState::Ended;
                } else {
                    warn!("[kag] {diag}");
                }
            }
            KagEvent::Snapshot(snap) => {
                ev.write(EvInterpreterCall::Snapshot(snap));
            }
        }
    }
}
