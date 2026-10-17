# HUD scout
Film: hud-scout
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --assets none --install . --install testbench/scene

A scratch board, 2026-10-17: the promo's HUD shots (`films/promo/storyboard.md`, u01 and
u02) filmed against the HUD's new bottom row (ui 0.11.2: the bar beside the minimap at
16:9, the info panel moved right for it, a group's tiles smaller), with their layout
changed from a wipe to a dissolve. The user, 2026-10-17: the split across the centre
does not work for the menus and the HUD, which look too different from stock, even in
the geometry and placement of their parts. g01 and g02 are here as they are in the
promo, to see the drags select the whole block now that the bar and the group no
longer cover it. Nothing here is approved; the promo's board is not changed.

## The HUD

### u01 · The HUD in battle
Shows: hud
Scene: ../promo/hud-showcase.ini
Variants: stock, refit
Layout: dissolve 2.5 1.2
Transition: dissolve 0.8
Title: And the HUD
Sub: Every panel where it was, in LCARS, at any aspect ratio
Caption: top
In: 1.0
Length: 7
Status: draft

The showcase battle with the HUD on and the Enterprise selected, so the info panel is
full, the camera high and still as a player's would be. Stock's HUD held, then dissolved
into refit's: the eye goes to the bottom panels and the type.

```setup
scene hover off; shieldfx off; tooltips off
scene hud on
scene select ente
scene orbit 2250 0 2350 160 45 1200
```
```action
wait 9
```

### u02 · The info panel, close
Shows: hud: info panel
Scene: ../promo/hud-showcase.ini
Variants: stock, refit
Layout: dissolve 2 1
Crop: 0.16 0.36 0.64 0.64
Transition: dissolve 0.5
Caption: top
In: 1.0
Length: 6
Status: draft

The bar and the whole info panel: stock's stretched sprites dissolving into refit's flat
icons and live close-up. The crop holds both layouts (stock: the bar above the panel,
x 0.22-0.78; refit: the bar beside the minimap, the panel to its right, x 0.18-0.78).

```setup
scene hover off; shieldfx off; tooltips off
scene hud on
scene select ente
scene orbit 2250 0 2350 160 45 1200
```
```action
wait 8
```

## Gameplay

### g01 · Selections beyond sixteen
Shows: qol: selections
Scene: ../promo/qol.ini
Variants: stock, refit
Layout: dissolve 2.5 1.2
Transition: dissolve 0.5
Title: Select the whole fleet
Sub: Twenty-four ships in one selection, where stock stopped at sixteen
Caption: top
In: 1.0
Length: 7
Status: draft

As in the promo: twenty-four destroyers dragged into one selection.

```setup
ptr 400 640 0.1
wait 2
```
```action
wait 1.5
drag 830 430 1350 745 1.6
ptr 1700 300 0.5
wait 7
```

### g02 · Shift adds to a group
Shows: qol: control groups
Scene: ../promo/qol.ini
Variants: refit
Layout: full refit
Transition: dissolve 0.4
Title: Shift+number adds to a group
Caption: top
Keys: auto
In: 1.0
Length: 15
Status: draft

As in the promo: half the block to group 1, the other half added with Shift+1, then 1
calls up all twenty-four. In the promo's probe the second drag began under the bar,
which sat over the block's left column, and missed it.

```setup
lclick 400 640
wait 1
```
```action
wait 1
drag 830 430 1345 580 1.2
wait 0.8
press ctrl+1
wait 1.2
lclick 400 640
wait 0.8
drag 830 585 1355 745 1.2
wait 0.8
press shift+1
wait 1.2
lclick 400 640
wait 1.2
press 1
wait 9.5
```
