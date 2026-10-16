# What's new in the refit
Film: whats-new
Final: 3840x2160

The second showcase. The October film (textures, per-pixel light and shadows, planets
with a night side, colonies, nebula and weapon light, the HUD and menus, quality of life)
argued that the refit makes Armada II *look its age less*. This one argues something
bolder: that the game's **effects** — the things that happen in a battle, not the
things that sit still in it — are now drawn the way a modern engine would draw them, and
still by the 2001 engine, frame for frame the same game. Every effect in it is a shader
in place of a sprite or a flipbook, and every comparison is stock and refit filmed from
the same shot list.

About three minutes. Four acts, in the order a battle goes: **fire** (explosions, shields,
impacts), **damage and repair**, **space itself** (planets, moons, asteroids, nebulae,
the sky), and **tricks** (cloaks, special weapons, the warp flash, the fog of war).

For the editor:

- The house style holds (films/README.md): close-ups, the camera a little below the
  subject looking up, a wipe for one centred subject, a dissolve when the subject is
  spread over the frame, no game music in the takes, one music bed at a time.
- **Every layer filmed here is still "installed, not yet seen in game"** in its changelog
  (explosions, shields, repair, venting, specials, fog, sky, nebulae, surfaces). Footage
  that shows a fault is a bug report first: don't cut around it, flag it.
- **A shot that kills its subject must orbit a point, not the object** (`orbit X Y Z ...`):
  an orbit follows its object, and when the object dies the camera jumps to the origin
  and the frame is empty sky (seen on the bench, 2026-10-14).
- Effects are timed from the start of the action block, and both variants run the same
  block, so a kill or a cloak lands at the same second in stock and refit: the wipes
  line up without alignment.

## Cold open
Music: Mspecmp300.wav from 0 gain -2 until a00 fade 1.0

Thirteen seconds that promise the film: three refit moments, no comparison, no titles,
then the title card. Cut on the music's pulse. The teasers are cut from other shots'
takes, so they cost no filming of their own.

### o01 · Card: since the last refit
Length: 3
Transition: fade 0.8

```card
[small] ARMADA II REFIT
SINCE THE LAST VIDEO
```

### o02 · Teaser: the fireball
Kind: footage
From: e01 refit
In: 1.6
Length: 2.5
Audio: game

The cruiser's fireball at its height, debris flying past the camera. Cut in hard on the
flash.

### o03 · Teaser: the warbird cloaks
Kind: footage
From: c01 refit
In: 2.5
Length: 2.5
Transition: dissolve 0.3

The hull halfway to glass, the planet bending through it.

### o04 · Teaser: into the nebula
Kind: footage
From: n01 refit
In: 4.0
Length: 2.5
Transition: dissolve 0.3

Gas sliding past the lens.

### o05 · Title
Length: 4
Transition: dip 0.4
Background: n01

```card
[big] ARMADA II
[accent] R E F I T
[small] Every effect, redrawn
```

## Act 1 — Fire
Music: Mkli300.wav from 0 gain 0 until k00 fade 1.5

The argument: a battle in stock Armada II is flipbooks — a sprite sheet for every
fireball, a textured disc for every shockwave, a green shell for every shield hit.
Refit draws each of them as light and gas in the scene. Klingon music: this is the loud
act.

### a00 · Card: Fire
Length: 2.5
Transition: dip 0.4

```card
[accent] IN BATTLE
FIRE
```

### e01 · A cruiser explodes
Shows: explosions
Scene: explosion
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Explosions
Sub: Fireballs ray-marched as gas, cooling from white flash to smoke
In: 0.5
Length: 7.5
Status: draft

The eye lands on the fireball's core: stock's sprite is a flat disc that turns with the
camera; refit's is a volume that the camera moves around, and the flash lights the
Galaxy and the Sovereign beside it. The shockwave band crosses the frame at about 1 s.
Framing checked on the bench at 150 (1080p): the fireball fills the frame at its height,
so the shot opens at 220 and drifts in.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit 2090 0 2230 80 -6 220
```
```action
wait 1
scene kill mid; glide 7 110 -4 200
wait 7.5
```

### e02 · The Sovereign, in slow motion
Shows: explosions
Scene: explosion
Variants: refit
Layout: full refit label
Speed: 0.5
In: 0.6
Length: 6
Status: draft

The same effect at half speed, refit alone: time to see the billows roll and cool, the
smoke going brown at the edges. No title — the e01 title still holds in the viewer's
head. If the half-speed take stutters, film it with `scene pause`/`resume` steps instead
and say so here.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit 2060 0 2340 40 -8 240
```
```action
wait 0.5
scene kill big
wait 6.5
```

