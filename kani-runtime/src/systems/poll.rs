use bevy::ecs::system::SystemParam;
use bevy::log::{error, info, warn};
use bevy::prelude::{MessageWriter, Res, ResMut};
use kag_interpreter::KagEvent;

use crate::asset::AssetBackend;
use crate::bridge::{BridgeState, InterpreterBridge};
use crate::events::*;
use crate::systems::scenario::load_and_send_scenario;
use crate::systems::tags::route_tag;

#[derive(SystemParam)]
pub struct BridgeMessages<'w> {
    ev_display_text: MessageWriter<'w, EvDisplayText>,
    ev_insert_line: MessageWriter<'w, EvInsertLineBreak>,
    ev_clear_message: MessageWriter<'w, EvClearMessage>,
    ev_clear_current_message: MessageWriter<'w, EvClearCurrentMessage>,
    ev_begin_choices: MessageWriter<'w, EvBeginChoices>,
    ev_input_requested: MessageWriter<'w, EvInputRequested>,
    ev_embed_text: MessageWriter<'w, EvEmbedText>,
    ev_push_backlog: MessageWriter<'w, EvPushBacklog>,
    ev_snapshot: MessageWriter<'w, EvSnapshot>,
    ev_unknown: MessageWriter<'w, EvUnknownTag>,
    ev_image: MessageWriter<'w, EvImageTag>,
    ev_audio: MessageWriter<'w, EvAudioTag>,
    ev_transition: MessageWriter<'w, EvTransitionTag>,
    ev_effect: MessageWriter<'w, EvEffectTag>,
    ev_message: MessageWriter<'w, EvMessageTag>,
    ev_chara: MessageWriter<'w, EvCharaTag>,
}

pub fn poll_interpreter(
    mut bridge: ResMut<InterpreterBridge>,
    backend: Res<AssetBackend>,
    mut msgs: BridgeMessages,
) {
    while let Ok(event) = bridge.event_rx.try_recv() {
        match event {
            KagEvent::DisplayText {
                text,
                speaker,
                speed,
                log,
            } => {
                msgs.ev_display_text.write(EvDisplayText {
                    text,
                    speaker,
                    speed,
                    log,
                });
            }
            KagEvent::InsertLineBreak => {
                msgs.ev_insert_line.write(EvInsertLineBreak);
            }
            KagEvent::ClearMessage => {
                msgs.ev_clear_message.write(EvClearMessage);
            }
            KagEvent::ClearCurrentMessage => {
                msgs.ev_clear_current_message.write(EvClearCurrentMessage);
            }
            KagEvent::WaitForClick { clear_after } => {
                bridge.state = BridgeState::WaitingClick { clear_after };
            }
            KagEvent::WaitMs(ms) => bridge.state.set_wait_ms(ms),
            KagEvent::Stop => bridge.state = BridgeState::Stopped,
            KagEvent::WaitForCompletion { tag, params } => {
                bridge.state = BridgeState::WaitingCompletion { tag, params };
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
                msgs.ev_input_requested.write(EvInputRequested {
                    name,
                    prompt,
                    title,
                });
            }
            KagEvent::WaitForTrigger { name } => {
                bridge.state = BridgeState::WaitingTrigger { name };
            }
            KagEvent::BeginChoices(choices) => {
                bridge.state = BridgeState::WaitingChoice;
                msgs.ev_begin_choices.write(EvBeginChoices(choices));
            }
            KagEvent::Jump { storage, target: _ } => {
                if let Some(storage) = storage
                    && let Err(err) = load_and_send_scenario(&backend, &bridge.input_tx, &storage)
                {
                    error!("failed to load jump scenario: {err:#}");
                }
            }
            KagEvent::Return { storage } => {
                if let Err(err) = load_and_send_scenario(&backend, &bridge.input_tx, &storage) {
                    error!("failed to load return scenario: {err:#}");
                }
            }
            KagEvent::EmbedText(text) => {
                msgs.ev_embed_text.write(EvEmbedText(text));
            }
            KagEvent::Trace(msg) => info!("kag trace: {msg}"),
            KagEvent::PushBacklog { text, join } => {
                msgs.ev_push_backlog.write(EvPushBacklog { text, join });
            }
            KagEvent::Tag { name, params } => {
                route_tag(
                    name,
                    params,
                    &mut msgs.ev_image,
                    &mut msgs.ev_audio,
                    &mut msgs.ev_transition,
                    &mut msgs.ev_effect,
                    &mut msgs.ev_message,
                    &mut msgs.ev_chara,
                    &mut msgs.ev_unknown,
                );
            }
            KagEvent::End => bridge.state = BridgeState::Ended,
            KagEvent::Warning(message) => warn!("kag warning: {message}"),
            KagEvent::Error(message) => error!("kag error: {message}"),
            KagEvent::Snapshot(snapshot) => {
                msgs.ev_snapshot.write(EvSnapshot(snapshot));
            }
        }
    }
}
