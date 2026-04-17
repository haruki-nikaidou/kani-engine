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

use systems::input::{handle_click_input, handle_timer, handle_ui_inputs};
use systems::poll::poll_interpreter;
use systems::tags::dispatch_tags;

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
            .add_message::<events::EvInterpreterCall>()
            // tag routing (interpreter → dispatch)
            .add_message::<events::EvTagRouted>()
            // image / layer
            .add_message::<events::EvLayerTag>()
            // audio
            .add_message::<events::EvAudioTag>()
            // animation
            .add_message::<events::EvAnimTag>()
            // video
            .add_message::<events::EvVideoTag>()
            // transition
            .add_message::<events::EvTransitionTag>()
            // effect
            .add_message::<events::EvEffectTag>()
            // message window
            .add_message::<events::EvMessageWindowTag>()
            // character sprites
            .add_message::<events::EvCharacterTag>()
            // skip + key config
            .add_message::<events::EvControlTag>()
            // ui
            .add_message::<events::EvUiTag>()
            // misc
            .add_message::<events::EvMiscTag>()
            // host → interpreter
            .add_message::<events::EvHostInput>();

        // 4. Add systems
        //
        // Order:
        //   poll_interpreter  – drains channel, emits EvInterpreterCall + EvTagRouted
        //   dispatch_tags     – reads EvTagRouted, emits typed action events
        //   input systems     – translate Bevy input / EvHostInput → HostEvent
        app.add_systems(
            Update,
            (
                poll_interpreter,
                dispatch_tags.after(poll_interpreter),
                (handle_click_input, handle_timer, handle_ui_inputs).after(poll_interpreter),
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
