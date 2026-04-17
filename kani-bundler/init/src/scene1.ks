; kani-engine demo scenario — scene 1.
;
; Porting notes vs. TyranoScript / scene1.ks:
;   [iscript]...[endscript]          →  [eval exp="<rhai>"]
;   [chara_new] / [chara_face]       →  removed; storage= passed directly
;   [chara_show name=x]              →  [chara name=x storage="..."]
;   [chara_mod name=x face=happy]    →  [chara_mod name=x storage="chara/akane/happy.png"]
;   [position layer=message0 ...]    →  [wndctrl x=... y=... width=... height=...]
;   [ptext] / [chara_config ptext=]  →  [chara_ptext name=...] or #Name shorthand
;   [clearfix] / [start_keyconfig]   →  not used (engine-internal in TyranoScript)
;   [hidemenubutton] / [showmenubutton] →  not used
;   @showmenubutton                  →  not used
;   [button name=role_button role=…] →  not used (system UI handled by the runtime)
;   [anim ...]                       →  not used (no equivalent yet)
;   [playbgm]                        →  [bgm]
;   [freeimage layer=1]              →  [free layer=1]
;   [glink color=… size=… x=… y=…]  →  [link] (text choices)
;   CG unlock: sf.cg_<id> = true    →  [eval exp="sf.cg_room = true;"]
;   Replay unlock: sf.replay_<id>   →  [eval exp="sf.replay_demo = true;"]

; ─────────────────────────────────────────────────────────────────────────────
*start

[cm]
[bg storage="bgimage/room.jpg" time=100]
[layopt layer=message0 visible=true]
[wndctrl x=160 y=500 width=1000 height=200]

; Mark this scene as replayable.  The replay_start label is what replay.ks
; will jump to so the scene begins cleanly (skipping the variable init above).
[eval exp="sf.replay_demo = true;"]

*replay_start

; Unlock the room CG for the gallery
[eval exp="sf.cg_room = true;"]

#
Hm. They said making a game was easy here...[p]

Nobody's home.[p]

...[p]

I guess I'll just head back.[p]

; ── Akane appears ─────────────────────────────────────────────────────────────

[font size=30]
#?
Hey, wait![p]
[resetfont]

#
Who's there?![p]

[chara name=akane storage="chara/akane/normal.png" x=300 y=100]
[chara_ptext name=akane]

#?
Hello![p]

My name is Akane.[p]

#akane
Are you interested in making visual novel games?[p]

[link target="*selectinterest"]
Yes, definitely!
[link target="*selectinterest"]
I'm interested!
[link target="*selectinterest"]
Sort of, maybe...
[endlink]

; ── Branch converges here ──────────────────────────────────────────────────────

*selectinterest

[chara_mod name=akane storage="chara/akane/happy.png"]
#akane
Really?! That makes me so happy![p]

#
...Well, I want to make one, but it sounds complicated.[p]
I've never done any programming before.[p]

[chara_mod name=akane storage="chara/akane/normal.png"]
#akane
Well, have I got news for you![p]
Interested?[p]

#
Sure, I guess.

#akane
[cm]
[font size=40]
[delay speed=160]
kani-engine![p]
[delay speed=30]
[resetfont]

#
...kani-engine?[p]

#akane
With kani-engine you can create a full-featured visual novel with ease![p]

#
Hm, that actually sounds interesting.[p]

[chara_mod name=akane storage="chara/akane/happy.png"]
#akane
I knew you'd say that![p]
Playing through this demo you can see kani-engine's features first-hand,[p]
so please stick with me to the end![p]

First — kani-engine runs on [font color=red]Bevy[resetfont], a powerful Rust game engine.[p]

#
What does that mean, exactly?[p]

#akane
It means your game can target Windows, macOS, Linux, and more from a single codebase![p]

#
Oh nice. I want as many people as possible to play my game.[p]

#akane
And because the scripting language is [font color=blue]Rhai[resetfont] instead of JavaScript,[p]
you get type-safety and Rust-native speed out of the box.[p]

#
So... no web browser needed?[p]

#akane
Exactly! Games run natively as a Bevy app.[p]
Now let me show you some features. Starting with scene transitions.[p]
We're heading to the hallway![p]

[bg storage="bgimage/hallway.jpg" time=3000 method=crossfade]

; Unlock the hallway CG
[eval exp="sf.cg_hallway = true;"]

#
Oh — we're in the hallway![p]

#akane
Brrr, it's cold! Let's hurry back.[p]

[bg storage="bgimage/room.jpg" time=1000 method=slide]

#
That transition was different![p]

#akane
Right — kani-engine supports multiple transition effects for scene changes.[p]
crossfade, slide, wipe, and more.[p]

#
Very handy.[p]

#akane
Next, let me show you different ways to display text.[p]
In addition to this adventure-style box, you can use[p]
a full-screen visual novel layout![p]

