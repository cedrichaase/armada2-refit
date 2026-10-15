# Armada II Refit
Film: promo
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --assets none --install . --install testbench/scene

The promo: the one video that has to make someone who loved Armada II in 2001 install
it again tonight. The two showcases before it argued *what changed* to people who already
follow the project; this one argues **why to play it now**, to people who don't.

The argument, in the order the film makes it:

1. **It is the game you remember, redrawn.** Same ships, same maps, same 2001 engine,
   frame for frame the same game: the picture is what changed. Every comparison is stock
   and refit from one shot list, so the viewer never has to take our word for it.
2. **Every part of the picture.** Hulls and light, then battle, then space itself, then
   the screen around it (widescreen menus and HUD). Visuals carry the first three minutes.
3. **It plays better too, and still plays with everyone.** Quality of life that stock
   players can play against, and online play that works under Wine and Proton.
4. **It takes a minute to install.** A step-by-step guide through the graphical
   installer, ending on the game's own main menu, refitted.

About 4:20 by the shot lengths, a little less once transitions overlap. Long for a promo; `PLAN.md` (D5) proposes where to cut to
about 3:30, after the animatic shows what drags.
`PLAN.md` beside this board holds the open decisions and the tooling the board needs
before every shot can be filmed; read it first.

For the editor:

- The house style holds (`films/README.md`): close-ups, the camera a little below the
  subject looking up, a wipe for one centred subject, a dissolve for subjects spread over
  the frame and for UI, no game music in any take, one music bed at a time.
- **Pace rises and falls by act**: quick cuts on the music in the cold open and in battle;
  long holds in space; the gameplay act is paced by the pointer, not the music; the
  install guide is calm and legible, every step on screen long enough to read twice.
- **"Refit" is the game as a release installs it**, once the new effects are in the
  installer: every code layer, and none of `$A2_DATA`'s assets — no remastered textures,
  no menu backdrop plates, no movies (`--assets none`). The film is published when the
  installer installs everything it shows. The thumbnail is the one exception: it uses
  the remastered textures.
- Lower-third titles name a feature once, the first time it is on screen. No act cards
  except where the film turns (gameplay, installing): the promo should not feel like a
  chaptered changelog.
- **g06 is two games side by side** (`Players: host, joiner`): the chat shows the
  machine's LAN address ("Local IP Address: ..."); blur it in the cut before publishing.
- Camera values copied from the October showcase (`$A2_DATA/promo/2026-10-showcase/
  tools/*.shots`) and from `films/whats-new` were checked on the bench; values marked
  *unprobed* in a shot's prose are first guesses.

## Cold open
Music: Mbor300.wav from 0 gain -1 until h01 fade 1.2

Fifteen seconds: the game as it shipped, then what it is now. Opens on stock — the
Enterprise-E from h01's camera, flat and dated — holds just long enough to be
recognised, then hard cuts to the same moment in refit on the Borg bed's first hit, and
the title. Every shot here is cut from other shots' takes.

### o01 · Stock, as it shipped
Kind: footage
From: h03 stock
In: 3.0
Length: 3
Transition: fade 1.0
Audio: none
Caption: top
Title: 2001

Stock's Enterprise, the same frame o02 cuts to in refit. If h01's stock take shows the
hull ghostly (see h01), take it from h03's stock instead and o02 from h03's refit.

Probe review 2026-10-15: h01's stock hull is ghostly near the camera, so o01/o02 come from h03.


### o02 · Teaser: the Enterprise
Kind: footage
From: h03 refit
In: 3.0
Length: 2.5
Audio: none

Hard cut on the bed's first hit: the same frame as o01, refit. The change is the whole
point; no transition.

### o03 · Teaser: the fireball
Kind: footage
From: b01 refit
In: 1.7
Length: 1.2
Transition: cut
Audio: game

The cruiser's fireball at its height. The game's sound of the blast lands on the cut.

### o04 · Teaser: into the nebula
Kind: footage
From: s05b refit
In: 2.0
Length: 2.4
Transition: dissolve 0.3
Audio: none

Gas sliding past the lens.

### o05 · Title
Length: 4.5
Transition: dip 0.5
Background: s05

```card
[big] ARMADA II
[accent] R E F I T
[small] The 2001 classic, redrawn. Free and open source.
```