### s01 · Shield hits
Shows: shields
Scene: shields
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.5
Title: Shields
Sub: Each hit lights the shield where it lands, weaker as the shields run down
In: 1.0
Length: 8
Status: draft

The target Galaxy under phaser and torpedo fire. Watch the struck point: stock pastes the
same green shell over the whole ship; refit draws a hot spot and a ring where the shot
lands, and the shell glows only around it. Keep `shieldfx` on (it is the subject).

```setup
scene hover off; tooltips off
scene orbit target 200 -6 220
```
```action
wait 1
scene glide 8 235 -4 200
wait 8
```

### s02 · Shields down
Shows: shields: shields-down arcs
Scene: shields
Variants: stock, refit
Layout: wipe
In: 0.5
Length: 7
Status: draft

The target's shields have run out: stock's turning blue band against refit's jagged
arcs crackling over the hull and lighting it. The arcs must read as electricity, not as
a rotating pattern — if they look like they turn, that is a bug for `shields/`. The setup
waits for the shields to fail (45 s in `scenarios/shields-down-repair.md`).

```setup
scene hover off; tooltips off
scene heal target off
wait 45
scene orbit target 150 -8 160
```
```action
wait 0.5
scene glide 7 175 -6 140
wait 7
```

### i01 · Torpedo impacts
Shows: lighting: impacts
Scene: impacts
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Impacts
Sub: A torpedo that strikes a hull lights it, for a moment, in its own colour
In: 1.0
Length: 7
Status: draft

The Klingon flagship's torpedoes landing on the Galaxy. The eye goes to the flash on the
hull at each strike; in stock there is none. Shield flashes are off in this scene, so
every hit is on the shield until `heal tgt off` — leave it, the flash on the shield is
what lights the hull.

```setup
scene hover off; tooltips off
scene orbit tgt 200 -6 200
```
```action
wait 1
scene glide 7 220 -4 190
wait 7
```

## Act 2 — Damage and repair

The Klingon bed carries on. The argument: a damaged ship in stock shows it with
billboards that look pasted on; refit's damage is part of the ship — gas venting from the
hull, plasma streaming from a nacelle, a welding arc lighting the plating as the crew
repairs it.

### v01 · Venting plasma and gas
Shows: venting
Scene: venting
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.6
Title: Damage
Sub: Plasma and gas vent from the hull, glowing as they cool
In: 0.5
Length: 8
Status: draft

The Galaxy parked, every system damaged: plasma from the nacelle, gas from the hull. The
eye goes to the nacelle's vent: the glow at its mouth lights the hull around it.

```setup
scene hover off; tooltips off
scene orbit galaxy 130 -6 200
```
```action
wait 0.5
scene glide 8 160 -8 180
wait 8
```

### v02 · Venting under way
Shows: venting
Scene: venting
Variants: stock, refit
Layout: wipe
In: 2.0
Length: 7
Status: draft

The same ship moving: one unbroken trail behind it (stock leaves puffs). Judge the colour
moving, not parked (`venting/README.md`: the plasma cools to its trail within about a
second). The orbit follows the ship.

```setup
scene hover off; tooltips off
scene orbit galaxy 200 -6 260
```
```action
scene goto galaxy 2000 0 3500
wait 9
```

### r01 · Repairs
Shows: repair
Scene: shields
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.6
Title: Repairs
Sub: A welding arc at each system being repaired, lighting the hull
In: 2.0
Length: 8
Status: draft

The target, its shields down and systems damaged, after the shooters stop: the crew
repairs. Stock's sparks are a few sprites; refit's arc flickers, throws sparks and lights
the hull. Close in: the welds are small. If the take catches no weld in its window, move
`In:` rather than lengthening the shot.

