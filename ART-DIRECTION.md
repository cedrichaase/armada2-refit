# Art direction

How the effects should look: the user's direction, filled in from reference frames
(kept out of the repository, hard rule 8), with what was measured in them. Each section
names the layer that will carry it out. A section describes the target, not what is
installed: what a layer draws today is in its README, and what changed in its
CHANGELOG. When a layer's recipes take a direction up, say so here.

## Tractor and assimilation beams (`specials/`)

The user's direction (2026-10-15), not yet in the recipes: the tractor beams
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

Which ODFs "assimilation" covers is still to be settled when the recipes are written:
at least `techassimbeam`, and possibly `borgbor` (the auto-assimilator).

## Shield hits (`shields/`)

The user's direction (2026-10-15), not yet in `Shields.ini` or the shader. A shield hit
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
  lattice and no grid (in line with the 2026-10-12 decision, `shields/README.md`, "The draw").
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

Measured against today's look (`shields/README.md`, "The draw"): the hot spot, white core, faction colour
and the glow that stays near the hit already fit. Not in the references:
- the ring running out from the hit;
- a rim that brightens where the shell is seen edge on, in place of an edge that just
  ends where the lit mist stops.

Missing from today's look:
- the misty, uneven texture;
- the rays fanning out from the impact, on Klingon shields;
- the light on the struck hull.

The next pass should settle whether the ring and the rim stay.
