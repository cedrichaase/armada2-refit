# RTS starbase scout
Film: rts-sb-scout
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --install . --install testbench/scene

A scratch board for the cold open's middle shot (the user, rough cut 4: Enterprise from
the Federation intro, then a top-down RTS view of a game scene, then the fleet in front of
an M-class planet). Not part of the promo.

### t01 · The game, as a player sees it
Kind: scene
Scene: rts.ini
Variants: refit
Layout: full refit
Transition: cut
In: 1.5
Length: 10.5
Status: draft

The game's RTS camera, close, slightly tilted, HUD off: a Federation fleet arrives at a
working base by a dilithium moon (a starbase, three freighters mining), while a
construction ship builds a new station beside the moon.

```setup
scene hud off
scene build rig fmining 2130 0 2250
wait 5
```
```action
scene goto gal,c1,d1,d2 2290 0 2190
wait 1.5
scene special f1 gmineDilithium moon
wait 1.5
scene special f2 gmineDilithium moon
wait 1.5
scene special f3 gmineDilithium moon
wait 1.5
scene special f1 gmineDilithium moon
wait 1.5
scene special f2 gmineDilithium moon
wait 1.5
scene special f3 gmineDilithium moon
wait 3
```