## Hulls and light
Music: Mfed300.wav from 0 gain 1 until b01 fade 1.5

The argument: the same models, lit the way a modern engine lights them — per-pixel
light from the scene's sun, shadows the hulls cast on themselves, weapons and nebulae
that light what is near them, planets with a night side. Federation music, unhurried.
Every shot a wipe: one hull, centred.

### h01 · The Enterprise
Shows: lighting, textures
Scene: showcase
Variants: stock, refit
Layout: wipe
Transition: dissolve 1.0
Title: Per-pixel lighting and shadows
Sub: The same models, lit by the scene's own sun
In: 0.5
Length: 8
Status: draft

Close and low under the Enterprise-E's saucer, sliding along it. The eye lands on the
saucer's rim, where refit's key light falls off into shadow and stock is flat. The Borg
in this scene beam boarding parties over: if a transporter effect drifts into frame,
move `In:` rather than keep it. From the October showcase (`PFX-l5-ente-close`).
The October stock take of this camera shows the hull dark and half transparent (stock
fading hulls near the camera?): check the stock probe first; if it repeats, pull the
camera back until stock draws the hull solid, or this wipe looks broken, not old.

Probe review 2026-10-15: pulled back from 105 to 170: stock fades a hull within about 150.

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

### h02 · The Borg cube
Shows: lighting: bump-mapped hulls
Scene: showcase
Variants: stock, refit
Layout: wipe
Transition: cut
In: 0.5
Length: 6.5
Status: draft

From under the cube, turning. The eye lands on the plating's relief and the night
lights in it (the Borg have their own light profile, dark plating). From the October
showcase (`PFX-l2-cube`).

```setup
step Move the mouse to 1900,8
scene hover off; shieldfx off; tooltips off
scene orbit cube1 25 -12 180
```
```action
wait 1
scene glide 12 -35 -10 230
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
Length: 7
Status: draft

The one shot from above in this act, on purpose: the Enterprise's phaser from above
lights its own hull, and the light on the plating is the subject. From the October
showcase (`PFX-l1-ente-phaser`). If the beam does not fire inside the window, move `In:`.

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

### h04 · Klingon disruptors
Shows: lighting: weapons
Scene: factions
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.4
In: 1.0
Length: 6
Status: draft

The Klingon battleship's torpedoes leaving the tubes, the red light running down its
hull. From the October showcase (`PFX-f1-kli`); lower than it was (pitch 12 → -6),
*unprobed*.

Probe review 2026-10-15: closer (300 → 130); the battleship fires disruptor beams, not torpedoes, in this window: the prose is about beams now, and `In:` wants a volley. A cut candidate (D5).

```setup
step Move the mouse to 1900,8
scene hover off; shieldfx off; tooltips off
scene orbit kli 90 -6 130
```
```action
wait 1
scene glide 10 40 -3 120
wait 7.5
```

### h05 · Colony at night
Shows: lighting: planets, colonies
Scene: colony
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.8
Title: Planets with a night side
Sub: City lights on the colonies you build
In: 2.0
Length: 8
Status: draft

The fleet backs into the planet's shadow, then leaves it while the camera swings from
the terminator in to the ships: day side, night side, city lights, then the hulls coming
out of the dark into the sun. A long shot that ends the act; let it breathe. From the
October showcase (`PFX-c1-shadow`).

```setup
step Move the mouse to 1900,8
scene hover off; shieldfx off; tooltips off
scene goto ente,sov,galaxy,cruiser 2200 0 2390
wait 16
scene goto ente,sov,galaxy,cruiser 2350 10 2250
wait 13
scene orbit 2420 0 2200 115 -3 760
```
```action
wait 3
scene goto ente,sov,galaxy,cruiser 2560 0 1990
scene glide 16 166 3 470
wait 15
```

## Battle
Music: Mkli300.wav from 0 gain 0 until s01 fade 1.5

The loud act, Klingon music, quick cuts. The argument: a battle in stock Armada II is
flipbooks — a sprite sheet per fireball, a disc per shockwave, a green shell per shield
hit; refit draws each as light and gas in the scene, touching the hulls around it. The
shots are `films/whats-new`'s, trimmed: here each effect gets one beat, not two.

### b01 · A cruiser explodes
Shows: explosions
Scene: explosion
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Explosions
Sub: Ray-marched fire and smoke, lighting the ships around it
In: 1.3
Length: 3.5
Status: draft

The fireball's core: stock's sprite turns with the camera; refit's is a volume the camera
moves around, and the flash lights the Galaxy and the Sovereign beside it. The cut into
this shot lands on the act's first beat of music.

Probe review 2026-10-15: the fireball peaks at 2.0–2.6 s; from 2.7 s stock's shockwave disc fills the frame in both variants, so the shot ends before it.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit 2090 0 2230 80 -6 150
```
```action
wait 1
scene kill mid; glide 7 110 -4 140
wait 7
```

