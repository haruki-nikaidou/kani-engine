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
            .add_event::<events::EvDisplayText>()
            .add_event::<events::EvInsertLineBreak>()
            .add_event::<events::EvClearMessage>()
            .add_event::<events::EvClearCurrentMessage>()
            .add_event::<events::EvBeginChoices>()
            .add_event::<events::EvInputRequested>()
            .add_event::<events::EvEmbedText>()
            .add_event::<events::EvPushBacklog>()
            .add_event::<events::EvSnapshot>()
            // internal tag routing
            .add_event::<events::EvTagRouted>()
            // public unknown-tag escape hatch
            .add_event::<events::EvUnknownTag>()
            // image
            .add_event::<events::EvSetBackground>()
            .add_event::<events::EvSetImageLayer>()
            .add_event::<events::EvSetLayerOpt>()
            .add_event::<events::EvFreeLayer>()
            .add_event::<events::EvSetLayerPosition>()
            // audio
            .add_event::<events::EvPlayBgm>()
            .add_event::<events::EvStopBgm>()
            .add_event::<events::EvPlaySe>()
            .add_event::<events::EvStopSe>()
            .add_event::<events::EvPlayVoice>()
            .add_event::<events::EvFadeBgm>()
            // transition
            .add_event::<events::EvRunTransition>()
            .add_event::<events::EvFadeScreen>()
            .add_event::<events::EvMoveLayerTransition>()
            // effect
            .add_event::<events::EvQuake>()
            .add_event::<events::EvShake>()
            .add_event::<events::EvFlash>()
            // message window
            .add_event::<events::EvMessageWindow>()
            .add_event::<events::EvWindowControl>()
            .add_event::<events::EvResetFont>()
            .add_event::<events::EvSetFont>()
            .add_event::<events::EvSetRuby>()
            .add_event::<events::EvSetNowrap>()
            // character sprites
            .add_event::<events::EvSetCharacter>()
            .add_event::<events::EvHideCharacter>()
            .add_event::<events::EvFreeCharacter>()
            .add_event::<events::EvModCharacter>()
            // host → interpreter
            .add_event::<events::EvSelectChoice>()
            .add_event::<events::EvSubmitInput>()
            .add_event::<events::EvFireTrigger>()
            .add_event::<events::EvCompletionSignal>();

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
