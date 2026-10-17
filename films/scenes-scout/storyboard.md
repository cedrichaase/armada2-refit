# Scenes: scout board
Film: scenes-scout
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --install . --install testbench/scene

A scratch board for two shots the user asked for on rough cut 4 (2026-10-17): a Species
8472 mothership firing at a Fusion Cube, to replace the 8472 scene that "doesn't add much",
and a Federation fleet in front of a class M planet for the cold open. Not part of the
promo yet; every value is a first guess until a probe says otherwise.

## Species 8472

### c03 · The mothership fires
Shows: lighting: weapons
Scene: fusion-8472.ini
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: cut
In: 0.5
Length: 10
Status: draft

The mothership's pulse cannons firing at a Borg Fusion Cube, the pulses lighting its
plating. The cube's hull fills a good part of the frame and the pulses arrive across it;
the mothership is at the frame's edge. Shield flashes are off, so the hull is what is lit.

```setup
scene hover off; tooltips off; weapons sp off
scene orbit 3000 0 2740 165 12 340
wait 1
```
```action
wait 0.5
scene weapons sp on; glide 10 150 8 300
wait 10
```

## Cold open

### p01 · The fleet before the planet
Shows: lighting: planet
Scene: fleet-planet.ini
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: dissolve 0.6
In: 0.5
Length: 14
Status: draft

Five Federation ships in a loose formation under way, slowly, in front of a class M
planet that fills the frame behind them, lit by the sun. The camera stays low and drifts
along with the lead ship. No combat.

```setup
scene hover off; tooltips off
scene orbit fente 225 -22 270
wait 1
```
```action
wait 0.5
scene goto fente,sov,gal,akira,intr 2865 0 2594; glide 14 205 -26 240
wait 14
```

### p01b · The fleet from above
Shows: lighting: planet
Scene: fleet-planet.ini
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: dissolve 0.6
In: 0.5
Length: 14
Status: draft

Five Federation ships in a loose formation under way, slowly, in front of a class M
planet that fills the frame behind them, lit by the sun. The camera stays high and drifts
along with the lead ship. No combat. The high variant of p01: the planet below and behind.

```setup
scene hover off; tooltips off
scene orbit fente 225 25 330
wait 1
```
```action
wait 0.5
scene goto fente,sov,gal,akira,intr 2865 0 2594; glide 14 205 30 300
wait 14
```
