# Intro cinematics, scouted 2026-10-17

Moments from the in-engine intro cinematics of three first missions, from the board
`films/intros-scout` (probe takes, 1280x720, filmed with the HD textures, `-nointro`).
Times are seconds into the take. Each cinematic ends in a hard cut to the mission's
briefing window (s01 at 70.5 s, s02 at about 103.5 s, s03 at about 74.5 s); nothing after
that is cinematic.

Things that apply to every entry:

- **No burned-in text.** The takes are filmed with `Captions=0` in the board's scene file
  (`testbench/scene/README.md`), re-filmed 2026-10-17: the subtitles at the top of the
  Federation cinematic ("Space... the final frontier" to "To boldly go where no one has
  gone before.", 0 to 24 s), its lower third (`USS Enterprise / Stardate 54522.2 /
  Location: Alpha Quadrant`) and title ("Invasion", 26 to 35 s), and the Klingon and
  Borg intros' stardate lower thirds and titles ("A Dagger At Zarush", "Werewolf Pack")
  are not drawn. The rest of each cinematic is unchanged. The s02 and s03 takes of this
  pass start 2.1 s and 3.7 s earlier in their cinematics than the takes first logged
  here, so they show a little more of each opening shot; the entries' times are
  re-measured for these takes (s01 moved by under 0.15 s and is kept).
- **Battles vary from take to take.** The camera paths, fades and cuts are scripted and
  land within 0.15 s of each other on every take, but the fighting is simulated live:
  which ship dies, and when, moved by a second or more between the two Klingon takes
  (`kling01-saucer-nebula`). Check an explosion's time on the take that is cut.
- **The cinematics fade through black between scenes**, so each entry is cut inside one
  scene and its `In` and `Out` keep clear of the fades.
- **Stock and refit are not on the same clock.** The Federation cinematic is the same
  content in both, but the stock take runs about 4.7 s ahead of the refit take. Take
  stock times as refit minus 4.7 s (measured on the takes of 2026-10-17 without captions:
  the fade to black, refit 59.6 s, stock 55.0 s; the hard cut into the nebula, refit
  36.2 s, stock 31.4 s; the first takes measured 4.8 s). The offset is a property of when
  each take's recording started, so measure it again on a new take.
- **Audio.** The voice stem is the cinematic's narration, the music stem its score (both
  in every take); the effects stem is the 3D sound of the scene. A music stem is present in
  all takes. For a bed with a voice-over of the film's own, use the effects stem alone.

## Federation, a2_fed01 (s01)

### fed01-fleet-planet · Enterprise-E over a brown planet
Take: intros-scout s01 refit probe
Mission: a2_fed01, the intro cinematic, 13 s in (stock take: same moment at 8.4 s)
In: 13.2
Out: 17.6
Audio: score; narration not yet started; no effects. Usable under a bed
Quality: usable: 720p
Shows: models, textures, lighting

A brown, cratered planet fills the bottom of the frame and the Enterprise-E crosses it
slowly from the left, lit from above, its red nacelle caps and the deflector glow the
brightest points. The eye follows the ship along the planet's limb. The scene fades out
at 18 s. A second pass of the same set follows at 19.0 to 23.0 s (the ship against the
planet's disc, a smaller Federation ship ahead), which also cuts well.

### fed01-fleet-station · Federation ships by a station and a planet
Take: intros-scout s01 refit probe
Mission: a2_fed01, the intro cinematic, 26 s in (stock take: 21.5 s)
In: 26.4
Out: 36.0
Audio: score; narration begins at 27 s; no effects. The narration makes it unusable under a bed of the film's own unless the voice stem is dropped
Quality: usable: 720p
Shows: models, textures, lighting

A slow drift past a group of Federation ships: a Galaxy-class and a Sovereign-class ship
in line, a saucer-section station with a ring, a small ship and the edge of an orbital
platform, in front of a pale blue-white planet at the right. The Enterprise's lit saucer
is what the eye lands on, then the planet. The camera barely moves; the ships hold
position. It ends in a hard cut at 36.2 s, so keep `Out` at 36.0.

### fed01-nebula-planet · The camera pushes through a nebula to a planet
Take: intros-scout s01 refit probe
Mission: a2_fed01, the intro cinematic, 36 s in (stock take: 31.4 s)
In: 36.4
Out: 59.4
Audio: score; narration from 27 s to 69 s over it; faint whoosh effects 38 to 62 s. Narration makes it unusable under a bed unless the voice stem is dropped
Quality: good: 720p; the long, clean push, no text on screen
Shows: nebulae, lighting, textures

The camera pushes slowly forward through a thick yellow nebula with a purple and green
one beyond, towards a striped gas giant with a small bright sun at its right. The planet
grows from a pea to a quarter of the frame height over 23 s; the sun flares at 44 to 48 s;
the frame darkens to black by 59.5 s. The eye goes to the planet and the sun at the far
end of the nebula. In the stock take the nebula is more saturated (magenta and yellow);
in refit it is softer blue-purple. Same camera path in both. This is the pan with a planet
at its end.

### fed01-enterprise-e · The Enterprise-E, from above and below
Take: intros-scout s01 refit probe
Mission: a2_fed01, the intro cinematic, 60 s in (stock take: 55.4 s)
In: 60.3
Out: 64.0
Audio: score; narration to 69 s; no effects. Usable only with the voice dropped
Quality: usable: 720p
Shows: models, textures

A full view of the Enterprise-E from above and in front, turning slowly in a dark green
nebula; the two long nacelles sweep across the frame. The eye lands on the saucer and the
nacelle caps. It continues, as a second scene, at 64.2 to 68.0 s: the saucer's underside
pass, then a Federation cargo ship (hull number 4881) crossing. 68.2 to 70.5 s is two
ships passing close to the camera with a station behind them. Hard cut to the briefing at
70.5 s.

## Klingon, a2_kling01 "A Dagger at Zarush" (s02, refit only)

### kling01-fleet-nebula · The Federation fleet over a red nebula
Take: intros-scout s02 refit probe
Mission: a2_kling01, the intro cinematic, opening shot
In: 0.0
Out: 13.9
Audio: score; narration throughout (0 to 51.1 s); effects bed throughout, no distinct hits
Quality: usable: 720p
Shows: nebulae, models, lighting

A slow track along a red nebula with a Federation fleet at anchor: a rust-coloured planet
at the centre, a station, Federation ships in two lines. The planet is the first thing
the eye lands on, then the ships glide into frame at the left. It fades through black
at 14.1 s. A second scene follows at 16.1 to 24.6 s, a gold Klingon ship crossing the red
nebula with a small planet behind it.

### kling01-station-attack · Phasers and a shield bubble over the station
Take: intros-scout s02 refit probe
Mission: a2_kling01, the intro cinematic, 27.1 s in
In: 27.1
Out: 38.6
Audio: score; narration; effects bed (phasers, no clear hits above it)
Quality: good: 720p
Shows: effects (beams, shields), models, nebulae

A long Federation station, blue windows lit, under fire: yellow phaser beams cross the
frame from every side (29.1 to 33.1 s), a Federation ship's blue shield bubble flares
under hits at 31.1 to 33.1 s, and fireballs bloom on the station at 34.1 to 38.1 s as the scene
moves from black space into a red and green nebula. The eye lands on the beams, then the
bubble. Klingon ships are on the edges of the frame.

### kling01-saucer-nebula · Beam fire over a station, green nebula, a blast ring
Take: intros-scout s02 refit probe
Mission: a2_kling01, the intro cinematic, 39.6 s in
In: 39.9
Out: 52.4
Audio: score; narration; effects bed, loudest hits later
Quality: good: 720p; the first 1.5 s fade up from black
Shows: effects (beams, explosion), nebulae, models

A Federation saucer and the station in a green nebula, phaser beams from off-frame
crossing them, the saucer's shield flaring at about 46 s. At about 51 s a ship dies at the
upper left and the explosion's shockwave opens as an orange ring with a dark core (about
51.5 s to the fade at 52.5 s), seen with the station and saucer in front of it. The eye
goes to the ring. In this take the ring comes about 1.5 s later in the scene than in the
first take, where it had 1.7 s before the fade: see "Battles vary" above.

### kling01-station-defence · The starbase under fire, fire rings and nebulae
Take: intros-scout s02 refit probe
Mission: a2_kling01, the intro cinematic, 53.1 s in
In: 52.9
Out: 67.7
Audio: score; narration; effects hits at 54.6, 56.1, 56.6 and 60.1 s are the loudest of the cinematic
Quality: good: 720p
Shows: effects (beams, explosions), nebulae, models, lighting

A domed starbase with a spire and ring in front of a green and red nebula and a planet,
with Federation and Klingon ships around it and beams across everything. A fireball
opens on its flank at 58.1 s; from 59.1 s large orange shockwave rings fill the background
(59.1 to 66.1 s) with debris crossing the frame. The camera orbits the station slowly. The
eye lands on the station, then the rings. The best single minute for combat, explosions
and nebulae together. Fades at 67.7 s.

### kling01-bop-ring · A saucer's last shots and a ring blast
Take: intros-scout s02 refit probe
Mission: a2_kling01, the intro cinematic, 68.1 s in
In: 68.4
Out: 77.6
Audio: score; narration to 77.1 s; effects hit at 75.6 to 76.1 s
Quality: usable: 720p; the first 2 s are dark
Shows: effects (beams, explosions), models

A lone saucer in a dark green nebula under fire from below (69.1 to 73.1 s), beams
converging on it from two Klingon birds of prey in gold at the lower right (70.1 to 73.1 s);
at 74.1 to 77.1 s a ship dies and a wide orange ring opens round the two Klingon ships.
Cut at 77.6 s: after it the cinematic spends 20 s on a dark, poorly lit Klingon cruiser
(79.1 to 101.1 s) which is not worth using.

## Borg, a2_borg01 (s03, refit only)

### borg01-station-explodes · A Borg station destroyed in a nebula
Take: intros-scout s03 refit probe
Mission: a2_borg01, the intro cinematic, opening shot
In: 0.0
Out: 12.5
Audio: score (quiet); narration throughout; effects loud throughout (explosion at 5.7 s, shockwave ring after) at the loudest level of the three takes
Quality: good: 720p
Shows: effects (explosion), nebulae, lighting

A Borg ring station hangs in a red and green nebula with a dark Borg ship at its lower
left; it blows up at 5.5 s, a yellow-white fireball (5.7 to 7.7 s), then a bright orange ring
shockwave with a dark core opens (8.2 to 12.2 s) and the dark ship drifts through it. The
eye lands on the fireball, then the ring. Fades to black at 12.5 s.

### borg01-cube-planet · A cube goes behind a planet and the ring blast follows
Take: intros-scout s03 refit probe
Mission: a2_borg01, the intro cinematic, 12.7 to 30.7 s
In: 13.2
Out: 30.5
Audio: score (a gap 20.7 to 23.7 s); narration; effects peak at 19.2 s and 29.7 to 31.2 s
Quality: usable: 720p. The cube is not seen exploding: see the note
Shows: models, textures, effects

A Borg cube, a speck in black space, closes on a tan, cloud-banded planet whose limb
fills the lower right. It grows from a few pixels to a good size by 24.7 s, is crossing in
front of the limb at 25.7 s and goes behind the planet at 28.2 s, as fragments of a
second, angular Borg ship show at the edge (28.5 s). From 28.9 s an orange explosion
ring opens behind the planet's limb with a ship caught in it (29.7 s), then it fades to
black at 30.9 s. This is the nearest the cinematic has to a cube blown up into a planet:
the cube's own destruction is behind the planet. Check with the user before a promo
claims it.

### borg01-diamond-asteroids · A Borg diamond among asteroids and nebulae
Take: intros-scout s03 refit probe
Mission: a2_borg01, the intro cinematic, 31.7 to 44.7 s
In: 31.5
Out: 44.2
Audio: score; narration; effects bed
Quality: usable: 720p; the ship is nearly black against the nebula, lit only on its edges
Shows: nebulae, models, lighting

A Borg ship of the diamond type crosses the frame as a dark silhouette against a red,
yellow and green nebula, with a field of dark asteroids drifting past in front. The
nebula's colours are the subject; the Borg ship is a lattice shape in the middle of the
frame. At 44.7 s the scene fades to a close, green nebula.

### borg01-diamond-nebula · The Borg ship in front of a green and purple nebula
Take: intros-scout s03 refit probe
Mission: a2_borg01, the intro cinematic, 44.7 to 60.7 s
In: 45.2
Out: 60.5
Audio: score; narration; effects bed
Quality: good: 720p. The same nearly black silhouette as the entry above
Shows: nebulae, models

The Borg ship, small at the left, flies at the camera across a wide green and yellow
nebula with a purple one at the right; it grows from a few pixels to a fifth of the
frame (44.7 to 59.7 s) and slides right over the purple cloud, with a small escort ship
beside it. The eye follows the ship. It fades to black at 61.2 s and a close-up of the
hull follows (62.2 to 73.7 s, dim, grey, with violet and amber lights in the hull; usable
as a texture shot only).

## Problems seen in these takes

- No crashes and no black or frozen stretches inside a cinematic: all `frozen` flags in
  the review are the briefing window after the cut (about 70.5, 103.5 and 74.5 s onwards).
- The Borg and Klingon ships in the dark scenes are very dark (lit by the nebula alone).
  Check on the 4K take whether this is the scene or the render.
- No burned-in text since the 2026-10-17 re-take (`Captions=0`).

### fed01-nebula-planet-stock · The push through the nebula, stock
Take: intros-scout s01 stock probe
Mission: a2_fed01, the intro cinematic, 31.4 s into the stock take (refit: 36.4 s)
In: 31.6
Out: 54.6
Audio: as `fed01-nebula-planet`, 4.8 s earlier
Quality: good: 720p, no text on screen
Shows: nebulae

`fed01-nebula-planet` in the stock take, on the stock clock (refit minus 4.8 s, measured
on the hard cut into this scene and the fade out of it). Stock's nebula is a flat, loud
yellow against magenta with hard edges; the planet and the sun sit in the same places,
so a dissolve into the refit entry at the matching time compares the two in place.
