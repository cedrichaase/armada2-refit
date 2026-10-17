# Armada II Refit
Film: promo
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --assets none --install . --install testbench/scene

The promo for the release that puts render3d and the UI in the installer (12.24.0): the one
video that has to make someone who loved Armada II in 2001 install it again tonight.
Rewritten 2026-10-17 for the launch campaign (maker-promo, `2026-11-refit-launch`), after
the UI rewrite; the board before it is in git.

The angle, confirmed by the user: **Armada II as it shipped, and the same moment with what
I've built on top of it: same game, same camera, side by side, and you can install it over
your own copy.** Every shot that can be is a before/after from one shot list.

The order is the user's, feature by feature:

1. **Combat**: fire, shields, damage and weapons drawn as light and gas, not flipbooks.
2. **Beams**: the tractor, holding and assimilation beams, drawn after the TV series.
3. **Nebulae**: a volume of gas the ships fly through, up close, filmed against the Key
   light so the hulls' shadows run through it.
4. **Menus**, and 5. **the HUD**: a rewrite of their own, not stock's art un-stretched.
   The HUD looks very different but keeps stock's geometry (the user, 2026-10-17), so a
   wipe compares it part for part; the main menu is a column over a scene, so it dissolves.
6. **Quality of life**: limits gone, still playable against stock players.
7. **Installing**: the installer in a few steps, ending in the game.

**Planets, hulls and the sky are in the shots but never called out** (the user,
2026-10-17): no titles of their own; the showcase fleet over its planet, the cloak in front
of one, the menu scene's planet and the sky behind everything carry them. Hull lighting is
named only where a weapon's light falls on a hull.

About 3:50 by the shot lengths, a little less once transitions overlap; the brief aims at
3:30, so trim after the animatic (candidates: s05b, u03, g03, i08). The HUD and the UI
get a deeper video of their own later, with voice-over: here they get one beat each.

For the editor:

- The house style holds (`films/README.md`): close-ups, the camera a little below the
  subject looking up, a wipe for one centred subject, a dissolve for subjects spread over
  the frame, no game music in any take, one music bed at a time.
- **Captions are first person and plain** (maker-promo's `brand/brand.md`, "Voice"): a
  lower third names a feature once, the first time it is on screen. No act cards except
  where the film turns (gameplay, installing).
- **"Refit" is the game as the release installs it**: every code layer, none of
  `$A2_DATA`'s assets: no remastered textures, no backdrop plates, no movies
  (`--assets none`). The thumbnail is the one exception.
- **Every current-state shot is filmed fresh** from the release's commit; the takes from
  2026-10-15 predate the nebula rework and the UI.
- Online play is in progress and is not in this film.
- Camera values copied from the October showcase and `films/whats-new` were checked on the
  bench; values marked *unprobed* are first guesses.

## Cold open
Music: Mbor300.wav from 0 gain -1 until h01 fade 1.2

Fifteen seconds: the game as it shipped, then what it is now. Stock's Enterprise held just
long enough to be recognised, a hard cut to the same moment in refit on the Borg bed's
first hit, two teasers, the title. Every shot here is cut from other shots' takes.

### o01 · Stock, as it shipped
Kind: footage
From: h03 stock
In: 3.0
Length: 3
Transition: fade 1.0
Audio: none
Caption: top
Title: 2001

Stock's Enterprise, the same frame o02 cuts to in refit (h03: h01's stock hull fades near
the camera).

### o02 · Teaser: the Enterprise
Kind: footage
From: h03 refit
In: 3.0
Length: 2.5
Audio: none

Hard cut on the bed's first hit: the same frame as o01, refit. No transition.

### o03 · Teaser: the fireball
Kind: footage
From: b01 refit
In: 1.5
Length: 0.9
Transition: cut
Audio: game

The cruiser's fireball at its height; the blast's sound lands on the cut.

### o04 · Teaser: into the nebula
Kind: footage
From: n01 refit
In: 2.0
Length: 2.4
Transition: dissolve 0.3
Audio: none

Gas sliding past the lens, a hull's shadow in it.

