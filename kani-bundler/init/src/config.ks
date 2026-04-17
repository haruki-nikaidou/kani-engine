; Config / settings screen.
;
; Porting notes vs. TyranoScript / config.ks:
;   [iscript]...[endscript]     →  [eval exp="<rhai>"]
;   [button fix=true ...]       →  [link] text choices (no positioned image buttons)
;   [bgmopt volume=...]         →  [fadebgm time=500 volume=...]
;   [seopt volume=...]          →  stored in sf.se_volume (applied by game logic)
;   [configdelay speed=...]     →  [configdelay speed=...] ✓ (same tag)
;   [autoconfig speed=...]      →  stored in sf.auto_speed (no direct tag equivalent)
;   [config_record_label skip=] →  stored in sf.skip_unread (engine reads this flag)
;   [awakegame]                 →  [clearstack] then jump back (no sleepgame system)
;   [stop_keyconfig] / [start_keyconfig] →  not used
;   [hidemenubutton]            →  not used
;   bg_config.png path          →  "image/config/bg_config.png"

*start

[cm]

; Initialise settings to defaults on first visit
[eval exp="if sf.bgm_volume == () { sf.bgm_volume = 100; }"]
[eval exp="if sf.se_volume  == () { sf.se_volume  = 100; }"]
[eval exp="if sf.text_speed == () { sf.text_speed = 30; }"]
[eval exp="if sf.auto_speed == () { sf.auto_speed = 2000; }"]
[eval exp="if sf.skip_unread == () { sf.skip_unread = false; }"]

@bg storage="image/config/bg_config.png" time=100
@layopt layer=message0 visible=true

; ── Main menu ─────────────────────────────────────────────────────────────────

*config_main

[cm]
── Settings ──[r]
[r]
  BGM Volume  :  [emb exp="sf.bgm_volume"]%[r]
  SE  Volume  :  [emb exp="sf.se_volume"]%[r]
  Text Speed  :  [emb exp="sf.text_speed"] ms / char[r]
  Auto Speed  :  [emb exp="sf.auto_speed"] ms[r]
  Skip Unread :  
[if exp="sf.skip_unread"]
ON[r]
[else]
OFF[r]
[endif]
[p]

What would you like to adjust?[p]

[link target="*conf_bgm"]
BGM Volume
[link target="*conf_se"]
SE Volume
[link target="*conf_speed"]
Text Speed
[link target="*conf_auto"]
Auto Speed
[link target="*conf_skip"]
Skip Mode
[link target="*exit_config"]
← Back to Title
[endlink]

; ── BGM Volume ────────────────────────────────────────────────────────────────

*conf_bgm

[cm]
BGM Volume: [emb exp="sf.bgm_volume"]%[p]

[link target="*bgm_0"]
0% — Mute
[link target="*bgm_30"]
30% — Low
[link target="*bgm_60"]
60% — Medium
[link target="*bgm_100"]
100% — Full
[link target="*config_main"]
← Back
[endlink]

*bgm_0
[eval exp="sf.bgm_volume = 0;"]
[fadebgm time=500 volume=0.0]
@jump target="*conf_bgm"

*bgm_30
[eval exp="sf.bgm_volume = 30;"]
[fadebgm time=500 volume=0.3]
@jump target="*conf_bgm"

*bgm_60
[eval exp="sf.bgm_volume = 60;"]
[fadebgm time=500 volume=0.6]
@jump target="*conf_bgm"

*bgm_100
[eval exp="sf.bgm_volume = 100;"]
[fadebgm time=500 volume=1.0]
@jump target="*conf_bgm"

; ── SE Volume ─────────────────────────────────────────────────────────────────

*conf_se

[cm]
SE Volume: [emb exp="sf.se_volume"]%[p]

[link target="*se_0"]
0% — Mute
[link target="*se_30"]
30% — Low
[link target="*se_60"]
60% — Medium
[link target="*se_100"]
100% — Full
[link target="*config_main"]
← Back
[endlink]

*se_0
[eval exp="sf.se_volume = 0;"]
@jump target="*conf_se"

*se_30
[eval exp="sf.se_volume = 30;"]
@jump target="*conf_se"

*se_60
[eval exp="sf.se_volume = 60;"]
@jump target="*conf_se"

*se_100
[eval exp="sf.se_volume = 100;"]
@jump target="*conf_se"

; ── Text Speed ────────────────────────────────────────────────────────────────

*conf_speed

[cm]
Text Speed: [emb exp="sf.text_speed"] ms / char  (lower = faster)[p]

[link target="*speed_slow"]
Slow   (80 ms)
[link target="*speed_normal"]
Normal (30 ms)
[link target="*speed_fast"]
Fast   (10 ms)
[link target="*speed_instant"]
Instant (0 ms)
[link target="*config_main"]
← Back
[endlink]

*speed_slow
[eval exp="sf.text_speed = 80;"]
[configdelay speed=80]
[cm]
Sample text at this speed: The quick brown fox jumps over the lazy dog.
[wait time=4000 canskip=true]
[cm]
@jump target="*conf_speed"

*speed_normal
[eval exp="sf.text_speed = 30;"]
[configdelay speed=30]
[cm]
Sample text at this speed: The quick brown fox jumps over the lazy dog.
[wait time=2000 canskip=true]
[cm]
@jump target="*conf_speed"

*speed_fast
[eval exp="sf.text_speed = 10;"]
[configdelay speed=10]
[cm]
Sample text at this speed: The quick brown fox jumps over the lazy dog.
[wait time=1000 canskip=true]
[cm]
@jump target="*conf_speed"

*speed_instant
[eval exp="sf.text_speed = 0;"]
[configdelay speed=0]
[cm]
Sample text at this speed: The quick brown fox jumps over the lazy dog.
[wait time=500 canskip=true]
[cm]
@jump target="*conf_speed"

; ── Auto Speed ───────────────────────────────────────────────────────────────

*conf_auto

[cm]
Auto Speed: [emb exp="sf.auto_speed"] ms  (time to wait after each page in auto mode)[p]

[link target="*auto_slow"]
Slow   (5000 ms)
[link target="*auto_normal"]
Normal (2000 ms)
[link target="*auto_fast"]
Fast   (800 ms)
[link target="*config_main"]
← Back
[endlink]

*auto_slow
[eval exp="sf.auto_speed = 5000;"]
@jump target="*conf_auto"

*auto_normal
[eval exp="sf.auto_speed = 2000;"]
@jump target="*conf_auto"

*auto_fast
[eval exp="sf.auto_speed = 800;"]
@jump target="*conf_auto"

; ── Skip Mode ─────────────────────────────────────────────────────────────────

*conf_skip

[cm]
Skip Unread:
[if exp="sf.skip_unread"]
ON
[else]
OFF
[endif]
[p]

When ON  — skip mode advances through text you have not read before.[r]
When OFF — skip mode only advances through already-seen text.[r]
[p]

[link target="*skip_on"]
Turn ON
[link target="*skip_off"]
Turn OFF
[link target="*config_main"]
← Back
[endlink]

*skip_on
[eval exp="sf.skip_unread = true;"]
@jump target="*conf_skip"

*skip_off
[eval exp="sf.skip_unread = false;"]
@jump target="*conf_skip"

; ── Exit ──────────────────────────────────────────────────────────────────────

*exit_config
[cm]
[clearstack]
@jump storage="title.ks"