```setup
scene hover off; tooltips off
scene heal target off
wait 45
scene weapons shooter off; weapons klingon off
scene orbit target 140 -8 110
```
```action
wait 2
scene glide 8 165 -6 100
wait 8
```

## Act 3 — Space itself
Music: Mfed300.wav from 0 gain 2 until c00 fade 1.5

The quiet act: the things that do not move much, drawn from recipes rather than painted.
Federation music, slower cuts, longer holds. The argument: the planets, the moons, the
rocks and the sky were textures on low-polygon shapes; now they are surfaces with
relief, oceans, ice and cloud, and a sky with depth.

### k00 · Card: Space itself
Length: 2.5
Transition: dip 0.5

```card
[accent] AT REST
SPACE ITSELF
```

### p01 · A class M world
Shows: surfaces, models: planet tessellation
Scene: planets
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.8
Title: Planets
Sub: Surfaces built from recipes: relief, oceans, ice and cloud
In: 0.5
Length: 9
Status: draft

The class M planet filling two thirds of the frame, turning slowly past the camera. The
eye lands on the coastlines and the cloud; the wipe crosses the limb, where stock's
facets show.

```setup
scene hover off; tooltips off
scene orbit m 300 -6 700
```
```action
wait 0.5
scene glide 9 330 -4 640
wait 9
```

### p02 · Six classes
Shows: surfaces
Scene: planets
Variants: stock, refit
Layout: dissolve 3 1.5
In: 0.5
Length: 8
Status: draft

A wide shot down the row of D, H, J, K, L and M: the variety is the point (a desert, an
ice world, a gas giant, ...). Spread over the frame, so a dissolve. One of the few wide
shots, on purpose.

```setup
scene hover off; tooltips off
scene orbit 2500 0 2150 200 -8 2600
```
```action
wait 0.5
scene glide 8 215 -6 2400
wait 8
```

### p03 · The dilithium moon
Shows: surfaces: moon, models: moon smoothing
Scene: planets
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Dilithium moon
Sub: Rock with depth, its cracks open to the glow inside
In: 0.5
Length: 7
Status: draft

Close on the moon; the glow shell shows only where a crack opens at the silhouette, so
keep the limb in frame.

```setup
scene hover off; tooltips off
scene orbit moon 200 -8 260
```
```action
wait 0.5
scene glide 7 225 -6 240
wait 7
```

### a01 · Asteroids
Shows: surfaces: asteroids, models: asteroid smoothing
Scene: asteroids
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: dissolve 0.6
Title: Asteroids
Sub: Smoothed, cratered, lit as rock
In: 0.5
Length: 8
Status: draft

The big field with the Galaxy beyond it for scale. Rocks spread over the frame: a
dissolve. Which rocks a session shows is drawn at random — the stock and refit takes may
differ in which models appear. If they do, cut this as `full refit label` instead.

```setup
scene hover off; tooltips off
scene orbit big 210 -8 300
```
```action
wait 0.5
scene spin 4
wait 8
```

### n01 · Into the nebula
Shows: nebulae
Scene: nebula
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: dissolve 0.8
Title: Nebulae
Sub: A volume of gas the camera flies through, lit by its ships
In: 0.5
Length: 10
Status: draft

The camera pushes in from outside the Mutara nebula towards the Galaxy at its edge. The
gas fills the frame, so dissolve rather than wipe. Hold: the parallax between the layers
of gas is what sells the volume, and it needs camera motion and time.

```setup
scene hover off; tooltips off
scene orbit nebula 200 -5 900
```
```action
wait 0.5
scene glide 10 230 -3 420
wait 10
```

### k01 · The sky
Shows: sky
Scene: planet
Variants: stock, refit
Layout: wipe
Title: The sky
Sub: Computed for every direction, not six painted faces
In: 0.5
Length: 7
Status: draft

No subject but the sky: the camera tilted up and turning slowly, the Galaxy small at the
bottom of the frame for scale. The wipe crosses open sky, where stock's cube faces and
their seams show. Sky taste: soft billows, no filaments.

```setup
scene hover off; tooltips off
scene orbit ship 230 -25 450
```
```action
wait 0.5
scene spin 3
wait 7
```

## Act 4 — Tricks
Music: Mrommp300.wav from 0 gain 0 until z01 fade 1.5