### o05 · Title
Length: 4.5
Transition: dip 0.5
Background: s05b

```card
[big] ARMADA II
[accent] R E F I T
[small] The 2001 game, redrawn. Free and open source.
```

## Combat
Music: Mkli300.wav from 0 gain 0 until a02 fade 1.5

The loud act, Klingon music, quick cuts. The argument: a battle in stock Armada II is
flipbooks: a sprite sheet per fireball, a disc per shockwave, a green shell per shield
hit. Refit draws each as light and gas in the scene, touching the hulls around it. It
opens on the showcase fleet over its planet, which also sets the scene for the hulls and
the sky without a word about them.

### h01 · The fleet in action
Shows: lighting, hulls
Scene: showcase
Variants: stock, refit
Layout: wipe
Transition: dissolve 1.0
In: 0.5
Length: 6
Status: draft

Close and low under the Enterprise-E's saucer, sliding along it, the battle beyond. No
title: the hull's light and shadow are there to be noticed, not named. Pulled back to 170
on 2026-10-15: stock fades a hull within about 150. If a transporter effect drifts into
frame, move `In:`.

```setup
step Move the mouse to 1900,8
scene hover off; shieldfx off; tooltips off
scene orbit ente 300 -12 170
```
```action
wait 1
scene glide 10 380 -5 200
wait 9.5
```

### b01 · A cruiser explodes
Shows: explosions
Scene: explosion
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Explosions
Sub: Fire and smoke as a volume, lighting the ships around it
In: 1.3
Length: 2.0
Speed: 0.5
Status: draft

