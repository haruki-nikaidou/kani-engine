---
title: Setup LSP
description: Install and configure the kag-lsp language server for KAG syntax support in VS Code, Neovim, and other editors.
---

import { Steps, Aside, Tabs, TabItem } from '@astrojs/starlight/components';

`kag-lsp` is a Language Server Protocol implementation for KAG scripts (`.ks` files). It provides:

- Syntax highlighting and error diagnostics
- Hover documentation for every built-in tag and attribute
- Go-to-definition for labels, macros, and files
- Auto-completion for tag names and attribute keys
- Find-all-references for labels and macros

## Build from source

`kag-lsp` lives in the `kag-lsp` crate of this repository.

<Steps>

1. **Clone the repository**

   ```sh
   git clone https://github.com/haruki-nikaidou/kani-engine.git
   cd kani-engine
   ```

2. **Build in release mode**

   ```sh
   cargo build -p kag-lsp --release
   ```

   The binary is placed at `target/release/kag-lsp`.

3. **Put it on your `PATH`**

   ```sh
   # Linux / macOS
   cp target/release/kag-lsp ~/.local/bin/

   # Or add the release dir to PATH in your shell profile
   export PATH="$PATH:/path/to/kani-engine/target/release"
   ```

</Steps>

## Editor setup

<Tabs>
<TabItem label="VS Code">

1. Install the **[KAG Script](https://marketplace.visualstudio.com/search?term=kag-script)** extension (or any generic LSP client such as **vscode-glspc**).

2. Add this to your `settings.json`:

   ```json
   {
     "languageServerExample.serverPath": "/path/to/kag-lsp"
   }
   ```

   Or, with a generic client:

   ```json
   {
     "genericLSP.servers": [
       {
         "name": "KAG",
         "language": "kag",
         "extensions": [".ks"],
         "command": ["kag-lsp"]
       }
     ]
   }
   ```

</TabItem>
<TabItem label="Neovim (nvim-lspconfig)">

Add to your Neovim config:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.kag_lsp then
  configs.kag_lsp = {
    default_config = {
      cmd = { 'kag-lsp' },
      filetypes = { 'kag' },
      root_dir = lspconfig.util.root_pattern('Cargo.toml', '.git'),
      settings = {},
    },
  }
end

lspconfig.kag_lsp.setup {}
```

Then add a filetype detection rule:

```vim
autocmd BufRead,BufNewFile *.ks set filetype=kag
```

</TabItem>
<TabItem label="Helix">

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "kag"
scope = "source.kag"
file-types = ["ks"]
language-servers = ["kag-lsp"]

[language-server.kag-lsp]
command = "kag-lsp"
```

</TabItem>
<TabItem label="Other editors">

Any editor that supports the LSP protocol can use `kag-lsp`. Configure your editor to:

- Launch `kag-lsp` as a server process (communicates over **stdio**)
- Associate the `.ks` file extension with the `kag` language ID

</TabItem>
</Tabs>

## Features overview

### Hover documentation

Hovering over a tag name displays its description, required and optional attributes, and a short example — sourced directly from the `tag_defs` module.

### Go-to-definition

`Ctrl`+click (or your editor's equivalent) on a `target=*label` value jumps to the label's definition, even across files.

### Diagnostics

The LSP validates attribute completeness in real time:

| Severity | Condition |
|----------|-----------|
| **Error** | A required attribute (e.g. `storage=` on `[bg]`) is missing |
| **Warning** | A recommended attribute is absent and the tag will have no effect |

### Completion

Pressing the completion key inside `[` or after a space inside a tag gives you:

- All known tag names
- All valid attribute keys for the current tag
- Boolean value suggestions (`true` / `false`)

<Aside type="note">
The server is started fresh for each workspace root. Open your project at the `Cargo.toml` level so the LSP can resolve cross-file label references correctly.
</Aside>

