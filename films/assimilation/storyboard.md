# The assimilation beam
Film: assimilation
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --assets none --install . --install testbench/scene

A Borg cube reads a Sovereign with its technology assimilation beam (`gTechAssimBeam`),
for a stock and refit before and after (a wipe) in the promo films, and for a 9:16 Short
cut from the same takes. In refit the beam is render3d's cone of light through vapour
(`render3d/ordnance.ini`, `techassimbeam`, `Kind=cone`; `ART-DIRECTION.md`, "Tractor
and assimilation beams"); in stock it is the engine's textured beam model.

The scene is `testbench/scene/scenes/assimilation.ini`: the Sovereign (`tgt`, an enemy)
and the cube (`borg`, 220 away, engines off, special energy kept full). The beam only
lives while the Sovereign carries a special weapon its team has available and the cube
does not carry yet; the scene makes the Sovereign's corbomite reflector available to
its team (`Tech=gcorbom`), which `a2_borg01`'s tech tree otherwise forbids. The cube's
weapons are off until the action fires the beam (switched on 0.2 s before: the special
does not fire on the tick they come on), and off again once it is over; switching them
off while the beam is on ends it.

For the editor:

- The beam is faint: keep the background dark. On `a2_borg01` the sky behind the beam
  from these angles is mostly black, with the Borg sky's green gas at the edges.
- The subject (the beam's middle, where it meets the saucer) sits near the middle of
  the frame, so a 9:16 crop of the centre keeps the cone and the lit saucer.
- Timing, from the probes of 2026-10-17: the beam is fired at 1 s and reaches the
  Sovereign at about 2-2.5 s (it grows at the shot's speed), holds, and ends at about
  8-8.5 s, when the cube has taken the corbomite. As its weapons come on, the cube also
  fires a phaser (a thin cyan line, 1.5-2.5 s) and in some takes a torpedo (about 3 s),
  and after the beam its phasers again. `In: 4.0`, `Length: 3.8` keeps the clean
  middle; don't run a cut of this shot past 8 s.

### as01 · The cone on the saucer
Shows: specials: assimilation beam
Scene: assimilation
Variants: stock, refit
Layout: wipe
Transition: cut
In: 4.0
Length: 3.8
Status: probed

Close, from the Sovereign's side and a little below: the cone opens from a bright
point on the cube and takes in the saucer, which takes a faint green wash. The camera
drifts a little round towards the cube.

Probe review 2026-10-17 (fourth round): from 4 to 8 s the beam holds in both variants.
Refit: a green cone from a bright point on the cube, widening onto the saucer's
underside, against black sky; it reads, but it is a quiet effect beside stock's, and
the hull wash is slight. Stock: the engine's beam model, a wide fan of green and red
streaks over the Sovereign, much louder, and its sky shows more of the Borg green gas
(the stock sky is brighter than render3d's). The beam's middle sits at the frame's
centre, so a 9:16 crop keeps the cone and the saucer.

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

### as02 · The cube and the Sovereign
Shows: specials: assimilation beam
Scene: assimilation
Variants: stock, refit
Layout: wipe
Transition: cut
In: 4.0
Length: 3.8
Status: probed

A little wider, from the side: the cube on the right, the Sovereign on the left, the
cone between them, so the eye goes from the cube along the beam to the ship.

Probe review 2026-10-17: the beam holds from 4 to 8 s in both variants. From this side
refit's cone is a thin, faint green line most of the time; it reads at full size but
would be weak in a 9:16 crop. Use `as01` for the Short.

```setup
scene orbit 3000 0 3110 122 -9 300
```
```action
wait 0.8
scene weapons borg on
wait 0.2
scene special borg gTechAssimBeam tgt; glide 8 136 -9 280
wait 8
scene weapons borg off
wait 1
```