The Romulan act: cloaks, special weapons, warp, and the fog of war. The argument: the
game's special effects were the most dated part of it; refit draws them as light that
touches the hulls around them.

### c00 · Card: Tricks
Length: 2.5
Transition: dip 0.4

```card
[accent] SLEIGHT OF HAND
TRICKS
```

### c01 · The warbird cloaks
Shows: lighting: cloak
Scene: cloak
Variants: stock, refit
Layout: wipe
Transition: cut
Title: Cloaking
Sub: A cloaked ship drawn as thick glass, bending what is behind it
In: 0.5
Length: 9
Status: draft

The warbird in front of the planet. Stock fades the hull out; refit turns it to glass
patch by patch behind a glowing front, and the planet bends through it. The cloak starts
1 s into the action, so it lands at 0.5 s into the shot.

```setup
scene hover off; tooltips off
scene orbit warbird 230 -6 260
```
```action
wait 1
scene cloak warbird on
wait 8.5
```

### c02 · Glass
Shows: lighting: cloak
Scene: cloak
Variants: refit
Layout: full refit label
In: 1.0
Length: 6
Status: draft

Refit alone, the warbird and the Bird of Prey both fully cloaked, the camera sliding past
so the refraction moves against the planet. No stock counterpart: stock shows nothing.

```setup
scene hover off; tooltips off
scene cloak warbird now; cloak bop now
scene orbit warbird 300 -8 230
```
```action
wait 1
scene glide 6 330 -6 210
wait 6
```

### x01 · The holding beam
Shows: specials
Scene: specials
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.5
Title: Special weapons
Sub: Beams, bolts and waves of light, lighting the hulls they touch
In: 1.5
Length: 7
Status: draft

A Borg cube spawned behind the Sovereign holds it in a beam. The eye goes from the beam
to the light it throws on the Sovereign's hull. `special` answers `fired` when it worked
(`scenarios/specials.md`); a probe whose log says otherwise needs a retake.

```setup
scene hover off; tooltips off
scene spawn holder bbattle1 3000 0 3220 180 1
wait 3
scene orbit 3000 0 3110 90 -6 420
```
```action
wait 1
scene special holder gholdbm tgt
wait 7.5
```

### w01 · The warp flash
Shows: specials: warp flash
Scene: specials
Variants: stock, refit
Layout: wipe
In: 1.0
Length: 6
Status: draft

The Galaxy jumps to warp past the camera. The orbit is about a point, not the ship, so
the camera stays while the ship leaves. Short: the flash is the subject, and it lasts a
moment.

```setup
scene hover off; tooltips off
scene orbit 1500 0 1500 200 -6 350
```
```action
wait 1.5
scene goto warper 9000 0 1500 warp
wait 5.5
```

### f01 · The fog of war
Shows: fog
Scene: fog
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: dissolve 0.6
Title: Fog of war
Sub: Smooth edges instead of a staircase, as the scout reveals the map
Caption: top
In: 0.5
Length: 8
Status: draft

The one shot from above (the subject is the map): the scout's revealed trail across the
fog, seen from high and steep. Watch the edge of the fog: stock's cell-by-cell staircase
against refit's smooth line. The setup sends the scout out first, so the take opens with
a trail already cut.

```setup
scene hover off; tooltips off
scene goto scout 4200 0 3800
wait 30
scene orbit 2000 0 2000 200 60 7000
```
```action
wait 0.5
scene glide 8 215 55 6000
wait 8
```

## Close
Music: Mfedmp300.wav from 0 gain 0

### z01 · What's new
Length: 7
Transition: dip 0.5
Background: n01

```card
[accent] WHAT'S NEW IN THE REFIT
Ray-marched explosions and shockwaves
Shield hits and shields-down arcs drawn by shaders
Venting plasma, welding repairs
Planets, moons and asteroids from recipes
Volumetric nebulae and a computed sky
Cloaks as glass, special weapons as light, smooth fog of war
```

### z02 · End card
Length: 6
Transition: dissolve 0.8
Background: e01

```card
[big] ARMADA II
[accent] R E F I T
github.com/cedrichaase/armada2-refit
[small] Free and open source. Bring your own copy of the game.
```
