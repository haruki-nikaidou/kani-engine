; Replay (Recollection) mode screen.
;
; Porting notes vs. TyranoScript / replay.ks:
;   [iscript]...[endscript]              →  [eval exp="<rhai>"]
;   [button graphic=... x=... y=...]     →  [link] text choices
;   sf.replay_view[name] = {storage, target}  →  sf.replay_<id> boolean + hardcoded storage/target
;   replay_image_button macro            →  inline [if] / [link] per entry
;   tf.flag_replay controlled by iscript →  [eval exp="tf.flag_replay = true;"]
;   [awakegame] / sleepgame system       →  not used; [endreplay] macro in tyrano.ks handles return
;
; To add a new replayable scene:
;   1. In your scenario, write:  [eval exp="sf.replay_myscene = true;"]
;   2. Mark the entry label with *replay_start (or any chosen label).
;   3. Put [endreplay] at the end of the replayable block.
;   4. Add a [if] / [link] / *play block below (follow the pattern for *play_demo).

*start

[cm]
@layopt layer=message0 visible=true
@bg storage="bgimage/title.jpg" time=100

*replay_menu

[cm]
── Replay ──[r]
[r]
[if exp="sf.replay_demo == true"]
Select a scene to replay:[r]
[r]
[link target="*play_demo"]
○  Demo Scene
[link storage="title.ks"]
← Back to Title
[endlink]
[else]
No scenes have been unlocked yet.[r]
Play through the game to unlock replay scenes.[p]
@jump storage="title.ks"
[endif]

; ── Scene launchers ───────────────────────────────────────────────────────────

*play_demo

[eval exp="tf.flag_replay = true;"]
@jump storage="scene1.ks" target="*replay_start"
