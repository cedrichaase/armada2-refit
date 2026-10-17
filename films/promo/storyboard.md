# Armada II Refit
Film: promo
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --install . --install testbench/scene

The promo for the release that puts render3d and the UI in the installer (12.24.0): the one
video that has to make someone who loved Armada II in 2001 install it again tonight.
Rewritten 2026-10-17 for the launch campaign (maker-promo, `2026-11-refit-launch`), after
the UI rewrite, and restructured the same day after rough cut 3 (the board before it is
in git).

The angle, confirmed by the user: **Armada II as it shipped, and the same moment with what
I've built on top of it: same game, same camera, side by side, and you can install it over
your own copy.**

Rough cut 3's notes (the user, 2026-10-17) set the shape: **less of a tech demo, more
emotion; show, don't tell.** The beams, the shields, the explosions need no captions.

1. **Cold open**: the main menu's music over the Federation campaign intro's slow shots,
   one stock/refit dissolve, the title.
2. **The intros**: the Klingon and Borg campaign intros, refit, cut short, on their own
   sound. They carry the combat rough cut 3's Combat act argued shot by shot (b01, h03,
   b02, b04 are in git).

Out of rough cut 6 (the user's notes): f01 (the Romulans out of a planet's shadow) and
c03 (Species 8472's mothership on a Fusion Cube); their last staging is in
`films/shadow-scout` and `films/c03-scout`.

Out of rough cut 4 (in git, to bring back): n01 (s05b carries the nebula; no shafts read
in n01's gas, the faint-shadow bug in `nebulae/`), m03 (its warp flashes still draw as
white squares, render3d), f02 (the Romulans leaving, wide; in `films/fleets-scout`).
3. **Before and after**: the strongest pairs, without captions: the fleet through the
   nebula, the cloak; then the Fusion Cube's holding beam.
4. **Menus and the HUD**: new work, dissolved against stock, not wiped.
5. **Gameplay and online**: the limits gone, and a working online mode.
6. **Installing** and the end, trimmed.

**Planets, hulls and the sky are in the shots but never called out** (the user,
2026-10-17).

About 3 minutes is the aim (the user, rough cut 3); the HUD and the UI get a deeper video
of their own later, with voice-over.

For the editor:

- The house style holds (`films/README.md`): close-ups, the camera a little below the
  subject looking up, a wipe for one centred subject, a dissolve for subjects spread over
  the frame and for UI, no game music in any take, one music bed at a time. The intros
  are the one exception to "no game music": their score is their own, and no bed plays
  under them.
- **Captions only where the picture can't say it** (brief, "Decided" 7): g01's sixteen-
  ship limit, g02 and g04's keys, the online line, and the cards. First person and plain
  (maker-promo's `brand/brand.md`, "Voice").
- **"Refit" is every code layer with the HD textures** (the user, 2026-10-17): the
  install takes `$A2_DATA`'s built assets, the remastered textures with the backdrop
  plates and movies (no `--assets none`). The release ships none of them (copyright:
  they are derived from the game's art), so every video description says so; the
  installing section's i06 shows the HD textures too, which a fresh install without a
  pack doesn't.
- **Online play works** (the user confirmed it on 2026-10-17): *Internet – Online* in the
  multiplayer menu, join codes, no port forwarding, under Proton. Said plainly, never
  "in progress"; the user's further ideas for it are not in the film. The lobby's chat
  shows the host's LAN address, so the online shots are cut around it.
- **Every current-state shot is filmed fresh** from the release's commit.
- Camera values marked *unprobed* are first guesses.

## Cold open
Music: effects/Shellsd0.WAV from 0 gain 0 until r02 fade 1.5

The game's own opening, slowly, on the main menu's music (`sounds/effects/Shellsd0.WAV`,
a 60 s loop): the Federation campaign intro's Enterprise, then the game as a player sees
it (a working base, a station going up, a fleet arriving), then a Federation fleet over a
class M planet, stock dissolving into refit on the same camera, 2001 and now; then the
title. The user's order after rough cut 4. No captions: the cinematic's own are hidden on
the bench (`Captions=0`), and the cinematic's sound is off under the bed.

### o10 · An Enterprise over a planet
Kind: footage
From: ref:fed01-fleet-planet
In: 0
Length: 4.4
Transition: fade 1.0
Audio: none

The Enterprise slides in over the brown planet's limb, slow; the cinematic's "Its
continuing mission" subtitles are hidden on the bench (`Captions=0`, the user's note of
rough cut 4). The first image is the game's own.

### t01 · The game, as a player sees it
Kind: scene
Scene: rts.ini
Variants: refit
Layout: full refit
Transition: dissolve 0.8
In: 1.5
Length: 10.5
Status: draft

The game's RTS camera, close, slightly tilted, HUD off: a Federation fleet arrives at a
working base by a dilithium moon (a mining station, three freighters mining), while a
construction ship builds a new station beside the moon. A Federation starbase sits in the
top right corner, clear of the moon and the mining station, to round the frame off.

The user's order for the cold open (rough cut 4): the Enterprise, then the game as a player
sees it, then a Federation fleet in front of a class M planet. Adopted from
`films/rts-scout` (probe of 2026-10-17); the first second is cut (a Galaxy half out of
frame), and the fleet settles by the moon at about 8 s. The user's note on rough cut 5 asked
for the starbase and allowed a slightly wider camera: Zoom goes from 0.17 to 0.20 and the
starbase (`fbase`) stands at 2650 0 2420. It is large and near the camera, so it is
placed to sit whole in the corner (scout: `films/rts-sb-scout`).

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
### p01 · The fleet before the planet
Shows: lighting: planet
Scene: fleet-planet.ini
Variants: stock, refit
Layout: sweep 3.5 1.8
Transition: dissolve 0.8
Labels: stock 2001; refit now
In: 4
Length: 10.5
Status: draft

Five Federation ships in a loose formation under way, slowly, in front of a class M
planet that fills the frame behind them, lit by the sun. The camera stays low and drifts
along with the lead ship. No combat.

The cold open's before/after (the user, rough cut 4: "a fleet of federation ships in
front of an M-class planet might prove more impactful"; then "in profile, the ships don't
look very impressive", so the camera is low, looking up past the saucers). Stock held,
then a divider sweeps across the frame from the right, revealing refit on the same
camera (the user, rough cut 6: "a sweep instead of a fade"); the corner labels read 2001 and NOW (the
cut's 2001/now pair, which the user liked). The window is the take's settled part
(about 4.5 to 14.5 s). The refit hulls' undersides are fairly dark from below; the high
camera (p01b in `films/scenes-scout`) lights them better but reads as an overview.
Adopted from `films/scenes-scout`.

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

### o05 · Title
Length: 4.5
Transition: dip 0.5
Background: p01

```card
[mark] REFIT
A Star Trek: Armada II Mod
[small] modern graphics · online multiplayer · better gameplay
```

The card's words are the user's (rough cut 6): REFIT in the accent colour, the line under
it in white, the three things the film shows below.

## The intros
Music: Mfedmp300.wav from 0 gain -5 until b01 fade 1.5

The loud part, in the game's own cinematics (the user, rough cut 3): the Borg campaign's
opening, then the Klingon campaign's "A Dagger at Zarush", refit, cut short. Borg first
(the user, rough cut 4): the slow Borg diamond against the nebula follows the calm
Federation opening better than the Klingon firefight did, and the section builds from
there. They carry the combat, with no captions. Their own sound is the effects stem
only: the cinematics' score is left out (the user, rough cut 5), and the film's own bed
for the space section (`Mfedmp300.wav`) begins here instead and runs on into it; the
narration is left out, since cut this short it stops mid-sentence. The cinematics'
burned-in captions are hidden on the bench (`Captions=0`, the user's note of rough cut 4).

### r02 · The diamond against the nebula
Kind: footage
From: ref:borg01-diamond-nebula
In: 6.0
Length: 5.5
Transition: dip 0.6
Audio: effects -3

The Borg diamond growing against green and purple gas.

### r01 · A Borg station explodes
Kind: footage
From: ref:borg01-station-explodes
In: 0.8
Length: 6
Transition: dissolve 0.4
Audio: effects -3

The Borg station blowing up in a red and green nebula, the ring going out. Ends before
the stardate lower third (6 to 15 s of the take).

### r03 · A cube at the planet
Kind: footage
From: ref:borg01-cube-planet
In: 12.0
Length: 5.3
Transition: cut
Audio: effects -3

The cube closing on the planet, passing behind its limb, and the blast ring opening behind
it. The user asked for a cube exploding as it flies into a planet: in the cinematic the
cube goes behind the planet and only the ring is seen, so this is the nearest moment.

### k01 · Phasers on the station
Kind: footage
From: ref:kling01-station-attack
In: 4.5
Length: 4
Transition: dip 0.4
Audio: effects -3

Phaser beams on the station, the blue shield bubble flaring, the first fireball.

### k02 · The starbase under fire
Kind: footage
From: ref:kling01-station-defence
In: 5.0
Length: 5
Transition: cut
Audio: effects -3

A fireball on the domed starbase, then the ring blasts against red and green nebulae.

### k03 · Birds of prey in the blast
Kind: footage
From: ref:kling01-bop-ring
In: 5.5
Length: 4
Transition: cut
Audio: effects -3

A ring blast opening round two Klingon birds of prey.

## Before and after: space

The intros' bed runs on.

The quiet half, long holds, no captions: the picture is the argument. The fleet flying
into the Mutara's Key light from behind (the user's favourite of rough cut 3), then inside
the gas; the warbird turning to glass.

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

### b06 · The warbird cloaks
Shows: lighting: cloak
Scene: cloak
Variants: stock, refit
Layout: wipe
Transition: dissolve 0.4
In: 1.5
Length: 7
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

## Before and after: battle
Music: Mkli300.wav from 0 gain 6 until a01 fade 1.5

Back for the longer cut (rough cut 4: 4–5 minutes, "no need to rush"): the explosion,
the shield hits and the venting, each a wipe, no captions ("the video shows ... the
shields, the explosions; they need no captions", the user after rough cut 3).

### b01 · A cruiser explodes
Shows: explosions
Scene: explosion
Variants: stock, refit
Layout: wipe
Transition: cut
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

### b02 · Shield hits
Shows: shields
Scene: shields
Variants: stock, refit
Layout: wipe
Transition: cut
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

## Before and after: beams and the Borg
Music: Mbor300.wav from 40 gain 6 until m01 fade 1.5

The Borg bed. The Fusion Cube holding a Sovereign in its beam, filmed from below so the
cube towers (the user, rough cut 4), the refit cone held long.

### a01 · The holding beam
Shows: specials: holding beam
Scene: fusion-low.ini
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: cut
In: 5.5
Length: 12
Status: draft

Restaged on the user's notes on rough cut 4 (2026-10-17), and again on the director's: the
Fusion Cube is the only beam-from-a-cube scene (the tractor is cut), it fires the holding
beam (`gholdbm`, weapon 7 of a Fusion Cube's ODF; `special` answers fired), the shot is
long, and the camera is under the Sovereign looking up past it at the cube. The scene is
`fusion-low.ini`, a copy of `assimilation-fusion` with the Sovereign hung 120 below the
cube's plane and the cube 40 above it, because with both on one plane a camera below the
Sovereign sees the cube lower than the ship, never over it. The camera orbits a point 40
under the cube's plane (-80), behind the Sovereign (yaw 185, pitch -28, 220 away, gliding to
pitch -31, 205 over 15 s): the cube's face fills the frame foreshortened, the beam runs
from its top edge down to the Sovereign. Stock is held 3 s, then a dissolve over 1.5 s to
refit's beam, held for the rest.

The cube's phasers and torpedo fire from 1.8 to 4.1 s and the Sovereign's shield bubble
lights up under them (bright blue at 4 s, a faint shell by 6 s; `shieldfx off` does not
remove it, it is the shield's own draw), so the shot comes in at 5.5 s. Stock's torpedo
burst (orange) is at about 4 to 5 s, also before the window. The beam holds to the end of
the take. Stock draws it as thick green streaks with blue shield lightning, no red;
refit as a pale, wide green cone. The Sovereign stays whole in frame from 5.5 to 17.5 s (probed at 6, 10, 14 and 17 s), large in the lower right with the cube towering behind it; from about 14 s it covers part of the beam's lower end, the beam still reading from the cube's top edge.

In the promo: the one Borg beam shot (the user after rough cut 4: the tractor and the
Assimilator's beam cut; then "the holding beam looks better than the assimilation beam",
so the Fusion Cube fires `gholdbm`, which its ODF carries). b05 (a cube's holding beam)
is out, since this shot now carries it. The Sovereign's shield shell, lit by the cube's
phasers at the start, stays faintly round it (`shieldfx off` doesn't take it out: it is
the game's own shield draw). Adopted from `films/beams-scout`.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit 3000 -80 3080 185 -28 220
```
```action
wait 1.5
scene weapons borg on
wait 0.2
scene special borg gholdbm tgt;  glide 15 196 -31 205
wait 15.8
```

## Menus
Music: Mcarmp300.wav from 0 gain 0 until i02 fade 1.5

The film turns from the 3D view to the screens around it: the menus are new work, drawn
by the UI over a live scene of the game's own ships, where stock had 800x600 art. They
dissolve: the entries keep stock's click positions but not its layout. The picture says
it; the captions went (rough cut 3).

### m01 · The main menu
Kind: mission
Scene: menu.ini
Args: -nointro
Variants: refit
Layout: full refit
Transition: dissolve 0.6
Audio: none
In: 1.0
Length: 7
Title: New title menus
Status: draft

The main menu as the player first sees it. The eye goes to the fleet moving over the
planet behind the column of entries, then to the entries themselves. *Unprobed*: stock's
800x600 art, pillarboxed at 16:9. Refit's main menu is one column over the scene where
stock scatters its entries over the art, so this one is a dissolve, not a wipe (probe 1).

Refit only since rough cut 4 (the user: stock's menu doesn't add much), with a short title in its place, the user's wording.

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
Variants: refit
Layout: full refit
Transition: dissolve 0.5
Audio: none
In: 1.0
Length: 16
Status: draft

The campaign selection: in refit, pointing at each campaign moves the camera to its scene
on the same map: the starbase (training), the Federation fleet, Klingon ships decloaking
before a red nebula, a Borg cube's shadow through the Mutara. **Each entry is hovered in
turn, the camera panning to its scene.** Measured on the probes (frame differences over
the refit takes, 2026-10-17): training's starbase is already up when the screen opens, so
its hover only lights the entry (2 s); the pans to Federation, Klingon and Borg take
1.6, 2.2 and 1.9 s from about 0.4 s after the hover; the Klingon ships are in frame by the
pan's end, the Borg cube and diamond about 4 s after the hover. So 3 s an entry, 2 for
training, 4 for Klingon (the user's note of 2026-10-17: their ships need a second more to
decloak) and about 5 for Borg. Stock is held for two seconds, then dissolves. The pointer
positions are `testbench/scenarios/ui-menus-scene.md`'s (220 × 270 / 322 / 374 / 426 at
1280x720), scaled to the Pointer space. *Unprobed*.

Rough cut 4: refit only (stock adds nothing here), and the Borg view held 3 s longer (the user: linger on it), so the action's last wait was 10.5; rough cut 5 took 2.5 s off the end again (the user), so the shot is 16 s; the take keeps its last 2.5 s unused.

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
wait 4
ptr 330 639 0.6
wait 10.5
```

## Modern online multiplayer

The menus' bed runs on. Moved before the HUD and the gameplay (the user, rough cut 5:
"i think this should be more exciting to a lot of players"), titled "Modern online
multiplayer" in the user's words.

### l01 · A join code
Shows: online: Internet - Online, join code, no port forwarding
Players: host, joiner
Prepare: online/bench-asi.sh --server
Args: -nointro
Scene: menu.ini
Layout: full joiner
In: 1
Length: 8.9
Crop: 0.2 0.2 0.6 0.6
Transition: dissolve 0.5
Audio: none
Title: Modern online multiplayer
Sub: Internet – Online: a join code, no port forwarding
Status: draft

The joiner's screen only, through the UI's menus: *Internet – Online* in the
multiplayer list, the join-code dialog, the code typed, O.K. It ends at 9.9 s, before the
game list, whose chat shows the host's LAN address (it opens at 10.1 s in the final take;
the probe, filmed with clicks found by their text, opened it at 8.4 s).
The one caption kept besides g01's: the picture can't say that there is no port
forwarding. The user confirmed on 2026-10-17 that online play works, so it is said
plainly. Adopted from `films/online-scout` o1.

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
# at design positions, not by their text: each text search took about 4 s at 4K, and the
# take was too slow for its window (the final take, 2026-10-17)
joiner: step Click at design 400,336
wait 1
joiner: step Click at design 400,292
joiner: typelog host Online.log "join code"
wait 0.6
joiner: step Click at design 412,347
joiner: step Wait up to 30 seconds for "host's Game"
wait 0.8
joiner: step Click "host's Game"
joiner: step Click "Join Game"
joiner: step Wait up to 30 seconds for "GAME SETUP"
wait 4
```

### l02 · Both games in the match
Shows: online: a match on our own transport
Players: host, joiner
Prepare: online/bench-asi.sh --server
Args: -nointro
Scene: menu.ini
Layout: split 1
In: 34.5
Length: 7
Transition: dissolve 0.5
Audio: none
Labels: host Player 1; joiner Player 2
Status: draft

Host and joiner side by side: both loading screens at 100 %, then both games in the
same match, the HUD on each. It starts at 34.5 s, after the lobby, whose chat shows the
host's LAN address until the launch (the final take's: the host's last LAUNCH is done at
28.6 s, the loading screens reach 100 % at about 34 s, the match is on at about 38 s;
the probe ran faster, its window started at 18.5 s). `split 1`: each game whole, so the
loading screen's player list (host, AI, joiner) is in frame. Adopted from `films/online-scout` o2.

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
joiner: step Click "Internet - Online"
joiner: step Wait up to 10 seconds for "Join code or host address"
joiner: step Click at design 400,292
joiner: typelog host Online.log "join code"
joiner: step Click "O.K."
joiner: step Wait up to 30 seconds for "host's Game"
joiner: step Click "host's Game"
joiner: step Click "Join Game"
joiner: step Wait up to 30 seconds for "GAME SETUP"
```
```action
host: step Click "LAUNCH"
wait 5
joiner: step Click "LAUNCH"
wait 5
host: step Click "LAUNCH"
wait 32
```

## The HUD

The same bed. The HUD is redrawn too, in LCARS and in the colours of the player's race,
every panel where stock had it, laid out for the screen. Dissolves (the user, rough cut
3): the new layout puts the bar beside the minimap at 16:9, so a wipe no longer lines up.
**The race is pinned to the Federation** here and in the QOL shots (`Race=` in
`hud-showcase.ini`, `qol.ini`, `warp.ini`), and the RTS camera's zoom with `Zoom=0.37`
(rough cut 3's failed selection came from a zoom that differed per launch).

### u01 · The HUD in battle
Shows: hud
Scene: hud-showcase.ini
Variants: stock, refit
Layout: dissolve 2.5 1.2
Transition: dissolve 0.8
In: 1.0
Length: 8
Status: draft

The showcase battle with the HUD on and the Enterprise selected, so the info panel is
full, the camera high and still as a player's would be. The eye goes to the bottom panels
and the type. *Unprobed*.

The user, rough cut 4: "this before/after of the game hud is actually pretty cool at this angle." u02's close-up is out ("doesn't do much").

```setup
scene hover off; shieldfx off; tooltips off
scene hud on
scene select ente
scene orbit 2250 0 2350 160 45 1200
```
```action
wait 9
```

## Gameplay

The limits that made Armada II fiddly (sixteen ships a selection, a menu tree under every
button, long moves at impulse) are gone. The menus' bed runs on. Paced by the pointer:
every press gets a key hint and time to follow. Captions at the top, over the HUD, only
where a key or a number has to be said.

### g00 · Card: gameplay improvements
Length: 3
Transition: dip 0.5
Background: u01

The user's wording (rough cut 4).

```card
[accent] GAMEPLAY IMPROVEMENTS
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

### g04 · The grid keys
Shows: grid
Scene: qol.ini
Variants: refit
Layout: full refit
Crop: 0.02 0.5 0.5 0.5
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
short; trim no further. The UI's layout fix put the bar beside the minimap, at about x
0.18–0.37, y 0.77–0.99 at 16:9; the crop is wider than rough cut 3's (the user: not as
close): half the frame, the minimap, the bar and part of the selection panel along the
bottom, the selected block above (rough cut 4's first try at 0.36 was still too close). Presses at
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
Scene: warp.ini
Variants: refit
Layout: full refit
Transition: dissolve 0.4
In: 0.5
Length: 10
Status: draft

Probe 1: no warp at four zoom steps (the move was shorter than `WarpDistance`); rough
cut 3 pressed ctrl+Down 30 times, which lands on a different zoom each launch. Now
`Zoom=0.8` in `warp.ini`: fully out a pixel is about 1.75 units (`scenarios/qol-warp.md`),
so at 0.8 the pointer's 890 px are about 1400 units, past `WarpDistance=1100`, and the
ships are a quarter bigger than fully out (the warp read small). *Unprobed*. Zoomed out, three destroyers selected, the pointer far to the right: the cursor turns to
the warp ring, a right-click, and they jump.

```setup
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

Short and calm: where to get it, the installer, the game. The window is the GUI installer
filmed on a bare desktop (`Kind: desktop`), installing into a stock copy of the game, so
the last shot is the honest result. i08 (the "good to know" card) is out: its lines belong in the description.

### i02 · Get it
Length: 6
Transition: dip 0.6

A card, not a browser: a page filmed today dates the video. The user's wording (rough cut
4): "Buy from GOG, Mod Download in the description"; the address is in the description,
not in the video.

```card
[accent] INSTALLING
Buy the game from GOG
The mod's download is in the description
[small] Linux · Windows · macOS
```

### i03 · Run it
Kind: desktop
Run: python3 {repo}/gui-installer/armada2-refit-installer.py
Wallpaper: s05b refit 5
Layout: full
Transition: dissolve 0.5
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
Audio: none
In: 0.5
Length: 6
Status: draft

Install pressed, the sky going to warp, the engine glow riding the progress bar, the
status line naming each layer. The probe ran the checkout's 1.5.0 script, which shows "A newer installer (1.6.0) is
available" throughout: the final take must run the release's AppImage. The install is real and takes about 40 s: this shot is its
start, i05b its end, one take (restored for the longer cut after rough cut 4).

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
Length: 7
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

## End
Music: Mfedmp300.wav from 0 gain 0

### z01 · End card
Length: 6
Transition: dip 0.6
Background: s05b

The open door, not a sales line (the brand's ending): an invitation to comment, in the
user's words (rough cut 6).
No address on screen (the user, rough cut 4): the repository and the download go in the
description.

```card
[mark] REFIT
A Star Trek: Armada II Mod
[small] I'd love to hear your feedback, drop a comment!
```