The fireball's core: stock's sprite turns with the camera; refit's is a volume the camera
moves around, and the flash lights the Galaxy and the Sovereign beside it. The cut lands on
the act's first beat of music. From 2.7 s stock's shockwave disc fills the frame in both
variants (2026-10-15), so the shot ends before it. Probe 1: the flash at 1.4 s lights the Galaxy and the Sovereign
at 1.5–1.7; the disc fills the frame from 2.5, so the window is 1.3–2.3, at half speed;
closer (150 → 120) so the fireball is bigger.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit 2090 0 2230 80 -6 120
```
```action
wait 1
scene kill mid; glide 7 110 -4 115
wait 7
```

### h03 · Phaser light
Shows: lighting: weapons
Scene: showcase
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Weapons light the hulls
Sub: Phasers, torpedoes and pulses, each in its own colour
In: 1.0
Length: 6
Status: draft

From above, on purpose: the Enterprise's phaser lights its own hull, and the light on the
plating is the subject. If the beam does not fire inside the window, move `In:`.

```setup
step Move the mouse to 1900,8
scene hover off; shieldfx off; tooltips off
scene orbit ente 130 32 140
```
```action
wait 1
scene glide 12 205 22 175
wait 8.5
```

### b02 · Shield hits
Shows: shields
Scene: shields
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Shields
Sub: Each hit lights the shield where it lands
In: 5.0
Length: 2.8
Status: draft

Stock's green shell over the whole ship against refit's hot spot and ring where each shot
lands. Probe 1: an electric sphere sits on the saucer at 2.0–4.5 s (refit) and 1–2 s (stock),
an explosion sprite at 3 s in stock: 5.0–7.8 is clean in both.

```setup
scene hover off; tooltips off
scene orbit target 200 -6 130
```
```action
wait 1
scene glide 8 235 -4 120
wait 11
```

### b04 · Damage
Shows: venting
Scene: venting
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.4
Title: Damage
Sub: Venting plasma that lights the hull
In: 4.0
Length: 3
Status: draft

The Galaxy's saucer venting plasma, the glow at the vent lighting the hull. Probe 1: the plume ran off the top of the frame at 90; pulled back to 120,
and the window is the end of the take, where the plume leans into frame and the blue gas
vents.

```setup
scene hover off; tooltips off
scene orbit galaxy 130 -6 120
```
```action
wait 0.5
scene glide 8 160 -8 110
wait 6.5
```

### b06 · The warbird cloaks
Shows: lighting: cloak
Scene: cloak
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.4
Title: Cloaking
Sub: A cloaked ship as thick glass, bending the light behind it
In: 1.5
Length: 6
Status: draft

The act ends quietly, on the warbird turning to glass in front of the planet. Refit cloaks
at 2.8–4.0 s, stock at 3.4–5.2 s (2026-10-15). Refit's glass was nearly invisible over the
planet then: judge it again on the new probe before keeping the shot.

```setup
scene hover off; tooltips off
scene orbit warbird 230 -6 200
```
```action
wait 1
scene cloak warbird on
wait 11
```

## Beams
Music: Mbor300.wav from 40 gain -2 until n01 fade 1.5

The Borg bed back. The argument: the beams are drawn after the TV series (DS9, Voyager): a
translucent cone of light through vapour, in place of stock's flat textured line. The
beams are faint by design, so they are filmed **against dark, empty space**, never over a
nebula or a planet (the user, 2026-10-17).

### a01 · The assimilation beam
Shows: specials: assimilation beam
Scene: assimilation
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Beams, as in the series
Sub: Assimilation, tractor and holding beams as cones of light through vapour
In: 3.5
Length: 3.8
Status: draft

A Borg cube assimilating the Sovereign, close and a little below, against black space
(the user: the beam is faint, so a dark background). The cone opens from a bright point on
the cube and takes in the saucer. Staged by `films/assimilation`'s as01 (merged from
`worktree-assimilation-scene`): the cube's weapons must be on a tick before the special
fires, and its phaser and a torpedo fire at 1.5–3 s, so the window starts after them.
Stock's beam is a loud fan of green and red streaks; refit's is quieter and lights the
hull. The cone was widest at 3–5 s on the first probe.

```setup
scene orbit 3000 0 3070 150 -10 175
```
```action
wait 0.8
scene weapons borg on
wait 0.2
scene special borg gTechAssimBeam tgt; glide 8 162 -8 160
wait 8
scene weapons borg off
wait 1
```

### a02 · The tractor beam
Shows: specials
Scene: specials
Variants: stock, refit
Layout: wipe
Transition: cut
In: 4.0
Length: 5
Status: draft

An Assault ship holds the Sovereign in its tractor beam; the eye goes from the cone to the
light it throws on the hull. The steps are `testbench/scenarios/specials.md`'s;
`special` must answer `fired`. *Unprobed*: the camera is b05's, and whatever lies behind
3000,0,3110 at that heading must be dark sky: if it isn't, turn the camera. Probe 1: the cone opens at 4.5 s and is best at 5.5–8.5; the Sovereign
leaves the frame after about 9; the ships were small at 280, so closer.

```setup
scene hover off; tooltips off
scene spawn tractor bassault 3000 0 3220 180 1
scene engines tractor off; energy tractor on
wait 3
scene orbit 3000 0 3110 90 -6 200
```
```action
wait 1
scene special tractor gtracbm tgt; glide 8 110 -5 180
wait 11
```

### b05 · The holding beam
Shows: specials
Scene: specials
Variants: stock, refit
Layout: wipe
Transition: cut
In: 4.0
Length: 4
Status: draft

A Borg cube holds the Sovereign in a beam; the eye goes from the beam to the light it
throws on the hull. `special` must answer `fired`. Probe 1: refit's beam read as a thin, flickering line, weaker
than stock's thick one; closer and turned to see more of its length (*unprobed*). If it still
doesn't win, drop it and let a02 carry the beams.

```setup
scene hover off; tooltips off
scene spawn holder bbattle1 3000 0 3220 180 1
wait 3
scene orbit 3000 0 3110 120 -8 180
```
```action
wait 1
scene special holder gholdbm tgt; glide 8 140 -6 160
wait 11
```

## Nebulae
Music: Mfedmp300.wav from 0 gain 1 until m01 fade 1.5

The quiet act, long holds. The argument: stock's nebula is a stack of flat billboards;
refit's is a volume of gas the ships are *in*, lit by the Key light, and the hulls cast
their shadows through it. Both shots are inside the Mutara (`nebula-fleet.ini`, the user's
idea), the camera under and behind the ships looking up towards the Key, so the shadows
run back down through the gas. Stock gets its before here too (the user, 2026-10-17):
a dissolve, since the subject is spread over the frame.

### n01 · Inside the gas
Shows: nebulae, nebulae: shadows
Scene: nebula-fleet.ini
Variants: stock, refit
Layout: dissolve 2.5 1.5
Transition: dissolve 0.8
Title: Nebulae
Sub: A volume of gas the ships fly through, casting their shadows into it
In: 1.0
Length: 9
Status: draft

Up close: the Galaxy alone in frame, the camera under and behind it, gas sliding past
the lens, the hull's shadow a darker shaft running back from it. The Galaxy's shaft read
best on 2026-10-15. Probe 1 (2026-10-17): a mid-wide of six ships, no shafts: the camera looked up along the shafts, which foreshortens them. Now across them, about 90° off the fleet's heading. *Unprobed*. A hull close to the camera made the
shafts vanish then (`orbit sov 48 -24 220`), so it stays at 260 or more; if the shafts
still don't read, this is a bug report for `nebulae/` before it is a framing note.

```setup
scene hover off; shieldfx off; tooltips off
scene goto ente,sov,galaxy,akira,nebula,def1,def2 1700 0 1470
scene orbit galaxy 125 -10 280
wait 4
```
```action
wait 0.5
scene glide 12 140 -8 260
wait 12.5
```

### s05b · The fleet in the Mutara
Shows: nebulae: shadows
Scene: nebula-fleet.ini
Variants: stock, refit
Layout: dissolve 2.5 1.5
Transition: dissolve 0.8
In: 1.0
Length: 7
Status: draft

Wider: the wedge of seven flying into the Key light, the shafts running back through the
gas. Stock's billboards looked punchier from here on 2026-10-15 and refit's Mutara dimmer;
the nebula rework came after, so judge again on the new probe. The formation breaks up
after about 6 s and an Akira crosses the frame: 7 s at most.

```setup
scene hover off; shieldfx off; tooltips off
scene goto ente,sov,galaxy,akira,nebula,def1,def2 1700 0 1470
scene orbit ente 28 -26 400
wait 4
```
```action
wait 0.5
scene glide 13 40 -20 340
wait 13.5
```

## Menus
Music: Mcarmp300.wav from 0 gain 0 until g00 fade 1.0

The film turns from the 3D view to the screens around it. The argument: the menus are new
work, drawn by the UI over a live scene of the game's own ships (render3d's menu scene),
where stock had 800x600 art. The menu shots dissolve: the entries keep stock's click
positions but not its layout. The captions say who made it.

### m01 · The main menu
Kind: mission
Scene: menu.ini
Args: -nointro
Variants: stock, refit
Layout: dissolve 2 1.2
Transition: dissolve 0.6
Title: I redrew the menus
Sub: The same screens, over a live scene of the game's own ships
Audio: none
In: 1.0
Length: 7
Status: draft

The main menu as the player first sees it. The eye goes to the fleet moving over the
planet behind the column of entries, then to the entries themselves. *Unprobed*: stock's
800x600 art, pillarboxed at 16:9. Refit's main menu is one column over the scene where
stock scatters its entries over the art, so this one is a dissolve, not a wipe (probe 1).

```setup
step Wait up to 300 seconds for the main menu
step Move the mouse to 1900,1070
```
```action
wait 9
```

### m02 · Choosing a campaign
Kind: mission
Scene: menu.ini
Args: -nointro
Variants: stock, refit
Layout: dissolve 2 1.2
Transition: dissolve 0.5
Audio: none
In: 1.0
Length: 14.5
Status: draft

The campaign selection: in refit, pointing at each campaign moves the camera to its scene
on the same map: the starbase (training), the Federation fleet, Klingon ships decloaking
before a red nebula, a Borg cube's shadow through the Mutara. **Each entry is hovered in
turn, the camera panning to its scene.** Measured on the probes (frame differences over
the refit takes, 2026-10-17): training's starbase is already up when the screen opens, so
its hover only lights the entry (2 s); the pans to Federation, Klingon and Borg take
1.6, 2.2 and 1.9 s from about 0.4 s after the hover; the Klingon ships are in frame by the
pan's end, the Borg cube and diamond about 4 s after the hover. So 3 s an entry, 2 for
training and about 5 for Borg. Stock is held for two seconds, then dissolves. The pointer
positions are `testbench/scenarios/ui-menus-scene.md`'s (220 × 270 / 322 / 374 / 426 at
1280x720), scaled to the Pointer space. *Unprobed*.

```setup
step Wait up to 300 seconds for the main menu
step Click the single player emblem
wait 3
ptr 330 300 0.1
```
```action
wait 0.5
ptr 330 405 0.6
wait 2
ptr 330 483 0.6
wait 3
ptr 330 561 0.6
wait 3
ptr 330 639 0.6
wait 7.5
```

### m03 · Into warp
Kind: mission
Scene: menu.ini
Args: -nointro
Variants: refit
Layout: full refit
Transition: dissolve 0.5
Audio: game
In: 1.8
Length: 4.5
Status: draft

Refit only: stock goes from the menu to a still loading screen. The Federation campaign's
first mission started from the selection: the fleet goes to warp front first, the camera
swings behind it, and the tunnel comes in under the loading screen. The steps are
`testbench/scenarios/ui-loading-warp.md`'s, without its `Loading.Hold`: a real load. Probe 1: O.K. at 2.0, the loading screen at 2.25, the ships at 3–4, the warp flashes
4.25–5, the tunnel 5.5–6.25, black at 6.4: the window is 1.8–6.3. No caption: at the top it
meets the mission title, at the bottom the progress bar. The warp flashes draw as
hard-edged white squares here: a render3d question, flagged to the user.

```setup
step Wait up to 300 seconds for the main menu
step Click the single player emblem
wait 3
step Click "Federation Campaign"
wait 3
step Click "Invasion"
wait 1
```
```action
wait 0.5
step Click "O.K."
wait 10
```

## The HUD

The same bed. The argument: the HUD is redrawn too, in LCARS and in the colours of the
player's race, with every panel where stock had it, laid out for the screen at any aspect
ratio. One beat here; the deeper look gets a video of its own.

**The race must match in both takes**: the bench's HUD race came up Federation, Borg or
Klingon on different launches (`ui/HUD.md`, "Seen on the bench"), and stock's skin follows
it too. Re-probe a pair until both are the same race, until Scene.asi can pin it.

### u01 · The HUD in battle
Shows: hud
Scene: showcase
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.8
Title: And the HUD
Sub: Every panel where it was, in LCARS, at any aspect ratio
Caption: top
In: 1.0
Length: 7
Status: draft

The showcase battle with the HUD on and the Enterprise selected, so the info panel is
full, the camera high and still as a player's would be. The eye goes to the bottom panels
and the type. *Unprobed*.

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
Scene: showcase
Variants: stock, refit
Layout: wipe
Crop: 0.22 0.44 0.56 0.56
Transition: dissolve 0.5
Caption: top
In: 1.0
Length: 6
Status: draft

The bar and the whole info panel, bottom centre: stock's stretched sprites against
refit's flat icons and live wireframe. Crop from probe 1's frames.

```setup
scene hover off; shieldfx off; tooltips off
scene hud on
scene select ente
scene orbit 2250 0 2350 160 45 1200
```
```action
wait 8
```

### u03 · The minimap, close
Shows: hud: minimap
Scene: showcase
Variants: stock, refit
Layout: wipe
Crop: 0 0.62 0.38 0.38
Transition: dissolve 0.5
Caption: top
In: 1.0
Length: 5
Status: draft

The minimap, bottom left: stock's against refit's, with its filters as words. Crop from
probe 1's frames. A cut candidate.

```setup
scene hover off; shieldfx off; tooltips off
scene hud on
scene select ente
scene orbit 2250 0 2350 160 45 1200
```
```action
wait 7
```

## Gameplay
Music: Mcarmp300.wav from 24 gain -3 until i02 fade 1.5

The argument: the limits that made Armada II fiddly (sixteen ships a selection, one
shipyard at a time, a menu tree under every button) are gone. (Whether a stock player can
join a game with them is still open in `qol/README.md`, so the film doesn't claim it.) Paced by the pointer: every press gets a key hint and enough time to
follow. Only g01 has a before: stock has nothing to compare for the rest. Captions at the
top, over the HUD.

### g00 · Card: it plays better
Length: 3
Transition: dip 0.5
Background: h01

```card
[accent] AND IT PLAYS BETTER
The limits I kept running into, gone
```

### g01 · Selections beyond sixteen
Shows: qol: selections
Scene: qol.ini
Variants: stock, refit
Layout: dissolve 2.5 1.2
Transition: dissolve 0.5
Title: Select the whole fleet
Sub: Twenty-four ships in one selection, where stock stopped at sixteen
Caption: top
In: 1.0
Length: 7
Status: draft

Twenty-four destroyers dragged into one selection. The eye counts the panel: two rows of
eight in stock, three in refit (probe 1: the command panel hides part of the block, so the
rings don't count cleanly). The pointer leaves the block after the drag so no tooltip
comes up. On 2026-10-15 refit's old panel showed 16 icons
even with 24 selected; the UI's panel is new, so check it on the probe.

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
Scene: qol.ini
Variants: refit
Layout: full refit
Transition: dissolve 0.4
Title: Shift+number adds to a group
Caption: top
Keys: auto
In: 1.0
Length: 15
Status: draft

Half the block to group 1, the other half added with Shift+1, then 1 calls up all
twenty-four.

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

### g03 · Three shipyards, one build menu
Shows: qol: station groups
Scene: qol.ini
Variants: refit
Layout: full refit
Transition: dissolve 0.4
Title: Stations in groups
Sub: One build menu for every selected shipyard; the work spread across them
Caption: top
Keys: auto
In: 1.0
Length: 10
Status: draft

Three shipyards selected together; seven presses of Q queue destroyers across all three.
The eye goes to the three build bars filling. Probe 1 measured the yards at x 690–1460, y 150–280 (Pointer space): the box covers them now,
clear of the resource bar above. *Unprobed*.

```setup
lclick 400 640
wait 1
```
```action
wait 1
drag 640 120 1500 340 1.4
ptr 1700 300 0.4
wait 0.8
press q
wait 1.2
press q
wait 0.5
press q
wait 0.5
press q
wait 0.5
press q
wait 0.5
press q
wait 0.5
press q
wait 6
```

### g04 · The grid keys
Shows: grid
Scene: qol.ini
Variants: refit
Layout: full refit
Crop: 0.26 0.42 0.30 0.30
Transition: dissolve 0.5
Title: Grid hotkeys
Sub: Every button on a block of keys, the same place every time
Caption: top
Keys: auto
In: 3.5
Length: 17
Status: draft

The close crop of the button bar, held long enough to follow: Q opens a menu, G goes back,
W and E the next ones. The user's note from October: about 18 s worked, 5 s was far too
short; trim no further. The UI moved the command grid (x 0.29–0.47, y 0.47–0.66, probe 1); the crop follows it. Presses at
5.6 (Q), 8.6, 10.2 (W), 13.2, 14.8 (E), 17.8.

```setup
lclick 400 640
wait 1
```
```action
wait 1
drag 830 430 1350 745 1.2
wait 2.5
press q
wait 3
press g
wait 1.5
press w
wait 3
press g
wait 1.5
press e
wait 3
press g
wait 5
```

### g05 · Long moves at warp
Shows: qol: warp
Scene: warp
Variants: refit
Layout: full refit
Transition: dissolve 0.4
Title: Long moves go to warp
Sub: Order a ship across the map and it jumps
Caption: top
In: 0.5
Length: 10
Status: draft

Probe 1: no warp at four zoom steps (the move was shorter than `WarpDistance`): zoomed out
fully, as `scenarios/qol-warp.md` does (30 steps), and the pointer leaves the ships after
the click. Zoomed out, three destroyers selected, the pointer far to the right: the cursor turns to
the warp ring, a right-click, and they jump.

```setup
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
press ctrl+Down
wait 0.1
wait 1
scene select ship1 ship2 ship3
ptr 1150 540 0.3
```
```action
wait 0.5
ptr 1850 540 1.2
wait 1.0
click 1850 540 --button 3
ptr 960 250 0.6
wait 9.5
```

## Installing
Music: Mfed300.wav from 60 gain -6

Short and calm: where to get it, the installer, the game. Every step on screen long enough
to read twice. The window is the GUI installer filmed on a bare desktop (`Kind: desktop`),
installing into a stock copy of the game, so the last shot is the honest result.

### i02 · Get it
Length: 6
Transition: dip 0.6

A card, not a browser: a page filmed today dates the video.

```card
[accent] INSTALLING
Your own copy from GOG, then the installer:
github.com/cedrichaase/armada2-refit/releases
[small] Linux: AppImage · Windows: .exe · macOS: .app
```

### i03 · Run it
Kind: desktop
Run: python3 {repo}/gui-installer/armada2-refit-installer.py
Wallpaper: s05b refit 5
Layout: full
Transition: dissolve 0.5
Title: Run the installer
Sub: It finds the game on its own
Audio: none
In: 0.5
Length: 7
Status: draft

The window opening on the desktop: the sky and the nebula fade in, the title settles. The
eye lands on the game row: it found Armada II in Heroic by itself. The final take runs the
release's AppImage; the probe runs the checkout's script, the same window.

```action
wait 0.5
run
wait 7.5
```

### i05 · Install
Kind: desktop
Run: python3 {repo}/gui-installer/armada2-refit-installer.py
Wallpaper: s05b refit 5
Layout: full
Transition: dissolve 0.4
Title: One button
Sub: The launch settings go into Heroic for you
Audio: none
In: 0.5
Length: 6
Status: draft

Install pressed, the sky going to warp, the engine glow riding the progress bar, the
status line naming each layer. The probe ran the checkout's 1.5.0 script, which shows "A newer installer (1.6.0) is
available" throughout: the final take must run the release's AppImage. The install is real and takes about 40 s: this shot is its
start, i05b its end, one take.

```setup
run
wait 8
ptr 960 960 0.1
```
```action
wait 0.5
ptr 960 684 0.8
lclick
ptr 960 960 0.6
wait 52
```

### i05b · Done
Kind: footage
From: i05
In: 31.3
Length: 5
Transition: dissolve 0.8
Audio: none

The end of the same take: the bar full, "Refit installed. Heroic launch settings
written.", Launch game lit amber. Probe 1: "installed" settles at 31.0.

### i06 · Play
Kind: mission
Scene: menu.ini
Args: -nointro
Variants: refit
Layout: full refit
Transition: dissolve 0.8
Audio: none
In: 1.0
Length: 6
Status: draft

The game's main menu as the installer leaves it, the fleet over the planet: the honest end
of the guide. (The black sides of the old board are gone: the menu scene fills the
screen.)

```setup
step Wait up to 300 seconds for the main menu
step Move the mouse to 1900,1070
```
```action
wait 8
```

### i08 · Good to know
Length: 6
Transition: dissolve 0.5

What the install does not do, said once and plainly, and the way back out.

```card
[accent] GOOD TO KNOW
Uninstall takes it back out
Windows: SmartScreen → More info → Run anyway · macOS: Privacy & Security → Open Anyway
[small] Remastered textures are built from your own game files: see the README
```

## End
Music: Mfedmp300.wav from 0 gain 0

### z01 · End card
Length: 6
Transition: dip 0.6
Background: h01

The open door, not a sales line (the brand's ending): a question the comments can answer.

```card
[big] ARMADA II
[accent] R E F I T
github.com/cedrichaase/armada2-refit
[small] What would you fix next? Tell me in the comments.
```
