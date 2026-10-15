# specials — `Specials.asi`

Two things stock draws with flat sprites and stretched models, drawn by shaders instead:

- **The warp flash.** A craft going to warp flies into a white four-pointed star
  (`warp_sprite`, `Mwarp`). `Specials.asi` draws a flash there: a white-hot core in a
  blue glow, stock's four-pointed star and a long horizontal lens streak, growing as the
  craft closes on the point, bursting as it gets there, then an afterglow and a ring
  running out from it; it lights the hulls about it.
- **The special weapons.** Each special weapon's shot (a tractor or holding beam, an
  assimilation beam, a plasma cannon, an ion storm torpedo, a psionic disruption…) and
  each special weapon's shockwave (the shield and energy rechargers', psionic
  insanity's, the flux wave's…) is drawn from a **recipe** in `Specials.ini`: a beam with
  a hot core in a glow, pulses flowing along it, strands twisting about it or a crackle
  throwing it about, flares at the ship and the target; a bolt with a tail and turning
  rays; a band of light running out in the wave's own plane with rays reaching in from
  it. Each lights the hulls about it.

It changes what is drawn and nothing else: where a flash or a shot is, how long it
lasts and what it does stay the engine's, so a player with it can play a player
without it.

The special weapons' **shield shells** (the corbomite reflector's, the shield
recharger's and drain's, remodulation, the Gemini effect, the clairvoyant link) are
drawn by `Shields.asi`, which owns the shield draw (`shields/README.md`, "The special
weapons' shells").

```
specials/install.sh            build, then install Specials.asi and Specials.ini
specials/install.sh --remove   take them out again
```

`./install` runs it; `./a2mod` switches it as the `specials` layer. It needs nothing from
`A2_DATA`: every effect is procedural. Without a Direct3D 9 device behind d3d8 (DXVK's
own d3d8, `platform/D3D9.md`) or with `Enable=0`, the game draws its own.

## The hooks

Addresses from `armada2.map`, GOG patch 1.1. Every site is checked before it is
written; a different `Armada2.exe` leaves the plugin inert and says so in
`Specials.log`. Call sites are chained: the plugin calls whatever the call went to when
it patched it, so another plugin's hook at the same site keeps working.

- **The call to `ST3D_Sprite::DrawScaled3D` (0x63aa60) at 0x59cc6b in
  `WarpSpriteEffect::Render`** (0x59ca40): the warp flash. The plugin notes the flash
  and draws nothing. Checked: `Render` draws at `ebx`, which it sets to the effect
  record's +4 (`lea ebx,[edi+4]` at 0x59caa9).
- **The call `[eax+0x5c]` at 0x58e6df in `Ordnance::RenderAll`** (0x58e580): the
  special weapons' shots. The five bytes that put the ordnance into `ecx` and call its
  `Submit` through the vtable (+0x5c) become a call to a
  thunk that hands the ordnance (`edi`) to the plugin: one with a recipe is noted and
  not submitted; any other is submitted through its vtable as before. The bytes are
  checked; if another plugin has them, the shots stay stock.
- **Slot 15 of the `ShockwaveExplosion` vtable (0x6afdac), `Submit`**, recognised by
  slot 13 (`ShockwaveExplosion::Init`, 0x466fd0): the special weapons' waves. Whatever
  the slot holds is called for every wave without a recipe: `Explosions.asi` patches
  the same slot for its rings, and either may load first.
- **The call to `ST3D_GraphicsEngine::RenderChunks` (0x62d470) at 0x598446 in
  `Armada_RenderAllOurStuff`**: after the objects, with the hulls in the depth buffer.
  The plugin calls it, then draws what this pass noted. `Explosions.asi`, `Shields.asi`
  and `Repair.asi` chain the same call. Noting and drawing per pass keeps the HUD's
  action camera, which draws the scene a second time each frame, right.
- **The call to `Ordnance::PreRenderAll` (0x58e4f0) at 0x598199**, right after
  `GameObject_PreRenderAll`: the plugin calls it, then registers its lights, ordinary
  `ST3D_Point_Light`s that `Lighting.asi` hands to the GPU-drawn hulls (the last pass's
  flashes and shots).
- **The d3d8 device's `Reset`** (slot 14), chained: the state block is let go.

## The warp flash

`mWarpCheck` (`cHoverCraftControlBase`'s and `cBorgControl`'s) calls
`CreateWarpSpriteEffect(craft, distance)` as a craft goes to warp: a `WarpSpriteEffect`
(0x18 bytes: the craft's handle +0, a point +4, the distance +0x10) at that distance
ahead of the craft along its velocity (+0xdc). `WarpSpriteEffect::Render` keeps the point
on the craft's line of motion, at signed distance `s` ahead of it, and draws the sprite
there at `(1 − |s| / distance) × 2.5` bounding radii, removing the effect once
`s < −2.5` radii: the star grows as the craft closes on it and shrinks past it. On the
bench a Galaxy (radius 48) made it 253 units ahead, a destroyer (radius 13) 40.

The plugin keeps a record per craft, by handle: how close it is (`k = 1 − |s|/distance`,
from the craft's position +0xac and velocity), and when `s` first reached zero (the
craft got there). Brightness goes as `k^Rise` while it closes; at arrival a burst
(`Burst` times over, dying in a third of the afterglow) and the afterglow, fading over
`Afterglow` seconds at the last place the engine drew it, after the engine has dropped
the effect. The ring runs out from there at `RingSpeed` radii a second for `RingTime`.
The flash is one quad, wide enough for the streak, pulled half its size toward the
camera so the hull it sits in does not hide it; its size has a floor in pixels
(`MinPixels`). Its light is full to `LightReach` radii.

## The special weapons

Every ordnance (a shot) has a class (+0x34) whose `cPrjID` (+0x150) names its ODF
(`cPrjID::GetOdfName`, 0x6593a0); a special weapon's ordnance is told by that name,
never by its C++ class: the `Phaser`, `Missile` and `Bullet` classes are shared with the
ordinary weapons (the Borg ion beam is a `Phaser`, the ion storm torpedo a `Bullet`). A
recipe `[Ordnance.<odf>]` makes it the plugin's.

What each kind reads:

- **beam**: a `Beam`'s or `BeamToggle`'s start (+0xbc, on the firing ship's hardpoint)
  and end (+0xc8, on the target), set by the engine every frame. One ribbon facing the
  camera from start to end, in units of its half width so the patterns do not stretch
  with its length; a cone (`Taper`) is twelve segments, so the core and glow widen with
  it. Flares at both ends; the light at the end.
- **bolt**: a projectile's position (`Entity::GetTransform`, 0x4cfd50, +0x24); the way
  it flies from where it was the frame before (at first, toward its target, +0x60). A
  ribbon from its tail to its head, and a flare at the head so that a bolt seen head on
  does not vanish.
- **field**: a bubble about the target (+0x60) or the owner (+0x5c), its radius in the
  ship's bounding radius. No surveyed weapon uses it yet.

Each shot's age (for `FadeIn`) is kept by the ordnance's address and recipe.

**Waves.** A `ShockwaveExplosion`'s class (+0x30) keeps its `cPrjID` at +0x24 (as
`ExplosionClass::Find` compares it), so a special weapon's wave is told by its ODF too:
`wengywav`, `wshldwav`, `wPsiInsanity` and the rest are the plugin's when they have a
`[Wave.<odf>]` recipe; the death shockwave (`xshock`) and the self-destruct's stay
`Explosions.asi`'s. The wave lies in the plane normal to its transform's up axis (+0xc)
about its position (+0x24); `ShockwaveExplosion::GetCurrentRadius` (0x467730) says how
far it has run and +0x50 how far it will, and it fades after its class's `fadeFactor`
(+0xe0) of that, as stock fades its disc.

### Finding out what a weapon is

`Survey=1` logs every ordnance the game submits, the first few times each ODF shows:
its ODF, C++ class, time left, position, beam ends, owner and target. The bench scene
fires special weapons on demand (`testbench/scene/README.md`, `special`):

```
SCENE=specials ./a2test session start --install . --install testbench/scene --args '-nointro a2_borg01'
./a2test drive --session ID scene "spawn s1 bbattle1 3000 0 3220 180 1"
./a2test drive --session ID scene "energy s1 on"
./a2test drive --session ID scene "special s1 gholdbm tgt"
```

A carrier's weapons must be on to fire a special (`Carrier::TriggerSelected` refuses
while they are off), and within its range of the target. Firing every special weapon
there (2026-10-13), these made ordnance and have recipes: `techassimbeam`, `holdbm`,
`tracbm` (and `tracbmo`, the Omega tractor, alike), `borgbor`, `borgionbeam`, `accelcannon`, `clairvoyantlink`,
`psionicdisruption`, `resourceextractionbeam`, `psyblst`, `hologen`,
`sdisrupt`, `shieldi` (beams); `ionstm`, `micro`, `microorgs`, `manheim`, `nanites`,
`nebulaflux`, `overrid`, `plasmabanko`, `shieldh` (bolts). Their colours follow stock's.
The thin coloured lines that some area weapons seemed to fire there were the ships'
own phasers (Klingon red, Romulan green): those weapons (death chant, the
holo-emitter, the sensor jammer, the energy and shield rechargers) work on everything
about the ship, and show as their wave.

The waves' colours are the stock textures' own, measured as the luminance-weighted mean
of each texture.

### Art direction: tractor and assimilation beams

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

## On the bench

`SCENE=specials`: an enemy Sovereign (`tgt`, immortal, unarmed, engines off) and a
Galaxy (`warper`) off to one side; `./a2test run specials --install .` fires the
holding beam, the tractor and psionic insanity and warps the Galaxy, checks the hooks
and that each was drawn, and keeps shots.

Seen there (2026-10-13), stock beside the plugin's at the same moment: the holding
beam (stock: a cone of thin green rays) a glowing green beam with pulses and a bright
impact; the tractor beam (stock: a thin double blue line) a soft blue beam with glows
at both ends; the auto-assimilator (stock: a cone of blue rays) crackling blue
lightning; psionic insanity's wave a pink band with rays running out over the field;
the warp flash a white core and blue streak bursting as the Galaxy goes, with its ring
and afterglow.

## Not covered yet

- The release package (`publish/`, the GUI installer) does not carry the layer.
- Not yet seen in game; tuned only on the bench, at one distance.
- Special weapons that draw a model rather than ordnance stay stock: the temporal
  stasis field's faceted shell (`Wstoptim.sod`), mines, colony pods, the commando team,
  the transwarp drive, the black-hole and rift weapons, the subspace shockwave's disc
  (`RepulsionWave`), the flux wave (`FluxWave`) and the ion storm cloud (an `IonStorm`
  game object). So do the lingering effects on a struck ship (`SpecialObjectEffect`:
  insanity, stasis, disable, the hologram), cloaks, and the mining beams.
- Weapons that never made ordnance on the bench have no recipe: `gcDreadnaught` (the
  rift; it would not fire there), `romspy`, `myotron`, `poleron`, `remod`, `utribm`,
  `uballbm`, `quantumEffect`, `transientRift`, `tempor`, `twarpd`, `wcover`, `posijam`,
  `dchant`, `rfield`, `holoemt`, `clairvoyance`, `colbeam`. A recipe under their
  ordnance ODF's name takes them when they do.
- The technology assimilation beam was submitted only on the frame it was fired on the
  bench; what keeps it off screen after that was not looked into.
- Psionic insanity's wave came at once when fired by hand in a bench session, and not at
  all in four runs of the `specials` scenario, from the same commands; the scenario takes
  its shot but does not assert the wave. What sets the wave off was not looked into.
  `PsionicInsanity::Submit` draws nothing in stock, so its ordnance has no recipe: the
  weapon's look is its wave.
