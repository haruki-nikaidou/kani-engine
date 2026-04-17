---
title: KAG-Rhai リファレンス
description: KAG シナリオに Rhai 式を埋め込んでスクリプティングするための完全リファレンス。
---

KAG シナリオは `.ks` テキストファイルで、条件分岐・変数操作・動的テキストのために [Rhai](https://rhai.rs/) 式を埋め込めます。このページは KAG タグセットと Rhai 統合レイヤーの公式リファレンスです。

---

## KAG スクリプトの基本

`.ks` ファイルは 1 行ずつ処理されます。各行は以下のいずれかです：

| 行の種類 | 例 | 意味 |
|----------|----|----|
| **テキスト** | `こんにちは！` | メッセージウィンドウに表示 |
| **ラベル** | `*start` | ジャンプ先 |
| **話者ショートハンド** | `#Alice` | 次のテキストブロックの話者名を設定 |
| **ブロックタグ** | `[wait time=500]` | インラインタグ（行の唯一の内容） |
| **ラインタグ** | `@jump target=*start` | `@` は `[…]` の糖衣構文 |
| **コメント** | `; コメントです` | インタープリタに無視される |

### ラベル

```ks
*my_label
```

ラベルは `*` に続く識別子です。`[jump]`・`[call]`・`[link]` などのジャンプ先として使います。

### 話者ショートハンド

```ks
#Alice
このテキストは Alice の発言です。
```

`#Name` ショートハンドは現在の話者名を設定します。名前は次のテキストブロックが出力された後にリセットされます。

---

## 変数スコープ

KAG は 4 つの変数マップを Rhai グローバルオブジェクトとして公開します：

| 名前 | スコープ | 永続性 | 用途 |
|------|----------|--------|------|
| `f`  | ゲームフラグ | セーブデータと一緒に保存 | ストーリーフラグ・カウンター・選択履歴 |
| `sf` | システムフラグ | ゲームセーブとは別に保存 | 設定・グローバルアンロック |
| `tf` | 一時フラグ | 保存されない（ロード時にリセット） | シーン内の一時的な値 |
| `mp` | マクロパラメータ | マクロ呼び出し時に設定 | `[macro]` ボディ内での受け渡し |

Rhai のオブジェクトマップとして読み書きします：

```rhai
f.visited_village = true;
sf.bgm_volume = 0.8;
tf.temp_counter = tf.temp_counter + 1;
```

### 変数のクリア

| タグ | 効果 |
|------|------|
| `[clearvar]` | `f` のすべてのエントリをクリア |
| `[clearsysvar]` | `sf` のすべてのエントリをクリア |

---

## Rhai 式の埋め込み

### `[eval exp=…]` — 文を実行する

Rhai スクリプトを実行しますが、出力は生成しません。

```ks
[eval exp="f.score = f.score + 10;"]
[eval exp="f.name = mp.player_name;"]
```

副作用（変数の変更）はスコープに永続化されます。

### `[emb exp=…]` — 結果をテキストとして埋め込む

式を評価し、現在のメッセージストリームに結果を挿入します。

```ks
あなたのスコアは [emb exp="f.score"] 点です！
```

評価に失敗した場合は空文字列が代入されます（エラーは無視されます）。

### `[trace exp=…]` — デバッグログ

式を評価して結果をデバッグログに出力します。画面には表示されません。

```ks
[trace exp="f.score"]
```

### エンティティ式（`&expr`）

`&` で始まる属性値はすべてランタイムで Rhai 式として評価されます：

```ks
[jump target=&"*" + f.next_scene]
[bg storage=&"bg/" + sf.theme + "/bg01.jpg"]
```

### マクロパラメータ参照（`%key` / `%key|default`）

`[macro]` ボディ内では、`%key` が呼び出し元で渡された値に置換されます。`|` の後にデフォルト値を指定できます：

```ks
[macro name=say_hello]
こんにちは、%name|名無しさん！
[endmacro]

[say_hello name=Alice]
```

---

## 条件分岐

### `[if exp=…]` / `[elsif exp=…]` / `[else]` / `[endif]`

```ks
[if exp="f.score >= 100"]
完璧なスコアです！
[elsif exp="f.score >= 50"]
なかなかです。頑張りましょう！
[else]
次は頑張ってください。
[endif]
```

`exp=` は論理値に評価される Rhai 式です。真偽判定のルール：

- `bool` — そのまま
- `int` — `0` は偽、それ以外は真
- `string` — 空文字列は偽
- その他の型 — ユニット以外は真

### `[ignore exp=…]` / `[endignore]`

`exp=` が真のとき、`[ignore]` と `[endignore]` の間をすべてスキップします。大きなブロックを条件付きでコメントアウトするのに便利です：

```ks
[ignore exp="sf.debug == false"]
[trace exp="f.current_scene"]
[endignore]
```

---

## ナビゲーション

### `[jump storage=… target=…]`

ラベルへ無条件ジャンプします。`storage=` で現在のファイルを切り替え、`target=` でラベル名（`*` プレフィックス込み）を指定します。

```ks
[jump target=*game_over]
[jump storage=scene02.ks target=*start]
```

### `[call storage=… target=…]`

`[jump]` と同様ですが、現在の位置をコールスタックに積むので `[return]` で戻れます。

```ks
[call target=*show_inventory]
; [return] の後、ここから実行が再開される
```

### `[return]`

直近の `[call]` で保存された位置に戻ります。

### `[clearstack]`

コールスタック（マクロ・if スタックも含む）を全破棄します。新しいシーンへのハードジャンプ前に使います。

---

## 選択肢リンク

### `[link]` / `[endlink]`

`[link]` と `[endlink]` の間に 1 つ以上の選択肢ボタンを定義します。各 `[link]` タグが 1 つの選択肢です。

```ks
[link target=*choice_a]
選択肢 A
[link target=*choice_b]
選択肢 B
[endlink]
```

属性：

| 属性 | 型 | 説明 |
|------|----|------|
| `storage=` | string | ジャンプ先シナリオファイル |
| `target=` | string | ジャンプ先ラベル |
| `text=` | string | ボタンラベルテキスト（インラインテキストの代替） |

`storage=` か `target=` のどちらか一方は指定してください。

### `[glink]` — グラフィカルリンクボタン

`[link]` と同じ動作ですが、テキストスパンではなく画像ボタンを想定します。

---

## 表示制御

| タグ | 説明 |
|------|------|
| `[l]` | クリック待ち（行待ち）。 |
| `[p]` | クリック待ち後にメッセージウィンドウをクリア。 |
| `[r]` | 改行を挿入。 |
| `[s]` | イベント（クリック・タイムアウトなど）が発火するまで実行を停止。 |
| `[cm]` | 現在のメッセージレイヤーをクリア。 |
| `[er]` | すべてのレイヤーを消去。 |
| `[ch text=…]` | 全角 1 文字を出力。 |
| `[hch text=…]` | 半角 1 文字を出力。 |

### ノーウェイトモード

| タグ | 説明 |
|------|------|
| `[nowait]` | `[l]` と `[p]` がクリック待ちをしなくなる。 |
| `[endnowait]` | 通常のクリック待ち動作に戻す。 |

---

## タイムドウェイト

### `[wait time=N canskip=…]`

`N` ミリ秒実行を一時停止します。`canskip=true`（デフォルト）の場合はプレイヤーがクリックでスキップできます。

```ks
[wait time=2000]
[wait time=5000 canskip=false]
```

### `[wc time=N]`

最大 `N` ミリ秒クリックを待ちます。クリックするとタイマーが早期終了します。

### 完了待ちタグ

以下のタグは指定した非同期操作の完了を待ちます：

| タグ | 待ち対象 |
|------|---------|
| `[wa]` | すべての非同期処理 |
| `[wm]` | 移動・位置アニメーション |
| `[wt]` | トランジション |
| `[wq]` | クエイク・シェイク・フラッシュエフェクト |
| `[wb]` | BGM フェード |
| `[wf]` | フェードイン・フェードアウト |
| `[wl]` | レイヤーフェード |
| `[ws]` | 効果音 |
| `[wv]` | ボイス |
| `[wp]` | 汎用ポーズ |

すべて `canskip=true/false` と `buf=N` のオプション属性を受け付けます。

### `[ct]`

進行中のすべての非同期処理を即座にキャンセルします。

---

## イベントハンドラ

### `[click storage=… target=… exp=…]`

`[s]` で停止中にプレイヤーが次にクリックしたときに実行されるジャンプ（または式）を登録します。`storage=`・`target=`・`exp=` のうち少なくとも 1 つを指定してください。

```ks
[click target=*resume]
[s]
```

### `[wheel storage=… target=… exp=…]`

`[click]` と同じですが、マウスホイールスクロールで発火します。

### `[timeout time=N storage=… target=…]`

`[s]` 停止中に `N` ミリ秒後に自動発火するジャンプを登録します。

```ks
[timeout time=3000 target=*auto_continue]
[s]
```

### キャンセル

| タグ | キャンセル対象 |
|------|--------------|
| `[cclick]` | アクティブな `[click]` ハンドラ |
| `[ctimeout]` | アクティブな `[timeout]` ハンドラ |
| `[cwheel]` | アクティブな `[wheel]` ハンドラ |
| `[waitclick]` | 次のクリックまで待機してから続行 |

---

## 画像・レイヤーシステム

### `[bg storage=… time=… method=…]`

背景画像を設定します。

| 属性 | 型 | 必須 | 説明 |
|------|----|------|------|
| `storage=` | string | **はい** | 画像ファイルのパス |
| `time=` | ms | いいえ | トランジション時間 |
| `method=` | string | いいえ | トランジション方法名 |

```ks
[bg storage=bg/forest.jpg time=1000 method=crossfade]
```

### `[image storage=… layer=… x=… y=… visible=…]`

指定レイヤーに画像を表示します。

| 属性 | 型 | 必須 | 説明 |
|------|----|------|------|
| `storage=` | string | **はい** | 画像ファイルのパス |
| `layer=` | string | いいえ | レイヤー識別子 |
| `x=` | float | いいえ | 水平位置 |
| `y=` | float | いいえ | 垂直位置 |
| `visible=` | bool | いいえ | 初期表示状態 |

### `[layopt layer=… visible=… opacity=…]`

既存レイヤーのオプションを変更します。

| 属性 | 型 | 必須 | 説明 |
|------|----|------|------|
| `layer=` | string | **はい** | レイヤー識別子 |
| `visible=` | bool | いいえ | 表示・非表示 |
| `opacity=` | float | いいえ | 透明度（0.0–1.0） |

### `[free layer=…]`

レイヤーを完全に削除します。

### `[position layer=… x=… y=…]`

レイヤーを新しい位置に移動します。

---

## オーディオ

### `[bgm storage=… loop=… volume=… fadetime=…]`

BGM を再生します。

| 属性 | 型 | 必須 | 説明 |
|------|----|------|------|
| `storage=` | string | **はい** | 音声ファイルのパス |
| `loop=` | bool | いいえ | ループ再生（デフォルト: true） |
| `volume=` | float | いいえ | 音量 0.0–1.0 |
| `fadetime=` | ms | いいえ | フェードイン時間 |

### `[stopbgm fadetime=…]`

BGM を停止します。フェードアウトも指定可能。

### `[fadebgm time=… volume=…]`

BGM の音量を滑らかに変更します。

### `[se storage=… buf=… volume=… loop=…]`

効果音を再生します。エイリアス：`[playSe]`。

| 属性 | 型 | 必須 | 説明 |
|------|----|------|------|
| `storage=` | string | **はい** | 音声ファイルのパス |
| `buf=` | int | いいえ | バッファスロット（SE の多重再生用） |
| `volume=` | float | いいえ | 音量 0.0–1.0 |
| `loop=` | bool | いいえ | ループ再生 |

### `[stopse buf=…]`

効果音バッファを停止します。

### `[vo storage=… buf=…]`

ボイスを再生します。エイリアス：`[voice]`。

---

## トランジション

### `[trans method=… time=… rule=…]`

ビジュアルシーントランジションを適用します。

| 属性 | 型 | 説明 |
|------|----|------|
| `method=` | string | トランジション種類（例：`crossfade`・`wipe`） |
| `time=` | ms | 時間 |
| `rule=` | string | カスタムトランジション用ルール画像 |

### `[fadein time=… color=…]`

指定色からフェードインします。

### `[fadeout time=… color=…]`

指定色へフェードアウトします。

### `[movetrans layer=… time=… x=… y=…]`

レイヤーを `time` ms かけて `(x, y)` に移動するトランジション。

---

## エフェクト

### `[quake time=… hmax=… vmax=…]`

画面クエイクエフェクト。`hmax`/`vmax` は最大ピクセル変位。

### `[shake time=… amount=… axis=…]`

画面シェイク。`axis=` は `"h"`（水平）・`"v"`（垂直）、または省略で両方。

### `[flash time=… color=…]`

指定色で画面をフラッシュします。

---

## メッセージウィンドウ

### `[msgwnd visible=… layer=…]`

メッセージウィンドウを表示・非表示にします。

### `[wndctrl x=… y=… width=… height=…]`

メッセージウィンドウの位置とサイズを設定します。

### フォント制御

| タグ | 説明 |
|------|------|
| `[font face=… size=… bold=… italic=…]` | 複数のフォントプロパティをまとめて設定 |
| `[size value=…]` | フォントサイズ（ポイント） |
| `[bold value=…]` | 太字のオン・オフ |
| `[italic value=…]` | イタリックのオン・オフ |
| `[resetfont]` | すべてのフォントプロパティをデフォルトにリセット |
| `[ruby text=…]` | 次の文字にルビ（フリガナ）を設定 |

### 禁則処理

| タグ | 説明 |
|------|------|
| `[nowrap]` | 折り返しを無効にする |
| `[endnowrap]` | 折り返しを有効に戻す |

---

## 表示速度

| タグ | 説明 |
|------|------|
| `[delay speed=N]` | 1 文字あたりの表示ディレイ（ms）を設定 |
| `[configdelay speed=N]` | コンフィグレイヤーの表示ディレイを設定 |
| `[resetdelay]` | ディレイをシステムデフォルトにリセット |
| `[nowait]` | 1 文字ごとのディレイを完全に無効化 |
| `[endnowait]` | ディレイを有効に戻す |
| `[autowc time=N]` | 1 文字表示後の追加待機時間。`time=` 省略で無効化 |
| `[resetwait]` | オートウェイトのベースラインタイマーをリセット |

---

## バックログ

### `[pushlog text=… join=…]`

バックログに文字列を手動で追加します。

| 属性 | 型 | 説明 |
|------|----|------|
| `text=` | string | 記録するテキスト |
| `join=` | bool | 直前のバックログエントリに追記する場合 true |

### `[nolog]` / `[endnolog]`

`DisplayText` イベントのバックログ自動記録を無効・有効にします。

---

## プレイヤー入力

### `[input name=… prompt=… title=…]`

テキスト入力ダイアログを開きます。入力値は `f[name]` に格納されます。

| 属性 | 型 | 説明 |
|------|----|------|
| `name=` | string | 結果を書き込む `f` のキー |
| `prompt=` | string | プレースホルダー・ヒントテキスト |
| `title=` | string | ダイアログタイトル |

### `[waittrig name=…]`

ホストが指定名のトリガーイベントを発火するまで実行を停止します。

---

## キャラクタースプライト

### `[chara name=… id=… storage=… slot=… x=… y=…]`

キャラクタースプライトを表示します。`name=` か `id=` のどちらか一方は必須です。

| 属性 | 型 | 説明 |
|------|----|------|
| `name=` / `id=` | string | キャラクター識別子 |
| `storage=` | string | スプライト画像パス |
| `slot=` | string | 表示スロット |
| `x=` / `y=` | float | 位置 |

### `[chara_hide name=… id=… slot=…]`

キャラクタースプライトを非表示にします（ロードしたまま保持）。

### `[chara_free name=… id=… slot=…]`

キャラクタースプライトをアンロードします。

### `[chara_mod name=… id=… face=… pose=… storage=…]`

表示中のキャラクタースプライトの表情・ポーズを変更します。

### `[chara_ptext name=…]`

`ptext` ネームボックスに表示するキャラクター名を設定します。

---

## マクロシステム

### `[macro name=…]` / `[endmacro]`

再利用可能なスクリプトブロックを定義します。マクロ本体はその名前で呼び出されるたびに実行されます。

```ks
[macro name=fade_to_black]
[fadeout time=500 color=0x000000]
[wf]
[endmacro]

; 呼び出し：
[fade_to_black]
```

マクロ本体内では、呼び出し元で渡されたパラメータが `%key` または `mp` Rhai マップで利用できます。

### `[erasemacro name=…]`

ランタイムでマクロ定義を削除します。

---

## その他

### `[clickskip enabled=…]`

トランジション・アニメーション中のクリックスキップモードを有効・無効にします。

### `[clearvar]` / `[clearsysvar]` / `[clearstack]`

| タグ | クリア対象 |
|------|-----------|
| `[clearvar]` | `f` のすべてのゲームフラグ |
| `[clearsysvar]` | `sf` のすべてのシステムフラグ |
| `[clearstack]` | コール・if・マクロスタック |

---

## クイックリファレンス：全タグ一覧

| タグ | 必須属性 | 任意属性 |
|------|---------|---------|
| `[if]` | `exp=` | — |
| `[elsif]` | `exp=` | — |
| `[else]` | — | — |
| `[endif]` | — | — |
| `[ignore]` | `exp=` | — |
| `[endignore]` | — | — |
| `[jump]` | `storage=` または `target=` | 両方 |
| `[call]` | `storage=` または `target=` | 両方 |
| `[return]` | — | — |
| `[link]` | `storage=` または `target=` | `text=` |
| `[endlink]` | — | — |
| `[glink]` | `storage=` または `target=` | `text=` |
| `[eval]` | `exp=` | — |
| `[emb]` | `exp=` | — |
| `[trace]` | `exp=` | — |
| `[l]` | — | — |
| `[p]` | — | — |
| `[r]` | — | — |
| `[s]` | — | — |
| `[cm]` | — | — |
| `[er]` | — | — |
| `[ch]` | `text=` | — |
| `[hch]` | `text=` | — |
| `[wait]` | `time=` | `canskip=` |
| `[wc]` | `time=` | — |
| `[ct]` | — | — |
| `[timeout]` | `time=` | `storage=`・`target=` |
| `[waitclick]` | — | — |
| `[click]` | `storage=`・`target=`・`exp=` のいずれか | 3 つすべて |
| `[wheel]` | `storage=`・`target=`・`exp=` のいずれか | 3 つすべて |
| `[cclick]` | — | — |
| `[ctimeout]` | — | — |
| `[cwheel]` | — | — |
| `[nolog]` | — | — |
| `[endnolog]` | — | — |
| `[nowait]` | — | — |
| `[endnowait]` | — | — |
| `[delay]` | `speed=` | — |
| `[configdelay]` | `speed=` | — |
| `[resetdelay]` | — | — |
| `[autowc]` | — | `time=` |
| `[resetwait]` | — | — |
| `[pushlog]` | `text=` | `join=` |
| `[input]` | `name=` | `prompt=`・`title=` |
| `[waittrig]` | `name=` | — |
| `[macro]` | — | `name=` |
| `[erasemacro]` | `name=` | — |
| `[endmacro]` | — | — |
| `[clearvar]` | — | — |
| `[clearsysvar]` | — | — |
| `[clearstack]` | — | — |
| `[clickskip]` | — | `enabled=` |
| `[chara_ptext]` | `name=` | — |
| `[bg]` | `storage=` | `time=`・`method=` |
| `[image]` | `storage=` | `layer=`・`x=`・`y=`・`visible=` |
| `[layopt]` | `layer=` | `visible=`・`opacity=` |
| `[free]` | `layer=` | — |
| `[position]` | `layer=` | `x=`・`y=` |
| `[bgm]` | `storage=` | `loop=`・`volume=`・`fadetime=` |
| `[stopbgm]` | — | `fadetime=` |
| `[se]` / `[playSe]` | `storage=` | `buf=`・`volume=`・`loop=` |
| `[stopse]` | — | `buf=` |
| `[vo]` / `[voice]` | `storage=` | `buf=` |
| `[fadebgm]` | — | `time=`・`volume=` |
| `[trans]` | — | `method=`・`time=`・`rule=` |
| `[fadein]` | — | `time=`・`color=` |
| `[fadeout]` | — | `time=`・`color=` |
| `[movetrans]` | — | `layer=`・`time=`・`x=`・`y=` |
| `[quake]` | — | `time=`・`hmax=`・`vmax=` |
| `[shake]` | — | `time=`・`amount=`・`axis=` |
| `[flash]` | — | `time=`・`color=` |
| `[msgwnd]` | — | `visible=`・`layer=` |
| `[wndctrl]` | — | `x=`・`y=`・`width=`・`height=` |
| `[resetfont]` | — | — |
| `[font]` | — | `face=`・`size=`・`bold=`・`italic=` |
| `[size]` | — | `value=` |
| `[bold]` | — | `value=` |
| `[italic]` | — | `value=` |
| `[ruby]` | — | `text=` |
| `[nowrap]` | — | — |
| `[endnowrap]` | — | — |
| `[chara]` | `name=` または `id=` | `storage=`・`slot=`・`x=`・`y=` |
| `[chara_hide]` | `name=` または `id=` | `slot=` |
| `[chara_free]` | `name=` または `id=` | `slot=` |
| `[chara_mod]` | `name=` または `id=` | `face=`・`pose=`・`storage=` |
| `[wa]`〜`[wp]` | — | `canskip=`・`buf=` |

