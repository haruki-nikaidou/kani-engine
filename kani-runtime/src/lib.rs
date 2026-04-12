//! `kani-runtime` — Bevy runtime bridge for the KAG interpreter.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use kani_runtime::{AssetBackend, KaniRuntimePlugin};
//! use std::path::PathBuf;
//!
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(KaniRuntimePlugin {
//!         asset_backend: AssetBackend::FileSystem {
//!             base: PathBuf::from("assets"),
//!         },
//!         entry_script: "scenario/first.ks".into(),
//!     })
//!     .run();
//! ```

mod asset;
mod bridge;
pub mod events;
pub mod systems;

pub use asset::AssetBackend;
pub use bridge::{BridgeState, InterpreterBridge, spawn_interpreter};

use bevy::prelude::*;

use systems::input::{handle_click_input, handle_completion, handle_timer, handle_ui_inputs};
use systems::poll::poll_interpreter;
use systems::tags::{
    audio::handle_audio_tags,
    chara::handle_chara_tags,
    effect::handle_effect_tags,
    emit_unknown_tags,
    image::handle_image_tags,
    message::handle_message_tags,
    transition::handle_transition_tags,
};

// ─── Plugin ───────────────────────────────────────────────────────────────────

/// Bevy plugin that:
/// 1. Registers the asset source (pak or filesystem).
/// 2. Spawns the interpreter thread and inserts [`InterpreterBridge`].
/// 3. Registers all bridge events.
/// 4. Adds all bridge systems to `Update`.
pub struct KaniRuntimePlugin {
    pub asset_backend: AssetBackend,
    /// Path to the entry scenario file, e.g. `"scenario/first.ks"`.
    pub entry_script: String,
}

impl Plugin for KaniRuntimePlugin {
    fn build(&self, app: &mut App) {
        // 1. Register asset source
        self.asset_backend.clone().register_bevy_source(app);

        // Insert backend as a resource so systems can do synchronous reads.
        app.insert_resource(self.asset_backend.clone());

        // 2. Spawn interpreter and insert bridge resource
        let bridge = spawn_interpreter(&self.entry_script, &self.asset_backend)
            .unwrap_or_else(|e| panic!("kani-runtime: failed to start interpreter: {e:#}"));
        app.insert_resource(bridge);

        // 3. Register all events
        app
            // core interpreter → Bevy
            .add_message::<events::EvDisplayText>()
            .add_message::<events::EvInsertLineBreak>()
            .add_message::<events::EvClearMessage>()
            .add_message::<events::EvClearCurrentMessage>()
            .add_message::<events::EvBeginChoices>()
            .add_message::<events::EvInputRequested>()
            .add_message::<events::EvEmbedText>()
            .add_message::<events::EvPushBacklog>()
            .add_message::<events::EvSnapshot>()
            // internal tag routing
            .add_message::<events::EvTagRouted>()
            // public unknown-tag escape hatch
            .add_message::<events::EvUnknownTag>()
            // image
            .add_message::<events::EvSetBackground>()
            .add_message::<events::EvSetImageLayer>()
            .add_message::<events::EvSetLayerOpt>()
            .add_message::<events::EvFreeLayer>()
            .add_message::<events::EvSetLayerPosition>()
            // audio
            .add_message::<events::EvPlayBgm>()
            .add_message::<events::EvStopBgm>()
            .add_message::<events::EvPlaySe>()
            .add_message::<events::EvStopSe>()
            .add_message::<events::EvPlayVoice>()
            .add_message::<events::EvFadeBgm>()
            // transition
            .add_message::<events::EvRunTransition>()
            .add_message::<events::EvFadeScreen>()
            .add_message::<events::EvMoveLayerTransition>()
            // effect
            .add_message::<events::EvQuake>()
            .add_message::<events::EvShake>()
            .add_message::<events::EvFlash>()
            // message window
            .add_message::<events::EvMessageWindow>()
            .add_message::<events::EvWindowControl>()
            .add_message::<events::EvResetFont>()
            .add_message::<events::EvSetFont>()
            .add_message::<events::EvSetRuby>()
            .add_message::<events::EvSetNowrap>()
            // character sprites
            .add_message::<events::EvSetCharacter>()
            .add_message::<events::EvHideCharacter>()
            .add_message::<events::EvFreeCharacter>()
            .add_message::<events::EvModCharacter>()
            // host → interpreter
            .add_message::<events::EvSelectChoice>()
            .add_message::<events::EvSubmitInput>()
            .add_message::<events::EvFireTrigger>()
            .add_message::<events::EvCompletionSignal>();

        // 4. Add systems
        //
        // Order:
        //   poll_interpreter          – drains channel, emits EvTagRouted
        //   tag handlers              – read EvTagRouted, emit typed action events
        //   emit_unknown_tags         – turns unrecognised EvTagRouted into EvUnknownTag
        //   input systems             – translate Bevy input → HostEvent
        app.add_systems(
            Update,
            (
                poll_interpreter,
                (
                    handle_image_tags,
                    handle_audio_tags,
                    handle_transition_tags,
                    handle_effect_tags,
                    handle_message_tags,
                    handle_chara_tags,
                    emit_unknown_tags,
                )
                    .after(poll_interpreter),
                (handle_click_input, handle_timer, handle_ui_inputs, handle_completion)
                    .after(poll_interpreter),
            ),
        );
    }
}

// Make AssetBackend a Bevy Resource so poll.rs can access it.
impl Resource for AssetBackend {}