### b02 · Shield hits
Shows: shields
Scene: shields
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Shields
Sub: Each hit lights the shield where it lands
In: 1.0
Length: 6
Status: draft

Stock's green shell over the whole ship against refit's hot spot and ring where each
shot lands.

```setup
scene hover off; tooltips off
scene orbit target 200 -6 130
```
```action
wait 1
scene glide 8 235 -4 120
wait 7
```

### b04 · Damage
Shows: venting
Scene: venting
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.4
Title: Damage and repairs
Sub: Venting plasma and welding arcs that light the hull
In: 0.5
Length: 6
Status: draft

The Galaxy's saucer venting plasma, the glow at the vent lighting the hull. Repairs
(`whats-new` r01) are left out for pace; the subtitle names them.

Probe review 2026-10-15: the vent is on the saucer, not a nacelle, and was tiny at 200: closer (90). *Unprobed* aim.

```setup
scene hover off; tooltips off
scene orbit galaxy 130 -6 90
```
```action
wait 0.5
scene glide 8 160 -8 85
wait 6.5
```

### b05 · The holding beam
Shows: specials
Scene: specials
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Special weapons
Sub: Beams, bolts and waves of light
In: 1.5
Length: 6
Status: draft

A Borg cube holds the Sovereign in a beam; the eye goes from the beam to the light it
throws on the hull. `special` must answer `fired`.

