# kani-runtime

`kani-runtime` bridges `kag-interpreter` and a Bevy ECS host app.

It is responsible for:

- starting the `KagInterpreter` on a dedicated thread (required because the interpreter is `!Send`),
- exposing interpreter output to Bevy systems via messages,
- forwarding host input back to the interpreter,
- loading scenario text from either filesystem assets or a `.pak` archive,
- routing passthrough KAG tags (image/audio/transition/effect/message/chara) into typed host messages.

## Quick start

```rust,no_run
use bevy_app::App;
use kani_runtime::{asset::AssetBackend, KaniRuntimePlugin};

fn main() {
    let mut app = App::new();

    app.add_plugins(KaniRuntimePlugin {
        asset_backend: AssetBackend::file_system("assets"),
        entry_script: "scenario/first.ks".to_string(),
    });

    // Add your own systems that read kani-runtime messages and render UI/audio.
    app.run();
}
```

For `.pak` mode:

```rust,no_run
# use anyhow::Result;
# use bevy_app::App;
# use kani_runtime::{asset::AssetBackend, KaniRuntimePlugin};
# fn demo() -> Result<()> {
let backend = AssetBackend::pak("game.pak")?;
let mut app = App::new();
app.add_plugins(KaniRuntimePlugin {
    asset_backend: backend,
    entry_script: "scenario/first.ks".to_string(),
});
# Ok(())
# }
```

## Runtime flow

1. `KaniRuntimePlugin` registers assets and inserts the `InterpreterBridge` resource.
2. `poll_interpreter` drains `KagEvent`s and emits Bevy messages.
3. `input_bridge` observes bridge wait-state and sends `HostEvent`s (`Clicked`, `TimerElapsed`, etc.).
4. Your game systems consume emitted messages to render text, choices, visuals, audio, and transitions.

## Extending tag handling

`KagEvent::Tag` is categorized by `systems::tags::route_tag`.

- Known categories emit typed messages (`EvImageTag`, `EvAudioTag`, ...).
- Unknown tags emit `EvUnknownTag` so game-specific systems can implement custom behavior.
