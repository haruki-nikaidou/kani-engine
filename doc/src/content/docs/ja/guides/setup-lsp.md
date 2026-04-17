---
title: LSP のセットアップ
description: kag-lsp 言語サーバーをインストールして VS Code・Neovim などのエディタで KAG のシンタックスサポートを有効にする。
---

import { Steps, Aside, Tabs, TabItem } from '@astrojs/starlight/components';

`kag-lsp` は KAG スクリプト（`.ks` ファイル）向けの Language Server Protocol 実装です。以下の機能を提供します：

- シンタックスハイライトとエラー診断
- すべての組み込みタグと属性のホバードキュメント
- ラベル・マクロ・ファイルの定義へのジャンプ
- タグ名と属性キーのオートコンプリート
- ラベルとマクロのすべての参照を検索

## ソースからビルド

`kag-lsp` はこのリポジトリの `kag-lsp` クレートにあります。

<Steps>

1. **リポジトリをクローンする**

   ```sh
   git clone https://github.com/haruki-nikaidou/kani-engine.git
   cd kani-engine
   ```

2. **リリースモードでビルドする**

   ```sh
   cargo build -p kag-lsp --release
   ```

   バイナリは `target/release/kag-lsp` に生成されます。

3. **`PATH` に追加する**

   ```sh
   # Linux / macOS
   cp target/release/kag-lsp ~/.local/bin/

   # またはシェルの設定ファイルでリリースディレクトリを PATH に追加
   export PATH="$PATH:/path/to/kani-engine/target/release"
   ```

</Steps>

## エディタ設定

<Tabs>
<TabItem label="VS Code">

1. **[KAG Script](https://marketplace.visualstudio.com/search?term=kag-script)** 拡張機能（または **vscode-glspc** などの汎用 LSP クライアント）をインストールする。

2. `settings.json` に以下を追加する：

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

Neovim の設定ファイルに追加する：

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

ファイルタイプ検出ルールも追加する：

```vim
autocmd BufRead,BufNewFile *.ks set filetype=kag
```

</TabItem>
<TabItem label="Helix">

`~/.config/helix/languages.toml` に追加する：

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
<TabItem label="その他のエディタ">

LSP プロトコルをサポートするエディタであれば `kag-lsp` を利用できます。以下を設定してください：

- `kag-lsp` をサーバープロセスとして起動（**stdio** で通信）
- `.ks` ファイル拡張子を `kag` 言語 ID に関連付ける

</TabItem>
</Tabs>

## 機能概要

### ホバードキュメント

タグ名にカーソルを合わせると、`tag_defs` モジュールから取得した説明・必須属性・任意属性・短いサンプルが表示されます。

### 定義へのジャンプ

`target=*label` の値を `Ctrl`+クリック（または各エディタの操作）すると、ファイルをまたいでもラベルの定義位置にジャンプできます。

### 診断

LSP はリアルタイムで属性の完全性を検証します：

| 深刻度 | 条件 |
|--------|------|
| **エラー** | 必須属性（例：`[bg]` の `storage=`）が不足している |
| **警告** | 推奨属性が不足しており、タグが効果を持たない |

### コンプリート

`[` の内側またはタグ内でスペースを押した後にコンプリートキーを押すと以下が表示されます：

- すべての既知タグ名
- 現在のタグに有効な属性キー
- ブール値の候補（`true` / `false`）

<Aside type="note">
サーバーはワークスペースルートごとに起動します。クロスファイルのラベル参照を正しく解決するために、`Cargo.toml` があるレベルでプロジェクトを開いてください。
</Aside>

