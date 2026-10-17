# c03: scout board
Film: c03-scout
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --install . --install testbench/scene

A scratch board for restaging c03 on the user's notes on rough cut 5 (2026-10-17).

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

```setup
scene hover off; tooltips off; weapons sp off
scene orbit 3030 0 2800 54 -4 580
wait 1
```
```action
wait 0.5
scene weapons sp on; glide 10 61 -5 560
wait 6
scene kill bs1
wait 4
```
