# I never wrote a line of this
Film: origin
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --assets none --install . --install testbench/scene

The game footage for the origin video: how the refit came to be, told by its maker, who
wrote none of its code and directed all of it by sentences. The video itself is mostly
screen recordings, screenshots and cards, and is cut elsewhere (the maker-promo board
`yt-origin.board.md`); this board films only the in-game shots it needs that
`films/promo` does not have. Every other game shot in that video is a promo take, used
by path.

Each shot here answers one of the maker's prompts: the prompt is typed on screen over
it in the cut, and the shot is the result. So each shot shows exactly what that prompt
changed, and nothing on screen may claim more.

For the editor:

- The house style holds (`films/README.md`): close-ups, the camera a little below the
  subject looking up, a wipe for one centred subject, no game music in any take.
- **"Refit" is the game as a release installs it** (`--assets none`), as in
  `films/promo`: every code layer and none of `$A2_DATA`'s assets, so no remastered hull
  textures. What changes between stock and refit here is the renderer, not the art.
- **No titles or captions in the takes or the layouts**: the video's own text track
  (the typed prompts) goes over them. The `Title:` fields are left out on purpose.
- Every shot leaves at least a second of take before `In:` and after its `Length:`, as
  handles for the cut.

## Taste

The video's fourth part: the maker's prompts were judgement, not "fix it", and what the
agents did with them. Each shot is one prompt's result, held long enough to read the
prompt over it.

### ak13 · The Akira class
Shows: lighting: hulls
Scene: akira.ini
Variants: stock, refit
Layout: wipe
Transition: cut
In: 1.0
Length: 12
Status: probed

The prompt: "the akira class hulls look too dark". One Akira class, close and a little
below, turning slowly from its bow quarter towards its side: stock's hull against
refit's, lit per pixel by the scene's key light. The eye lands on the saucer and the
weapons pod above it, where stock is flat and dark and refit shows the plating.

Probe review 2026-10-16: at 175 the Akira was a sliver a fifth of the frame wide, and
at pitch -6 nearly edge-on: closer (80 to 70) and lower (-12). The bow faces the camera
at about yaw 190 and the side at 245. Stock drew the hull solid at 70-80 (no fade).
Re-probed: stock's underside is near black, refit's blue-grey plating reads, and the
nacelles and deflector glow in both: the prompt's before and after. Tried from above
(pitch 14, as ak13b): both variants lit alike there, so the comparison is weaker; the
low angle stays. The Akira is about a third of the frame wide; a 4K take may want it
closer still (60).

```setup
scene hover off; shieldfx off; tooltips off
scene orbit akira 200 -12 80
```
```action
wait 1
scene glide 16 240 -8 70
wait 14.5
```

### mt14 · The Metreon nebula
Shows: nebulae
Scene: metreon.ini
Variants: refit
Layout: full refit
Transition: cut
In: 1.0
Length: 10
Status: probed

The prompt: "make the metreon nebula look more like actual thunderstorm clouds". A storm
front of three Metreon clouds from outside, the camera drifting in: crimson billows,
lit and shadowed sides, and lightning flickering inside them. Refit alone: the prompt is
about the result, and stock's billboards answer a different question. Lightning falls at
random: if a take has none inside the window, move `In:` or re-probe.

