# Beams scout
Film: beams-scout
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --install . --install testbench/scene

A scratch board for two beam shots of the promo (`films/promo`, "Beams"), worked out
here so the promo board is not touched until they are reviewed: the assimilation beam
again, from a Fusion Cube with the camera gliding round the two ships (now the holding beam, see a01), and the
Assimilator's auto-assimilator. Variants and the refit install are the promo's.

Not filmed yet: filming stopped on 2026-10-17 before the first probe. The camera
values come from a stock bench session that day (stills, `assimilation-fusion` and
`assimilator`); the refit cone has not been seen from these angles, and `In`/`Length`
are first guesses from the stills' timing.

What the stock session showed, for the editor:

- **The red in stock's assimilation beam is stock's own art**, not an artifact of the
  bench: every streak of `WTechAssimBeam.tga` has a dark red core (about 90,0,10) inside
  its green fringe, so the fan draws red lines. It cannot be taken out without faking
  stock. What can be chosen is the angle: seen from above, the fan is foreshortened to a
  narrow streak and its red lines are short; seen from the side and below (the old a01)
  it spreads into long red-cored fans across the frame. The red blobs in the old a01 are
  something else: a torpedo of the cube's bursting on the Sovereign (about 3 s and 9 s),
  avoidable by timing.
- **Dark sky.** The Borg sky's green gas is everywhere on `a2_borg01`, brighter in
  stock. Behind these ships it is darkest looking down from above (pitch +15 to +30)
  towards -x (yaw 60 to 100): mean luminance 10-13 against 21-28 from yaw 180-300.
- **The Fusion Cube is about twice a cube's size**; the Sovereign looks small beside it.
  It stands 300 from the Sovereign (centre to centre), a gap of about 130 between the
  hulls; the beam held there. Its beam leaves from a hardpoint near the top edge of the
  face towards the Sovereign, so the frame must take in the top of the cube.
- **The auto-assimilator holds with the Assimilator's weapons off** once fired, so its
  torpedoes never fire and the ships can stand 300 apart (out of torpedo range they had
  to be 520 apart, both tiny in the frame). Stock draws it as a straight blue beam.

## Beams

### a01 · The holding beam
Shows: specials: holding beam
Scene: fusion-low.ini
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: cut
In: 5.5
Length: 12
Status: probed

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

### a03 · The auto-assimilator
Shows: specials: auto-assimilator
Scene: assimilator
Variants: stock, refit
Layout: wipe
Transition: cut
In: 2.0
Length: 4.0
Status: probed

The Borg Assimilator fires its auto-assimilator into the Sovereign, from the side and
a little above, the beam running across the frame from the small Borg ship on the right
to the Sovereign on the left; the camera drifts round a little. **Wipe**: one beam
across the middle of the frame, the same in both variants, so the divider coming in
from the right shows stock's straight blue line turning into refit's blue cone with
its green fans, along the same beam. The weapons go off at once after the beam is
fired, so the Assimilator's torpedoes never fire; the beam lasts 30 s and the Sovereign,
healed, keeps its crew, so any window after the first second works.

```setup
scene orbit 3000 0 3110 100 10 330
```
```action
wait 0.8
scene weapons assim on
wait 0.2
scene special assim gborgbor tgt; glide 9 118 8 270
wait 0.3
scene weapons assim off
wait 9
```
