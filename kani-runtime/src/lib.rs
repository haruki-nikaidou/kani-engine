#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//! `kani-runtime` — Bevy runtime bridge for the KAG interpreter.
//!
//! # Quick start (plugin API)
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
//!
//! # Launch functions (preferred for most callers)
//!
//! | Function | When to use |
//! |----------|-------------|
//! | [`run_release`] | Ship to players — reads assets from a `.pak` archive |
//! | [`run_develop`] | Dev tooling — reads assets from the filesystem (requires `develop` feature) |

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
    animation::handle_animation_tags, audio::handle_audio_tags, chara::handle_chara_tags,
    effect::handle_effect_tags, image::handle_image_tags, message::handle_message_tags,
    misc::handle_misc_tags, transition::handle_transition_tags, ui::handle_ui_tags,
    video::handle_video_tags,
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
        #[allow(clippy::panic)]
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
            // internal tag routing (ResolvedTag variants; game code matches Extension)
            .add_message::<events::EvTagRouted>()
            // image / layer
            .add_message::<events::EvSetBackground>()
            .add_message::<events::EvSetImageLayer>()
            .add_message::<events::EvSetLayerOpt>()
            .add_message::<events::EvFreeLayer>()
            .add_message::<events::EvSetLayerPosition>()
            .add_message::<events::EvBacklay>()
            .add_message::<events::EvSetCurrentLayer>()
            .add_message::<events::EvLocateCursor>()
            .add_message::<events::EvSetLayerMode>()
            .add_message::<events::EvResetLayerMode>()
            .add_message::<events::EvSetFilter>()
            .add_message::<events::EvFreeFilter>()
            .add_message::<events::EvPositionFilter>()
            .add_message::<events::EvSetMask>()
            .add_message::<events::EvRemoveMask>()
            .add_message::<events::EvDrawGraph>()
            // audio
            .add_message::<events::EvPlayBgm>()
            .add_message::<events::EvStopBgm>()
            .add_message::<events::EvPauseBgm>()
            .add_message::<events::EvResumeBgm>()
            .add_message::<events::EvFadeBgm>()
            .add_message::<events::EvCrossFadeBgm>()
            .add_message::<events::EvSetBgmOpt>()
            .add_message::<events::EvPlaySe>()
            .add_message::<events::EvStopSe>()
            .add_message::<events::EvPauseSe>()
            .add_message::<events::EvResumeSe>()
            .add_message::<events::EvSetSeOpt>()
            .add_message::<events::EvPlayVoice>()
            .add_message::<events::EvChangeVol>()
            // animation
            .add_message::<events::EvPlayAnim>()
            .add_message::<events::EvStopAnim>()
            .add_message::<events::EvPlayKanim>()
            .add_message::<events::EvStopKanim>()
            .add_message::<events::EvPlayXanim>()
            .add_message::<events::EvStopXanim>()
            // video
            .add_message::<events::EvPlayBgMovie>()
            .add_message::<events::EvStopBgMovie>()
            .add_message::<events::EvPlayMovie>()
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
            .add_message::<events::EvShowCharacter>()
            .add_message::<events::EvHideCharacter>()
            .add_message::<events::EvHideAllCharacters>()
            .add_message::<events::EvFreeCharacter>()
            .add_message::<events::EvDeleteCharacter>()
            .add_message::<events::EvModCharacter>()
            .add_message::<events::EvMoveCharacter>()
            .add_message::<events::EvSetCharacterLayer>()
            .add_message::<events::EvModCharacterLayer>()
            .add_message::<events::EvSetCharacterPart>()
            .add_message::<events::EvResetCharacterParts>()
            // misc
            .add_message::<events::EvOpenUrl>()
            // skip + key config
            .add_message::<events::EvSkipMode>()
            .add_message::<events::EvKeyConfig>()
            // ui
            .add_message::<events::EvSpawnButton>()
            .add_message::<events::EvSetClickable>()
            .add_message::<events::EvOpenPanel>()
            .add_message::<events::EvShowDialog>()
            .add_message::<events::EvSetCursor>()
            .add_message::<events::EvSetSpeakerBoxVisible>()
            .add_message::<events::EvSetGlyph>()
            .add_message::<events::EvModeEffect>()
            // host → interpreter
            .add_message::<events::EvSelectChoice>()
            .add_message::<events::EvSubmitInput>()
            .add_message::<events::EvFireTrigger>()
            .add_message::<events::EvCompletionSignal>();

        // 4. Add systems
        //
        // Order:
        //   poll_interpreter  – drains channel, emits EvTagRouted(ResolvedTag)
        //   tag handlers      – read EvTagRouted, dispatch typed action events
        //   input systems     – translate Bevy input → HostEvent
        app.add_systems(
            Update,
            (
                poll_interpreter,
                (
                    handle_image_tags,
                    handle_audio_tags,
                    handle_animation_tags,
                    handle_video_tags,
                    handle_transition_tags,
                    handle_effect_tags,
                    handle_message_tags,
                    handle_chara_tags,
                    handle_ui_tags,
                    handle_misc_tags,
                )
                    .after(poll_interpreter),
                (
                    handle_click_input,
                    handle_timer,
                    handle_ui_inputs,
                    handle_completion,
                )
                    .after(poll_interpreter),
            ),
        );
    }
}

// Make AssetBackend a Bevy Resource so poll.rs can access it.
impl Resource for AssetBackend {}

// ─── Launch functions ─────────────────────────────────────────────────────────

/// Run the game from a `.pak` archive.
///
/// Constructs a Bevy [`App`] with [`DefaultPlugins`] and [`KaniRuntimePlugin`],
/// then blocks until the application exits.  Returns an error if the pak file
/// cannot be opened.
pub fn run_release(
    pak_path: impl AsRef<std::path::Path>,
    entry_script: &str,
) -> anyhow::Result<()> {
    let backend = AssetBackend::from_pak(pak_path)?;
    build_app(backend, entry_script).run();
    Ok(())
}

/// Run the game directly from the filesystem (developer mode).
///
/// Assets are read from `base_path` on disk.  Bevy's built-in asset watcher
/// (enabled via the workspace `dev` feature) provides hot-reload for binary
/// assets.  Script (`.ks`) hot-reload is deferred to a future version.
///
/// Blocks until the Bevy application exits.
#[cfg(feature = "develop")]
pub fn run_develop(
    base_path: impl AsRef<std::path::Path>,
    entry_script: &str,
) -> anyhow::Result<()> {
    let backend = AssetBackend::FileSystem {
        base: base_path.as_ref().to_owned(),
    };
    build_app(backend, entry_script).run();
    Ok(())
}

fn build_app(backend: AssetBackend, entry_script: &str) -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(KaniRuntimePlugin {
            asset_backend: backend,
            entry_script: entry_script.to_owned(),
        });
    app
}
