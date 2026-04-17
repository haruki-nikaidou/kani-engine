---
title: はじめかた
description: Kani をインストールし、最初の KAG シナリオを書いて Bevy アプリで実行する手順。
---

import { Steps, Aside, Tabs, TabItem } from '@astrojs/starlight/components';

**Kani Game Engine** へようこそ。[Bevy](https://bevyengine.org/) 上に構築され、KAG スクリプトと [Rhai](https://rhai.rs/) 式で動くビジュアルノベルランタイムです。

## 前提条件

- Rust **1.80** 以降（[rustup](https://rustup.rs/) でインストール）
- Cargo（Rust に同梱）
- Bevy 対応プラットフォーム（Windows・macOS・Linux）

## インストール

<Steps>

1. **新しい Bevy プロジェクトを作成する**

   ```sh
   cargo new my-vn
   cd my-vn
   ```

2. **`kani-runtime` を依存関係に追加する**

   ```sh
   cargo add kani-runtime
   cargo add bevy
   ```

   または `Cargo.toml` を直接編集する：

   ```toml
   [dependencies]
   bevy = "0.15"
   kani-runtime = "0.1"
   ```

3. **エントリシナリオを書く**

   `assets/scenario/first.ks` を作成する：

   ```ks
   *start
   #ナレーター
   ビジュアルノベルへようこそ！
   @l
   ここから冒険が始まります。
   @l
   @jump target=*start
   ```

4. **プラグインを組み込む**

   `src/main.rs` の内容を以下に置き換える：

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

5. **実行する**

   ```sh
   cargo run
   ```

</Steps>

## プロジェクト構成

```
my-vn/
├── Cargo.toml
├── src/
│   └── main.rs
└── assets/
    ├── scenario/
    │   └── first.ks      ← エントリスクリプト
    ├── bg/               ← 背景画像
    ├── chara/            ← キャラクタースプライト
    ├── bgm/              ← BGM
    └── se/               ← 効果音
```

## 次のステップ

- **[KAG-Rhai リファレンス](/ja/reference/kag-rhai/)** ですべてのタグと式を確認する。
- **[LSP サーバーのセットアップ](/ja/guides/setup-lsp/)** でエディタのシンタックスハイライトやホバードキュメントを有効にする。

<Aside type="tip">
アセットパスはすべて `AssetBackend::FileSystem` に渡した `base` ディレクトリからの相対パスです。リリース時は `AssetBackend::Pak` に切り替えて、すべてを単一の `.pak` ファイルにまとめることができます。
</Aside>

