; Shared macro definitions.
; Called via [call storage="tyrano.ks"] in first.ks — executes until [return].
;
; Porting notes vs. TyranoScript:
;   [iscript]...[endscript]  →  [eval exp="<rhai expr>"]
;   sf.cg_view / sf.replay_view  →  simple per-name boolean flags (sf.cg_<id> / sf.replay_<id>)
;   [chara_new] / [chara_face]   →  not used; pass storage= directly to [chara] / [chara_mod]

*start

; ── Replay mode: return to the replay menu when a replayable scene ends ───────
;
; Insert [endreplay] at the very end of every replayable scene block.
; When tf.flag_replay is true the engine is in replay mode and should jump back.
;
; Usage:
;   *replay_start
;   ...scene content...
;   [endreplay]

[macro name="endreplay"]
[if exp="tf.flag_replay == true"]
[eval exp="tf.flag_replay = false;"]
@layopt layer=message0 visible=false
@jump storage="replay.ks"
[endif]
[endmacro]

[return]
