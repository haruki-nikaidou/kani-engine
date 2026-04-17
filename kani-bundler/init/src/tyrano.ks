
*start

; ── CG mode: mark a CG image as seen ─────────────────────────────────────────

[macro name="cg"]

[iscript]
    if !sf.contains_key("cg_view") {
        sf.cg_view = #{};
    }
    sf.cg_view[mp.storage] = "on";
[endscript]

[endmacro]

; ── CG mode: show a button for a CG image (locked if not yet seen) ───────────

[macro name="cg_image_button"]

[iscript]
    mp.graphic = mp.graphic.split(",");
    mp.tmp_graphic = mp.graphic.clone();
    tf.is_cg_open = false;
    if sf.contains_key("cg_view") && sf.cg_view.contains_key(mp.graphic[0]) {
        tf.is_cg_open = true;
    }
    if mp.contains_key("thumb") {
        mp.tmp_graphic[0] = mp.thumb;
    }
[endscript]

[if exp="tf.is_cg_open == true"]
    [button graphic=&mp.tmp_graphic[0] x=&mp.x y=&mp.y width=&mp.width height=&mp.height preexp="mp.graphic" exp="tf.selected_cg_image = preexp" storage="cg.ks" target="*clickcg" folder="bgimage"]
[else]
    [button graphic=&mp.no_graphic x=&mp.x y=&mp.y width=&mp.width height=&mp.height storage="cg.ks" target="*no_image" folder="bgimage"]
[endif]

[endmacro]

; ── Replay mode: unlock a replay scene ───────────────────────────────────────

[macro name="setreplay"]

[iscript]
    if !sf.contains_key("replay_view") {
        sf.replay_view = #{};
    }
    sf.replay_view[mp.name] = #{storage: mp.storage, target: mp.target};
[endscript]

[endmacro]

; ── Replay mode: show a button for a replay scene (locked if not yet seen) ───

[macro name="replay_image_button"]

[iscript]
    tf.is_replay_open = false;
    if sf.contains_key("replay_view") && sf.replay_view.contains_key(mp.name) {
        tf.is_replay_open = true;
    }
[endscript]

[if exp="tf.is_replay_open == true"]
    [button graphic=&mp.graphic x=&mp.x y=&mp.y width=&mp.width height=&mp.height preexp="sf.replay_view[mp.name]" exp="tf.selected_replay_obj = preexp" storage="replay.ks" target="*clickcg" folder="bgimage"]
[else]
    [button graphic=&mp.no_graphic x=&mp.x y=&mp.y width=&mp.width height=&mp.height storage="replay.ks" target="*no_image" folder="bgimage"]
[endif]

[endmacro]

; ── Replay mode: return to replay menu after a scene ends ────────────────────

[macro name="endreplay"]

[if exp="tf.flag_replay == true"]
    @layopt page="fore" layer="message0" visible=false
    @jump storage="replay.ks"
[endif]

[endmacro]

[return]