```setup
scene hover off; tooltips off
scene spawn holder bbattle1 3000 0 3220 180 1
wait 3
scene orbit 3000 0 3110 90 -6 280
```
```action
wait 1
scene special holder gholdbm tgt; glide 8 110 -5 250
wait 7
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

The act ends quietly, on the warbird turning to glass in front of the planet. The bed
fades over it, into the space act.

Probe review 2026-10-15: refit cloaks at 2.8–4.0 s, stock at 3.4–5.2 s. Refit's glass is nearly invisible over the planet: a question for `lighting/`.

```setup
scene hover off; tooltips off
scene orbit warbird 230 -6 260
```
```action
wait 1
scene cloak warbird on
wait 8
```

### b07 · Glass
Shows: lighting: cloak
Scene: cloak
Variants: refit
Layout: full refit
In: 1.0
Length: 6
Status: draft

Refit alone, both ships fully cloaked, the camera sliding past so the planet bends
through them. No stock counterpart: stock shows nothing.

Probe review 2026-10-15: from b06's side, so the planet is behind the glass (at 300 it was sky).

```setup
scene hover off; tooltips off
scene cloak warbird now; cloak bop now
scene orbit warbird 215 -8 200
```
```action
wait 1
scene glide 6 245 -6 180
wait 6.5
```

## Space
Music: Mfedmp300.wav from 0 gain 1 until u02 fade 1.5

The quiet act, slow cuts and long holds. The argument: planets, moons, rocks and the sky
were textures on low-polygon shapes; now they are surfaces with relief, oceans, ice and
cloud, nebulae you fly through, and a sky with depth.

### s01 · A class M world
Shows: surfaces, models: planet tessellation
Scene: planets
Variants: stock, refit
Layout: wipe
Transition: dissolve 1.0
Title: Planets, moons and asteroids
Sub: Surfaces with relief, oceans, ice and cloud
In: 0.5
Length: 8
Status: draft

The class M planet filling two thirds of the frame, turning slowly. The wipe crosses the
limb, where stock's facets show.

Probe review 2026-10-15: turned about 40° towards the lit limb: refit's planet was mostly night side.

```setup
scene hover off; tooltips off
scene orbit m 340 -6 700
```
```action
wait 0.5
scene glide 9 370 -4 640
wait 8.5
```

### s02 · Six classes
Shows: surfaces
Scene: planets
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: dissolve 0.6
In: 0.5
Length: 7
Status: draft

The row of D, H, J, K, L and M: the variety is the point. One of the few wide shots.

```setup
scene hover off; tooltips off
scene orbit 2500 0 2150 200 -8 1800
```
```action
wait 0.5
scene glide 8 215 -6 1650
wait 7.5
```

### s03 · The dilithium moon
Shows: surfaces: moon, models: moon smoothing
Scene: planets
Variants: stock, refit
Layout: wipe
Transition: cut
In: 0.5
Length: 5.5
Status: draft

Close on the moon, the limb in frame: the glow shows where a crack opens at the
silhouette.

Probe review 2026-10-15: closer (130) and from the other side (heading 20, *unprobed*), so the moon sits against black sky and its silhouette glow reads.

```setup
scene hover off; tooltips off
scene orbit moon 20 -8 130
```
```action
wait 0.5
scene glide 7 45 -6 120
wait 6
```

### s04 · Asteroids
Shows: surfaces: asteroids, models: asteroid smoothing
Scene: asteroids
Variants: refit
Layout: full refit
Transition: dissolve 0.6
In: 0.5
Length: 6
Status: draft

Refit alone: which rocks a session draws is random, so stock and refit would show
different fields and the comparison would lie. The Galaxy beyond the field for scale.

Probe review 2026-10-15: the Galaxy was out of frame; from the other side (30) at 500, a guess.

```setup
scene hover off; tooltips off
scene orbit big 30 -8 260
```
```action
wait 0.5
scene spin 4
wait 6.5
```

### s05 · Into the nebula
Shows: nebulae
Scene: nebula
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: dissolve 0.8
Title: Nebulae
Sub: A volume of gas you fly through, lit by the ships inside it
In: 0.5
Length: 10
Status: draft

The camera pushes in from outside the Mutara nebula towards the Galaxy at its edge.
Hold: the parallax between layers of gas is what sells the volume.

Probe review 2026-10-15: refit's gas reads dim and murky beside stock's billboards here, so the dissolve argues for stock. Either s05b replaces it or s05 needs a brighter, closer view of the gas.

```setup
scene hover off; tooltips off
scene orbit nebula 200 -5 900
```
```action
wait 0.5
scene glide 10 230 -3 420
wait 10
```

### s05b · The fleet in the Mutara nebula
Shows: nebulae: shadows
Scene: nebula-fleet.ini
Variants: refit
Layout: full refit
Transition: dissolve 0.8
In: 1.0
Length: 7
Status: draft

The user's idea: a fleet inside the dense purple Mutara, flying towards the Key, the
camera under and behind it looking up towards the light, so the hulls' shadows run back
down through the gas. Refit only: stock has no shadows, and its billboards look punchier
from here. The shafts are faint (only the Galaxy's reads clearly, lower left, early);
stronger gas shadows are a question for `nebulae/`. A hull near the camera can make the
shafts vanish (seen paused at `orbit sov 48 -24 220`), a bug report for `nebulae/`.
Found by a 7-angle probe (2026-10-14, n05).

Probe review 2026-10-15: the formation breaks up after about 6 s and an Akira crosses the frame: 7 s.

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

### s06 · The sky
Shows: sky
Scene: planet
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.6
Title: The sky
Sub: Computed for every direction, not six painted faces
In: 0.5
Length: 7
Status: draft

The camera tilted up and turning slowly, the Galaxy small at the bottom for scale. The
wipe crosses open sky, where stock's cube faces show.

Probe review 2026-10-15: turned 180°: the planet filled half the frame and hid the sky.

```setup
scene hover off; tooltips off
scene orbit ship 50 -25 450
```
```action
wait 0.5
scene spin 3
wait 7
```

## Widescreen
Music: Mcarmp300.wav from 0 gain 0 until g00 fade 1.0

A beat on the screen around the picture: the game was made for 800x600 and stretched
everything to fit anything wider. Refit lays the HUD (and the menus) out for the screen
they are on, at any aspect ratio. The menus are not shown: without the backdrop plates,
which no release carries, they stand in black sides. A dissolve: UI is spread over the
frame.

### u02 · The HUD in battle
Shows: hud, grid
Scene: showcase
Variants: stock, refit
Layout: dissolve 2.5 1.2
Transition: dissolve 0.8
Title: Widescreen HUD
Sub: Laid out for your screen, at any aspect ratio
Caption: top
In: 1.0
Length: 7
Status: draft

The showcase battle with the HUD on, the camera high and still as a player's would be:
stock's panels stretched and its font squashed against refit's laid out at 16:9. The
eye goes to the bottom panels and the font. *Unprobed*: the orbit is a guess at an
RTS camera.

Probe review 2026-10-15: BUG: stock's HUD is the purple skin and refit's the green one, and the skin differs between takes (g01 gold, g02 green, g03/g05 purple). The player's race has to be pinned before this compares like with like (`PLAN.md`).

```setup
scene hover off; shieldfx off; tooltips off
scene hud on
scene orbit 2250 0 2350 160 45 1200
```
```action
wait 9
```

## Gameplay
Music: Mcarmp300.wav from 24 gain -3 until i00 fade 1.5

The film turns: it plays better, too. The argument: the limits that made Armada II
fiddly (sixteen ships a selection, one shipyard at a time, a menu tree under every
button) are gone, **and none of it locks out stock players** — everything here plays
against someone without the refit, except online play, which needs it on both sides.
Paced by the pointer: every press gets a key hint and enough time to follow. The HUD
shots are refit only (stock has nothing to compare), cropped to where the action is,
captions at the top. The pointer coordinates are for 1920x1080 (`PLAN.md` T2 scales
them to the take).

### g00 · Card: it plays better
Length: 3
Transition: dip 0.5
Background: h05

```card
[accent] AND IT PLAYS BETTER
Quality of life that still plays with stock players
```

### g01 · Selections beyond sixteen
Shows: qol: selections
Scene: qol.ini
Variants: refit
Layout: full refit
Transition: dissolve 0.5
Title: Select the whole fleet
Sub: All 24 selected: rings on every ship, beyond the panel's 16 icons
Caption: top
In: 1.0
Length: 5
Status: draft

Twenty-four destroyers dragged into one selection: stock stops at sixteen. From the
October showcase (`PFX-q1-select`).

Probe review 2026-10-15: all 24 show selection rings, but the panel shows 16 icons, which reads as "stops at 16": crop to the block, or say the count in the Sub.

```setup
ptr 400 640 0.1
wait 2
```
```action
wait 1.5
drag 830 430 1350 745 1.6
wait 4.5
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
twenty-four. From the October showcase (`PFX-q2-groups`).

