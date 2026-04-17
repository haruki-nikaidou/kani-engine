---
title: KAG-Rhai Reference
description: Complete reference for scripting KAG scenarios with embedded Rhai expressions.
---

KAG scenarios are plain text files (`.ks`) that can embed [Rhai](https://rhai.rs/) expressions for conditional logic, variable mutation, and dynamic text. This page is the authoritative reference for the KAG tag set and the Rhai integration layer.

---

## KAG Script Basics

A `.ks` file is processed line-by-line. Each line is one of:

| Line type | Example | Meaning |
|-----------|---------|---------|
| **Text** | `Hello, world!` | Printed to the message window |
| **Label** | `*start` | Jump target |
| **Speaker shorthand** | `#Alice` | Sets the speaker name for the next text block |
| **Block tag** | `[wait time=500]` | Inline tag (must be the only content on the line) |
| **Line tag** | `@jump target=*start` | `@` is syntactic sugar for `[…]` |
| **Comment** | `; this is a comment` | Ignored by the interpreter |

### Labels

```ks
*my_label
```

Labels are `*` followed by an identifier. Labels are used as jump targets for `[jump]`, `[call]`, `[link]`, etc.

### Speaker shorthands

```ks
#Alice
This text is attributed to Alice.
```

The `#Name` shorthand sets the current speaker name. The name is reset after the next text block is emitted.

---

## Variable Scopes

KAG exposes four variable maps as Rhai global objects:

| Name | Scope | Persistence | Use case |
|------|-------|-------------|----------|
| `f`  | Game flags | Saved with the game | Story flags, counters, choices made |
| `sf` | System flags | Saved separately from game saves | Config, global unlocks |
| `tf` | Transient flags | Not saved — reset on load | Temporary per-scene values |
| `mp` | Macro parameters | Set at macro call site | Pass-through inside `[macro]` bodies |

Access and mutate them like Rhai object maps:

```rhai
f.visited_village = true;
sf.bgm_volume = 0.8;
tf.temp_counter = tf.temp_counter + 1;
```

### Clearing variables

| Tag | Effect |
|-----|--------|
| `[clearvar]` | Clears all `f` entries |
| `[clearsysvar]` | Clears all `sf` entries |

---

## Rhai Expression Embedding

### `[eval exp=…]` — Execute a statement

Runs a Rhai script without producing any output.

```ks
[eval exp="f.score = f.score + 10;"]
[eval exp="f.name = mp.player_name;"]
```

Side-effects (variable mutations) are persisted into the scope.

### `[emb exp=…]` — Embed result as text

Evaluates the expression and injects the result into the current message stream.

```ks
Your score is [emb exp="f.score"] points!
```

If evaluation fails, an empty string is substituted (errors are swallowed).

### `[trace exp=…]` — Debug logging

Evaluates the expression and writes the result to the debug log. No visible output.

```ks
[trace exp="f.score"]
```

### Entity expressions (`&expr`)

Any attribute value prefixed with `&` is evaluated as a Rhai expression at runtime:

```ks
[jump target=&"*" + f.next_scene]
[bg storage=&"bg/" + sf.theme + "/bg01.jpg"]
```

### Macro parameter references (`%key` / `%key|default`)

Inside a `[macro]` body, `%key` is substituted with the value passed at the call site. An optional default follows `|`:

```ks
[macro name=say_hello]
Hello, %name|stranger!
[endmacro]

[say_hello name=Alice]
```

---

## Conditional Branching

### `[if exp=…]` / `[elsif exp=…]` / `[else]` / `[endif]`

```ks
[if exp="f.score >= 100"]
You achieved a perfect score!
[elsif exp="f.score >= 50"]
Not bad — keep going!
[else]
Better luck next time.
[endif]
```

`exp=` is a Rhai expression evaluated to a boolean. Truthy rules:

- `bool` — direct value
- `int` — `0` is false, anything else true
- `string` — empty string is false
- all other types — non-unit is true

### `[ignore exp=…]` / `[endignore]`

Skips everything between `[ignore]` and `[endignore]` when `exp=` is truthy. Use this to comment out large blocks conditionally:

```ks
[ignore exp="sf.debug == false"]
[trace exp="f.current_scene"]
[endignore]
```

---

## Navigation

### `[jump storage=… target=…]`

Unconditionally jump to a label. `storage=` changes the current file; `target=` names the label (include the `*` prefix).

```ks
[jump target=*game_over]
[jump storage=scene02.ks target=*start]
```

### `[call storage=… target=…]`

Like `[jump]`, but pushes the current position onto the call stack so `[return]` can come back.

```ks
[call target=*show_inventory]
; execution continues here after [return]
```

### `[return]`

Return to the position saved by the most recent `[call]`.

### `[clearstack]`

Discard the entire call stack (and macro/if stacks). Useful before a hard jump to a new scene.

---

## Choice Links

### `[link]` / `[endlink]`

Accumulate one or more choice buttons between `[link]` and `[endlink]`. Each `[link]` tag defines one option.

```ks
[link target=*choice_a]
Option A
[link target=*choice_b]
Option B
[endlink]
```

Attributes:

| Attribute | Type | Description |
|-----------|------|-------------|
| `storage=` | string | Target scenario file |
| `target=` | string | Target label |
| `text=` | string | Button label text (alternative to inline text) |

At least one of `storage=` or `target=` should be provided.

### `[glink]` — Graphical link button

Same semantics as `[link]` but intended for an image-based button rather than a text span.

---

## Display Control

| Tag | Description |
|-----|-------------|
| `[l]` | Wait for a click (line wait). |
| `[p]` | Wait for a click then clear the message window. |
| `[r]` | Insert a line break. |
| `[s]` | Halt execution until an event fires (click, timeout, …). |
| `[cm]` | Clear the current message layer. |
| `[er]` | Erase all layers. |
| `[ch text=…]` | Output a single full-width character. |
| `[hch text=…]` | Output a single half-width character. |

### Nowait mode

| Tag | Description |
|-----|-------------|
| `[nowait]` | `[l]` and `[p]` no longer block for a click. |
| `[endnowait]` | Restore normal click-wait behaviour. |

---

## Timed Waits

### `[wait time=N canskip=…]`

Pause execution for `N` milliseconds. If `canskip=true` (default), the player can click to skip.

```ks
[wait time=2000]
[wait time=5000 canskip=false]
```

### `[wc time=N]`

Wait up to `N` ms for a click. Click cancels the timer early.

### Wait-for-completion tags

The following tags wait for the named async operation to finish:

| Tag | Waits for… |
|-----|-----------|
| `[wa]` | All async operations |
| `[wm]` | Move/position animation |
| `[wt]` | Transition |
| `[wq]` | Quake/shake/flash effect |
| `[wb]` | BGM fade |
| `[wf]` | Fadein/fadeout |
| `[wl]` | Layer fade |
| `[ws]` | Sound effect |
| `[wv]` | Voice |
| `[wp]` | Pause (generic) |

All accept optional `canskip=true/false` and `buf=N` attributes.

### `[ct]`

Cancel all in-progress asynchronous operations immediately.

---

## Event Handlers

### `[click storage=… target=… exp=…]`

Register a jump (or expression) to execute the next time the player clicks while execution is halted at `[s]`. Provide at least one of `storage=`/`target=`/`exp=`.

```ks
[click target=*resume]
[s]
```

### `[wheel storage=… target=… exp=…]`

Same as `[click]` but fires on mouse-wheel scroll.

### `[timeout time=N storage=… target=…]`

Register a jump that fires automatically after `N` milliseconds while at `[s]`.

```ks
[timeout time=3000 target=*auto_continue]
[s]
```

### Cancellation

| Tag | Cancels |
|-----|---------|
| `[cclick]` | Active `[click]` handler |
| `[ctimeout]` | Active `[timeout]` handler |
| `[cwheel]` | Active `[wheel]` handler |
| `[waitclick]` | Wait until next click, then continue |

---

## Image / Layer System

### `[bg storage=… time=… method=…]`

Set the background image.

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `storage=` | string | **Yes** | Path to image file |
| `time=` | ms | No | Transition duration |
| `method=` | string | No | Transition method name |

```ks
[bg storage=bg/forest.jpg time=1000 method=crossfade]
```

### `[image storage=… layer=… x=… y=… visible=…]`

Display an image on a named layer.

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `storage=` | string | **Yes** | Path to image file |
| `layer=` | string | No | Layer identifier |
| `x=` | float | No | Horizontal position |
| `y=` | float | No | Vertical position |
| `visible=` | bool | No | Initial visibility |

### `[layopt layer=… visible=… opacity=…]`

Change options on an existing layer.

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `layer=` | string | **Yes** | Layer identifier |
| `visible=` | bool | No | Show / hide |
| `opacity=` | float | No | Transparency (0.0–1.0) |

### `[free layer=…]`

Remove a layer entirely.

### `[position layer=… x=… y=…]`

Move a layer to a new position.

---

## Audio

### `[bgm storage=… loop=… volume=… fadetime=…]`

Start background music.

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `storage=` | string | **Yes** | Path to audio file |
| `loop=` | bool | No | Loop playback (default: true) |
| `volume=` | float | No | Volume 0.0–1.0 |
| `fadetime=` | ms | No | Fade-in duration |

### `[stopbgm fadetime=…]`

Stop BGM, optionally with a fade-out.

### `[fadebgm time=… volume=…]`

Smoothly change BGM volume.

### `[se storage=… buf=… volume=… loop=…]`

Play a sound effect. Alias: `[playSe]`.

| Attribute | Type | Required | Description |
|-----------|------|----------|-------------|
| `storage=` | string | **Yes** | Path to audio file |
| `buf=` | int | No | Buffer slot (for overlapping SEs) |
| `volume=` | float | No | Volume 0.0–1.0 |
| `loop=` | bool | No | Loop playback |

### `[stopse buf=…]`

Stop a sound effect buffer.

### `[vo storage=… buf=…]`

Play a voice clip. Alias: `[voice]`.

---

## Transitions

### `[trans method=… time=… rule=…]`

Apply a visual scene transition.

| Attribute | Type | Description |
|-----------|------|-------------|
| `method=` | string | Transition type (e.g. `crossfade`, `wipe`) |
| `time=` | ms | Duration |
| `rule=` | string | Rule image for custom transitions |

### `[fadein time=… color=…]`

Fade in from a solid colour.

### `[fadeout time=… color=…]`

Fade out to a solid colour.

### `[movetrans layer=… time=… x=… y=…]`

Move a layer to `(x, y)` over `time` ms as a transition.

---

## Effects

### `[quake time=… hmax=… vmax=…]`

Screen quake effect. `hmax`/`vmax` are maximum pixel displacement.

### `[shake time=… amount=… axis=…]`

Screen shake. `axis=` is `"h"` (horizontal), `"v"` (vertical), or omitted for both.

### `[flash time=… color=…]`

Flash the screen with a colour.

---

## Message Window

### `[msgwnd visible=… layer=…]`

Show or hide the message window.

### `[wndctrl x=… y=… width=… height=…]`

Set the position and size of the message window.

### Font control

| Tag | Description |
|-----|-------------|
| `[font face=… size=… bold=… italic=…]` | Set multiple font properties at once |
| `[size value=…]` | Font size in points |
| `[bold value=…]` | Bold on/off |
| `[italic value=…]` | Italic on/off |
| `[resetfont]` | Reset all font properties to defaults |
| `[ruby text=…]` | Set ruby (furigana) annotation for the next characters |

### Word wrap

| Tag | Description |
|-----|-------------|
| `[nowrap]` | Disable word wrapping |
| `[endnowrap]` | Re-enable word wrapping |

---

## Display Speed

| Tag | Description |
|-----|-------------|
| `[delay speed=N]` | Set per-character display delay (ms) |
| `[configdelay speed=N]` | Set the config-layer display delay |
| `[resetdelay]` | Reset delay to system default |
| `[nowait]` | Disable character-by-character delay entirely |
| `[endnowait]` | Re-enable delay |
| `[autowc time=N]` | Per-character post-display wait; `time=` omitted to disable |
| `[resetwait]` | Reset the auto-wait baseline timer |

---

## Backlog

### `[pushlog text=… join=…]`

Manually push a string into the backlog.

| Attribute | Type | Description |
|-----------|------|-------------|
| `text=` | string | The text to record |
| `join=` | bool | Append to the previous backlog entry instead of starting a new one |

### `[nolog]` / `[endnolog]`

Disable/re-enable automatic backlog recording of `DisplayText` events.

---

## Player Input

### `[input name=… prompt=… title=…]`

Open a text-input dialog. The entered value is stored in `f[name]`.

| Attribute | Type | Description |
|-----------|------|-------------|
| `name=` | string | Key in `f` to write the result into |
| `prompt=` | string | Placeholder / hint text |
| `title=` | string | Dialog title |

### `[waittrig name=…]`

Halt execution until the host fires a named trigger event.

---

## Character Sprites

### `[chara name=… id=… storage=… slot=… x=… y=…]`

Display a character sprite. At least one of `name=` or `id=` is required.

| Attribute | Type | Description |
|-----------|------|-------------|
| `name=` / `id=` | string | Character identifier |
| `storage=` | string | Sprite image path |
| `slot=` | string | Display slot |
| `x=` / `y=` | float | Position |

### `[chara_hide name=… id=… slot=…]`

Hide a character sprite (keeps it loaded).

### `[chara_free name=… id=… slot=…]`

Unload a character sprite.

### `[chara_mod name=… id=… face=… pose=… storage=…]`

Modify an already-displayed character sprite's expression/pose.

### `[chara_ptext name=…]`

Set the character name shown in the `ptext` name box.

---

## Macro System

### `[macro name=…]` / `[endmacro]`

Define a reusable block of script. The macro body runs each time the macro is invoked by its name.

```ks
[macro name=fade_to_black]
[fadeout time=500 color=0x000000]
[wf]
[endmacro]

; invoke:
[fade_to_black]
```

Inside a macro body, parameters passed at the call site are available as `%key` or via the `mp` Rhai map.

### `[erasemacro name=…]`

Delete a macro definition at runtime.

---

## Misc

### `[clickskip enabled=…]`

Enable or disable click-to-skip mode for transitions and animations.

### `[clearvar]` / `[clearsysvar]` / `[clearstack]`

| Tag | Clears |
|-----|--------|
| `[clearvar]` | All `f` game flags |
| `[clearsysvar]` | All `sf` system flags |
| `[clearstack]` | Call, if, and macro stacks |

---

## Quick Reference: All Tags

| Tag | Required attrs | Optional attrs |
|-----|---------------|---------------|
| `[if]` | `exp=` | — |
| `[elsif]` | `exp=` | — |
| `[else]` | — | — |
| `[endif]` | — | — |
| `[ignore]` | `exp=` | — |
| `[endignore]` | — | — |
| `[jump]` | `storage=` or `target=` | both |
| `[call]` | `storage=` or `target=` | both |
| `[return]` | — | — |
| `[link]` | `storage=` or `target=` | `text=` |
| `[endlink]` | — | — |
| `[glink]` | `storage=` or `target=` | `text=` |
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
| `[timeout]` | `time=` | `storage=`, `target=` |
| `[waitclick]` | — | — |
| `[click]` | `storage=`, `target=`, or `exp=` | all three |
| `[wheel]` | `storage=`, `target=`, or `exp=` | all three |
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
| `[input]` | `name=` | `prompt=`, `title=` |
| `[waittrig]` | `name=` | — |
| `[macro]` | — | `name=` |
| `[erasemacro]` | `name=` | — |
| `[endmacro]` | — | — |
| `[clearvar]` | — | — |
| `[clearsysvar]` | — | — |
| `[clearstack]` | — | — |
| `[clickskip]` | — | `enabled=` |
| `[chara_ptext]` | `name=` | — |
| `[bg]` | `storage=` | `time=`, `method=` |
| `[image]` | `storage=` | `layer=`, `x=`, `y=`, `visible=` |
| `[layopt]` | `layer=` | `visible=`, `opacity=` |
| `[free]` | `layer=` | — |
| `[position]` | `layer=` | `x=`, `y=` |
| `[bgm]` | `storage=` | `loop=`, `volume=`, `fadetime=` |
| `[stopbgm]` | — | `fadetime=` |
| `[se]` / `[playSe]` | `storage=` | `buf=`, `volume=`, `loop=` |
| `[stopse]` | — | `buf=` |
| `[vo]` / `[voice]` | `storage=` | `buf=` |
| `[fadebgm]` | — | `time=`, `volume=` |
| `[trans]` | — | `method=`, `time=`, `rule=` |
| `[fadein]` | — | `time=`, `color=` |
| `[fadeout]` | — | `time=`, `color=` |
| `[movetrans]` | — | `layer=`, `time=`, `x=`, `y=` |
| `[quake]` | — | `time=`, `hmax=`, `vmax=` |
| `[shake]` | — | `time=`, `amount=`, `axis=` |
| `[flash]` | — | `time=`, `color=` |
| `[msgwnd]` | — | `visible=`, `layer=` |
| `[wndctrl]` | — | `x=`, `y=`, `width=`, `height=` |
| `[resetfont]` | — | — |
| `[font]` | — | `face=`, `size=`, `bold=`, `italic=` |
| `[size]` | — | `value=` |
| `[bold]` | — | `value=` |
| `[italic]` | — | `value=` |
| `[ruby]` | — | `text=` |
| `[nowrap]` | — | — |
| `[endnowrap]` | — | — |
| `[chara]` | `name=` or `id=` | `storage=`, `slot=`, `x=`, `y=` |
| `[chara_hide]` | `name=` or `id=` | `slot=` |
| `[chara_free]` | `name=` or `id=` | `slot=` |
| `[chara_mod]` | `name=` or `id=` | `face=`, `pose=`, `storage=` |
| `[wa]`…`[wp]` | — | `canskip=`, `buf=` |

