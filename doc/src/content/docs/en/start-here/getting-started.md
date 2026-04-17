---
title: Getting Started
description: Install Kani, write your first KAG scenario, and run it inside a Bevy app.
---

import { Steps, Aside, Tabs, TabItem } from '@astrojs/starlight/components';

Welcome to **Kani Game Engine** — a visual-novel runtime for Rust built on top of [Bevy](https://bevyengine.org/) and driven by KAG scripts with embedded [Rhai](https://rhai.rs/) expressions.

## Prerequisites

- Rust **1.80** or newer (install via [rustup](https://rustup.rs/))
- Cargo (bundled with Rust)
- A Bevy-compatible platform (Windows, macOS, or Linux)

## Installation

<Steps>

1. **Create a new Bevy project**

   ```sh
   cargo new my-vn
   cd my-vn
   ```

2. **Add `kani-runtime` to your dependencies**

   ```sh
   cargo add kani-runtime
   cargo add bevy
   ```

   Or edit `Cargo.toml` manually:

   ```toml
   [dependencies]
   bevy = "0.15"
   kani-runtime = "0.1"
   ```

3. **Write your entry scenario**

   Create `assets/scenario/first.ks`:

   ```ks
   *start
   #Narrator
   Welcome to my visual novel!
   @l
   The adventure begins here.
   @l
   @jump target=*start
   ```

4. **Wire up the plugin**

   Replace the contents of `src/main.rs`:

   ```rust
   use bevy::prelude::*;
   use kani_runtime::{AssetBackend, KaniRuntimePlugin};
   use std::path::PathBuf;

   fn main() {
       App::new()
           .add_plugins(DefaultPlugins)
           .add_plugins(KaniRuntimePlugin {
               asset_backend: AssetBackend::FileSystem {
                   base: PathBuf::from("assets"),
               },
               entry_script: "scenario/first.ks".into(),
           })
           .run();
   }
   ```

5. **Run**

   ```sh
   cargo run
   ```

</Steps>

## Project layout

```
my-vn/
├── Cargo.toml
├── src/
│   └── main.rs
└── assets/
    ├── scenario/
    │   └── first.ks      ← entry script
    ├── bg/               ← background images
    ├── chara/            ← character sprites
    ├── bgm/              ← background music
    └── se/               ← sound effects
```

## Next steps

- Read the **[KAG-Rhai Reference](/en/reference/kag-rhai/)** for every tag and expression.
- Set up your editor with the **[LSP server](/en/guides/setup-lsp/)** for syntax highlighting and hover docs.

<Aside type="tip">
All asset paths are relative to the `base` directory you pass to `AssetBackend::FileSystem`. When shipping, you can switch to `AssetBackend::Pak` to bundle everything into a single `.pak` file.
</Aside>

