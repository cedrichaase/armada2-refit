# Art direction

How the effects should look: the user's direction, filled in from reference frames
(kept out of the repository, hard rule 8), with what was measured in them. Each section
names the render3d module that will carry it out. A section describes the target, not what is
installed: what a module draws today is in `render3d/README.md`, and what changed in
`render3d/CHANGELOG.md`. When a module's recipes take a direction up, say so here.

## Tractor and assimilation beams (render3d's `ordnance`, `render3d/ordnance.ini`)

The user's direction (2026-10-15), drawn since render3d 0.4.0 (`Kind=cone` in
`render3d/ordnance.ini`), not yet seen in game: the tractor beams
(`tracbm`, `tracbmo`), the Borg holding beam (`holdbm`, the Borg tractor) and the
assimilation beams should not be lasers. The current recipes draw them as ribbons with
a hot core, flowing pulses and twisting strands. These beams should be **a fluctuating
cone, like a laser projector shining through vapour**:

- **A cone, not a line.** It starts at a point on the emitter and widens toward the
  target. At the wide end it takes in **most of the target's area**, so the target sits
  inside the beam rather than at its tip.
- **Light in a medium, not a solid object.** The cone is translucent: stars and hull
  show through it. It is brightest at the emitter and thins toward the target.
  Brightness runs in streaks along its length, as a projector's beam does in haze.
  There is no white-hot core.
- **Fluctuating.** The streaks and the vapour drift and shimmer the whole time it is on.
  The beam's outline stays steady while the light inside it moves. Nothing should twist
  or pulse in a regular, readable way.
- **It lights the target.** The hull inside the cone takes a faint wash of the beam's
  colour, the same colour as the beam. The wash covers the part of the hull the cone
  reaches. It is faint enough that the hull's own detail still reads.
- **Colours stay stock's**: blue for the tractors, green for the Borg.

Seen in the references (frames from DS9's tractor shots and TNG's Borg cube; kept out of
the repository):

- **The streaks.** A profile across a DS9 tractor beam has three or four broad
  lengthwise bands (blue 140, 85, 135, 70, 125), not a smooth falloff. The edges are as
  bright as the middle, and the beam drops to black within a few pixels of its edge.
- **The colour.** Saturated blue near the emitter (about 1,56,162). Toward the target it
  fades into the sky. Over a bright background it reads as pale, almost white cyan,
  because it adds light.
- **The hull wash.** The struck runabout is lit blue all over (74,129,186 against a grey
  hull). The Enterprise's saucer under the Borg beam measures 55,69,64 where the beam
  falls and 10,11,11 just outside it: a faint green-grey cast confined to the cone's
  footprint.
- **The Borg beam's source.** A small bright point on the cube (97,162,104) where the
  cone starts. Its end is as wide as the saucer.

Which ODFs "assimilation" covers: `techassimbeam` (the technology assimilation beam)
and `borgbor` (the auto-assimilator: an assimilator by name, and drawn in stock from
the same `Wborgbore` art as the holding and tractor beams). `borgbor` keeps the blue
its recipe had rather than the Borg green; that it is stock's blue is still to be
checked against stock in game. `fusionholdbm` (a
holding beam no ship carries in stock) has no recipe.

## Shield hits (render3d's `effects`)

The user's direction (2026-10-15), drawn since render3d 0.4.0 (`[Shields]`, `shaders/effects_shields.hlsl`), not yet seen in game. A shield hit
should read as **a misty, glowing dispersal of light over a curved ellipsoid that
encloses the ship. Only the area around the impact is actually lit up.** Federation
shields are a pale, light blue; Klingon shields are the same pattern in a strong,
bright green.

What that means, filled in from the references (frames from *Voyager* and *DS9*: the
Intrepid and Prometheus classes taking hits; *Star Trek VI*: a Klingon Bird-of-Prey's
shield; kept out of the repository):

- **The shell is implied, not drawn.** Nowhere is the ellipsoid visible as a surface
  of its own. You read its curve from how the lit area bends: the glow wraps around the
  ship as if painted on a curved surface. Where the lit area reaches the shell's
  silhouette, it ends on a soft but definite curved edge against space (the
  Bird-of-Prey's and the Prometheus's frames). Away from the hit there is nothing:
  space and the hull show as if no shield were there.
- **Mist, not a surface pattern.** Around the impact the light is soft, cloudy haze
  with uneven density, like lit vapour rather than a smooth glow. There is no hex
  lattice and no grid (in line with the 2026-10-12 decision for `Shields.asi`, whose look render3d keeps).
- **Rays are Klingon only.** The Bird-of-Prey's shield breaks into clear rays, a
  sunburst from the impact point about as long as the lit area's radius. The Federation
  shields (Intrepid, Prometheus) have none: just the mist. So rays are part of the
  Klingon look, not of every hit. Every other faction uses the Federation pattern, mist
  without rays, in its own colour.
- **A bright point where the shot lands.** Where the beam or torpedo meets the shell
  there is a small, near-white flare (Bird-of-Prey about 205,200,187; Prometheus about
  254,230,211, warmed by the orange disruptor). The beam visibly stops there, at the
  shell, not at the hull.
- **The falloff is wide and soft.** In the Intrepid long shot the glow is brightest
  just beside the impact, at about 63,163,220, and fades to the sky over about a fifth
  of the frame. On the Bird-of-Prey the green is still about 42,116,1 at 40% of the
  lit radius and about 1,36,2 at 80%. The lit area is large, a good part of the ship's
  length, but it never covers the whole shell.
