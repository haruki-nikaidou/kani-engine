; CG Gallery screen.
;
; Porting notes vs. TyranoScript / cg.ks:
;   [iscript]...[endscript]            →  [eval exp="<rhai>"]
;   [button graphic=... x=... y=...]   →  [link] text choices (no positioned thumbnails)
;   sf.cg_view[key] = "on" / check     →  sf.cg_<id> boolean flags set from scene1.ks
;   cg_image_button macro              →  inline [if] / [link] per CG entry
;   pagination (tf.page)               →  single page (add more *page_N labels for large galleries)
;   [freeimage layer=1]                →  [free layer=1]
;
; To add a new CG to the gallery:
;   1. In your scenario, write:  [eval exp="sf.cg_myname = true;"]
;   2. Add a [if] / [link] / *view block below (follow the pattern for *view_room).

*start

[cm]
@layopt layer=message0 visible=true
@bg storage="bgimage/title.jpg" time=100

*cg_menu

[cm]
── CG Gallery ──[r]
[r]
[if exp="sf.cg_room == true || sf.cg_hallway == true"]
Select a CG to view:[r]
[r]
[if exp="sf.cg_room == true"]
[link target="*view_room"]
○  Classroom
[endif]
[if exp="sf.cg_hallway == true"]
[link target="*view_hallway"]
○  Hallway
[endif]
[link storage="title.ks"]
← Back to Title
[endlink]
[else]
No CGs have been unlocked yet.[r]
Play through the game to discover them.[p]
@jump storage="title.ks"
[endif]

; ── CG viewers ────────────────────────────────────────────────────────────────

*view_room

[cm]
[bg storage="bgimage/room.jpg" time=500]
[l]
@jump target="*cg_menu"

*view_hallway

[cm]
[bg storage="bgimage/hallway.jpg" time=500]
[l]
@jump target="*cg_menu"
