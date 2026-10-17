# Fleets: scout board
Film: fleets-scout
Final: 3840x2160
Pointer: 1920x1080
Variant refit: --install . --install testbench/scene

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
Variants: refit
Layout: full refit
Transition: dissolve 0.6
In: 2.0
Length: 13
Status: probed

Restaged on the user's notes on rough cut 4 (2026-10-17): refit only, no stock recording
("it doesn't add anything here"), the fleet cloaked first, then decloaking, then flying
out of the planet's shadow so the light moves across the hulls, with part of the planet
in frame. Refit only, so the layout is `full refit`.

The six ships start cloaked inside the shadow; the lead warbird decloaks first (3.5 s to
6.5 s of the take, the Griffins and the Shrike after it, done by 8.5 s), a beat, then
`goto` at 8.4 s takes the echelon out across the shadow's narrow side. The camera is
`orbit wb1`, ahead of the lead and a little to the shadow side at first (yaw 100, pitch
-5, 190 away), so the planet's night side is in the upper left corner behind the ships;
it glides round over 10 s to yaw 195, 150 away, the sun's side, following the lead. As
the camera comes round the lead leaves the shadow and its far wing goes from dark to lit
(11.5 s dark, 13 s catching the light, 15 s lit, the green hull plating and the wing
edge bright). The planet slides out of frame to the left by about 12 s; the focus is the
ships and the light. Probed twice: a camera at yaw 120 to 215 showed the lighting as
well but lost the planet from the start; one at yaw 70 kept the planet large but saw the
dark side of the hulls.

The cloak holds invisible: team 2's cloaked ships draw as nothing from this side (the
first 3.5 s is empty starfield with the planet corner), so the cloak's shimmer shows
only as the ships decloak (3.5 to 8.5 s, the green-edged glass fading to hull). Cut the
shot in at 2.0 s so the empty hold is short. The terminator is soft rather than a line:
the sweep reads as the hull brightening, not as a sharp edge.

```setup
scene hover off; shieldfx off; tooltips off
scene cloak wb1 now; cloak wb2 now; cloak wb3 now; cloak gr1 now; cloak gr2 now; cloak sh1 now
scene orbit wb1 100 -5 190
wait 3
```
```action
wait 3
scene cloak wb1 off
wait 0.4
scene cloak wb2 off; cloak wb3 off
wait 0.4
scene cloak gr1 off; cloak gr2 off; cloak sh1 off
wait 2.0
scene goto wb1,wb2,wb3,gr1,gr2,sh1 3380 0 1709; glide 10 195 -3 150
wait 10
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
Status: probed

Behind and below the lead bioship as the 8472 fleet closes on the Borg and opens fire,
the cubes ahead in shade, the camera easing in. A dissolve: the subject is spread over the
frame, and the fight plays out differently in each variant (who fires when is not the
same from session to session), so a wipe would join two different moments. If the
Borg's attacks bring up a transporter effect, it is out of the cut (`films/README.md`, keep
out); the showcase found that against the Federation, and with `Heal=1` shields should
hold.

```setup
scene hover off; tooltips off
scene orbit sp1 30 -8 230
wait 1
```
```action
wait 0.5
scene glide 14 20 -5 170
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
Status: probed

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
