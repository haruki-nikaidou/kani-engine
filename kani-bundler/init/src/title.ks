; Title screen.
; Uses [link]/[endlink] for navigation since [button role=...] is Tyrano-specific.

[cm]
@clearstack
@bg storage="bgimage/title.jpg" time=500
@wait time=200

*start

[link target="*gamestart"]
▶  New Game
[link storage="cg.ks"]
▶  CG Gallery
[link storage="replay.ks"]
▶  Replay
[link storage="config.ks"]
▶  Config
[endlink]

*gamestart
@clearvar
@jump storage="scene1.ks"