#

; ── Full-screen text mode ──────────────────────────────────────────────────────

[chara_hide name=akane]
[wndctrl x=20 y=40 width=1200 height=660]

What do you think? Full-screen mode is great for narration.[l][r]
kani-engine gives you flexible, pixel-level control over text layout.[l][cm]

[font size=40]You can change the font size like this,[l][r]
[resetfont]
[font color=pink]change the colour,[resetfont][l][r]

or add [ruby text=annotation]ruby text[ruby text=like][ruby text=this] like this.[l]
[cm]

; ── Back to adventure mode ─────────────────────────────────────────────────────

[wndctrl x=160 y=500 width=1000 height=200]
[chara name=akane storage="chara/akane/normal.png" x=300 y=100]
[chara_ptext name=akane]

#akane
The message window can use any design you like![p]

By the way, kani-engine also supports screen effects.[p]

[flash time=300 color=0xFFFFFF]

Like a flash effect — useful for lightning or dramatic reveals.[p]

[quake time=600 hmax=8 vmax=4]

And a screen quake for impact moments.[p]

Now — should I play some background music?[l][cm]

[link target="*playmusic"]
Yes, please play something!
[link target="*noplay"]
No, not right now.
[endlink]

; ── BGM branch ────────────────────────────────────────────────────────────────

*playmusic

[cm]
Alright, here we go![l]
@bgm storage="bgm/main_theme.ogg" fadetime=3000 loop=true
It fades in gradually — pretty smooth, right?[l][cm]

@jump target="*common_bgm"

*noplay

[cm]
OK, no music then. You can change it in the Config screen later.[l][cm]

*common_bgm

Story branches from player choices — just like what you did a moment ago.[l][cm]

; ── Second character ──────────────────────────────────────────────────────────

#akane
Let me introduce another character![l][cm]
Yamato![p]

[chara name=yamato storage="chara/yamato/normal.png" x=700 y=100]

See how easy it is to have multiple characters on screen.[l][r]
You can bring in as many characters as the story needs.[p]

#yamato
Can I go home now?[l][cm]

#akane
Sorry about that! Thanks for coming.[l][cm]

[chara_hide name=yamato]

; ── Wrap-up ───────────────────────────────────────────────────────────────────

#akane
That covers kani-engine's core features![p]
What do you think?[p]

#
I think even I could make a game with this.[p]

[chara_mod name=akane storage="chara/akane/happy.png"]
#akane
That's exactly what I hoped to hear![p]
kani-engine uses KAG scripts with embedded Rhai — no JavaScript required.[p]
The full tag reference and getting-started guide are in the docs.[p]
Thanks for playing![p]

; Replay mode exit: if we are in a replay, jump back to replay.ks
[endreplay]

[cm]

; ── Info link menu ────────────────────────────────────────────────────────────

*info_menu

@layopt layer=message0 visible=false

[link target="*info_kag_rhai"]
About KAG + Rhai scripting
[link target="*info_variables"]
About game variables (f / sf / tf)
[link target="*info_features"]
Feature highlights
[link storage="title.ks"]
Back to Title
[endlink]

; ─────────────────────────────────────────────────────────────────────────────
*info_kag_rhai

@layopt layer=message0 visible=true

#akane
kani-engine scenarios are plain .ks files using KAG syntax.[p]
Rhai expressions can be embedded anywhere:[p]
  eval runs a Rhai script with side-effects,[p]
  emb injects an expression result into the text stream,[p]
  and attribute values prefixed with & are evaluated as Rhai at runtime.[p]

@jump target="*info_menu"

; ─────────────────────────────────────────────────────────────────────────────
*info_variables

@layopt layer=message0 visible=true

#akane
There are four variable scopes accessible from Rhai:[p]
  f  — game flags (saved with the game save),[p]
  sf — system flags (saved globally, survives new games),[p]
  tf — transient flags (NOT saved, reset on load),[p]
  mp — macro parameters (available inside macro bodies).[p]

For example, this demo used sf.cg_room and sf.replay_demo[p]
to track unlocked gallery entries across saves.[p]

@jump target="*info_menu"

; ─────────────────────────────────────────────────────────────────────────────
*info_features

@layopt layer=message0 visible=true

#akane
Some of what this demo showed:[p]
  Background images with timed transitions (crossfade, slide),[p]
  Character sprites with expression changes,[p]
  Story branching with [link] choices,[p]
  Background music with fade-in,[p]
  Font size and colour changes plus ruby text,[p]
  Screen flash and quake effects,[p]
  Full-screen vs. windowed message layout,[p]
  and a CG gallery and replay mode backed by sf flags.[p]
Check the docs for the complete KAG-Rhai tag reference![p]

@jump target="*info_menu"

[s]
