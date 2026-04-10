#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

//! Runtime bridge crate between `kag-interpreter` and a Bevy ECS host.
//!
//! ## What this crate provides
//! - [`KaniRuntimePlugin`] to wire bridge resources/systems into your app.
//! - [`asset::AssetBackend`] to load scenario text from filesystem or `.pak`.
//! - [`bridge::InterpreterBridge`] to communicate with the running interpreter.
//! - Message types in [`events`] for host-side rendering/audio systems.

pub mod asset;
pub mod bridge;
pub mod events;
pub mod systems;

use asset::AssetBackend;
use bevy::prelude::{App, Plugin, Update};
use bridge::spawn_interpreter;

/// Plugin that installs the runtime bridge into a Bevy app.
///
/// This plugin:
/// 1. Registers `.pak` asset source when `AssetBackend::Pak` is used.
/// 2. Spawns and inserts an [`bridge::InterpreterBridge`].
/// 3. Registers all bridge messages in [`events`].
/// 4. Adds polling and input-forwarding systems.
pub struct KaniRuntimePlugin {
    /// Asset backend used for scenario loading and optional `pak://` registration.
    pub asset_backend: AssetBackend,
    /// Entry script path (for example, `"scenario/first.ks"`).
    pub entry_script: String,
}

impl Plugin for KaniRuntimePlugin {
    fn build(&self, app: &mut App) {
        self.asset_backend.register_bevy_source(app);

        let bridge = spawn_interpreter(self.entry_script.clone(), self.asset_backend.clone());
        app.insert_resource(self.asset_backend.clone())
            .insert_resource(bridge)
            .add_message::<events::EvDisplayText>()
            .add_message::<events::EvInsertLineBreak>()
            .add_message::<events::EvClearMessage>()
            .add_message::<events::EvClearCurrentMessage>()
            .add_message::<events::EvBeginChoices>()
            .add_message::<events::EvInputRequested>()
            .add_message::<events::EvEmbedText>()
            .add_message::<events::EvPushBacklog>()
            .add_message::<events::EvSnapshot>()
            .add_message::<events::EvUnknownTag>()
            .add_message::<events::EvImageTag>()
            .add_message::<events::EvAudioTag>()
            .add_message::<events::EvTransitionTag>()
            .add_message::<events::EvEffectTag>()
            .add_message::<events::EvMessageTag>()
            .add_message::<events::EvCharaTag>()
            .add_systems(Update, systems::poll_interpreter)
            .add_systems(Update, systems::input_bridge);
    }
}
