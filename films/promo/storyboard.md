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
   They look very different but keep stock's geometry (the user, 2026-10-17), so a wipe
   compares them part for part, like any other before/after.
6. **Quality of life**: limits gone, still playable against stock players.
7. **Installing**: the installer in a few steps, ending in the game.

**Planets, hulls and the sky are in the shots but never called out** (the user,
2026-10-17): no titles of their own; the showcase fleet over its planet, the cloak in front
of one, the menu scene's planet and the sky behind everything carry them. Hull lighting is
named only where a weapon's light falls on a hull.

About 3:50 by the shot lengths, a little less once transitions overlap; the brief aims at
3:30, so trim after the animatic (candidates: h04, n02, u03, g03, i08). The HUD and the UI
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
In: 1.7
Length: 1.2
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
Length: 3.5
Status: draft

The fireball's core: stock's sprite turns with the camera; refit's is a volume the camera
moves around, and the flash lights the Galaxy and the Sovereign beside it. The cut lands on
the act's first beat of music. From 2.7 s stock's shockwave disc fills the frame in both
variants (2026-10-15), so the shot ends before it.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit 2090 0 2230 80 -6 150
```
```action
wait 1
scene kill mid; glide 7 110 -4 140
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
In: 1.0
Length: 6
Status: draft

Stock's green shell over the whole ship against refit's hot spot and ring where each shot
lands.

```setup
scene hover off; tooltips off
scene orbit target 200 -6 130
```
```action
wait 1
scene glide 8 235 -4 120
wait 11
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

The Klingon battleship's disruptor beams, the red light running down its hull. A cut
candidate.

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

### b04 · Damage
Shows: venting
Scene: venting
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.4
Title: Damage
Sub: Venting plasma that lights the hull
In: 0.5
Length: 6
Status: draft

The Galaxy's saucer venting plasma, the glow at the vent lighting the hull.

```setup
scene hover off; tooltips off
scene orbit galaxy 130 -6 90
```
```action
wait 0.5
scene glide 8 160 -8 85
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
scene orbit warbird 230 -6 260
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
Length: 6
Transition: cut

A placeholder until the bench can stage the assimilation beam (being built on the branch
`worktree-assimilation-scene`): a Borg ship assimilating a Sovereign, close and low,
against black space, stock then refit as a wipe. Its scene, setup and action come from
that branch's `films/assimilation` board once it is reviewed.

```card
[accent] THE ASSIMILATION BEAM
To be filmed
```

### a02 · The tractor beam
Shows: specials
Scene: specials
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Beams, as in the series
Sub: Tractor, holding and assimilation beams as cones of light through vapour
In: 1.5
Length: 6
Status: draft

An Assault ship holds the Sovereign in its tractor beam; the eye goes from the cone to the
light it throws on the hull. The steps are `testbench/scenarios/specials.md`'s;
`special` must answer `fired`. *Unprobed*: the camera is b05's, and whatever lies behind
3000,0,3110 at that heading must be dark sky: if it isn't, turn the camera.

```setup
scene hover off; tooltips off
scene spawn tractor bassault 3000 0 3220 180 1
scene engines tractor off; energy tractor on
wait 3
scene orbit 3000 0 3110 90 -6 280
```
```action
wait 1
scene special tractor gtracbm tgt; glide 8 110 -5 250
wait 11
```

### b05 · The holding beam
Shows: specials
Scene: specials
Variants: stock, refit
Layout: wipe
Transition: cut
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
best on 2026-10-15. *Unprobed*: the camera is new. A hull close to the camera made the
shafts vanish then (`orbit sov 48 -24 220`), so it stays at 260 or more; if the shafts
still don't read, this is a bug report for `nebulae/` before it is a framing note.

