# Fleets: scout board
Film: fleets-scout
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --assets none --install . --install testbench/scene

A scratch board for two fleet scenes the user asked for, to be moved into the promo board
(`films/promo/storyboard.md`) once they read well. Not part of the promo yet. Every shot
here is a stock-against-refit pair, filmed the way the user liked the nebula shot best:
the ships moving, the camera behind or alongside them, the light doing the work.

Filming stopped on 2026-10-17 at the user's word, before any probe: every value here is
*unprobed* unless a note says what a bench session showed. One interactive refit session
(720p, `rom-shadow.ini`) scouted the shadow scene; what it showed is under f01.

## Planet shadows

Planets shadow what is behind them in refit (`PlanetShadows`, lighting 1.15.0); stock has
no planet shadows at all, so its fleet is lit the whole way. The argument: a fleet comes
out of a planet's night into the sun, and you see the edge of the shadow cross each hull.
Neither shot gets a title of its own until the user decides where it goes (planets are
not called out in the promo, the user, 2026-10-17; this is a lighting shot).

### f01 · Out of the planet's shadow
Shows: lighting: planet shadows, lighting: cloak
Scene: rom-shadow.ini
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: dissolve 0.6
In: 1.0
Length: 9
Status: draft

Six Romulan ships (three warbirds, two Griffins, a Shrike) sit cloaked in the shadow of a
class M planet, decloak one after another, and fly out across the shadow's narrow side
into the Key light. The camera is alongside the lead warbird on the sun's side, a little
behind it and below, following it (`orbit wb1`), so the shadow's edge stays put in the
world and sweeps back over each hull as it leaves. The edge is the subject: the eye
should land on the lead warbird's hull going from dark to lit, front first.

A dissolve, not a wipe: the edge runs across the frame as a line of its own, and a wipe's
divider beside it would read as a second edge; and stock decloaks on another timing
(b06: stock 3.4–5.2 s, refit 2.8–4.0 s), so the two halves would not match.

Scouted 2026-10-17 (refit, interactive, 720p): the geometry is right: the whole fleet is
dark inside the shadow, and the planet's night side reads as a disc behind it from yaw
35–80. The fleet covers about 300 units in 4 s (75 a second): the lead leaves the shadow
about 3 s after `goto`. From 400–500 units away and from yaw 80 (towards the planet) the
lit hulls were hardly brighter than the shadowed ones, because that side of each hull
faces away from the Key; from the sun's side (yaw 180–260) at 450 the hulls were small and
the edge did not read either. Hence the close camera below: about 160 from the lead, on
the sun's side. One frame from yaw 250, pitch 10, distance 500 had a dark diagonal band
in a corner: probably the planet's edge, which is above that line of sight; keep the pitch
at or below 0 there.

```setup
scene hover off; shieldfx off; tooltips off
scene cloak wb1 now; cloak wb2 now; cloak wb3 now; cloak gr1 now; cloak gr2 now; cloak sh1 now
scene orbit wb1 245 -6 170
wait 3
```
```action
wait 1
scene cloak wb1 off
wait 0.4
scene cloak wb2 off; cloak wb3 off
wait 0.4
scene cloak gr1 off; cloak gr2 off; cloak sh1 off
wait 2.2
scene goto wb1,wb2,wb3,gr1,gr2,sh1 3380 0 1709; glide 10 270 -4 150
wait 9
```

### f02 · The fleet leaves the night
Shows: lighting: planet shadows
Scene: rom-shadow.ini
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: dissolve 0.6
In: 1.0
Length: 8
Status: draft

Wider, behind the fleet, the planet's dark limb above it: the echelon flying away from
the camera and out of the shadow, each ship lighting up in turn as it crosses the edge.
Decloaked already in setup, so the shot is only the crossing. Behind and to the sun's side
(the fleet's course is yaw 125, so behind is 305; the sun is at 215), looking up a
little.

```setup
scene hover off; shieldfx off; tooltips off
scene orbit wb1 285 -8 380
wait 2
```
```action
wait 0.5
scene goto wb1,wb2,wb3,gr1,gr2,sh1 3380 0 1709; glide 10 270 -4 330
wait 9.5
```

## Species 8472 against the Borg

The fleet battle: five bioships against two cubes, a tactical cube and two spheres
(`borg-8472.ini`), everyone immortal and healed, so the fight lasts as long as the take.
The argument is the combat act's: weapons as light on the hulls, shield hits where they
land (8472 orange, Borg green, ART-DIRECTION.md), seen in a battle rather than on a
bench. The bioships fly towards the Key, so the near faces of the cubes are in shade and
the weapons' light on them reads.

### c01 · The bioships go in
Shows: lighting: weapons, shields
Scene: borg-8472.ini
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: cut
In: 4.0
Length: 8
Status: draft

Behind and below the lead bioship as the 8472 fleet closes on the Borg and opens fire,
the cubes ahead in shade, the camera easing in. A dissolve: the subject is spread over the
frame, and the fight plays out differently in each variant (who fires when is not the
same from session to session), so a wipe would join two different moments. If the
Borg's attacks bring up a transporter effect, it is out of the cut (`films/README.md`, keep
out); the showcase found that against the Federation, and with `Heal=1` shields should
hold.

```setup
scene hover off; tooltips off
scene orbit sp1 45 -8 380
wait 1
```
```action
wait 0.5
scene glide 14 30 -6 260
wait 14
```

### c02 · A cube under fire
Shows: lighting: weapons, shields
Scene: borg-8472.ini
Variants: stock, refit
Layout: dissolve 3 1.5
Transition: dissolve 0.4
In: 3.0
Length: 6
Status: draft

Close on the first cube, low, the bioships' beams striking it and lighting its plating,
its own fire going back. The cube is the one centred subject, but the beams around it are
not the same moment in both variants: a dissolve. Starts 12 s into the fight, when both
fleets are in range (a guess; the probe decides).

```setup
scene hover off; tooltips off
wait 12
scene orbit cube1 60 -8 260
```
```action
wait 0.5
scene glide 10 85 -5 230
wait 10
```