Probe review 2026-10-16: the clouds fill the frame from the first second (the camera
is at the storm's edge, not outside it): crimson billows with lit and shadowed sides.
Lightning flashed at about 0.9, 2.6 and 10.0 s of the take, each a soft pale glow that
lights a patch of cloud for about half a second; it reads as a glow more than as
lightning. Stars draw over the densest gas.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit 2250 0 2300 200 -5 1000
```
```action
wait 1
scene glide 13 225 -3 820
wait 12.5
```

## Bigger prompts

The fifth part: the renderer rewritten in Rust, shown at its best.

### as17 · The assimilation beam
Shows: specials
Scene: specials
Variants: stock, refit
Layout: full refit
Transition: cut
In: 3.0
Length: 10
Status: probed

A Borg cube fires its technology assimilation beam (`gTechAssimBeam`) at the Sovereign:
in refit a green cone of light in vapour from the cube onto the ship
(`render3d/ordnance.ini`, `techassimbeam`, `Kind=cone`; `ART-DIRECTION.md`, "Tractor
and assimilation beams"). The cube large in frame, the camera on its side and a little
below, so the eye goes from the cube along the beam to the target. The beam needs its
target within 300 (`TechAssimBeam.odf`): the cube is spawned 220 away, as in
`films/promo` b05. `special` must answer `fired`.

Probe review 2026-10-16: the beam fires at about 5 s of the take (the cube turns to
it first) and stops by 12 s. Refit's beam is the same thin cyan line as stock's, pixel
for pixel at the same moment, although `Render3D.log` says `drawing special
techassimbeam (Beam)`: the recipe's green beam with twisting strands does not show (a
question for render3d's `ordnance`). So the wipe is `full refit` until it does, and
this shot is weak: there is nothing in it render3d visibly draws.

Answered 2026-10-16, after `worktree-cloak-art-direction` was merged: **the
assimilation beam never forms in this scene**, in stock or refit
(`testbench/scenarios/specials.md`). It only lives while the cube can take a special
weapon the target's team has researched, and the bench's Sovereign has none, so it ends
as soon as it is fired. The thin cyan line is the cube's own Borg phaser (`bbphaso`),
firing at the enemy Sovereign, drawn as stock draws it. A research station and a
`fedpod1` pod spawned for the Sovereign's team did not make the beam form. This shot
cannot show the assimilation beam until the scene can give the target's team a
researched special (a `Scene.asi` command that does not exist yet); `tr15` shows the
same cone look in the tractor beam's blue. Do not cut this shot as the assimilation
beam.

2026-10-17: the scene can now give the target's team its special (`tech tgt gcorbom`,
testbench 3.31.0), and `films/assimilation` films the beam against a darker sky; this
shot is not changed.

```setup
scene hover off; tooltips off
scene spawn holder bbattle1 3000 0 3220 180 1
wait 3
scene engines holder off; energy holder on
scene orbit 3000 0 3120 75 -6 320
```
```action
wait 1
scene special holder gTechAssimBeam tgt; glide 13 95 -4 290
wait 13
```

### tr15 · The tractor beam
Shows: specials
Scene: specials
Variants: refit
Layout: full refit
Transition: cut
In: 2.5
Length: 10
Status: probed

The prompt behind the beams' look (`ART-DIRECTION.md`, "Tractor and assimilation
beams"): a beam as a cone of light through vapour, fans of light coming and going,
its wide end taking in the target and washing its hull in the beam's colour. A
Federation Assault ship (`fassault`) holds the Sovereign in its blue tractor beam; the
camera close and a little below, between the two, so the cone and the lit hull both
read. The tractor needs its target within 200 (`gtracbm.odf`); 220 between the centres
fired in `testbench/scenarios/specials.md`. Refit alone: stock's tractor is a thin
textured beam and answers a different question.

First filmed on a scratch board from the unmerged branch (2026-10-16): the cone came in
from the right edge, the Assault ship out of frame. Here the camera sits further back
along the beam so both ships are in frame. Probe 2026-10-16 at pitch -12: both ships
edge-on, the cone a thin blue line, since its wide end spans the target as seen across
the beam. From below (pitch -32 to -28), about the point between the ships, both are in
frame and the cone opens: at 3 s of the probe the fans show as streaks inside it, at 5 s
the Sovereign's saucer is lit blue-grey, and the cone thins as the camera comes round
towards the side (11-13 s). The cone is faint against the dark sky; a 4K take may want
it seen a little longer from 40-55.

```setup
scene hover off; tooltips off
scene spawn tractor fassault 3000 0 3220 180 1
wait 3
scene engines tractor off; energy tractor on
scene orbit 3000 0 3115 40 -32 250
```
```action
wait 1
scene special tractor gtracbm tgt; glide 13 70 -28 230
wait 13
```

### sh14 · Shield hits
Shows: shields
Scene: shields
Variants: refit
Layout: full refit
Transition: cut
In: 1.0
Length: 10
Status: probed

The prompt behind the shield hits' look (`ART-DIRECTION.md`, "Shield hits"): an uneven
mist about each impact, a soft edge where the shell curves away, and light on the
struck hull. A Galaxy class takes phaser and torpedo fire from a Galaxy and a Klingon
battleship; the camera close on its saucer and a little below, turning slowly, so
several impacts land in frame. Refit alone: stock's hit is a smooth green shell over
the whole ship.

First filmed on a scratch board from the unmerged branch (2026-10-16): several hits
read clearly, white-blue mist on the saucer's shell. Probe from `main` 2026-10-16: the
same, a hit in frame every couple of seconds, the mist reaching across the saucer's
rim at 10-13 s.

```setup
scene hover off; tooltips off
scene orbit target 200 -6 140
```
```action
wait 1
scene glide 12 235 -4 125
wait 12
```