```setup
scene hover off; shieldfx off; tooltips off
scene goto ente,sov,galaxy,akira,nebula,def1,def2 1700 0 1470
scene orbit galaxy 30 -16 300
wait 4
```
```action
wait 0.5
scene glide 12 48 -12 260
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
where stock had 800x600 art. The entries sit where stock's did, so a wipe compares the
same screen. The captions say who made it.

### m01 · The main menu
Kind: mission
Scene: menu.ini
Args: -nointro
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.6
Title: I redrew the menus
Sub: The same screens, over a live scene of the game's own ships
Audio: none
In: 1.0
Length: 7
Status: draft

The main menu as the player first sees it. The eye goes to the fleet moving over the
planet behind the column of entries, then to the entries themselves. *Unprobed*: stock's
800x600 art at 16:9 with no Menus.asi; if it stretches, the wipe shows that too, which is
honest.

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
Length: 12
Status: draft

The campaign selection: in refit, pointing at each campaign moves the camera to its scene
on the same map: the Federation fleet, Klingon ships decloaking before a red nebula, a Borg
cube's shadow through the Mutara. Stock is held for two seconds, then dissolves. The
pointer positions are `testbench/scenarios/ui-menus-scene.md`'s (220,322 / 374 / 426 at
1280x720), scaled to the Pointer space. *Unprobed*.

```setup
step Wait up to 300 seconds for the main menu
step Click the single player emblem
wait 3
ptr 330 405 0.1
```
```action
wait 1
ptr 330 483 0.8
wait 3.5
ptr 330 561 0.8
wait 4
ptr 330 639 0.8
wait 5
```

### m03 · Into warp
Kind: mission
Scene: menu.ini
Args: -nointro
Variants: refit
Layout: full refit
Transition: dissolve 0.5
Sub: A mission departs from the menu scene into warp, under the loading screen
Caption: top
Audio: game
In: 0.5
Length: 7
Status: draft

Refit only: stock goes from the menu to a still loading screen. The Federation campaign's
first mission started from the selection: the fleet goes to warp front first, the camera
swings behind it, and the tunnel comes in under the loading screen. The steps are
`testbench/scenarios/ui-loading-warp.md`'s, without its `Loading.Hold`: a real load. If
the load is too quick to see the departure, say so to the user rather than hold it.

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
Crop: 0 0.5 0.5 0.5
Transition: dissolve 0.5
Caption: top
In: 1.0
Length: 6
Status: draft

The bottom left quarter at twice the size: the minimap and the selected ship's panel,
stock's stretched sprites against refit's flat icons and live wireframe. *Unprobed*: set
the crop from u01's probe.

```setup
scene hover off; shieldfx off; tooltips off
scene hud on
scene select ente
scene orbit 2250 0 2350 160 45 1200
```
```action
wait 8
```

### u03 · The bar, close
Shows: hud: button bar
Scene: showcase
Variants: stock, refit
Layout: wipe
Crop: 0.5 0.5 0.5 0.5
Transition: dissolve 0.5
Caption: top
In: 1.0
Length: 5
Status: draft

The bottom right quarter: the button bar and the unit view. A cut candidate.
*Unprobed*.

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
shipyard at a time, a menu tree under every button) are gone, **and none of it locks out
stock players**. Paced by the pointer: every press gets a key hint and enough time to
follow. Only g01 has a before: stock has nothing to compare for the rest. Captions at the
top, over the HUD.

### g00 · Card: it plays better
Length: 3
Transition: dip 0.5
Background: h01

```card
[accent] AND IT PLAYS BETTER
Quality of life that still plays with stock players
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

Twenty-four destroyers dragged into one selection: stock rings sixteen, refit all
twenty-four. The eye counts the rings. On 2026-10-15 refit's old panel showed 16 icons
even with 24 selected; the UI's panel is new, so check it on the probe.

```setup
ptr 400 640 0.1
wait 2
```
```action
wait 1.5
drag 830 430 1350 745 1.6
wait 6.5
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
The eye goes to the three build bars filling. The yards sit at about y 340–410: the drag
from 2026-10-15 missed them, so it starts higher. *Unprobed*.

```setup
lclick 400 640
wait 1
```
```action
wait 1
drag 760 280 1260 460 1.4
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
Sub: Every button on a block of keys, the same place every time
Caption: top
Keys: auto
In: 2.5
Length: 17.5
Status: draft

The close crop of the button bar, held long enough to follow: Q opens a menu, G goes back,
W and E the next ones. The user's note from October: about 18 s worked, 5 s was far too
short; trim no further. The bar is the UI's now: check the crop on the probe.

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
the warp ring, a right-click, and they jump.

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
status line naming each layer. The install is real and takes about 40 s: this shot is its
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
In: 29.5
Length: 5
Transition: dissolve 0.8
Audio: none

The end of the same take: the bar full, "Refit installed. Heroic launch settings
written.", Launch game lit amber. Move `In:` to land a beat after the bar fills.

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
Uninstall puts the game back as it was
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
