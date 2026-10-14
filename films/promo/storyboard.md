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
- **g06 is a `PLACEHOLDER` card** standing in for footage the bench cannot film yet (two
  games online, `PLAN.md` T4). Its prose says what the real shot is; its length is the
  real shot's.
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
From: h01 stock
In: 3.0
Length: 3
Transition: fade 1.0
Audio: none
Caption: top
Title: 2001

Stock's Enterprise, the same frame o02 cuts to in refit. If h01's stock take shows the
hull ghostly (see h01), take it from h03's stock instead and o02 from h03's refit.

### o02 · Teaser: the Enterprise
Kind: footage
From: h01 refit
In: 3.0
Length: 2.5
Audio: none

Hard cut on the bed's first hit: the same frame as o01, refit. The change is the whole
point; no transition.

### o03 · Teaser: the fireball
Kind: footage
From: b01 refit
In: 1.6
Length: 2.2
Transition: cut
Audio: game

The cruiser's fireball at its height. The game's sound of the blast lands on the cut.

### o04 · Teaser: into the nebula
Kind: footage
From: s05 refit
In: 4.0
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

```setup
step Move the mouse to 1900,8
scene hover off; shieldfx off; tooltips off
scene orbit ente 300 -12 105
```
```action
wait 1
scene glide 10 380 -5 135
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
scene orbit cube1 25 -11 230
```
```action
wait 1
scene glide 12 -35 -5 290
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

### h04 · Klingon torpedoes
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

```setup
step Move the mouse to 1900,8
scene hover off; shieldfx off; tooltips off
scene orbit kli 90 -6 300
```
```action
wait 1
scene glide 10 40 -3 270
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
Length: 10
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
In: 0.5
Length: 7
Status: draft

The fireball's core: stock's sprite turns with the camera; refit's is a volume the camera
moves around, and the flash lights the Galaxy and the Sovereign beside it. The cut into
this shot lands on the act's first beat of music.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit 2090 0 2230 80 -6 220
```
```action
wait 1
scene kill mid; glide 7 110 -4 200
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
scene orbit target 200 -6 220
```
```action
wait 1
scene glide 8 235 -4 200
wait 7
```

### b03 · Torpedo impacts
Shows: lighting: impacts
Scene: impacts
Variants: stock, refit
Layout: wipe
Transition: cut
In: 1.0
Length: 5
Status: draft

A flash on the hull at each strike; stock has none. No title: it reads as part of the
shields beat.

```setup
scene hover off; tooltips off
scene orbit tgt 200 -6 200
```
```action
wait 1
scene glide 7 220 -4 190
wait 6
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

The Galaxy's nacelle venting plasma, the glow at the vent lighting the hull. Repairs
(`whats-new` r01) are left out for pace; the subtitle names them.

```setup
scene hover off; tooltips off
scene orbit galaxy 130 -6 200
```
```action
wait 0.5
scene glide 8 160 -8 180
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
scene orbit 3000 0 3110 90 -6 420
```
```action
wait 1
scene special holder gholdbm tgt
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
In: 0.5
Length: 8
Status: draft

The act ends quietly, on the warbird turning to glass in front of the planet. The bed
fades over it, into the space act.

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

```setup
scene hover off; tooltips off
scene cloak warbird now; cloak bop now
scene orbit warbird 300 -8 230
```
```action
wait 1
scene glide 6 330 -6 210
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

```setup
scene hover off; tooltips off
scene orbit m 300 -6 700
```
```action
wait 0.5
scene glide 9 330 -4 640
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
scene orbit 2500 0 2150 200 -8 2600
```
```action
wait 0.5
scene glide 8 215 -6 2400
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

```setup
scene hover off; tooltips off
scene orbit moon 200 -8 260
```
```action
wait 0.5
scene glide 7 225 -6 240
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

```setup
scene hover off; tooltips off
scene orbit big 210 -8 300
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
Length: 12
Status: draft

The user's idea: a fleet inside the dense purple Mutara, flying towards the Key, the
camera under and behind it looking up towards the light, so the hulls' shadows run back
down through the gas. Refit only: stock has no shadows, and its billboards look punchier
from here. The shafts are faint (only the Galaxy's reads clearly, lower left, early);
stronger gas shadows are a question for `nebulae/`. A hull near the camera can make the
shafts vanish (seen paused at `orbit sov 48 -24 220`), a bug report for `nebulae/`.
Found by a 7-angle probe (2026-10-14, n05).

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

```setup
scene hover off; tooltips off
scene orbit ship 230 -25 450
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
Sub: Selections and control groups beyond sixteen ships
Caption: top
In: 1.0
Length: 5
Status: draft

Twenty-four destroyers dragged into one selection: stock stops at sixteen. From the
October showcase (`PFX-q1-select`).

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
Length: 9
Status: draft

Half the block to group 1, the other half added with Shift+1, then 1 calls up all
twenty-four. From the October showcase (`PFX-q2-groups`).

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
wait 3
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

```setup
lclick 400 640
wait 1
```
```action
wait 1
drag 640 100 1500 300 1.4
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
In: 1.0
Length: 15
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
wait 2.5
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
Length: 7
Status: draft

Zoomed out, three destroyers selected, the pointer far to the right: the cursor turns to
the warp ring, a right-click, and they jump. *Unprobed*: the steps are
`scenarios/qol-warp.md`'s.

```setup
press ctrl+Down
wait 0.2
press ctrl+Down
wait 0.2
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
wait 6
```

### g06 · Online, under Wine and Proton · PLACEHOLDER
Length: 8
Transition: dissolve 0.5

The real shot is two games side by side (`split`): the host opens *Internet – Online*,
its game setup shows a join code; the joiner types it and appears in the host's lobby.
Needs two bench sessions in one shot (`PLAN.md` T4); the steps are
`scenarios/multiplayer-online-code.md`'s. Title: "Online play that works under Wine and
Proton", sub: "Host a game, share a code, no port forwarding".

```card
[accent] PLACEHOLDER — TWO GAMES, SIDE BY SIDE
Online play that works under Wine and Proton
[small] Host a game, share a join code, no port forwarding
```

## Installing
Music: Mfed30.wav from 0 gain -4

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
Caption: top
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
Caption: top
Audio: none
In: 0.5
Length: 7
Status: draft

The game row, the version picker and the line under them, which says what goes into
Heroic. The pointer travels down them: the game it found, the newest release picked for
you, the launch settings. Coordinates from the desktop test (2026-10-14): the window
sits in the middle of a 1600x900 desktop, the game row at y 410, the picker at
1044,489, the Heroic line at y 560, Install at 960,684 (Pointer space, 1920x1080).

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
Caption: top
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
wait 52
```

### i05b · Step 5: done
Kind: footage
From: i05
In: 46
Length: 5
Transition: dissolve 0.8
Audio: none

The end of the same take, about forty seconds on: the bar full, "Refit installed.
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
step Move the mouse to 1900,8
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