- **The hull under the hit is lit.** On the Intrepid close-up the saucer under the
  impact is washed pale blue-white, about 179,202,247 at its brightest and 113,129,175
  a little further off. The hull far from the hit stays dark (15,23,30). The shield
  light falls on the hull as well as glowing in front of it: a light on the hull is
  part of the look, not an extra.
- **Colour by faction, saturated in the falloff and white at the core.**
  - Federation: light blue to periwinkle. The haze over space reads lavender-grey where
    it is thin (82,78,91 on the Prometheus).
  - Klingon: pure green, with red and blue near zero in the falloff (3,80,1).
  - Borg and Romulan: green, in the Federation pattern.
  - Species 8472: orange, in the Federation pattern.
  - Cardassian and any other faction: the Federation pattern; colour not yet given.

Measured against today's look (`render3d/README.md`, "effects"): the hot spot, white core, faction colour
and the glow that stays near the hit already fit. Not in the references:
- the ring running out from the hit;
- a rim that brightens where the shell is seen edge on, in place of an edge that just
  ends where the lit mist stops.

Missing from the look before render3d 0.4.0, and added there (`Mist*`, `Edge`, `Ray*`,
`HitLight*`; not yet seen in game):
- the misty, uneven texture, and the soft curved edge at the shell's silhouette;
- the rays fanning out from the impact, on Klingon shields;
- the light on the struck hull;
- the faction colours above (Klingon green, Species 8472 orange).

The next pass should settle whether the ring and the rim stay.

## Cloaking (render3d's `hulls`, the `Cloak*` keys in `Render3D.ini`)

The user's direction (2026-10-15). In `ps_cloak` and `vs_hull` (`shaders/hulls.hlsl`)
since render3d 0.3.0, not yet seen in game; the keys are in `render3d/README.md`. The
cloak as it is now looks good and stays the base: the hull as glass over a copy of the
frame, bending what lies behind it. Two things should change: the transition, and how
much is left of a ship once it is cloaked.

### The transition: the ship wavers out

As in the TV series, a ship that cloaks should **look as though its whole geometry is
bending in waves before it is gone**, and the same in reverse when it decloaks. Today a
noise threshold eats the hull away patch by patch, with a glowing front where it is
eating. The ship never changes shape; it is cut away.

- **The shape moves, not just the shading.** Waves run across the whole hull and visibly
  shift its outline: the silhouette ripples against the stars, nacelles and saucer edge
  bend and swim. It should read as the ship's geometry bending, not as a pattern
  painted on a ship that stays still.
- **Waves, not noise.** Broad, regular-looking bands of distortion that travel across
  the hull, a few bands across the ship's length, rather than random blotches. They
  move steadily in one direction while the transition runs.
- **The hull dissolves into the bending, not away from it.** While the waves run, the
  hull fades into the refracting glass: first the shape starts to waver, then its
  colour thins into the bent background, then the last of it ripples out. Decloaking
  plays the same thing backwards: the waves bring the ship in, then settle, and the
  ship holds still once it is solid.
- **Strongest mid-transition.** The waves are weak at the start, largest around the
  middle, and settle to the cloaked look at the end. The current `CloakRipple` already
  peaks mid-transition (`vis·(1−vis)`). The new waves follow the same envelope, but much
  larger: big enough to move the silhouette by a visible fraction of the hull's width.
- **The glowing front is optional.** Whether the green edge (`CloakFront`) stays,
  is replaced by the waves, or becomes a faint shimmer along the crests is open. In
  the series there is no sharp front: the ship is simply gone at the end of the waver.

### Cloaked: a little more there

Fully cloaked, a ship today is nearly invisible (`CloakHull=0`; only a faint bend, rim
and glint). It should be **a little more visible**: never a ship-shaped object, but a
patch of space that is clearly not quite right.

- **A slow, moving wave pattern in the refraction.** Faint waves keep drifting across
  the cloaked hull, and they change *where the light behind it is bent*: the stars and
  hulls seen through it slide and wobble a little as each wave passes. The pattern
  should be in the bending, not in a brightness or colour overlay: what you see is the
  background refracted differently from moment to moment.
- **Slow and gentle.** Much slower and smaller than during the transition: a heat
  shimmer, not a ripple of water. It should never turn or pulse in a readable rhythm.
- **Visible against a busy background, nearly gone against black.** Since it shows only
  as bent background, it should be easy to spot in front of a nebula, a planet or
  another ship, and barely there against empty space. That is the right behaviour, not
  something to correct with a glow.
- **Your own ships keep a trace of their hull.** A cloaked ship of the player's own
  shows its hull texture at about **10% opacity** under the glass and the shimmer:
  enough to find and pick out your own ships at a glance, never enough to read as
  uncloaked. Every other cloaked ship the engine draws keeps the bend and shimmer
  alone, with no hull. Today `CloakHull` (the hull's floor while cloaked) is one value
  for every ship; this needs it per owner, 0.1 for the player's own and 0 otherwise.
- **Fair in multiplayer.** This applies only to cloaked ships the engine already draws
  (your own, and what your side can see). It must never draw a ship the engine hides
  from the player: render only, as everything in render3d is.

Measured against the look before render3d 0.3.0: the glass, the colour split along the
bend, the rim and glint fit. Missing then, and drawn now:
- waves large enough to move the silhouette during the transition;
- the hull dissolving through the waves, in place of the noise threshold;
- a slow drifting wave in the refraction while fully cloaked;
- the player's own cloaked ships showing their hull at about 10%.

The green front stays for now as a thin line along each band (`CloakFront=0` takes it
out). The next pass should settle whether it stays, and how strong the cloaked
shimmer may be before it reads as a ship rather than a disturbance (judge it in game,
against a nebula and against empty space).
