# Shadow scout
Film: shadow-scout
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --install . --install testbench/scene

Scratch board for the promo's f01.

## Scout

### f01 · Out of the planet's shadow
Shows: lighting: planet shadows, lighting: cloak
Scene: rom-shadow.ini
Variants: refit
Layout: full refit
In: 2.0
Length: 13
Status: draft

```setup
scene hover off; shieldfx off; tooltips off
scene cloak wb1 now; cloak wb2 now; cloak wb3 now; cloak gr1 now; cloak gr2 now; cloak sh1 now
scene orbit wb1 80 -5 230
ptr 1900 1060 0.1
wait 3
```
```action
scene goto wb1,wb2,wb3,gr1,gr2,sh1 2973 0 1998
wait 3
scene cloak wb1 off
wait 0.4
scene cloak wb2 off; cloak wb3 off
wait 0.4
scene cloak gr1 off; cloak gr2 off; cloak sh1 off
wait 2.0
scene glide 10 195 -3 150
wait 10
```