Probe review 2026-10-15: a drag takes about 2.1 s on the bench, so `press 1` lands at 12.9 s: 15 s.

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
The eye goes to the three build bars filling. From the October showcase (`PFX-q3-yards`).

Probe review 2026-10-15: the drag missed the yards (they sit at about y 340–410).

```setup
lclick 400 640
wait 1
```
```action
wait 1
drag 780 300 1250 450 1.4
wait 1.2
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
Crop: 0.15 0.64 0.43 0.36
Transition: dissolve 0.5
Title: Grid hotkeys
Sub: Every button on a 3x3 block of keys, the same place every time
Caption: top
Keys: auto
In: 2.5
Length: 17.5
Status: draft

The close crop of the button bar, held long enough to follow: Q opens a menu, G goes
back, W and E the next ones. The user's note from October: about 18 s worked, 5 s was
far too short; this is 15, trim no further. From the October showcase (`PFX-q4-grid`).

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

Zoomed out, three destroyers selected, the pointer far to the right: the cursor turns to
the warp ring, a right-click, and they jump. *Unprobed*: the steps are
`scenarios/qol-warp.md`'s.

Probe review 2026-10-15: no jump before 7.4 s: zoomed out four steps, not six, and 10 s.

```setup
press ctrl+Down
wait 0.2
press ctrl+Down
wait 0.2
press ctrl+Down
wait 0.2
press ctrl+Down
wait 1
scene select ship1 ship2 ship3
ptr 1150 540 0.3
```
```action
wait 0.5
ptr 1850 540 1.2
wait 1.0
click 1850 540 --button 3
wait 10
```

### g06 · Online, under Wine and Proton
Kind: mission
Players: host, joiner
Scene: menu.ini
Prepare: online/bench-asi.sh --server
Args: -nointro
Layout: split 0.75
Caption: top
Title: Online play that works under Wine and Proton
Sub: Host a game, share a code, no port forwarding
Transition: dissolve 0.5
In: 1.0
Length: 16
Speed: 1.5

Two games side by side, the host on the left. The host is already in its game setup,
its join code in the chat; the joiner opens *Internet – Online*, types the code, finds
the host's game and joins, and each says hello. The steps are
`testbench/scenarios/multiplayer-online-code.md`'s, through a local `a2online-server`
(`Prepare:`). The eye should land on the code: shown on the left, typed on the right.
The take runs about 26 s; `In: 1.0` at `Speed: 1.5` shows 24 s of it in 16, from the
joiner's first click to both hellos. The editor may cut the game list's wait instead.

```setup
step Wait for the main menu
step Click the multiplayer emblem
step Wait for the multiplayer connection screen
step Click at design 400,187
step Press End
step Press BackSpace 32 times
host: step Type "host"
joiner: step Type "joiner"
host: step Click "Internet - Online"
host: step Wait up to 10 seconds for "Join code or host address"
host: step Click "O.K."
host: step Wait up to 30 seconds for "Create Game"
host: step Click "Create Game"
host: wait 3
host: step Click "O.K."
host: step Wait up to 30 seconds for "GAME SETUP"
host: step Wait up to 20 seconds for "Join code"
```
```action
wait 1
joiner: step Click "Internet - Online"
joiner: step Wait up to 10 seconds for "Join code or host address"
joiner: step Click at design 400,292
joiner: typelog host Online.log "join code"
wait 0.6
joiner: step Click "O.K."
joiner: step Wait up to 30 seconds for "host's Game"
wait 0.8
joiner: step Click "host's Game"
joiner: step Click "Join Game"
joiner: step Wait up to 30 seconds for "GAME SETUP"
joiner: step Click at design 155,547
joiner: step Type "hello from joiner"
joiner: step Press Return
host: step Wait up to 20 seconds for "hello from joiner"
host: step Click at design 155,547
host: step Type "hello from host"
host: step Press Return
wait 3
```

## Installing
Music: Mfed300.wav from 60 gain -6

The guide: calm, legible, every step on screen long enough to read twice, each numbered
the same way. The window is the GUI installer filmed on a bare desktop (`Kind: desktop`),
installing a real release into a stock copy of the game, so that the last shot — the
game launched after it — is the honest result. Linux first, in full; Windows and macOS
in one step each, since the window is the same. What the install leaves out (textures)
is said plainly, not hidden (`PLAN.md`, decision D1).

### i00 · Card: get it
Length: 3
Transition: dip 0.6
Background: s05

```card
[accent] INSTALLING
In about a minute
```

### i01 · Step 1: the game
Length: 5
Transition: dissolve 0.5

The prerequisite, on a card: their own copy from GOG, installed. On Linux through
Heroic (the installer finds it there); on Windows GOG's own installer; on macOS in
CrossOver or another Wine.

```card
[accent] 1 · YOUR OWN COPY
Star Trek: Armada II from GOG, installed
[small] Linux: through Heroic · Windows: GOG Galaxy or the offline installer · macOS: CrossOver
```

### i02 · Step 2: download the installer
Length: 6
Transition: dissolve 0.5

A card, not a browser: a page filmed today dates the video. The three files, by name,
under the repository's address.

```card
[accent] 2 · DOWNLOAD THE INSTALLER
github.com/cedrichaase/armada2-refit/releases
[small] Linux: armada2-refit-installer-x86_64.AppImage
[small] Windows: armada2-refit-installer-windows.exe
[small] macOS: armada2-refit-installer-macos.zip
```

### i03 · Step 3: run it
Kind: desktop
Run: python3 {repo}/gui-installer/armada2-refit-installer.py
Wallpaper: h05 refit 9
Layout: full
Transition: dissolve 0.5
Title: 3 · Run it
Sub: It finds the game on its own
Audio: none
In: 0.5
Length: 7
Status: draft
The desktop, the window opening on it: the sky and the nebula fade in, the title
settles. The eye lands on the game row: it found Armada II in Heroic by itself. For
terminal users, the caption could carry `chmod +x armada2-refit-installer-x86_64.AppImage`.
The final take runs the release's AppImage, once the release carries the new effects;
the probe runs the checkout's script, the same window.

```action
wait 0.5
run
wait 7.5
```

### i04 · Step 4: check what goes in
Kind: desktop
Run: python3 {repo}/gui-installer/armada2-refit-installer.py
Wallpaper: h05 refit 9
Layout: full
Transition: dissolve 0.4
Title: 4 · Check what goes in
Sub: The newest release is picked for you; each line says what installs here
Audio: none
In: 0.5
Length: 7
Status: draft
The game row, the version picker and the line under them, which says what goes into
Heroic. The pointer travels down them: the game it found, the newest release picked for
you, the launch settings. Coordinates from the desktop test (2026-10-14): the window
sits in the middle of a 1600x900 desktop, the game row at y 410, the picker at
1044,489, the Heroic line at y 560, Install at 960,684 (Pointer space, 1920x1080).

Probe review 2026-10-15: BUG in the installer: the version line truncates ("pre-release, not yet seen i…").

```setup
run
wait 6
ptr 960 960 0.1
```
```action
wait 0.5
ptr 700 410 1.2
wait 1.5
ptr 1044 489 1.0
wait 1.5
ptr 700 560 1.0
wait 4.5
```

### i05 · Step 5: install
Kind: desktop
Run: python3 {repo}/gui-installer/armada2-refit-installer.py
Wallpaper: h05 refit 9
Layout: full
Transition: dissolve 0.4
Title: 5 · Install
Sub: One button. The launch settings go into Heroic for you
Audio: none
In: 0.5
Length: 6
Status: draft
The installer's best moment: Install pressed, the sky going to warp, the engine glow
riding the progress bar, the status line naming each layer ("Refitting the HUD",
"Installing lighting"…), and the bar full. Hold one beat on "done". Real speed: the
install is real, into the take's own stock copy of the game, and takes about 40 s,
most of it on "Rewriting the models…": this shot is its start, i05b its end, one take.

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

### i05b · Step 5: done
Kind: footage
From: i05
In: 29.5
Length: 5
Transition: dissolve 0.8
Audio: none

The end of the same take, about 28 s on: the bar full, "Refit installed.
Heroic launch settings written.", Launch game lit amber. Move `In:` to land a beat after
the bar fills.

### i06 · Step 6: play
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

The game's main menu, refitted, as the installer leaves it: the honest end of the guide.
With no backdrop plate the menu stands in black sides at 16:9; if that reads as broken,
end the guide on i05's "done" and a cut to a refit game shot instead.

```setup
step Wait up to 300 seconds for the main menu
step Move the mouse to 1900,1070
```
```action
wait 8
```

### i07 · Windows and macOS
Length: 6
Transition: dissolve 0.5

A card until the Tk window can be filmed: it should be a desktop shot of the window the
`.exe` and the `.app` show, run on Linux with `A2_INSTALLER_TK=1` (`Run:
A2_INSTALLER_TK=1 python3 {repo}/gui-installer/armada2-refit-installer.py`), which needs
Tk on the filming machine (`pacman -S tk`; this one has none, 2026-10-14).

```card
[accent] WINDOWS AND MACOS
The same window, the same steps
[small] Windows: if SmartScreen holds it, More info → Run anyway
[small] macOS: System Settings → Privacy & Security → Open Anyway
```

### i08 · Good to know
Length: 6
Transition: dissolve 0.5

What the install does not do, said once and plainly, and the way back out.

```card
[accent] GOOD TO KNOW
Uninstall puts the game back as it was
Online play works with anyone else on the refit
[small] Remastered textures are built from your own game files: see the README
```

## End
Music: Mfedmp300.wav from 0 gain 0

### z01 · End card
Length: 6
Transition: dip 0.6
Background: h01

```card
[big] ARMADA II
[accent] R E F I T
github.com/cedrichaase/armada2-refit
[small] Free and open source. Bring your own copy of the game.
```
