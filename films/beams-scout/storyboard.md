# Beams scout
Film: beams-scout
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --install . --install testbench/scene

A scratch board for two beam shots of the promo (`films/promo`, "Beams"), worked out
here so the promo board is not touched until they are reviewed: the assimilation beam
again, from a Fusion Cube with the camera gliding round the two ships, and the
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

### a01 · The assimilation beam
Shows: specials: assimilation beam
Scene: assimilation-fusion
Variants: stock, refit
Layout: dissolve
Transition: cut
In: 3.0
Length: 4.0
Status: draft

A Borg Fusion Cube reads the Sovereign, seen from above and to the side against the
darkest part of the sky, while the camera glides slowly round the pair (30 degrees of
yaw over the shot, closing in a little). The eye goes from the bright point on the
cube's top edge down the beam to the saucer. **Dissolve, not wipe**: the subject is
spread over the frame (a huge cube on the right, a small ship on the left, the beam
between) and the camera moves; a wipe's divider would cut through the beam and show
half a cube in each variant, while a dissolve shows each whole, the same moment. The
cube's phasers fire as its weapons come on (cyan lines, the first 1.5 s or so), so the
window starts after them; the beam ends about 7 s after it is fired, when the cube has
taken the corbomite, and the weapons go off at 9 s so its phasers stay quiet after it.
To check on the probe: that refit's cone reads at this distance (if not, closer: the
glide's end distance down to 340), and where the stock fan's red lines are least.

```setup
scene orbit 3000 20 3110 70 16 460
```
```action
wait 0.8
scene weapons borg on
wait 0.2
scene special borg gTechAssimBeam tgt; glide 10 100 12 400
wait 8
scene weapons borg off
wait 2
```

### a03 · The auto-assimilator
Shows: specials: auto-assimilator
Scene: assimilator
Variants: stock, refit
Layout: wipe
Transition: cut
In: 2.0
Length: 4.0
Status: draft

The Borg Assimilator fires its auto-assimilator into the Sovereign, from the side and
a little above, the beam running across the frame from the small Borg ship on the right
to the Sovereign on the left; the camera drifts round a little. **Wipe**: one beam
across the middle of the frame, the same in both variants, so the divider coming in
from the right shows stock's straight blue line turning into refit's blue cone with
its green fans, along the same beam. The weapons go off at once after the beam is
fired, so the Assimilator's torpedoes never fire; the beam lasts 30 s and the Sovereign,
healed, keeps its crew, so any window after the first second works.

```setup
scene orbit 3000 0 3150 100 12 380
```
```action
wait 0.8
scene weapons assim on
wait 0.2
scene special assim gborgbor tgt; glide 9 118 10 340
wait 0.3
scene weapons assim off
wait 9
```
